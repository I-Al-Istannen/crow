use crate::auth::oidc::Oidc;
use crate::auth::{Claims, Keys};
use crate::config::{Config, TeamEntry};
use crate::db::Database;
use crate::endpoints::{
    delete_test, executor_info, get_final_tasks, get_integration_status, get_n_recent_tasks,
    get_queue, get_queued_task, get_recent_tasks, get_running_task_info, get_task,
    get_tasks_for_team, get_team_info, get_team_repo, get_test, get_test_tasting_work,
    get_top_task_per_team, get_work, get_work_tar, head_running_task_info,
    integration_get_task_status, integration_request_revision, list_tests, list_users, login_oidc,
    login_oidc_callback, rehash_tests, request_revision, rerun_submissions, runner_done,
    runner_ping, runner_register, runner_update, set_final_task, set_team_repo, set_test,
    show_me_myself, snapshot_state, taste_testing_done, team_statistics,
};
use crate::error::WebError;
use crate::storage::LocalRepos;
use crate::types::{AppState, TeamId, UserId, UserRole};
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::{delete, get, head, post, put};
use axum::{Router, middleware};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Basic;
use axum_prometheus::metrics_exporter_prometheus::PrometheusHandle;
use axum_prometheus::{GenericMetricLayer, Handle, PrometheusMetricLayerBuilder};
use clap::Parser;
use clap::builder::Styles;
use clap::builder::styling::AnsiColor;
use snafu::{Report, ResultExt, Whatever, location};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::{env, fs};
use tokio::select;
use tokio::signal::unix::{SignalKind, signal};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{Instrument, Span, error, instrument, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

mod auth;
mod config;
mod db;
mod endpoints;
mod error;
mod grading_formulas;
mod integration;
mod storage;
mod types;

// noinspection DuplicatedCode
const CLAP_STYLE: Styles = Styles::styled()
    .header(AnsiColor::Red.on_default().bold())
    .usage(AnsiColor::Red.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Green.on_default());

/// Webserver for managing compiler submissions
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles = CLAP_STYLE)]
struct Args {
    /// Path to the config file
    config_file: PathBuf,
}

fn logger_config() -> Box<dyn Layer<Registry> + Send + Sync> {
    match env::var("LOG_FORMAT")
        .unwrap_or("plain".to_string())
        .as_str()
    {
        "json" => Box::new(tracing_subscriber::fmt::layer().json()),
        _ => Box::new(tracing_subscriber::fmt::layer()),
    }
}

#[tokio::main]
async fn main() {
    // Maybe: https://fasterthanli.me/articles/request-coalescing-in-async-rust#a-bit-of-tracing
    tracing_subscriber::registry()
        .with(logger_config())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    if let Err(e) = main_impl().await {
        error!(
            error = %Report::from_error(e),
            "Error in main"
        );
        std::process::exit(1);
    }
}

async fn main_impl() -> Result<(), Whatever> {
    let args = Args::parse();
    let config_file = args.config_file;
    if !config_file.exists() || !config_file.is_file() {
        eprintln!("Config file {config_file:?} does not exist or is not a file!");
        std::process::exit(1);
    }

    let config: Config =
        toml::from_str(&fs::read_to_string(config_file).whatever_context("File not readable")?)
            .whatever_context("Invalid config")?;

    let db = Database::new(&config.database_path)
        .await
        .whatever_context("Database error")?;

    db.sync_teams(&config.teams)
        .await
        .whatever_context("Error syncing teams")?;

    let local_repo_path = config.execution.local_repo_path.clone();
    let state = AppState::new(
        db,
        Keys::new(config.jwt_secret.as_bytes()),
        config.github.as_ref().map(|it| it.app_name.to_string()),
        config.execution,
        config.grading,
        config.test,
        get_team_mapping(config.teams),
        LocalRepos::new(local_repo_path, config.ssh),
        Oidc::build_new(config.oidc.clone())
            .await
            .whatever_context("OIDC error")?,
    );

    if let Some(github_config) = config.github.clone() {
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = integration::run_github_app(github_config, state).await {
                error!(
                    error = %Report::from_error(e),
                    "Fatal error in GitHub handler, functionality disabled"
                );
            }
        });
    }

    let (prometheus_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
        .with_prefix("compilers-backend")
        .with_default_metrics()
        .build_pair();

    let main = main_server(state, prometheus_layer);
    let server = metrics_server(metric_handle);

    let (a, b) = tokio::join!(main, server);

    a?;
    b
}

fn get_team_mapping(teams: Vec<TeamEntry>) -> HashMap<UserId, (TeamId, UserRole)> {
    teams
        .into_iter()
        .flat_map(|team| {
            let role = if team.is_admin {
                UserRole::Admin
            } else {
                UserRole::Regular
            };
            team.members
                .into_iter()
                .map(move |user| (user, (team.id.clone(), role)))
        })
        .collect()
}

#[instrument(skip_all)]
#[allow(clippy::type_complexity)]
async fn main_server(
    state: AppState,
    prometheus_layer: GenericMetricLayer<'static, PrometheusHandle, Handle>,
) -> Result<(), Whatever> {
    let authed_admin = middleware::from_fn_with_state(
        state.clone(),
        |claims: Claims, request: Request, next: Next| async move {
            if claims.role != UserRole::Admin {
                return WebError::unauthorized(location!()).into_response();
            };
            next.run(request).await
        },
    );
    let authed_runner = middleware::from_fn_with_state(
        state.clone(),
        |TypedHeader(header): TypedHeader<Authorization<Basic>>,
         State(state): State<AppState>,
         request: Request,
         next: Next| async move {
            let token = header.0.password();
            if token != state.execution_config.runner_token {
                return WebError::invalid_credentials(location!()).into_response();
            }
            next.run(request).await
        },
    );

    let app = Router::new()
        .route(
            "/executor/done",
            post(runner_done).layer(authed_runner.clone()),
        )
        .route(
            "/executor/done-taste-test",
            post(taste_testing_done).layer(authed_runner.clone()),
        )
        .route(
            "/executor/info",
            get(executor_info).layer(authed_admin.clone()),
        )
        .route(
            "/executor/ping",
            post(runner_ping).layer(authed_runner.clone()),
        )
        .route(
            "/executor/register",
            post(runner_register).layer(authed_runner.clone()),
        )
        .route(
            "/executor/request-tar",
            get(get_work_tar).layer(authed_runner.clone()),
        )
        .route(
            "/executor/request-work",
            post(get_work).layer(authed_runner.clone()),
        )
        .route(
            "/executor/request-work-taste-test",
            post(get_test_tasting_work).layer(authed_runner.clone()),
        )
        .route(
            "/executor/update",
            post(runner_update).layer(authed_runner.clone()),
        )
        .route(
            "/integration/token/queue/rev/:revision",
            put(integration_request_revision),
        )
        .route(
            "/integration/token/task/:task_id",
            get(integration_get_task_status),
        )
        .route("/queue", get(get_queue))
        .route("/queue/rev/:revision", put(request_revision))
        .route("/queue/task/:task_id", get(get_queued_task))
        .route("/repo/:team_id", get(get_team_repo))
        .route("/repo/:team_id", put(set_team_repo))
        .route("/tasks/:task_id", get(get_task))
        .route("/tasks/:task_id/stream", get(get_running_task_info))
        .route("/tasks/:task_id/stream", head(head_running_task_info))
        .route("/team/info/:team_id", get(get_team_info))
        .route(
            "/team/tasks/:team_id",
            get(get_tasks_for_team).layer(authed_admin.clone()),
        )
        .route("/team/recent-tasks", get(get_recent_tasks))
        .route("/team/recent-tasks/:count", get(get_n_recent_tasks))
        .route("/team/final-tasks", get(get_final_tasks))
        .route("/team/final-tasks", put(set_final_task))
        .route("/tests", get(list_tests))
        .route("/tests/:test_id", delete(delete_test))
        .route("/tests/:test_id", get(get_test))
        .route("/tests/:test_id", put(set_test))
        .route("/top-tasks", get(get_top_task_per_team))
        .route("/users", get(list_users).layer(authed_admin.clone()))
        .route("/users/me", get(show_me_myself))
        .route("/users/me/integrations", get(get_integration_status))
        .route(
            "/admin/snapshot",
            post(snapshot_state).layer(authed_admin.clone()),
        )
        .route(
            "/admin/rerun_submissions/:category",
            post(rerun_submissions).layer(authed_admin.clone()),
        )
        .route(
            "/admin/rehash_tests",
            post(rehash_tests).layer(authed_admin.clone()),
        )
        .route(
            "/admin/team_statistics",
            get(team_statistics).layer(authed_admin.clone()),
        )
        .route("/login", get(login_oidc))
        .route("/login/oidc/callback", post(login_oidc_callback))
        .layer(DefaultBodyLimit::max(25 * 1024 * 1024)) // 25 MiB
        .layer(prometheus_layer)
        .layer(CorsLayer::very_permissive()) // TODO: Make nicer
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .whatever_context("Failed to bind to port 3000")?;
    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async { graceful_shutdown().await }.instrument(Span::current()))
    .await
    .whatever_context("Failed to start server")
}

#[instrument(skip_all)]
async fn metrics_server(metric_handle: PrometheusHandle) -> Result<(), Whatever> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4317")
        .await
        .whatever_context("Failed to bind to port 4317")?;
    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(
        listener,
        Router::new()
            .route("/metrics", get(|| async move { metric_handle.render() }))
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async { graceful_shutdown().await }.instrument(Span::current()))
    .await
    .whatever_context("Failed to start metrics server")
}

async fn graceful_shutdown() {
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let interrupt = tokio::signal::ctrl_c();
    select! {
        _ = sigterm.recv() => warn!("Received SIGTERM"),
        _ = interrupt => warn!("Received SIGINT")
    }
}
