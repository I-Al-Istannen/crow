use crate::error::WebError;
use crate::types::{Repo, TeamId};
use sqlx::{query, Acquire, Sqlite, SqliteConnection};

pub async fn fetch_repo(
    con: &mut SqliteConnection,
    team_id: &TeamId,
) -> Result<Option<Repo>, WebError> {
    Ok(query!("SELECT * FROM Repos WHERE team = ?", team_id)
        .map(|it| Repo {
            team: team_id.clone(),
            url: it.url,
            auto_fetch: it.auto_fetch,
        })
        .fetch_optional(con)
        .await?)
}

pub async fn get_repo(con: &mut SqliteConnection, team_id: &TeamId) -> Result<Repo, WebError> {
    let Some(repo) = fetch_repo(con, team_id).await? else {
        return Err(WebError::NotFound);
    };
    Ok(repo)
}

pub async fn patch_or_create_repo(
    con: impl Acquire<'_, Database = Sqlite>,
    team_id: &TeamId,
    repo_url: &str,
    auto_fetch: bool,
) -> Result<Repo, WebError> {
    let mut con = con.begin().await?;

    query!(
        r#"
        INSERT INTO Repos
            (team, url, auto_fetch)
        VALUES
            (?, ?, ?)
        ON CONFLICT DO UPDATE SET
          url = ?,
          auto_fetch = ?
        "#,
        team_id,
        repo_url,
        auto_fetch,
        repo_url,
        auto_fetch
    )
    .execute(&mut *con)
    .await?;

    let repo = query!("SELECT * FROM Repos WHERE team = ?", team_id)
        .map(|it| Repo {
            team: team_id.clone(),
            url: it.url,
            auto_fetch: it.auto_fetch,
        })
        .fetch_one(&mut *con)
        .await?;

    con.commit().await?;

    Ok(repo)
}
