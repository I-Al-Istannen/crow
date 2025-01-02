use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{AppState, Repo, TeamId};
use axum::extract::{Path, State};
use serde::Deserialize;
use shared::FinishedCompilerTask;
use std::time::Duration;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn set_team_repo(
    State(state): State<AppState>,
    claims: Claims,
    Path(target_team): Path<TeamId>,
    Json(payload): Json<TeamPatchPayload>,
) -> Result<Json<Repo>> {
    let user = state.db.get_user(&claims.sub).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    if !claims.is_admin() {
        let Some(team) = user.user.team else {
            return Err(WebError::NotInTeam);
        };

        if team != target_team {
            return Err(WebError::NoPermissions);
        }
    }

    let repo = state
        .db
        .set_team_repo(&target_team, &payload.repo_url, payload.auto_fetch)
        .await?;
    state.local_repos.update_repo(&repo).await?;

    Ok(Json(repo))
}

#[instrument(skip_all)]
pub async fn get_team_repo(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
    Path(team_id): Path<TeamId>,
) -> Result<Json<Repo>> {
    let user = db.get_user(&claims.sub).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    if claims.is_admin() {
        return Ok(Json(db.get_repo(&team_id).await?));
    }

    let Some(team) = user.user.team else {
        return Err(WebError::NotInTeam);
    };

    Ok(Json(db.get_repo(&team).await?))
}

#[instrument(skip_all)]
pub async fn get_recent_tasks(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
) -> Result<Json<Vec<FinishedCompilerTask>>> {
    let user = db.get_user(&claims.sub).await?;
    let Some(team) = user.user.team else {
        return Err(WebError::NotInTeam);
    };

    Ok(Json(db.get_recent_tasks(&team, 20).await?))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPatchPayload {
    pub repo_url: String,
    pub auto_fetch: bool,
}
