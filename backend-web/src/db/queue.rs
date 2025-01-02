use crate::error::Result;
use crate::types::{TaskId, TeamId, WorkItem};
use sqlx::{query, SqliteConnection};
use tracing::{info_span, instrument, Instrument};

#[instrument(skip_all)]
pub(super) async fn queue_task(con: &mut SqliteConnection, task: WorkItem) -> Result<()> {
    query!(
        "INSERT INTO Queue (id, team, revision) VALUES (?, ?, ?)",
        task.id,
        task.team,
        task.revision,
    )
    .execute(con)
    .instrument(info_span!("sqlx_insert_queue"))
    .await?;

    Ok(())
}

#[instrument(skip_all)]
pub(super) async fn remove_queued_task(con: &mut SqliteConnection, task: &TaskId) -> Result<()> {
    query!("DELETE FROM Queue WHERE id = ?", task)
        .execute(con)
        .instrument(info_span!("sqlx_delete_queue"))
        .await?;

    Ok(())
}

#[instrument(skip_all)]
pub(super) async fn get_queued_tasks(con: &mut SqliteConnection) -> Result<Vec<WorkItem>> {
    let tasks =
        query!(r#"SELECT id as "id!: TaskId", team as "team!: TeamId", revision FROM Queue"#)
            .fetch_all(con)
            .instrument(info_span!("sqlx_get_queue"))
            .await?;

    Ok(tasks
        .into_iter()
        .map(|task| WorkItem {
            id: task.id,
            team: task.team,
            revision: task.revision,
        })
        .collect())
}
