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
    pub test: TestConfig,
    pub oidc: OidcConfig,
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
    pub binary_arguments: Vec<String>,
    pub compile_command: Vec<String>,
    #[serde(deserialize_with = "parse_duration")]
    pub build_timeout: Duration,
    #[serde(deserialize_with = "parse_duration")]
    pub test_timeout: Duration,
    pub build_image: String,
    pub reference_compiler_image: Option<String>,

    pub local_repo_path: PathBuf,
}

impl ExecutionConfig {
    pub fn tasting_disabled(&self) -> bool {
        self.reference_compiler_image.is_none()
    }
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

#[derive(Debug, Clone, Deserialize)]
pub struct TestConfig {
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfig {
    pub client_id: String,
    pub client_secret: String,
    pub issuer_url: String,
    pub redirect_url: String,
    pub scopes: Vec<String>,
}

fn parse_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    parse_duration::parse(&s).map_err(serde::de::Error::custom)
}
