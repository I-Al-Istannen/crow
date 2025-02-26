use crate::error::{Result, SqlxSnafu, WebError};
use crate::types::{Repo, TeamId};
use snafu::{location, ResultExt};
use sqlx::{query, query_as, Acquire, Sqlite, SqliteConnection};
use tracing::{info_span, instrument, Instrument};

#[instrument(skip_all)]
pub(super) async fn fetch_repo(
    con: &mut SqliteConnection,
    team_id: &TeamId,
) -> Result<Option<Repo>> {
    query_as!(Repo, r#"SELECT * FROM Repos WHERE team = ?"#, team_id)
        .fetch_optional(con)
        .instrument(info_span!("sqlx_get_repo"))
        .await
        .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn get_repo(con: &mut SqliteConnection, team_id: &TeamId) -> Result<Repo> {
    let Some(repo) = fetch_repo(con, team_id).await? else {
        return Err(WebError::not_found(location!()));
    };
    Ok(repo)
}

#[instrument(skip_all)]
pub(super) async fn get_repos(con: &mut SqliteConnection) -> Result<Vec<Repo>> {
    query_as!(Repo, "SELECT team, url FROM Repos")
        .fetch_all(con)
        .instrument(info_span!("sqlx_get_repos"))
        .await
        .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn patch_or_create_repo(
    con: impl Acquire<'_, Database = Sqlite>,
    team_id: &TeamId,
    repo_url: &str,
) -> Result<Repo> {
    let mut con = con.begin().await.context(SqlxSnafu)?;

    query!(
        r#"
        INSERT INTO Repos
            (team, url)
        VALUES
            (?, ?)
        ON CONFLICT DO UPDATE SET
          url = excluded.url
        "#,
        team_id,
        repo_url,
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_update_insert_repo"))
    .await
    .context(SqlxSnafu)?;

    let repo = query_as!(Repo, "SELECT * FROM Repos WHERE team = ?", team_id)
        .fetch_one(&mut *con)
        .instrument(info_span!("sqlx_update_get_repo"))
        .await
        .context(SqlxSnafu)?;

    con.commit().await.context(SqlxSnafu)?;

    Ok(repo)
}
