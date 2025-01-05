mod execution;
mod streaming;
mod team;
mod test;
mod user;

use crate::error::WebError;
use axum::extract::rejection::JsonRejection;
use axum::extract::FromRequest;
use axum::response::IntoResponse;
use serde::Serialize;
use std::error::Error;

pub use self::execution::executor_info;
pub use self::execution::get_queue;
pub use self::execution::get_task;
pub use self::execution::get_work;
pub use self::execution::get_work_tar;
pub use self::execution::list_task_ids;
pub use self::execution::request_revision;
pub use self::execution::runner_done;
pub use self::execution::runner_ping;
pub use self::execution::runner_register;
pub use self::execution::runner_update;
pub use self::streaming::get_running_task_info;
pub use self::streaming::head_running_task_info;
pub use self::team::get_recent_tasks;
pub use self::team::get_team_info;
pub use self::team::get_team_repo;
pub use self::team::set_team_repo;
pub use self::test::delete_test;
pub use self::test::get_test;
pub use self::test::list_tests;
pub use self::test::set_test;
pub use self::user::list_users;
pub use self::user::login;
pub use self::user::show_me_myself;

// create an extractor that internally uses `axum::Json` but has a custom rejection
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(WebError))]
pub struct Json<T>(T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        let Self(value) = self;
        axum::Json(value).into_response()
    }
}

impl From<JsonRejection> for WebError {
    fn from(rejection: JsonRejection) -> Self {
        let msg = match rejection {
            JsonRejection::JsonDataError(e) => {
                e.source().map(|e| e.to_string()).unwrap_or(e.body_text())
            }
            any => any.body_text(),
        };
        Self::InvalidJson(msg)
    }
}
