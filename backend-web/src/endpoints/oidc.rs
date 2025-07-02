use crate::auth::create_jwt;
use crate::auth::oidc::{OidcError, OidcFlowId};
use crate::endpoints::Json;
use crate::endpoints::user::LoginResponse;
use crate::error::WebError;
use crate::error::{HttpError, Result};
use crate::types::{AppState, UserRole};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, Expiration, SameSite};
use serde::Deserialize;
use snafu::{Report, location};
use tracing::{info, warn};

pub async fn login_oidc(
    State(state): State<AppState>,
    cookies: CookieJar,
) -> Result<(CookieJar, Redirect)> {
    let oidc_auth_redirect = state.oidc.get_oidc_auth_redirect().await;
    let cookies = cookies.add(
        Cookie::build(("oidc_flow_id", oidc_auth_redirect.flow_id.to_string()))
            .http_only(true)
            .secure(true)
            .expires(Expiration::Session)
            .same_site(SameSite::Lax)
            .build(),
    );

    Ok((cookies, Redirect::temporary(&oidc_auth_redirect.url)))
}

pub async fn login_oidc_callback(
    State(state): State<AppState>,
    cookies: CookieJar,
    Json(oidc_callback_payload): Json<OidcCallbackPayload>,
) -> Result<(CookieJar, Json<LoginResponse>)> {
    let flow_id = match cookies.get("oidc_flow_id") {
        Some(flow_id) => flow_id,
        None => {
            warn!("Received oidc login callback without oidc flow id cookie");
            return Err(WebError::invalid_credentials(location!()));
        }
    };
    let flow_id = OidcFlowId::from_string(flow_id.value().to_string());

    info!(flow_id = %flow_id, "Handling OIDC callback");

    let res = state
        .oidc
        .handle_oidc_callback(
            flow_id.clone(),
            &oidc_callback_payload.code,
            &oidc_callback_payload.state,
        )
        .await;

    let user = match res {
        Ok(user) => user,
        Err(e) => {
            info!(flow_id = %flow_id, error = %Report::from_error(&e), "OIDC login failed");
            return Err(WebError::http_error(e, location!()));
        }
    };

    let mapping = state.team_mapping.get(&user.id.clone().into()).cloned();
    let role = mapping.as_ref().map(|it| it.1);
    let team = mapping.map(|it| it.0);
    let own_user = state
        .db
        .synchronize_oidc_user(user.clone(), team, role)
        .await?;
    let user = &own_user.user;
    let jwt = create_jwt(user.id.clone(), &state.jwt_keys, UserRole::Regular)?;

    info!(
        flow_id = flow_id.to_string(),
        user = %user.id,
        user_name = %user.display_name,
        "OIDC login successful"
    );

    Ok((
        cookies.remove("oidc_flow_id"),
        Json(LoginResponse {
            user: own_user,
            token: jwt,
        }),
    ))
}

#[derive(Deserialize)]
pub struct OidcCallbackPayload {
    code: String,
    state: String,
}

impl HttpError for OidcError {
    fn to_http_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_error_code(&self) -> &'static str {
        "oidc_error"
    }
}
