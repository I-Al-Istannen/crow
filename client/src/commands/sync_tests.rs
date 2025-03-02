use crate::context::{CliContext, CliContextError, Test};
use crate::error::{ContextSnafu, CrowClientError, SyncTestsSnafu};
use clap::Args;
use snafu::{ensure, Location, OptionExt, ResultExt, Snafu};
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug, Snafu)]
pub enum SyncTestsError {
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
        "The test directory `{}` could not be opened at {location}", test_dir.display())
    )]
    TestDirOpen {
        test_dir: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Reading of a test directory entry failed at {location}"))]
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
    #[snafu(display("The meta file `{}` could not be read at {location}", meta_file.display()))]
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
        "The meta file `{}` for test `{}` contains category `{actual_category}` but was in the \
        folder for `{expected_category}` at {location}",
        meta_file.display(),
        test_file.display(),
    ))]
    TestCategoryMismatch {
        test_file: PathBuf,
        meta_file: PathBuf,
        expected_category: String,
        actual_category: String,
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
    #[snafu(display("Failed to write test `{test_id}` at {location}"))]
    WriteTest {
        test_id: String,
        source: std::io::Error,
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
    info!("Syncing tests!");
    let test_dir = args.test_dir;

    let remote = ctx.get_remote_tests().context(ContextSnafu)?;

    let local = get_local_tests(&test_dir).context(SyncTestsSnafu)?;

    info!("Ensuring category directories exist");
    create_category_dirs(&test_dir, &remote.categories).context(SyncTestsSnafu)?;

    let remote_only: Vec<&Test> = get_remote_only_tests(&remote.tests, &local);
    if !remote_only.is_empty() {
        info!("Downloading {} missing remote tests", remote_only.len());
        for test in remote_only {
            download_remote_test(&test_dir, test, &ctx).context(SyncTestsSnafu)?;
        }
    }

    let remote_changed: Vec<&Test> = get_remote_changed_tests(&remote.tests, &local);
    if !remote_changed.is_empty() {
        info!("Downloading {} changed remote tests", remote_changed.len());
        for test in remote_changed {
            download_remote_test(&test_dir, test, &ctx).context(SyncTestsSnafu)?;
        }
    }

    Ok(())
}

pub fn get_local_tests(test_dir: &Path) -> Result<Vec<Test>, SyncTestsError> {
    ensure!(
        test_dir.exists(),
        TestDirMissingSnafu {
            test_dir: test_dir.to_path_buf()
        }
    );
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

    let test: Test = serde_json::from_str(&meta).context(MetaMalformedSnafu {
        meta_file: meta_file.clone(),
    })?;

    ensure!(
        test.category == category,
        TestCategoryMismatchSnafu {
            test_file,
            meta_file,
            expected_category: category,
            actual_category: test.category
        }
    );

    Ok(test)
}

fn create_category_dirs(test_dir: &Path, categories: &[String]) -> Result<(), SyncTestsError> {
    for category in categories {
        let path = test_dir.join(category);
        if !path.exists() {
            info!("  Creating dir `{}`", path.display());
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
    info!("  Downloading `{}`", test.id);
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
        .filter(|remote| !local.iter().any(|local| remote.id == local.id))
        .collect()
}

fn get_remote_changed_tests<'a>(remote: &'a [Test], local: &[Test]) -> Vec<&'a Test> {
    remote
        .iter()
        .filter(|remote| {
            local
                .iter()
                .find(|local| remote.id == local.id)
                .map(|local| local.hash != remote.hash)
                .unwrap_or(false)
        })
        .collect()
}
