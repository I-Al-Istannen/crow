use crate::error::WebError;
use crate::types::{TaskId, TeamId, WorkItem};
use sqlx::{query, SqliteConnection};

pub async fn queue_task(con: &mut SqliteConnection, task: WorkItem) -> Result<(), WebError> {
    query!(
        "INSERT INTO Queue (id, team, revision) VALUES (?, ?, ?)",
        task.id,
        task.team,
        task.revision,
    )
    .execute(con)
    .await?;

    Ok(())
}

pub async fn remove_queued_task(con: &mut SqliteConnection, task: &TaskId) -> Result<(), WebError> {
    query!("DELETE FROM Queue WHERE id = ?", task)
        .execute(con)
        .await?;

    Ok(())
}

pub async fn get_queued_tasks(con: &mut SqliteConnection) -> Result<Vec<WorkItem>, WebError> {
    let tasks =
        query!(r#"SELECT id as "id!: TaskId", team as "team!: TeamId", revision FROM Queue"#)
            .fetch_all(con)
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
