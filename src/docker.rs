use bollard::container::Config;
use bollard::Docker;
use futures_util::TryStreamExt;
use snafu::{IntoError, NoneError, ResultExt, Snafu};
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tokio::task::spawn_blocking;

#[derive(Debug, Snafu)]
pub enum DockerError {
    #[snafu(display("Docker communication error: {message}"))]
    Docker {
        source: bollard::errors::Error,
        message: &'static str,
    },
    #[snafu(display("Image `{image}` not found"))]
    ImageNotFound { image: String },
    #[snafu(display("Error creating or writing to export file"))]
    TarExportIo { source: std::io::Error },
    #[snafu(display("Error creating or writing to tempfile"))]
    TempfileIo { source: std::io::Error },
    #[snafu(display("Error untarring image"))]
    Untar { source: tokio::task::JoinError },
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
            .await
            .context(DockerSnafu {
                message: "while listing images",
            })?
            .into_iter()
            .any(|it| it.repo_tags.contains(&image_name.to_string()));

        if !image_exists {
            return Err(ImageNotFoundSnafu {
                image: image_name.to_string(),
            }
            .into_error(NoneError));
        }

        let mut file = tokio::fs::File::create(target)
            .await
            .context(TarExportIoSnafu)?;

        let container = self
            .docker
            .create_container::<String, String>(
                None,
                Config {
                    image: Some(image_name.to_string()),
                    ..Default::default()
                },
            )
            .await
            .context(DockerSnafu {
                message: "while creating container",
            })?;

        let mut tarball = self.docker.export_container(&container.id);

        loop {
            let chunk = tarball.try_next().await;
            match chunk {
                Err(e) => {
                    let _ = self.docker.remove_container(&container.id, None).await;
                    return Err(DockerSnafu {
                        message: "while exporting container",
                    }
                    .into_error(e));
                }
                Ok(Some(chunk)) => {
                    if let Err(e) = file.write_all(&chunk).await {
                        let _ = self.docker.remove_container(&container.id, None).await;
                        return Err(TarExportIoSnafu.into_error(e));
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
        let dir = tempfile::tempdir().context(TempfileIoSnafu)?;
        let tarball_path = dir.path().join("image.tar");
        self.export_image_to_tar(image_name, &tarball_path).await?;

        let target_folder = target_folder.as_ref().to_path_buf();
        spawn_blocking(move || {
            tar::Archive::new(std::fs::File::open(tarball_path).context(TempfileIoSnafu)?)
                .unpack(target_folder)
                .context(TempfileIoSnafu)
        })
        .await
        .context(UntarSnafu)?
    }
}
