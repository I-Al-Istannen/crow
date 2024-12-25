use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    // TODO: Files?
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
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionOutput {
    Finished(FinishedExecution),
    Timeout(FinishedExecution),
    Error(InternalError),
    Aborted(AbortedExecution),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishedTest {
    pub test_id: String,
    pub output: ExecutionOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishedCompilerTask {
    BuildFailed {
        build_output: ExecutionOutput,
    },
    RanTests {
        build_output: FinishedExecution,
        tests: Vec<FinishedTest>,
    },
}
