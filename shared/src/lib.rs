use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::fmt::Formatter;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

pub mod execute;
pub mod judge;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerTask {
    pub task_id: String,
    pub revision_id: String,
    pub commit_message: String,
    pub team_id: String,
    pub image: String,
    pub build_command: Vec<String>,
    #[serde(serialize_with = "serialize_duration")]
    #[serde(deserialize_with = "deserialize_duration")]
    pub build_timeout: Duration,
    pub tests: Vec<CompilerTest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerTest {
    pub test_id: String,
    #[serde(serialize_with = "serialize_duration")]
    #[serde(deserialize_with = "deserialize_duration")]
    pub timeout: Duration,
    pub compile_command: Vec<String>,
    pub binary_arguments: Vec<String>,
    pub compiler_modifiers: Vec<TestModifier>,
    pub binary_modifiers: Vec<TestModifier>,
    pub provisional_for_category: Option<String>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum CrashSignal {
    SegmentationFault,
    FloatingPointException,
}

impl Display for CrashSignal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SegmentationFault => write!(f, "SegmentationFault"),
            Self::FloatingPointException => write!(f, "FloatingPointException"),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum CompilerFailReason {
    Parsing,
    SemanticAnalysis,
}

impl Display for CompilerFailReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parsing => write!(f, "Parsing"),
            Self::SemanticAnalysis => write!(f, "SemanticAnalysis"),
        }
    }
}

impl CompilerFailReason {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Parsing => "Parsing",
            Self::SemanticAnalysis => "SemanticAnalysis",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Parsing => 42,
            Self::SemanticAnalysis => 7,
        }
    }
}

impl CrashSignal {
    pub fn linux_signal_name(&self) -> &'static str {
        match self {
            Self::SegmentationFault => "SIGSEGV",
            Self::FloatingPointException => "SIGFPE",
        }
    }

    pub fn signal_number(&self) -> i32 {
        match self {
            Self::SegmentationFault => 11,
            Self::FloatingPointException => 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TestModifier {
    ExpectedOutput { output: String },
    ProgramArgument { arg: String },
    ProgramArgumentFile { contents: String },
    ProgramInput { input: String },
    ShouldCrash { signal: CrashSignal },
    ShouldFail { reason: CompilerFailReason },
    ShouldSucceed,
}

impl TestModifier {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ExpectedOutput { .. } => "ExpectedOutput",
            Self::ProgramArgument { .. } => "ProgramArgument",
            Self::ProgramArgumentFile { .. } => "ProgramArgumentFile",
            Self::ProgramInput { .. } => "ProgramInput",
            Self::ShouldCrash { .. } => "ShouldCrash",
            Self::ShouldFail { .. } => "ShouldFail",
            Self::ShouldSucceed => "ShouldSucceed",
        }
    }
}

pub trait TestModifierExt {
    fn full_input(&self) -> String;
    fn full_output(&self) -> Option<String>;
    fn all_arguments(&self) -> Vec<String>;
}

impl<'a, T: Borrow<&'a [TestModifier]>> TestModifierExt for T {
    fn full_input(&self) -> String {
        self.borrow()
            .iter()
            .filter_map(|it| match it {
                TestModifier::ProgramInput { input } => Some(input),
                _ => None,
            })
            .map(|it| it.to_string())
            .collect()
    }

    fn full_output(&self) -> Option<String> {
        let output = self
            .borrow()
            .iter()
            .filter_map(|it| match it {
                TestModifier::ExpectedOutput { output } => Some(output),
                _ => None,
            })
            .map(|it| it.to_string())
            .collect::<Vec<String>>();

        if output.is_empty() {
            None
        } else {
            Some(output.into_iter().collect())
        }
    }

    fn all_arguments(&self) -> Vec<String> {
        self.borrow()
            .iter()
            .filter_map(|it| match it {
                TestModifier::ProgramArgument { arg } => Some(arg),
                _ => None,
            })
            .map(|it| it.to_string())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedExecution {
    pub stdout: String,
    pub stderr: String,
    #[serde(serialize_with = "serialize_duration")]
    #[serde(deserialize_with = "deserialize_duration")]
    pub runtime: Duration,
    pub exit_status: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbortedExecution {
    pub stdout: String,
    pub stderr: String,
    #[serde(serialize_with = "serialize_duration")]
    #[serde(deserialize_with = "deserialize_duration")]
    pub runtime: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalError {
    pub message: String,
    #[serde(serialize_with = "serialize_duration")]
    #[serde(deserialize_with = "deserialize_duration")]
    pub runtime: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExecutionOutput {
    Aborted(AbortedExecution),
    Error(InternalError),
    Success(FinishedExecution),
    Failure(FinishedExecution),
    Timeout(FinishedExecution),
}

impl ExecutionOutput {
    pub fn is_successful(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    pub fn into_finished_execution(self) -> Option<FinishedExecution> {
        match self {
            Self::Failure(finished) => Some(finished),
            Self::Success(finished) => Some(finished),
            Self::Timeout(finished) => Some(finished),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TestExecutionOutput {
    #[serde(rename_all = "camelCase")]
    BinaryFailed {
        compiler_output: ExecutionOutput,
        binary_output: ExecutionOutput,
    },
    #[serde(rename_all = "camelCase")]
    CompilerFailed { compiler_output: ExecutionOutput },
    #[serde(rename_all = "camelCase")]
    Error { output_so_far: ExecutionOutput },
    #[serde(rename_all = "camelCase")]
    Success {
        compiler_output: ExecutionOutput,
        binary_output: Option<ExecutionOutput>,
    },
}

impl TestExecutionOutput {
    pub fn compiler_output(&self) -> &ExecutionOutput {
        match self {
            Self::BinaryFailed {
                compiler_output, ..
            } => compiler_output,
            Self::CompilerFailed {
                compiler_output, ..
            } => compiler_output,
            Self::Error { output_so_far, .. } => output_so_far,
            Self::Success {
                compiler_output, ..
            } => compiler_output,
        }
    }

    pub fn binary_output(&self) -> Option<&ExecutionOutput> {
        match self {
            Self::BinaryFailed { binary_output, .. } => Some(binary_output),
            Self::Success { binary_output, .. } => binary_output.as_ref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Display, Serialize, Deserialize)]
pub enum TestExecutionOutputType {
    CompilerFailed,
    BinaryFailed,
    Success,
    Error,
}

impl TestExecutionOutputType {
    pub fn to_test_execution(
        &self,
        compiler_output: ExecutionOutput,
        binary_output: Option<ExecutionOutput>,
    ) -> TestExecutionOutput {
        match self {
            Self::BinaryFailed => TestExecutionOutput::BinaryFailed {
                compiler_output,
                binary_output: binary_output.expect("Binary output is required for BinaryFailed"),
            },
            Self::CompilerFailed => TestExecutionOutput::CompilerFailed { compiler_output },
            Self::Error => TestExecutionOutput::Error {
                output_so_far: compiler_output,
            },
            Self::Success => TestExecutionOutput::Success {
                compiler_output,
                binary_output,
            },
        }
    }
}

impl From<&TestExecutionOutput> for TestExecutionOutputType {
    fn from(value: &TestExecutionOutput) -> Self {
        match value {
            TestExecutionOutput::BinaryFailed { .. } => Self::BinaryFailed,
            TestExecutionOutput::CompilerFailed { .. } => Self::CompilerFailed,
            TestExecutionOutput::Error { .. } => Self::Error,
            TestExecutionOutput::Success { .. } => Self::Success,
        }
    }
}

impl FromStr for TestExecutionOutputType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CompilerFailed" => Ok(Self::CompilerFailed),
            "BinaryFailed" => Ok(Self::BinaryFailed),
            "Success" => Ok(Self::Success),
            "Error" => Ok(Self::Error),
            _ => Err(format!("Invalid TestExecutionOutputType: `{}`", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedTest {
    pub test_id: String,
    pub output: TestExecutionOutput,
    pub provisional_for_category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedTaskInfo {
    pub task_id: String,

    #[serde(serialize_with = "serialize_system_time")]
    #[serde(deserialize_with = "deserialize_system_time")]
    pub start: SystemTime,
    #[serde(serialize_with = "serialize_system_time")]
    #[serde(deserialize_with = "deserialize_system_time")]
    pub end: SystemTime,

    pub team_id: String,
    pub revision_id: String,
    pub commit_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FinishedCompilerTask {
    #[serde(rename_all = "camelCase")]
    BuildFailed {
        info: FinishedTaskInfo,
        build_output: ExecutionOutput,
    },
    #[serde(rename_all = "camelCase")]
    RanTests {
        info: FinishedTaskInfo,
        build_output: FinishedExecution,
        tests: Vec<FinishedTest>,
    },
}

impl FinishedCompilerTask {
    pub fn info(&self) -> &FinishedTaskInfo {
        match self {
            Self::BuildFailed { info, .. } => info,
            Self::RanTests { info, .. } => info,
        }
    }
}

#[derive(Debug, Clone, Hash, From, PartialEq, Eq, Display, Serialize, Deserialize)]
pub struct RunnerId(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerInfo {
    pub id: RunnerId,
    pub info: String,
    pub current_task: Option<String>,
    pub test_taster: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RunnerUpdate {
    StartedBuild,
    FinishedBuild {
        result: FinishedExecution,
    },
    #[serde(rename_all = "camelCase")]
    StartedTest {
        test_id: String,
    },
    FinishedTest {
        result: FinishedTest,
    },
    Done,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerWorkResponse {
    pub task: Option<CompilerTask>,
    pub reset: bool,
}

#[derive(Debug, Clone, Hash, From, PartialEq, Eq, Display, Serialize, Deserialize)]
pub struct TestTasteId(String);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkTasteTestTask {
    pub id: TestTasteId,
    pub test: CompilerTest,
    pub image_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerWorkTasteTestResponse {
    pub task: Option<WorkTasteTestTask>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerWorkTasteTestDone {
    pub output: TestExecutionOutput,
    pub id: TestTasteId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerRegisterResponse {
    pub reset: bool,
}

pub fn serialize_system_time<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards");
    serializer.serialize_u64(duration.as_millis() as u64)
}

pub fn deserialize_system_time<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let millis = u64::deserialize(deserializer)?;
    Ok(SystemTime::UNIX_EPOCH + Duration::from_millis(millis))
}

pub fn serialize_duration<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(duration.as_millis() as u64)
}

pub fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let millis = u64::deserialize(deserializer)?;
    Ok(Duration::from_millis(millis))
}

pub fn validate_test_id(input: &str) -> Result<(), &'static str> {
    let is_allowed_chars = input
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' || c == '(' || c == ')');

    if !is_allowed_chars {
        return Err("Test id must only contain alphanumerics, dashes, underscores or spaces");
    }
    if input.len() > 300 {
        return Err("Input is longer than 300 chars");
    }

    Ok(())
}
