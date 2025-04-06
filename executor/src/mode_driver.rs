use crate::{AnyError, DriverSnafu};
use shared::CompilerTest;
use snafu::{Location, ResultExt, Snafu};
use std::io::stdin;
use std::path::Path;

#[derive(Debug, Snafu)]
pub enum DriverError {
    #[snafu(display("Could not successfully execute program at {location}"))]
    Execute {
        source: shared::execute::ExecuteError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Serializing output failed at {location}"))]
    SerializeOutput {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

pub fn run_driver() -> Result<(), AnyError> {
    let test: CompilerTest = serde_json::from_reader(stdin()).unwrap();
    let result = shared::execute::execute_test(test, Path::new("out.🦆"))
        .context(ExecuteSnafu)
        .context(DriverSnafu)?;

    println!(
        "{}",
        serde_json::to_string(&result)
            .context(SerializeOutputSnafu)
            .context(DriverSnafu)?
    );

    Ok(())
}
