use crate::containers::{ContainerCreateError, TestRunError, WaitForContainerError};
use crate::executor::execute_task;
use crate::types::{CompilerTask, CompilerTest};
use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::Parser;
use rayon::ThreadPoolBuilder;
use snafu::{Location, Report, Snafu};
use std::time::Duration;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod containers;
mod docker;
mod executor;
mod types;

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

        let test = CompilerTest {
            test_id: "test".to_string(),
            timeout: Duration::from_secs(3),
            run_command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "echo 'hey' >> /tmp/foo.txt && cat /tmp/foo.txt".to_string(),
            ],
        };
        let tests = vec![test.clone(), test.clone(), test.clone()];

        let res = execute_task(
            CompilerTask {
                run_id: "test".to_string(),
                image: "alpine:latest".to_string(),
                build_command: vec![
                    "/bin/sh".to_string(),
                    "-c".to_string(),
                    "echo Hello, world!".to_string(),
                ],
                build_timeout: Duration::from_secs(200000),
                tests,
            },
            &ThreadPoolBuilder::new().build().unwrap(),
        );

        info!("Result: {:#?}", res);

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
