use crate::auth::Keys;
use crate::db::Database;
use serde::{Deserialize, Serialize};

pub use self::user::User;
pub use self::user::UserId;
pub use self::user::UserRole;
pub use self::user::OwnUser;
pub use self::user::FullUserForAdmin;

mod user;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtIssuer(pub String);

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub jwt_keys: Keys,
}

impl AppState {
    pub fn new(db: Database, jwt_secret: Keys) -> Self {
        Self {
            db,
            jwt_keys: jwt_secret,
        }
    }
}
