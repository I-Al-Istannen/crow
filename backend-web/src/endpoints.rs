mod admin;
mod executor;
mod oidc;
mod streaming;
mod tasks;
mod team;
mod test;
mod user;

pub use self::admin::rehash_tests;
pub use self::admin::rerun_submissions;
pub use self::admin::snapshot_state;
pub use self::executor::get_test_tasting_work;
pub use self::executor::get_work;
pub use self::executor::get_work_tar;
pub use self::executor::runner_done;
pub use self::executor::runner_ping;
pub use self::executor::runner_register;
pub use self::executor::runner_update;
pub use self::executor::taste_testing_done;
pub use self::oidc::login_oidc;
pub use self::oidc::login_oidc_callback;
pub use self::streaming::get_running_task_info;
pub use self::streaming::head_running_task_info;
pub use self::tasks::executor_info;
pub use self::tasks::get_queue;
pub use self::tasks::get_queued_task;
pub use self::tasks::get_task;
pub use self::tasks::get_top_task_per_team;
pub use self::tasks::integration_get_task_status;
pub use self::tasks::integration_request_revision;
pub use self::tasks::request_revision;
pub use self::team::get_final_tasks;
pub use self::team::get_n_recent_tasks;
pub use self::team::get_recent_tasks;
pub use self::team::get_tasks_for_team;
pub use self::team::get_team_info;
pub use self::team::get_team_repo;
pub use self::team::set_final_task;
pub use self::team::set_team_repo;
pub use self::test::delete_test;
pub use self::test::get_test;
pub use self::test::list_tests;
pub use self::test::set_test;
pub use self::user::get_integration_status;
pub use self::user::list_users;
pub use self::user::show_me_myself;
use crate::error::{HttpError, WebError};
use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{FromRequest, FromRequestParts};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use snafu::location;

// create an extractor that internally uses `axum::Json` but has a custom rejection
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(WebError))]
pub struct Json<T>(T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(WebError))]
pub struct Path<T>(T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}

impl HttpError for JsonRejection {
    fn to_http_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn to_error_code(&self) -> &'static str {
        "invalid_json"
    }
}

impl From<JsonRejection> for WebError {
    fn from(rejection: JsonRejection) -> Self {
        Self::http_error(rejection, location!())
    }
}

impl HttpError for PathRejection {
    fn to_http_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn to_error_code(&self) -> &'static str {
        "invalid_path"
    }
}

impl From<PathRejection> for WebError {
    fn from(value: PathRejection) -> Self {
        Self::http_error(value, location!())
    }
}
