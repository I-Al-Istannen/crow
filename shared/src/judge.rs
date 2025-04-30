use crate::{
    CompilerFailReason, CrashSignal, ExecutionOutput, FinishedExecution, TestModifier,
    TestModifierExt,
};
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
    let mut problems = Vec::new();

    let expected_output = (&modifiers).full_output();
    if let Some(expected_output) = expected_output {
        if let Some(problem) = judge_program_output(&execution, expected_output) {
            problems.push(problem);
        }
    }

    for modifier in modifiers {
        let problem = match modifier {
            TestModifier::ShouldCrash { signal } => {
                judge_program_should_crash(exit_status, *signal)
            }
            TestModifier::ShouldSucceed => judge_program_should_succeed(exit_status),
            TestModifier::ShouldFail { reason } => judge_program_should_fail(exit_status, *reason),
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

fn judge_program_should_fail(
    exit_status: ExitStatus,
    expected: CompilerFailReason,
) -> Option<JudgeProblem> {
    let Some(code) = exit_status.code() else {
        return Some(JudgeProblem {
            message: format!(
                "Program should have failed with {} but it exited with an unknown error: {:?}",
                expected.name(),
                exit_status
            ),
            modifier_name: "ShouldFail".to_string(),
        });
    };

    if code == expected.exit_code() {
        return None;
    }

    Some(JudgeProblem {
        message: format!(
            "Program should have failed with `{}`, but it exited with {code}.",
            expected.name()
        ),
        modifier_name: "ShouldFail".to_string(),
    })
}
