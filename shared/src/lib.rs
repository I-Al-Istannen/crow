use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

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
    pub run_command: Vec<String>,
    pub expected_output: String,
    // TODO: Files?
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
#[serde(rename_all = "camelCase")]
pub struct FinishedTest {
    pub test_id: String,
    pub output: ExecutionOutput,
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
            FinishedCompilerTask::BuildFailed { info, .. } => info,
            FinishedCompilerTask::RanTests { info, .. } => info,
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
