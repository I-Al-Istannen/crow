use crate::error::{Result, SqlxSnafu};
use crate::types::{TaskId, TeamId, WorkItem};
use snafu::ResultExt;
use sqlx::{query, SqliteConnection};
use std::ops::Add;
use std::time::{Duration, SystemTime};
use tracing::{info_span, instrument, Instrument};

#[instrument(skip_all)]
pub(super) async fn queue_task(con: &mut SqliteConnection, task: WorkItem) -> Result<()> {
    let insert_time = task
        .insert_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as i64;

    query!(
        "INSERT INTO Queue (id, team, revision, commit_message, insert_time) VALUES (?, ?, ?, ?, ?)",
        task.id,
        task.team,
        task.revision,
        task.commit_message,
        insert_time
    )
    .execute(con)
    .instrument(info_span!("sqlx_insert_queue"))
    .await.context(SqlxSnafu)?;

    Ok(())
}

#[instrument(skip_all)]
pub(super) async fn remove_queued_task(con: &mut SqliteConnection, task: &TaskId) -> Result<()> {
    query!("DELETE FROM Queue WHERE id = ?", task)
        .execute(con)
        .instrument(info_span!("sqlx_delete_queue"))
        .await
        .context(SqlxSnafu)?;

    Ok(())
}

#[instrument(skip_all)]
pub(super) async fn get_queued_tasks(con: &mut SqliteConnection) -> Result<Vec<WorkItem>> {
    query!(
        r#"
        SELECT
            id as "id!: TaskId",
            team as "team!: TeamId",
            revision,
            commit_message,
            insert_time as "insert_time!: u64"
        FROM Queue"#
    )
    .map(|row| WorkItem {
        id: row.id,
        team: row.team,
        revision: row.revision,
        commit_message: row.commit_message,
        insert_time: SystemTime::UNIX_EPOCH.add(Duration::from_millis(row.insert_time)),
    })
    .fetch_all(con)
    .instrument(info_span!("sqlx_get_queue"))
    .await
    .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn fetch_queued_task(
    con: &mut SqliteConnection,
    task_id: &TaskId,
) -> Result<Option<WorkItem>> {
    query!(
        r#"
        SELECT
            id as "id!: TaskId",
            team as "team!: TeamId",
            revision,
            commit_message,
            insert_time as "insert_time!: u64"
        FROM Queue
        WHERE id = ?
        "#,
        task_id
    )
    .map(|row| WorkItem {
        id: row.id,
        team: row.team,
        revision: row.revision,
        commit_message: row.commit_message,
        insert_time: SystemTime::UNIX_EPOCH.add(Duration::from_millis(row.insert_time)),
    })
    .fetch_optional(con)
    .instrument(info_span!("sqlx_get_queued_task"))
    .await
    .context(SqlxSnafu)
}
