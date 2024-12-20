use derive_more::{Display, From};
use serde::Deserialize;
use snafu::{IntoError, NoneError, ResultExt, Snafu};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::time;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Snafu, Debug)]
pub enum RunConfigError {
    #[snafu(display("Could not serialize arguments to json"))]
    ArgsNotJson { source: serde_json::Error },
    #[snafu(display("Could not write file {path:?}"))]
    FileWrite { source: io::Error, path: PathBuf },
}

#[derive(Snafu, Debug)]
pub enum ContainerCreateError {
    #[snafu(display("Could not copy image rootfs"))]
    ImageCopy { source: io::Error },
    #[snafu(display("Could not apply container config"))]
    ConfigApply { source: RunConfigError },
    #[snafu(display("Could not create temporary directory"))]
    TempDirCreation { source: io::Error },
}

#[derive(Snafu, Debug)]
pub enum ContainerDestroyError {
    #[snafu(display("Could not execute kill command `{container_id}`"))]
    KillInvocation {
        container_id: ContainerId,
        source: io::Error,
    },
    #[snafu(display("Could not kill container `{container_id}`: {message:?}"))]
    UnknownKillFailure {
        container_id: ContainerId,
        message: RuncLogMessage,
    },
    #[snafu(display(
        "Could not parse container kill output for `{container_id}`: `{raw_output}`"
    ))]
    KillOutputUnparsable {
        container_id: ContainerId,
        source: serde_json::Error,
        raw_output: String,
    },
    #[snafu(display("Could not delete directory `{path:?}`"))]
    DirNotDeleted { source: io::Error, path: PathBuf },
    #[snafu(display("Multiple errors occurred: {errors:?}"))]
    Multiple { errors: Vec<ContainerDestroyError> },
    #[snafu(display("Container `{container_id}` was leaked and left alive"))]
    LeakedProcess { container_id: ContainerId },
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuncLogMessage {
    #[allow(unused)]
    pub level: String,
    pub msg: String,
    #[allow(unused)]
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
                        &serde_json::to_string(args).context(ArgsNotJsonSnafu)?,
                    );

                tokio::fs::write(&path_config, config)
                    .await
                    .context(FileWriteSnafu {
                        path: path_config.to_path_buf(),
                    })?;
            }
            ContainerConfig::OverlayRootfs => {
                let path_upper = workdir.join("overlay-upper");
                let path_work = workdir.join("overlay-work");

                tokio::fs::create_dir(&path_upper)
                    .await
                    .context(FileWriteSnafu {
                        path: path_upper.to_path_buf(),
                    })?;
                tokio::fs::create_dir(&path_work)
                    .await
                    .context(FileWriteSnafu {
                        path: path_work.to_path_buf(),
                    })?;

                let config = include_str!("../resources/runc-overlay.json")
                    .replace("{rootfs}", &rootfs.display().to_string())
                    .replace(
                        "{args}",
                        &serde_json::to_string(args).context(ArgsNotJsonSnafu)?,
                    )
                    .replace("{lower_dir}", &rootfs.display().to_string())
                    .replace("{upper_dir}", &path_upper.display().to_string())
                    .replace("{work_dir}", &path_work.display().to_string());

                tokio::fs::write(&path_config, config)
                    .await
                    .context(FileWriteSnafu {
                        path: path_config.to_path_buf(),
                    })?;
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
    let workdir = TempDir::new().context(TempDirCreationSnafu)?;
    let path_rootfs = workdir.path().join("rootfs");

    // We modify the rootfs during the build process (as these changes are replicated into each
    // container), so we need to copy it.
    registry
        .copy_image_rootfs(image, &path_rootfs)
        .await
        .context(ImageCopySnafu)?;

    ContainerConfig::WritableRootfs
        .apply_to_workdir(&path_rootfs, workdir.path(), args)
        .await
        .context(ConfigApplySnafu)?;

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
    let workdir = TempDir::new().context(TempDirCreationSnafu)?;
    let rootfs = build_container.rootfs();

    ContainerConfig::OverlayRootfs
        .apply_to_workdir(rootfs, workdir.path(), args)
        .await
        .context(ConfigApplySnafu)?;

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
            errors.push(
                LeakedProcessSnafu {
                    container_id: self.container.container_id.clone(),
                }
                .into_error(NoneError),
            );
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
            Err(MultipleSnafu { errors }.into_error(NoneError))
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
        .context(KillInvocationSnafu {
            container_id: container_id.clone(),
        })?;

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
                Err(UnknownKillFailureSnafu {
                    container_id: container_id.clone(),
                    message: msg,
                }
                .into_error(NoneError))
            }
        }
        Err(e) => {
            debug!(
                container_id=%container_id,
                stderr = %String::from_utf8_lossy(&res.stderr),
                error=%e, "Could not parse kill output"
            );

            Err(KillOutputUnparsableSnafu {
                container_id: container_id.clone(),
                raw_output: String::from_utf8_lossy(&res.stderr).to_string(),
            }
            .into_error(e))
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
        .context(DirNotDeletedSnafu {
            path: workdir.to_path_buf(),
        })?;

    if !output.status.success() {
        debug!(
            workdir = %workdir.display(),
            container = ?container_id,
            stderr = %String::from_utf8_lossy(&output.stderr),
            "Failed to delete workdir"
        );
        return Err(DirNotDeletedSnafu {
            path: workdir.to_path_buf(),
        }
        .into_error(io::Error::new(
            io::ErrorKind::Other,
            format!("rm failed: {:?}", output.stderr),
        )));
    }

    Ok(())
}

/// Reads an async read to a single String.
pub async fn read_to_string(reader: &mut (impl AsyncRead + Unpin)) -> io::Result<String> {
    let mut output = String::new();
    reader.read_to_string(&mut output).await?;
    Ok(output)
}
