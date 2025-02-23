use crate::task_executor::{ExecutingTask, execute_task};
use crate::{
    AnyError, Endpoints, NO_TASK_BACKOFF, ReqwestSnafu, TempFileSnafu, ThreadPoolBuildSnafu,
};
use clap::Args;
use rayon::{ThreadPool, ThreadPoolBuilder};
use reqwest::blocking::{Client, ClientBuilder};
use shared::{RunnerInfo, RunnerUpdate, RunnerWorkResponse};
use snafu::{Report, ResultExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

#[derive(Args, Debug)]
pub struct CliExecutorArgs {
    /// A unique name for this runner
    id: String,
    /// The runner token of the server
    token: String,
    /// Endpoint to poll for work updates
    endpoint: String,
}

pub fn run_executor(args: CliExecutorArgs) -> Result<(), AnyError> {
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
}

fn start_periodic_pings(endpoints: &Endpoints, args: &CliExecutorArgs) {
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
    args: &CliExecutorArgs,
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

fn start_update_listener(
    args: &CliExecutorArgs,
    endpoints: &Endpoints,
    rx: Receiver<RunnerUpdate>,
) {
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
