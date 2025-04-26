use super::{Json, Path};
use crate::auth::Claims;
use crate::error::{Result, WebError};
use crate::types::{
    AppState, FinalSubmittedTask, FinishedCompilerTaskSummary, Repo, TaskId, TeamId, TeamInfo,
};
use axum::extract::State;
use serde::Deserialize;
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
) -> Result<Json<HashMap<String, FinalSubmittedTask>>> {
    let mut result = HashMap::new();

    for (name, meta) in state.test_config.categories {
        if let Some(task) = state
            .db
            .get_final_submitted_task_for_team_and_category(&claims.team, &name, &meta)
            .await?
        {
            result.insert(name, task);
        }
    }

    Ok(Json(result))
}

#[instrument(skip_all)]
pub async fn set_final_task(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<SetFinalTaskPayload>,
) -> Result<()> {
    for category_name in &payload.categories {
        let Some(category) = state.test_config.categories.get(category_name) else {
            return Err(WebError::named_not_found(
                format!("category `{}`", category_name),
                location!(),
            ));
        };
        if category.is_after_test_deadline() {
            return Err(WebError::named_unauthorized(
                format!("submit solution, as `{}` was already due", category_name),
                location!(),
            ));
        }
    }

    // FIXME: Require that all existing expired categories stay

    state
        .db
        .set_final_submitted_task(
            &claims.team,
            &claims.sub,
            &payload.task_id,
            payload.categories.iter().map(|s| s.as_str()),
        )
        .await?;

    Ok(())
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFinalTaskPayload {
    task_id: TaskId,
    categories: Vec<String>,
}
