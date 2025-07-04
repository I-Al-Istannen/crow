use crate::docker::{Docker, DockerError, ImageId};
use derive_more::{Display, From};
use serde::Deserialize;
use shared::execute::{CommandResult, RunWithTimeoutError};
use shared::exit::CrowExitStatus;
use shared::{
    AbortedExecution, CompilerTest, ExecutionOutput, FinishedExecution, InternalError,
    TestExecutionOutput, remove_directory_force,
};
use snafu::{IntoError, Location, NoneError, Report, ResultExt, Snafu, ensure, location};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use std::{fs, io};
use tempfile::{TempDir, TempPath};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub const CROW_SIGNAL_SHIM_MAGIC: &str = "crow-internal_KILLED_BY_SIGNAL: ";
const CROW_SHIM_IN_CONTAINER_PATH: &str = "crow-shim";

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
pub enum IntegrateSourceError {
    #[snafu(display("Could not run 'tar -C `{work_path:?}` xf `{tar_path:?}`' at {location}"))]
    SourceUntarStart {
        source: io::Error,
        tar_path: PathBuf,
        work_path: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Could not untar `{tar_path:?}` into `{work_path:?}` with {stdout} and {stderr} at {location}"
    ))]
    SourceUntar {
        tar_path: PathBuf,
        work_path: PathBuf,
        stdout: String,
        stderr: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug)]
pub enum WaitForContainerError {
    #[snafu(display("Timeout occurred after `{runtime:?}` while waiting at {location}"))]
    Timeout {
        runtime: Duration,
        stdout: String,
        stderr: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Was asked and aborted run after {runtime:?} at {location}"))]
    Aborted {
        runtime: Duration,
        stdout: String,
        stderr: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not wait for the running container at {location}"))]
    WaitContainerIo {
        source: io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not kill container `{container_id}` at {location}"))]
    WaitContainerKillFailed {
        container_id: ContainerId,
        source: ContainerDestroyError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Killed container `{container_id}` as waiting failed at {location}"))]
    WaitContainerWaitFailed {
        container_id: ContainerId,
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
    #[snafu(display("Could not pass test into container via stdin at {location}"))]
    PassInputToContainer {
        source: io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Driver returned invalid json `{json}` with stderr\n`{stderr}` at {location}"
    ))]
    DriverInvalidJson {
        json: String,
        stderr: String,
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not find path to own executable at {location}"))]
    FindOwnExecutable {
        source: io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not copy own executable to container at {location}"))]
    CopyExecutorToContainer {
        source: io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not start the test container at {location}"))]
    ExecutionStart {
        source: io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Runc reported an error during container start: `{message}` at {location}"))]
    RuncStart {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not finish executing test container at {location}"))]
    Execution {
        source: WaitForContainerError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Underlying build container failed with {exit_status}, can not run tests at {location}"
    ))]
    BaseNotBuilt {
        exit_status: CrowExitStatus,
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

pub struct LimitsConfig {
    pub cpus: Option<u32>,
    pub memory_bytes: Option<usize>,
}

impl LimitsConfig {
    pub fn new(cpus: u32, memory_bytes: usize) -> Self {
        Self {
            cpus: if cpus > 0 { Some(cpus) } else { None },
            memory_bytes: if memory_bytes > 0 {
                Some(memory_bytes)
            } else {
                None
            },
        }
    }

    pub fn apply(&self, mut config: String) -> String {
        if let Some(cpus) = self.cpus {
            config = config.replace(
                "{cpu_limits}",
                &format!(
                    r#"
                   "cpu": {{
                       "quota": {},
                       "period": 100000
                   }},
                   "#,
                    cpus * 100_000
                ),
            )
        } else {
            config = config.replace("{cpu_limits}", "");
        }

        if let Some(memory) = self.memory_bytes {
            config = config.replace(
                "{memory_limits}",
                &format!(
                    r#"
                   "memory": {{
                       "limit": {memory},
                       "swap": {memory}
                   }},
                   "#
                ),
            )
        } else {
            config = config.replace("{memory_limits}", "");
        }

        config
    }
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
        args: &[String],
        exists_okay: bool,
        limits_config: &LimitsConfig,
    ) -> Result<PathBuf, RunConfigError> {
        let path_config = workdir.join("config.json");

        let root = match self {
            Self::WritableRootfs => {
                let config = include_str!("../resources/runc-read-write.json")
                    .replace("{rootfs}", &rootfs.display().to_string())
                    .replace("{host_uid}", &users::get_current_uid().to_string())
                    .replace(
                        "{args}",
                        &serde_json::to_string(args).context(ArgsNotJsonSnafu)?,
                    );
                let config = limits_config.apply(config);

                fs::write(&path_config, config).context(FileWriteSnafu {
                    path: path_config.to_path_buf(),
                })?;

                rootfs.to_path_buf()
            }
            Self::OverlayRootfs => {
                let path_upper = workdir.join("overlay-upper");
                let path_work = workdir.join("overlay-work");

                if !exists_okay || !path_upper.exists() {
                    fs::create_dir(&path_upper).context(FileWriteSnafu {
                        path: path_upper.to_path_buf(),
                    })?;
                }
                if !exists_okay || !path_work.exists() {
                    fs::create_dir(&path_work).context(FileWriteSnafu {
                        path: path_work.to_path_buf(),
                    })?;
                }

                let config = include_str!("../resources/runc-overlay.json")
                    .replace("{rootfs}", &rootfs.display().to_string())
                    .replace("{host_uid}", &users::get_current_uid().to_string())
                    .replace(
                        "{args}",
                        &serde_json::to_string(args).context(ArgsNotJsonSnafu)?,
                    )
                    .replace("{lower_dir}", &rootfs.display().to_string())
                    .replace("{upper_dir}", &path_upper.display().to_string())
                    .replace("{work_dir}", &path_work.display().to_string());
                let config = limits_config.apply(config);

                fs::write(&path_config, config).context(FileWriteSnafu {
                    path: path_config.to_path_buf(),
                })?;

                path_upper
            }
        };

        Ok(root)
    }
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
    pub exit_status: CrowExitStatus,
    pub runtime: Duration,
}

#[derive(Debug, Clone)]
pub struct ForTest<'a> {
    parent: &'a TaskContainer<Built>,
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
        args: &[String],
        docker: &Docker,
        limits: &LimitsConfig,
    ) -> Result<TaskContainer<Created>, ContainerCreateError> {
        let workdir = TempDir::new().context(TempDirCreationSnafu)?;
        let path_rootfs = workdir.path().join("rootfs");

        // We modify the rootfs during the build process (as these changes are replicated into each
        // container), so we need to copy it.
        docker
            .export_image_unpacked(image, &path_rootfs)
            .context(ImageCopySnafu)?;

        ContainerConfig::WritableRootfs
            .apply_to_workdir(&path_rootfs, workdir.path(), args, false, limits)
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
    pub fn integrate_source(&self, source_tar: TempPath) -> Result<(), IntegrateSourceError> {
        let work_path = self.rootfs.join("work");
        let tar_path = source_tar.to_path_buf();

        fs::create_dir(&work_path).context(SourceUntarStartSnafu {
            tar_path: tar_path.clone(),
            work_path: work_path.clone(),
        })?;

        let res = Command::new("tar")
            .arg("-C")
            .arg(&work_path)
            .arg("-xf")
            .arg(&source_tar)
            .output()
            .context(SourceUntarStartSnafu {
                tar_path: tar_path.clone(),
                work_path: work_path.clone(),
            })?;

        drop(source_tar);

        ensure!(
            res.status.success(),
            SourceUntarSnafu {
                tar_path,
                work_path,
                stdout: String::from_utf8_lossy(&res.stdout).to_string(),
                stderr: String::from_utf8_lossy(&res.stderr).to_string(),
            }
        );

        Ok(())
    }

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
    pub fn wait_for_build(
        mut self,
        timeout: Duration,
        aborted: Arc<AtomicBool>,
    ) -> Result<TaskContainer<Built>, ExecutionOutput> {
        let start = Instant::now();
        let wait_result = wait_for_container(
            aborted,
            &self.container_id,
            &mut self.data.stdout,
            &mut self.data.stderr,
            &mut self.data.process,
            timeout,
        );
        let (exit_status, wait_result) =
            match wait_result_to_command_result(&self.container_id, wait_result) {
                Ok(res) => match res {
                    CommandResult::ProcessedFailed(output) => return Err(output),
                    CommandResult::Unprocessed((status, execution)) => {
                        if !status.success() {
                            return Err(ExecutionOutput::Failure {
                                execution,
                                accumulated_errors: None,
                            });
                        }
                        (status, execution)
                    }
                },
                Err(e) => {
                    return Err(ExecutionOutput::Error(InternalError {
                        message: Report::from_error(e).to_string(),
                        runtime: start.elapsed(),
                    }));
                }
            };

        // Do not delete us on drop, we still live on in the new task container
        self.do_cleanup = false;

        Ok(TaskContainer {
            rootfs: self.rootfs.clone(),
            workdir: self.workdir.clone(),
            container_id: self.container_id.clone(),
            do_cleanup: true,
            data: Built {
                stdout: wait_result.stdout,
                stderr: wait_result.stderr,
                exit_status,
                runtime: wait_result.runtime,
            },
        })
    }
}

impl TaskContainer<Built> {
    pub fn run_test(
        &self,
        test: &CompilerTest,
        timeout: Duration,
        aborted: Arc<AtomicBool>,
        limits: &LimitsConfig,
    ) -> Result<TestExecutionOutput, TestRunError> {
        if !self.data.exit_status.success() {
            return Err(BaseNotBuiltSnafu {
                exit_status: self.data.exit_status,
            }
            .into_error(NoneError));
        }

        let mut test_container = TaskContainer::<ForTest<'_>>::new(self)?;
        let output_binary_path = test_container.rootfs.join("out.🦆");

        let res = shared::execute::execute_test(
            test,
            &test_container.rootfs.clone(),
            &output_binary_path,
            Path::new("/"),
            |path, cmd, override_timeout, stdin| {
                let timeout = override_timeout.unwrap_or(timeout);
                let res = test_container.execute_command(
                    path,
                    cmd,
                    aborted.clone(),
                    timeout,
                    limits,
                    stdin,
                );
                match res {
                    Ok(res) => Ok(res),
                    Err(e) => Err(Box::new(e)),
                }
            },
        );

        Ok(res)
    }
}

impl<'a> TaskContainer<ForTest<'a>> {
    pub fn new(outer: &'a TaskContainer<Built>) -> Result<Self, TestRunError> {
        if !outer.data.exit_status.success() {
            return Err(BaseNotBuiltSnafu {
                exit_status: outer.data.exit_status,
            }
            .into_error(NoneError));
        }

        let workdir = TempDir::new()
            .context(TempDirCreationSnafu)
            .context(CreationSnafu)?;
        let container_id = ContainerId(Uuid::new_v4().to_string());

        let container_root = workdir.path().join("overlay-upper");
        fs::create_dir(&container_root)
            .context(TempDirCreationSnafu)
            .context(CreationSnafu)?;

        fs::copy(
            std::env::current_exe().context(FindOwnExecutableSnafu)?,
            container_root.join(CROW_SHIM_IN_CONTAINER_PATH),
        )
        .context(CopyExecutorToContainerSnafu)?;

        Ok(Self {
            workdir: workdir.into_path(),
            rootfs: container_root,
            container_id,
            do_cleanup: true,
            data: ForTest { parent: outer },
        })
    }

    pub fn execute_command(
        &mut self,
        binary_path: &Path,
        args: &[String],
        aborted: Arc<AtomicBool>,
        timeout: Duration,
        limits: &LimitsConfig,
        stdin: String,
    ) -> Result<CommandResult, TestRunError> {
        let mut full_command = vec![
            format!("/{CROW_SHIM_IN_CONTAINER_PATH}"),
            "shim".to_string(),
            "--".to_string(),
            binary_path.to_str().expect("path was Unicode").to_string(),
        ];
        full_command.extend_from_slice(args);

        ContainerConfig::OverlayRootfs
            .apply_to_workdir(
                &self.data.parent.rootfs,
                &self.workdir,
                &full_command,
                true,
                limits,
            )
            .context(ConfigApplySnafu)
            .context(CreationSnafu)?;

        let mut process =
            start_container(&self.workdir, &self.container_id).context(ExecutionStartSnafu)?;

        // Do this in a new thread to ensure it does not block ourselves, which would prevent
        // us from advancing the stdout of the child, creating a deadlock.
        let mut stdin_pipe = process.stdin.take().expect("stdin was piped");
        let stdin_error = std::thread::spawn(move || {
            stdin_pipe
                .write_all(stdin.as_bytes())
                .context(PassInputToContainerSnafu)
        });

        let res = wait_for_container(
            aborted,
            &self.container_id,
            &mut process.stdout.take().unwrap(),
            &mut process.stderr.take().unwrap(),
            &mut process,
            timeout,
        );

        // If we finished the stdin writing, and it had an error, we probably want to report it.
        // If it did not finish, something weird happened, and it will finish at some point, as
        // the child process should be dead by now. The OS just might take a bit to communicate
        // that.
        if stdin_error.is_finished() {
            if let Ok(res) = stdin_error.join() {
                res?;
            }
        } else {
            info!(
                container_id = %self.container_id,
                "Stdin writing did not finish before waiting for container"
            );
        }

        wait_result_to_command_result(&self.container_id, res)
    }
}

impl<T> Drop for TaskContainer<T> {
    fn drop(&mut self) {
        if !self.do_cleanup {
            return;
        }

        if let Err(e) = kill_container(&self.container_id) {
            error!(
                error = %Report::from_error(e),
                container = ?self.container_id,
                "Failed to kill container"
            );
        }
        if let Err(e) = delete_container_dir(&self.container_id, &self.workdir) {
            error!(
                error = %Report::from_error(e),
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
    aborted: Arc<AtomicBool>,
    container_id: &ContainerId,
    stdout: &mut ChildStdout,
    stderr: &mut ChildStderr,
    process: &mut Child,
    timeout: Duration,
) -> Result<(String, String, ExitStatus, Duration), WaitForContainerError> {
    let res = shared::execute::run_with_timeout(aborted, stdout, stderr, process, timeout);

    match res {
        Ok(res) => Ok(res),
        Err(RunWithTimeoutError::Aborted {
            runtime,
            stdout,
            stderr,
            ..
        }) => {
            if let Err(e) = kill_container(container_id) {
                error!(
                    container_id = %container_id,
                    error = ?e,
                    "Failed to kill container after abort"
                );
                return Err(WaitContainerKillFailedSnafu {
                    container_id: container_id.clone(),
                }
                .into_error(e));
            }
            Err(WaitForContainerError::Aborted {
                runtime,
                stdout,
                stderr,
                location: location!(),
            })
        }
        Err(RunWithTimeoutError::Timeout {
            runtime,
            stdout,
            stderr,
            ..
        }) => Err(WaitForContainerError::Timeout {
            runtime,
            stdout,
            stderr,
            location: location!(),
        }),
        Err(RunWithTimeoutError::WaitFailed { source, .. }) => {
            warn!(
                container_id = %container_id,
                error = ?source,
                "Error while waiting for container"
            );
            if let Err(e) = kill_container(container_id) {
                error!(
                    container_id = %container_id,
                    error = ?e,
                    "Failed to kill container after wait error"
                );
                return Err(WaitContainerKillFailedSnafu {
                    container_id: container_id.clone(),
                }
                .into_error(e));
            }
            Err(WaitContainerWaitFailedSnafu {
                container_id: container_id.clone(),
            }
            .into_error(source))
        }
    }
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

    match remove_directory_force(workdir) {
        Ok(_) => Ok(()),
        Err(e) => {
            debug!(
                workdir = %workdir.display(),
                error = %Report::from_error(&e),
                "Failed to delete container workdir"
            );
            Err(e).context(DirNotDeletedSnafu {
                path: workdir.to_path_buf(),
            })
        }
    }
}

fn wait_result_to_command_result(
    container_id: &ContainerId,
    res: Result<(String, String, ExitStatus, Duration), WaitForContainerError>,
) -> Result<CommandResult, TestRunError> {
    let (stdout, mut stderr, exit_status, runtime) = match res {
        Err(e) => {
            if let Some(output) = execution_output_from_wait_error(&e) {
                return Ok(CommandResult::ProcessedFailed(output));
            }
            return Err(e).context(ExecutionSnafu);
        }
        Ok(res) => res,
    };
    let mut exit_status: CrowExitStatus = exit_status.into();

    if !exit_status.success()
        && stderr.trim().lines().count() == 1
        && stderr.contains("runc run failed:")
    {
        warn!(
            container_id = %container_id,
            message = %stderr,
            "Running a test failed with a runc error"
        );
        return Err(RuncStartSnafu {
            message: stderr.trim(),
        }
        .into_error(NoneError));
    }

    if let Some(last_line) = stderr.trim().lines().last() {
        if let Some(rest) = last_line.strip_prefix(CROW_SIGNAL_SHIM_MAGIC) {
            if let Ok(signal) = rest.trim().parse::<i32>() {
                exit_status = CrowExitStatus::WithSignal { signal };

                // Remove our internal marker from stderr
                let mut lines = stderr.lines().collect::<Vec<_>>();
                lines.pop();
                stderr = lines.join("\n");
            }
        }
    }

    Ok(CommandResult::Unprocessed((
        exit_status,
        FinishedExecution {
            exit_status: exit_status.code(),
            stdout,
            stderr,
            runtime,
        },
    )))
}

pub fn execution_output_from_wait_error(error: &WaitForContainerError) -> Option<ExecutionOutput> {
    if let WaitForContainerError::Timeout {
        runtime,
        stdout,
        stderr,
        ..
    } = error
    {
        return Some(ExecutionOutput::Timeout(FinishedExecution {
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            runtime: *runtime,
            exit_status: None,
        }));
    }
    if let WaitForContainerError::Aborted {
        runtime,
        stdout,
        stderr,
        ..
    } = error
    {
        return Some(ExecutionOutput::Aborted(AbortedExecution {
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            runtime: *runtime,
        }));
    }

    None
}
