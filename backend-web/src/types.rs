pub use self::execution::ExecutionExitStatus;
pub use self::execution::Executor;
pub use self::execution::TaskId;
pub use self::execution::WorkItem;
pub use self::repo::Repo;
pub use self::test::Test;
pub use self::test::TestId;
pub use self::user::FullUserForAdmin;
pub use self::user::OwnUser;
pub use self::user::Team;
pub use self::user::TeamId;
pub use self::user::User;
pub use self::user::UserId;
pub use self::user::UserRole;
use crate::auth::Keys;
use crate::config::ExecutionConfig;
use crate::db::Database;
use crate::storage::LocalRepos;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

mod execution;
mod repo;
mod test;
mod user;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtIssuer(pub String);

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub jwt_keys: Keys,
    pub execution_config: ExecutionConfig,
    pub executor: Arc<Mutex<Executor>>,
    pub local_repos: LocalRepos,
}

impl AppState {
    pub fn new(
        db: Database,
        jwt_secret: Keys,
        runner_config: ExecutionConfig,
        local_repos: LocalRepos,
    ) -> Self {
        Self {
            db,
            jwt_keys: jwt_secret,
            execution_config: runner_config,
            executor: Arc::new(Mutex::new(Executor::default())),
            local_repos,
        }
    }
}
