use crate::config::OidcConfig;
use derive_more::Display;
use oauth2::basic::{BasicErrorResponseType, BasicRevocationErrorResponse};
use oauth2::{HttpClientError, PkceCodeVerifier, RequestTokenError, StandardRevocableToken};
use openidconnect::core::{
    CoreAuthDisplay, CoreAuthPrompt, CoreAuthenticationFlow, CoreClient, CoreGenderClaim,
    CoreIdTokenClaims, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm, CoreProviderMetadata,
    CoreTokenIntrospectionResponse, CoreTokenResponse,
};
use openidconnect::{
    AccessTokenHash, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    EmptyAdditionalClaims, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, RedirectUrl, Scope, StandardErrorResponse,
    TokenResponse,
};
use snafu::{Location, OptionExt, ResultExt, Snafu, ensure};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::debug;

#[derive(Debug, Snafu)]
pub enum OidcError {
    #[snafu(display("Could not create http client at {location}"))]
    HttpClientCreation {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not parse issuer URL `{url}` at {location}"))]
    IssuerUrlInvalid {
        url: String,
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not parse redirect URL `{url}` at {location}"))]
    RedirectUrlInvalid {
        url: String,
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not fetch provider metadata at {location}"))]
    ProviderMetadata {
        source: openidconnect::DiscoveryError<HttpClientError<reqwest::Error>>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("CSRF token mismatch (`{expected}` vs `{actual}`) at {location}"))]
    InvalidCsrfToken {
        expected: String,
        actual: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Unknown flow id `{flow_id}` at {location}"))]
    UnknownFlowId {
        flow_id: OidcFlowId,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Unable to configure authentication token exchange at {location}"))]
    AuthTokenExchangeConfig {
        source: oauth2::ConfigurationError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error during authentication token exchange request at {location}"))]
    AuthTokenExchangeRequest {
        source: RequestTokenError<
            HttpClientError<reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Server returned no id token after token exchange at {location}"))]
    NoIdTokenReceived {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error verifying claims at {location}"))]
    ClaimVerification {
        source: openidconnect::ClaimsVerificationError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error verifying signature at {location}"))]
    SignatureVerification {
        source: openidconnect::SignatureVerificationError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not hash access token at {location}"))]
    AccessTokenHashing {
        source: openidconnect::SigningError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Access token signature did not match expected at {location}"))]
    AccessTokenSignatureVerification {
        actual: String,
        expected: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Display)]
pub struct OidcFlowId(String);

impl OidcFlowId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

struct PendingAuthentication {
    time: Instant,
    pkce_code_verifier: PkceCodeVerifier,
    csrf_token: CsrfToken,
    nonce: Nonce,
}

#[derive(Debug, Clone)]
pub struct OidcAuthRedirect {
    pub flow_id: OidcFlowId,
    pub url: String,
}

#[derive(Clone)]
pub struct Oidc {
    http_client: reqwest::Client,
    oidc_config: OidcConfig,
    #[allow(clippy::type_complexity)]
    oidc_client: Client<
        EmptyAdditionalClaims,
        CoreAuthDisplay,
        CoreGenderClaim,
        CoreJweContentEncryptionAlgorithm,
        CoreJsonWebKey,
        CoreAuthPrompt,
        StandardErrorResponse<BasicErrorResponseType>,
        CoreTokenResponse,
        CoreTokenIntrospectionResponse,
        StandardRevocableToken,
        BasicRevocationErrorResponse,
        EndpointSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointMaybeSet,
        EndpointMaybeSet,
    >,
    pending_auths: Arc<Mutex<HashMap<OidcFlowId, PendingAuthentication>>>,
}

impl Oidc {
    pub async fn build_new(oidc_config: OidcConfig) -> Result<Self, OidcError> {
        let http_client = reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context(HttpClientCreationSnafu)?;

        let provider_metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(oidc_config.issuer_url.clone()).context(IssuerUrlInvalidSnafu {
                url: oidc_config.issuer_url.clone(),
            })?,
            &http_client,
        )
        .await
        .context(ProviderMetadataSnafu)?;

        let oidc_client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(oidc_config.client_id.clone()),
            Some(ClientSecret::new(oidc_config.client_secret.to_string())),
        )
        .set_redirect_uri(
            RedirectUrl::new(oidc_config.redirect_url.to_string()).context(
                RedirectUrlInvalidSnafu {
                    url: oidc_config.redirect_url.to_string(),
                },
            )?,
        );

        let pending_auths: Arc<Mutex<HashMap<OidcFlowId, PendingAuthentication>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Periodically trim the pending authentications. If a user takes longer than 5 minutes on
        // shibboleth they can start again
        let pending_auths_clone = pending_auths.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let mut pending_auths = pending_auths_clone.lock().unwrap();

                debug!(
                    pending = pending_auths.len(),
                    "Trimming pending authentications"
                );

                let now = Instant::now();
                pending_auths
                    .retain(|_, auth| now.duration_since(auth.time) < Duration::from_secs(5 * 60));
                drop(pending_auths);
            }
        });

        Ok(Self {
            http_client,
            oidc_config,
            oidc_client,
            pending_auths,
        })
    }

    pub async fn get_oidc_auth_redirect(&self) -> OidcAuthRedirect {
        let flow_id = OidcFlowId::new();

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (auth_url, csrf_token, nonce) = self
            .oidc_client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scopes(
                self.oidc_config
                    .scopes
                    .iter()
                    .map(|s| Scope::new(s.clone())),
            )
            .set_pkce_challenge(pkce_challenge)
            .url();

        self.pending_auths.lock().unwrap().insert(
            flow_id.clone(),
            PendingAuthentication {
                time: Instant::now(),
                pkce_code_verifier: pkce_verifier,
                csrf_token,
                nonce,
            },
        );

        OidcAuthRedirect {
            flow_id,
            url: auth_url.to_string(),
        }
    }

    pub async fn handle_oidc_callback(
        &self,
        flow_id: OidcFlowId,
        auth_token: &str,
        state: &str,
    ) -> Result<OidcUser, OidcError> {
        let pending_auth = self
            .pending_auths
            .lock()
            .unwrap()
            .remove(&flow_id)
            .context(UnknownFlowIdSnafu { flow_id })?;

        ensure!(
            pending_auth.csrf_token.secret() == state,
            InvalidCsrfTokenSnafu {
                expected: pending_auth.csrf_token.secret(),
                actual: state
            }
        );

        let token_response = self
            .oidc_client
            .exchange_code(AuthorizationCode::new(auth_token.to_string()))
            .context(AuthTokenExchangeConfigSnafu)?
            .set_pkce_verifier(pending_auth.pkce_code_verifier)
            .request_async(&self.http_client)
            .await
            .context(AuthTokenExchangeRequestSnafu)?;

        // Extract the ID token claims after verifying its authenticity and nonce.
        let id_token = token_response.id_token().context(NoIdTokenReceivedSnafu)?;
        let claims: &CoreIdTokenClaims = id_token
            .claims(&self.oidc_client.id_token_verifier(), &pending_auth.nonce)
            .context(ClaimVerificationSnafu)?;

        // Verify the access token hash to ensure that the access token hasn't been substituted for
        // another user's.
        if let Some(expected_access_token_hash) = claims.access_token_hash() {
            let actual_access_token_hash = AccessTokenHash::from_token(
                token_response.access_token(),
                id_token.signing_alg().context(SignatureVerificationSnafu)?,
                id_token
                    .signing_key(&self.oidc_client.id_token_verifier())
                    .context(SignatureVerificationSnafu)?,
            )
            .context(AccessTokenHashingSnafu)?;

            ensure!(
                actual_access_token_hash == *expected_access_token_hash,
                AccessTokenSignatureVerificationSnafu {
                    actual: actual_access_token_hash.to_string(),
                    expected: expected_access_token_hash.to_string()
                }
            );
        }

        Ok(OidcUser {
            id: claims.subject().to_string(),
            name: claims
                .preferred_username()
                .map(|name| name.as_str())
                .unwrap_or("<not provided>")
                .to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct OidcUser {
    pub id: String,
    pub name: String,
}
