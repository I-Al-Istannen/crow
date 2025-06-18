use crate::types::{ExecutionExitStatus, TaskId, TestId, UserId};
use serde::{Deserialize, Serialize};
use shared::{FinishedCompilerTask, FinishedTaskInfo, FinishedTest};
use std::borrow::Borrow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedTestSummary {
    pub test_id: TestId,
    pub output: ExecutionExitStatus,
    pub provisional_for_category: Option<String>,
    pub category: Option<String>,
}

impl From<FinishedTest> for FinishedTestSummary {
    fn from(value: FinishedTest) -> Self {
        Self {
            test_id: value.test_id.into(),
            output: (&value.output).into(),
            provisional_for_category: value.provisional_for_category,
            category: value.category,
        }
    }
}

impl From<&FinishedTest> for FinishedTestSummary {
    fn from(value: &FinishedTest) -> Self {
        Self {
            test_id: value.test_id.clone().into(),
            output: (&value.output).into(),
            provisional_for_category: value.provisional_for_category.clone(),
            category: value.category.clone(),
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
        outdated: Vec<TestId>,
        statistics: FinishedCompilerTaskStatistics,
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
            FinishedCompilerTask::RanTests { info, tests, .. } => {
                let statistics = tests.as_slice().into();
                Self::RanTests {
                    info,
                    outdated,
                    statistics,
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CountWithProvisional {
    pub normal: usize,
    pub provisional: usize,
    pub total: usize,
}

impl CountWithProvisional {
    pub fn inc(&mut self, provisional: bool) {
        if provisional {
            self.provisional += 1;
        } else {
            self.normal += 1;
        }
        self.total += 1;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedCompilerTaskStatistics {
    abort: CountWithProvisional,
    error: CountWithProvisional,
    failure: CountWithProvisional,
    success: CountWithProvisional,
    timeout: CountWithProvisional,
    total: CountWithProvisional,
}

impl<T: Borrow<FinishedTestSummary>> From<&[T]> for FinishedCompilerTaskStatistics {
    fn from(tests: &[T]) -> Self {
        let mut statistics = Self::default();
        for test in tests {
            let provisional = test.borrow().provisional_for_category.is_some();
            match &test.borrow().output {
                ExecutionExitStatus::Aborted => statistics.abort.inc(provisional),
                ExecutionExitStatus::Error => statistics.error.inc(provisional),
                ExecutionExitStatus::Failure => statistics.failure.inc(provisional),
                ExecutionExitStatus::Success => statistics.success.inc(provisional),
                ExecutionExitStatus::Timeout => statistics.timeout.inc(provisional),
            }
            statistics.total.inc(provisional);
        }

        statistics
    }
}

impl From<&[FinishedTest]> for FinishedCompilerTaskStatistics {
    fn from(tests: &[FinishedTest]) -> Self {
        tests
            .iter()
            .map(FinishedTestSummary::from)
            .collect::<Vec<_>>()
            .as_slice()
            .into()
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
    #[serde(rename_all = "camelCase")]
    Finalized {
        summary: FinishedCompilerTaskSummary,
    },
}

impl FinalSubmittedTask {
    pub fn task_id(&self) -> TaskId {
        self.summary().info().task_id.clone().into()
    }

    pub fn summary(&self) -> &FinishedCompilerTaskSummary {
        match self {
            Self::AutomaticallySelected { summary } => summary,
            Self::ManuallyOverridden { summary, .. } => summary,
            Self::Finalized { summary } => summary,
        }
    }
}
