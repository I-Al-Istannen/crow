use crate::commands::sync_tests::get_local_tests;
use crate::error::{CrowClientError, RunTestSnafu, SyncTestsSnafu};
use clap::Args;
use console::style;
use similar::{DiffableStr, TextDiff};
use snafu::{ensure, IntoError, Location, NoneError, ResultExt, Snafu};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;
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
    #[snafu(display("Test expectation for `{test_id}` is missing at {location}"))]
    TestExpectationMissing {
        test_id: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Compiler invocation failed at {location}"))]
    CompilerInvoke {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Compiler failed with status {status} at {location}"))]
    CompilerFailed {
        status: i32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read expectation file at {location}"))]
    ReadExpectationFile {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to create input file for external diff command at {location}"))]
    ExternalDiffFileCreate {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to invoke external diff command at {location}"))]
    ExternalDiffInvoke {
        source: std::io::Error,
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
    /// The diff program to use for comparing output. If omitted, the internal diff will be used.
    /// The diff program must exit with 1 if the files differ and print to stdout.
    #[clap(long = "diff-tool")]
    diff_tool: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct CliRunTestsArgs {
    /// The directory containing all tests
    #[clap(long = "test-dir", short = 'd')]
    test_dir: PathBuf,
    /// The run binary for your compiler
    #[clap(long = "compiler-run", short = 'c')]
    compiler_run: PathBuf,
    /// The diff program to use for comparing output. If omitted, the internal diff will be used.
    /// The diff program must exit with 1 if the files differ and print to stdout.
    #[clap(long = "diff-tool")]
    diff_tool: Option<PathBuf>,
}

pub fn command_run_tests(args: CliRunTestsArgs) -> Result<(), CrowClientError> {
    let tests = get_local_tests(&args.test_dir).context(SyncTestsSnafu)?;

    let mut failures = 0;
    let mut errors = 0;
    let mut successes = 0;
    let separator_width = 80;

    for test in tests {
        let remaining_padding = separator_width - test.id.len() - 2;
        println!(
            "{} {} {}",
            style("=".repeat(remaining_padding / 2)).dim(),
            style(&test.id).bold().bright().cyan(),
            style("=".repeat((remaining_padding as f32 / 2.0).ceil() as usize)).dim(),
        );
        let res = run_test(CliRunTestArgs {
            test_dir: args.test_dir.clone(),
            test_id: test.id,
            compiler_run: args.compiler_run.clone(),
            diff_tool: args.diff_tool.clone(),
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
                error!("\n{}", err);
            }
        }
    }

    println!("{}", style("=".repeat(separator_width)).dim());

    info!(
        "{}{}{}{}{}{}{}",
        style("Tests finished. ").bright().cyan(),
        style(successes).green(),
        style(" passed, ").bright().cyan(),
        style(failures).yellow(),
        style(" failed, ").bright().cyan(),
        style(errors).red(),
        style(" errored.").bright().cyan()
    );

    if failures > 0 || errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}

pub fn command_run_test(args: CliRunTestArgs) -> Result<(), CrowClientError> {
    let res = run_test(args)?;

    if !res {
        std::process::exit(1);
    }

    Ok(())
}

pub fn run_test(args: CliRunTestArgs) -> Result<bool, CrowClientError> {
    verify_test_dir(&args).context(RunTestSnafu)?;

    let (test, expected) = find_test(&args.test_dir, &args.test_id).context(RunTestSnafu)?;
    let actual = run_compiler(&args.compiler_run, &test).context(RunTestSnafu)?;

    let success = match compute_diff(expected, actual, args.diff_tool).context(RunTestSnafu)? {
        None => {
            info!("{}", style("Test passed!").bright().bold().green());
            true
        }
        Some(diff) => {
            error!(
                "{}`{}`{}",
                style("Test ").bright().red(),
                style(args.test_id).bold().red(),
                style(" failed!").bright().red()
            );
            error!("\n{}", diff);
            false
        }
    };

    Ok(success)
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

fn find_test(test_dir: &Path, test_id: &str) -> Result<(PathBuf, String), RunTestError> {
    let target_file_name = format!("{}.crow-test", test_id);
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

    let expectation_file = test_file.with_extension("crow-test.expected");

    ensure!(
        expectation_file.exists(),
        TestExpectationMissingSnafu {
            test_id: test_id.to_string(),
        }
    );

    let expected_output =
        std::fs::read_to_string(&expectation_file).context(ReadExpectationFileSnafu)?;

    Ok((test_file, expected_output))
}

fn run_compiler(compiler_run: &Path, test: &Path) -> Result<String, RunTestError> {
    info!(
        "Running {}",
        style(format!("'{}' '{}'", compiler_run.display(), test.display())).cyan()
    );

    let output = Command::new(compiler_run)
        .arg(test)
        .output()
        .context(CompilerInvokeSnafu)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        error!(
            "{}{}",
            style("Compiler failed with status ").red(),
            style(format!("{}", output.status)).bright().red()
        );
        error!("{}", style("Stdout follows:").red());
        error!("{}", stdout);
        error!("{}", style("Stderr follows:").red());
        error!("{}", stderr);
    }

    ensure!(
        output.status.success(),
        CompilerFailedSnafu {
            status: output.status.code().unwrap_or(-1),
        }
    );

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn compute_diff(
    mut expected: String,
    mut actual: String,
    diff_tool: Option<PathBuf>,
) -> Result<Option<String>, RunTestError> {
    if !expected.ends_with_newline() {
        expected.push('\n');
    }
    if !actual.ends_with_newline() {
        actual.push('\n');
    }

    Ok(match diff_tool {
        Some(diff_program) => judge_output_diff_driver(expected, actual, diff_program)?,
        None => judge_output_internal(&expected, &actual),
    })
}

fn judge_output_internal(expected: &str, actual: &str) -> Option<String> {
    if expected == actual {
        return None;
    }

    let diff = TextDiff::from_lines(expected, actual);
    let mut diff = diff.unified_diff();
    let diff = diff
        .context_radius(5)
        .header("missing from yours", "extraneous in yours");

    let diff = diff
        .to_string()
        .lines()
        .map(|line| {
            if line.starts_with("-") {
                format!("{}", style(line).red())
            } else if line.starts_with("+") {
                format!("{}", style(line).green())
            } else if line.starts_with("@") {
                format!("{}", style(line).magenta())
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>();

    Some(diff.join("\n"))
}

fn judge_output_diff_driver(
    expected: String,
    actual: String,
    diff_tool: PathBuf,
) -> Result<Option<String>, RunTestError> {
    let expected_file = NamedTempFile::new().context(ExternalDiffFileCreateSnafu)?;
    let actual_file = NamedTempFile::new().context(ExternalDiffFileCreateSnafu)?;

    std::fs::write(expected_file.path(), expected).context(ExternalDiffFileCreateSnafu)?;
    std::fs::write(actual_file.path(), actual).context(ExternalDiffFileCreateSnafu)?;

    let result = Command::new(diff_tool)
        .arg(expected_file.path())
        .arg(actual_file.path())
        .output()
        .context(ExternalDiffInvokeSnafu)?;

    if result.status.success() {
        return Ok(None);
    }

    Ok(Some(String::from_utf8_lossy(&result.stdout).to_string()))
}
