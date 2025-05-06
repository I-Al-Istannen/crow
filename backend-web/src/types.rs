pub use self::execution::ExecutionExitStatus;
pub use self::execution::Executor;
pub use self::execution::ExecutorInfo;
pub use self::execution::QueuedTaskStatus;
pub use self::execution::RunnerForFrontend;
pub use self::execution::RunningTaskState;
pub use self::execution::TaskId;
pub use self::execution::WorkItem;
pub use self::external::CreatedExternalRun;
pub use self::external::ExternalRunId;
pub use self::external::ExternalRunStatus;
pub use self::repo::Repo;
pub use self::task::FinalSubmittedTask;
pub use self::task::FinishedCompilerTaskSummary;
pub use self::test::Test;
pub use self::test::TestId;
pub use self::test::TestSummary;
pub use self::test::TestWithTasteTesting;
pub use self::test_tasting::TestTasting;
pub use self::user::FullUserForAdmin;
pub use self::user::OwnUser;
pub use self::user::Team;
pub use self::user::TeamId;
pub use self::user::TeamInfo;
pub use self::user::TeamIntegrationToken;
pub use self::user::User;
pub use self::user::UserId;
pub use self::user::UserRole;
use crate::auth::oidc::Oidc;
use crate::auth::Keys;
use crate::config::{ExecutionConfig, TestConfig};
use crate::db::Database;
use crate::storage::LocalRepos;
use crate::types::queue::Queue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod execution;
mod external;
mod queue;
mod repo;
mod task;
mod test;
mod test_tasting;
mod user;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtIssuer(pub String);

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub jwt_keys: Keys,
    pub execution_config: ExecutionConfig,
    pub test_config: TestConfig,
    pub team_mapping: HashMap<UserId, (TeamId, UserRole)>,
    pub executor: Arc<Mutex<Executor>>,
    pub test_tasting: Arc<Mutex<TestTasting>>,
    pub queue: Arc<Mutex<Queue>>,
    pub local_repos: LocalRepos,
    pub github_app_name: Option<String>,
    pub oidc: Oidc,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: Database,
        jwt_secret: Keys,
        github_app_name: Option<String>,
        execution_config: ExecutionConfig,
        test_config: TestConfig,
        team_mapping: HashMap<UserId, (TeamId, UserRole)>,
        local_repos: LocalRepos,
        oidc: Oidc,
    ) -> Self {
        Self {
            db,
            jwt_keys: jwt_secret,
            execution_config,
            test_config,
            team_mapping,
            executor: Executor::new(),
            test_tasting: TestTasting::new(),
            queue: Arc::new(Mutex::new(Queue::new())),
            local_repos,
            github_app_name,
            oidc,
        }
    }
}
