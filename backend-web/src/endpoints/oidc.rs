use crate::auth::create_jwt;
use crate::auth::oidc::OidcFlowId;
use crate::endpoints::Json;
use crate::endpoints::user::LoginResponse;
use crate::error::Result;
use crate::error::WebError;
use crate::types::{AppState, UserRole};
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, Expiration, SameSite};
use serde::Deserialize;
use snafu::Report;

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

#[axum::debug_handler]
pub async fn login_oidc_callback(
    State(state): State<AppState>,
    Query(oidc_callback_query): Query<OidcCallbackQuery>,
    cookies: CookieJar,
) -> Result<(CookieJar, Json<LoginResponse>)> {
    let flow_id = cookies
        .get("oidc_flow_id")
        .ok_or(WebError::InvalidCredentials)?;
    let flow_id = OidcFlowId::from_string(flow_id.value().to_string());

    let res = state
        .oidc
        .handle_oidc_callback(
            flow_id,
            &oidc_callback_query.code,
            &oidc_callback_query.state,
        )
        .await;

    let user = match res {
        Ok(user) => user,
        Err(e) => {
            return Err(WebError::InternalServerError(
                Report::from_error(e).to_string(),
            ));
        }
    };

    let user = state.db.synchronize_oidc_user(user.clone()).await?.user;
    let jwt = create_jwt(user.id.clone(), &state.jwt_keys, UserRole::Regular)?;

    Ok((
        cookies.remove("oidc_flow_id"),
        Json(LoginResponse { user, token: jwt }),
    ))
}

#[derive(Deserialize)]
pub struct OidcCallbackQuery {
    code: String,
    state: String,
}
