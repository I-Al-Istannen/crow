use crate::auth::{CrowJwt, validate_jwt};
use crate::db::UserForAuth;
use crate::error::WebError;
use crate::types::{AppState, JwtIssuer, TeamId, UserId, UserRole};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::{RequestPartsExt, async_trait};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use serde::{Deserialize, Serialize};
use snafu::location;
use std::fmt::Debug;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims<T: Debug + Clone = TeamId> {
    pub sub: UserId,
    pub team: T,
    pub exp: u64,
    pub iss: JwtIssuer,
    pub role: UserRole,
}

macro_rules! implement_request_parts {
    ($generic:ty) => {
        #[async_trait]
        impl FromRequestParts<AppState> for Claims<$generic> {
            type Rejection = WebError;

            async fn from_request_parts(
                parts: &mut Parts,
                state: &AppState,
            ) -> Result<Self, Self::Rejection> {
                let TypedHeader(Authorization(bearer)) = parts
                    .extract::<TypedHeader<Authorization<Bearer>>>()
                    .await
                    .map_err(|_| WebError::invalid_credentials(location!()))?;

                Self::from_token(state, bearer.token()).await
            }
        }
    };
}

implement_request_parts!(Option<TeamId>);
implement_request_parts!(TeamId);

impl<T: Debug + Clone> Claims<T> {
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }
}

impl Claims<Option<TeamId>> {
    pub async fn from_token(state: &AppState, token: &str) -> Result<Self, WebError> {
        let (claims, user) = jwt_user_from_token(state, token).await?;

        Ok(Self {
            sub: claims.sub,
            team: user.user.team,
            exp: claims.exp,
            iss: claims.iss,
            role: claims.role,
        })
    }
}

impl Claims<TeamId> {
    pub async fn from_token(state: &AppState, token: &str) -> Result<Self, WebError> {
        let (claims, user) = jwt_user_from_token(state, token).await?;

        let Some(team) = user.user.team else {
            return Err(WebError::not_in_team(location!()));
        };

        Ok(Self {
            sub: claims.sub,
            team,
            exp: claims.exp,
            iss: claims.iss,
            role: claims.role,
        })
    }
}

async fn jwt_user_from_token(
    state: &AppState,
    token: &str,
) -> Result<(CrowJwt, UserForAuth), WebError> {
    let mut claims = validate_jwt(token, &state.jwt_keys)?;

    // Update claims from DB to instantly process role changes (yes, this kind of
    // defeats normal stateless JWT flows)
    let Some(user) = state.db.get_user_for_login(&claims.sub).await? else {
        info!(user = %claims.sub, "User no longer found but tried using valid JWT");

        return Err(WebError::invalid_credentials(location!()));
    };
    claims.role = user.role;

    Ok((claims, user))
}
