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
}

fn parse_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    parse_duration::parse(&s).map_err(serde::de::Error::custom)
}
