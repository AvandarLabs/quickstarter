//! Binary entry point for the `quickstarter` CLI.

use std::process::ExitCode;

use clap::Parser;
use quickstarter::app;
use quickstarter::cli::Args;

fn main() -> ExitCode {
    // clap handles --help, --version, and bad usage, exiting on its own.
    let args = Args::parse();

    match app::run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("\nError: {error:#}");
            ExitCode::FAILURE
        }
    }
}
