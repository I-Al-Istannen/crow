use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{Result, WebError};
use crate::types::{JwtIssuer, UserId, UserRole};
pub use extractors::Claims;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use snafu::location;
use tracing::{debug, info, instrument, warn};

pub mod extractors;
pub mod oidc;

#[derive(Clone)]
pub struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrowJwt {
    pub sub: UserId,
    pub exp: u64,
    pub iss: JwtIssuer,
    pub role: UserRole,
}

const JWT_ISSUER: &str = "compilers";

#[instrument(level = "debug", skip(keys))]
pub fn create_jwt(user: UserId, keys: &Keys, role: UserRole) -> Result<String> {
    let exp = SystemTime::now()
        .add(Duration::from_secs(60 * 60 * 24)) // 24 hours
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let claims = CrowJwt {
        sub: user,
        exp,
        iss: JwtIssuer(JWT_ISSUER.to_string()),
        role,
    };
    encode(&Header::default(), &claims, &keys.encoding).map_err(|e| {
        info!(error = ?e, "Error creating JWT");
        WebError::internal_error("Error creating JWT".to_string(), location!())
    })
}

#[instrument(level = "debug", skip(keys))]
fn validate_jwt(jwt: &str, keys: &Keys) -> Result<CrowJwt> {
    let mut validation = Validation::default();
    validation.set_issuer(&[JWT_ISSUER]);
    decode::<CrowJwt>(jwt, &keys.decoding, &validation)
        .map(|x| x.claims)
        .map_err(|e| {
            debug!(error = ?e, "JWT parsing error");

            WebError::invalid_credentials(location!())
        })
}
