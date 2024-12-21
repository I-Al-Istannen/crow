use crate::containers::{
    create_build_container, create_test_container, read_to_string, run_container,
    ContainerCreateError, ContainerDestroyError, CreatedContainer, ImageId, ImageRegistry,
    StartedContainer,
};
use snafu::futures::TryFutureExt;
use snafu::{ensure, ResultExt, Snafu};
use std::io;
use std::process::ExitStatus;
use tokio::join;
use tracing::warn;

#[derive(Debug, Snafu)]
pub enum ExecutorError {
    #[snafu(display("Error creating container: {source}"))]
    Creation { source: ContainerCreateError },
    #[snafu(display("Error running container {source}"))]
    Start { source: io::Error },
    #[snafu(display(
        "I/O error while building the compiler {source}. Stdout: {stdout:?}. Stderr: {stderr:?}"
    ))]
    RunIo {
        stderr: Option<String>,
        stdout: Option<String>,
        source: io::Error,
    },
    #[snafu(display("Container run was unsuccessful. Stdout: {stdout:?}. Stderr: {stderr:?}"))]
    RunUnsuccessful {
        stderr: Option<String>,
        stdout: Option<String>,
        exit_status: ExitStatus,
    },
    #[snafu(display("Error destroying container {source}"))]
    Destroy { source: ContainerDestroyError },
}

#[derive(Debug)]
pub struct ContainerBuildResult {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub container: StartedContainer,
}

#[derive(Debug, Clone)]
pub struct ContainerTestResult {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_status: ExitStatus,
}

pub struct Executor {
    image_registry: ImageRegistry,
}

impl Executor {
    pub fn new(image_registry: ImageRegistry) -> Self {
        Self { image_registry }
    }

    pub async fn build_main_container(
        &self,
        image: ImageId,
        command: &[&str],
    ) -> Result<ContainerBuildResult, ExecutorError> {
        let container = create_build_container(&self.image_registry, &image, command)
            .await
            .context(CreationSnafu)?;
        let mut container = run_container(container).await.context(StartSnafu)?;

        let (stdout, stderr, _) = Self::execute_for_output(command, &mut container).await?;

        Ok(ContainerBuildResult {
            stdout,
            stderr,
            container,
        })
    }

    async fn execute_for_output(
        command: &[&str],
        container: &mut StartedContainer,
    ) -> Result<(Option<String>, Option<String>, ExitStatus), ExecutorError> {
        let stderr = &mut container.stderr.take().unwrap();
        let stdout = &mut container.stdout.take().unwrap();
        let (stdout, stderr) = join!(read_to_string(stdout), read_to_string(stderr));

        let stdout = match stdout {
            Ok(stdout) => Some(stdout),
            Err(e) => {
                warn!(error = ?e, command = ?command, "Error reading stdout");
                None
            }
        };
        let stderr = match stderr {
            Ok(stderr) => Some(stderr),
            Err(e) => {
                warn!(error = ?e, command = ?command, "Error reading stderr");
                None
            }
        };

        let res = container
            .await_death()
            .context(RunIoSnafu {
                stderr: stderr.clone(),
                stdout: stdout.clone(),
            })
            .await?;

        ensure!(
            res.success(),
            RunUnsuccessfulSnafu {
                stderr,
                stdout,
                exit_status: res
            }
        );

        Ok((stdout, stderr, res))
    }

    pub async fn run_test(
        &self,
        build_container: &CreatedContainer,
        command: &[&str],
    ) -> Result<ContainerTestResult, ExecutorError> {
        let container = create_test_container(build_container, command)
            .await
            .context(CreationSnafu)?;
        let mut container = run_container(container).await.context(StartSnafu)?;

        let (stdout, stderr, exit_status) =
            Self::execute_for_output(command, &mut container).await?;

        if let Err(e) = container.destroy().await.context(DestroySnafu) {
            warn!(
                error = ?e,
                container = ?build_container.container_id(),
                "Error destroying container"
            );
        }

        Ok(ContainerTestResult {
            stdout,
            stderr,
            exit_status,
        })
    }
}
