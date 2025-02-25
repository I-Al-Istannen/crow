use shared::{CompilerTest, ExecutionOutput, FinishedExecution};
use similar::{DiffableStr, TextDiff};
use std::process::ExitStatus;

pub fn judge_output(
    test: &CompilerTest,
    exit_status: ExitStatus,
    execution: FinishedExecution,
) -> ExecutionOutput {
    if !exit_status.success() {
        return ExecutionOutput::Failure(execution);
    }

    let mut expected_output = test.expected_output.clone();
    let mut actual_output = execution.stdout.clone();

    // Normalize newlines for diff. This helps users understand it better, many people are not
    // well versed in that distinction.
    if !expected_output.ends_with_newline() {
        expected_output.push('\n');
    }
    if !actual_output.ends_with_newline() {
        actual_output.push('\n');
    }

    if expected_output == *actual_output {
        return ExecutionOutput::Success(execution);
    }

    let mut final_stderr = execution.stderr;

    if !final_stderr.is_empty() {
        final_stderr += "\n\n";
    }
    final_stderr += "A diff of your result follows. ";
    final_stderr += "You can always compute it yourself by copying the stdout.\n";

    let diff = TextDiff::from_lines(&expected_output, &actual_output);
    let mut diff = diff.unified_diff();
    let diff = diff
        .context_radius(5)
        .header("missing from yours", "extraneous in yours");

    final_stderr += &diff.to_string();

    ExecutionOutput::Failure(FinishedExecution {
        stdout: execution.stdout,
        stderr: final_stderr,
        runtime: execution.runtime,
        exit_status: execution.exit_status,
    })
}
