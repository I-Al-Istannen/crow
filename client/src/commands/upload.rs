use crate::context::{CliContext, CliContextError, RemoteTests};
use crate::error::{ContextSnafu, CrowClientError, UploadTestSnafu};
use clap::Args;
use console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{FuzzySelect, Input, Select};
use snafu::{IntoError, Location, NoneError, ResultExt, Snafu};
use std::path::PathBuf;
use tracing::info;

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
}

pub fn command_upload_test(
    args: CliUploadTestArgs,
    ctx: CliContext,
) -> Result<(), CrowClientError> {
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

    ctx.upload_test(&name, &category, &input, &output)
        .context(UploadingSnafu)
        .context(UploadTestSnafu)?;

    info!("Test uploaded {}", style("successfully").green().bright());

    Ok(())
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
