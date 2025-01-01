use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerTask {
    pub task_id: String,
    pub image: String,
    pub build_command: Vec<String>,
    pub build_timeout: Duration,
    pub tests: Vec<CompilerTest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerTest {
    pub test_id: String,
    pub timeout: Duration,
    pub run_command: Vec<String>,
    pub expected_output: String, // TODO: Files?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishedExecution {
    pub stdout: String,
    pub stderr: String,
    pub runtime: Duration,
    pub exit_status: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbortedExecution {
    pub stdout: String,
    pub stderr: String,
    pub runtime: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalError {
    pub message: String,
    pub runtime: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionOutput {
    Aborted(AbortedExecution),
    Error(InternalError),
    Finished(FinishedExecution),
    Timeout(FinishedExecution),
}

impl ExecutionOutput {
    pub fn produced_results(&self) -> bool {
        matches!(
            self,
            ExecutionOutput::Finished(_) | ExecutionOutput::Timeout(_)
        )
    }

    pub fn into_finished_execution(self) -> Option<FinishedExecution> {
        match self {
            ExecutionOutput::Finished(finished) => Some(finished),
            ExecutionOutput::Timeout(finished) => Some(finished),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishedTest {
    pub test_id: String,
    pub output: ExecutionOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishedCompilerTask {
    BuildFailed {
        start: SystemTime,
        build_output: ExecutionOutput,
    },
    RanTests {
        start: SystemTime,
        build_output: FinishedExecution,
        tests: Vec<FinishedTest>,
    },
}

impl FinishedCompilerTask {
    pub fn start_time(&self) -> SystemTime {
        match self {
            FinishedCompilerTask::BuildFailed { start, .. } => *start,
            FinishedCompilerTask::RanTests { start, .. } => *start,
        }
    }
}

#[derive(Debug, Clone, Hash, From, PartialEq, Eq, Display, Serialize, Deserialize)]
pub struct RunnerId(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerInfo {
    pub id: RunnerId,
    pub info: String,
    pub current_task: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunnerWorkResponse {
    pub task: Option<CompilerTask>,
    pub reset: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunnerPingResponse {
    pub reset: bool,
}
