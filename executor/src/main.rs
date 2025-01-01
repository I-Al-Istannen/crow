// We do not care about the extra few ns/ms it takes to copy the errors around, the callstacks
// are not deep and the other operations much more expensive.
#![allow(clippy::result_large_err)]

use crate::containers::{ContainerCreateError, TestRunError, WaitForContainerError};
use crate::executor::{execute_task, ExecutingTask};
use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::Parser;
use rayon::ThreadPoolBuilder;
use reqwest::blocking::ClientBuilder;
use reqwest::StatusCode;
use shared::{RunnerInfo, RunnerWorkResponse};
use snafu::{Location, Report, ResultExt, Snafu};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod containers;
mod docker;
mod executor;

#[derive(Debug, Snafu)]
pub enum AnyError {
    #[snafu(display("Could not run container"))]
    Run { source: std::io::Error },
    #[snafu(display("Could not create the build container at {location}"))]
    Create {
        source: ContainerCreateError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not wait for build at {location}"))]
    WaitForBuild {
        source: WaitForContainerError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not run test at {location}"))]
    TestRun {
        source: TestRunError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not build thread pool at {location}"))]
    ThreadPoolBuild {
        source: rayon::ThreadPoolBuildError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not build reqwest client at {location}"))]
    Reqwest {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not create temp file at {location}"))]
    TempFile {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

const CLAP_STYLE: Styles = Styles::styled()
    .header(AnsiColor::Red.on_default().bold())
    .usage(AnsiColor::Red.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Green.on_default());

const NO_TASK_BACKOFF: Duration = Duration::from_secs(2);

/// Executor of compiler tasks, crow of judgement.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles = CLAP_STYLE)]
struct Args {
    /// A unique name for this runner
    id: String,
    /// The runner token of the server
    token: String,
    /// Endpoint to poll for work updates
    endpoint: String,
}

fn main() -> Report<AnyError> {
    Report::capture(|| {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .init();

        let args = Args::parse();
        let work_endpoint = format!("{}/executor/request-work", args.endpoint);
        let tar_endpoint = format!("{}/executor/request-tar", args.endpoint);
        let done_endpoint = format!("{}/executor/done", args.endpoint);
        let ping_endpoint = format!("{}/executor/ping", args.endpoint);

        let thread_pool = ThreadPoolBuilder::new()
            .build()
            .context(ThreadPoolBuildSnafu)?;

        let mut current_backoff = Duration::from_secs(1);
        let stop_requested = Arc::new(AtomicBool::new(false));

        register_termination_handler(&stop_requested);

        let client = ClientBuilder::new().build().context(ReqwestSnafu)?;

        while !stop_requested.load(Ordering::Relaxed) {
            let runner_info = RunnerInfo {
                id: args.id.clone().into(),
                info: "hey".to_string(),
                current_task: None,
            };
            client
                .post(&ping_endpoint)
                .basic_auth(&args.id, Some(&args.token))
                .json(&runner_info)
                .send()
                .context(ReqwestSnafu)?;

            let response = match client
                .post(&work_endpoint)
                .json(&runner_info)
                .basic_auth(&args.id, Some(&args.token))
                .send()
            {
                Err(e) => {
                    warn!(
                        error = ?Report::from_error(e),
                        endpoint = %work_endpoint,
                        next_retry = ?current_backoff,
                        "Failed to poll endpoint"
                    );
                    backoff(&mut current_backoff, &stop_requested);
                    continue;
                }
                Ok(response) => response,
            };
            if !response.status().is_success() || response.status() == StatusCode::NOT_FOUND {
                warn!(
                    status = %response.status(),
                    endpoint = %work_endpoint,
                    next_retry = ?current_backoff,
                    "Failed to poll endpoint"
                );
                backoff(&mut current_backoff, &stop_requested);
                continue;
            }
            let task = match response.json::<RunnerWorkResponse>() {
                Err(e) => {
                    warn!(
                        error = ?Report::from_error(e),
                        endpoint = %work_endpoint,
                        next_retry = ?current_backoff,
                        "Failed to parse task"
                    );
                    backoff(&mut current_backoff, &stop_requested);
                    continue;
                }
                Ok(task) => task,
            };
            let Some(task) = task.task else {
                let current_backoff = &mut NO_TASK_BACKOFF.clone();
                info!(backoff = ?current_backoff, "No task received");
                backoff(current_backoff, &stop_requested);
                continue;
            };
            let task_id = task.task_id.clone();
            info!(id = task_id, "Received task");
            let task = ExecutingTask {
                inner: task,
                pool: &thread_pool,
                aborted: stop_requested.clone(),
            };
            let source_tar = tempfile::NamedTempFile::new().context(TempFileSnafu)?;
            client
                .get(&tar_endpoint)
                .basic_auth(&args.id, Some(&args.token))
                .send()
                .context(ReqwestSnafu)?
                .copy_to(&mut source_tar.as_file())
                .context(ReqwestSnafu)?;
            let res = execute_task(task, source_tar.into_temp_path());
            info!(id = task_id, res = ?res, "Task finished");
            client
                .post(&done_endpoint)
                .json(&res)
                .basic_auth(&args.id, Some(&args.token))
                .send()
                .context(ReqwestSnafu)?;
        }

        info!("Goodbye!");

        Ok(())
    })
}

fn register_termination_handler(stop_requested: &Arc<AtomicBool>) {
    let stop_requested_clone = stop_requested.clone();
    let ctrlc_result = ctrlc::set_handler(move || {
        stop_requested_clone.store(true, Ordering::Relaxed);
    });

    if let Err(e) = ctrlc_result {
        warn!(
            error = ?e,
            "Could not register termination handler, program behaviour on SIGINT/SIGTERM is undefined"
        );
    }
}

fn backoff(current_backoff: &mut Duration, stop_requested: &Arc<AtomicBool>) {
    // We need to be responsive to stop requests (CTRL+C), so we can't just sleep for
    // the full duration
    let target = Instant::now() + *current_backoff;
    while Instant::now() < target && !stop_requested.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(100));
    }
    *current_backoff *= 2;
    *current_backoff = (*current_backoff).min(Duration::from_secs(60));
}
