// We do not care about the extra few ns/ms it takes to copy the errors around, the callstacks
// are not deep and the other operations much more expensive.
#![allow(clippy::result_large_err)]
#![allow(unsafe_code)]

use crate::containers::{ContainerCreateError, TestRunError, WaitForContainerError};
use crate::mode_executor::{run_executor, CliExecutorArgs};
use crate::mode_shim::{run_shim, CliShimArgs};
use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{Parser, Subcommand};
use snafu::{Location, Report, Snafu};
use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod containers;
mod docker;
mod mode_executor;
mod mode_shim;
mod task_executor;

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
    #[snafu(display("Shim error `{msg}` at {location}"))]
    Shim {
        msg: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Shim error `{msg}` at {location}"))]
    ShimWithSource {
        msg: String,
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error communicating with docker at {location}"))]
    Docker {
        source: docker::DockerError,
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

// noinspection DuplicatedCode
/// Executor of compiler tasks, crow of judgement.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles = CLAP_STYLE)]
#[command(propagate_version = true)]
struct CliArgs {
    #[clap(subcommand)]
    subcommand: CliCommand,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Runs the executor fetching tasks from the server and executing them.
    Executor(CliExecutorArgs),
    /// Runs the executor in-container shim translating killed-by-signal
    Shim(CliShimArgs),
}

struct Endpoints {
    done: String,
    done_taste_test: String,
    ping: String,
    register: String,
    tar: String,
    update: String,
    work: String,
    work_taste_test: String,
}

impl Endpoints {
    pub fn new(base: &str) -> Self {
        Self {
            done: format!("{base}/executor/done"),
            done_taste_test: format!("{base}/executor/done-taste-test"),
            ping: format!("{base}/executor/ping"),
            register: format!("{base}/executor/register"),
            tar: format!("{base}/executor/request-tar"),
            update: format!("{base}/executor/update"),
            work: format!("{base}/executor/request-work"),
            work_taste_test: format!("{base}/executor/request-work-taste-test"),
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

        let args = CliArgs::parse();

        match args.subcommand {
            CliCommand::Executor(args) => run_executor(args),
            CliCommand::Shim(args) => run_shim(args),
        }
    })
}
