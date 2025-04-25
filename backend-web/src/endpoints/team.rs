use super::{Json, Path};
use crate::auth::Claims;
use crate::error::{Result, WebError};
use crate::types::{AppState, FinishedCompilerTaskSummary, Repo, TeamId, TeamInfo};
use axum::extract::State;
use serde::{Deserialize, Serialize};
use snafu::location;
use std::collections::HashMap;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn set_team_repo(
    State(state): State<AppState>,
    claims: Claims,
    Path(target_team): Path<TeamId>,
    Json(payload): Json<TeamPatchPayload>,
) -> Result<Json<Repo>> {
    if !claims.is_admin() && claims.team != target_team {
        return Err(WebError::unauthorized(location!()));
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
    if claims.is_admin() {
        return Ok(Json(db.get_repo(&team_id).await?));
    }

    Ok(Json(db.get_repo(&claims.team).await?))
}

#[instrument(skip_all)]
pub async fn get_n_recent_tasks(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
    Path(count): Path<u32>,
) -> Result<Json<Vec<FinishedCompilerTaskSummary>>> {
    let count = if count == 0 { u32::MAX } else { count };

    Ok(Json(db.get_recent_tasks(&claims.team, count).await?))
}

#[instrument(skip_all)]
pub async fn get_recent_tasks(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
) -> Result<Json<Vec<FinishedCompilerTaskSummary>>> {
    Ok(Json(db.get_recent_tasks(&claims.team, 10).await?))
}

#[instrument(skip_all)]
pub async fn get_final_tasks(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<HashMap<String, FinalSelectedTask>>> {
    let mut result = HashMap::new();

    for category in state.test_config.categories {
        if let Some(summary) = state
            .db
            .get_top_task_for_team_and_category(&claims.team, &category)
            .await?
        {
            result.insert(
                category,
                FinalSelectedTask {
                    summary,
                    automatically_selected: true,
                },
            );
        }
    }

    Ok(Json(result))
}

#[instrument(skip_all)]
pub async fn get_team_info(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
    Path(team_id): Path<TeamId>,
) -> Result<Json<TeamInfo>> {
    if !claims.is_admin() && claims.team != team_id {
        return Err(WebError::unauthorized(location!()));
    }

    Ok(Json(db.get_team_info(&team_id).await?))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPatchPayload {
    pub repo_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalSelectedTask {
    pub summary: FinishedCompilerTaskSummary,
    pub automatically_selected: bool,
}
