// We do not actually care
#![allow(clippy::result_large_err)]

mod auth;
mod commands;
mod context;
mod error;
mod formats;
mod util;

use self::error::Result;
use crate::auth::get_stored_auth;
use crate::commands::login::command_login;
use crate::commands::run_test::{CliRunTestArgs, CliRunTestsArgs};
use crate::commands::sync_tests::{command_sync_tests, CliSyncTestsArgs};
use crate::commands::upload::CliUploadTestArgs;
use crate::context::CliContext;
use crate::error::AuthSnafu;
use crate::util::st;
use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{Parser, Subcommand};
use console::style;
use reqwest::blocking::Client;
use snafu::{ensure_whatever, OptionExt, Report, ResultExt, Whatever};
use std::fs::File;
use std::io::Write;
use std::process::ExitCode;
use std::time::SystemTime;
use tracing::{debug, error, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// noinspection DuplicatedCode
const CLAP_STYLE: Styles = Styles::styled()
    .header(AnsiColor::Red.on_default().bold())
    .usage(AnsiColor::Red.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Green.on_default());

// noinspection DuplicatedCode
/// Client for crow, the avian testing solution.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles = CLAP_STYLE)]
#[command(propagate_version = true)]
struct CliArgs {
    #[clap(long, default_value = "https://compiler.vads.kastel.kit.edu")]
    frontend_url: String,
    #[clap(long, default_value = "https://compiler.vads.kastel.kit.edu/api")]
    backend_url: String,
    #[clap(subcommand)]
    subcommand: CliCommand,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Logs you in to crow
    Login,
    /// One-way synchronizes crow tests with a local folder.
    /// Creates git snapshots to prevent data loss.
    SyncTests(CliSyncTestsArgs),
    /// Runs a single test against your compiler locally
    RunTest(CliRunTestArgs),
    /// Runs all tests against your compiler locally
    RunTests(CliRunTestsArgs),
    /// Uploads a new test or updates an existing to crow
    UploadTest(CliUploadTestArgs),
}

fn main() -> ExitCode {
    // Maybe: https://fasterthanli.me/articles/request-coalescing-in-async-rust#a-bit-of-tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .without_time()
                .with_target(false),
        )
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    if let Err(e) = check_updates() {
        println!("{}", Report::from_error(e));
    }

    let res = Report::capture_into_result(|| {
        let args = CliArgs::parse();
        let client = Client::new();
        let backend_url = &args.backend_url;
        let frontend_url = &args.frontend_url;

        match args.subcommand {
            CliCommand::Login => command_login(client, backend_url, frontend_url),
            CliCommand::SyncTests(args) => {
                command_sync_tests(args, get_context(backend_url, frontend_url, client)?)
            }
            CliCommand::RunTest(args) => commands::run_test::command_run_test(args),
            CliCommand::RunTests(args) => commands::run_test::command_run_tests(args),
            CliCommand::UploadTest(args) => commands::upload::command_upload_test(
                args,
                get_context(backend_url, frontend_url, client)?,
            ),
        }
    });

    let res = match res {
        Ok(false) => ExitCode::FAILURE,
        Ok(true) => ExitCode::SUCCESS,
        Err(report) => {
            error!("\n{}", style(report).bright().red());
            ExitCode::FAILURE
        }
    };

    println!();

    if res == ExitCode::SUCCESS {
        info!(
            "{}",
            st("Exiting ")
                .append(style("successfully").green())
                .append(". Goodbye, have a n")
                .append(style("ice").blue().bright())
                .append(" day!")
        );
    } else {
        info!(
            "{}",
            st("Exiting ")
                .append(style("unsuccessfully").red())
                .append(". Goodbye, have a n")
                .append(style("ice").blue().bright())
                .append(" day!")
        );
    }

    res
}

fn get_context(backend_url: &str, frontend_url: &str, client: Client) -> Result<CliContext> {
    Ok(CliContext::new(
        get_stored_auth(frontend_url).context(AuthSnafu)?,
        client,
        backend_url.to_string(),
        frontend_url.to_string(),
    ))
}

fn check_updates() -> std::result::Result<(), Whatever> {
    if !should_perform_update_check()? {
        debug!("Skipping update check");
        return Ok(());
    }

    info!("Checking for updates...");

    let my_version = semver::Version::parse(clap::crate_version!())
        .whatever_context("Could not parse own version")?;

    let client = Client::new();

    let remote_version = client
        .get("https://api.github.com/repos/I-Al-Istannen/crow/releases/latest")
        .header("User-Agent", "crow-client")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .whatever_context("Could not fetch latest version from GitHub")?
        .json::<serde_json::Value>()
        .whatever_context("Could not parse GitHub version response")?;

    let remote_version = remote_version
        .get("tag_name")
        .whatever_context("Could not find tag_name")?
        .as_str()
        .whatever_context("tag_name is not a string")?;
    let remote_version = remote_version.strip_prefix('v').unwrap_or(remote_version);

    let remote_version = semver::Version::parse(remote_version)
        .whatever_context("Could not parse remote version")?;

    if remote_version > my_version {
        println!();
        warn!(
            "{}",
            st("A new version of crow is available: ")
                .append(style(format!("{remote_version}")).green().bold())
                .append(". You are using ")
                .append(style(format!("{my_version}")).red().bold())
                .append(". Please update!")
        );
        println!();
    } else {
        info!(
            "{}",
            st("You are using the latest version of crow: ")
                .append(style(format!("{my_version}")).green())
        );
    }

    Ok(())
}

fn should_perform_update_check() -> std::result::Result<bool, Whatever> {
    let temp_dir = tempfile::env::temp_dir();
    ensure_whatever!(
        temp_dir.exists(),
        "Temp dir does not exist, skipping update check"
    );

    let update_file = temp_dir.join("crow-client-last-update-check");
    let file_existed = update_file.exists();
    let last_modified = std::fs::metadata(&update_file)
        .map(|it| it.modified())
        .unwrap_or(Ok(SystemTime::UNIX_EPOCH));

    let time_is_over = match last_modified {
        Ok(last_modified) => match last_modified.elapsed() {
            Err(e) => {
                info!(source = %Report::from_error(e), "Could not get last modified time, performing update check");
                true
            }
            Ok(elapsed) => elapsed.as_secs() > 60 * 60,
        },
        Err(e) => {
            info!(source = %Report::from_error(e), "Could not get last modified time, performing update check");
            true
        }
    };

    if file_existed && !time_is_over {
        return Ok(false);
    }

    File::create(&update_file)
        .whatever_context("Could not create update tracking file")?
        .write_all("Hello there :)".as_bytes())
        .whatever_context("Could not write to update tracking file")?;

    Ok(true)
}
