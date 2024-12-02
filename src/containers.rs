use derive_more::Display;
use serde::Deserialize;
use std::collections::HashMap;
use std::path;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, event, Level};

#[derive(Error, Debug)]
pub enum ContainerCreateError {
    #[error("Failed to create temporary directory: {0}")]
    TempdirNotCreated(std::io::Error),
    #[error("Failed to create directory `{0}`: {1}")]
    DirNotCreated(PathBuf, std::io::Error),
    #[error("Image `{0}` not found in registry")]
    ImageUnknown(DockerImage),
    #[error("Failed to convert path `{0}` to string")]
    PathNotRenderable(PathBuf),
}

#[derive(Error, Debug)]
pub enum ContainerCleanupError {
    #[error("Container `{0}` not found")]
    NotFound(ContainerId),
    #[error("Could not execute kill command `{0}`: {1}")]
    KillInvokeFailed(ContainerId, std::io::Error),
    #[error("Could not kill container `{0}`: {1:?}")]
    KillFailed(ContainerId, RuncLogMessage),
    #[error("Could not parse container kill output `{0}`: {1}. Raw string {2}")]
    KillOutputUnparsable(ContainerId, serde_json::Error, String),
    #[error("Could not delete directory `{0}`: {1}")]
    DirNotDeleted(PathBuf, std::io::Error),
}

#[derive(Error, Debug)]
pub enum ContainerStartError {
    #[error("Container `{0}` not found")]
    NotFound(ContainerId),
    #[error("Executing runc failed: {0}")]
    RuncFailed(std::io::Error),
}

/// A docker image
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct DockerImage {
    pub name: String,
    pub tag: String,
}

impl Display for DockerImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.tag)
    }
}

/// A registry mapping docker images to their extracted rootfs
#[derive(Debug, Clone)]
pub struct ImageRegistry {
    root_dir: PathBuf,
}

impl ImageRegistry {
    pub fn new(root_dir: &Path) -> Self {
        let root_dir = path::absolute(root_dir).expect("Failed to get absolute path");
        Self { root_dir }
    }

    pub fn get_image_rootfs(&self, image: &DockerImage) -> Option<PathBuf> {
        let path = self.root_dir.join(format!(
            "{}-{}",
            Self::clean_name(&image.name),
            Self::clean_name(&image.tag)
        ));
        event!(
            Level::DEBUG,
            path = path.to_string_lossy().to_string(),
            "Checking for image rootfs"
        );

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    fn clean_name(name: &str) -> String {
        name.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
    }
}

#[derive(Debug, Clone)]
pub struct RuncTemplate {
    template: String,
}

impl RuncTemplate {
    pub fn new(template: String) -> Self {
        Self { template }
    }

    pub fn render(
        &self,
        args: &[String],
        rootfs: &Path,
        upper_dir: &Path,
        work_dir: &Path,
    ) -> Result<String, ContainerCreateError> {
        let path_to_string: for<'a> fn(&'a Path) -> Result<&'a str, ContainerCreateError> =
            |p: &Path| {
                p.to_str()
                    .ok_or_else(|| ContainerCreateError::PathNotRenderable(p.to_path_buf()))
            };

        Ok(self
            .template
            .replace("{rootfs}", path_to_string(rootfs)?)
            .replace("{lower_dir}", path_to_string(rootfs)?)
            .replace("{upper_dir}", path_to_string(upper_dir)?)
            .replace("{work_dir}", path_to_string(work_dir)?)
            // strings serialize should not fail
            .replace("{args}", &serde_json::to_string(args).unwrap()))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuncLogMessage {
    pub level: String,
    pub msg: String,
    pub time: String,
}

#[derive(Debug)]
pub struct ContainerRegistry {
    containers: HashMap<ContainerId, Container>,
    image_registry: ImageRegistry,
    runc_template: RuncTemplate,
}

impl Drop for ContainerRegistry {
    fn drop(&mut self) {
        for container in self.containers.values() {
            error!(container=?container, "Container was not cleaned up properly");
        }
    }
}

impl ContainerRegistry {
    pub fn new(image_registry: ImageRegistry, runc_template: RuncTemplate) -> Self {
        Self {
            containers: HashMap::new(),
            image_registry,
            runc_template,
        }
    }

    pub async fn create_container(
        &mut self,
        image: DockerImage,
    ) -> Result<MustCleanupContainerId, ContainerCreateError> {
        let id = ContainerId(uuid::Uuid::new_v4().to_string());

        let workdir = tempfile::tempdir().map_err(ContainerCreateError::TempdirNotCreated)?;

        let dir_overlay_upper = workdir.path().join("overlay-upper");
        let dir_overlay_work = workdir.path().join("overlay-work");
        let file_config = workdir.path().join("config.json");

        tokio::fs::create_dir_all(&dir_overlay_work)
            .await
            .map_err(|e| ContainerCreateError::DirNotCreated(dir_overlay_work.to_path_buf(), e))?;
        tokio::fs::create_dir_all(&dir_overlay_upper)
            .await
            .map_err(|e| ContainerCreateError::DirNotCreated(dir_overlay_upper.to_path_buf(), e))?;

        let image_rootfs_path = self
            .image_registry
            .get_image_rootfs(&image)
            .ok_or(ContainerCreateError::ImageUnknown(image.clone()))?;
        let runc_config = self.runc_template.render(
            &["/bin/sh".to_string()],
            &image_rootfs_path,
            &dir_overlay_upper,
            &dir_overlay_work,
        )?;
        tokio::fs::write(&file_config, runc_config)
            .await
            .map_err(|e| ContainerCreateError::DirNotCreated(file_config, e))?;

        let container = Container {
            id,
            image,
            // stop automatic deletion when workdir goes out of scope
            workdir: workdir.into_path(),
            image_fs: image_rootfs_path.to_path_buf(),
        };

        let container_id = container.id.clone();
        self.containers.insert(container_id.clone(), container);

        Ok(MustCleanupContainerId(container_id))
    }

    pub async fn run_container(
        &mut self,
        id: MustCleanupContainerId,
    ) -> Result<MustCleanupContainerId, ContainerStartError> {
        let container = self
            .containers
            .get(&id.0)
            .ok_or(ContainerStartError::NotFound(id.0.clone()))?;

        let mut child = tokio::process::Command::new("runc")
            .arg("run")
            .arg(id.to_string())
            .current_dir(&container.workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(ContainerStartError::RuncFailed)?;

        let stdin = child.stdin.take();
        tokio::spawn(async {
            if let Some(mut stdin) = stdin {
                stdin
                    .write_all("echo hey".as_bytes())
                    .await
                    .expect("Failed to write to stdin");
                stdin.shutdown().await.expect("Failed to shutdown stdin");
            }
        });

        let res = child
            .wait_with_output()
            .await
            .map_err(ContainerStartError::RuncFailed)?;

        println!("Container exited with status: {}", res.status);
        println!(
            "Container stdout: {}",
            String::from_utf8_lossy(&res.stdout).trim()
        );
        println!(
            "Container stderr: {}",
            String::from_utf8_lossy(&res.stderr).trim()
        );

        Ok(id)
    }

    pub async fn kill_container(
        &mut self,
        container_id: MustCleanupContainerId,
    ) -> Result<(), ContainerCleanupError> {
        debug!(container_id = %container_id, "Killing container");

        let container = self
            .containers
            .remove(&container_id.0)
            .ok_or(ContainerCleanupError::NotFound(container_id.0.clone()))?;

        let res = tokio::process::Command::new("runc")
            .arg("--log-format=json")
            .arg("kill")
            .arg(container_id.to_string())
            // We directly SIGKILL, the container does not have any persistent state anyways
            .arg("KILL")
            .current_dir(&container.workdir)
            .output()
            .await
            .map_err(|e| ContainerCleanupError::KillInvokeFailed(container_id.0.clone(), e))?;

        if res.status.success() {
            debug!(container_id = %container_id, "Container killed, cleaning up");
            return Self::cleanup_container(&container.workdir).await;
        }

        debug!(
            stderr = %String::from_utf8_lossy(&res.stderr),
            status=%res.status,
            "Container kill failed"
        );

        match serde_json::from_slice::<RuncLogMessage>(&res.stderr) {
            Ok(msg) => {
                if msg.msg == "container does not exist" {
                    debug!(container_id = %container_id, "Container does not exist, cleaning up");
                    Self::cleanup_container(&container.workdir).await
                } else {
                    let _ = Self::cleanup_container(&container.workdir).await;
                    Err(ContainerCleanupError::KillFailed(
                        container_id.0.clone(),
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
                let _ = Self::cleanup_container(&container.workdir).await;

                Err(ContainerCleanupError::KillOutputUnparsable(
                    container_id.0.clone(),
                    e,
                    String::from_utf8_lossy(&res.stdout).to_string(),
                ))
            }
        }
    }

    async fn cleanup_container(workdir: &Path) -> Result<(), ContainerCleanupError> {
        debug!(workdir = %workdir.display(), "Cleaning up container workdir");

        let output = tokio::process::Command::new("rm")
            .arg("-rf")
            .arg(workdir)
            .output()
            .await
            .map_err(|e| ContainerCleanupError::DirNotDeleted(workdir.to_path_buf(), e))?;

        if !output.status.success() {
            debug!(
                workdir = %workdir.display(),
                stderr = %String::from_utf8_lossy(&output.stderr),
                "Failed to delete workdir"
            );
            return Err(ContainerCleanupError::DirNotDeleted(
                workdir.to_path_buf(),
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("rm failed: {:?}", output.stderr),
                ),
            ));
        }

        Ok(())
    }
}

/// Represents a container id that needs to be disposed of by the user to release resources.
#[derive(Debug, Display, PartialEq, Eq, Hash)]
#[must_use]
pub struct MustCleanupContainerId(ContainerId);

/// The id of a created container
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub struct ContainerId(String);

/// A created container. Might not be running, but needs to be cleaned up.
#[derive(Debug)]
pub struct Container {
    id: ContainerId,
    image: DockerImage,
    workdir: PathBuf,
    image_fs: PathBuf,
}
