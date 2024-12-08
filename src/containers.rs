use derive_more::{Display, From};
use serde::Deserialize;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use thiserror::Error;
use tokio::process::Command;
use tokio::time;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ContainerDestroyError {
    #[error("Could not execute kill command `{0}`: {1}")]
    KillInvokeFailed(ContainerId, io::Error),
    #[error("Could not kill container `{0}`: {1:?}")]
    KillFailed(ContainerId, RuncLogMessage),
    #[error("Could not parse container kill output `{0}`: {1}. Raw string {2}")]
    KillOutputUnparsable(ContainerId, serde_json::Error, String),
    #[error("Could not delete directory `{0}`: {1}")]
    DirNotDeleted(PathBuf, io::Error),
    #[error("Multiple errors occurred: {0:?}")]
    Multiple(Vec<ContainerDestroyError>),
    #[error("Container `{0}` was leaked and left alive")]
    LeakedProcess(ContainerId),
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuncLogMessage {
    pub level: String,
    pub msg: String,
    pub time: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub struct ImageId(String);

#[derive(Debug)]
pub struct ImageRegistry {
    directory: PathBuf,
}

impl ImageRegistry {
    pub fn new(directory: impl AsRef<Path>) -> Self {
        Self {
            directory: directory.as_ref().to_path_buf(),
        }
    }

    pub fn get_images(&self) -> io::Result<Vec<ImageId>> {
        Ok(std::fs::read_dir(&self.directory)?
            .collect::<io::Result<Vec<_>>>()?
            .into_iter()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .map(ImageId)
            .collect::<Vec<_>>())
    }

    pub async fn copy_image_rootfs(&self, image: &ImageId, target: &Path) -> Result<(), io::Error> {
        let image_rootfs = self.directory.join(&image.0);

        if !image_rootfs.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Image {} not found", image),
            ));
        }

        let output = Command::new("cp")
            .arg("-r")
            .arg(image_rootfs)
            .arg(target)
            .output()
            .await?;

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to copy rootfs: {:?}", output.stderr),
            ));
        }

        Ok(())
    }
}

pub enum ContainerConfig {
    WritableRootfs,
    OverlayRootfs,
}

impl ContainerConfig {
    pub async fn apply_to_workdir(
        &self,
        rootfs: &Path,
        workdir: &Path,
        args: &[&str],
    ) -> Result<(), Box<dyn Error>> {
        let path_config = workdir.join("config.json");

        match self {
            ContainerConfig::WritableRootfs => {
                let config = include_str!("../resources/runc-read-write.json")
                    .replace("{rootfs}", &rootfs.display().to_string())
                    .replace("{args}", &serde_json::to_string(args)?);

                tokio::fs::write(&path_config, config).await?;
            }
            ContainerConfig::OverlayRootfs => {
                let path_upper = workdir.join("overlay-upper");
                let path_work = workdir.join("overlay-work");

                tokio::fs::create_dir(&path_upper).await?;
                tokio::fs::create_dir(&path_work).await?;

                let config = include_str!("../resources/runc-overlay.json")
                    .replace("{rootfs}", &rootfs.display().to_string())
                    .replace("{args}", &serde_json::to_string(args)?)
                    .replace("{lower_dir}", &rootfs.display().to_string())
                    .replace("{upper_dir}", &path_upper.display().to_string())
                    .replace("{work_dir}", &path_work.display().to_string());

                tokio::fs::write(&path_config, config).await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, From, Display)]
pub struct ContainerId(String);

#[derive(Debug)]
#[must_use]
pub struct CreatedContainer {
    workdir: TempDir,
    container_id: ContainerId,
}

pub async fn create_build_container(
    registry: &ImageRegistry,
    image: &ImageId,
    args: &[&str],
) -> Result<CreatedContainer, Box<dyn Error>> {
    let workdir = TempDir::new()?;
    let path_rootfs = workdir.path().join("rootfs");

    // We modify the rootfs during the build process (as these changes are replicated into each
    // container), so we need to copy it.
    registry.copy_image_rootfs(image, &path_rootfs).await?;

    ContainerConfig::WritableRootfs
        .apply_to_workdir(&path_rootfs, workdir.path(), args)
        .await?;

    Ok(CreatedContainer {
        workdir,
        container_id: ContainerId(Uuid::new_v4().to_string()),
    })
}

#[derive(Debug)]
#[must_use]
pub struct RunningContainer {
    container: CreatedContainer,
    process: tokio::process::Child,
    cleaned_up: bool,
}

impl RunningContainer {
    pub async fn destroy(mut self) -> Result<(), ContainerDestroyError> {
        let mut errors = Vec::new();
        if let Err(e) = kill_container(&self.container.container_id).await {
            error!(
                error = ?e,
                container = ?self.container,
                process = ?self.process.id(),
                "Failed to kill container"
            );
            errors.push(e);
        }
        if let Err(e) = time::timeout(Duration::from_secs(5), self.process.wait()).await {
            error!(
                error = ?e,
                container = ?self.container,
                process = ?self.process.id(),
                "Container was still alive"
            );
            errors.push(ContainerDestroyError::LeakedProcess(
                self.container.container_id.clone(),
            ));
        }

        if let Err(e) =
            delete_container_dir(&self.container.container_id, self.container.workdir.path()).await
        {
            error!(
                error = ?e,
                container = ?self.container,
                "Failed to delete container workdir"
            );
            errors.push(e);
        }

        self.cleaned_up = true;

        if !errors.is_empty() && errors.len() > 1 {
            Err(ContainerDestroyError::Multiple(errors))
        } else if !errors.is_empty() {
            Err(errors.remove(0))
        } else {
            Ok(())
        }
    }

    pub fn process(&mut self) -> &mut tokio::process::Child {
        &mut self.process
    }
}

impl Drop for RunningContainer {
    fn drop(&mut self) {
        if !self.cleaned_up {
            error!(
                container = ?self.container,
                process = ?self.process.id(),
                "Container was not cleaned up before dropping"
            );
        }
    }
}

pub async fn run_container(
    container: CreatedContainer,
) -> Result<RunningContainer, Box<dyn Error>> {
    let process = Command::new("runc")
        .arg("run")
        .arg(container.container_id.to_string())
        .current_dir(container.workdir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()?;

    Ok(RunningContainer {
        container,
        process,
        cleaned_up: false,
    })
}

async fn kill_container(container_id: &ContainerId) -> Result<(), ContainerDestroyError> {
    debug!(container_id = %container_id, "Killing container");

    let res = Command::new("runc")
        .arg("--log-format=json")
        .arg("kill")
        .arg(container_id.to_string())
        // We directly SIGKILL, the container does not have any persistent state anyways
        .arg("KILL")
        .output()
        .await
        .map_err(|e| ContainerDestroyError::KillInvokeFailed(container_id.clone(), e))?;

    if res.status.success() {
        debug!(container_id = %container_id, "Container killed");
        return Ok(());
    }

    debug!(
        stderr = %String::from_utf8_lossy(&res.stderr),
        status=%res.status,
        "Container kill failed"
    );

    match serde_json::from_slice::<RuncLogMessage>(&res.stderr) {
        Ok(msg) => {
            if msg.msg == "container does not exist" {
                debug!(container_id = %container_id, "Container does not exist");
                Ok(())
            } else {
                Err(ContainerDestroyError::KillFailed(container_id.clone(), msg))
            }
        }
        Err(e) => {
            debug!(
                container_id=%container_id,
                stderr = %String::from_utf8_lossy(&res.stderr),
                error=%e, "Could not parse kill output"
            );

            Err(ContainerDestroyError::KillOutputUnparsable(
                container_id.clone(),
                e,
                String::from_utf8_lossy(&res.stdout).to_string(),
            ))
        }
    }
}

async fn delete_container_dir(
    container_id: &ContainerId,
    workdir: &Path,
) -> Result<(), ContainerDestroyError> {
    debug!(
        workdir = %workdir.display(),
        container = ?container_id,
        "Cleaning up container workdir"
    );

    let output = Command::new("rm")
        .arg("-rf")
        .arg(workdir)
        .output()
        .await
        .map_err(|e| ContainerDestroyError::DirNotDeleted(workdir.to_path_buf(), e))?;

    if !output.status.success() {
        debug!(
            workdir = %workdir.display(),
            container = ?container_id,
            stderr = %String::from_utf8_lossy(&output.stderr),
            "Failed to delete workdir"
        );
        return Err(ContainerDestroyError::DirNotDeleted(
            workdir.to_path_buf(),
            io::Error::new(
                io::ErrorKind::Other,
                format!("rm failed: {:?}", output.stderr),
            ),
        ));
    }

    Ok(())
}
