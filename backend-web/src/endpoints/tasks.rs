use super::{Json, Path};
use crate::auth::Claims;
use crate::error::{Result, WebError};
use crate::types::{
    AppState, ExecutorInfo, FinishedCompilerTaskStatistics, FinishedCompilerTaskSummary,
    QueuedTaskStatus, RunnerForFrontend, TaskId, TeamId, TestId, WorkItem,
};
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{
    ExecutionOutput, FinishedCompilerTask, FinishedExecution, FinishedTaskInfo, FinishedTest,
};
use snafu::location;
use std::collections::HashMap;
use std::time::SystemTime;
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(skip_all)]
pub async fn integration_request_revision(
    State(state): State<AppState>,
    TypedHeader(Authorization(auth)): TypedHeader<Authorization<Bearer>>,
    Path(revision): Path<String>,
    payload: Option<Json<IntegrationRequestRevisionPayload>>,
) -> Result<Response> {
    let token = auth.token().to_string().into();
    let Some(team_id) = state.db.fetch_team_by_integration_token(&token).await? else {
        return Err(WebError::invalid_credentials(location!()));
    };

    info!(revision = %revision, team = %team_id, "Integration requested revision run");

    queue_task(state, &revision, team_id, payload.map(|it| it.0)).await
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

    let (task, _) = state.db.get_task(&task_id).await?;
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
    queue_task(state, &revision, claims.team, None).await
}

async fn queue_task(
    state: AppState,
    revision: &str,
    team: TeamId,
    overrides: Option<IntegrationRequestRevisionPayload>,
) -> Result<Response> {
    // Update repo to ensure revision is present
    let repo = state.db.get_repo(&team).await?;
    state.local_repos.update_repo(&repo).await?;
    let Some(revision) = state.local_repos.get_revision(&repo, revision).await? else {
        return Err(WebError::named_not_found(
            format!("Revision `{revision}`"),
            location!(),
        ));
    };
    let commit_message = match overrides.as_ref().map(|it| it.commit_message.clone()) {
        Some(message) => message,
        None => {
            state
                .local_repos
                .get_revision_message(&repo, &revision)
                .await?
        }
    };

    let task_id: TaskId = Uuid::new_v4().to_string().into();
    let task = WorkItem {
        id: task_id.clone(),
        team: team.clone(),
        revision: revision.to_string(),
        commit_message,
        insert_time: SystemTime::now(),
    };
    state.db.queue_task(task.clone()).await?;

    if let Some(overrides) = overrides {
        if let Some(commit) = overrides.checked_commit {
            state
                .db
                .add_external_run_revision_mapping(&task_id, &commit)
                .await?;
        }
    }

    info!(
        task_id = %task_id,
        revision = %revision,
        team = %team,
        "Queued task"
    );

    Ok(Json(json!({ "taskId": task_id })).into_response())
}

#[instrument(skip_all)]
pub async fn get_queue(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<QueueResponse>> {
    let tasting_runners = state.test_tasting.lock().unwrap().get_tasting_runners();
    let runners = state.executor.lock().unwrap().get_runners(tasting_runners);

    let mut executing_tasks = runners
        .iter()
        .flat_map(|runner| runner.working_on.as_ref().and_then(|it| it.task()))
        .map(Clone::clone)
        .collect::<Vec<_>>();

    let queue = state
        .db
        .get_queued_tasks()
        .await?
        .into_iter()
        .filter(|item| !executing_tasks.iter().any(|it| it.id == item.id))
        .collect::<Vec<_>>();
    let queue = state.queue.lock().unwrap().reorder_queue(queue);

    executing_tasks.extend(queue);

    Ok(Json(QueueResponse {
        queue: executing_tasks,
        runners,
    }))
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
) -> Result<Json<FinishedCompilerTaskWithOutdated>> {
    Ok(Json(state.db.get_task(&task_id).await?.into()))
}

#[instrument(skip_all)]
pub async fn get_top_task_per_team(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<HashMap<TeamId, ApiFinishedCompilerTaskSummary>>> {
    let top_tasks = state.db.get_top_task_per_team().await?;
    let teams = state
        .db
        .get_teams()
        .await?
        .into_iter()
        .map(|team| (team.id, team.display_name))
        .collect::<HashMap<_, _>>();

    let top_tasks = top_tasks
        .into_iter()
        .map(|(team_id, task)| {
            let team_name = teams.get(&team_id).cloned().unwrap_or_default();
            (team_id, ApiFinishedCompilerTaskSummary { task, team_name })
        })
        .collect::<HashMap<_, _>>();

    Ok(Json(top_tasks))
}

#[instrument(skip_all)]
pub async fn executor_info(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<ExecutorInfo>> {
    let tasting_runners = state.test_tasting.lock().unwrap().get_tasting_runners();
    Ok(Json(state.executor.lock().unwrap().info(tasting_runners)))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueResponse {
    pub queue: Vec<WorkItem>,
    pub runners: Vec<RunnerForFrontend>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationRequestRevisionPayload {
    pub commit_message: String,
    pub checked_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationTaskStatusResponse {
    status: QueuedTaskStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum FinishedCompilerTaskWithOutdated {
    #[serde(rename_all = "camelCase")]
    BuildFailed {
        info: FinishedTaskInfo,
        build_output: ExecutionOutput,
        outdated: Vec<TestId>,
    },
    #[serde(rename_all = "camelCase")]
    RanTests {
        info: FinishedTaskInfo,
        build_output: FinishedExecution,
        tests: Vec<FinishedTest>,
        outdated: Vec<TestId>,
        statistics: FinishedCompilerTaskStatistics,
    },
}

impl From<(FinishedCompilerTask, Vec<TestId>)> for FinishedCompilerTaskWithOutdated {
    fn from((task, outdated): (FinishedCompilerTask, Vec<TestId>)) -> Self {
        match task {
            FinishedCompilerTask::BuildFailed { info, build_output } => Self::BuildFailed {
                info,
                build_output,
                outdated,
            },
            FinishedCompilerTask::RanTests {
                info,
                build_output,
                tests,
            } => {
                let statistics = tests.as_slice().into();
                Self::RanTests {
                    info,
                    build_output,
                    tests,
                    outdated,
                    statistics,
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiFinishedCompilerTaskSummary {
    #[serde(flatten)]
    pub task: FinishedCompilerTaskSummary,
    pub team_name: String,
}
