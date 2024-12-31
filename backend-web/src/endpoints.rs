mod users;

use std::error::Error;
use axum::extract::FromRequest;
use axum::extract::rejection::JsonRejection;
use axum::response::IntoResponse;
use serde::Serialize;
use crate::error::WebError;

pub use self::users::show_me_myself;
pub use self::users::list_users;
pub use self::users::login;

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

// We implement `From<JsonRejection> for ApiError`
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
