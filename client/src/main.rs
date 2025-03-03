// We do not actually care
#![allow(clippy::result_large_err)]

mod auth;
mod commands;
mod context;
mod error;
mod util;

use self::error::Result;
use crate::auth::get_stored_auth;
use crate::commands::login::command_login;
use crate::commands::run_test::{CliRunTestArgs, CliRunTestsArgs};
use crate::commands::sync_tests::{command_sync_tests, CliSyncTestsArgs};
use crate::commands::upload::CliUploadTestArgs;
use crate::context::CliContext;
use crate::error::AuthSnafu;
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

// [x] Authentication dance at the beginning
//   - If not authenticated => Print url to <frontend>/cli-auth
//     - If already authed => Show JWT, else normal login stuff
//     - Remember auth token in keyring (or ~/.config/crow-client/config.toml)
//   - We are authenticated

// crow-client sync-tests --test-dir <test folder>
//   - [x] Auth
//   - [x] Download new tests
//     - [x] List all local files:
//         <foo.crow-test>
//         <foo.crow-test.expected>
//         <foo.crow-test.meta>     // contains author
//     - [x] Fetch remote test list:
//       - test id
//       - hash of input/output
//     - [x] Download remote-only files:
//       - create <foo.crow-test>
//       - create <foo.crow-test.expected>
//     - [x] Keep track in git to allow user to decide what to do
//     - [-] Prompt whether to update local test
//       - show unified diff (?)
//     - [-] Prompt user to delete tests that do not exist in remote
//     - [-] CLI args for: prefer-local, prefer-remote, prompt
//  - [x] Wish user a good day

// [x] crow-client upload-test <input file> <output file> [--name <name>] [--category <category>]
//   - [x] Prompt for name/category if not given
//   - [ ] Execute test against reference compiler
//   - [ ] If failed
//     - [ ] Warn user, prompt for continuing upload
//   - [x] Upload test

// [x] crow-client run-test --test-dir <test dir> --test <test id> --run <compiler run path>
//   - Runs a single test against your local compiler run binary
//   - Shows you the diff on failure

// [x] crow-client run-tests --test-dir <test dir> --run <compiler run path>
//   - Executes all tests
//   - Sequentially (we do not implement the whole executor virtualization stuff)

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

        match args.subcommand {
            CliCommand::Login => command_login(client),
            CliCommand::SyncTests(args) => command_sync_tests(args, get_context(client)?),
            CliCommand::RunTest(args) => commands::run_test::command_run_test(args),
            CliCommand::RunTests(args) => commands::run_test::command_run_tests(args),
            CliCommand::UploadTest(args) => {
                commands::upload::command_upload_test(args, get_context(client)?)
            }
        }
    });

    let Err(report) = res else {
        info!(
            "{}{}{}",
            style("Exiting successfully. Goodbye, have a n").green(),
            style("ice").blue().bright(),
            style(" day!").green()
        );
        return ExitCode::SUCCESS;
    };

    error!("\n{}", style(report.to_string()).bright().red());

    ExitCode::FAILURE
}

fn get_context(client: Client) -> Result<CliContext> {
    Ok(CliContext::new(
        get_stored_auth().context(AuthSnafu)?,
        client,
    ))
}
