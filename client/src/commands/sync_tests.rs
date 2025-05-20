use crate::context::{CliContext, CliContextError, Test, TestDetail};
use crate::error::{ContextSnafu, CrowClientError, SyncTestsSnafu};
use crate::formats::{from_markdown, to_markdown, FormatError};
use crate::util::{infer_test_metadata_from_path, st};
use clap::Args;
use console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use sha2::{Digest, Sha256};
use snafu::{ensure, location, IntoError, Location, NoneError, Report, ResultExt, Snafu};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, error, info, warn};
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
        "Could not infer test metadata for `{}` with `{msg}` at {location}", path.display())
    )]
    InferTestMeta {
        path: PathBuf,
        msg: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not parse test file `{}` at {location}", input.display()))]
    ParseTest {
        input: PathBuf,
        source: FormatError,
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
    /// Automatically commit before and after syncing, if there were changes.
    /// This allows you to undo accidental changes.
    #[clap(short, long, default_value = "false")]
    commit_changes: bool,
}

#[derive(Debug)]
pub struct FullTest {
    pub test: Test,
    pub detail: TestDetail,
}

pub fn command_sync_tests(
    args: CliSyncTestsArgs,
    ctx: CliContext,
) -> Result<bool, CrowClientError> {
    let test_dir = args.test_dir;

    if args.commit_changes {
        if let Err(e) = commit_if_dirty(&test_dir, "backup before sync") {
            error!("{}", style(Report::from_error(e)).red());
        }
    }

    let remote = ctx.get_remote_tests().context(ContextSnafu)?;
    let local = get_local_tests(&test_dir).context(SyncTestsSnafu)?;

    create_category_dirs(&test_dir, &remote.categories.keys().collect::<Vec<_>>())
        .context(SyncTestsSnafu)?;

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
        for test in &remote_changed {
            download_remote_test(&test_dir, test, &ctx).context(SyncTestsSnafu)?;
        }
    }

    let inconsistent = get_locally_inconsistent_tests(&local)
        .context(SyncTestsSnafu)?
        .into_iter()
        .filter(|local| {
            remote.tests.iter().any(|remote| {
                remote.id == local.test.id
                    && remote.category == local.test.category
                    && remote.hash == local.test.hash
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
        for test in &inconsistent {
            let id = &test.test.id;
            let category = &test.test.category;
            let remote_test = remote
                .tests
                .iter()
                .find(|remote| &remote.id == id && &remote.category == category);

            if let Some(test) = remote_test {
                download_remote_test(&test_dir, test, &ctx).context(SyncTestsSnafu)?;
            }
        }
    }

    let deleted_any = delete_local_only_tests(&test_dir, &remote.tests).context(SyncTestsSnafu)?;

    if args.commit_changes {
        commit_if_dirty(&test_dir, "sync tests").context(SyncTestsSnafu)?;
    } else if deleted_any || !inconsistent.is_empty() || !remote_changed.is_empty() {
        warn!(
            "{}",
            st("Existing files were changed or deleted. Run with '")
                .append(style("--commit-changes").cyan())
                .append("' to automatically commit before and after a sync.")
        );
    }

    Ok(true)
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

pub fn get_local_tests(test_dir: &Path) -> Result<Vec<FullTest>, SyncTestsError> {
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

    for entry in WalkDir::new(test_dir).max_depth(2) {
        let entry = entry.context(TestDirWalkSnafu)?;
        if !entry.path().is_file() {
            continue;
        }
        if !entry.path().to_string_lossy().ends_with(".crow-test.md") {
            continue;
        }
        let test_path = entry.path();
        let category = test_path
            .parent()
            .and_then(|it| it.file_name())
            .and_then(|it| it.to_str());

        let Some(category) = category else {
            warn!(
                "Skipping test file without category: `{}`",
                entry.file_name().to_string_lossy()
            );
            continue;
        };

        tests.push(parse_test(category, test_path)?);
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

fn parse_test(category: &str, test_file: &Path) -> Result<FullTest, SyncTestsError> {
    let (_, name) =
        infer_test_metadata_from_path(test_file).map_err(|msg| SyncTestsError::InferTestMeta {
            path: test_file.to_path_buf(),
            msg,
            location: location!(),
        })?;

    let (test, detail) =
        from_markdown(test_file, category.to_string(), name).context(ParseTestSnafu {
            input: test_file.to_path_buf(),
        })?;

    Ok(FullTest { test, detail })
}

fn create_category_dirs(test_dir: &Path, categories: &[&String]) -> Result<(), SyncTestsError> {
    for &category in categories {
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

    let test_path = test_dir.join(format!("{}.crow-test.md", test.id));

    std::fs::write(test_path, to_markdown(test, &detail)).context(WriteTestSnafu {
        test_id: test.id.clone(),
    })?;

    Ok(())
}

fn get_remote_only_tests<'a>(remote: &'a [Test], local: &[FullTest]) -> Vec<&'a Test> {
    remote
        .iter()
        .filter(|remote| {
            !local
                .iter()
                .any(|local| remote.id == local.test.id && remote.category == local.test.category)
        })
        .collect()
}

fn get_remote_changed_tests<'a>(remote: &'a [Test], local: &[FullTest]) -> Vec<&'a Test> {
    remote
        .iter()
        .filter(|remote| {
            local
                .iter()
                .find(|local| remote.id == local.test.id && remote.category == local.test.category)
                .map(|local| local.test.hash != remote.hash)
                .unwrap_or(false)
        })
        .collect()
}

fn get_locally_inconsistent_tests(
    local_tests: &[FullTest],
) -> Result<Vec<&FullTest>, SyncTestsError> {
    let mut inconsistent = Vec::new();

    for local in local_tests {
        let test = &local.test;
        let detail = &local.detail;

        let compiler_modifiers = serde_json::to_string(&detail.compiler_modifiers)
            .expect("Unexpected json serialize error");
        let binary_modifiers = serde_json::to_string(&detail.binary_modifiers)
            .expect("Unexpected json serialize error");

        let mut hash = Sha256::new();
        hash.update(compiler_modifiers.as_bytes());
        hash.update(binary_modifiers.as_bytes());
        hash.update(test.creator_id.to_string().as_bytes());
        hash.update([test.admin_authored as u8]);
        hash.update([test.limited_to_category as u8]);
        hash.update(test.category.as_bytes());
        let actual_hash = format!("{:x}", hash.finalize());

        if local.test.hash != actual_hash {
            inconsistent.push(local);
        }
    }

    Ok(inconsistent)
}

fn delete_local_only_tests(test_dir: &Path, remote_tests: &[Test]) -> Result<bool, SyncTestsError> {
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

        if !entry
            .file_name()
            .to_string_lossy()
            .ends_with(".crow-test.md")
        {
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

        for file in &to_remove {
            info!("  {}", style(file.display()).dim().red());
            std::fs::remove_file(file).context(DeleteTestFileSnafu { file })?;
        }
    }

    Ok(!to_remove.is_empty())
}
