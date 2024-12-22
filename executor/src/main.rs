// We do not care about the extra few ns/ms it takes to copy the errors around, the callstacks
// are not deep and the other operations much more expensive.
#![allow(clippy::result_large_err)]

use crate::containers::{ContainerCreateError, TestRunError, WaitForContainerError};
use crate::executor::{execute_task, ExecutingTask};
use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::Parser;
use rayon::ThreadPoolBuilder;
use shared::CompilerTask;
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
}

const CLAP_STYLE: Styles = Styles::styled()
    .header(AnsiColor::Red.on_default().bold())
    .usage(AnsiColor::Red.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Green.on_default());

/// Executor of compiler tasks, crow of judgement.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles = CLAP_STYLE)]
struct Args {
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

        let thread_pool = ThreadPoolBuilder::new()
            .build()
            .context(ThreadPoolBuildSnafu)?;

        let mut current_backoff = Duration::from_secs(1);
        let stop_requested = Arc::new(AtomicBool::new(false));

        register_termination_handler(&stop_requested);

        while !stop_requested.load(Ordering::Relaxed) {
            let response = match reqwest::blocking::get(&args.endpoint) {
                Err(e) => {
                    warn!(
                        error = ?Report::from_error(e),
                        endpoint = ?args.endpoint,
                        next_retry = ?current_backoff,
                        "Failed to poll endpoint"
                    );
                    backoff(&mut current_backoff, &stop_requested);
                    continue;
                }
                Ok(response) => response,
            };
            let task = match response.json::<CompilerTask>() {
                Err(e) => {
                    warn!(
                        error = ?Report::from_error(e),
                        endpoint = ?args.endpoint,
                        next_retry = ?current_backoff,
                        "Failed to parse task"
                    );
                    backoff(&mut current_backoff, &stop_requested);
                    continue;
                }
                Ok(task) => task,
            };
            let run_id = task.run_id.clone();
            info!(id = run_id, "Received task");
            let task = ExecutingTask {
                inner: task,
                pool: &thread_pool,
                aborted: stop_requested.clone(),
            };
            let res = execute_task(task);
            info!(id = run_id, res = ?res, "Task finished");
        }

        info!("Goodbye!");

        Ok(())

        // Create and run the build container
        //  1. Call a standardized entrypoint
        //    1. Stream the log output directly to a higher layer (might take a while to build).
        //       maybe even force colors or emulate a PTY in the build inside the container.
        //    2. stdout/stderr are both recorded and passed through
        //    3. size limits on output?
        //  2. Capture results (stdout, stderr, exit code, time)
        //  3. Commit rootfs somehow (previously created overlayfs? Or copy the original rootfs and use
        //     the resulting, modified FS as new rootfs?)

        // Create a test container
        //   1. Create a working dir
        //   2. Create the overlayfs mount (or let runc do it)
        //   3. Render the runc config
        // Run the test container
        //   1. Build the test case program and execute it (standardized entrypoint)
        //   2. Wait for death
        //     1. Stream the log output directly to a higher layer? Or buffer with size limit and return
        //        at the end?
        //     2. Capture results (stdout, stderr, exit code, time)
        //     3. Kill container after timeout
        //   3. On container death
        //     1. Unmount all mounts (none if runc did it)
        //     2. Delete the workdir (mounts must be dead at this point)
        //   4. Return result
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
