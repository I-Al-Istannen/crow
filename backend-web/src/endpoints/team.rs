use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::WebError;
use crate::types::{AppState, Repo, TeamId};
use axum::extract::{Path, State};
use serde::Deserialize;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn set_team_repo(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
    Path(target_team): Path<TeamId>,
    Json(payload): Json<TeamPatchPayload>,
) -> Result<Json<Repo>, WebError> {
    let user = db.get_user(&claims.sub).await?;

    let Some(team) = user.user.team else {
        return Err(WebError::NotInTeam);
    };

    if team != target_team && !claims.is_admin() {
        return Err(WebError::NoPermissions);
    }

    let repo = db
        .set_team_repo(&target_team, &payload.repo_url, payload.auto_fetch)
        .await?;

    Ok(Json(repo))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPatchPayload {
    pub repo_url: String,
    pub auto_fetch: bool,
}
