use crate::auth::oidc::Oidc;
use crate::auth::{Claims, Keys};
use crate::config::Config;
use crate::db::Database;
use crate::endpoints::{
    delete_test, executor_info, get_final_tasks, get_integration_status, get_n_recent_tasks,
    get_queue, get_queued_task, get_recent_tasks, get_running_task_info, get_task, get_team_info,
    get_team_repo, get_test, get_test_tasting_work, get_top_task_per_team, get_work, get_work_tar,
    head_running_task_info, integration_get_task_status, integration_request_revision, list_tests,
    list_users, login_oidc, login_oidc_callback, request_revision, runner_done, runner_ping,
    runner_register, runner_update, set_final_task, set_team_repo, set_test, show_me_myself,
    taste_testing_done,
};
use crate::error::WebError;
use crate::storage::LocalRepos;
use crate::types::{AppState, TeamId, UserId, UserRole};
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::{delete, get, head, post, put};
use axum::{middleware, Router};
use axum_extra::headers::authorization::Basic;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use axum_prometheus::metrics_exporter_prometheus::PrometheusHandle;
use axum_prometheus::{GenericMetricLayer, Handle, PrometheusMetricLayerBuilder};
use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::Parser;
use snafu::{location, Report};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::{env, fs};
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, instrument, warn, Instrument, Span};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

mod auth;
mod config;
mod db;
mod endpoints;
mod error;
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

    let args = Args::parse();
    let config_file = args.config_file;
    if !config_file.exists() || !config_file.is_file() {
        eprintln!("Config file {config_file:?} does not exist or is not a file!");
        std::process::exit(1);
    }

    let config: Config =
        toml::from_str(&fs::read_to_string(config_file).expect("File not readable"))
            .expect("Config file is valid");

    let db = Database::new(&config.database_path).await.unwrap();

    db.sync_teams(&config.teams).await.unwrap();
    let team_mapping: HashMap<UserId, TeamId> = config
        .teams
        .into_iter()
        .flat_map(|team| {
            team.members
                .into_iter()
                .map(move |user| (user, team.id.clone()))
        })
        .collect();

    let local_repo_path = config.execution.local_repo_path.clone();
    let state = AppState::new(
        db,
        Keys::new(config.jwt_secret.as_bytes()),
        config.github.as_ref().map(|it| it.app_name.to_string()),
        config.execution,
        config.test,
        team_mapping,
        LocalRepos::new(local_repo_path, config.ssh),
        Oidc::build_new(config.oidc.clone()).await.unwrap(),
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

    tokio::join!(main, server);
}

#[instrument(skip_all)]
#[allow(clippy::type_complexity)]
async fn main_server(
    state: AppState,
    prometheus_layer: GenericMetricLayer<'static, PrometheusHandle, Handle>,
) {
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
        .route("/team/recent-tasks", get(get_recent_tasks))
        .route("/team/recent-tasks/:count", get(get_n_recent_tasks))
        .route("/team/final-tasks", get(get_final_tasks))
        .route("/team/final-tasks", put(set_final_task))
        .route("/tests", get(list_tests))
        .route("/tests/:test_id", delete(delete_test))
        .route("/tests/:test_id", get(get_test))
        .route("/tests/:test_id", put(set_test))
        .route("/top-tasks", get(get_top_task_per_team))
        .route("/users", get(list_users).layer(authed_admin))
        .route("/users/me", get(show_me_myself))
        .route("/users/me/integrations", get(get_integration_status))
        .route("/login", get(login_oidc))
        .route("/login/oidc/callback", post(login_oidc_callback))
        .layer(prometheus_layer)
        .layer(CorsLayer::very_permissive()) // TODO: Make nicer
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async { graceful_shutdown().await }.instrument(Span::current()))
    .await
    .unwrap()
}

#[instrument(skip_all)]
async fn metrics_server(metric_handle: PrometheusHandle) {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4317").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(
        listener,
        Router::new()
            .route("/metrics", get(|| async move { metric_handle.render() }))
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async { graceful_shutdown().await }.instrument(Span::current()))
    .await
    .unwrap()
}

async fn graceful_shutdown() {
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let interrupt = tokio::signal::ctrl_c();
    select! {
        _ = sigterm.recv() => warn!("Received SIGTERM"),
        _ = interrupt => warn!("Received SIGINT")
    }
}
