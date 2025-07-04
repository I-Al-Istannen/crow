use crate::containers::LimitsConfig;
use crate::docker::Docker;
use crate::{AnyError, Endpoints, ReqwestSnafu};
use clap::Args;
use reqwest::blocking::{Client, ClientBuilder};
use shared::{RunnerInfo, RunnerUpdate};
use snafu::{Report, ResultExt, location};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

mod test_compiler;
mod test_tasting;

pub trait Iteration {
    fn iteration(
        &mut self,
        args: &CliExecutorArgs,
        endpoints: &Endpoints,
        current_backoff: &mut Duration,
        shutdown_requested: &Arc<AtomicBool>,
        client: &Client,
        runner_info: &RunnerInfo,
    ) -> Result<(), AnyError>;
}

#[derive(Args, Debug)]
pub struct CliExecutorArgs {
    /// A unique name for this runner
    pub id: String,
    /// The runner token of the server
    pub token: String,
    /// Endpoint to poll for work updates
    pub endpoint: String,
    /// If set, this executor will request the reference compiler and then only accept tests
    /// to validate against it
    #[clap(long, default_value = "false")]
    pub test_taster: bool,
    /// The directory to cache docker images in. If not set, no cache will be used.
    #[clap(long)]
    pub image_cache_dir: Option<PathBuf>,
    /// The amount of tests to execute in parallel. If 0, the number of processors is used.
    #[clap(long, short = 'j', default_value = "0")]
    pub parallelism: usize,
    /// The number of CPUs to allow the build container. 0 means no limit.
    #[clap(long, default_value = "0")]
    pub build_max_cpu: u32,
    /// The max ram size in bytes to allow the build container. 0 means no limit.
    #[clap(long, default_value_t = 5 * 1024 * 1024 * 1024)]
    pub build_max_memory: usize,
    /// The number of CPUs to allow the test containers. 0 means no limit.
    #[clap(long, default_value = "1")]
    pub test_max_cpu: u32,
    /// The max ram size in bytes to allow the test containers. 0 means no limit.
    #[clap(long, default_value_t = 2 * 1024 * 1024 * 1024)]
    pub test_max_memory: usize,
}

pub fn run_executor(args: CliExecutorArgs) -> Result<(), AnyError> {
    let endpoints = Endpoints::new(&args.endpoint);

    let mut current_backoff = Duration::from_secs(1);
    let shutdown_requested = Arc::new(AtomicBool::new(false));

    register_termination_handler(&shutdown_requested);
    start_periodic_pings(&endpoints, &args);

    let client = ClientBuilder::new().build().context(ReqwestSnafu)?;
    let docker = match Docker::new(args.image_cache_dir.clone()) {
        Ok(docker) => docker,
        Err(e) => {
            return Err(AnyError::Docker {
                source: e,
                location: location!(),
            });
        }
    };

    let mut iteration: Box<dyn Iteration> = if args.test_taster {
        Box::new(test_tasting::TestTastingState::new(
            docker,
            LimitsConfig::new(args.test_max_cpu, args.test_max_memory),
        ))
    } else {
        Box::new(test_compiler::TestCompilerState::new(
            docker,
            args.parallelism,
            LimitsConfig::new(args.build_max_cpu, args.build_max_memory),
            LimitsConfig::new(args.test_max_cpu, args.test_max_memory),
        )?)
    };

    while !shutdown_requested.load(Ordering::Relaxed) {
        let runner_info = RunnerInfo {
            id: args.id.clone().into(),
            info: "hey".to_string(),
            current_task: None,
            test_taster: args.test_taster,
        };
        let res = iteration.iteration(
            &args,
            &endpoints,
            &mut current_backoff,
            &shutdown_requested,
            &client,
            &runner_info,
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

pub fn backoff(current_backoff: &mut Duration, shutdown_requested: &Arc<AtomicBool>) {
    // We need to be responsive to stop requests (CTRL+C), so we can't just sleep for
    // the full duration
    let target = Instant::now() + *current_backoff;
    while Instant::now() < target && !shutdown_requested.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));
    }
    *current_backoff *= 2;
    *current_backoff = (*current_backoff).min(Duration::from_secs(60));
}
