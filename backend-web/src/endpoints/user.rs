use crate::auth;
use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{AppState, FullUserForAdmin, OwnUser, Team, User, UserId};
use axum::extract::State;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[instrument(skip_all)]
pub async fn show_me_myself(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
) -> Result<Json<MeResponse>> {
    let user = db.get_user(&claims.sub).await?;
    let team = match &user.user.team {
        Some(team) => Some(db.get_team(team).await?),
        None => None,
    };

    Ok(Json(MeResponse { user, team }))
}

#[instrument(skip_all)]
pub async fn list_users(
    State(AppState { db, .. }): State<AppState>,
    claims: Option<Claims>,
) -> Result<Json<Vec<UserInfoResponse>>> {
    let users = db
        .fetch_users()
        .await?
        .into_iter()
        .flat_map(|x| UserInfoResponse::from_full_user(&claims, x))
        .collect();

    Ok(Json(users))
}

#[instrument(skip_all)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<LoginResponse>> {
    let user = state.db.get_user_for_login(&payload.username).await?;

    let auth_user = match user {
        Some(user) => user,
        None => return Err(WebError::NotFound),
    };

    let user = auth_user.user;
    let token = auth::create_jwt(user.id.clone(), &state.jwt_keys, auth_user.role)?;

    info!(user = %user.id, "Logged in user");

    Ok(Json(LoginResponse { user, token }))
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: OwnUser,
    pub team: Option<Team>,
}

#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    pub username: UserId,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: User,
    pub token: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum UserInfoResponse {
    User(User),
    FullUser(FullUserForAdmin),
}

impl UserInfoResponse {
    pub fn from_full_user(claims: &Option<Claims>, user: FullUserForAdmin) -> Option<Self> {
        if Claims::is_admin_opt(claims) {
            Some(Self::FullUser(user))
        } else {
            Some(Self::User(user.into_user()))
        }
    }
}
