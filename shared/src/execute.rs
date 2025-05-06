use crate::exit::CrowExitStatus;
use crate::judge::judge_output;
use crate::{
    CompilerTest, ExecutionOutput, FinishedExecution, InternalError, TestExecutionOutput,
    TestModifier,
};
use is_executable::IsExecutable;
use snafu::{Report, ResultExt, Snafu};
use std::error::Error;
use std::path::Path;
use std::process::Command;
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
    run_cmd: impl FnMut(&Path, &[String]) -> Result<CommandResult, Box<dyn Error + Sync + Send>>,
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

#[allow(clippy::result_large_err)] // we accept it here, it will hopefully be inlined anyway
fn impl_execute_test(
    test: &CompilerTest,
    working_dir: &Path,
    output_binary_host_path: &Path,
    parent_dir_in_container: &Path,
    mut run_cmd: impl FnMut(&Path, &[String]) -> Result<CommandResult, Box<dyn Error + Sync + Send>>,
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
    let compiler_result = run_cmd(Path::new(&test.compile_command[0]), &compiler_commands)
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
    let binary_result =
        run_cmd(Path::new(&output_binary_run_path), &run_commands).context(BinarySnafu {
            runtime: start.elapsed(),
            compiler_output: compiler_output.clone(),
        })?;

    let binary_output = match binary_result {
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
) -> Result<CommandResult, Box<dyn Error + Sync + Send>> {
    let start = Instant::now();
    let output = Command::new(path).args(cmd).output().map_err(Box::new)?;

    Ok(CommandResult::Unprocessed((
        output.status.into(),
        FinishedExecution {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            runtime: start.elapsed(),
            exit_status: output.status.code(),
        },
    )))
}
