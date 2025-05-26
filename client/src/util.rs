use console::style;
use shared::execute::{run_with_timeout, CommandResult, RunWithTimeoutError};
use shared::exit::CrowExitStatus;
use shared::{indent, ExecutionOutput, FinishedExecution, TestExecutionOutput};
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind};
use tracing::{error, info};

#[derive(Debug, Clone, Default)]
pub struct StyledText(String);

impl Display for StyledText {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StyledText {
    pub fn append<T: Display>(mut self, text: T) -> Self {
        self.0.push_str(&text.to_string());
        self
    }
}

pub fn st<T: Display>(start: T) -> StyledText {
    StyledText(start.to_string())
}

pub fn color_diff<T: Display>(diff: T) -> String {
    let diff = diff
        .to_string()
        .lines()
        .map(|line| {
            if line.starts_with("-") {
                format!("{}", style(line).red().bright())
            } else if line.starts_with("+") {
                format!("{}", style(line).green().bright())
            } else if line.starts_with("@") {
                format!("{}", style(line).magenta())
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>();

    diff.join("\n")
}

pub fn infer_test_metadata_from_path(path: &Path) -> Result<(String, String), String> {
    let path = path
        .canonicalize()
        .map_err(|_| format!("Could not canonicalize {}", path.display()))?;

    let name = path
        .file_name()
        .ok_or("Path has no filename".to_string())?
        .to_str()
        .ok_or("File name is no valid string".to_string())?;
    let name = name
        .strip_suffix(".crow-test.md")
        .ok_or("File has no `.crow-test.md` suffix".to_string())?
        .to_string();

    let category = path
        .parent()
        .ok_or("File has no parent directory".to_string())?;
    let category = category
        .file_name()
        .ok_or("Parent folder has no file name".to_string())?
        .to_str()
        .ok_or("Parent folder's name is not valid unicode".to_string())?
        .to_string();

    Ok((category, name))
}

pub fn print_test_output(output: &TestExecutionOutput) {
    match output {
        TestExecutionOutput::BinaryFailed {
            compiler_output,
            binary_output,
        } => {
            error!(
                "{}",
                st(style("Your compiler succeeded, but your binary failed\n")
                    .bright()
                    .red())
                .append(style("Compiler output:\n").bold())
                .append(indent(&execution_output_to_string(compiler_output), 2))
                .append(style("Binary output:\n").bold())
                .append(indent(&execution_output_to_string(binary_output), 2))
            );
        }
        TestExecutionOutput::CompilerFailed { compiler_output } => {
            error!(
                "{}",
                st(style("Your compiler failed\n").bright().red())
                    .append(style("Compiler output:\n").bold())
                    .append(indent(&execution_output_to_string(compiler_output), 2))
            );
        }
        TestExecutionOutput::Error { output_so_far } => {
            error!(
                "{}",
                st(style("An unspecified error occurred\n").bright().red())
                    .append(style("Full output:\n").bold())
                    .append(indent(&execution_output_to_string(output_so_far), 2))
            );
        }
        TestExecutionOutput::Success { .. } => {
            info!("{}", style("Test passed!").bright().bold().green());
        }
    }
}

fn execution_output_to_string(output: &ExecutionOutput) -> String {
    match output {
        ExecutionOutput::Aborted(e) => {
            format!(
                "{}\n",
                st("Execution was ")
                    .append(style("aborted").red().bright())
                    .append(" after ")
                    .append(e.runtime.as_secs().to_string())
                    .append("s")
                    .append(style("\nStdout:\n").bold())
                    .append(indent(e.stdout.trim(), 1))
                    .append(style("\nStderr:\n").bold())
                    .append(indent(e.stderr.trim(), 1))
            )
        }
        ExecutionOutput::Error(e) => {
            format!(
                "{}\n",
                st("Execution encountered ")
                    .append(style("an internal error").red().bright())
                    .append(" after ")
                    .append(e.runtime.as_secs().to_string())
                    .append("s")
                    .append(style("\nMessage:\n").bold())
                    .append(indent(&e.message, 1))
            )
        }
        ExecutionOutput::Success(s) => {
            format!(
                "{}\n",
                st("Execution was ")
                    .append(style("successful").green().bright())
                    .append(" after ")
                    .append(s.runtime.as_secs().to_string())
                    .append("s")
                    .append(style("\nStdout:\n").bold())
                    .append(indent(s.stdout.trim(), 1))
                    .append(style("\nStderr:\n").bold())
                    .append(indent(s.stderr.trim(), 1))
            )
        }
        ExecutionOutput::Failure {
            execution,
            accumulated_errors,
        } => {
            format!(
                "{}\n",
                st("Execution was ")
                    .append(style("unsuccessful").red().bright())
                    .append(" after ")
                    .append(execution.runtime.as_secs().to_string())
                    .append("s")
                    .append(style("\nErrors:\n").bold())
                    .append(indent(
                        accumulated_errors
                            .as_ref()
                            .unwrap_or(&"No specific errors provided".to_string()),
                        1,
                    ))
                    .append(style("\nStdout:\n").bold())
                    .append(indent(execution.stdout.trim(), 1))
                    .append(style("\nStderr:\n").bold())
                    .append(indent(&color_diff(execution.stderr.trim()), 1))
            )
        }
        ExecutionOutput::Timeout(e) => {
            format!(
                "{}\n",
                st("Execution timed out")
                    .append(style("timed out ").red().bright())
                    .append("after ")
                    .append(e.runtime.as_secs().to_string())
                    .append("s")
                    .append(style("\nStdout:\n").bold())
                    .append(indent(e.stdout.trim(), 1))
                    .append(style("\nStderr:\n").bold())
                    .append(indent(e.stderr.trim(), 1))
            )
        }
    }
}

pub fn execute_locally(
    path: &Path,
    cmd: &[String],
    timeout: Option<Duration>,
) -> Result<CommandResult, Box<dyn Error + Sync + Send>> {
    let mut child = Command::new(path)
        .args(cmd)
        .process_group(0)
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
            kill_tree(Pid::from_u32(child.id()));
            (stdout, stderr, CrowExitStatus::Timeout, runtime)
        }
        Err(e) => {
            // Make sure it is dead on error
            kill_tree(Pid::from_u32(child.id()));
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

fn kill_tree(root: Pid) {
    let system = sysinfo::System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_tasks()),
    );

    let mut pids_in_tree = HashSet::new();
    pids_in_tree.insert(root);
    while find_descendants(&system, &mut pids_in_tree) {}

    for pid in pids_in_tree {
        if let Some(process) = system.process(pid) {
            process.kill();
        }
    }
}

fn find_descendants(system: &sysinfo::System, pids_in_tree: &mut HashSet<Pid>) -> bool {
    let mut modified = false;

    for (pid, process) in system.processes() {
        if process.thread_kind().is_some() {
            continue; // Skip threads
        }
        if let Some(ppid) = process.parent() {
            if pids_in_tree.contains(&ppid) {
                modified |= pids_in_tree.insert(*pid);
            }
        }
    }

    modified
}
