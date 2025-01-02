use crate::types::{ExecutionExitStatus, TestId};
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
pub enum FinishedCompilerTaskSummary {
    #[serde(rename_all = "camelCase")]
    BuildFailed {
        #[serde(flatten)]
        info: FinishedTaskInfo,
    },
    #[serde(rename_all = "camelCase")]
    RanTests {
        #[serde(flatten)]
        info: FinishedTaskInfo,
        tests: Vec<FinishedTestSummary>,
    },
}

impl From<FinishedCompilerTask> for FinishedCompilerTaskSummary {
    fn from(value: FinishedCompilerTask) -> Self {
        match value {
            FinishedCompilerTask::BuildFailed { info, .. } => Self::BuildFailed { info },
            FinishedCompilerTask::RanTests { info, tests, .. } => Self::RanTests {
                info,
                tests: tests.into_iter().map(Into::into).collect(),
            },
        }
    }
}
