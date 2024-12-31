mod user;

pub use self::user::UserForAuth;
use crate::error::WebError;
use crate::types::{FullUserForAdmin, OwnUser, UserId};
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
}

impl From<sqlx::Error> for WebError {
    fn from(value: sqlx::Error) -> Self {
        warn!(error = ?value, "sqlx query error");
        WebError::InternalServerError
    }
}
