use crate::error::{Result, WebError};
use crate::types::{FullUserForAdmin, OwnUser, User, UserId, UserRole};
use sqlx::{SqliteConnection, query};
use tracing::{Instrument, info, instrument, trace_span};

#[instrument(skip_all)]
pub(super) async fn get_user_for_login(
    con: &mut SqliteConnection,
    user_id: &UserId,
) -> Result<Option<UserForAuth>> {
    Ok(sqlx::query_as("SELECT * FROM Users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(con)
        .instrument(trace_span!("sqlx_get_user_login"))
        .await?)
}

#[instrument(skip_all)]
pub(super) async fn fetch_user(
    con: &mut SqliteConnection,
    user_id: &UserId,
) -> Result<Option<FullUserForAdmin>> {
    Ok(sqlx::query_as("SELECT * FROM Users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(con)
        .instrument(trace_span!("sqlx_fetch_user"))
        .await?)
}

#[instrument(skip_all)]
pub(super) async fn get_user(con: &mut SqliteConnection, user_id: &UserId) -> Result<OwnUser> {
    let maybe_user = fetch_user(con, user_id).await?;

    match maybe_user {
        None => {
            info!(user = ?user_id, "Tried to query non-existing user");
            Err(WebError::NotFound)
        }
        Some(user) => Ok(user.user),
    }
}

#[instrument(skip_all)]
pub(super) async fn fetch_users(con: &mut SqliteConnection) -> Result<Vec<FullUserForAdmin>> {
    Ok(sqlx::query_as("SELECT * FROM Users")
        .fetch_all(con)
        .instrument(trace_span!("sqlx_get_users"))
        .await?)
}

#[instrument(skip_all)]
pub(super) async fn add_user(con: &mut SqliteConnection, user: &UserForAuth) -> Result<()> {
    let inner = &user.user;
    query!(
        "INSERT INTO Users (id, display_name, role, team) VALUES (?, ?, ?, ?)",
        inner.id,
        inner.display_name,
        user.role,
        inner.team
    )
    .execute(con)
    .instrument(trace_span!("sqlx_add_user"))
    .await?;

    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct UserForAuth {
    pub role: UserRole,
    #[sqlx(flatten)]
    pub user: User,
}
