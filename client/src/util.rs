use console::style;
use shared::{ExecutionOutput, TestExecutionOutput};
use std::fmt::{Display, Formatter};
use std::path::Path;
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
    let name = path
        .file_name()
        .ok_or("Path has no filename".to_string())?
        .to_str()
        .ok_or("File name is no valid string".to_string())?;
    let name = name
        .strip_suffix(".crow-test")
        .ok_or("File has no `.crow-test` suffix".to_string())?
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

fn indent(string: &str, count: usize) -> String {
    let indented = string
        .trim()
        .lines()
        .map(|it| format!("{:indent$}{}", "", it, indent = count))
        .collect::<Vec<_>>()
        .join("\n");

    if string.ends_with("\n") {
        format!("{}\n", indented)
    } else {
        indented
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
        ExecutionOutput::Failure(e) => {
            format!(
                "{}\n",
                st("Execution was ")
                    .append(style("unsuccessful").red().bright())
                    .append(" after ")
                    .append(e.runtime.as_secs().to_string())
                    .append("s")
                    .append(style("\nStdout:\n").bold())
                    .append(indent(e.stdout.trim(), 1))
                    .append(style("\nStderr:\n").bold())
                    .append(indent(&color_diff(e.stderr.trim()), 1))
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
