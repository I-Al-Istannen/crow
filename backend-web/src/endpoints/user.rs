use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::Result;
use crate::types::{AppState, FullUserForAdmin, OwnUser, Team, TeamId, TeamIntegrationToken, User};
use axum::extract::State;
use serde::Serialize;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn show_me_myself(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims<Option<TeamId>>,
) -> Result<Json<MeResponse>> {
    let user = db.get_user(&claims.sub).await?;
    let team = match claims.team {
        Some(team) => Some(db.get_team(&team).await?),
        None => None,
    };

    Ok(Json(MeResponse { user, team }))
}

#[instrument(skip_all)]
pub async fn get_integration_status(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<IntegrationInfoResponse>> {
    let token = state.db.get_team_integration_token(&claims.team).await?;
    let github = state
        .github_app_name
        .map(GithubIntegrationInfoResponse::from_app_name);

    Ok(Json(IntegrationInfoResponse { token, github }))
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub user: OwnUser,
    pub team: Option<Team>,
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

#[derive(Debug, Serialize)]
pub struct GithubIntegrationInfoResponse {
    pub url: String,
}

impl GithubIntegrationInfoResponse {
    pub fn from_app_name(app_name: String) -> Self {
        Self {
            url: format!("https://github.com/apps/{app_name}/installations/new"),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IntegrationInfoResponse {
    pub token: TeamIntegrationToken,
    pub github: Option<GithubIntegrationInfoResponse>,
}
