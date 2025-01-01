use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::WebError;
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
use tokio_util::io::ReaderStream;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn request_revision(
    State(state): State<AppState>,
    claims: Claims,
    Path(revision): Path<String>,
) -> Result<Response, WebError> {
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
    };
    state.db.queue_task(task.clone()).await?;

    Ok(Json(json!({ "task_id": task_id })).into_response())
}

pub async fn get_queued_tasks(
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkItem>>, WebError> {
    Ok(Json(state.db.get_queued_tasks().await?))
}

pub async fn get_task(
    State(state): State<AppState>,
    _claims: Claims,
    Path(task_id): Path<TaskId>,
) -> Result<Json<FinishedCompilerTask>, WebError> {
    Ok(Json(state.db.get_task(&task_id).await?))
}

pub async fn list_task_ids(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<Vec<TaskId>>, WebError> {
    Ok(Json(state.db.get_task_ids().await?))
}

pub async fn runner_register(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(runner): Json<RunnerInfo>,
) -> Result<Json<RunnerRegisterResponse>, WebError> {
    let runner_id = auth.username().to_string();
    if runner.id.to_string() != runner_id {
        return Err(WebError::InvalidCredentials);
    }

    let task = state.executor.lock().unwrap().update_runner(&runner);
    let current_task = runner.current_task.map(|it| it.into());

    if task != current_task {
        info!(runner = %runner.id, task = ?task, "Runner task changed, resetting it");
        return Ok(Json(RunnerRegisterResponse { reset: true }));
    }

    Ok(Json(RunnerRegisterResponse { reset: false }))
}

pub async fn runner_update(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(update): Json<RunnerUpdate>,
) -> Result<(), WebError> {
    let runner_id: RunnerId = auth.username().to_string().into();

    // TODO: Handle update
    info!(runner = %runner_id, update = ?update, "Runner update");

    Ok(())
}

pub async fn get_work(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(runner): Json<RunnerInfo>,
) -> Result<Json<RunnerWorkResponse>, WebError> {
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

pub async fn get_work_tar(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> Result<Response, WebError> {
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

pub async fn runner_done(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(task): Json<FinishedCompilerTask>,
) -> Result<(), WebError> {
    let task_id = state
        .executor
        .lock()
        .unwrap()
        .get_current_task(&auth.username().to_string().into());
    let Some(task_id) = task_id else {
        return Err(WebError::NotFound);
    };
    let task_id = task_id.id;

    state.db.add_finished_task(&task_id, &task).await?;
    state
        .executor
        .lock()
        .unwrap()
        .finish_task(&auth.username().to_string().into());

    Ok(())
}
