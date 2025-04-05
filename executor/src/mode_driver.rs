use crate::{AnyError, DriverSnafu};
use shared::judge::judge_output;
use shared::{CompilerTest, ExecutionOutput, FinishedExecution, TestExecutionOutput};
use snafu::{Location, ResultExt, Snafu};
use std::io::stdin;
use std::process::{Command, Output};
use std::time::Instant;

#[derive(Debug, Snafu)]
pub enum DriverError {
    #[snafu(display("Starting compiler command failed at {location}"))]
    CompilerInvocation {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Starting binary command failed at {location}"))]
    BinaryInvocation {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Serializing output failed at {location}"))]
    SerializeOutput {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

pub fn run_driver() -> Result<(), AnyError> {
    let result = run().context(DriverSnafu)?;

    println!(
        "{}",
        serde_json::to_string(&result)
            .context(SerializeOutputSnafu)
            .context(DriverSnafu)?
    );

    Ok(())
}

fn run() -> Result<TestExecutionOutput, DriverError> {
    let test: CompilerTest = serde_json::from_reader(stdin()).unwrap();

    // Run the compiler
    let run_start = Instant::now();
    let compiler_output = Command::new(&test.compile_command[0])
        .args(&test.compile_command[1..])
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
    let run_start = Instant::now();
    let test_output = Command::new(&test.run_command[0])
        .args(&test.run_command[1..])
        .output()
        .context(BinaryInvocationSnafu)?;

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
