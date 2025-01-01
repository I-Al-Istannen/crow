use crate::error::WebError;
use crate::types::{FullUserForAdmin, OwnUser, User, UserId, UserRole};
use sqlx::{query, SqliteConnection};
use tracing::{info, trace_span, Instrument};

pub(super) async fn get_user_for_login(
    con: &mut SqliteConnection,
    user_id: &UserId,
) -> Result<Option<UserForAuth>, WebError> {
    Ok(sqlx::query_as("SELECT * FROM Users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(con)
        .instrument(trace_span!("sqlx_get_user_login"))
        .await?)
}

pub(super) async fn fetch_user(
    con: &mut SqliteConnection,
    user_id: &UserId,
) -> Result<Option<FullUserForAdmin>, WebError> {
    Ok(sqlx::query_as("SELECT * FROM Users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(con)
        .instrument(trace_span!("sqlx_fetch_user"))
        .await?)
}

pub(super) async fn get_user(
    con: &mut SqliteConnection,
    user_id: &UserId,
) -> Result<OwnUser, WebError> {
    let maybe_user = fetch_user(con, user_id).await?;

    match maybe_user {
        None => {
            info!(user = ?user_id, "Tried to query non-existing user");
            Err(WebError::NotFound)
        }
        Some(user) => Ok(user.user),
    }
}

pub(super) async fn fetch_users(
    con: &mut SqliteConnection,
) -> Result<Vec<FullUserForAdmin>, WebError> {
    Ok(sqlx::query_as("SELECT * FROM Users")
        .fetch_all(con)
        .instrument(trace_span!("sqlx_get_users"))
        .await?)
}

pub async fn add_user(con: &mut SqliteConnection, user: &UserForAuth) -> Result<(), WebError> {
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
