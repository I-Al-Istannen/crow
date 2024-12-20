use crate::containers::{
    create_build_container, read_to_string, run_container, ContainerCreateError, ImageId,
    ImageRegistry, StartedContainer,
};
use crate::docker::DockerClient;
use std::process::ExitStatus;
use thiserror::Error;
use tokio::join;
use tracing::warn;

#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("Error creating container {0}")]
    ContainerCreation(ContainerCreateError),
    #[error("Error running container {0}")]
    ContainerRun(std::io::Error),
    #[error(
        "I/O error while building the compiler {error}. Stdout: {stdout:?}. Stderr: {stderr:?}"
    )]
    CompilerBuildIo {
        stderr: std::io::Result<String>,
        stdout: std::io::Result<String>,
        error: std::io::Error,
    },
    #[error("Compiler build was unsuccessful. Stdout: {stdout:?}. Stderr: {stderr:?}")]
    CompilerBuildUnsuccessful {
        stderr: std::io::Result<String>,
        stdout: std::io::Result<String>,
        exit_status: ExitStatus,
    },
}

#[derive(Debug)]
pub struct CompilerBuildResult {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub container: StartedContainer,
}

pub struct Executor {
    docker: DockerClient,
    image_registry: ImageRegistry,
}

impl Executor {
    pub fn new(docker: DockerClient, image_registry: ImageRegistry) -> Self {
        Self {
            docker,
            image_registry,
        }
    }

    pub async fn build_compiler(
        &self,
        image: ImageId,
        command: &[&str],
    ) -> Result<CompilerBuildResult, ExecutorError> {
        let container = create_build_container(&self.image_registry, &image, command)
            .await
            .map_err(ExecutorError::ContainerCreation)?;
        let mut container = run_container(container)
            .await
            .map_err(ExecutorError::ContainerRun)?;

        let stderr = &mut container.stderr.take().unwrap();
        let stdout = &mut container.stdout.take().unwrap();
        let (stdout, stderr) = join!(read_to_string(stdout), read_to_string(stderr));

        let res = match container.await_death().await {
            Ok(res) => res,
            Err(err) => {
                return Err(ExecutorError::CompilerBuildIo {
                    stderr,
                    stdout,
                    error: err,
                });
            }
        };

        if !res.success() {
            return Err(ExecutorError::CompilerBuildUnsuccessful {
                stderr,
                stdout,
                exit_status: res,
            });
        }

        if let Err(e) = &stdout {
            warn!(error = ?e, image = ?image, command = ?command, "Error reading stdout");
        }
        if let Err(e) = &stderr {
            warn!(error = ?e, image = ?image, command = ?command, "Error reading stderr");
        }

        Ok(CompilerBuildResult {
            stdout: stdout.ok(),
            stderr: stderr.ok(),
            container,
        })
    }
}
