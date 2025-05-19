use crate::config::TeamEntry;
use crate::error::{Result, SqlxSnafu, WebError};
use crate::types::{Team, TeamId, TeamIntegrationToken, User, UserRole};
use snafu::{location, ResultExt};
use sqlx::{query, query_as, Acquire, Sqlite, SqliteConnection};
use std::collections::HashSet;
use tracing::{info, info_span, instrument, warn, Instrument};

#[instrument(skip_all)]
pub(super) async fn get_team(con: &mut SqliteConnection, team_id: &TeamId) -> Result<Team> {
    let team = query_as!(
        Team,
        r#"SELECT id as "id!", display_name FROM Teams WHERE id = ?"#,
        team_id
    )
    .fetch_optional(con)
    .instrument(info_span!("sqlx_get_team"))
    .await
    .context(SqlxSnafu)?;

    match team {
        None => Err(WebError::not_found(location!())),
        Some(team) => Ok(team),
    }
}

#[instrument(skip_all)]
pub(super) async fn get_teams(con: &mut SqliteConnection) -> Result<Vec<Team>> {
    query_as!(Team, r#"SELECT id as "id!", display_name FROM Teams"#,)
        .fetch_all(con)
        .instrument(info_span!("sqlx_get_teams"))
        .await
        .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn get_team_info(
    con: &mut SqliteConnection,
    team_id: &TeamId,
) -> Result<(Team, Vec<User>)> {
    let team = get_team(con, team_id).await?;

    let members = query_as!(
        User,
        r#"SELECT id as "id!", display_name, team as "team!: TeamId" FROM Users WHERE team = ?"#,
        team_id
    )
    .fetch_all(con)
    .instrument(info_span!("sqlx_get_team_info"))
    .await
    .context(SqlxSnafu)?;

    Ok((team, members))
}

#[instrument(skip_all)]
pub(super) async fn get_team_integration_token(
    con: &mut SqliteConnection,
    team_id: &TeamId,
) -> Result<TeamIntegrationToken> {
    query!(
        "SELECT token FROM TeamIntegrationTokens WHERE team_id = ?",
        team_id
    )
    .map(|it| it.token.into())
    .fetch_one(con)
    .instrument(info_span!("sqlx_get_team_integration_token"))
    .await
    .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn fetch_team_by_integration_token(
    con: &mut SqliteConnection,
    token: &TeamIntegrationToken,
) -> Result<Option<TeamId>> {
    query!(
        r#"SELECT team_id as "team_id!: TeamId" FROM TeamIntegrationTokens WHERE token = ?"#,
        token
    )
    .map(|it| it.team_id)
    .fetch_optional(con)
    .await
    .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn sync_teams(
    con: impl Acquire<'_, Database = Sqlite>,
    teams: &[TeamEntry],
) -> Result<()> {
    let mut con = con.begin().await.context(SqlxSnafu)?;

    query!("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *con)
        .instrument(info_span!("sqlx_sync_teams_pragma"))
        .await
        .context(SqlxSnafu)?;

    // Delete all existing teams as we will build them anew (if still relevant)
    query!("UPDATE Users SET TEAM = null")
        .execute(&mut *con)
        .instrument(info_span!("sqlx_sync_teams_update_users"))
        .await
        .context(SqlxSnafu)?;

    for team in teams {
        query!(
            r#"
            INSERT INTO Teams
                (id, display_name)
            VALUES
                (?, ?)
            ON CONFLICT DO UPDATE SET
                display_name = excluded.display_name"#,
            team.id,
            team.display_name
        )
        .execute(&mut *con)
        .instrument(info_span!("sqlx_sync_teams_insert"))
        .await
        .context(SqlxSnafu)?;

        for member in &team.members {
            let role = if team.is_admin {
                UserRole::Admin
            } else {
                UserRole::Regular
            };
            let res = query!(
                "UPDATE Users SET team = ?, role = ? WHERE id = ?",
                team.id,
                role,
                member
            )
            .execute(&mut *con)
            .instrument(info_span!("sqlx_sync_teams_update_user"))
            .await
            .context(SqlxSnafu)?;

            if res.rows_affected() == 0 {
                warn!(user = ?member, team = ?team.id, "User not found when adding to team");
            }
        }

        let new_integration_token = uuid::Uuid::new_v4().to_string();
        query!(
            r#"
            INSERT INTO TeamIntegrationTokens
                (team_id, token)
            VALUES
                (?, ?)
            ON CONFLICT DO NOTHING
            "#,
            team.id,
            new_integration_token
        )
        .execute(&mut *con)
        .await
        .context(SqlxSnafu)?;
    }

    let existing_teams = query!(r#"SELECT id as "id!" FROM Teams"#)
        .map(|it| it.id)
        .fetch_all(&mut *con)
        .instrument(info_span!("sqlx_sync_teams_get_existing"))
        .await
        .context(SqlxSnafu)?;
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
                .await
                .context(SqlxSnafu)?;
        }
    }

    con.commit().await.context(SqlxSnafu)?;

    Ok(())
}
