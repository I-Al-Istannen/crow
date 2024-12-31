use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Hash, From, PartialEq, Eq, Display, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct TeamId(String);

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: TeamId,
    pub display_name: String,
}

#[derive(Debug, Clone, Hash, From, PartialEq, Eq, Display, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(String);

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub team: Option<TeamId>,
    pub id: UserId,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OwnUser {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub user: User,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FullUserForAdmin {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub user: OwnUser,
    pub role: UserRole,
}

impl FullUserForAdmin {
    pub fn into_user(self) -> User {
        self.user.user
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, sqlx::Type)]
pub enum UserRole {
    Regular,
    Admin,
}
