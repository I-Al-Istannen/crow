use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::Result;
use crate::types::{AppState, FullUserForAdmin, OwnUser, Team, TeamId, TeamIntegrationToken};
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
    _claims: Option<Claims>,
) -> Result<Json<Vec<UserInfo>>> {
    let mut result = Vec::new();

    for user in db.fetch_users().await? {
        let (repo_url, team) = if let Some(team) = &user.user.user.team {
            (
                db.fetch_repo(team).await?.map(|repo| repo.url),
                db.get_team(team).await.ok(),
            )
        } else {
            (None, None)
        };
        result.push(UserInfo {
            user,
            repo_url,
            team,
        });
    }

    Ok(Json(result))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub user: OwnUser,
    pub team: Option<Team>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: OwnUser,
    pub token: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    #[serde(flatten)]
    user: FullUserForAdmin,
    repo_url: Option<String>,
    team: Option<Team>,
}
