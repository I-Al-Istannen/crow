use crate::context::{CliContext, CliContextError, RemoteTests, SetTestResponse};
use crate::error::{ContextSnafu, CrowClientError, UploadTestSnafu};
use crate::util::color_diff;
use clap::Args;
use console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, FuzzySelect, Input, Select};
use shared::{validate_test_id, ExecutionOutput, FinishedTest};
use snafu::{IntoError, Location, NoneError, OptionExt, ResultExt, Snafu};
use std::path::{Path, PathBuf};
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
    #[snafu(display("Could not infer metadata from `{}` at {location}: {msg}", path.display()))]
    MetadataInfer {
        path: PathBuf,
        msg: &'static str,
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
    /// Whether to silently infer the name and category from the input file name:
    ///   `<category>/<nam>.crow-test`
    #[clap(long)]
    infer_metadata_from_input: Option<bool>,
}

pub fn command_upload_test(
    mut args: CliUploadTestArgs,
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

    if let Some(true) = args.infer_metadata_from_input {
        info!(
            "Inferring metadata from input file `{}`",
            args.input_file.display()
        );
        let (category, name) =
            infer_metadata_from_path(&args.input_file).context(UploadTestSnafu)?;
        info!("  Inferred category=`{category}` and name=`{name}`");

        args.category = Some(category);
        args.name = Some(name);
    }

    let category = match args.category {
        Some(category) => category,
        None => prompt_test_category(&remote_tests.categories).context(UploadTestSnafu)?,
    };

    let name = match args.name {
        Some(name) => name,
        None => prompt_test_name(&category, &remote_tests, &my_team).context(UploadTestSnafu)?,
    };

    if let Err(e) = validate_test_id(&name) {
        return Err(TestNameSnafu { error: e, name }.into_error(NoneError))
            .context(UploadTestSnafu);
    }

    let should_taste_test = match args.taste_test {
        None => prompt_should_taste_test().context(UploadTestSnafu)?,
        Some(val) => val,
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

fn infer_metadata_from_path(path: &Path) -> Result<(String, String), UploadTestError> {
    let name = path
        .file_name()
        .context(MetadataInferSnafu {
            path: path.to_path_buf(),
            msg: "",
        })?
        .to_str()
        .context(MetadataInferSnafu {
            path: path.to_path_buf(),
            msg: "",
        })?;
    let name = name
        .strip_suffix(".crow-test")
        .context(MetadataInferSnafu {
            path: path.to_path_buf(),
            msg: "",
        })?
        .to_string();

    let category = path.parent().context(MetadataInferSnafu {
        path: path.to_path_buf(),
        msg: "File has no parent directory",
    })?;
    let category = category
        .file_name()
        .context(MetadataInferSnafu {
            path: path.to_path_buf(),
            msg: "Parent folder has no file name",
        })?
        .to_str()
        .context(MetadataInferSnafu {
            path: path.to_path_buf(),
            msg: "Parent folder's name is not valid unicode",
        })?
        .to_string();

    Ok((category, name))
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
            .validate_with(|input: &String| validate_test_id(input))
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
