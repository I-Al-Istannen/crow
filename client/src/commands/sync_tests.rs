use crate::context::{CliContext, CliContextError, Test};
use crate::error::{ContextSnafu, CrowClientError, SyncTestsSnafu};
use crate::util::st;
use clap::Args;
use console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use sha2::{Digest, Sha256};
use snafu::{ensure, IntoError, Location, NoneError, OptionExt, Report, ResultExt, Snafu};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, error, info};
use walkdir::WalkDir;

#[derive(Debug, Snafu)]
pub enum SyncTestsError {
    #[snafu(display("The test directory `{}` is a file at {location}", test_dir.display()))]
    TestDirIsFile {
        test_dir: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not create the test directory `{}` at {location}", test_dir.display()))]
    TestDirCreate {
        test_dir: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("You aborted the test directory creation at {location}"))]
    TestDirCreateAborted {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not run git command to initialize test repo at {location}"))]
    GitInitSpawn {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Git command to initialize test repo failed with {code} at {location}:\n`{stderr}`"
    ))]
    GitInit {
        code: i32,
        stderr: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not execute git to check the test dir status at {location}"))]
    GitStatusSpawn {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not execute git to commit the test dir at {location}"))]
    GitCommitSpawn {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Git command to commit test repo failed with exit code {code} at {location}:\n{stdout}\n{stderr}"
    ))]
    GitCommit {
        code: i32,
        stdout: String,
        stderr: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Could not open the test directory `{}` at {location}", test_dir.display())
    )]
    TestDirOpen {
        test_dir: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not read a test directory entry at {location}"))]
    TestDirRead {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "The test category `{}` is not valid unicode at {location}", category.display()
    ))]
    TestCategoryNotUnicode {
        category: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not read test input file `{}` at {location}", input.display()))]
    ReadInput {
        input: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not read test expected file `{}` at {location}", expected.display()))]
    ReadExpected {
        expected: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not read meta file `{}` at {location}", meta_file.display()))]
    ReadMeta {
        meta_file: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("The meta file `{}` is invalid at {location}", meta_file.display()))]
    MetaMalformed {
        meta_file: PathBuf,
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Test id in meta file (`{test_id}`) does not match file name (`{file_name}`) at {location}"
    ))]
    TestMetaNameMismatch {
        test_id: String,
        file_name: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not create directory for category `{}` at {location}"))]
    CreateCategoryDir {
        category: String,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not download test detail for `{test_id}` at {location}"))]
    FetchTestDetail {
        test_id: String,
        source: CliContextError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not write test `{test_id}` at {location}"))]
    WriteTest {
        test_id: String,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not delete test file `{}` at {location}", file.display()))]
    DeleteTestFile {
        file: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not walk test directory at {location}"))]
    TestDirWalk {
        source: walkdir::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Args, Debug)]
pub struct CliSyncTestsArgs {
    /// The directory containing all tests
    #[clap(long = "test-dir", short = 'd')]
    test_dir: PathBuf,
}

pub fn command_sync_tests(args: CliSyncTestsArgs, ctx: CliContext) -> Result<(), CrowClientError> {
    let test_dir = args.test_dir;

    if let Err(e) = commit_if_dirty(&test_dir, "backup before sync") {
        error!("{}", style(Report::from_error(e)).red());
    }

    let remote = ctx.get_remote_tests().context(ContextSnafu)?;
    let local = get_local_tests(&test_dir).context(SyncTestsSnafu)?;

    create_category_dirs(&test_dir, &remote.categories).context(SyncTestsSnafu)?;

    let remote_only: Vec<&Test> = get_remote_only_tests(&remote.tests, &local);
    if !remote_only.is_empty() {
        info!(
            "{} {} test{}",
            style(remote_only.len()).green().bold(),
            style("new").green(),
            if remote_only.len() == 1 { "" } else { "s" }
        );
        for test in remote_only {
            download_remote_test(&test_dir, test, &ctx).context(SyncTestsSnafu)?;
        }
    }

    let remote_changed: Vec<&Test> = get_remote_changed_tests(&remote.tests, &local);
    if !remote_changed.is_empty() {
        info!(
            "{} {} remote test{}",
            style(remote_changed.len()).magenta().bold(),
            style("changed").magenta(),
            if remote_changed.len() == 1 { "" } else { "s" }
        );
        for test in remote_changed {
            download_remote_test(&test_dir, test, &ctx).context(SyncTestsSnafu)?;
        }
    }

    let inconsistent = get_locally_inconsistent_tests(&test_dir, &local)
        .context(SyncTestsSnafu)?
        .into_iter()
        .filter(|test| {
            remote.tests.iter().any(|remote| {
                remote.id == test.id && remote.category == test.category && remote.hash == test.hash
            })
        })
        .collect::<Vec<_>>();
    if !inconsistent.is_empty() {
        info!(
            "{} {} test{}",
            style(inconsistent.len()).red().bold(),
            style("locally inconsistent").red().underlined(),
            if inconsistent.len() == 1 { "" } else { "s" }
        );
        for test in inconsistent {
            download_remote_test(&test_dir, test, &ctx).context(SyncTestsSnafu)?;
        }
    }

    delete_local_only_tests(&test_dir, &remote.tests).context(SyncTestsSnafu)?;

    commit_if_dirty(&test_dir, "sync tests").context(SyncTestsSnafu)
}

fn commit_if_dirty(test_dir: &Path, commit_message: &'static str) -> Result<(), SyncTestsError> {
    if !test_dir.join(".git").exists() {
        debug!("Test directory is not a git repository");
        return Ok(());
    }

    let res = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(test_dir)
        .output()
        .context(GitStatusSpawnSnafu)?;

    let stdout = String::from_utf8_lossy(&res.stdout);
    if stdout.trim().is_empty() {
        return Ok(());
    }

    debug!(
        stdout = %stdout,
        stderr = %String::from_utf8_lossy(&res.stderr),
        "Test directory is dirty"
    );

    run_git_command(test_dir, &["add", "-A"])?;
    run_git_command(
        test_dir,
        &[
            "commit",
            "-a",
            "-m",
            &format!("crow-client: {commit_message}"),
        ],
    )?;

    info!("Committed changes to test directory");

    Ok(())
}

fn run_git_command(test_dir: &Path, command: &[&str]) -> Result<(), SyncTestsError> {
    let res = Command::new("git")
        .args(command)
        .current_dir(test_dir)
        .output()
        .context(GitCommitSpawnSnafu)?;

    if !res.status.success() {
        return Err(GitCommitSnafu {
            code: res.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&res.stdout).to_string().trim(),
            stderr: String::from_utf8_lossy(&res.stderr).to_string().trim(),
        }
        .into_error(NoneError));
    }

    Ok(())
}

pub fn get_local_tests(test_dir: &Path) -> Result<Vec<Test>, SyncTestsError> {
    if !test_dir.exists() {
        create_test_dir(test_dir)?;
        initialize_git_repo(test_dir)?;
    }
    ensure!(
        test_dir.is_dir(),
        TestDirIsFileSnafu {
            test_dir: test_dir.to_path_buf()
        }
    );

    let mut tests = Vec::new();

    for category in get_local_categories(test_dir)? {
        tests.extend(get_tests_in_category(&category)?)
    }

    Ok(tests)
}

fn create_test_dir(test_dir: &Path) -> Result<(), SyncTestsError> {
    let res = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to create the test directory?")
        .interact();
    if res.is_err() || !res.unwrap() {
        return Err(TestDirCreateAbortedSnafu.into_error(NoneError));
    }
    std::fs::create_dir_all(test_dir).context(TestDirCreateSnafu {
        test_dir: test_dir.to_path_buf(),
    })?;
    info!("Created test directory");
    Ok(())
}

fn initialize_git_repo(test_dir: &Path) -> Result<(), SyncTestsError> {
    let res = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to initialize a git repository updated by crow-client?")
        .interact();

    if let Ok(true) = res {
        let output = Command::new("git")
            .arg("init")
            .current_dir(test_dir)
            .output()
            .context(GitInitSpawnSnafu)?;
        if !output.status.success() {
            return Err(GitInitSnafu {
                code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            }
            .into_error(NoneError));
        }

        info!("Initialized git repository");
    }

    Ok(())
}

fn get_local_categories(test_dir: &Path) -> Result<Vec<PathBuf>, SyncTestsError> {
    let mut categories = Vec::new();

    let dir = test_dir.read_dir().context(TestDirOpenSnafu {
        test_dir: test_dir.to_path_buf(),
    })?;
    for entry in dir {
        let entry = entry.context(TestDirReadSnafu)?;
        if entry.path().is_dir() {
            // Found a category
            categories.push(entry.path());
        }
    }

    Ok(categories)
}

fn get_tests_in_category(category: &Path) -> Result<Vec<Test>, SyncTestsError> {
    let mut tests = Vec::new();
    let category_name = category
        .file_name()
        .expect("called with .. as arg")
        .to_str()
        .context(TestCategoryNotUnicodeSnafu {
            category: category.to_path_buf(),
        })?;

    let dir = category.read_dir().context(TestDirOpenSnafu {
        test_dir: category.to_path_buf(),
    })?;
    for entry in dir {
        let path = entry.context(TestDirReadSnafu)?.path();
        let extension = path.extension().and_then(|it| it.to_str());

        if path.is_file() && extension == Some("crow-test") {
            tests.push(parse_test(category_name, &path)?);
        }
    }

    Ok(tests)
}

fn parse_test(category: &str, test_file: &Path) -> Result<Test, SyncTestsError> {
    let meta_file = test_file.with_extension("crow-test.meta");
    let meta = std::fs::read_to_string(&meta_file).context(ReadMetaSnafu {
        meta_file: meta_file.clone(),
    })?;

    let mut test: Test = serde_json::from_str(&meta).context(MetaMalformedSnafu {
        meta_file: meta_file.clone(),
    })?;
    test.category = category.to_string();

    // Verify the file name
    let file_name = test_file
        .file_stem()
        .and_then(|it| it.to_str())
        .unwrap_or("n/a");
    ensure!(
        test.id == file_name,
        TestMetaNameMismatchSnafu {
            test_id: test.id.clone(),
            file_name: file_name.to_string(),
        }
    );

    Ok(test)
}

fn create_category_dirs(test_dir: &Path, categories: &[String]) -> Result<(), SyncTestsError> {
    for category in categories {
        let path = test_dir.join(category);
        if !path.exists() {
            info!(
                "{}",
                st("Creating directory for category ").append(style(category).cyan())
            );
            std::fs::create_dir(path).context(CreateCategoryDirSnafu { category })?;
        }
    }
    Ok(())
}

fn download_remote_test(
    test_dir: &Path,
    test: &Test,
    context: &CliContext,
) -> Result<(), SyncTestsError> {
    info!(
        "{}",
        st("  Downloading `")
            .append(style(&test.id).bold().green())
            .append("`  ")
            .append(style(&test.category).dim().green())
    );
    let test_dir = test_dir.join(&test.category);

    let detail = context
        .get_test_detail(&test.id)
        .context(FetchTestDetailSnafu {
            test_id: test.id.clone(),
        })?;

    let test_path = test_dir.join(format!("{}.crow-test", test.id));
    let meta_path = test_dir.join(format!("{}.crow-test.meta", test.id));
    let expected_path = test_dir.join(format!("{}.crow-test.expected", test.id));

    std::fs::write(test_path, &detail.input).context(WriteTestSnafu {
        test_id: test.id.clone(),
    })?;
    std::fs::write(expected_path, &detail.expected_output).context(WriteTestSnafu {
        test_id: test.id.clone(),
    })?;
    std::fs::write(
        meta_path,
        serde_json::to_string_pretty(test).expect("test should serialize"),
    )
    .context(WriteTestSnafu {
        test_id: test.id.clone(),
    })?;

    Ok(())
}

fn get_remote_only_tests<'a>(remote: &'a [Test], local: &[Test]) -> Vec<&'a Test> {
    remote
        .iter()
        .filter(|remote| {
            !local
                .iter()
                .any(|local| remote.id == local.id && remote.category == local.category)
        })
        .collect()
}

fn get_remote_changed_tests<'a>(remote: &'a [Test], local: &[Test]) -> Vec<&'a Test> {
    remote
        .iter()
        .filter(|remote| {
            local
                .iter()
                .find(|local| remote.id == local.id && remote.category == local.category)
                .map(|local| local.hash != remote.hash)
                .unwrap_or(false)
        })
        .collect()
}

fn get_locally_inconsistent_tests<'a>(
    test_dir: &'_ Path,
    local: &'a [Test],
) -> Result<Vec<&'a Test>, SyncTestsError> {
    let mut inconsistent = Vec::new();

    for test in local {
        let expected_hash = &test.hash;

        let input = test.path_input(test_dir);
        if !input.exists() {
            continue;
        }
        let input = std::fs::read_to_string(&input).context(ReadInputSnafu { input })?;

        let expected = test.path_expected(test_dir);
        let expected =
            std::fs::read_to_string(&expected).context(ReadExpectedSnafu { expected })?;

        let mut actual_hash = Sha256::new();
        actual_hash.update(expected.as_bytes());
        actual_hash.update(input.as_bytes());
        actual_hash.update(test.creator_id.to_string().as_bytes());
        actual_hash.update([test.admin_authored as u8]);
        actual_hash.update(test.category.as_bytes());
        let actual_hash = format!("{:x}", actual_hash.finalize());

        if expected_hash != &actual_hash {
            inconsistent.push(test);
        }
    }

    Ok(inconsistent)
}

fn delete_local_only_tests(test_dir: &Path, remote_tests: &[Test]) -> Result<(), SyncTestsError> {
    let expected_files = remote_tests
        .iter()
        .flat_map(|test| test.local_file_paths(test_dir))
        .collect::<HashSet<_>>();

    let mut to_remove: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(test_dir).max_depth(2) {
        let entry = entry.context(TestDirWalkSnafu)?;

        if expected_files.contains(entry.path()) || !entry.path().is_file() {
            continue;
        }

        let hidden = entry
            .path()
            .components()
            .filter_map(|it| it.as_os_str().to_str())
            .any(|it| it.starts_with("."));
        if hidden {
            debug!("Skipping hidden file: {}", entry.path().display());
            continue;
        }

        if !entry.file_name().to_string_lossy().contains(".crow-test") {
            debug!("Skipping non-test file: {}", entry.path().display());
            continue;
        }

        to_remove.push(entry.path().to_path_buf());
    }

    if !to_remove.is_empty() {
        info!(
            "{} {} file{} missing on remote",
            style(to_remove.len()).bold().red(),
            style("local-only").red(),
            if to_remove.len() == 1 { "" } else { "s" }
        );

        for file in to_remove {
            info!("  {}", style(file.display()).dim().red());
            std::fs::remove_file(&file).context(DeleteTestFileSnafu { file })?;
        }
    }

    Ok(())
}
