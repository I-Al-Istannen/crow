use crate::config::TeamEntry;
use crate::error::WebError;
use crate::types::{Team, TeamId};
use sqlx::{query, Acquire, Sqlite, SqliteConnection};
use std::collections::HashSet;
use tracing::{info, warn};

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

    for team in teams {
        query!(
            r#"
            INSERT INTO Teams
                (id, display_name)
            VALUES
                (?, ?)
            ON CONFLICT DO UPDATE SET
                display_name = ?"#,
            team.id,
            team.display_name,
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

    let existing_teams = query!(r#"SELECT id as "id!" FROM Teams"#)
        .map(|it| it.id)
        .fetch_all(&mut *con)
        .await?;
    let expected_teams = teams
        .iter()
        .map(|team| team.id.to_string())
        .collect::<HashSet<_>>();

    for team in existing_teams {
        if !expected_teams.contains(&team) {
            info!(team = ?team, "Removing team");
            query!("DELETE FROM Teams WHERE id = ?", team)
                .execute(&mut *con)
                .await?;
        }
    }

    con.commit().await?;

    Ok(())
}
