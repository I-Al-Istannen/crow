use crate::{CrashSignal, ExecutionOutput, FinishedExecution, TestModifier, TestModifierExt};
use similar::{DiffableStr, TextDiff};
use std::fmt::{Display, Formatter};
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

struct JudgeProblem {
    message: String,
    modifier_name: String,
}

impl Display for JudgeProblem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "==== {} ====", self.modifier_name,)?;
        write!(f, "{}", self.message.trim_end())
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
            TestModifier::ShouldCrash { signal } => {
                judge_program_should_crash(exit_status, *signal)
            }
            TestModifier::ShouldSucceed => judge_program_should_succeed(exit_status),
            TestModifier::ExpectedOutput { .. } => None,
            TestModifier::ProgramArgument { .. } => None,
            TestModifier::ProgramArgumentFile { .. } => None,
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
        .join("\n\n");

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
        modifier_name: "ExpectedOutput".to_string(),
    })
}

fn judge_program_exit_status(exit_status: ExitStatus, expected_code: u32) -> Option<JudgeProblem> {
    match exit_status.code() {
        None => Some(JudgeProblem {
            message: "The program had no exit status".to_string(),
            modifier_name: "ExitCode".to_string(),
        }),
        Some(val) => {
            if val != expected_code as i32 {
                Some(JudgeProblem {
                    message: format!("Program exited with {val}, expected was {expected_code}."),
                    modifier_name: "ExitCode".to_string(),
                })
            } else {
                None
            }
        }
    }
}

fn judge_program_should_crash(
    exit_status: ExitStatus,
    signal: CrashSignal,
) -> Option<JudgeProblem> {
    if let Some(signal_num) = exit_status.signal() {
        if signal_num == signal.signal_number() {
            None
        } else {
            Some(JudgeProblem {
                message: format!(
                    "Program should crash with signal `{}` ({}), but was killed by signal {signal_num}.",
                    signal.linux_signal_name(),
                    signal.signal_number()
                ),
                modifier_name: "ShouldCrash".to_string(),
            })
        }
    } else {
        let exit_code = exit_status
            .code()
            .map(|it| format!(" It exited with code `{it}`."))
            .unwrap_or("".to_string());
        Some(JudgeProblem {
            message: format!(
                "Program should have crashed with signal `{}` ({}), but wasn't killed by a signal at all.{exit_code}",
                signal.linux_signal_name(),
                signal.signal_number()
            ),
            modifier_name: "ShouldCrash".to_string(),
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
            modifier_name: "ShouldSucceed".to_string(),
        })
    } else {
        Some(JudgeProblem {
            message: format!(
                "Program should have exited with success, exited with an unknown error: {exit_status:?}"
            ),
            modifier_name: "ShouldSucceed".to_string()
        })
    }
}
