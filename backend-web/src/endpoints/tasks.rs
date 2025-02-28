use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{
    AppState, ExecutorInfo, FinishedCompilerTaskSummary, QueuedTaskStatus, RunnerForFrontend,
    TaskId, TeamId, WorkItem,
};
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use serde::Serialize;
use serde_json::json;
use shared::FinishedCompilerTask;
use snafu::location;
use std::collections::HashMap;
use std::time::SystemTime;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip_all)]
pub async fn integration_request_revision(
    State(state): State<AppState>,
    TypedHeader(Authorization(auth)): TypedHeader<Authorization<Bearer>>,
    Path(revision): Path<String>,
) -> Result<Response> {
    let token = auth.token().to_string().into();
    let Some(team_id) = state.db.fetch_team_by_integration_token(&token).await? else {
        return Err(WebError::invalid_credentials(location!()));
    };

    queue_task(state, &revision, team_id).await
}

#[instrument(skip_all)]
pub async fn integration_get_task_status(
    State(state): State<AppState>,
    TypedHeader(Authorization(auth)): TypedHeader<Authorization<Bearer>>,
    Path(task_id): Path<TaskId>,
) -> Result<Json<IntegrationTaskStatusResponse>> {
    let token = auth.token().to_string().into();
    if state
        .db
        .fetch_team_by_integration_token(&token)
        .await?
        .is_none()
    {
        return Err(WebError::invalid_credentials(location!()));
    };

    if state
        .executor
        .lock()
        .unwrap()
        .get_running_task(&task_id)
        .is_some()
    {
        return Ok(Json(IntegrationTaskStatusResponse {
            status: QueuedTaskStatus::Running,
        }));
    }

    if state.db.fetch_queued_task(&task_id).await?.is_some() {
        return Ok(Json(IntegrationTaskStatusResponse {
            status: QueuedTaskStatus::Queued,
        }));
    }

    let task = state.db.get_task(&task_id).await?;
    Ok(Json(IntegrationTaskStatusResponse {
        status: task.into(),
    }))
}

#[instrument(skip_all)]
pub async fn request_revision(
    State(state): State<AppState>,
    claims: Claims,
    Path(revision): Path<String>,
) -> Result<Response> {
    queue_task(state, &revision, claims.team).await
}

async fn queue_task(state: AppState, revision: &str, team: TeamId) -> Result<Response> {
    // Update repo to ensure revision is present
    let repo = state.db.get_repo(&team).await?;
    state.local_repos.update_repo(&repo).await?;
    let Some(revision) = state.local_repos.get_revision(&repo, revision).await? else {
        return Err(WebError::not_found(location!()));
    };
    let commit_message = state
        .local_repos
        .get_revision_message(&repo, &revision)
        .await?;

    let task_id: TaskId = Uuid::new_v4().to_string().into();
    let task = WorkItem {
        id: task_id.clone(),
        team,
        revision: revision.to_string(),
        commit_message,
        insert_time: SystemTime::now(),
    };
    state.db.queue_task(task.clone()).await?;

    Ok(Json(json!({ "taskId": task_id })).into_response())
}

#[instrument(skip_all)]
pub async fn get_queue(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<QueueResponse>> {
    let queue = state.db.get_queued_tasks().await?;
    let runners = state.executor.lock().unwrap().get_runners();

    Ok(Json(QueueResponse { queue, runners }))
}

#[instrument(skip_all)]
pub async fn get_queued_task(
    State(state): State<AppState>,
    _claims: Claims,
    Path(task_id): Path<TaskId>,
) -> Result<Json<WorkItem>> {
    if state
        .executor
        .lock()
        .unwrap()
        .get_running_task(&task_id)
        .is_some()
    {
        return Err(WebError::not_found(location!()));
    }
    let Some(item) = state.db.fetch_queued_task(&task_id).await? else {
        return Err(WebError::not_found(location!()));
    };

    Ok(Json(item))
}

#[instrument(skip_all)]
pub async fn get_task(
    State(state): State<AppState>,
    _claims: Claims,
    Path(task_id): Path<TaskId>,
) -> Result<Json<FinishedCompilerTask>> {
    Ok(Json(state.db.get_task(&task_id).await?))
}

#[instrument(skip_all)]
pub async fn get_top_task_per_team(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<HashMap<TeamId, FinishedCompilerTaskSummary>>> {
    Ok(Json(state.db.get_top_task_per_team().await?))
}

#[instrument(skip_all)]
pub async fn list_task_ids(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<Vec<TaskId>>> {
    Ok(Json(state.db.get_task_ids().await?))
}

#[instrument(skip_all)]
pub async fn executor_info(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<ExecutorInfo>> {
    Ok(Json(state.executor.lock().unwrap().info()))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueResponse {
    pub queue: Vec<WorkItem>,
    pub runners: Vec<RunnerForFrontend>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationTaskStatusResponse {
    status: QueuedTaskStatus,
}
