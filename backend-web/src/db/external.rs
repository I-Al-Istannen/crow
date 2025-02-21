use crate::error::Result;
use crate::types::{CreatedExternalRun, ExternalRunId, ExternalRunStatus, TaskId};
use sqlx::{SqliteConnection, query, query_as};

pub(crate) async fn add_external_run(
    con: &mut SqliteConnection,
    run: &CreatedExternalRun,
) -> Result<()> {
    let run_id = *run.run_id as i64;
    query!(
        r#"
        INSERT INTO ExternalRuns
            (task_id, run_id, platform, owner, repo, revision, status)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT DO UPDATE SET
            status = excluded.status
        "#,
        run.task_id,
        run_id,
        run.platform,
        run.owner,
        run.repo,
        run.revision,
        run.status,
    )
    .execute(con)
    .await?;

    Ok(())
}

pub(crate) async fn update_external_run_status(
    con: &mut SqliteConnection,
    run_id: &ExternalRunId,
    status: ExternalRunStatus,
) -> Result<bool> {
    let run_id = **run_id as i64;
    let res = query!(
        "UPDATE ExternalRuns SET status = ? WHERE run_id = ?",
        status,
        run_id
    )
    .execute(con)
    .await?;

    Ok(res.rows_affected() > 0)
}

pub(crate) async fn get_external_runs(
    con: &mut SqliteConnection,
    platform: &str,
) -> Result<Vec<CreatedExternalRun>> {
    Ok(query_as!(
        CreatedExternalRun,
        r#"
        SELECT
           task_id as "task_id!: TaskId",
           run_id as "run_id!: ExternalRunId",
           platform,
           owner,
           repo,
           revision,
           status as "status!: ExternalRunStatus"
        FROM ExternalRuns WHERE platform = ?
        "#,
        platform
    )
    .fetch_all(con)
    .await?)
}

pub(crate) async fn delete_external_run(
    con: &mut SqliteConnection,
    run_id: &ExternalRunId,
) -> Result<bool> {
    let run_id = **run_id as i64;
    let res = query!("DELETE FROM ExternalRuns WHERE run_id = ?", run_id)
        .execute(con)
        .await?;

    Ok(res.rows_affected() > 0)
}
