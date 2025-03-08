use crate::context::{CliContext, CliContextError, RemoteTests, SetTestResponse};
use crate::error::{ContextSnafu, CrowClientError, UploadTestSnafu};
use crate::util::color_diff;
use clap::Args;
use console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, FuzzySelect, Input, Select};
use shared::{ExecutionOutput, FinishedTest};
use snafu::{IntoError, Location, NoneError, ResultExt, Snafu};
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Debug, Snafu)]
pub enum UploadTestError {
    #[snafu(display("User aborted {what} at {location}"))]
    UserAbort {
        what: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("You are not in a team at {location}"))]
    NotInTeam {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Test name `{name}` invalid due to `{error}` at {location}"))]
    TestName {
        error: &'static str,
        name: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error reading input file `{}` at {location}", path.display()))]
    InputFileRead {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error reading output file `{}` at {location}", path.display()))]
    OutputFileRead {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error uploading test at {location}"))]
    Uploading {
        source: CliContextError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Args, Debug)]
pub struct CliUploadTestArgs {
    /// The test input file
    input_file: PathBuf,
    /// The test expected output file
    output_file: PathBuf,
    /// The test name
    #[clap(short, long)]
    name: Option<String>,
    /// The test category
    #[clap(short, long)]
    category: Option<String>,
    /// Should the test be only submitted if it works with the reference compiler?
    #[clap(long)]
    taste_test: Option<bool>,
}

pub fn command_upload_test(
    args: CliUploadTestArgs,
    ctx: CliContext,
) -> Result<bool, CrowClientError> {
    let remote_tests = ctx.get_remote_tests().context(ContextSnafu)?;
    let myself = ctx.get_myself().context(ContextSnafu)?;
    let Some(my_team) = myself.team else {
        return Err(NotInTeamSnafu.into_error(NoneError)).context(UploadTestSnafu);
    };

    let input = std::fs::read_to_string(&args.input_file)
        .context(InputFileReadSnafu {
            path: args.input_file.to_path_buf(),
        })
        .context(UploadTestSnafu)?;
    let output = std::fs::read_to_string(&args.output_file)
        .context(OutputFileReadSnafu {
            path: args.output_file.to_path_buf(),
        })
        .context(UploadTestSnafu)?;

    let category = match args.category {
        Some(category) => category,
        None => prompt_test_category(&remote_tests.categories).context(UploadTestSnafu)?,
    };

    let name = match args.name {
        Some(name) => name,
        None => prompt_test_name(&category, &remote_tests, &my_team).context(UploadTestSnafu)?,
    };

    if let Err(e) = validate_test_name(&name) {
        return Err(TestNameSnafu { error: e, name }.into_error(NoneError))
            .context(UploadTestSnafu);
    }

    let should_taste_test = match args.taste_test {
        None => prompt_should_taste_test().context(UploadTestSnafu)?,
        Some(val) => val
    };

    let res = ctx
        .upload_test(&name, &category, &input, &output, should_taste_test)
        .context(UploadingSnafu)
        .context(UploadTestSnafu)?;

    Ok(match res {
        SetTestResponse::TestAdded(_) => {
            info!("Test uploaded {}", style("successfully").green().bright());
            true
        }
        SetTestResponse::TastingFailed(test) => {
            error!("Test failed test tasting");
            print_finished_test(test);
            false
        }
    })
}

fn prompt_test_category(categories: &[String]) -> Result<String, UploadTestError> {
    let selected = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a category")
        .items(categories)
        .interact_opt();

    let Ok(Some(selected)) = selected else {
        return Err(UserAbortSnafu {
            what: "category selection",
        }
        .into_error(NoneError));
    };

    Ok(categories[selected].clone())
}

fn prompt_test_name(
    category: &str,
    remote_tests: &RemoteTests,
    my_team: &str,
) -> Result<String, UploadTestError> {
    let selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to update an existing test or add a new one?")
        .items(&["Update existing test", "Add new test"])
        .interact_opt();

    let Ok(Some(selected)) = selected else {
        return Err(UserAbortSnafu {
            what: "update or create selection",
        }
        .into_error(NoneError));
    };

    let name = if selected == 0 {
        let possible_tests: Vec<String> = remote_tests
            .tests
            .iter()
            .filter(|test| test.category == category)
            .filter(|test| test.creator_id == my_team)
            .map(|it| it.id.clone())
            .collect();

        FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a test to update")
            .items(&possible_tests)
            .interact_opt()
            .map(|selected| selected.map(|selected| possible_tests[selected].clone()))
    } else {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter the name of the new test")
            .validate_with(|input: &String| validate_test_name(input))
            .interact()
            .map(Some)
    };

    let Ok(Some(name)) = name else {
        return Err(UserAbortSnafu {
            what: "test name input",
        }
        .into_error(NoneError));
    };

    Ok(name)
}

fn validate_test_name(input: &str) -> Result<(), &'static str> {
    if input
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
    {
        Ok(())
    } else {
        Err("Test id must only contain alphanumerics, dashes, underscores or spaces")
    }
}

fn prompt_should_taste_test() -> Result<bool, UploadTestError> {
    let selected = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Should the test be only submitted if it works with the reference compiler?")
        .interact_opt();

    let Ok(Some(should_taste_test)) = selected else {
        return Err(UserAbortSnafu {
            what: "test tasting confirmation",
        }
        .into_error(NoneError));
    };

    Ok(should_taste_test)
}

fn print_finished_test(test: FinishedTest) {
    match test.output {
        ExecutionOutput::Aborted(e) => {
            error!("The test execution was {}", style("aborted").dim());
            error!("Stdout:\n{}", style(e.stdout));
            error!("Stderr:\n{}", style(e.stderr).red());
        }
        ExecutionOutput::Error(e) => {
            error!("Crow encountered an {}", style("internal error").red());
            error!("\n{}", style(e.message).red());
        }
        ExecutionOutput::Success(_) => {}
        ExecutionOutput::Failure(f) => {
            error!(
                "Your test {} on the reference compiler",
                style("failed").yellow()
            );
            error!("Stdout:\n{}", style(f.stdout));
            error!("Stderr:\n{}", style(color_diff(f.stderr)).red());
        }
        ExecutionOutput::Timeout(t) => {
            error!("Your test {}", style("timed out").red());
            error!("Stdout:\n{}", style(t.stdout));
            error!("Stderr:\n{}", style(t.stderr).red());
        }
    }
}
