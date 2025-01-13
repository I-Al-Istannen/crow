use crate::types::TeamId;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Repo {
    pub team: TeamId,
    pub url: String,
}
