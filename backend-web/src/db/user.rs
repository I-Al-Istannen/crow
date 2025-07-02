use crate::auth::oidc::OidcUser;
use crate::error::{Result, SqlxSnafu, WebError};
use crate::types::{FullUserForAdmin, OwnUser, TeamId, User, UserId, UserRole};
use snafu::{ResultExt, location};
use sqlx::{SqliteConnection, query};
use tracing::{Instrument, instrument, trace_span};

#[instrument(skip_all)]
pub(super) async fn get_user_for_login(
    con: &mut SqliteConnection,
    user_id: &UserId,
) -> Result<Option<UserForAuth>> {
    sqlx::query_as("SELECT * FROM Users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(con)
        .instrument(trace_span!("sqlx_get_user_login"))
        .await
        .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn fetch_user(
    con: &mut SqliteConnection,
    user_id: &UserId,
) -> Result<Option<FullUserForAdmin>> {
    sqlx::query_as("SELECT * FROM Users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(con)
        .instrument(trace_span!("sqlx_fetch_user"))
        .await
        .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn get_user(con: &mut SqliteConnection, user_id: &UserId) -> Result<OwnUser> {
    let maybe_user = fetch_user(con, user_id).await?;

    match maybe_user {
        None => Err(WebError::not_found(location!())),
        Some(user) => Ok(user.user),
    }
}

#[instrument(skip_all)]
pub(super) async fn fetch_users(con: &mut SqliteConnection) -> Result<Vec<FullUserForAdmin>> {
    sqlx::query_as("SELECT * FROM Users")
        .fetch_all(con)
        .instrument(trace_span!("sqlx_get_users"))
        .await
        .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn synchronize_oidc_user(
    con: &mut SqliteConnection,
    user: OidcUser,
    team: Option<TeamId>,
    role: Option<UserRole>,
) -> Result<OwnUser> {
    let role = role.unwrap_or(UserRole::Regular);
    query!(
        r#"
        INSERT INTO Users
            (id, display_name, role, team)
        VALUES
            (?, ?, ?, ?)
        ON CONFLICT DO UPDATE SET
            display_name = excluded.display_name,
            team = coalesce(excluded.team, team),
            role = excluded.role
        "#,
        user.id,
        user.name,
        role,
        team,
    )
    .execute(&mut *con)
    .await
    .context(SqlxSnafu)?;

    let user_id = UserId::from(user.id);

    get_user(&mut *con, &user_id).await
}

#[derive(sqlx::FromRow)]
pub struct UserForAuth {
    pub role: UserRole,
    #[sqlx(flatten)]
    pub user: User,
}
