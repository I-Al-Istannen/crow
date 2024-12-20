use derive_more::{Display, From};
use serde::Deserialize;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::time;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RunConfigError {
    #[error("Could not serialize arguments to json: {0}")]
    ArgsNotJson(serde_json::Error),
    #[error("Could not write file {0} due to {1}")]
    FileWrite(PathBuf, io::Error),
}

#[derive(Error, Debug)]
pub enum ContainerCreateError {
    #[error("Could not copy image rootfs: {0}")]
    ImageCopy(io::Error),
    #[error("Could not apply container config: {0}")]
    ConfigApply(RunConfigError),
    #[error("Could not create temporary directory: {0}")]
    TempDirCreation(io::Error),
}

#[derive(Error, Debug)]
pub enum ContainerDestroyError {
    #[error("Could not execute kill command `{0}`: {1}")]
    KillInvocation(ContainerId, io::Error),
    #[error("Could not kill container `{0}`: {1:?}")]
    UnknownKillFailure(ContainerId, RuncLogMessage),
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

    pub async fn copy_image_rootfs(&self, image: &ImageId, target: &Path) -> io::Result<()> {
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
    ) -> Result<(), RunConfigError> {
        let path_config = workdir.join("config.json");

        match self {
            ContainerConfig::WritableRootfs => {
                let config = include_str!("../resources/runc-read-write.json")
                    .replace("{rootfs}", &rootfs.display().to_string())
                    .replace(
                        "{args}",
                        &serde_json::to_string(args).map_err(RunConfigError::ArgsNotJson)?,
                    );

                tokio::fs::write(&path_config, config)
                    .await
                    .map_err(|e| RunConfigError::FileWrite(path_config.to_path_buf(), e))?;
            }
            ContainerConfig::OverlayRootfs => {
                let path_upper = workdir.join("overlay-upper");
                let path_work = workdir.join("overlay-work");

                tokio::fs::create_dir(&path_upper)
                    .await
                    .map_err(|e| RunConfigError::FileWrite(path_upper.to_path_buf(), e))?;
                tokio::fs::create_dir(&path_work)
                    .await
                    .map_err(|e| RunConfigError::FileWrite(path_work.to_path_buf(), e))?;

                let config = include_str!("../resources/runc-overlay.json")
                    .replace("{rootfs}", &rootfs.display().to_string())
                    .replace(
                        "{args}",
                        &serde_json::to_string(args).map_err(RunConfigError::ArgsNotJson)?,
                    )
                    .replace("{lower_dir}", &rootfs.display().to_string())
                    .replace("{upper_dir}", &path_upper.display().to_string())
                    .replace("{work_dir}", &path_work.display().to_string());

                tokio::fs::write(&path_config, config)
                    .await
                    .map_err(|e| RunConfigError::FileWrite(path_config.to_path_buf(), e))?;
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
    rootfs: PathBuf,
    container_id: ContainerId,
}

impl CreatedContainer {
    pub fn container_id(&self) -> &ContainerId {
        &self.container_id
    }

    pub fn rootfs(&self) -> &Path {
        &self.rootfs
    }
}

pub async fn create_build_container(
    registry: &ImageRegistry,
    image: &ImageId,
    args: &[&str],
) -> Result<CreatedContainer, ContainerCreateError> {
    let workdir = TempDir::new().map_err(ContainerCreateError::TempDirCreation)?;
    let path_rootfs = workdir.path().join("rootfs");

    // We modify the rootfs during the build process (as these changes are replicated into each
    // container), so we need to copy it.
    registry
        .copy_image_rootfs(image, &path_rootfs)
        .await
        .map_err(ContainerCreateError::ImageCopy)?;

    ContainerConfig::WritableRootfs
        .apply_to_workdir(&path_rootfs, workdir.path(), args)
        .await
        .map_err(ContainerCreateError::ConfigApply)?;

    Ok(CreatedContainer {
        workdir,
        rootfs: path_rootfs,
        container_id: ContainerId(Uuid::new_v4().to_string()),
    })
}

pub async fn create_test_container(
    build_container: &CreatedContainer,
    args: &[&str],
) -> Result<CreatedContainer, ContainerCreateError> {
    let workdir = TempDir::new().map_err(ContainerCreateError::TempDirCreation)?;
    let rootfs = build_container.rootfs();

    ContainerConfig::OverlayRootfs
        .apply_to_workdir(rootfs, workdir.path(), args)
        .await
        .map_err(ContainerCreateError::ConfigApply)?;

    Ok(CreatedContainer {
        workdir,
        rootfs: rootfs.to_path_buf(),
        container_id: ContainerId(Uuid::new_v4().to_string()),
    })
}

#[derive(Debug)]
#[must_use]
pub struct StartedContainer {
    container: CreatedContainer,
    process: tokio::process::Child,
    pub stdout: Option<tokio::process::ChildStdout>,
    pub stderr: Option<tokio::process::ChildStderr>,
    cleaned_up: bool,
}

impl StartedContainer {
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

    pub async fn await_death(&mut self) -> io::Result<std::process::ExitStatus> {
        self.process.wait().await
    }

    pub fn container(&self) -> &CreatedContainer {
        &self.container
    }
}

impl Drop for StartedContainer {
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

pub async fn run_container(container: CreatedContainer) -> Result<StartedContainer, io::Error> {
    let mut process = Command::new("runc")
        .arg("run")
        .arg(container.container_id.to_string())
        .current_dir(container.workdir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()?;

    let stdout = process.stdout.take();
    let stderr = process.stderr.take();

    Ok(StartedContainer {
        container,
        process,
        stdout,
        stderr,
        cleaned_up: false,
    })
}

async fn kill_container(container_id: &ContainerId) -> Result<(), ContainerDestroyError> {
    debug!(container_id = %container_id, "Killing container");

    let res = Command::new("runc")
        .arg("--log-format=json")
        .arg("kill")
        .arg(container_id.to_string())
        // We directly SIGKILL, the container does not have any persistent state anyway
        .arg("KILL")
        .output()
        .await
        .map_err(|e| ContainerDestroyError::KillInvocation(container_id.clone(), e))?;

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
                Err(ContainerDestroyError::UnknownKillFailure(
                    container_id.clone(),
                    msg,
                ))
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

/// Reads an async read to a single String.
pub async fn read_to_string(reader: &mut (impl AsyncRead + Unpin)) -> io::Result<String> {
    let mut output = String::new();
    reader.read_to_string(&mut output).await?;
    Ok(output)
}
