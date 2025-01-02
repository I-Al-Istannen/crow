use crate::error::{Result, WebError};
use crate::types::{ExecutionExitStatus, FinishedCompilerTaskSummary, TaskId, TeamId};
use shared::{
    AbortedExecution, ExecutionOutput, FinishedCompilerTask, FinishedExecution, FinishedTaskInfo,
    FinishedTest, InternalError,
};
use sqlx::{query, Acquire, Sqlite, SqliteConnection};
use std::ops::Add;
use std::time::{Duration, SystemTime};
use tracing::{info_span, instrument, Instrument};

#[instrument(skip_all)]
pub(super) async fn add_finished_task(
    con: impl Acquire<'_, Database = Sqlite>,
    task_id: &TaskId,
    result: &FinishedCompilerTask,
) -> Result<()> {
    let mut con = con.begin().await?;

    query!("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *con)
        .instrument(info_span!("sqlx_add_finished_pragma"))
        .await?;

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

    let build_id = uuid::Uuid::new_v4().to_string();

    query!(
        r#"
        INSERT INTO Tasks
            (task_id, team_id, revision, start_time, end_time, execution_id)
        VALUES
            (?, ?, ?, ?, ?, ?)
        "#,
        task_id,
        result.info().team_id,
        result.info().revision_id,
        start_time,
        end_time,
        build_id
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_add_finished_insert_task"))
    .await?;

    match result {
        FinishedCompilerTask::BuildFailed { build_output, .. } => {
            record_execution_output(&mut con, &build_id, build_output).await?;
        }
        FinishedCompilerTask::RanTests {
            tests,
            build_output,
            ..
        } => {
            record_finished_execution(
                &mut con,
                &build_id,
                build_output,
                ExecutionExitStatus::Finished,
            )
            .await?;

            for test in tests {
                let test_exec_id = uuid::Uuid::new_v4().to_string();

                record_execution_output(&mut con, &test_exec_id, &test.output).await?;

                query!(
                    "INSERT INTO TestResults (task_id, test_id, execution_id) VALUES (?, ?, ?)",
                    task_id,
                    test.test_id,
                    test_exec_id
                )
                .execute(&mut *con)
                .instrument(info_span!("sqlx_add_finished_insert_test"))
                .await?;
            }
        }
    }

    con.commit().await?;

    Ok(())
}

#[instrument(skip_all)]
pub(super) async fn get_task(
    con: impl Acquire<'_, Database = Sqlite>,
    task_id: &TaskId,
) -> Result<FinishedCompilerTask> {
    let mut con = con.begin().await?;

    let task = query!(
        r#"
        SELECT
            task_id as "task_id!: TaskId",
            start_time as "start_time!: u64",
            end_time as "end_time!: u64",
            team_id as "team_id!: TeamId",
            revision as "revision_id!: String",
            execution_id as "execution_id!: String"
        FROM Tasks
        WHERE task_id = ?
        "#,
        task_id
    )
    .fetch_optional(&mut *con)
    .instrument(info_span!("sqlx_get_task_query"))
    .await?;

    let Some(task) = task else {
        return Err(WebError::NotFound);
    };

    let build_output = get_execution(&mut con, &task.execution_id).await?;
    let start = SystemTime::UNIX_EPOCH.add(Duration::from_millis(task.start_time));
    let end = SystemTime::UNIX_EPOCH.add(Duration::from_millis(task.end_time));
    let info = FinishedTaskInfo {
        task_id: task_id.to_string(),
        start,
        end,
        revision_id: task.revision_id,
        team_id: task.team_id.to_string(),
    };

    if !build_output.produced_results() {
        return Ok(FinishedCompilerTask::BuildFailed { info, build_output });
    }

    let tests = query!(
        r#"
        SELECT
            test_id,
            ExecutionResults.execution_id as "execution_id!",
            stdout,
            stderr,
            error,
            result,
            duration_ms,
            exit_code
        FROM TestResults
        JOIN ExecutionResults ON TestResults.execution_id = ExecutionResults.execution_id
        WHERE task_id = ?"#,
        task_id
    )
    .fetch_all(&mut *con)
    .instrument(info_span!("sqlx_get_task_tests"))
    .await?;

    let mut finished_tests = Vec::new();
    for test in tests {
        let test_id = test.test_id;
        let output = get_execution(&mut con, &test.execution_id).await?;
        finished_tests.push(FinishedTest { test_id, output })
    }

    Ok(FinishedCompilerTask::RanTests {
        info,
        build_output: build_output.into_finished_execution().unwrap(),
        tests: finished_tests,
    })
}

pub(super) async fn get_recent_tasks(
    con: impl Acquire<'_, Database = Sqlite>,
    team_id: &TeamId,
    count: i64,
) -> Result<Vec<FinishedCompilerTaskSummary>> {
    let mut con = con.begin().await?;

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
    .await?;

    let mut finished_tasks = Vec::new();

    for task in tasks {
        let task = get_task(&mut con, &task)
            .instrument(info_span!("sqlx_get_recent_tasks_inner"))
            .await?;
        finished_tasks.push(task.into());
    }

    Ok(finished_tasks)
}

#[instrument(skip_all)]
pub(super) async fn get_task_ids(con: &mut SqliteConnection) -> Result<Vec<TaskId>> {
    Ok(query!(r#"SELECT task_id as "task_id!: TaskId" FROM Tasks"#)
        .map(|it| it.task_id)
        .fetch_all(con)
        .instrument(info_span!("sqlx_get_task_ids"))
        .await?)
}

#[instrument(skip_all)]
async fn get_execution(con: &mut SqliteConnection, execution_id: &str) -> Result<ExecutionOutput> {
    let execution = query!(
        r#"
        SELECT 
            execution_id,
            stdout,
            stderr,
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
    .await?;

    let Some(execution) = execution else {
        return Err(WebError::NotFound);
    };

    Ok(match execution.result {
        ExecutionExitStatus::Aborted => ExecutionOutput::Aborted(AbortedExecution {
            stdout: execution.stdout,
            stderr: execution.stderr,
            runtime: Duration::from_millis(execution.duration_ms),
        }),
        ExecutionExitStatus::Error => ExecutionOutput::Error(InternalError {
            message: execution.error.unwrap_or("N/A".to_string()),
            runtime: Duration::from_millis(execution.duration_ms),
        }),
        ExecutionExitStatus::Finished => ExecutionOutput::Finished(FinishedExecution {
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
    })
}

#[instrument(skip_all)]
async fn record_execution_output(
    con: &mut SqliteConnection,
    execution_id: &str,
    e: &ExecutionOutput,
) -> Result<()> {
    match e {
        ExecutionOutput::Aborted(e) => record_aborted(con, execution_id, e).await?,
        ExecutionOutput::Error(e) => record_internal_error(con, execution_id, e).await?,
        ExecutionOutput::Finished(e) => {
            record_finished_execution(con, execution_id, e, ExecutionExitStatus::Finished).await?
        }
        ExecutionOutput::Timeout(e) => {
            record_finished_execution(con, execution_id, e, ExecutionExitStatus::Timeout).await?
        }
    }

    Ok(())
}

#[instrument(skip_all)]
async fn record_finished_execution(
    con: &mut SqliteConnection,
    execution_id: &str,
    e: &FinishedExecution,
    status: ExecutionExitStatus,
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
        status,
        runtime,
        e.exit_status
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_record_finished_execution"))
    .await?;
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
    .await?;
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
    .await?;
    Ok(())
}
