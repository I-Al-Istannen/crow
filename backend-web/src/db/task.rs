use crate::config::TestCategory;
use crate::error::{Result, SqlxSnafu, WebError};
use crate::types::{
    ExecutionExitStatus, FinalSubmittedTask, FinishedCompilerTaskSummary, FinishedTestSummary,
    TaskId, TeamId, TestId,
};
use crate::UserId;
use jiff::Timestamp;
use shared::{
    AbortedExecution, ExecutionOutput, FinishedCompilerTask, FinishedExecution, FinishedTaskInfo,
    FinishedTest, InternalError, TestExecutionOutput, TestExecutionOutputType,
};
use snafu::{location, ResultExt};
use sqlx::{query, Acquire, Sqlite, SqliteConnection};
use std::collections::HashMap;
use std::ops::Add;
use std::time::{Duration, SystemTime};
use tracing::{info_span, instrument, Instrument};

#[instrument(skip_all)]
pub(super) async fn add_finished_task(
    con: impl Acquire<'_, Database = Sqlite>,
    result: &FinishedCompilerTask,
    queue_time: Timestamp,
) -> Result<()> {
    let mut con = con.begin().await.context(SqlxSnafu)?;

    let start_time = result
        .info()
        .start
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64;
    let end_time = result
        .info()
        .end
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64;
    let queue_time = queue_time.as_millisecond();

    match result {
        FinishedCompilerTask::BuildFailed { build_output, .. } => {
            let id = record_execution_output(&mut con, build_output).await?;
            record_task(&mut con, result, start_time, end_time, &id, queue_time).await?;
        }
        FinishedCompilerTask::RanTests {
            tests,
            build_output,
            ..
        } => {
            let build_id = uuid::Uuid::new_v4().to_string();
            record_finished_execution(
                &mut con,
                &build_id,
                build_output,
                &None,
                ExecutionExitStatus::Success,
            )
            .await?;
            record_task(
                &mut con, result, start_time, end_time, &build_id, queue_time,
            )
            .await?;

            for test in tests {
                let (compiler_exec_id, binary_exec_id) =
                    record_test_execution(&mut con, &test.output).await?;
                let status = TestExecutionOutputType::from(&test.output).to_string();

                query!(
                    r#"
                    INSERT INTO TestResults
                        (task_id, test_id, compiler_exec_id, binary_exec_id, status,
                         provisional_for_category)
                    VALUES
                        (?, ?, ?, ?, ?, ?)
                    "#,
                    result.info().task_id,
                    test.test_id,
                    compiler_exec_id,
                    binary_exec_id,
                    status,
                    test.provisional_for_category
                )
                .execute(&mut *con)
                .instrument(info_span!("sqlx_add_finished_insert_test"))
                .await
                .context(SqlxSnafu)?;
            }
        }
    }

    con.commit().await.context(SqlxSnafu)?;

    Ok(())
}

#[instrument(skip_all)]
pub(super) async fn get_task(
    con: impl Acquire<'_, Database = Sqlite>,
    task_id: &TaskId,
) -> Result<(FinishedCompilerTask, Vec<TestId>)> {
    let mut con = con.begin().await.context(SqlxSnafu)?;

    let task = query!(
        r#"
        SELECT
            task_id as "task_id!: TaskId",
            start_time as "start_time!: u64",
            end_time as "end_time!: u64",
            team_id as "team_id!: TeamId",
            revision as "revision_id!: String",
            commit_message as "commit_message!: String",
            execution_id as "execution_id!: String"
        FROM Tasks
        WHERE task_id = ?
        "#,
        task_id
    )
    .fetch_optional(&mut *con)
    .instrument(info_span!("sqlx_get_task_query"))
    .await
    .context(SqlxSnafu)?;

    let Some(task) = task else {
        return Err(WebError::not_found(location!()));
    };

    let build_output = get_execution(&mut con, &task.execution_id).await?;
    let start = SystemTime::UNIX_EPOCH.add(Duration::from_millis(task.start_time));
    let end = SystemTime::UNIX_EPOCH.add(Duration::from_millis(task.end_time));
    let info = FinishedTaskInfo {
        task_id: task_id.to_string(),
        start,
        end,
        revision_id: task.revision_id,
        commit_message: task.commit_message,
        team_id: task.team_id.to_string(),
    };

    let outdated_tests = get_outdated_tests(&mut con, task_id)
        .instrument(info_span!("sqlx_get_task_outdated_tests"))
        .await?;

    if !matches!(build_output, ExecutionOutput::Success(_)) {
        return Ok((
            FinishedCompilerTask::BuildFailed { info, build_output },
            Vec::new(),
        ));
    }

    let tests = query!(
        r#"
        SELECT
            test_id,
            compiler_exec_id as "compiler_exec_id!",
            binary_exec_id,
            status,
            provisional_for_category as "provisional_for_category?",
            (SELECT category FROM Tests WHERE id = test_id) as "category?"
        FROM TestResults
        WHERE task_id = ?"#,
        task_id
    )
    .fetch_all(&mut *con)
    .instrument(info_span!("sqlx_get_task_tests"))
    .await
    .context(SqlxSnafu)?;

    let mut finished_tests = Vec::new();
    for test in tests {
        let test_id = test.test_id;
        let execution_output = get_test_execution(
            &mut con,
            &test.compiler_exec_id,
            test.binary_exec_id,
            test.status.parse().unwrap(),
        )
        .await?;
        finished_tests.push(FinishedTest {
            test_id,
            category: test.category,
            output: execution_output,
            provisional_for_category: test.provisional_for_category,
        })
    }

    Ok((
        FinishedCompilerTask::RanTests {
            info,
            build_output: build_output.into_finished_execution().unwrap(),
            tests: finished_tests,
        },
        outdated_tests,
    ))
}

pub(super) async fn get_recent_tasks(
    con: impl Acquire<'_, Database = Sqlite>,
    team_id: &TeamId,
    count: i64,
) -> Result<Vec<FinishedCompilerTaskSummary>> {
    let mut con = con.begin().await.context(SqlxSnafu)?;

    let tasks = query!(
        r#"
        SELECT
            task_id as "task_id!: TaskId"
        FROM Tasks
        WHERE team_id = ?
        ORDER BY start_time DESC
        LIMIT ?
        "#,
        team_id,
        count
    )
    .map(|it| it.task_id)
    .fetch_all(&mut *con)
    .instrument(info_span!("sqlx_get_recent_tasks_query"))
    .await
    .context(SqlxSnafu)?;

    let mut finished_tasks = Vec::new();

    for task in tasks {
        let task = get_task_summary(&mut con, &task)
            .instrument(info_span!("sqlx_get_recent_tasks_inner"))
            .await?;
        finished_tasks.push(task);
    }

    Ok(finished_tasks)
}

#[instrument(skip_all)]
pub(super) async fn get_test_execution(
    con: &mut SqliteConnection,
    compiler_exec_id: &str,
    binary_exec_id: Option<String>,
    test_type: TestExecutionOutputType,
) -> Result<TestExecutionOutput> {
    let compiler_exec = get_execution(con, compiler_exec_id).await?;
    let binary_exec = match binary_exec_id {
        Some(id) => Some(get_execution(con, &id).await?),
        None => None,
    };

    Ok(test_type.to_test_execution(compiler_exec, binary_exec))
}

#[instrument(skip_all)]
async fn get_execution(con: &mut SqliteConnection, execution_id: &str) -> Result<ExecutionOutput> {
    let Some(execution) = fetch_execution(con, execution_id).await? else {
        return Err(WebError::not_found(location!()));
    };

    Ok(execution)
}

#[instrument(skip_all)]
async fn fetch_execution(
    con: &mut SqliteConnection,
    execution_id: &str,
) -> Result<Option<ExecutionOutput>> {
    let execution = query!(
        r#"
        SELECT
            execution_id,
            stdout,
            stderr,
            accumulated_errors,
            error,
            result as "result!: ExecutionExitStatus",
            duration_ms as "duration_ms!: u64",
            exit_code as "exit_code?: i32"
        FROM ExecutionResults
        WHERE execution_id = ?"#,
        execution_id
    )
    .fetch_optional(con)
    .instrument(info_span!("sqlx_get_execution"))
    .await
    .context(SqlxSnafu)?;

    let Some(execution) = execution else {
        return Ok(None);
    };

    Ok(Some(match execution.result {
        ExecutionExitStatus::Aborted => ExecutionOutput::Aborted(AbortedExecution {
            stdout: execution.stdout,
            stderr: execution.stderr,
            runtime: Duration::from_millis(execution.duration_ms),
        }),
        ExecutionExitStatus::Error => ExecutionOutput::Error(InternalError {
            message: execution.error.unwrap_or("N/A".to_string()),
            runtime: Duration::from_millis(execution.duration_ms),
        }),
        ExecutionExitStatus::Failure => ExecutionOutput::Failure {
            execution: FinishedExecution {
                stdout: execution.stdout,
                stderr: execution.stderr,
                runtime: Duration::from_millis(execution.duration_ms),
                exit_status: execution.exit_code,
            },
            accumulated_errors: execution.accumulated_errors,
        },
        ExecutionExitStatus::Success => ExecutionOutput::Success(FinishedExecution {
            stdout: execution.stdout,
            stderr: execution.stderr,
            runtime: Duration::from_millis(execution.duration_ms),
            exit_status: execution.exit_code,
        }),
        ExecutionExitStatus::Timeout => ExecutionOutput::Timeout(FinishedExecution {
            stdout: execution.stdout,
            stderr: execution.stderr,
            runtime: Duration::from_millis(execution.duration_ms),
            exit_status: execution.exit_code,
        }),
    }))
}

#[instrument(skip_all)]
pub(super) async fn record_test_execution(
    con: &mut SqliteConnection,
    e: &TestExecutionOutput,
) -> Result<(String, Option<String>)> {
    let compiler_exec_id = record_execution_output(con, e.compiler_output()).await?;
    let binary_exec_id = match &e.binary_output() {
        Some(output) => Some(record_execution_output(con, output).await?),
        None => None,
    };

    Ok((compiler_exec_id, binary_exec_id))
}

#[instrument(skip_all)]
pub(super) async fn record_execution_output(
    con: &mut SqliteConnection,
    e: &ExecutionOutput,
) -> Result<String> {
    let execution_id = uuid::Uuid::new_v4().to_string();

    match e {
        ExecutionOutput::Aborted(e) => record_aborted(con, &execution_id, e).await?,
        ExecutionOutput::Error(e) => record_internal_error(con, &execution_id, e).await?,
        ExecutionOutput::Failure {
            execution,
            accumulated_errors,
        } => {
            record_finished_execution(
                con,
                &execution_id,
                execution,
                accumulated_errors,
                ExecutionExitStatus::Failure,
            )
            .await?
        }
        ExecutionOutput::Success(e) => {
            record_finished_execution(con, &execution_id, e, &None, ExecutionExitStatus::Success)
                .await?
        }
        ExecutionOutput::Timeout(e) => {
            record_finished_execution(con, &execution_id, e, &None, ExecutionExitStatus::Timeout)
                .await?
        }
    }

    Ok(execution_id)
}

#[instrument(skip_all)]
async fn record_task(
    con: &mut SqliteConnection,
    result: &FinishedCompilerTask,
    start_time: i64,
    end_time: i64,
    build_id: &str,
    queue_time: i64,
) -> Result<()> {
    query!(
        r#"
        INSERT INTO Tasks
            (task_id, team_id, revision, commit_message, start_time, end_time, execution_id,
             queue_time)
        VALUES
            (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        result.info().task_id,
        result.info().team_id,
        result.info().revision_id,
        result.info().commit_message,
        start_time,
        end_time,
        build_id,
        queue_time
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_add_finished_insert_task"))
    .await
    .context(SqlxSnafu)?;

    Ok(())
}

#[instrument(skip_all)]
async fn record_finished_execution(
    con: &mut SqliteConnection,
    execution_id: &str,
    e: &FinishedExecution,
    accumulated_errors: &Option<String>,
    status: ExecutionExitStatus,
) -> Result<()> {
    let runtime = e.runtime.as_millis() as i64;

    query!(
        "INSERT INTO ExecutionResults
            (execution_id, stdout, stderr, error, accumulated_errors, result, duration_ms, exit_code)
         VALUES
            (?, ?, ?, ?, ?, ?, ?, ?)
        ",
        execution_id,
        e.stdout,
        e.stderr,
        None::<&str>,
        accumulated_errors,
        status,
        runtime,
        e.exit_status
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_record_finished_execution"))
    .await
    .context(SqlxSnafu)?;
    Ok(())
}

#[instrument(skip_all)]
async fn record_internal_error(
    con: &mut SqliteConnection,
    execution_id: &str,
    e: &InternalError,
) -> Result<()> {
    let runtime = e.runtime.as_millis() as i64;
    query!(
        "INSERT INTO ExecutionResults
            (execution_id, stdout, stderr, error, result, duration_ms, exit_code)
         VALUES
            (?, ?, ?, ?, ?, ?, ?)
        ",
        execution_id,
        "",
        "",
        e.message,
        ExecutionExitStatus::Error,
        runtime,
        None::<i32>
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_record_internal_error"))
    .await
    .context(SqlxSnafu)?;
    Ok(())
}

#[instrument(skip_all)]
async fn record_aborted(
    con: &mut SqliteConnection,
    execution_id: &str,
    e: &AbortedExecution,
) -> Result<()> {
    let runtime = e.runtime.as_millis() as i64;
    query!(
        "INSERT INTO ExecutionResults
            (execution_id, stdout, stderr, error, result, duration_ms, exit_code)
        VALUES
            (?, ?, ?, ?, ?, ?, ?)
        ",
        execution_id,
        e.stdout,
        e.stderr,
        None::<&str>,
        ExecutionExitStatus::Aborted,
        runtime,
        None::<i32>
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_record_aborted"))
    .await
    .context(SqlxSnafu)?;
    Ok(())
}

#[instrument(skip_all)]
pub(super) async fn get_top_task_per_team(
    con: impl Acquire<'_, Database = Sqlite>,
) -> Result<HashMap<TeamId, FinishedCompilerTaskSummary>> {
    let mut con = con.begin().await.context(SqlxSnafu)?;

    let query_res = query!(
        r#"
        WITH pass_by_task AS (
            SELECT
                Tasks.task_id as "task_id",
                Tasks.team_id as "team_id",
                COUNT(test_id) as "passed_count"
            FROM TestResults
            JOIN Tasks ON Tasks.task_id = TestResults.task_id
            WHERE TestResults.status = ?
            GROUP BY Tasks.task_id
        )
        SELECT
            pass_by_task.team_id as "team_id!: TeamId",
            pass_by_task.task_id as "task_id!: TaskId",
            -- Unused max to force SQLite to return extremal values for the other columns
            MAX(pass_by_task.passed_count) as "passes!: i64"
        FROM pass_by_task
        GROUP BY pass_by_task.team_id;
        "#,
        ExecutionExitStatus::Success,
    )
    .fetch_all(&mut *con)
    .instrument(info_span!("sqlx_get_top_task_per_team"))
    .await
    .context(SqlxSnafu)?;

    let mut result = HashMap::new();

    for row in query_res {
        let task = get_task_summary(&mut con, &row.task_id)
            .instrument(info_span!("sqlx_get_top_task_per_team_inner"))
            .await?;
        result.insert(row.team_id, task);
    }

    Ok(result)
}

#[instrument(skip_all)]
async fn get_task_summary(
    con: &mut SqliteConnection,
    task_id: &TaskId,
) -> Result<FinishedCompilerTaskSummary> {
    let task = query!(
        r#"
        SELECT
            task_id as "task_id!: TaskId",
            start_time as "start_time!: u64",
            end_time as "end_time!: u64",
            team_id as "team_id!: TeamId",
            revision as "revision_id!: String",
            commit_message as "commit_message!: String",
            execution_id as "execution_id!: String",
            (
                SELECT result FROM ExecutionResults ER WHERE ER.execution_id = Tasks.execution_id
            ) as "build_result!: ExecutionExitStatus"
        FROM Tasks
        WHERE task_id = ?
        "#,
        task_id
    )
    .fetch_optional(&mut *con)
    .instrument(info_span!("sqlx_get_task_summary_query"))
    .await
    .context(SqlxSnafu)?;

    let Some(task) = task else {
        return Err(WebError::not_found(location!()));
    };

    let start = SystemTime::UNIX_EPOCH.add(Duration::from_millis(task.start_time));
    let end = SystemTime::UNIX_EPOCH.add(Duration::from_millis(task.end_time));
    let info = FinishedTaskInfo {
        task_id: task_id.to_string(),
        start,
        end,
        revision_id: task.revision_id,
        commit_message: task.commit_message,
        team_id: task.team_id.to_string(),
    };

    if task.build_result != ExecutionExitStatus::Success {
        return Ok(FinishedCompilerTaskSummary::BuildFailed {
            info,
            status: task.build_result,
        });
    }

    let tests = query!(
        r#"
        SELECT
            test_id as "test_id!: TestId",
            (
                SELECT result FROM ExecutionResults
                WHERE execution_id = binary_exec_id
            ) as "binary_status?: ExecutionExitStatus",
            (
                SELECT result FROM ExecutionResults
                WHERE execution_id = compiler_exec_id
            ) as "compiler_status!: ExecutionExitStatus",
            provisional_for_category as "provisional_for_category?"
        FROM TestResults
        WHERE task_id = ?"#,
        task_id
    )
    .map(|it| FinishedTestSummary {
        test_id: it.test_id,
        output: it.binary_status.unwrap_or(it.compiler_status),
        provisional_for_category: it.provisional_for_category,
    })
    .fetch_all(&mut *con)
    .instrument(info_span!("sqlx_get_task_summary_tests"))
    .await
    .context(SqlxSnafu)?;

    let statistics = tests.as_slice().into();
    Ok(FinishedCompilerTaskSummary::RanTests {
        info,
        outdated: get_outdated_tests(con, task_id).await?,
        statistics,
    })
}

async fn get_outdated_tests(con: &mut SqliteConnection, task_id: &TaskId) -> Result<Vec<TestId>> {
    query!(
        r#"
        SELECT test_id as "test_id!: TestId"
        FROM TestResults
        JOIN Tests ON Tests.id = TestResults.test_id
        JOIN Tasks ON Tasks.task_id = TestResults.task_id
        WHERE Tasks.task_id = ? AND Tests.last_updated > Tasks.queue_time
        "#,
        task_id
    )
    .map(|it| it.test_id)
    .fetch_all(&mut *con)
    .instrument(info_span!("sqlx_get_task_outdated_tests"))
    .await
    .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn get_final_submitted_task(
    con: impl Acquire<'_, Database = Sqlite>,
    team_id: &TeamId,
    category: &str,
    meta: &TestCategory,
    respect_finalized: bool,
) -> Result<Option<FinalSubmittedTask>> {
    let mut con = con.begin().await.context(SqlxSnafu)?;

    if respect_finalized {
        let finalized = fetch_finalized_task(&mut con, team_id, category)
            .instrument(info_span!("sqlx_get_final_submitted_task"))
            .await?;

        if let Some(summary) = finalized {
            return Ok(Some(FinalSubmittedTask::Finalized { summary }));
        }
    }

    let manual_task = query!(
        r#"
        SELECT
            task_id as "task_id!: TaskId",
            user_id as "user_id!: UserId",
            update_time
        FROM ManuallySubmittedTasks
        WHERE team_id = ? AND category = ? AND task_id IS NOT NULL
        "#,
        team_id,
        category
    )
    .fetch_optional(&mut *con)
    .instrument(info_span!("sqlx_get_final_submitted_task"))
    .await
    .context(SqlxSnafu)?;

    if let Some(override_task) = manual_task {
        let summary = get_task_summary(&mut con, &override_task.task_id)
            .instrument(info_span!("sqlx_get_final_submitted_task_inner"))
            .await?;

        return Ok(Some(FinalSubmittedTask::ManuallyOverridden {
            summary,
            user_id: override_task.user_id,
            time: override_task.update_time,
        }));
    }

    let task = get_top_task_for_team_and_category(&mut con, team_id, category, meta)
        .instrument(info_span!("sqlx_get_final_submitted_task_inner"))
        .await?;

    Ok(task.map(|summary| FinalSubmittedTask::AutomaticallySelected { summary }))
}

#[instrument(skip_all)]
pub(super) async fn set_final_submitted_task(
    con: impl Acquire<'_, Database = Sqlite>,
    team_id: &TeamId,
    user_id: &UserId,
    task_id: &TaskId,
    categories: impl Iterator<Item = &str>,
) -> Result<()> {
    let mut con = con.begin().await.context(SqlxSnafu)?;

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64;

    // Clear previous manual overrides for the same task
    query!(
        "DELETE FROM ManuallySubmittedTasks WHERE team_id = ? AND task_id = ?",
        team_id,
        task_id
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_set_final_submitted_task_delete"))
    .await
    .context(SqlxSnafu)?;

    // Insert new manual overrides
    for category in categories {
        query!(
            r#"
            INSERT INTO ManuallySubmittedTasks
                (team_id, category, task_id, user_id, update_time)
            VALUES
                (?, ?, ?, ?, ?)
            ON CONFLICT DO UPDATE SET
                task_id = excluded.task_id,
                user_id = excluded.user_id,
                update_time = excluded.update_time
            "#,
            team_id,
            category,
            task_id,
            user_id,
            current_time
        )
        .execute(&mut *con)
        .instrument(info_span!("sqlx_set_final_submitted_task"))
        .await
        .context(SqlxSnafu)?;
    }

    con.commit().await.context(SqlxSnafu)?;

    Ok(())
}

#[instrument(skip_all)]
async fn get_top_task_for_team_and_category(
    con: &mut SqliteConnection,
    team_id: &TeamId,
    category: &str,
    meta: &TestCategory,
) -> Result<Option<FinishedCompilerTaskSummary>> {
    let starts_at = meta.starts_at.timestamp().as_millisecond();
    let ends_at = meta.labs_end_at.timestamp().as_millisecond();

    let query_res = query!(
        r#"
        -- noinspection SqlAggregates
        -- We group by the primary key of Tasks, there will never be two differing
        -- queue_time values. SQLite will non-deterministically pick one of the copies.
        SELECT Tasks.task_id as "task_id!: TaskId"
        FROM TestResults
        JOIN Tasks ON Tasks.task_id = TestResults.task_id
        JOIN Tests ON Tests.id = TestResults.test_id
        WHERE
                Tasks.team_id = ?
            AND TestResults.status = ?
            AND Tasks.queue_time BETWEEN ? AND ?
            AND Tests.category = ?
            AND (Tests.provisional_for_category IS NULL OR Tests.provisional_for_category != ?)
        GROUP BY Tasks.task_id
        ORDER BY COUNT(test_id) DESC, Tasks.queue_time DESC
        LIMIT 1
        "#,
        team_id,
        ExecutionExitStatus::Success,
        starts_at,
        ends_at,
        category,
        category,
    )
    .fetch_optional(&mut *con)
    .instrument(info_span!("sqlx_get_top_task_for_team_and_category"))
    .await
    .context(SqlxSnafu)?;

    let query_res = match query_res {
        Some(row) => row,
        None => return Ok(None),
    };

    Ok(Some(get_task(con, &query_res.task_id).await?.into()))
}

pub(super) async fn finalize_submission(
    con: &mut SqliteConnection,
    team_id: &TeamId,
    task_id: &TaskId,
    category: &str,
) -> Result<()> {
    query!(
        r#"
        INSERT INTO FinalizedSubmittedTasks
            (team_id, task_id, category)
        VALUES
            (?, ?, ?)
        ON CONFLICT DO UPDATE SET
            task_id = excluded.task_id
        "#,
        team_id,
        task_id,
        category
    )
    .execute(con)
    .instrument(info_span!("sqlx_finalize_submission"))
    .await
    .context(SqlxSnafu)?;

    Ok(())
}

#[instrument(skip_all)]
pub(super) async fn fetch_finalized_task(
    con: &mut SqliteConnection,
    team_id: &TeamId,
    category: &str,
) -> Result<Option<FinishedCompilerTaskSummary>> {
    let task_id = query!(
        r#"
        SELECT task_id as "task_id: TaskId"
        FROM FinalizedSubmittedTasks
        WHERE team_id = ? AND category = ?
        "#,
        team_id,
        category
    )
    .map(|it| it.task_id)
    .fetch_optional(&mut *con)
    .instrument(info_span!("sqlx_get_finalized_task"))
    .await
    .context(SqlxSnafu)?;

    if let Some(task_id) = task_id {
        let summary = get_task_summary(con, &task_id)
            .instrument(info_span!("sqlx_get_finalized_task_inner"))
            .await?;
        Ok(Some(summary))
    } else {
        Ok(None)
    }
}
