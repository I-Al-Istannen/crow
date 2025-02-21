use crate::auth::validate_jwt;
use crate::error::WebError;
use crate::types::{AppState, JwtIssuer, UserId, UserRole};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::{RequestPartsExt, async_trait};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
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
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| WebError::InvalidCredentials)?;

        Self::from_token(state, bearer.token()).await
    }
}

impl Claims {
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }

    pub fn is_admin_opt(claims: &Option<Self>) -> bool {
        claims.as_ref().map(|x| x.role).unwrap_or(UserRole::Regular) == UserRole::Admin
    }

    pub async fn from_token(state: &AppState, token: &str) -> Result<Self, WebError> {
        let mut claims = validate_jwt(token, &state.jwt_keys)?;

        // Update claims from DB to instantly process role changes (yes, this kind of
        // defeats normal stateless JWT flows)
        let Some(user) = state.db.get_user_for_login(&claims.sub).await? else {
            return Err(WebError::InvalidCredentials);
        };
        claims.role = user.role;

        Ok(claims)
    }
}
