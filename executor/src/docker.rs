use derive_more::Display;
use file_guard::os::unix::FileGuardExt;
use file_guard::Lock;
use shared::indent;
use snafu::{location, IntoError, Location, NoneError, ResultExt, Snafu};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tracing::{info, warn};

#[derive(Debug, Snafu)]
pub enum DockerError {
    #[snafu(display("Could not delete cache directory `{}` at {location}", path.display()))]
    CacheDirDelete {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not create cache directory `{}` at {location}", path.display()))]
    CacheDirCreate {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Could not copy image from cache `{}` to target folder `{}` at {location}",
        cache.display(),
        target.display()
    ))]
    CacheImageCopy {
        cache: PathBuf,
        target: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not create lock file `{}` at {location}", path.display()))]
    LockFileCreate {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not create lock file `{}` at {location}", path.display()))]
    LockFileAcquire {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not acquire lock file `{}` at {location}",path.display()))]
    LockFileAcquireTriesExceeded {
        path: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
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

fn export_image_to_tar(image: &ImageId, target: &Path) -> Result<(), DockerError> {
    // Touch the file to ensure we can actually write to it
    File::create(target).context(TarExportIoSnafu)?;

    let res = Command::new("docker")
        .arg("create")
        .arg("-q")
        .arg(image.to_string())
        .handle_exitcode()
        .context(DockerCallSnafu {
            message: "creating container",
        })?;

    let container_id = String::from_utf8_lossy(&res.stdout).trim().to_string();

    Command::new("docker")
        .arg("export")
        .arg(&container_id)
        .arg("-o")
        .arg(target)
        .handle_exitcode()
        .context(DockerCallSnafu {
            message: "exporting image",
        })?;

    Command::new("docker")
        .arg("rm")
        .arg(&container_id)
        .handle_exitcode()
        .context(DockerCallSnafu {
            message: "deleting container",
        })?;

    Ok(())
}

pub struct Docker {
    cache_folder: Option<PathBuf>,
}

impl Docker {
    pub fn new(cache_folder: Option<PathBuf>) -> Result<Self, DockerError> {
        if let Some(cache) = &cache_folder {
            if cache.exists() {
                Command::new("rm")
                    .arg("-rf")
                    .arg(cache)
                    .handle_exitcode()
                    .context(CacheDirDeleteSnafu {
                        path: cache.clone(),
                    })?;
            }
            std::fs::create_dir_all(cache).context(CacheDirCreateSnafu {
                path: cache.clone(),
            })?;
        }

        Ok(Self { cache_folder })
    }

    pub fn export_image_unpacked(
        &self,
        image_name: &ImageId,
        target_folder: impl AsRef<Path>,
    ) -> Result<(), DockerError> {
        if let Some(cache) = &self.cache_folder {
            return export_image_cached(image_name, target_folder, cache);
        }
        export_image_to_dir(image_name, target_folder.as_ref())
    }
}

fn export_image_cached(
    image_name: &ImageId,
    target_folder: impl AsRef<Path>,
    cache_folder: impl AsRef<Path>,
) -> Result<(), DockerError> {
    let image_id = get_docker_image_id(image_name)?;
    let cached_path = cache_folder.as_ref().join(&image_id);
    let lockfile_path = cache_folder.as_ref().join(".lock");

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&lockfile_path)
        .context(LockFileCreateSnafu {
            path: lockfile_path.clone(),
        })?;

    let mut guard =
        file_guard::lock(&mut file, Lock::Shared, 0, 1).context(LockFileAcquireSnafu {
            path: lockfile_path.to_path_buf(),
        })?;

    if cached_path.exists() {
        info!(
            image_name = %image_name,
            image_id = %image_id,
            cache_path = %cached_path.display(),
            "Reusing image from cache"
        );
        copy_image_from_cache(target_folder.as_ref().to_path_buf(), cached_path)?;

        return Ok(());
    }
    info!(
        image_name = %image_name,
        image_id = %image_id,
        cache_path = %cached_path.display(),
        "Exporting image to cache"
    );

    for i in 0..10 {
        if let Err(e) = guard.try_upgrade() {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                info!("Waiting for lock to be released. Iteration {}/10", i + 1);
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            } else {
                return Err(e).context(LockFileAcquireSnafu {
                    path: lockfile_path.to_path_buf(),
                });
            }
        } else {
            break;
        }
    }

    if !guard.is_exclusive() {
        warn!("Lock is not exclusive, acquire failed");
        return Err(DockerError::LockFileAcquireTriesExceeded {
            path: lockfile_path.to_path_buf(),
            location: location!(),
        });
    }

    export_image_to_dir(image_name, &cached_path)?;

    copy_image_from_cache(target_folder.as_ref().to_path_buf(), cached_path.clone())?;

    // Release file lock
    drop(guard);

    Ok(())
}

fn export_image_to_dir(image_name: &ImageId, target_folder: &Path) -> Result<(), DockerError> {
    let dir = tempfile::tempdir().context(TempfileIoSnafu)?;
    let tarball_path = dir.path().join("image.tar");
    export_image_to_tar(image_name, &tarball_path)?;

    tar::Archive::new(File::open(tarball_path).context(TempfileIoSnafu)?)
        .unpack(target_folder)
        .context(TarExportIoSnafu)?;

    Ok(())
}

fn copy_image_from_cache(target_folder: PathBuf, cached_path: PathBuf) -> Result<(), DockerError> {
    Command::new("cp")
        .arg("-r")
        .arg(&cached_path)
        .arg(&target_folder)
        .handle_exitcode()
        .context(CacheImageCopySnafu {
            cache: cached_path,
            target: target_folder,
        })?;

    Ok(())
}

fn get_docker_image_id(image: &ImageId) -> Result<String, DockerError> {
    let output = Command::new("docker")
        .arg("image")
        .arg("inspect")
        .arg(image.to_string())
        .arg("--format")
        .arg("{{ .Id }}")
        .output()
        .context(DockerCallSnafu {
            message: "verifying image exists",
        })?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stdout.is_empty() && stderr.to_lowercase().contains("no such image") {
            return Err(ImageNotFoundSnafu {
                image: image.clone(),
            }
            .into_error(NoneError));
        }
        return Err(UnknownDockerResponseSnafu {
            message: "while inspecting image",
            response: stderr,
        }
        .into_error(NoneError));
    }

    let output = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let output = output.lines().next().unwrap_or_default().to_string();

    Ok(output)
}

trait HandleExitcode {
    fn handle_exitcode(self) -> std::io::Result<Output>;
}

impl HandleExitcode for &mut Command {
    fn handle_exitcode(self) -> std::io::Result<Output> {
        let output = self.output()?;

        if output.status.success() {
            return Ok(output);
        }
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        let mut response = "".to_string();
        if let Some(code) = output.status.code() {
            response.push_str(&format!("Exited with code {code}\n"));
        }
        if !stdout.trim().is_empty() {
            response.push_str(&format!("stdout:\n{}\n", indent(stdout.trim(), 2)));
        }
        if !stderr.trim().is_empty() {
            response.push_str(&format!("stderr:\n{}", indent(stderr.trim(), 2)));
        }

        Err(std::io::Error::new(std::io::ErrorKind::Other, response))
    }
}
