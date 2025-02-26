use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use snafu::{Location, Report, Snafu};
use std::fmt::Debug;
use tracing::warn;

pub type Result<T> = std::result::Result<T, WebError>;

pub trait HttpError: Debug + snafu::AsErrorSource + Send + Sync {
    fn to_http_code(&self) -> StatusCode;

    fn to_error_code(&self) -> &'static str;

    fn to_extra(&self) -> Option<Value> {
        None
    }

    fn into_weberror(self, location: Location) -> WebError
    where
        Self: Sized + 'static,
    {
        WebError::http_error(self, location)
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum WebError {
    #[snafu(display("No permissions to access this resource at {location}"))]
    Unauthorized {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid credentials at {location}"))]
    InvalidCredentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Not found at {location}"))]
    NotFound {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{what} not found at {location}"))]
    NamedNotFound {
        what: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("User not in a team at {location}"))]
    NotInTeam {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Internal error `{message}` at {location}"))]
    InternalError {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Caused by a sql error at {location}"))]
    Sqlx {
        source: sqlx::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Caused by an inner error at {location}"))]
    FromHttp {
        source: Box<dyn HttpError>,
        #[snafu(implicit)]
        location: Location,
    },
}

impl WebError {
    pub fn http_error<E>(source: E, location: Location) -> Self
    where
        E: HttpError + 'static,
    {
        Self::FromHttp {
            source: Box::new(source),
            location,
        }
    }

    pub fn named_not_found(what: String, location: Location) -> Self {
        Self::NamedNotFound { what, location }
    }

    pub fn not_found(location: Location) -> Self {
        Self::NotFound { location }
    }

    pub fn unauthorized(location: Location) -> Self {
        Self::Unauthorized { location }
    }

    pub fn invalid_credentials(location: Location) -> Self {
        Self::InvalidCredentials { location }
    }

    pub fn not_in_team(location: Location) -> Self {
        Self::NotInTeam { location }
    }

    pub fn internal_error(message: String, location: Location) -> Self {
        Self::InternalError { message, location }
    }
}

impl HttpError for WebError {
    fn to_http_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            Self::InvalidCredentials { .. } => StatusCode::UNAUTHORIZED,
            Self::NamedNotFound { .. } => StatusCode::NOT_FOUND,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::NotInTeam { .. } => StatusCode::FORBIDDEN,
            Self::FromHttp { source, .. } => source.to_http_code(),
            Self::Sqlx { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            Self::Unauthorized { .. } => "unauthorized",
            Self::InvalidCredentials { .. } => "invalid_credentials",
            Self::NamedNotFound { .. } => "named_not_found",
            Self::NotFound { .. } => "not_found",
            Self::NotInTeam { .. } => "not_in_team",
            Self::FromHttp { source, .. } => source.to_error_code(),
            Self::Sqlx { .. } => "sql_error",
            Self::InternalError { .. } => "internal_error",
        }
    }

    fn to_extra(&self) -> Option<Value> {
        match self {
            Self::Unauthorized { .. } => None,
            Self::InvalidCredentials { .. } => None,
            Self::NamedNotFound { what, .. } => Some(json!({ "what": what })),
            Self::NotFound { .. } => None,
            Self::NotInTeam { .. } => None,
            Self::FromHttp { source, .. } => source.to_extra(),
            Self::Sqlx { .. } => None,
            Self::InternalError { message, .. } => Some(json!({ "message": message })),
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        if self.to_http_code() == StatusCode::INTERNAL_SERVER_ERROR {
            warn!(err = %Report::from_error(&self), "Returned internal error to user");
        }

        (
            self.to_http_code(),
            Json(json!({
                "error": Report::from_error(&self).to_string(),
                "code": self.to_error_code(),
                "extra": self.to_extra(),
            })),
        )
            .into_response()
    }
}
