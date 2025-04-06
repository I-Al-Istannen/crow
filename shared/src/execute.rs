use crate::judge::judge_output;
use crate::{
    CompilerTest, ExecutionOutput, FinishedExecution, InternalError, TestExecutionOutput,
    TestModifierExt,
};
use is_executable::IsExecutable;
use snafu::{IntoError, Report, Snafu};
use std::error::Error;
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::time::Instant;

#[derive(Debug, Snafu)]
enum ExecuteInternalError {
    #[snafu(display("Executing the command failed at {location}"))]
    RunCommand {
        source: Box<dyn Error + Sync + Send>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

/// The result of executing a command for judging
#[derive(Debug)]
pub enum CommandResult {
    /// The command failed and was already cleaned up into an [ExecutionOutput]
    ProcessedFailed(ExecutionOutput),
    /// The command was not processed yet, and we have the raw output
    Unprocessed((ExitStatus, FinishedExecution)),
}

pub fn execute_test(
    test: &CompilerTest,
    output_binary_path: &Path,
    output_binary_cmd_arg: String,
    mut run_cmd: impl FnMut(&Path, &[String]) -> Result<CommandResult, Box<dyn Error + Sync + Send>>,
) -> TestExecutionOutput {
    // TODO: Provide input somehow.

    // Run the compiler
    let mut compiler_commands = test.compile_command[1..].to_vec();
    compiler_commands.extend(test.compiler_modifiers.as_slice().all_arguments());
    compiler_commands.push(output_binary_cmd_arg.clone());
    let start = Instant::now();
    let compiler_result = run_cmd(Path::new(&test.compile_command[0]), &compiler_commands);
    let compiler_result = match compiler_result {
        Ok(val) => val,
        Err(e) => {
            return TestExecutionOutput::CompilerFailed {
                compiler_output: ExecutionOutput::Error(InternalError {
                    message: Report::from_error(RunCommandSnafu.into_error(e)).to_string(),
                    runtime: start.elapsed(),
                }),
            };
        }
    };

    let compiler_output = match compiler_result {
        CommandResult::ProcessedFailed(output) => output,
        CommandResult::Unprocessed((exit_status, execution)) => {
            judge_output(&test.compiler_modifiers, exit_status, execution)
        }
    };

    // Verify we actually got out an executable so our later tests do not fail
    let compiler_output =
        *verify_compiler_built_executable(output_binary_path, compiler_output.clone());
    let compiler_output = match compiler_output {
        Ok(output) => output,
        Err(e) => {
            return e;
        }
    };

    // Run the test
    let mut run_commands = test.binary_arguments.to_vec();
    run_commands.extend(test.binary_modifiers.as_slice().all_arguments());
    let binary_result = run_cmd(Path::new(&output_binary_cmd_arg), &run_commands);
    let binary_result = match binary_result {
        Ok(val) => val,
        Err(e) => {
            return TestExecutionOutput::BinaryFailed {
                compiler_output,
                binary_output: ExecutionOutput::Error(InternalError {
                    message: Report::from_error(RunCommandSnafu.into_error(e)).to_string(),
                    runtime: start.elapsed(),
                }),
            };
        }
    };

    let binary_output = match binary_result {
        CommandResult::ProcessedFailed(output) => output,
        CommandResult::Unprocessed((exit_status, execution)) => {
            judge_output(&test.binary_modifiers, exit_status, execution)
        }
    };

    if !matches!(binary_output, ExecutionOutput::Success(_)) {
        return TestExecutionOutput::BinaryFailed {
            compiler_output,
            binary_output,
        };
    }

    TestExecutionOutput::Success {
        compiler_output,
        binary_output,
    }
}

fn verify_compiler_built_executable(
    output_binary_path: &Path,
    compiler_output: ExecutionOutput,
) -> Box<Result<ExecutionOutput, TestExecutionOutput>> {
    match compiler_output {
        ExecutionOutput::Success(ref finished_exec) => {
            if !output_binary_path.exists() {
                let mut finished_exec = finished_exec.clone();
                finished_exec
                    .stderr
                    .insert_str(0, "== ERROR ==\nNo output binary was created.\n\n");

                return Box::new(Err(TestExecutionOutput::CompilerFailed {
                    compiler_output: ExecutionOutput::Failure(finished_exec),
                }));
            }
            if !output_binary_path.is_executable() {
                let mut finished_exec = finished_exec.clone();
                finished_exec
                    .stderr
                    .insert_str(0, "== ERROR ==\nOutput binary is not executable.\n\n");

                return Box::new(Err(TestExecutionOutput::CompilerFailed {
                    compiler_output: ExecutionOutput::Failure(finished_exec),
                }));
            }

            Box::new(Ok(compiler_output))
        }
        _ => Box::new(Err(TestExecutionOutput::CompilerFailed { compiler_output })),
    }
}

pub fn execute_locally(
    path: &Path,
    cmd: &[String],
) -> Result<CommandResult, Box<dyn Error + Sync + Send>> {
    let start = Instant::now();
    let output = Command::new(path).args(cmd).output().map_err(Box::new)?;

    Ok(CommandResult::Unprocessed((
        output.status,
        FinishedExecution {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            runtime: start.elapsed(),
            exit_status: output.status.code(),
        },
    )))
}
