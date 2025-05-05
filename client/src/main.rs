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
use snafu::{Report, ResultExt};
use std::process::ExitCode;
use tracing::{error, info};
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
