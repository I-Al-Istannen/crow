use crate::types::TeamId;
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use shared::ExecutionOutput;

#[derive(Debug, Clone, Hash, From, PartialEq, Eq, Display, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct TestId(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TestTastingResult {
    Success,
    Failure { output: ExecutionOutput },
}

impl From<ExecutionOutput> for TestTastingResult {
    fn from(output: ExecutionOutput) -> Self {
        if matches!(output, ExecutionOutput::Success(_)) {
            return Self::Success;
        }
        Self::Failure { output }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Test {
    pub id: TestId,
    pub expected_output: String,
    pub input: String,
    pub owner: TeamId,
    pub admin_authored: bool,
    pub category: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestWithTasteTesting {
    #[serde(flatten)]
    pub test: Test,
    pub test_tasting_result: Option<TestTastingResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSummary {
    pub id: TestId,
    pub creator_id: TeamId,
    pub creator_name: String,
    pub admin_authored: bool,
    pub category: String,
    pub hash: String,
    pub test_taste_success: Option<bool>,
}
