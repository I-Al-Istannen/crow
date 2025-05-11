use crate::context::MyselfResponse;
use crate::util::st;
use console::style;
use keyring::Entry;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use shared::indent;
use snafu::{IntoError, Location, NoneError, Report, ResultExt, Snafu};
use std::fmt::{Display, Formatter};
use tracing::{error, info};

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

pub fn get_stored_auth(frontend_url: &str) -> Result<BackendAuth, AuthError> {
    let token = std::env::var("CROW_CLIENT_AUTH_TOKEN").ok();
    if let Some(token) = token {
        info!("CROW_CLIENT_AUTH_TOKEN is set, using it as the auth token.");
        return Ok(BackendAuth(token));
    }

    let entry = Entry::new("crow-client", "backend-auth").context(EntryNameInvalidSnafu)?;

    let token = match entry.get_password() {
        Ok(token) => token,
        Err(keyring::Error::NoEntry) => display_login(frontend_url),
        Err(keyring::Error::Ambiguous(creds)) => {
            return Err(AmbiguousSnafu { count: creds.len() }.into_error(NoneError));
        }
        Err(e) => return Err(PasswordReadSnafu.into_error(e)),
    };

    Ok(BackendAuth(token))
}

pub fn display_login(frontend_url: &str) -> ! {
    let me = match std::env::current_exe() {
        Err(_) => "crow-client".to_string(),
        Ok(exe) => exe
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or("crow-client".to_string()),
    };
    error!(
        "{}",
        st(style("Unauthenticated.").red())
            .append(" Please authenticate at ")
            .append(
                style(format!("{frontend_url}/cli-auth"))
                    .bright()
                    .bold()
                    .cyan()
            )
            .append(" and then use the '")
            .append(style(format!("{me} login")).bold().cyan())
            .append("' command to authenticate."),
    );
    std::process::exit(1);
}

pub enum LoginResult {
    WrongPassword,
    Success { name: String, auth: BackendAuth },
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
        let me = res.json::<MyselfResponse>().context(ReqwestSnafu)?;
        return Ok(LoginResult::Success {
            name: me.user.display_name,
            auth: BackendAuth(token),
        });
    }

    Err(LoginStatusCodeSnafu {
        status_code: res.status(),
        message: res.text().unwrap_or("N/A".to_string()),
    }
    .into_error(NoneError))
}

pub fn store_auth(auth: BackendAuth) -> Result<(), AuthError> {
    let entry = Entry::new("crow-client", "backend-auth").context(EntryNameInvalidSnafu)?;
    if let Err(e) = entry.set_password(&auth.0).context(PasswordWriteSnafu) {
        error!(
            "{}",
            st(style("Failed").red())
                .append(" to set password in keyring. ")
                .append("You can set the ")
                .append(style("CROW_CLIENT_AUTH_TOKEN").bold().cyan())
                .append(" environment variable to a valid token to bypass the keyring.\n")
                .append(indent(&Report::from_error(e).to_string(), 2))
        );
    };

    Ok(())
}
