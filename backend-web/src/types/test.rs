use crate::types::TeamId;
use derive_more::{Display, From};
use jiff::Timestamp;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use shared::{TestExecutionOutput, TestModifier, validate_test_id};

#[derive(Debug, Clone, Hash, From, PartialEq, Eq, Display, Serialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct TestId(String);

impl<'de> Deserialize<'de> for TestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let test_id: String = Deserialize::deserialize(deserializer)?;
        if let Err(e) = validate_test_id(&test_id) {
            return Err(Error::custom(e));
        }
        Ok(Self(test_id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TestTastingResult {
    Success,
    Failure { output: TestExecutionOutput },
}

impl From<TestExecutionOutput> for TestTastingResult {
    fn from(output: TestExecutionOutput) -> Self {
        if matches!(output, TestExecutionOutput::Success { .. }) {
            return Self::Success;
        }
        Self::Failure { output }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Test {
    pub id: TestId,
    pub compiler_modifiers: Vec<TestModifier>,
    pub binary_modifiers: Vec<TestModifier>,
    pub owner: TeamId,
    pub admin_authored: bool,
    pub category: String,
    pub provisional_for_category: Option<String>,
    /// This test is not applicable to later categories and should not be run
    pub limited_to_category: bool,
    #[serde(serialize_with = "jiff::fmt::serde::timestamp::millisecond::required::serialize")]
    #[serde(deserialize_with = "jiff::fmt::serde::timestamp::millisecond::required::deserialize")]
    pub last_updated: Timestamp,
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
    pub provisional_for_category: Option<String>,
    pub limited_to_category: bool,
    #[serde(serialize_with = "jiff::fmt::serde::timestamp::millisecond::required::serialize")]
    #[serde(deserialize_with = "jiff::fmt::serde::timestamp::millisecond::required::deserialize")]
    pub last_updated: Timestamp,
}
