use crate::error::WebError;
use crate::types::{ExecutionExitStatus, TaskId};
use shared::{
    AbortedExecution, ExecutionOutput, FinishedCompilerTask, FinishedExecution, InternalError,
};
use sqlx::{query, Acquire, Sqlite, SqliteConnection};
use std::time::Duration;

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
            }
        }
    }

    con.commit().await?;

    Ok(())
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
        None::<&str>,
        None::<&str>,
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
