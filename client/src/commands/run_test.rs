use crate::commands::sync_tests::{get_local_tests, FullTest};
use crate::error::{CrowClientError, RunTestSnafu, SyncTestsSnafu, TempdirSnafu};
use crate::formats::{from_markdown, FormatError};
use crate::util::{infer_test_metadata_from_path, print_test_output};
use clap::Args;
use console::style;
use shared::execute::execute_test;
use shared::{execute, CompilerTest, TestExecutionOutput};
use snafu::{ensure, location, IntoError, Location, NoneError, Report, ResultExt, Snafu};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{error, info};
use walkdir::WalkDir;

#[derive(Debug, Snafu)]
pub enum RunTestError {
    #[snafu(display("The test directory `{}` is missing at {location}", test_dir.display()))]
    TestDirMissing {
        test_dir: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("The test directory `{}` is a file at {location}", test_dir.display()))]
    TestDirIsFile {
        test_dir: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not walk the test directory at {location}"))]
    DirWalk {
        source: walkdir::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not find test `{test_id}` at {location}"))]
    TestNotFound {
        test_id: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Failed to infer name and category from `{}` due to `{msg}` at {location}", path.display())
    )]
    InferMetadata {
        msg: String,
        path: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse test file `{}` at {location}", path.display()))]
    ParseTest {
        path: PathBuf,
        source: FormatError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Args, Debug)]
pub struct CliRunTestArgs {
    /// The directory containing all tests
    #[clap(long = "test-dir", short = 'd')]
    test_dir: PathBuf,
    /// The id of the test to run
    #[clap(long = "test-id", short = 'i')]
    test_id: String,
    /// The run binary for your compiler
    #[clap(long = "compiler-run", short = 'c')]
    compiler_run: PathBuf,
}

#[derive(Args, Debug)]
pub struct CliRunTestsArgs {
    /// The directory containing all tests
    #[clap(long = "test-dir", short = 'd')]
    test_dir: PathBuf,
    /// The run binary for your compiler
    #[clap(long = "compiler-run", short = 'c')]
    compiler_run: PathBuf,
}

pub fn command_run_tests(args: CliRunTestsArgs) -> Result<bool, CrowClientError> {
    let tests = get_local_tests(&args.test_dir).context(SyncTestsSnafu)?;

    let mut failures = 0;
    let mut errors = 0;
    let mut successes = 0;
    let separator_width = 80;

    for local_test in tests {
        let test = &local_test.test;
        let remaining_padding = separator_width - test.id.len() - 2;
        println!(
            "{} {} {}",
            style("=".repeat(remaining_padding / 2)).dim(),
            style(&test.id).bold().bright().cyan(),
            style("=".repeat((remaining_padding as f32 / 2.0).ceil() as usize)).dim(),
        );
        let res = command_run_test(CliRunTestArgs {
            test_dir: args.test_dir.clone(),
            test_id: test.id.clone(),
            compiler_run: args.compiler_run.clone(),
        });

        match res {
            Ok(true) => {
                successes += 1;
            }
            Ok(false) => {
                failures += 1;
            }
            Err(err) => {
                errors += 1;
                error!("\n{}", style(Report::from_error(err)).red());
            }
        }
    }

    println!("{}", style("=".repeat(separator_width)).dim());

    info!(
        "{}{}{}{}{}{}{}",
        style("Tests finished. ").bright().cyan(),
        style(format!("{} passed", successes)).green(),
        style(", ").bright().cyan(),
        style(format!("{} failed", failures)).yellow(),
        style(", ").bright().cyan(),
        style(format!("{} errored", errors)).red(),
        style(".").bright().cyan()
    );

    Ok(failures == 0 && errors == 0)
}

pub fn command_run_test(args: CliRunTestArgs) -> Result<bool, CrowClientError> {
    verify_test_dir(&args).context(RunTestSnafu)?;

    let test = find_test(&args.test_dir, &args.test_id).context(RunTestSnafu)?;

    let tempdir = tempfile::tempdir().context(TempdirSnafu)?;

    let res = execute_test(
        &CompilerTest {
            test_id: test.test.id,
            timeout: Duration::from_secs(60 * 10), // 10 minutes
            compiler_modifiers: test.detail.compiler_modifiers,
            binary_modifiers: test.detail.binary_modifiers,
            compile_command: vec![args.compiler_run.display().to_string()],
            binary_arguments: vec![],
            provisional_for_category: None,
        },
        tempdir.path(),
        &tempdir.path().join("out.🦆"),
        "./out.🦆".to_string(), // execute_locally will make this absolute when needed
        execute::execute_locally(tempdir.path().to_path_buf()),
    );

    print_test_output(&res);

    Ok(matches!(res, TestExecutionOutput::Success { .. }))
}

fn verify_test_dir(args: &CliRunTestArgs) -> Result<(), RunTestError> {
    ensure!(
        args.test_dir.exists(),
        TestDirMissingSnafu {
            test_dir: args.test_dir.to_path_buf()
        }
    );
    ensure!(
        args.test_dir.is_dir(),
        TestDirIsFileSnafu {
            test_dir: args.test_dir.to_path_buf()
        }
    );

    Ok(())
}

fn find_test(test_dir: &Path, test_id: &str) -> Result<FullTest, RunTestError> {
    let target_file_name = format!("{}.crow-test.md", test_id);
    let mut test_file = None;

    for file in WalkDir::new(test_dir).max_depth(2) {
        let file = file.context(DirWalkSnafu)?;
        let path = file.path();
        let Some(name) = path.file_name().and_then(|it| it.to_str()) else {
            continue;
        };

        if name != target_file_name {
            continue;
        }
        test_file = Some(path.to_path_buf());
        break;
    }

    let Some(test_file) = test_file else {
        return Err(TestNotFoundSnafu {
            test_id: test_id.to_string(),
        }
        .into_error(NoneError));
    };

    let (category, test_id) =
        infer_test_metadata_from_path(&test_file).map_err(|msg| RunTestError::InferMetadata {
            path: test_file.to_path_buf(),
            msg,
            location: location!(),
        })?;
    let (test, detail) = from_markdown(&test_file, category, test_id).context(ParseTestSnafu {
        path: test_file.to_path_buf(),
    })?;

    Ok(FullTest { test, detail })
}
