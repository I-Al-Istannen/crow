use console::style;
use keyring::Entry;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use snafu::{IntoError, Location, NoneError, ResultExt, Snafu};
use std::fmt::{Display, Formatter};
use tracing::error;

#[derive(Debug, Clone)]
pub struct BackendAuth(String);

impl Display for BackendAuth {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Snafu)]
pub enum AuthError {
    #[snafu(display("Keyring did not accept our secret name at {location}"))]
    EntryNameInvalid {
        source: keyring::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("The keyring entry was ambiguous (found {count}) at {location}"))]
    Ambiguous {
        count: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not read password from keyring at {location}"))]
    PasswordRead {
        source: keyring::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not write password to keyring at {location}"))]
    PasswordWrite {
        source: keyring::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Http error at {location}"))]
    ReqwestError {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Server replied with an unexpected code ({status_code}) message to login {message} at {location}"
    ))]
    LoginStatusCode {
        status_code: StatusCode,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}

pub fn get_stored_auth() -> Result<BackendAuth, AuthError> {
    let entry = Entry::new("crow-client", "backend-auth").context(EntryNameInvalidSnafu)?;

    let token = match entry.get_password() {
        Ok(token) => token,
        Err(keyring::Error::NoEntry) => display_login(),
        Err(keyring::Error::Ambiguous(creds)) => {
            return Err(AmbiguousSnafu { count: creds.len() }.into_error(NoneError));
        }
        Err(e) => return Err(PasswordReadSnafu.into_error(e)),
    };

    Ok(BackendAuth(token))
}

pub fn display_login() -> ! {
    error!(
        "{}{}",
        style("Unauthenticated. Please authenticate at ").red(),
        style("http://localhost:5173/cli-auth")
            .underlined()
            .bright()
            .red()
    );
    std::process::exit(1);
}

pub enum LoginResult {
    WrongPassword,
    Success(BackendAuth),
}

pub fn validate_token(
    client: &Client,
    token: String,
    backend_url: &str,
) -> Result<LoginResult, AuthError> {
    let res = client
        .get(format!("{}/users/me", backend_url))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .context(ReqwestSnafu)?;

    if res.status() == StatusCode::UNAUTHORIZED {
        return Ok(LoginResult::WrongPassword);
    }

    if res.status() == StatusCode::OK {
        return Ok(LoginResult::Success(BackendAuth(token)));
    }

    Err(LoginStatusCodeSnafu {
        status_code: res.status(),
        message: res.text().unwrap_or("N/A".to_string()),
    }
    .into_error(NoneError))
}

pub fn store_auth(auth: BackendAuth) -> Result<(), AuthError> {
    let entry = Entry::new("crow-client", "backend-auth").context(EntryNameInvalidSnafu)?;
    entry.set_password(&auth.0).context(PasswordWriteSnafu)
}
