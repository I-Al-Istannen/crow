use super::{Json, Path};
use crate::auth::Claims;
use crate::config::TestCategory;
use crate::error::{Result, WebError};
use crate::grading_formulas::{GradingPoints, get_grading_points_for_task};
use crate::storage::GitError;
use crate::types::{
    AppState, FinalSubmittedTask, FinishedCompilerTaskSummary, Repo, TaskId, TeamId, TeamInfo,
};
use axum::extract::State;
use serde::{Deserialize, Serialize};
use snafu::{Report, location};
use std::collections::{HashMap, HashSet};
use tracing::{info, instrument};

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

    let repo = Repo {
        team: target_team,
        url: payload.repo_url,
    };

    if let Err(e) = state.local_repos.update_repo(&repo).await {
        info!(
            error = %Report::from_error(&e),
            team = %repo.team,
            url = %repo.url,
            "Failed to update repo"
        );

        if matches!(e, GitError::NotCloned { .. }) {
            return Err(WebError::named_bad_request(
                Report::from_error(&e).to_string(),
                location!(),
            ));
        }

        return Err(e)?;
    }

    // Update only after a successful clone
    let repo = state.db.set_team_repo(&repo.team, &repo.url).await?;

    info!(
        team = %repo.team,
        user = %claims.sub,
        url = %repo.url,
        "Updated team repo"
    );

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
pub async fn get_tasks_for_team(
    State(AppState { db, .. }): State<AppState>,
    Path(team_id): Path<TeamId>,
    _claims: Claims,
) -> Result<Json<Vec<FinishedCompilerTaskSummary>>> {
    Ok(Json(db.get_recent_tasks(&team_id, u32::MAX).await?))
}

#[instrument(skip_all)]
pub async fn get_final_tasks(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<HashMap<String, FinalSubmittedTaskWithPoints>>> {
    let mut result = HashMap::new();

    for (name, meta) in &state.test_config.categories {
        if let Some(task) = state
            .db
            .get_final_submitted_task_for_team_and_category(&claims.team, name, meta, true)
            .await?
        {
            // Only calculate points for finalized tasks
            let points = if matches!(task, FinalSubmittedTask::Finalized { .. }) {
                get_grading_points_for_task(
                    &state.test_config,
                    name,
                    meta,
                    &state
                        .db
                        .get_finished_test_summaries(&task.task_id())
                        .await?,
                )?
            } else {
                None
            };
            result.insert(name.clone(), FinalSubmittedTaskWithPoints { task, points });
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
    let mut current_categories = HashMap::new();
    for (name, category) in &state.test_config.categories {
        let task = state
            .db
            .get_final_submitted_task_for_team_and_category(&claims.team, name, category, true)
            .await?;
        if let Some(task) = task {
            if task.task_id() == payload.task_id {
                current_categories.insert(name.clone(), category.clone());
            }
        }
    }

    ensure_added_categories_not_past_due(&state, &payload, &mut current_categories)?;
    ensure_removed_categories_not_past_due(&state, &claims, &payload).await?;

    state
        .db
        .set_final_submitted_task(
            &claims.team,
            &claims.sub,
            &payload.task_id,
            payload.categories.iter().map(|s| s.as_str()),
        )
        .await?;

    info!(
        team = %claims.team,
        user = %claims.sub,
        task_id = %payload.task_id,
        categories = ?payload.categories,
        "Set final task"
    );

    Ok(())
}

fn ensure_added_categories_not_past_due(
    state: &AppState,
    payload: &SetFinalTaskPayload,
    current_categories: &mut HashMap<String, TestCategory>,
) -> Result<()> {
    for new_category in &payload.categories {
        if current_categories.contains_key(new_category) {
            continue;
        }
        let category = state
            .test_config
            .categories
            .get(new_category)
            .ok_or_else(|| {
                WebError::named_not_found(format!("category `{new_category}`"), location!())
            })?;
        if category.is_after_labs_deadline() {
            return Err(WebError::named_unauthorized(
                format!("submit solution, as `{new_category}` is already past the deadline"),
                location!(),
            ));
        }
    }

    Ok(())
}

async fn ensure_removed_categories_not_past_due(
    state: &AppState,
    claims: &Claims,
    payload: &SetFinalTaskPayload,
) -> Result<()> {
    for (name, category) in &state.test_config.categories {
        if category.is_after_labs_deadline() && !payload.categories.contains(name) {
            let current_submitted_task = state
                .db
                .get_final_submitted_task_for_team_and_category(&claims.team, name, category, true)
                .await?;
            if let Some(current_submitted_task) = current_submitted_task {
                if current_submitted_task.task_id() == payload.task_id {
                    return Err(WebError::named_unauthorized(
                        format!("change the solution, as `{name}` was already due"),
                        location!(),
                    ));
                }
            }
        }
    }
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
    let (team, members) = db.get_team_info(&team_id).await?;
    let repo_url = db.fetch_repo(&team_id).await?.map(|it| it.url);

    Ok(Json(TeamInfo {
        team,
        members,
        repo_url,
    }))
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
    categories: HashSet<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalSubmittedTaskWithPoints {
    #[serde(flatten)]
    pub task: FinalSubmittedTask,
    pub points: Option<GradingPoints>,
}
