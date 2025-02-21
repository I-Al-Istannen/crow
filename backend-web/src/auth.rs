use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{Result, WebError};
use crate::types::{JwtIssuer, UserId, UserRole};
pub use extractors::Claims;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use tracing::{debug, info, instrument, warn};

pub mod extractors;

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

const JWT_ISSUER: &str = "compilers";

#[instrument(level = "debug", skip(keys))]
pub fn create_jwt(user: UserId, keys: &Keys, role: UserRole) -> Result<String> {
    let exp = SystemTime::now()
        .add(Duration::from_secs(60 * 60 * 24)) // 24 hours
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let claims = Claims {
        sub: user,
        exp,
        iss: JwtIssuer(JWT_ISSUER.to_string()),
        role,
    };
    encode(&Header::default(), &claims, &keys.encoding).map_err(|e| {
        info!(error = ?e, "Error creating JWT");
        WebError::InternalServerError("Error creating JWT".to_string())
    })
}

#[instrument(level = "debug", skip(keys))]
fn validate_jwt(jwt: &str, keys: &Keys) -> Result<Claims> {
    let mut validation = Validation::default();
    validation.set_issuer(&[JWT_ISSUER]);
    decode::<Claims>(jwt, &keys.decoding, &validation)
        .map(|x| x.claims)
        .map_err(|e| {
            debug!(error = ?e, "JWT parsing error");

            WebError::InvalidCredentials
        })
}
