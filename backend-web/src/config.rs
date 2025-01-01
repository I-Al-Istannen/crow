use crate::types::{TeamId, UserId};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub jwt_secret: String,
    pub teams: Vec<TeamEntry>,
}

#[derive(Deserialize)]
pub struct TeamEntry {
    pub id: TeamId,
    pub display_name: String,
    pub members: Vec<UserId>,
}
