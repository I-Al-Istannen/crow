use crate::types::{ExecutionExitStatus, TestId, UserId};
use serde::{Deserialize, Serialize};
use shared::{FinishedCompilerTask, FinishedTaskInfo, FinishedTest};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedTestSummary {
    pub test_id: TestId,
    pub output: ExecutionExitStatus,
}

impl From<FinishedTest> for FinishedTestSummary {
    fn from(value: FinishedTest) -> Self {
        Self {
            test_id: value.test_id.into(),
            output: (&value.output).into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FinishedCompilerTaskSummary {
    #[serde(rename_all = "camelCase")]
    BuildFailed {
        info: FinishedTaskInfo,
        status: ExecutionExitStatus,
    },
    #[serde(rename_all = "camelCase")]
    RanTests {
        info: FinishedTaskInfo,
        tests: Vec<FinishedTestSummary>,
    },
}

impl From<FinishedCompilerTask> for FinishedCompilerTaskSummary {
    fn from(value: FinishedCompilerTask) -> Self {
        match value {
            FinishedCompilerTask::BuildFailed { info, build_output } => Self::BuildFailed {
                info,
                status: (&build_output).into(),
            },
            FinishedCompilerTask::RanTests { info, tests, .. } => Self::RanTests {
                info,
                tests: tests.into_iter().map(Into::into).collect(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum FinalSubmittedTask {
    #[serde(rename_all = "camelCase")]
    AutomaticallySelected {
        summary: FinishedCompilerTaskSummary,
    },
    #[serde(rename_all = "camelCase")]
    ManuallyOverridden {
        summary: FinishedCompilerTaskSummary,
        user_id: UserId,
        time: i64,
    },
}
