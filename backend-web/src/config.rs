use std::path::PathBuf;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub jwt_secret: String,
}
