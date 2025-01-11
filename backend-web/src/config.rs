use crate::types::{TeamId, UserId};
use serde::{Deserialize, Deserializer};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub jwt_secret: String,
    pub teams: Vec<TeamEntry>,
    pub execution: ExecutionConfig,
    pub github: Option<GithubConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TeamEntry {
    pub id: TeamId,
    pub display_name: String,
    pub members: Vec<UserId>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionConfig {
    pub runner_token: String,
    pub build_command: Vec<String>,
    pub test_command: Vec<String>,
    #[serde(deserialize_with = "parse_duration")]
    pub build_timeout: Duration,
    #[serde(deserialize_with = "parse_duration")]
    pub test_timeout: Duration,

    pub local_repo_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GithubConfig {
    pub app_id: u64,
    pub app_name: String,
    pub app_private_key: String,
    #[serde(deserialize_with = "parse_duration")]
    pub status_check_interval: Duration,
    #[serde(deserialize_with = "parse_duration")]
    pub workflow_check_interval: Duration,
    pub workflow_path: String,
    pub workflow_template: String,
}

fn parse_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    parse_duration::parse(&s).map_err(serde::de::Error::custom)
}
