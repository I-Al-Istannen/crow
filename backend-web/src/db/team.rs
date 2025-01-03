use crate::config::TeamEntry;
use crate::error::{Result, WebError};
use crate::types::{Team, TeamId, TeamInfo, User, UserId};
use sqlx::{query, Acquire, Sqlite, SqliteConnection};
use std::collections::HashSet;
use tracing::{info, info_span, instrument, warn, Instrument};

#[instrument(skip_all)]
pub(super) async fn get_team(con: &mut SqliteConnection, team_id: &TeamId) -> Result<Team> {
    let team = query!(
        r#"SELECT id as "id!: TeamId", display_name FROM Teams WHERE id = ?"#,
        team_id
    )
    .map(|it| Team {
        id: it.id,
        display_name: it.display_name,
    })
    .fetch_optional(con)
    .instrument(info_span!("sqlx_get_team"))
    .await?;

    match team {
        None => Err(WebError::NotFound),
        Some(team) => Ok(team),
    }
}

pub(super) async fn get_team_info(
    con: &mut SqliteConnection,
    team_id: &TeamId,
) -> Result<TeamInfo> {
    let team = get_team(con, team_id).await?;

    let members = query!(
        r#"
        SELECT
            id as "id!: UserId",
            display_name,
            team as "team?: TeamId"
        FROM Users
        WHERE team = ?
        "#,
        team_id
    )
    .map(|it| User {
        id: it.id,
        display_name: it.display_name,
        team: it.team,
    })
    .fetch_all(con)
    .instrument(info_span!("sqlx_get_team_info"))
    .await?;

    Ok(TeamInfo { team, members })
}

#[instrument(skip_all)]
pub(super) async fn sync_teams(
    con: impl Acquire<'_, Database = Sqlite>,
    teams: &[TeamEntry],
) -> Result<()> {
    let mut con = con.begin().await?;

    query!("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *con)
        .instrument(info_span!("sqlx_sync_teams_pragma"))
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
        .instrument(info_span!("sqlx_sync_teams_insert"))
        .await?;

        for member in &team.members {
            let res = query!("UPDATE Users SET team = ? WHERE id = ?", team.id, member)
                .execute(&mut *con)
                .instrument(info_span!("sqlx_sync_teams_update_user"))
                .await?;

            if res.rows_affected() == 0 {
                warn!(user = ?member, team = ?team.id, "User not found when adding to team");
            }
        }
    }

    let existing_teams = query!(r#"SELECT id as "id!" FROM Teams"#)
        .map(|it| it.id)
        .fetch_all(&mut *con)
        .instrument(info_span!("sqlx_sync_teams_get_existing"))
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
                .instrument(info_span!("sqlx_sync_teams_delete"))
                .await?;
        }
    }

    con.commit().await?;

    Ok(())
}
