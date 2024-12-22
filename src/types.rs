use std::time::Duration;

#[derive(Debug, Clone)]
pub struct CompilerTask {
    pub run_id: String,
    pub image: String,
    pub build_command: Vec<String>,
    pub build_timeout: Duration,
    pub tests: Vec<CompilerTest>,
}

#[derive(Debug, Clone)]
pub struct CompilerTest {
    pub test_id: String,
    pub timeout: Duration,
    pub arguments: Vec<String>,
    // TODO: Files?
}

#[derive(Debug)]
pub struct FinishedExecution {
    pub stdout: String,
    pub stderr: String,
    pub runtime: Duration,
    pub exit_status: Option<i32>,
    pub timeout: bool,
}

#[derive(Debug)]
pub struct InternalError {
    pub message: String,
    pub id: String,
}

#[derive(Debug)]
pub struct FinishedTest {
    pub test_id: String,
    pub output: Result<FinishedExecution, InternalError>,
}

#[derive(Debug)]
pub enum FinishedCompilerTask {
    BuildFailed {
        build_output: Result<FinishedExecution, InternalError>,
    },
    RanTests {
        build_output: FinishedExecution,
        tests: Vec<FinishedTest>,
    },
}
