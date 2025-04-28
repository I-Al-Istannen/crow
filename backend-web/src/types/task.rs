use crate::types::{ExecutionExitStatus, TaskId, TestId, UserId};
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
        outdated: Vec<TestId>,
    },
}

impl FinishedCompilerTaskSummary {
    pub fn info(&self) -> &FinishedTaskInfo {
        match self {
            Self::BuildFailed { info, .. } => info,
            Self::RanTests { info, .. } => info,
        }
    }
}

impl From<(FinishedCompilerTask, Vec<TestId>)> for FinishedCompilerTaskSummary {
    fn from((value, outdated): (FinishedCompilerTask, Vec<TestId>)) -> Self {
        match value {
            FinishedCompilerTask::BuildFailed { info, build_output } => Self::BuildFailed {
                info,
                status: (&build_output).into(),
            },
            FinishedCompilerTask::RanTests { info, tests, .. } => Self::RanTests {
                info,
                tests: tests.into_iter().map(Into::into).collect(),
                outdated,
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

impl FinalSubmittedTask {
    pub fn task_id(&self) -> TaskId {
        match self {
            Self::AutomaticallySelected { summary } => summary.info().task_id.clone().into(),
            Self::ManuallyOverridden { summary, .. } => summary.info().task_id.clone().into(),
        }
    }
}
