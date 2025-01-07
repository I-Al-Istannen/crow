use crate::types::TaskId;
use derive_more::{Deref, Display, From};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    From,
    Deref,
    Serialize,
    Deserialize,
    sqlx::Type,
    Display,
)]
#[sqlx(transparent)]
pub struct ExternalRunId(u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum ExternalRunStatus {
    Queued,
    Running,
    Finished,
}

#[derive(Debug, Clone)]
pub struct CreatedExternalRun {
    pub task_id: TaskId,
    pub run_id: ExternalRunId,
    pub platform: String,
    pub repo: String,
    pub owner: String,
    pub revision: String,
    pub status: ExternalRunStatus,
}
