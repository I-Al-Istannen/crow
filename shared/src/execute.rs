use crate::judge::judge_output;
use crate::{
    CompilerTest, ExecutionOutput, FinishedExecution, InternalError, TestExecutionOutput,
    TestModifierExt,
};
use snafu::{Location, Report, ResultExt, Snafu};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

#[derive(Debug, Snafu)]
pub enum ExecuteError {
    #[snafu(display("Starting compiler command failed at {location}"))]
    CompilerInvocation {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Starting binary `{}` failed at {location}", path.display()))]
    BinaryInvocation {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

pub fn execute_test(
    test: CompilerTest,
    output_binary_path: &Path,
) -> Result<TestExecutionOutput, ExecuteError> {
    // TODO: Provide input somehow.

    // Run the compiler
    let mut compiler_commands = test.compile_command[1..].to_vec();
    compiler_commands.extend(test.compiler_modifiers.as_slice().all_arguments());
    compiler_commands.push(
        output_binary_path
            .to_str()
            .expect("path was Unicode")
            .to_string(),
    );
    let run_start = Instant::now();
    let compiler_output = Command::new(&test.compile_command[0])
        .args(&compiler_commands)
        .output()
        .context(CompilerInvocationSnafu)?;

    let compiler_output = judge_output(
        &test.compiler_modifiers,
        compiler_output.status,
        output_to_execution(run_start, compiler_output),
    );

    if !matches!(compiler_output, ExecutionOutput::Success(_)) {
        return Ok(TestExecutionOutput::CompilerFailed { compiler_output });
    }

    // Run the test
    let mut run_commands = test.run_command.to_vec();
    run_commands.extend(test.binary_modifiers.as_slice().all_arguments());
    let run_start = Instant::now();
    let test_output = Command::new(output_binary_path)
        .args(&run_commands)
        .output()
        .context(BinaryInvocationSnafu {
            path: output_binary_path.to_path_buf(),
        });

    let test_output = match test_output {
        Ok(output) => output,
        Err(e) => {
            return Ok(TestExecutionOutput::BinaryFailed {
                compiler_output: compiler_output.clone(),
                binary_output: ExecutionOutput::Error(InternalError {
                    message: Report::from_error(e).to_string(),
                    runtime: Duration::from_secs(0),
                }),
            });
        }
    };

    let binary_output = judge_output(
        &test.binary_modifiers,
        test_output.status,
        output_to_execution(run_start, test_output),
    );

    if !matches!(binary_output, ExecutionOutput::Success(_)) {
        return Ok(TestExecutionOutput::BinaryFailed {
            compiler_output,
            binary_output,
        });
    }

    Ok(TestExecutionOutput::Success {
        compiler_output,
        binary_output,
    })
}

fn output_to_execution(start_time: Instant, output: Output) -> FinishedExecution {
    FinishedExecution {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_status: output.status.code(),
        runtime: Instant::now().duration_since(start_time),
    }
}
