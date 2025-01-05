// We do not care about the extra few ns/ms it takes to copy the errors around, the callstacks
// are not deep and the other operations much more expensive.
#![allow(clippy::result_large_err)]

use crate::containers::{ContainerCreateError, TestRunError, WaitForContainerError};
use crate::executor::{execute_task, ExecutingTask};
use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::Parser;
use rayon::{ThreadPool, ThreadPoolBuilder};
use reqwest::blocking::{Client, ClientBuilder};
use shared::{RunnerInfo, RunnerUpdate, RunnerWorkResponse};
use snafu::{Location, Report, ResultExt, Snafu};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread;
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

// noinspection DuplicatedCode
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

struct Endpoints {
    done: String,
    ping: String,
    register: String,
    tar: String,
    update: String,
    work: String,
}

impl Endpoints {
    pub fn new(base: &str) -> Self {
        Self {
            done: format!("{}/executor/done", base),
            ping: format!("{}/executor/ping", base),
            register: format!("{}/executor/register", base),
            tar: format!("{}/executor/request-tar", base),
            update: format!("{}/executor/update", base),
            work: format!("{}/executor/request-work", base),
        }
    }
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
        let endpoints = Endpoints::new(&args.endpoint);

        let thread_pool = ThreadPoolBuilder::new()
            .build()
            .context(ThreadPoolBuildSnafu)?;

        let mut current_backoff = Duration::from_secs(1);
        let shutdown_requested = Arc::new(AtomicBool::new(false));

        register_termination_handler(&shutdown_requested);
        start_periodic_pings(&endpoints, &args);

        let client = ClientBuilder::new().build().context(ReqwestSnafu)?;

        while !shutdown_requested.load(Ordering::Relaxed) {
            let res = iteration(
                &args,
                &endpoints,
                &thread_pool,
                &mut current_backoff,
                &shutdown_requested,
                &client,
            );
            if let Err(e) = res {
                // Emergency wait to prevent busy loops
                let mut emergency_backoff = Duration::from_secs(5);
                warn!(
                    backoff = ?emergency_backoff,
                    error = ?Report::from_error(e),
                    "Error during iteration"
                );
                backoff(&mut emergency_backoff, &shutdown_requested);
            }
        }

        info!("Goodbye!");

        Ok(())
    })
}

fn start_periodic_pings(endpoints: &Endpoints, args: &Args) {
    let id = args.id.clone();
    let token = args.token.clone();
    let url = endpoints.ping.clone();
    thread::spawn(move || {
        let client = Client::new();
        loop {
            thread::sleep(Duration::from_secs(15));
            let _ = client
                .post(&url)
                .basic_auth(&id, Some(&token))
                .send()
                .context(ReqwestSnafu);
        }
    });
}

fn iteration(
    args: &Args,
    endpoints: &Endpoints,
    thread_pool: &ThreadPool,
    current_backoff: &mut Duration,
    shutdown_requested: &Arc<AtomicBool>,
    client: &Client,
) -> Result<(), AnyError> {
    let runner_info = RunnerInfo {
        id: args.id.clone().into(),
        info: "hey".to_string(),
        current_task: None,
    };
    client
        .post(&endpoints.register)
        .basic_auth(&args.id, Some(&args.token))
        .json(&runner_info)
        .send()
        .context(ReqwestSnafu)?;

    let response = match client
        .post(&endpoints.work)
        .json(&runner_info)
        .basic_auth(&args.id, Some(&args.token))
        .send()
    {
        Err(e) => {
            warn!(
                error = ?Report::from_error(e),
                endpoint = %endpoints.work,
                next_retry = ?current_backoff,
                "Failed to request work"
            );
            backoff(current_backoff, shutdown_requested);
            return Ok(());
        }
        Ok(response) => response,
    };
    if !response.status().is_success() {
        warn!(
            status = %response.status(),
            endpoint = %endpoints.work,
            next_retry = ?current_backoff,
            "Failed to request work"
        );
        backoff(current_backoff, shutdown_requested);
        return Ok(());
    }
    let task = match response.json::<RunnerWorkResponse>() {
        Err(e) => {
            warn!(
                error = ?Report::from_error(e),
                endpoint = %endpoints.work,
                next_retry = ?current_backoff,
                "Failed to parse task"
            );
            backoff(current_backoff, shutdown_requested);
            return Ok(());
        }
        Ok(task) => task,
    };
    let Some(task) = task.task else {
        let current_backoff = &mut NO_TASK_BACKOFF.clone();
        info!(backoff = ?current_backoff, "No task received");
        backoff(current_backoff, shutdown_requested);
        return Ok(());
    };
    let task_id = task.task_id.clone();

    info!(id = task_id, "Received task");
    let (tx, rx) = std::sync::mpsc::channel();
    let source_tar = tempfile::NamedTempFile::new().context(TempFileSnafu)?;
    client
        .get(&endpoints.tar)
        .basic_auth(&args.id, Some(&args.token))
        .send()
        .context(ReqwestSnafu)?
        .copy_to(&mut source_tar.as_file())
        .context(ReqwestSnafu)?;

    let task = ExecutingTask {
        inner: task,
        pool: thread_pool,
        aborted: shutdown_requested.clone(),
        message_channel: tx,
    };
    start_update_listener(args, endpoints, rx);
    let res = execute_task(task, source_tar.into_temp_path());

    info!(id = task_id, res = ?res, "Task finished");
    client
        .post(&endpoints.done)
        .json(&res)
        .basic_auth(&args.id, Some(&args.token))
        .send()
        .context(ReqwestSnafu)?;

    Ok(())
}

fn start_update_listener(args: &Args, endpoints: &Endpoints, rx: Receiver<RunnerUpdate>) {
    let id = args.id.clone();
    let token = args.token.clone();
    let update_endpoint = endpoints.update.clone();

    // This is a daemon thread, so we do not care about the stop flag
    thread::spawn(move || {
        let client = Client::new();
        while let Ok(event) = rx.recv() {
            let res = client
                .post(&update_endpoint)
                .json(&event)
                .basic_auth(&id, Some(&token))
                .send()
                .context(ReqwestSnafu);
            if let Err(e) = res {
                warn!(
                    error = ?Report::from_error(e),
                    event = ?event,
                    endpoint = %update_endpoint,
                    "Failed to send update"
                );
            }
        }
    });
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

fn backoff(current_backoff: &mut Duration, shutdown_requested: &Arc<AtomicBool>) {
    // We need to be responsive to stop requests (CTRL+C), so we can't just sleep for
    // the full duration
    let target = Instant::now() + *current_backoff;
    while Instant::now() < target && !shutdown_requested.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));
    }
    *current_backoff *= 2;
    *current_backoff = (*current_backoff).min(Duration::from_secs(60));
}
