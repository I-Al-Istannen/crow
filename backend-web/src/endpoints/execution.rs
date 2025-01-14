use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{
    AppState, ExecutorInfo, QueuedTaskStatus, RunnerForFrontend, TaskId, TeamId, WorkItem,
};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum_extra::headers::authorization::{Basic, Bearer};
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use serde::Serialize;
use serde_json::json;
use shared::{
    CompilerTask, CompilerTest, FinishedCompilerTask, RunnerId, RunnerInfo, RunnerRegisterResponse,
    RunnerUpdate, RunnerWorkResponse,
};
use snafu::Report;
use std::time::SystemTime;
use tokio_util::io::ReaderStream;
use tracing::{info, instrument, warn};
use uuid::Uuid;

#[instrument(skip_all)]
pub async fn integration_request_revision(
    State(state): State<AppState>,
    TypedHeader(Authorization(auth)): TypedHeader<Authorization<Bearer>>,
    Path(revision): Path<String>,
) -> Result<Response> {
    let token = auth.token().to_string().into();
    let Some(team_id) = state.db.fetch_team_by_integration_token(&token).await? else {
        return Err(WebError::InvalidCredentials);
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
        return Err(WebError::InvalidCredentials);
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
    let Some(team) = state.db.get_user(&claims.sub).await?.user.team else {
        return Err(WebError::NotInTeam);
    };

    queue_task(state, &revision, team).await
}

async fn queue_task(state: AppState, revision: &str, team: TeamId) -> Result<Response> {
    // Update repo to ensure revision is present
    let repo = state.db.get_repo(&team).await?;
    state.local_repos.update_repo(&repo).await?;
    let Some(revision) = state.local_repos.get_revision(&repo, revision).await? else {
        return Err(WebError::NotFound);
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
        return Err(WebError::NotFound);
    }
    let Some(item) = state.db.fetch_queued_task(&task_id).await? else {
        return Err(WebError::NotFound);
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
pub async fn list_task_ids(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<Vec<TaskId>>> {
    Ok(Json(state.db.get_task_ids().await?))
}

#[instrument(skip_all)]
pub async fn runner_register(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(runner): Json<RunnerInfo>,
) -> Result<Json<RunnerRegisterResponse>> {
    let runner_id = auth.username().to_string();
    if runner.id.to_string() != runner_id {
        return Err(WebError::InvalidCredentials);
    }

    let task = state.executor.lock().unwrap().register_runner(&runner);
    let current_task = runner.current_task.map(|it| it.into());

    if task != current_task {
        info!(runner = %runner.id, task = ?task, "Runner task changed, resetting it");
        return Ok(Json(RunnerRegisterResponse { reset: true }));
    }

    Ok(Json(RunnerRegisterResponse { reset: false }))
}

#[instrument(skip_all)]
pub async fn runner_update(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(update): Json<RunnerUpdate>,
) -> Result<()> {
    let runner_id: RunnerId = auth.username().to_string().into();

    // TODO: Think about protocol errors more
    info!(runner = %runner_id, update = ?update, "Runner update");
    state
        .executor
        .lock()
        .unwrap()
        .update_task(&runner_id, update.into());

    Ok(())
}

#[instrument(skip_all)]
pub async fn runner_done(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(task): Json<FinishedCompilerTask>,
) -> Result<()> {
    println!("{}", serde_json::to_string(&task).unwrap());

    if state
        .executor
        .lock()
        .unwrap()
        .get_running_task(&task.info().task_id.clone().into())
        .is_none()
    {
        warn!(
            task = %task.info().task_id,
            runner_id = %auth.username(),
            "Runner submitted unknown task for completion"
        );
        return Err(WebError::NotFound);
    }

    state.db.add_finished_task(&task).await?;
    state
        .executor
        .lock()
        .unwrap()
        .finish_task(&auth.username().to_string().into());

    Ok(())
}

#[instrument(skip_all)]
pub async fn runner_ping(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> Result<()> {
    state
        .executor
        .lock()
        .unwrap()
        .runner_pinged(&auth.username().to_string().into());
    Ok(())
}

#[instrument(skip_all)]
pub async fn executor_info(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<ExecutorInfo>> {
    Ok(Json(state.executor.lock().unwrap().info()))
}

#[instrument(skip_all)]
pub async fn get_work(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(runner): Json<RunnerInfo>,
) -> Result<Json<RunnerWorkResponse>> {
    if runner.id.to_string() != auth.username() {
        return Err(WebError::InvalidCredentials);
    }
    if let Some(task) = runner.current_task {
        warn!(runner = %runner.id, task = %task, "Runner already has a task, resetting it");
        return Ok(Json(RunnerWorkResponse {
            task: None,
            reset: true,
        }));
    }

    let queue = state.db.get_queued_tasks().await?;
    let tests: Vec<CompilerTest> = state
        .db
        .get_tests()
        .await?
        .into_iter()
        .map(|test| CompilerTest {
            test_id: test.id.to_string(),
            timeout: state.execution_config.test_timeout,
            run_command: state.execution_config.test_command.clone(),
            expected_output: test.expected_output,
        })
        .collect();

    let test_ids = tests
        .iter()
        .map(|it| it.test_id.clone().into())
        .collect::<Vec<_>>();

    let task = match state
        .executor
        .lock()
        .unwrap()
        .assign_work(&runner, &queue, test_ids)
    {
        Err(e) => {
            warn!(
                error = %Report::from_error(e),
                runner = %runner.id,
                "Error assigning work to runner, resetting it"
            );
            return Ok(Json(RunnerWorkResponse {
                task: None,
                reset: true,
            }));
        }
        Ok(task) => task,
    };

    let Some(task) = task else {
        return Ok(Json(RunnerWorkResponse {
            task: None,
            reset: false,
        }));
    };

    // FIXME: Replace
    let task = CompilerTask {
        task_id: task.id.to_string(),
        team_id: task.team.to_string(),
        revision_id: task.revision.to_string(),
        commit_message: task.commit_message.clone(),
        image: "alpine:latest".to_string(),
        build_command: state.execution_config.build_command.clone(),
        build_timeout: state.execution_config.build_timeout,
        tests,
    };

    Ok(Json(RunnerWorkResponse {
        task: Some(task),
        reset: false,
    }))
}

#[instrument(skip_all)]
pub async fn get_work_tar(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> Result<Response> {
    let task = state
        .executor
        .lock()
        .unwrap()
        .get_current_task(&auth.username().to_string().into())
        .ok_or(WebError::NotFound)?;

    let repo = state.db.get_repo(&task.team).await?;
    let Some(revision) = state
        .local_repos
        .get_revision(&repo, &task.revision)
        .await?
    else {
        let runner_name = auth.username().to_string();
        warn!(
            task = %task.id,
            revision = %task.revision,
            runner_id = %runner_name,
            "Requested unknown revision"
        );
        return Err(WebError::NotFound);
    };

    let temp_file = tempfile::NamedTempFile::with_suffix(".tar.gz").unwrap();
    state
        .local_repos
        .export_repo(&repo, temp_file.path(), &revision)
        .await?;

    let file = tokio::fs::File::open(temp_file.path())
        .await
        .map_err(|e| WebError::InternalServerError(e.to_string()))?;

    // Delete the file, we have an open file handle to it
    drop(temp_file);

    Ok(Body::from_stream(ReaderStream::new(file)).into_response())
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
