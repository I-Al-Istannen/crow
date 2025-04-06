use super::auth::AuthError;
use super::commands::sync_tests::SyncTestsError;
use super::context::CliContextError;
use crate::commands::run_test::RunTestError;
use crate::commands::upload::UploadTestError;
use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum CrowClientError {
    #[snafu(display("Authentication error at {location}"))]
    Auth {
        source: AuthError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error in crow-client context at {location}"))]
    Context {
        source: CliContextError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error syncing tests at {location}"))]
    SyncTests {
        source: SyncTestsError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error running a test at {location}"))]
    RunTest {
        source: RunTestError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error creating a temporary directory at {location}"))]
    Tempdir {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error uploading a test at {location}"))]
    UploadTest {
        source: UploadTestError,
        #[snafu(implicit)]
        location: Location,
    },
}

pub type Result<T> = std::result::Result<T, CrowClientError>;
