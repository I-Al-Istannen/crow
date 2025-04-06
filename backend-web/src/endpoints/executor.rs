use super::Json;
use crate::error::{HttpError, Result, WebError};
use crate::types::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::headers::authorization::Basic;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use shared::{
    CompilerTask, CompilerTest, FinishedCompilerTask, RunnerId, RunnerInfo, RunnerUpdate,
    RunnerWorkResponse, RunnerWorkTasteTestDone, RunnerWorkTasteTestResponse, WorkTasteTestTask,
};
use snafu::{ensure, location, IntoError, Location, NoneError, Report, Snafu};
use tokio_util::io::ReaderStream;
use tracing::{info, instrument, warn};

#[derive(Debug, Snafu)]
pub enum ExecutorError {
    #[snafu(display("Runner `{victim}` tried to impersonate `{offender}` at {location}"))]
    RunnerImpersonation {
        victim: String,
        offender: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Runner `{runner_id}` submitted unknown task `{task_id}` for completion at {location}"
    ))]
    UnknownTask {
        task_id: String,
        runner_id: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("There was no work assigned to you we could tar at {location}"))]
    NoWorkWhenTaring {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Runner requested unknown revision `{revision}` at {location}"))]
    UnknownRevisionRequested {
        revision: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to open temporary file for taring at {location}"))]
    WorkTarOpen {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

impl HttpError for ExecutorError {
    fn to_http_code(&self) -> StatusCode {
        match self {
            Self::RunnerImpersonation { .. } => StatusCode::UNAUTHORIZED,
            Self::UnknownTask { .. } => StatusCode::NOT_FOUND,
            Self::NoWorkWhenTaring { .. } => StatusCode::NOT_FOUND,
            Self::UnknownRevisionRequested { .. } => StatusCode::NOT_FOUND,
            Self::WorkTarOpen { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            Self::RunnerImpersonation { .. } => "runner_impersonation",
            Self::UnknownTask { .. } => "unknown_task",
            Self::NoWorkWhenTaring { .. } => "no_work_when_taring",
            Self::UnknownRevisionRequested { .. } => "unknown_revision_requested",
            Self::WorkTarOpen { .. } => "work_tar_open",
        }
    }
}

impl From<ExecutorError> for WebError {
    fn from(e: ExecutorError) -> Self {
        Self::http_error(e, location!())
    }
}

#[instrument(skip_all)]
pub async fn runner_register(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(runner): Json<RunnerInfo>,
) -> Result<()> {
    let runner_id = auth.username().to_string();
    ensure!(
        runner.id.to_string() == runner_id,
        RunnerImpersonationSnafu {
            victim: runner.id.to_string(),
            offender: runner_id,
        }
    );

    state.executor.lock().unwrap().register_runner(&runner);

    Ok(())
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
        return Err(UnknownTaskSnafu {
            task_id: task.info().task_id.clone(),
            runner_id: auth.username().to_string(),
        }
        .into_error(NoneError)
        .into());
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
pub async fn get_work(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Json(runner): Json<RunnerInfo>,
) -> Result<Json<RunnerWorkResponse>> {
    if runner.id.to_string() != auth.username() {
        return Err(RunnerImpersonationSnafu {
            victim: runner.id.to_string(),
            offender: auth.username().to_string(),
        }
        .into_error(NoneError)
        .into());
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
            compile_command: state.execution_config.compile_command.clone(),
            binary_arguments: state.execution_config.binary_arguments.clone(),
            binary_modifiers: test.binary_modifiers,
            compiler_modifiers: test.compiler_modifiers,
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
        .ok_or(
            NoWorkWhenTaringSnafu {}
                .into_error(NoneError)
                .into_weberror(location!()),
        )?;

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
        return Err(UnknownRevisionRequestedSnafu {
            revision: task.revision,
        }
        .into_error(NoneError)
        .into());
    };

    let temp_file = tempfile::NamedTempFile::with_suffix(".tar.gz").unwrap();
    state
        .local_repos
        .export_repo(&repo, temp_file.path(), &revision)
        .await?;

    let file = tokio::fs::File::open(temp_file.path())
        .await
        .map_err(|e| WorkTarOpenSnafu.into_error(e).into_weberror(location!()))?;

    // Delete the file, we have an open file handle to it
    drop(temp_file);

    Ok(Body::from_stream(ReaderStream::new(file)).into_response())
}

#[instrument(skip_all)]
pub async fn get_test_tasting_work(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> Result<Json<RunnerWorkTasteTestResponse>> {
    let Some(image_id) = state.execution_config.reference_compiler_image else {
        return Ok(Json(RunnerWorkTasteTestResponse { task: None }));
    };
    let runner_id = auth.username().to_string().into();

    let task = state.test_tasting.lock().unwrap().poll_tasting(runner_id);
    let task = task.map(|task| WorkTasteTestTask {
        id: task.taste_id.clone(),
        test: CompilerTest {
            test_id: task.test.id.to_string(),
            timeout: state.execution_config.test_timeout,
            compile_command: state.execution_config.compile_command,
            binary_arguments: state.execution_config.binary_arguments,
            compiler_modifiers: task.test.compiler_modifiers,
            binary_modifiers: task.test.binary_modifiers,
        },
        image_id,
    });

    Ok(Json(RunnerWorkTasteTestResponse { task }))
}

#[instrument(skip_all)]
pub async fn taste_testing_done(
    State(state): State<AppState>,
    Json(payload): Json<RunnerWorkTasteTestDone>,
) -> Result<()> {
    state
        .test_tasting
        .lock()
        .unwrap()
        .finish_tasting(payload.id, payload.output);

    Ok(())
}
