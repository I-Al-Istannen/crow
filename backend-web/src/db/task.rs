use crate::error::WebError;
use crate::types::{ExecutionExitStatus, TaskId};
use shared::{
    AbortedExecution, ExecutionOutput, FinishedCompilerTask, FinishedExecution, FinishedTest,
    InternalError,
};
use sqlx::{query, Acquire, Sqlite, SqliteConnection};
use std::ops::Add;
use std::time::{Duration, SystemTime};

pub async fn add_finished_task(
    con: impl Acquire<'_, Database = Sqlite>,
    task_id: &TaskId,
    result: &FinishedCompilerTask,
) -> Result<(), WebError> {
    let mut con = con.begin().await?;

    query!("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *con)
        .await?;

    let start_time = result
        .start_time()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64;

    let build_id = uuid::Uuid::new_v4().to_string();

    query!(
        "INSERT INTO Tasks (task_id, start_time, execution_id) VALUES (?, ?, ?)",
        task_id,
        start_time,
        build_id
    )
    .execute(&mut *con)
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
                .await?;
            }
        }
    }

    con.commit().await?;

    Ok(())
}

pub async fn get_task(
    con: impl Acquire<'_, Database = Sqlite>,
    task_id: &TaskId,
) -> Result<FinishedCompilerTask, WebError> {
    let mut con = con.begin().await?;

    let task = query!(
        r#"
        SELECT
            task_id as "task_id!: TaskId",
            start_time as "start_time!: u64",
            execution_id as "execution_id!: String"
        FROM Tasks
        WHERE task_id = ?
        "#,
        task_id
    )
    .fetch_optional(&mut *con)
    .await?;

    let Some(task) = task else {
        return Err(WebError::NotFound);
    };

    let build_output = get_execution(&mut con, &task.execution_id).await?;
    let start = SystemTime::UNIX_EPOCH.add(Duration::from_millis(task.start_time));

    if !build_output.produced_results() {
        return Ok(FinishedCompilerTask::BuildFailed {
            start,
            build_output,
        });
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
    .await?;

    let mut finished_tests = Vec::new();
    for test in tests {
        let test_id = test.test_id;
        let output = get_execution(&mut con, &test.execution_id).await?;
        finished_tests.push(FinishedTest { test_id, output })
    }

    Ok(FinishedCompilerTask::RanTests {
        start,
        build_output: build_output.into_finished_execution().unwrap(),
        tests: finished_tests,
    })
}

pub async fn get_task_ids(con: &mut SqliteConnection) -> Result<Vec<TaskId>, WebError> {
    Ok(query!(r#"SELECT task_id as "task_id!: TaskId" FROM Tasks"#)
        .map(|it| it.task_id)
        .fetch_all(con)
        .await?)
}

async fn get_execution(
    con: &mut SqliteConnection,
    execution_id: &str,
) -> Result<ExecutionOutput, WebError> {
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

async fn record_execution_output(
    con: &mut SqliteConnection,
    execution_id: &str,
    e: &ExecutionOutput,
) -> Result<(), WebError> {
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

async fn record_finished_execution(
    con: &mut SqliteConnection,
    execution_id: &str,
    e: &FinishedExecution,
    status: ExecutionExitStatus,
) -> Result<(), WebError> {
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
    .await?;
    Ok(())
}

async fn record_internal_error(
    con: &mut SqliteConnection,
    execution_id: &str,
    e: &InternalError,
) -> Result<(), WebError> {
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
    .await?;
    Ok(())
}

async fn record_aborted(
    con: &mut SqliteConnection,
    execution_id: &str,
    e: &AbortedExecution,
) -> Result<(), WebError> {
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
    .await?;
    Ok(())
}
