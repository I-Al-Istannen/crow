use crate::types::TeamId;
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, From, PartialEq, Eq, Display, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct TestId(String);

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSummary {
    pub id: TestId,
    pub creator_id: TeamId,
    pub creator_name: String,
    pub admin_authored: bool,
    pub category: String,
    pub hash: String
}
