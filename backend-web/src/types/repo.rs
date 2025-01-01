use crate::types::TeamId;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Repo {
    pub team: TeamId,
    pub url: String,
    pub auto_fetch: bool,
}
