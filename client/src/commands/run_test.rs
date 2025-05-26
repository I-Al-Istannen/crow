use crate::commands::sync_tests::{get_local_tests, FullTest};
use crate::error::{CrowClientError, RunTestSnafu, SyncTestsSnafu, TempdirSnafu};
use crate::formats::{from_markdown, FormatError};
use crate::util::{infer_test_metadata_from_path, print_test_output};
use clap::Args;
use console::style;
use jiff::{Timestamp, Unit};
use rayon::ThreadPoolBuilder;
use shared::execute::execute_test;
use shared::{CompilerTest, ExecutionOutput, FinishedExecution, TestExecutionOutput, TestModifier};
use snafu::{ensure, location, IntoError, Location, NoneError, Report, ResultExt, Snafu};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
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
    #[snafu(display(
        "The compiler run binary `{}` could not be turned into an absolute path at {location}",
        path.display())
    )]
    AbsolutizeCompiler {
        path: PathBuf,
        source: std::io::Error,
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
    /// How many tests to run in parallel. 0 for processor count.
    #[clap(long = "jobs", short = 'j', default_value = "0")]
    parallelism: usize,
    /// The category of tests to run. If not set defaults to all, but will ignore
    /// tests limited to a category that is not the latest (defined by lexical order).
    #[clap(long = "category", short = 'l')]
    category: Option<String>,
    /// Only show failing tests in the output
    #[clap(long = "only-failing", default_value = "false")]
    only_failing: bool,
}

pub fn command_run_tests(args: CliRunTestsArgs) -> Result<bool, CrowClientError> {
    let mut tests = get_local_tests(&args.test_dir).context(SyncTestsSnafu)?;
    let mut categories = tests.iter().map(|it| &it.test.category).collect::<Vec<_>>();
    categories.sort();
    let newest_category = categories.last().map(|it| it.to_string());

    if let Some(category) = &args.category {
        info!("Running only tests belonging to category `{}`", category);
        tests.retain(|it| it.test.category == *category);
    } else if let Some(newest_category) = &newest_category {
        info!(
            "No category specified, running all tests. \
            Ignoring tests limited to any category besides `{}` to prevent unwanted errors.",
            newest_category
        );
        tests.retain(|it| !it.test.limited_to_category || it.test.category == *newest_category);
    }

    let mut failures = 0;
    let mut errors = 0;
    let mut successes = 0;
    let separator_width = 80;
    let test_count = tests.len();

    let pool = ThreadPoolBuilder::new()
        .num_threads(args.parallelism)
        .build()
        .unwrap();
    let (tx, rx) = mpsc::channel();

    for local_test in tests {
        let tx = tx.clone();
        let test = local_test.test;
        let test_dir = args.test_dir.clone();
        let compiler_run = args.compiler_run.clone();

        pool.spawn(move || {
            let res = run_test(CliRunTestArgs {
                test_dir,
                test_id: test.id.clone(),
                compiler_run,
            });
            tx.send((res, test)).unwrap();
        });
    }
    // Explicitly drop the sender to close the channel
    drop(tx);

    let start = Timestamp::now();

    while let Ok(res) = rx.recv() {
        let (res, test) = res;
        let remaining_padding = separator_width - test.id.len() - 2;
        let elapsed = start.duration_until(Timestamp::now());

        let print_test = !(args.only_failing && matches!(res, Ok((true, _))));

        if print_test {
            print!(
                "{} {} {}",
                style("=".repeat(remaining_padding / 2)).dim(),
                style(&test.id).bold().bright().cyan(),
                style("=".repeat((remaining_padding as f32 / 2.0).ceil() as usize)).dim(),
            );

            println!(
                "    {} {}",
                style(format!(
                    "{}/{} completed",
                    successes + failures + errors,
                    test_count
                ))
                .bold()
                .bright()
                .cyan(),
                style(format!(
                    " ({:?} elapsed)",
                    elapsed.round(Unit::Millisecond).unwrap()
                ))
                .dim(),
            );
        }

        match res {
            Ok((true, res)) => {
                if print_test {
                    print_test_output(&res);
                }
                successes += 1;
            }
            Ok((false, res)) => {
                print_test_output(&res);
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
    let (success, res) = run_test(args)?;

    print_test_output(&res);

    Ok(success)
}

fn run_test(args: CliRunTestArgs) -> Result<(bool, TestExecutionOutput), CrowClientError> {
    verify_test_dir(&args).context(RunTestSnafu)?;

    let test = find_test(&args.test_dir, &args.test_id).context(RunTestSnafu)?;

    let tempdir = tempfile::tempdir().context(TempdirSnafu)?;
    let compiler_run_path = args
        .compiler_run
        .canonicalize()
        .context(AbsolutizeCompilerSnafu {
            path: args.compiler_run.clone(),
        })
        .context(RunTestSnafu)?;

    let res = execute_test(
        &CompilerTest {
            test_id: test.test.id,
            category: test.test.category,
            timeout: Duration::from_secs(60 * 10), // 10 minutes
            compiler_modifiers: test.detail.compiler_modifiers,
            binary_modifiers: test.detail.binary_modifiers,
            compile_command: vec![compiler_run_path.as_os_str().to_string_lossy().to_string()],
            binary_arguments: vec![],
            provisional_for_category: None,
        },
        tempdir.path(),
        &tempdir.path().join("out.🦆"),
        tempdir.path(),
        crate::util::execute_locally,
    );

    Ok((matches!(res, TestExecutionOutput::Success { .. }), res))
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
