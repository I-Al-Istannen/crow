use crate::docker::{export_image_unpacked, DockerError, ImageId};
use derive_more::{Display, From};
use serde::Deserialize;
use snafu::{IntoError, Location, NoneError, ResultExt, Snafu};
use std::fs::create_dir;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::{fs, io, thread};
use tempfile::TempDir;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Snafu, Debug)]
pub enum RunConfigError {
    #[snafu(display("Could not serialize arguments to json at {location}"))]
    ArgsNotJson {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not write file `{path:?}` at {location}"))]
    FileWrite {
        source: io::Error,
        path: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug)]
pub enum ContainerCreateError {
    #[snafu(display("Could not copy image rootfs at {location}"))]
    ImageCopy {
        source: DockerError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not apply container config at {location}"))]
    ConfigApply {
        source: RunConfigError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not create temporary directory at {location}"))]
    TempDirCreation {
        source: io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug)]
pub enum ContainerDestroyError {
    #[snafu(display("Could not execute kill command for `{container_id}` at {location}"))]
    KillInvocation {
        container_id: ContainerId,
        source: io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Could not understand response to killing `{container_id}`: `{message:?}` at {location}"
    ))]
    UnknownKillFailure {
        container_id: ContainerId,
        message: RuncLogMessage,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Could not parse response to killing `{container_id}`: `{raw_output}` at {location}"
    ))]
    KillOutputUnparsable {
        container_id: ContainerId,
        source: serde_json::Error,
        raw_output: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not delete directory `{path:?}` at {location}"))]
    DirNotDeleted {
        source: io::Error,
        path: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug)]
pub enum TestRunError {
    #[snafu(display("Could not create a new temporary directory at {location}"))]
    Creation {
        source: ContainerCreateError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not start the test container at {location}"))]
    ExecutionStart {
        source: io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not finish executing test container at {location}"))]
    Execution {
        source: io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Underlying build container failed with {exit_status}, can not run tests at {location}"
    ))]
    BaseNotBuilt {
        exit_status: ExitStatus,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuncLogMessage {
    #[allow(unused)]
    pub level: String,
    pub msg: String,
    #[allow(unused)]
    pub time: String,
}

pub enum ContainerConfig {
    WritableRootfs,
    OverlayRootfs,
}

impl ContainerConfig {
    pub fn apply_to_workdir(
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

                fs::write(&path_config, config).context(FileWriteSnafu {
                    path: path_config.to_path_buf(),
                })?;
            }
            ContainerConfig::OverlayRootfs => {
                let path_upper = workdir.join("overlay-upper");
                let path_work = workdir.join("overlay-work");

                create_dir(&path_upper).context(FileWriteSnafu {
                    path: path_upper.to_path_buf(),
                })?;
                create_dir(&path_work).context(FileWriteSnafu {
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

                fs::write(&path_config, config).context(FileWriteSnafu {
                    path: path_config.to_path_buf(),
                })?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TestRunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_status: ExitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, From, Display)]
pub struct ContainerId(String);

#[derive(Debug, Clone)]
pub struct Created;

#[derive(Debug)]
pub struct Started {
    stdout: ChildStdout,
    stderr: ChildStderr,
    process: Child,
}

#[derive(Debug, Clone)]
pub struct Built {
    pub stdout: String,
    pub stderr: String,
    pub exit_status: ExitStatus,
}

#[derive(Debug)]
#[must_use]
pub struct TaskContainer<T> {
    workdir: PathBuf,
    rootfs: PathBuf,
    container_id: ContainerId,
    do_cleanup: bool,
    pub data: T,
}

impl TaskContainer<()> {
    pub fn new(
        image: &ImageId,
        args: &[&str],
    ) -> Result<TaskContainer<Created>, ContainerCreateError> {
        let workdir = TempDir::new().context(TempDirCreationSnafu)?;
        let path_rootfs = workdir.path().join("rootfs");

        // We modify the rootfs during the build process (as these changes are replicated into each
        // container), so we need to copy it.
        export_image_unpacked(image, &path_rootfs).context(ImageCopySnafu)?;

        ContainerConfig::WritableRootfs
            .apply_to_workdir(&path_rootfs, workdir.path(), args)
            .context(ConfigApplySnafu)?;

        Ok(TaskContainer {
            workdir: workdir.into_path(),
            rootfs: path_rootfs,
            container_id: ContainerId(Uuid::new_v4().to_string()),
            do_cleanup: true,
            data: Created,
        })
    }
}

impl TaskContainer<Created> {
    pub fn run(mut self) -> io::Result<TaskContainer<Started>> {
        let mut process = start_container(&self.workdir, &self.container_id)?;

        let stdout = process.stdout.take();
        let stderr = process.stderr.take();

        // Do not delete us on drop, we still live on in the new task container
        self.do_cleanup = false;

        Ok(TaskContainer {
            rootfs: self.rootfs.clone(),
            workdir: self.workdir.clone(),
            container_id: self.container_id.clone(),
            do_cleanup: true,
            data: Started {
                stdout: stdout.unwrap(),
                stderr: stderr.unwrap(),
                process,
            },
        })
    }
}

impl TaskContainer<Started> {
    pub fn wait_for_build(mut self) -> io::Result<TaskContainer<Built>> {
        let (stdout, stderr, result) = wait_for_container(
            &mut self.data.stdout,
            &mut self.data.stderr,
            &mut self.data.process,
        )?;

        // Do not delete us on drop, we still live on in the new task container
        self.do_cleanup = false;

        Ok(TaskContainer {
            rootfs: self.rootfs.clone(),
            workdir: self.workdir.clone(),
            container_id: self.container_id.clone(),
            do_cleanup: true,
            data: Built {
                stdout,
                stderr,
                exit_status: result,
            },
        })
    }
}

impl TaskContainer<Built> {
    pub fn run_test(&self, args: &[&str]) -> Result<TestRunResult, TestRunError> {
        if !self.data.exit_status.success() {
            return Err(BaseNotBuiltSnafu {
                exit_status: self.data.exit_status,
            }
            .into_error(NoneError));
        }

        let workdir = TempDir::new()
            .context(TempDirCreationSnafu)
            .context(CreationSnafu)?;
        let rootfs = &self.rootfs;

        ContainerConfig::OverlayRootfs
            .apply_to_workdir(rootfs, workdir.path(), args)
            .context(ConfigApplySnafu)
            .context(CreationSnafu)?;

        let container_id = ContainerId(Uuid::new_v4().to_string());

        let mut process =
            start_container(workdir.path(), &container_id).context(ExecutionStartSnafu)?;

        let res = wait_for_container(
            &mut process.stdout.take().unwrap(),
            &mut process.stderr.take().unwrap(),
            &mut process,
        );

        if let Err(e) = delete_container_dir(&container_id, workdir.path()) {
            error!(
                error = ?e,
                container = ?container_id,
                "Failed to delete test container workdir"
            );
        }

        let (stdout, stderr, result) = res.context(ExecutionSnafu)?;

        Ok(TestRunResult {
            stdout,
            stderr,
            exit_status: result,
        })
    }
}

impl<T> Drop for TaskContainer<T> {
    fn drop(&mut self) {
        if !self.do_cleanup {
            return;
        }

        if let Err(e) = kill_container(&self.container_id) {
            error!(
                error = ?e,
                container = ?self.container_id,
                "Failed to kill container"
            );
        }
        if let Err(e) = delete_container_dir(&self.container_id, &self.workdir) {
            error!(
                error = ?e,
                container = ?self.container_id,
                "Failed to delete container workdir"
            );
        }
    }
}

fn start_container(workdir: &Path, container_id: &ContainerId) -> io::Result<Child> {
    Command::new("runc")
        .arg("run")
        .arg(container_id.to_string())
        .current_dir(workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
}

fn wait_for_container(
    stdout: &mut ChildStdout,
    stderr: &mut ChildStderr,
    process: &mut Child,
) -> io::Result<(String, String, ExitStatus)> {
    let (stdout, stderr, result) = thread::scope(|s| {
        let stdout = s.spawn(|| {
            let mut string = String::new();
            let res = stdout.read_to_string(&mut string);
            res.map(|_| string)
        });
        let stderr = s.spawn(|| {
            let mut string = String::new();
            let res = stderr.read_to_string(&mut string);
            res.map(|_| string)
        });
        let result = process.wait();

        (stdout.join().unwrap(), stderr.join().unwrap(), result)
    });

    Ok((stdout?, stderr?, result?))
}

fn kill_container(container_id: &ContainerId) -> Result<(), ContainerDestroyError> {
    debug!(container_id = %container_id, "Killing container");

    let res = Command::new("runc")
        .arg("--log-format=json")
        .arg("kill")
        .arg(container_id.to_string())
        // We directly SIGKILL, the container does not have any persistent state anyway
        .arg("KILL")
        .output()
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

fn delete_container_dir(
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
