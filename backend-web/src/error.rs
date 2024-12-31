use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum WebError {
    NoPermissions,
    InvalidCredentials,
    InvalidJson(String),
    InternalServerError,
    NotFound,
}

impl WebError {
    fn to_code(&self) -> (&'static str, StatusCode) {
        match self {
            Self::NoPermissions => ("no_permissions", StatusCode::FORBIDDEN),
            Self::InvalidCredentials => ("invalid_credentials", StatusCode::UNAUTHORIZED),
            Self::InvalidJson(_) => ("invalid_json", StatusCode::BAD_REQUEST),
            Self::InternalServerError => ("internal_error", StatusCode::INTERNAL_SERVER_ERROR),
            Self::NotFound => ("not_found", StatusCode::NOT_FOUND),
        }
    }
}

impl Display for WebError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::NoPermissions => write!(f, "No permissions"),
            Self::InvalidCredentials => write!(f, "Invalid credentials"),
            Self::InvalidJson(msg) => write!(f, "Invalid JSON: `{}`", msg),
            Self::InternalServerError => write!(f, "Internal server error"),
            Self::NotFound => write!(f, "Not found"),
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (code, status) = self.to_code();
        (
            status,
            Json(json!({
                "error": self.to_string(),
                "code": code,
            })),
        )
            .into_response()
    }
}
