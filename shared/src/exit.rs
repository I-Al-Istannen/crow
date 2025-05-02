use std::fmt::{Display, Formatter};
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

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
