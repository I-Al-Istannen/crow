use crate::config::TeamEntry;
use crate::error::WebError;
use crate::types::{Team, TeamId};
use sqlx::{query, Acquire, Sqlite, SqliteConnection};
use tracing::warn;

pub async fn get_team(con: &mut SqliteConnection, team_id: &TeamId) -> Result<Team, WebError> {
    let team = query!(
        r#"SELECT id as "id!: TeamId", display_name FROM Teams WHERE id = ?"#,
        team_id
    )
    .map(|it| Team {
        id: it.id,
        display_name: it.display_name,
    })
    .fetch_optional(con)
    .await?;

    match team {
        None => Err(WebError::NotFound),
        Some(team) => Ok(team),
    }
}

pub async fn sync_teams(
    con: impl Acquire<'_, Database = Sqlite>,
    teams: &[TeamEntry],
) -> Result<(), WebError> {
    let mut con = con.begin().await?;

    query!("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *con)
        .await?;
    query!("DELETE FROM Teams").execute(&mut *con).await?;

    for team in teams {
        query!(
            "INSERT INTO Teams (id, display_name) VALUES (?, ?)",
            team.id,
            team.display_name
        )
        .execute(&mut *con)
        .await?;

        for member in &team.members {
            let res = query!("UPDATE Users SET team = ? WHERE id = ?", team.id, member)
                .execute(&mut *con)
                .await?;

            if res.rows_affected() == 0 {
                warn!(user = ?member, team = ?team.id, "User not found when adding to team");
            }
        }
    }

    con.commit().await?;

    Ok(())
}
