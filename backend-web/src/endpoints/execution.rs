use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::WebError;
use crate::types::{AppState, TaskId, WorkItem};
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use serde_json::json;
use shared::{CompilerTask, CompilerTest, FinishedCompilerTask};
use uuid::Uuid;

pub async fn request_revision(
    State(state): State<AppState>,
    claims: Claims,
    Path(revision): Path<String>,
) -> Result<Response, WebError> {
    let Some(team) = state.db.get_user(&claims.sub).await?.user.team else {
        return Err(WebError::NotInTeam);
    };

    let task_id: TaskId = Uuid::new_v4().to_string().into();
    let task = WorkItem {
        id: task_id.clone(),
        team,
        revision,
    };
    state.db.queue_task(task.clone()).await?;
    state.executor.lock().unwrap().add_task(task);

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

pub async fn get_work(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<CompilerTask>, WebError> {
    if state.execution_config.runner_token != auth.token() {
        return Err(WebError::InvalidCredentials);
    }

    let Some(task) = state.executor.lock().unwrap().pop_task() else {
        return Err(WebError::NotFound);
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

    Ok(Json(task))
}

pub async fn get_work_tar(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(task_id): Path<TaskId>,
) -> Result<(), WebError> {
    if state.execution_config.runner_token != auth.token() {
        return Err(WebError::InvalidCredentials);
    }

    let _task = state
        .executor
        .lock()
        .unwrap()
        .get_running_task(&task_id)
        .ok_or(WebError::NotFound)?;

    // TODO: Return task tar here

    Ok(())
}

pub async fn runner_done(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(task_id): Path<TaskId>,
    Json(task): Json<FinishedCompilerTask>,
) -> Result<(), WebError> {
    if state.execution_config.runner_token != auth.token() {
        return Err(WebError::InvalidCredentials);
    }

    state.db.add_finished_task(&task_id, &task).await?;
    state.executor.lock().unwrap().finish_task(&task_id, &task);

    Ok(())
}
