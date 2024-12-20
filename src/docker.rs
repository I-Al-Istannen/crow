use bollard::container::Config;
use bollard::Docker;
use futures_util::TryStreamExt;
use std::path::Path;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::task::spawn_blocking;

#[derive(Debug, Error)]
pub enum DockerError {
    #[error("Docker communication error: {0}")]
    Docker(#[from] bollard::errors::Error),
    #[error("Image not found: {0}")]
    ImageNotFound(String),
    #[error("Error creating or writing to export file: {0}")]
    TarExportIo(std::io::Error),
    #[error("Error creating or writing to tempfile: {0}")]
    TempfileIo(std::io::Error),
    #[error("Error untarring image: {0}")]
    Untar(tokio::task::JoinError),
}

#[derive(Debug)]
pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    pub async fn export_image_to_tar(
        &self,
        image_name: &str,
        target: &Path,
    ) -> Result<(), DockerError> {
        let image_exists = self
            .docker
            .list_images::<String>(None)
            .await?
            .into_iter()
            .any(|it| it.repo_tags.contains(&image_name.to_string()));

        if !image_exists {
            return Err(DockerError::ImageNotFound(image_name.to_string()));
        }

        let mut file = tokio::fs::File::create(target)
            .await
            .map_err(DockerError::TarExportIo)?;

        let container = self
            .docker
            .create_container::<String, String>(
                None,
                Config {
                    image: Some(image_name.to_string()),
                    ..Default::default()
                },
            )
            .await?;

        let mut tarball = self.docker.export_container(&container.id);

        loop {
            let chunk = tarball.try_next().await;
            match chunk {
                Err(e) => {
                    let _ = self.docker.remove_container(&container.id, None).await;
                    return Err(DockerError::Docker(e));
                }
                Ok(Some(chunk)) => {
                    if let Err(e) = file.write_all(&chunk).await {
                        let _ = self.docker.remove_container(&container.id, None).await;
                        return Err(DockerError::TarExportIo(e));
                    }
                }
                Ok(None) => break,
            }
        }

        let _ = self.docker.remove_container(&container.id, None).await;

        Ok(())
    }

    pub async fn export_image_unpacked(
        &self,
        image_name: &str,
        target_folder: impl AsRef<Path>,
    ) -> Result<(), DockerError> {
        let dir = tempfile::tempdir().map_err(DockerError::TempfileIo)?;
        let tarball_path = dir.path().join("image.tar");
        self.export_image_to_tar(image_name, &tarball_path).await?;

        let target_folder = target_folder.as_ref().to_path_buf();
        spawn_blocking(move || {
            tar::Archive::new(std::fs::File::open(tarball_path).map_err(DockerError::TempfileIo)?)
                .unpack(target_folder)
                .map_err(DockerError::TempfileIo)
        })
        .await
        .map_err(DockerError::Untar)?
    }
}
