use crate::{ExecutionOutput, FinishedExecution, TestModifier, TestModifierExt};
use similar::{DiffableStr, TextDiff};
use std::fmt::{Display, Formatter};
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

struct JudgeProblem {
    message: String,
}

impl Display for JudgeProblem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn judge_output(
    modifiers: &[TestModifier],
    exit_status: ExitStatus,
    execution: FinishedExecution,
) -> ExecutionOutput {
    if !exit_status.success() {
        return ExecutionOutput::Failure(execution);
    }

    let mut problems = Vec::new();

    let expected_output = (&modifiers).full_output();
    if let Some(expected_output) = expected_output {
        if let Some(problem) = judge_program_output(&execution, expected_output) {
            problems.push(problem);
        }
    }

    for modifier in modifiers {
        let problem = match modifier {
            TestModifier::ExitCode { code } => judge_program_exit_status(exit_status, *code),
            TestModifier::ShouldCrash => judge_program_should_crash(exit_status),
            TestModifier::ShouldSucceed => judge_program_should_succeed(exit_status),
            TestModifier::ExpectedOutput { .. } => None,
            TestModifier::ProgramArgument { .. } => None,
            TestModifier::ProgramInput { .. } => None,
        };
        if let Some(problem) = problem {
            problems.push(problem);
        }
    }

    if problems.is_empty() {
        return ExecutionOutput::Success(FinishedExecution {
            stdout: execution.stdout,
            stderr: execution.stderr,
            exit_status: execution.exit_status,
            runtime: execution.runtime,
        });
    }
    let mut final_stderr = execution.stderr;
    if !final_stderr.is_empty() {
        final_stderr += "\n\n"
    }
    final_stderr += &problems
        .into_iter()
        .map(|it| it.to_string())
        .collect::<Vec<String>>()
        .join("\n");

    ExecutionOutput::Failure(FinishedExecution {
        stdout: execution.stdout,
        stderr: final_stderr,
        runtime: execution.runtime,
        exit_status: execution.exit_status,
    })
}

fn judge_program_output(
    execution: &FinishedExecution,
    mut expected_output: String,
) -> Option<JudgeProblem> {
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
        return None;
    }

    let mut stderr_result = String::new();

    if !stderr_result.is_empty() {
        stderr_result += "\n\n";
    }
    stderr_result += "A diff of your result follows. ";
    stderr_result += "You can always compute it yourself by copying the stdout.\n";

    let diff = TextDiff::from_lines(&expected_output, &actual_output);
    let mut diff = diff.unified_diff();
    let diff = diff
        .context_radius(5)
        .header("missing from yours", "extraneous in yours");

    stderr_result += &diff.to_string();

    Some(JudgeProblem {
        message: stderr_result,
    })
}

fn judge_program_exit_status(exit_status: ExitStatus, expected_code: u32) -> Option<JudgeProblem> {
    match exit_status.code() {
        None => Some(JudgeProblem {
            message: "The program had no exit status".to_string(),
        }),
        Some(val) => {
            if val != expected_code as i32 {
                Some(JudgeProblem {
                    message: format!("Program exited with {val}, expected was {expected_code}."),
                })
            } else {
                None
            }
        }
    }
}

fn judge_program_should_crash(exit_status: ExitStatus) -> Option<JudgeProblem> {
    if let Some(signal_num) = exit_status.signal() {
        if signal_num == 6 {
            None
        } else {
            Some(JudgeProblem {
                message: format!(
                    "Program should crash with signal 6, but was killed by signal {signal_num}."
                ),
            })
        }
    } else {
        Some(JudgeProblem {
            message:
                "Program should have crashed with a signal, but wasn't killed by a signal at all :/"
                    .to_string(),
        })
    }
}

fn judge_program_should_succeed(exit_status: ExitStatus) -> Option<JudgeProblem> {
    if exit_status.success() {
        return None;
    }
    if let Some(code) = exit_status.code() {
        Some(JudgeProblem {
            message: format!("Program should have exited with success, exited with {code}."),
        })
    } else {
        Some(JudgeProblem{message: format!("Program should have exited with success, exited with an unknown error: {exit_status:?}")})
    }
}
