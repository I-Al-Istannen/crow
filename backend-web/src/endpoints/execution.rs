use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{AppState, TaskId, WorkItem};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum_extra::headers::authorization::Basic;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
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
pub async fn request_revision(
    State(state): State<AppState>,
    claims: Claims,
    Path(revision): Path<String>,
) -> Result<Response> {
    let Some(team) = state.db.get_user(&claims.sub).await?.user.team else {
        return Err(WebError::NotInTeam);
    };

    // Update repo to ensure revision is present
    let repo = state.db.get_repo(&team).await?;
    state.local_repos.update_repo(&repo).await?;

    let task_id: TaskId = Uuid::new_v4().to_string().into();
    let task = WorkItem {
        id: task_id.clone(),
        team,
        revision,
        insert_time: SystemTime::now(),
    };
    state.db.queue_task(task.clone()).await?;

    Ok(Json(json!({ "taskId": task_id })).into_response())
}

#[instrument(skip_all)]
pub async fn get_queued_tasks(State(state): State<AppState>) -> Result<Json<Vec<WorkItem>>> {
    // sleep 1s
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(Json(state.db.get_queued_tasks().await?))
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
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(update): Json<RunnerUpdate>,
) -> Result<()> {
    let runner_id: RunnerId = auth.username().to_string().into();

    // TODO: Handle update
    info!(runner = %runner_id, update = ?update, "Runner update");

    Ok(())
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

    let task = match state.executor.lock().unwrap().assign_work(&runner, &queue) {
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

    let tests = state
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

    // FIXME: Replace
    let task = CompilerTask {
        task_id: task.id.to_string(),
        team_id: task.team.to_string(),
        revision_id: task.revision,
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

    let temp_file = tempfile::NamedTempFile::with_suffix(".tar.gz").unwrap();
    state
        .local_repos
        .export_repo(&repo, temp_file.path(), &task.revision)
        .await?;

    let file = tokio::fs::File::open(temp_file.path())
        .await
        .map_err(|e| WebError::InternalServerError(e.to_string()))?;

    // Delete the file, we have an open file handle to it
    drop(temp_file);

    Ok(Body::from_stream(ReaderStream::new(file)).into_response())
}

#[instrument(skip_all)]
pub async fn runner_done(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(task): Json<FinishedCompilerTask>,
) -> Result<()> {
    println!("{}", serde_json::to_string(&task).unwrap());

    state.db.add_finished_task(&task).await?;
    state
        .executor
        .lock()
        .unwrap()
        .finish_task(&auth.username().to_string().into());

    Ok(())
}
