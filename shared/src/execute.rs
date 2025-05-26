use crate::exit::CrowExitStatus;
use crate::judge::judge_output;
use crate::{
    CompilerTest, ExecutionOutput, FinishedExecution, InternalError, TestExecutionOutput,
    TestModifier, TestModifierExt,
};
use is_executable::IsExecutable;
use snafu::{IntoError, NoneError, Report, ResultExt, Snafu};
use std::error::Error;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Snafu)]
enum ExecuteInternalError {
    #[snafu(display("Encountered error executing the compiler at {location}"))]
    Compiler {
        source: Box<dyn Error + Sync + Send>,
        runtime: Duration,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Compiler did not produce a valid executable at"))]
    CompilerFailed { compiler_output: ExecutionOutput },
    #[snafu(display("Encountered error executing the binary at {location}"))]
    Binary {
        source: Box<dyn Error + Sync + Send>,
        compiler_output: ExecutionOutput,
        runtime: Duration,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

impl From<ExecuteInternalError> for TestExecutionOutput {
    fn from(value: ExecuteInternalError) -> Self {
        let message = Report::from_error(&value).to_string();

        match value {
            ExecuteInternalError::Compiler { runtime, .. } => Self::CompilerFailed {
                compiler_output: ExecutionOutput::Error(InternalError { message, runtime }),
            },
            ExecuteInternalError::CompilerFailed {
                compiler_output, ..
            } => Self::CompilerFailed { compiler_output },
            ExecuteInternalError::Binary {
                compiler_output,
                runtime,
                ..
            } => Self::BinaryFailed {
                compiler_output,
                binary_output: ExecutionOutput::Error(InternalError { message, runtime }),
            },
        }
    }
}

/// The result of executing a command for judging
#[derive(Debug)]
pub enum CommandResult {
    /// The command failed and was already cleaned up into an [ExecutionOutput]
    ProcessedFailed(ExecutionOutput),
    /// The command was not processed yet, and we have the raw output
    Unprocessed((CrowExitStatus, FinishedExecution)),
}

pub fn execute_test(
    test: &CompilerTest,
    working_dir: &Path,
    output_binary_host_path: &Path,
    parent_dir_in_container: &Path,
    run_cmd: impl FnMut(
        &Path,
        &[String],
        Option<Duration>,
    ) -> Result<CommandResult, Box<dyn Error + Sync + Send>>,
) -> TestExecutionOutput {
    impl_execute_test(
        test,
        working_dir,
        output_binary_host_path,
        parent_dir_in_container,
        run_cmd,
    )
    .unwrap_or_else(From::from)
}

const TIMEOUT_MODIFIER_DURATION_SECONDS: u64 = 2;

#[allow(clippy::result_large_err)] // we accept it here, it will hopefully be inlined anyway
fn impl_execute_test(
    test: &CompilerTest,
    working_dir: &Path,
    output_binary_host_path: &Path,
    parent_dir_in_container: &Path,
    mut run_cmd: impl FnMut(
        &Path,
        &[String],
        Option<Duration>,
    ) -> Result<CommandResult, Box<dyn Error + Sync + Send>>,
) -> Result<TestExecutionOutput, ExecuteInternalError> {
    let should_run_binary = !test.binary_modifiers.is_empty();
    let output_binary_run_path = parent_dir_in_container.join(
        output_binary_host_path
            .file_name()
            .expect("Output binary should have a name"),
    );

    // Run the compiler
    let mut compiler_commands = test.compile_command[1..].to_vec();
    compiler_commands.extend(
        gather_arguments(
            &test.compiler_modifiers,
            working_dir,
            parent_dir_in_container,
        )
        .context(CompilerSnafu {
            runtime: Duration::ZERO,
        })?,
    );
    compiler_commands.push(output_binary_run_path.display().to_string());

    let start = Instant::now();
    let compiler_result = run_cmd(
        Path::new(&test.compile_command[0]),
        &compiler_commands,
        None,
    )
    .context(CompilerSnafu {
        runtime: start.elapsed(),
    })?;

    let compiler_output = match compiler_result {
        CommandResult::ProcessedFailed(output) => output,
        CommandResult::Unprocessed((exit_status, execution)) => {
            judge_output(&test.compiler_modifiers, exit_status, execution)
        }
    };

    if !matches!(compiler_output, ExecutionOutput::Success(_)) {
        return Ok(TestExecutionOutput::CompilerFailed { compiler_output });
    }

    if !should_run_binary {
        return Ok(TestExecutionOutput::Success {
            compiler_output,
            binary_output: None,
        });
    }

    // Verify we actually got out an executable so our later tests do not fail
    let compiler_output =
        *verify_compiler_built_executable(output_binary_host_path, compiler_output.clone());
    let compiler_output = compiler_output?;

    // Run the test
    let mut run_commands = test.binary_arguments.to_vec();
    run_commands.extend(
        gather_arguments(&test.binary_modifiers, working_dir, parent_dir_in_container).context(
            BinarySnafu {
                runtime: Duration::ZERO,
                compiler_output: compiler_output.clone(),
            },
        )?,
    );
    let timeout = if test.binary_modifiers.as_slice().should_timeout() {
        Some(Duration::from_secs(TIMEOUT_MODIFIER_DURATION_SECONDS))
    } else {
        None
    };
    let binary_result = run_cmd(Path::new(&output_binary_run_path), &run_commands, timeout)
        .context(BinarySnafu {
            runtime: start.elapsed(),
            compiler_output: compiler_output.clone(),
        })?;

    let binary_output = match binary_result {
        CommandResult::ProcessedFailed(ExecutionOutput::Timeout(execution)) => {
            judge_output(&test.binary_modifiers, CrowExitStatus::Timeout, execution)
        }
        CommandResult::ProcessedFailed(output) => output,
        CommandResult::Unprocessed((exit_status, execution)) => {
            judge_output(&test.binary_modifiers, exit_status, execution)
        }
    };

    if !matches!(binary_output, ExecutionOutput::Success(_)) {
        return Ok(TestExecutionOutput::BinaryFailed {
            compiler_output,
            binary_output,
        });
    }

    Ok(TestExecutionOutput::Success {
        compiler_output,
        binary_output: Some(binary_output),
    })
}

fn verify_compiler_built_executable(
    output_binary_path: &Path,
    compiler_output: ExecutionOutput,
) -> Box<Result<ExecutionOutput, ExecuteInternalError>> {
    match compiler_output {
        ExecutionOutput::Success(ref finished_exec) => {
            if !output_binary_path.exists() {
                return Box::new(Err(ExecuteInternalError::CompilerFailed {
                    compiler_output: ExecutionOutput::Failure {
                        execution: finished_exec.clone(),
                        accumulated_errors: Some(
                            "== ERROR ==\nNo output binary was created.\n\n".to_string(),
                        ),
                    },
                }));
            }
            if !output_binary_path.is_executable() {
                return Box::new(Err(ExecuteInternalError::CompilerFailed {
                    compiler_output: ExecutionOutput::Failure {
                        execution: finished_exec.clone(),
                        accumulated_errors: Some(
                            "== ERROR ==\nOutput binary is not executable.\n\n".to_string(),
                        ),
                    },
                }));
            }

            Box::new(Ok(compiler_output))
        }
        _ => Box::new(Err(ExecuteInternalError::CompilerFailed {
            compiler_output,
        })),
    }
}

fn gather_arguments(
    modifiers: &[TestModifier],
    work_dir: &Path,
    parent_dir_in_container: &Path,
) -> Result<Vec<String>, Box<dyn Error + Sync + Send>> {
    let mut args = Vec::new();

    let mut file_counter = 0;
    for modifier in modifiers {
        match modifier {
            TestModifier::ProgramArgument { arg } => args.push(arg.clone()),
            TestModifier::ProgramArgumentFile { contents } => {
                let file_name = format!("file_{file_counter}");
                std::fs::write(work_dir.join(&file_name), contents)?;
                args.push(
                    parent_dir_in_container
                        .join(file_name)
                        .display()
                        .to_string(),
                );

                file_counter += 1;
            }
            _ => {}
        }
    }

    Ok(args)
}

pub fn execute_locally(
    path: &Path,
    cmd: &[String],
    timeout: Option<Duration>,
) -> Result<CommandResult, Box<dyn Error + Sync + Send>> {
    let mut child = Command::new(path)
        .args(cmd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Box::new)?;

    let res = run_with_timeout(
        Arc::new(AtomicBool::new(false)),
        &mut child.stdout.take().expect("stdout"),
        &mut child.stderr.take().expect("stderr"),
        &mut child,
        timeout.unwrap_or(Duration::from_secs(5 * 60)), // Default to 5 minutes
    );
    let (stdout, stderr, status, runtime) = match res {
        Ok((stdout, stderr, status, runtime)) => (stdout, stderr, status.into(), runtime),
        Err(RunWithTimeoutError::Timeout {
            stdout,
            stderr,
            runtime,
            ..
        }) => {
            child.kill().map_err(Box::new)?;
            (stdout, stderr, CrowExitStatus::Timeout, runtime)
        }
        Err(e) => {
            // Make sure it is dead on error
            child.kill().map_err(Box::new)?;
            return Err(Box::new(e));
        }
    };

    Ok(CommandResult::Unprocessed((
        status,
        FinishedExecution {
            stdout,
            stderr,
            runtime,
            exit_status: status.code(),
        },
    )))
}

#[derive(Debug, Snafu)]
pub enum RunWithTimeoutError {
    Aborted {
        runtime: Duration,
        stdout: String,
        stderr: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    Timeout {
        runtime: Duration,
        stdout: String,
        stderr: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    WaitFailed {
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

pub fn run_with_timeout(
    aborted: Arc<AtomicBool>,
    stdout: &mut ChildStdout,
    stderr: &mut ChildStderr,
    process: &mut Child,
    timeout: Duration,
) -> Result<(String, String, ExitStatus, Duration), RunWithTimeoutError> {
    #[allow(unsafe_code)]
    unsafe {
        let flags = libc::fcntl(stdout.as_raw_fd(), libc::F_GETFL, 0);
        libc::fcntl(stdout.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK);
        let flags = libc::fcntl(stderr.as_raw_fd(), libc::F_GETFL, 0);
        libc::fcntl(stderr.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK);
    }

    let start = Instant::now();
    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();
    let mut exit_status: Result<ExitStatus, ()> = Err(());

    while Instant::now() < start + timeout {
        let mut tmpbuf = [0_u8; 1024];
        if let Ok(count) = stdout.read(&mut tmpbuf) {
            stdout_buf.extend_from_slice(&tmpbuf[..count]);
        }
        let mut tmpbuf = [0_u8; 1024];
        if let Ok(count) = stderr.read(&mut tmpbuf) {
            stderr_buf.extend_from_slice(&tmpbuf[..count]);
        }

        if aborted.load(Ordering::Relaxed) {
            return Err(AbortedSnafu {
                runtime: Instant::now().duration_since(start),
                stdout: String::from_utf8_lossy(&stdout_buf).to_string(),
                stderr: String::from_utf8_lossy(&stderr_buf).to_string(),
            }
            .into_error(NoneError));
        }

        match process.try_wait() {
            Err(e) => {
                return Err(WaitFailedSnafu.into_error(e));
            }
            Ok(None) => {
                thread::sleep(Duration::from_millis(100));
            }
            Ok(Some(status)) => {
                exit_status = Ok(status);
                break;
            }
        }
    }

    let stdout = String::from_utf8_lossy(&stdout_buf).to_string();
    let stderr = String::from_utf8_lossy(&stderr_buf).to_string();

    match exit_status {
        Ok(status) => Ok((stdout, stderr, status, Instant::now().duration_since(start))),
        Err(_) => Err(TimeoutSnafu {
            runtime: Instant::now().duration_since(start),
            stdout,
            stderr,
        }
        .into_error(NoneError)),
    }
}
