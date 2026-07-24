//! Binary entry point for the `quickstarter` CLI.

use std::process::ExitCode;

use quickstarter::app::{self, Args};
use quickstarter::template::DEFAULT_TEMPLATE_REPO;

fn main() -> ExitCode {
    let args = match parse_args() {
        ParsedArgs::Run(args) => args,
        ParsedArgs::Help => {
            print_help();
            return ExitCode::SUCCESS;
        }
    };

    match app::run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("\nError: {error:#}");
            ExitCode::FAILURE
        }
    }
}

enum ParsedArgs {
    Run(Args),
    Help,
}

/// Parses the small set of supported flags: `--repo <url>` to override the
/// template source and `-h`/`--help`.
fn parse_args() -> ParsedArgs {
    let mut args = Args::default();
    let mut iter = std::env::args().skip(1);
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "-h" | "--help" => return ParsedArgs::Help,
            "--repo" => {
                if let Some(url) = iter.next() {
                    args.repo_url = url;
                }
            }
            _ => {}
        }
    }
    ParsedArgs::Run(args)
}

fn print_help() {
    println!("quickstarter - build a new front-end project from composable template layers\n");
    println!("USAGE:");
    println!("    quickstarter [--repo <url>]\n");
    println!("OPTIONS:");
    println!("    --repo <url>   Template repository to clone");
    println!("                   (default: {DEFAULT_TEMPLATE_REPO})");
    println!("    -h, --help     Show this help");
}
