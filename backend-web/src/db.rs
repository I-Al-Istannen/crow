mod queue;
mod repo;
mod task;
mod team;
mod test;
mod user;

pub use self::user::UserForAuth;
use crate::config::TeamEntry;
use crate::error::WebError;
use crate::types::{
    FullUserForAdmin, OwnUser, Repo, TaskId, Team, TeamId, Test, TestId, UserId, WorkItem,
};
use shared::FinishedCompilerTask;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::{query, Pool, Sqlite, SqlitePool};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{info_span, instrument, warn, Instrument};

#[derive(Clone)]
pub struct Database {
    lock: Arc<RwLock<Pool<Sqlite>>>,
}

impl Database {
    pub async fn new(db_path: &Path) -> Result<Self, sqlx::Error> {
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
        })
    }

    async fn write_lock(&self) -> RwLockWriteGuard<Pool<Sqlite>> {
        self.lock.write().instrument(info_span!("w_lock")).await
    }

    async fn read_lock(&self) -> RwLockReadGuard<Pool<Sqlite>> {
        self.lock.read().instrument(info_span!("r_lock")).await
    }

    #[instrument(skip_all)]
    pub async fn get_user_for_login(
        &self,
        user_id: &UserId,
    ) -> Result<Option<UserForAuth>, WebError> {
        let pool = self.read_lock().await;
        user::get_user_for_login(&mut *pool.acquire().await?, user_id).await
    }

    pub async fn get_user(&self, user_id: &UserId) -> Result<OwnUser, WebError> {
        let pool = self.read_lock().await;
        user::get_user(&mut *pool.acquire().await?, user_id).await
    }

    pub async fn fetch_users(&self) -> Result<Vec<FullUserForAdmin>, WebError> {
        let pool = self.read_lock().await;
        user::fetch_users(&mut *pool.acquire().await?).await
    }

    pub async fn add_user(&self, user: &UserForAuth) -> Result<(), WebError> {
        let pool = self.write_lock().await;
        user::add_user(&mut *pool.acquire().await?, user).await
    }

    pub async fn set_team_repo(
        &self,
        team_id: &TeamId,
        repo_url: &str,
        auto_fetch: bool,
    ) -> Result<Repo, WebError> {
        let pool = self.write_lock().await;
        repo::patch_or_create_repo(&*pool, team_id, repo_url, auto_fetch).await
    }

    pub async fn get_repo(&self, team_id: &TeamId) -> Result<Repo, WebError> {
        let pool = self.read_lock().await;
        repo::get_repo(&mut *pool.acquire().await?, team_id).await
    }

    pub async fn get_team(&self, team_id: &TeamId) -> Result<Team, WebError> {
        let pool = self.read_lock().await;
        team::get_team(&mut *pool.acquire().await?, team_id).await
    }

    pub async fn sync_teams(&self, teams: &[TeamEntry]) -> Result<(), WebError> {
        let pool = self.write_lock().await;
        team::sync_teams(&*pool, teams).await
    }

    pub async fn queue_task(&self, task: WorkItem) -> Result<(), WebError> {
        let pool = self.write_lock().await;
        queue::queue_task(&mut *pool.acquire().await?, task).await
    }

    pub async fn get_queued_tasks(&self) -> Result<Vec<WorkItem>, WebError> {
        let pool = self.read_lock().await;
        queue::get_queued_tasks(&mut *pool.acquire().await?).await
    }

    pub async fn add_finished_task(
        &self,
        task_id: &TaskId,
        result: &FinishedCompilerTask,
    ) -> Result<(), WebError> {
        let pool = self.write_lock().await;
        let mut con = pool.begin().await?;

        queue::remove_queued_task(&mut con, task_id).await?;
        task::add_finished_task(&mut con, task_id, result).await?;

        con.commit().await?;

        Ok(())
    }

    pub async fn add_test(&self, test: Test) -> Result<Test, WebError> {
        let pool = self.write_lock().await;
        test::add_test(&mut *pool.acquire().await?, test).await
    }

    pub async fn get_tests(&self) -> Result<Vec<Test>, WebError> {
        let pool = self.read_lock().await;
        test::get_tests(&mut *pool.acquire().await?).await
    }

    pub async fn fetch_test(&self, test_id: &TestId) -> Result<Option<Test>, WebError> {
        let pool = self.read_lock().await;
        test::fetch_test(&mut *pool.acquire().await?, test_id).await
    }
}

impl From<sqlx::Error> for WebError {
    fn from(value: sqlx::Error) -> Self {
        warn!(error = ?value, "sqlx query error");
        WebError::InternalServerError
    }
}
