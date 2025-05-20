mod external;
mod queue;
mod repo;
mod task;
mod team;
mod test;
mod user;

pub use self::user::UserForAuth;
use crate::auth::oidc::OidcUser;
use crate::config::{TeamEntry, TestCategory};
use crate::error::{Result, SqlxSnafu, WebError};
use crate::types::{
    CreatedExternalRun, ExternalRunId, ExternalRunStatus, FinalSubmittedTask,
    FinishedCompilerTaskSummary, FullUserForAdmin, OwnUser, Repo, TaskId, Team, TeamId,
    TeamIntegrationToken, Test, TestId, TestSummary, TestWithTasteTesting, User, UserId, UserRole,
    WorkItem,
};
use jiff::Timestamp;
use shared::{indent, FinishedCompilerTask, TestExecutionOutput};
use snafu::{location, Report, ResultExt};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::{query, Pool, Sqlite, SqlitePool};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{info_span, instrument, Instrument};

#[derive(Clone)]
pub struct Database {
    lock: Arc<RwLock<Pool<Sqlite>>>,
    db_path: PathBuf,
}

impl Database {
    pub async fn new(db_path: &Path) -> std::result::Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::default()
                .foreign_keys(true)
                .create_if_missing(true)
                .read_only(false)
                .journal_mode(SqliteJournalMode::Wal)
                .optimize_on_close(true, None)
                .synchronous(SqliteSynchronous::Normal)
                .pragma("temp_store", "memory")
                .pragma("mmap_size", "30000000000")
                .filename(db_path),
        )
        .await?;

        sqlx::migrate!().run(&pool).await?;

        query!("VACUUM")
            .execute(&mut *pool.acquire().await?)
            .await?;

        Ok(Self {
            lock: Arc::new(RwLock::new(pool)),
            db_path: db_path.to_path_buf(),
        })
    }

    async fn write_lock(&self) -> RwLockWriteGuard<'_, Pool<Sqlite>> {
        self.lock.write().instrument(info_span!("w_lock")).await
    }

    async fn read_lock(&self) -> RwLockReadGuard<'_, Pool<Sqlite>> {
        self.lock.read().instrument(info_span!("r_lock")).await
    }

    #[instrument(skip_all)]
    pub async fn get_user_for_login(&self, user_id: &UserId) -> Result<Option<UserForAuth>> {
        let pool = self.read_lock().await;
        user::get_user_for_login(&mut *pool.acquire().await.context(SqlxSnafu)?, user_id).await
    }

    pub async fn get_user(&self, user_id: &UserId) -> Result<OwnUser> {
        let pool = self.read_lock().await;
        user::get_user(&mut *pool.acquire().await.context(SqlxSnafu)?, user_id).await
    }

    pub async fn fetch_users(&self) -> Result<Vec<FullUserForAdmin>> {
        let pool = self.read_lock().await;
        user::fetch_users(&mut *pool.acquire().await.context(SqlxSnafu)?).await
    }

    pub async fn synchronize_oidc_user(
        &self,
        user: OidcUser,
        team: Option<TeamId>,
        role: Option<UserRole>,
    ) -> Result<OwnUser> {
        let pool = self.write_lock().await;
        user::synchronize_oidc_user(
            &mut *pool.acquire().await.context(SqlxSnafu)?,
            user,
            team,
            role,
        )
        .await
    }

    pub async fn set_team_repo(&self, team_id: &TeamId, repo_url: &str) -> Result<Repo> {
        let pool = self.write_lock().await;
        repo::patch_or_create_repo(&*pool, team_id, repo_url).await
    }

    pub async fn get_repo(&self, team_id: &TeamId) -> Result<Repo> {
        let pool = self.read_lock().await;
        repo::get_repo(&mut *pool.acquire().await.context(SqlxSnafu)?, team_id).await
    }

    pub async fn fetch_repo(&self, team_id: &TeamId) -> Result<Option<Repo>> {
        let pool = self.read_lock().await;
        repo::fetch_repo(&mut *pool.acquire().await.context(SqlxSnafu)?, team_id).await
    }

    pub async fn get_repos(&self) -> Result<Vec<Repo>> {
        let pool = self.read_lock().await;
        repo::get_repos(&mut *pool.acquire().await.context(SqlxSnafu)?).await
    }

    pub async fn get_teams(&self) -> Result<Vec<Team>> {
        let pool = self.read_lock().await;
        team::get_teams(&mut *pool.acquire().await.context(SqlxSnafu)?).await
    }

    pub async fn get_team(&self, team_id: &TeamId) -> Result<Team> {
        let pool = self.read_lock().await;
        team::get_team(&mut *pool.acquire().await.context(SqlxSnafu)?, team_id).await
    }

    pub async fn get_team_info(&self, team_id: &TeamId) -> Result<(Team, Vec<User>)> {
        let pool = self.read_lock().await;
        team::get_team_info(&mut *pool.acquire().await.context(SqlxSnafu)?, team_id).await
    }

    pub async fn get_team_integration_token(
        &self,
        team_id: &TeamId,
    ) -> Result<TeamIntegrationToken> {
        let pool = self.read_lock().await;
        team::get_team_integration_token(&mut *pool.acquire().await.context(SqlxSnafu)?, team_id)
            .await
    }

    pub async fn fetch_team_by_integration_token(
        &self,
        token: &TeamIntegrationToken,
    ) -> Result<Option<TeamId>> {
        let pool = self.read_lock().await;
        team::fetch_team_by_integration_token(&mut *pool.acquire().await.context(SqlxSnafu)?, token)
            .await
    }

    pub async fn sync_teams(&self, teams: &[TeamEntry]) -> Result<()> {
        let pool = self.write_lock().await;
        team::sync_teams(&*pool, teams).await
    }

    pub async fn queue_task(&self, task: WorkItem) -> Result<()> {
        let pool = self.write_lock().await;
        queue::queue_task(&mut *pool.acquire().await.context(SqlxSnafu)?, task).await
    }

    pub async fn get_queued_tasks(&self) -> Result<Vec<WorkItem>> {
        let pool = self.read_lock().await;
        queue::get_queued_tasks(&mut *pool.acquire().await.context(SqlxSnafu)?).await
    }

    pub async fn fetch_queued_task(&self, task_id: &TaskId) -> Result<Option<WorkItem>> {
        let pool = self.read_lock().await;
        queue::fetch_queued_task(&mut *pool.acquire().await.context(SqlxSnafu)?, task_id).await
    }

    pub async fn add_finished_task(&self, result: &FinishedCompilerTask) -> Result<()> {
        let pool = self.write_lock().await;
        let mut con = pool.begin().await.context(SqlxSnafu)?;

        let queue_time =
            queue::remove_queued_task(&mut con, &(result.info().task_id.clone().into())).await?;
        let queue_time =
            queue_time.unwrap_or(Timestamp::try_from(result.info().start).expect("valid time"));
        task::add_finished_task(&mut con, result, queue_time).await?;

        con.commit().await.context(SqlxSnafu)?;

        Ok(())
    }

    /// Returns the task as well as any outdated tests in it.
    pub async fn get_task(&self, task_id: &TaskId) -> Result<(FinishedCompilerTask, Vec<TestId>)> {
        let pool = self.read_lock().await;
        task::get_task(&*pool, task_id).await
    }

    pub async fn get_recent_tasks(
        &self,
        team_id: &TeamId,
        count: u32,
    ) -> Result<Vec<FinishedCompilerTaskSummary>> {
        let pool = self.read_lock().await;
        task::get_recent_tasks(&*pool, team_id, count as i64).await
    }

    pub async fn get_top_task_per_team(
        &self,
    ) -> Result<HashMap<TeamId, FinishedCompilerTaskSummary>> {
        let pool = self.read_lock().await;
        task::get_top_task_per_team(&*pool).await
    }

    pub async fn get_final_submitted_task_for_team_and_category(
        &self,
        team_id: &TeamId,
        category: &str,
        meta: &TestCategory,
        respect_finalized: bool,
    ) -> Result<Option<FinalSubmittedTask>> {
        let pool = self.read_lock().await;
        task::get_final_submitted_task(&*pool, team_id, category, meta, respect_finalized).await
    }

    pub async fn set_final_submitted_task(
        &self,
        team_id: &TeamId,
        user_id: &UserId,
        task_id: &TaskId,
        categories: impl Iterator<Item = &str>,
    ) -> Result<()> {
        let pool = self.write_lock().await;
        task::set_final_submitted_task(
            &mut *pool.acquire().await.context(SqlxSnafu)?,
            team_id,
            user_id,
            task_id,
            categories,
        )
        .await
    }

    pub async fn finalize_submission(
        &self,
        team_id: &TeamId,
        task_id: &TaskId,
        category: &str,
    ) -> Result<()> {
        let pool = self.write_lock().await;
        task::finalize_submission(
            &mut *pool.acquire().await.context(SqlxSnafu)?,
            team_id,
            task_id,
            category,
        )
        .await
    }

    pub async fn add_test(
        &self,
        test: Test,
        test_tasting: Option<TestExecutionOutput>,
    ) -> Result<Test> {
        let pool = self.write_lock().await;
        test::add_test(
            &mut *pool.acquire().await.context(SqlxSnafu)?,
            test,
            test_tasting,
        )
        .await
    }

    pub async fn get_test_summaries(&self) -> Result<Vec<TestSummary>> {
        let pool = self.read_lock().await;
        test::get_tests_summaries(&mut *pool.acquire().await.context(SqlxSnafu)?).await
    }

    pub async fn get_tests(&self) -> Result<Vec<Test>> {
        let pool = self.read_lock().await;
        test::get_tests(&mut *pool.acquire().await.context(SqlxSnafu)?).await
    }

    pub async fn fetch_test(&self, test_id: &TestId) -> Result<Option<Test>> {
        let pool = self.read_lock().await;
        test::fetch_test(&mut *pool.acquire().await.context(SqlxSnafu)?, test_id).await
    }

    pub async fn fetch_test_with_tasting(
        &self,
        test_id: &TestId,
    ) -> Result<Option<TestWithTasteTesting>> {
        let pool = self.read_lock().await;
        test::fetch_test_with_tasting(&mut *pool.acquire().await.context(SqlxSnafu)?, test_id).await
    }

    pub async fn delete_test(&self, test_id: &TestId) -> Result<()> {
        let pool = self.write_lock().await;
        test::delete_test(&mut *pool.acquire().await.context(SqlxSnafu)?, test_id).await
    }

    pub async fn add_external_run(&self, run: &CreatedExternalRun) -> Result<()> {
        let pool = self.write_lock().await;
        external::add_external_run(&mut *pool.acquire().await.context(SqlxSnafu)?, run).await
    }

    pub async fn get_external_runs(&self, platform: &str) -> Result<Vec<CreatedExternalRun>> {
        let pool = self.read_lock().await;
        external::get_external_runs(&mut *pool.acquire().await.context(SqlxSnafu)?, platform).await
    }

    pub async fn update_external_run_status(
        &self,
        run_id: &ExternalRunId,
        status: ExternalRunStatus,
    ) -> Result<bool> {
        let pool = self.write_lock().await;
        external::update_external_run_status(
            &mut *pool.acquire().await.context(SqlxSnafu)?,
            run_id,
            status,
        )
        .await
    }

    pub async fn delete_external_run(&self, run_id: &ExternalRunId) -> Result<bool> {
        let pool = self.write_lock().await;
        external::delete_external_run(&mut *pool.acquire().await.context(SqlxSnafu)?, run_id).await
    }

    pub async fn add_external_run_revision_mapping(
        &self,
        task_id: &TaskId,
        revision: &str,
    ) -> Result<()> {
        let pool = self.write_lock().await;
        external::add_external_run_revision_mapping(
            &mut *pool.acquire().await.context(SqlxSnafu)?,
            task_id,
            revision,
        )
        .await
    }

    pub async fn fetch_external_run_revision_mapping(
        &self,
        task_id: &TaskId,
    ) -> Result<Option<String>> {
        let pool = self.read_lock().await;
        external::fetch_external_run_revision_mapping(
            &mut *pool.acquire().await.context(SqlxSnafu)?,
            task_id,
        )
        .await
    }

    pub async fn delete_external_run_revision_mapping(&self, task_id: &TaskId) -> Result<()> {
        let pool = self.write_lock().await;
        external::delete_external_run_revision_mapping(
            &mut *pool.acquire().await.context(SqlxSnafu)?,
            task_id,
        )
        .await
    }

    pub async fn snapshot_db(&self, path: &Path) -> Result<()> {
        // Guard against concurrent writes to the database
        let _ = self.write_lock().await;

        // Thanks sqlx.
        // https://github.com/launchbadge/sqlx/issues/190

        let res = tokio::process::Command::new("sqlite3")
            .arg(&self.db_path)
            .arg(format!(".backup {}", path.display()))
            .output()
            .await
            .map_err(|e| {
                WebError::internal_error(Report::from_error(&e).to_string(), location!())
            })?;

        if !res.status.success() {
            return Err(WebError::internal_error(
                format!(
                    "Failed to snapshot db. Stdout:\n{}\nStderr:\n{}",
                    indent(&String::from_utf8_lossy(&res.stdout), 2),
                    indent(&String::from_utf8_lossy(&res.stderr), 2)
                ),
                location!(),
            ));
        }

        Ok(())
    }
}
