use crate::containers::CROW_SIGNAL_SHIM_MAGIC;
use crate::AnyError;
use clap::Args;
use snafu::{location, Report};
use std::os::unix::process::ExitStatusExt;

#[derive(Args, Debug)]
pub struct CliShimArgs {
    // Pass-through arguments
    pub args: Vec<String>,
}

pub fn run_shim(args: CliShimArgs) -> Result<(), AnyError> {
    let args = args.args;
    if args.is_empty() {
        return Err(AnyError::Shim {
            msg: "No arguments, I don't know what to invoke".to_string(),
            location: location!(),
        });
    }

    let result = std::process::Command::new(&args[0])
        .args(&args[1..])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    let result = match result {
        Ok(status) => status,
        Err(e) => {
            return Err(AnyError::ShimWithSource {
                msg: format!(
                    "Failed to execute command `{}`: {}",
                    &args[0],
                    Report::from_error(&e)
                ),
                source: Box::new(e),
                location: location!(),
            });
        }
    };

    if let Some(signal) = result.signal() {
        eprintln!("{}{}", CROW_SIGNAL_SHIM_MAGIC, signal);
    }

    if let Some(code) = result.code() {
        std::process::exit(code);
    }

    std::process::exit(21);
}
