use crate::error::{Result, SqlxSnafu};
use crate::types::{CreatedExternalRun, ExternalRunId, ExternalRunStatus, TaskId};
use snafu::ResultExt;
use sqlx::{query, query_as, SqliteConnection};
use tracing::{info_span, Instrument};

pub(super) async fn add_external_run(
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
    .await
    .context(SqlxSnafu)?;

    Ok(())
}

pub(super) async fn update_external_run_status(
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
    .await
    .context(SqlxSnafu)?;

    Ok(res.rows_affected() > 0)
}

pub(super) async fn get_external_runs(
    con: &mut SqliteConnection,
    platform: &str,
) -> Result<Vec<CreatedExternalRun>> {
    query_as!(
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
    .await
    .context(SqlxSnafu)
}

pub(super) async fn delete_external_run(
    con: &mut SqliteConnection,
    run_id: &ExternalRunId,
) -> Result<bool> {
    let run_id = **run_id as i64;
    let res = query!("DELETE FROM ExternalRuns WHERE run_id = ?", run_id)
        .execute(con)
        .await
        .context(SqlxSnafu)?;

    Ok(res.rows_affected() > 0)
}

pub(super) async fn add_external_run_revision_mapping(
    con: &mut SqliteConnection,
    task_id: &TaskId,
    to_revision: &str,
) -> Result<()> {
    query!(
        r#"
        INSERT INTO ExternalRunRevisionMappings
            (task_id, revision)
        VALUES
            (?, ?)
        ON CONFLICT DO UPDATE SET
            revision = excluded.revision
        "#,
        task_id,
        to_revision
    )
    .execute(con)
    .instrument(info_span!("sqlx_add_external_run_revision_mapping"))
    .await
    .context(SqlxSnafu)?;

    Ok(())
}

pub(super) async fn fetch_external_run_revision_mapping(
    con: &mut SqliteConnection,
    task_id: &TaskId,
) -> Result<Option<String>> {
    query!(
        "SELECT revision FROM ExternalRunRevisionMappings WHERE task_id = ?",
        task_id
    )
    .map(|it| it.revision)
    .fetch_optional(con)
    .instrument(info_span!("sqlx_fetch_external_run_revision_mapping"))
    .await
    .context(SqlxSnafu)
}

pub(super) async fn delete_external_run_revision_mapping(
    con: &mut SqliteConnection,
    task_id: &TaskId,
) -> Result<()> {
    query!(
        "DELETE FROM ExternalRunRevisionMappings WHERE task_id = ?",
        task_id
    )
    .execute(con)
    .instrument(info_span!("sqlx_delete_external_run_revision_mapping"))
    .await
    .context(SqlxSnafu)?;

    Ok(())
}
