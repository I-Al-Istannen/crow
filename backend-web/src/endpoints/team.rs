use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{AppState, FinishedCompilerTaskSummary, Repo, TeamId, TeamInfo};
use axum::extract::{Path, State};
use serde::Deserialize;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn set_team_repo(
    State(state): State<AppState>,
    claims: Claims,
    Path(target_team): Path<TeamId>,
    Json(payload): Json<TeamPatchPayload>,
) -> Result<Json<Repo>> {
    let user = state.db.get_user(&claims.sub).await?;

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
        .set_team_repo(&target_team, &payload.repo_url)
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

    if claims.is_admin() {
        return Ok(Json(db.get_repo(&team_id).await?));
    }

    let Some(team) = user.user.team else {
        return Err(WebError::NotInTeam);
    };

    Ok(Json(db.get_repo(&team).await?))
}

#[instrument(skip_all)]
pub async fn get_n_recent_tasks(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
    Path(count): Path<u32>,
) -> Result<Json<Vec<FinishedCompilerTaskSummary>>> {
    let user = db.get_user(&claims.sub).await?;
    let Some(team) = user.user.team else {
        return Err(WebError::NotInTeam);
    };
    let count = if count == 0 { u32::MAX } else { count };

    Ok(Json(db.get_recent_tasks(&team, count).await?))
}

#[instrument(skip_all)]
pub async fn get_recent_tasks(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
) -> Result<Json<Vec<FinishedCompilerTaskSummary>>> {
    let user = db.get_user(&claims.sub).await?;
    let Some(team) = user.user.team else {
        return Err(WebError::NotInTeam);
    };

    Ok(Json(db.get_recent_tasks(&team, 10).await?))
}

#[instrument(skip_all)]
pub async fn get_team_info(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
    Path(team_id): Path<TeamId>,
) -> Result<Json<TeamInfo>> {
    if !claims.is_admin() {
        let user = db.get_user(&claims.sub).await?;
        let Some(team) = user.user.team else {
            return Err(WebError::NotInTeam);
        };
        if team != team_id {
            return Err(WebError::NoPermissions);
        }
    }

    Ok(Json(db.get_team_info(&team_id).await?))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPatchPayload {
    pub repo_url: String,
}
