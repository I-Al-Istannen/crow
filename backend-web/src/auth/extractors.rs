use crate::auth::validate_jwt;
use crate::error::WebError;
use crate::types::{AppState, JwtIssuer, UserId, UserRole};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::{async_trait, RequestPartsExt};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: UserId,
    pub exp: u64,
    pub iss: JwtIssuer,
    pub role: UserRole,
}

#[async_trait]
impl FromRequestParts<AppState> for Claims {
    type Rejection = WebError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| WebError::InvalidCredentials)?;

        Ok(validate_jwt(bearer.token(), &_state.jwt_keys)?)
    }
}

impl Claims {
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }

    pub fn is_admin_opt(claims: &Option<Claims>) -> bool {
        claims.as_ref().map(|x| x.role).unwrap_or(UserRole::Regular) == UserRole::Admin
    }
}
