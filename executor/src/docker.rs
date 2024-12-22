use derive_more::Display;
use snafu::{ensure, IntoError, Location, NoneError, ResultExt, Snafu};
use std::fs::File;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Snafu)]
pub enum DockerError {
    #[snafu(display("Docker command failed {message} at {location}"))]
    DockerCall {
        source: std::io::Error,
        message: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Could not understand docker response {message}: `{response}` at {location}"
    ))]
    UnknownDockerResponse {
        message: &'static str,
        response: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not find docker image `{image}` at {location}"))]
    ImageNotFound {
        image: ImageId,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not export tar file at {location}"))]
    TarExportIo {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not create/write tempfile at {location}"))]
    TempfileIo {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub struct ImageId(pub String);

pub fn export_image_to_tar(image: &ImageId, target: &Path) -> Result<(), DockerError> {
    let output = Command::new("docker")
        .arg("image")
        .arg("inspect")
        .arg(image.to_string())
        .output()
        .context(DockerCallSnafu {
            message: "verifying image exists",
        })?;
    if !output.status.success() {
        if String::from_utf8_lossy(&output.stdout).trim() == "[]" {
            return Err(ImageNotFoundSnafu {
                image: image.clone(),
            }
            .into_error(NoneError));
        }
        return Err(UnknownDockerResponseSnafu {
            message: "while inspecting image",
            response: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        }
        .into_error(NoneError));
    }

    // Touch the file to ensure we can actually write to it
    File::create(target).context(TarExportIoSnafu)?;

    let res = Command::new("docker")
        .arg("create")
        .arg("-q")
        .arg(image.to_string())
        .output()
        .context(DockerCallSnafu {
            message: "creating container",
        })?;

    ensure!(
        res.status.success(),
        UnknownDockerResponseSnafu {
            message: "while creating container",
            response: String::from_utf8_lossy(&res.stderr).to_string(),
        }
    );

    let container_id = String::from_utf8_lossy(&res.stdout).trim().to_string();

    let res = Command::new("docker")
        .arg("export")
        .arg(&container_id)
        .arg("-o")
        .arg(target)
        .output()
        .context(DockerCallSnafu {
            message: "exporting image",
        })?;

    ensure!(
        res.status.success(),
        UnknownDockerResponseSnafu {
            message: "while exporting container",
            response: String::from_utf8_lossy(&res.stderr).to_string(),
        }
    );

    let res = Command::new("docker")
        .arg("rm")
        .arg(&container_id)
        .output()
        .context(DockerCallSnafu {
            message: "deleting container",
        })?;

    ensure!(
        res.status.success(),
        UnknownDockerResponseSnafu {
            message: "while removing container",
            response: String::from_utf8_lossy(&res.stderr).to_string(),
        }
    );

    Ok(())
}

pub fn export_image_unpacked(
    image_name: &ImageId,
    target_folder: impl AsRef<Path>,
) -> Result<(), DockerError> {
    let dir = tempfile::tempdir().context(TempfileIoSnafu)?;
    let tarball_path = dir.path().join("image.tar");
    export_image_to_tar(image_name, &tarball_path)?;

    tar::Archive::new(File::open(tarball_path).context(TempfileIoSnafu)?)
        .unpack(target_folder.as_ref())
        .context(TarExportIoSnafu)
}
