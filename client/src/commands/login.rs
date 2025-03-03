use crate::auth::{store_auth, validate_token, LoginResult};
use crate::error::{AuthSnafu, Result};
use console::style;
use dialoguer::Password;
use dialoguer::theme::ColorfulTheme;
use reqwest::blocking::Client;
use snafu::ResultExt;
use tracing::{error, info};

pub fn command_login(client: Client, backend_url: &str) -> Result<()> {
    loop {
        if login_iteration(&client, backend_url)? {
            break;
        }
    }

    info!("{}", style("Successfully logged in!").bright().green());
    info!(
        "{}",
        style(
            "Your password is now cached in your systems keyring. \
            Further commands will work without authentication"
        )
        .bright()
        .green()
    );

    Ok(())
}

fn login_iteration(client: &Client, backend_url: &str) -> Result<bool> {
    let token = Password::with_theme(&ColorfulTheme::default())
        .with_prompt(style("Backend token").magenta().to_string())
        .interact();
    let Ok(token) = token else {
        error!("You aborted the login process");
        std::process::exit(1);
    };

    let auth = validate_token(client, token, backend_url).context(AuthSnafu)?;

    let auth = match auth {
        LoginResult::WrongPassword => {
            error!("{}", style("Wrong token").red());
            return Ok(false);
        }
        LoginResult::Success(auth) => auth,
    };

    store_auth(auth).context(AuthSnafu)?;

    Ok(true)
}
