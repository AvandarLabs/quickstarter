//! The flag surface: every answer the scaffolder needs can be given up front.
//!
//! The contract is deliberately simple. Anything the user does not pass is
//! asked for interactively (see [`crate::cli::resolve`]), so a bare
//! `quickstarter` is a full interview. Passing `--yes` turns prompting off: the
//! options given are taken as they are, and a missing *required* one is an
//! error rather than a question.

use anyhow::{Result, bail};
use clap::Parser;

use crate::template::DEFAULT_TEMPLATE_REPO;

/// Options parsed from the command line.
#[derive(Debug, Parser)]
#[command(
    name = "quickstarter",
    version,
    about = "Build a new front-end project from composable template layers",
    long_about = "Build a new front-end project from composable template layers.\n\n\
                  Options you leave out are asked for interactively. Pass --yes to turn\n\
                  prompting off, in which case --name and --stack are required."
)]
pub struct Args {
    /// Name of the new project (required with --yes).
    #[arg(short, long, value_name = "NAME")]
    pub name: Option<String>,

    /// Directory to create the project in; it lands at <DIR>/<NAME> [default: .]
    #[arg(short, long, value_name = "DIR")]
    pub dir: Option<String>,

    /// Stack to build with, by key, e.g. `router` or `start` (required with --yes).
    #[arg(short, long, value_name = "STACK")]
    pub stack: Option<String>,

    /// Template repository to clone.
    #[arg(long, value_name = "URL", default_value = DEFAULT_TEMPLATE_REPO)]
    pub repo: String,

    /// Never prompt: use the options as given, and fail if one is missing.
    #[arg(short = 'y', long = "yes", visible_alias = "no-input")]
    pub yes: bool,
}

impl Args {
    /// Whether answers that were not passed may be asked for.
    pub fn interactive(&self) -> bool {
        !self.yes
    }

    /// Checks that the run can complete without asking anything, which matters
    /// only under `--yes`. The required options are the ones with no sensible
    /// default, and every missing one is named at once so the user can fix the
    /// command in a single edit. The check runs before any network work, so an
    /// incomplete command fails immediately.
    pub fn ensure_answerable(&self) -> Result<()> {
        if self.interactive() {
            return Ok(());
        }
        let mut missing = Vec::new();
        if self.name.is_none() {
            missing.push("--name");
        }
        if self.stack.is_none() {
            missing.push("--stack");
        }
        if missing.is_empty() {
            return Ok(());
        }
        bail!(
            "--yes turns off the questions, so {} must be passed. \
             Drop --yes to be asked instead.",
            missing.join(" and ")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(argv: &[&str]) -> Args {
        Args::try_parse_from(std::iter::once("quickstarter").chain(argv.iter().copied())).unwrap()
    }

    #[test]
    fn every_answer_has_a_long_flag() {
        let args = parse(&["--name", "My App", "--dir", "~/src", "--stack", "router"]);
        assert_eq!(args.name.as_deref(), Some("My App"));
        assert_eq!(args.dir.as_deref(), Some("~/src"));
        assert_eq!(args.stack.as_deref(), Some("router"));
    }

    #[test]
    fn short_flags_mirror_the_long_ones() {
        let args = parse(&["-n", "My App", "-d", "~/src", "-s", "start", "-y"]);
        assert_eq!(args.name.as_deref(), Some("My App"));
        assert_eq!(args.dir.as_deref(), Some("~/src"));
        assert_eq!(args.stack.as_deref(), Some("start"));
        assert!(args.yes);
    }

    #[test]
    fn defaults_to_asking_and_to_the_canonical_repo() {
        let args = parse(&[]);
        assert!(args.interactive());
        assert_eq!(args.repo, DEFAULT_TEMPLATE_REPO);
    }

    #[test]
    fn no_input_is_an_alias_for_yes() {
        assert!(parse(&["--no-input"]).yes);
    }

    #[test]
    fn unknown_flags_are_rejected() {
        assert!(Args::try_parse_from(["quickstarter", "--nope"]).is_err());
    }

    #[test]
    fn interactive_runs_need_nothing_up_front() {
        assert!(parse(&[]).ensure_answerable().is_ok());
    }

    #[test]
    fn yes_without_required_options_names_all_of_them() {
        let error = parse(&["--yes"]).ensure_answerable().unwrap_err().to_string();
        assert!(error.contains("--name and --stack"), "{error}");
    }

    #[test]
    fn yes_with_every_required_option_is_answerable() {
        let args = parse(&["--yes", "--name", "My App", "--stack", "router"]);
        assert!(args.ensure_answerable().is_ok());
    }
}
