use crate::indent;
use std::fmt::{Display, Formatter};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, ExitStatus, Output};

#[derive(Debug, Clone, Copy)]
pub enum CrowExitStatus {
    WithSignal {
        exit_status: ExitStatus,
        signal: i32,
    },
    Original(ExitStatus),
}

impl From<ExitStatus> for CrowExitStatus {
    fn from(value: ExitStatus) -> Self {
        Self::Original(value)
    }
}

impl CrowExitStatus {
    pub fn code(&self) -> Option<i32> {
        match self {
            Self::WithSignal { .. } => None,
            Self::Original(exit_status) => exit_status.code(),
        }
    }

    pub fn success(&self) -> bool {
        self.inner().success()
    }

    pub fn signal(&self) -> Option<i32> {
        match self {
            Self::WithSignal { signal, .. } => Some(*signal),
            Self::Original(status) => status.signal(),
        }
    }

    pub fn inner(&self) -> ExitStatus {
        match self {
            Self::WithSignal { exit_status, .. } => *exit_status,
            Self::Original(exit_status) => *exit_status,
        }
    }
}

impl Display for CrowExitStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WithSignal {
                exit_status,
                signal,
            } => {
                write!(f, "{} (killed by signal {})", exit_status, signal)
            }
            Self::Original(exit_status) => write!(f, "{}", exit_status),
        }
    }
}

pub trait HandleExitcode {
    fn handle_exitcode(self) -> std::io::Result<Output>;
}

impl HandleExitcode for &mut Command {
    fn handle_exitcode(self) -> std::io::Result<Output> {
        let output = self.output()?;

        if output.status.success() {
            return Ok(output);
        }
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        let mut response = "".to_string();
        if let Some(code) = output.status.code() {
            response.push_str(&format!("Exited with code {code}\n"));
        }
        if !stdout.trim().is_empty() {
            response.push_str(&format!("stdout:\n{}\n", indent(stdout.trim(), 2)));
        }
        if !stderr.trim().is_empty() {
            response.push_str(&format!("stderr:\n{}", indent(stderr.trim(), 2)));
        }

        Err(std::io::Error::new(std::io::ErrorKind::Other, response))
    }
}
