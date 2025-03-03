use crate::auth::{store_auth, validate_token, LoginResult};
use crate::error::{AuthSnafu, Result};
use console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Password;
use reqwest::blocking::Client;
use snafu::ResultExt;
use tracing::{error, info};

pub fn command_login(client: Client, backend_url: &str) -> Result<bool> {
    loop {
        if login_iteration(&client, backend_url)? {
            break;
        }
    }

    info!("Successfully logged in!");
    info!(
        "Your password is now cached in your system keyring. \
        Further commands will work without authentication."
    );

    Ok(true)
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

    let (auth, name) = match auth {
        LoginResult::WrongPassword => {
            error!("{}", style("Wrong token").red());
            return Ok(false);
        }
        LoginResult::Success { auth, name } => (auth, name),
    };

    store_auth(auth).context(AuthSnafu)?;

    info!("Welcome, {}!", style(name).green().bold().bright());

    Ok(true)
}
