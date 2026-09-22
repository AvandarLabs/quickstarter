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
    about = "Build a new project from composable template layers",
    long_about = "Build a new project from composable template layers.\n\n\
                  A project has one project type (its language and product, such as\n\
                  typescript:web) and any number of capabilities layered on top.\n\n\
                  Options you leave out are asked for interactively. Pass --yes to turn\n\
                  prompting off, in which case --name and --project-type are required."
)]
pub struct Args {
    /// Name of the new project (required with --yes).
    #[arg(short, long, value_name = "NAME")]
    pub name: Option<String>,

    /// Directory to create the project in; it lands at <DIR>/<NAME> [default: .]
    #[arg(short, long, value_name = "DIR")]
    pub dir: Option<String>,

    /// Project type to build, by key, e.g. `typescript:web` (required with --yes).
    #[arg(short = 'p', long = "project-type", value_name = "KEY")]
    pub project_type: Option<String>,

    /// Capability to add, by key, e.g. `tanstack-router`. Repeatable, and
    /// accepts a comma-separated list.
    #[arg(short = 'c', long = "capability", value_name = "KEY", value_delimiter = ',')]
    pub capabilities: Vec<String>,

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
    ///
    /// A capability is not checked here: whether one is required depends on
    /// the project type, which is only known once the templates are cloned.
    pub fn ensure_answerable(&self) -> Result<()> {
        if self.interactive() {
            return Ok(());
        }
        let mut missing = Vec::new();
        if self.name.is_none() {
            missing.push("--name");
        }
        if self.project_type.is_none() {
            missing.push("--project-type");
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
        let args = parse(&[
            "--name",
            "My App",
            "--dir",
            "~/src",
            "--project-type",
            "typescript:web",
            "--capability",
            "tanstack-router",
        ]);
        assert_eq!(args.name.as_deref(), Some("My App"));
        assert_eq!(args.dir.as_deref(), Some("~/src"));
        assert_eq!(args.project_type.as_deref(), Some("typescript:web"));
        assert_eq!(args.capabilities, vec!["tanstack-router"]);
    }

    #[test]
    fn short_flags_mirror_the_long_ones() {
        let args = parse(&["-n", "My App", "-d", "~/src", "-p", "rust:cli", "-y"]);
        assert_eq!(args.name.as_deref(), Some("My App"));
        assert_eq!(args.dir.as_deref(), Some("~/src"));
        assert_eq!(args.project_type.as_deref(), Some("rust:cli"));
        assert!(args.yes);
    }

    #[test]
    fn the_capability_flag_repeats() {
        let args = parse(&["-c", "tanstack-router", "-c", "prettier"]);
        assert_eq!(args.capabilities, vec!["tanstack-router", "prettier"]);
    }

    #[test]
    fn the_capability_flag_also_takes_a_comma_separated_list() {
        let args = parse(&["--capability", "tanstack-router,prettier"]);
        assert_eq!(args.capabilities, vec!["tanstack-router", "prettier"]);
    }

    #[test]
    fn no_capability_is_no_capabilities() {
        assert!(parse(&[]).capabilities.is_empty());
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
        // --stack is gone: a project type replaced it.
        assert!(Args::try_parse_from(["quickstarter", "--stack", "router"]).is_err());
    }

    #[test]
    fn interactive_runs_need_nothing_up_front() {
        assert!(parse(&[]).ensure_answerable().is_ok());
    }

    #[test]
    fn yes_without_required_options_names_all_of_them() {
        let error = parse(&["--yes"]).ensure_answerable().unwrap_err().to_string();
        assert!(error.contains("--name and --project-type"), "{error}");
    }

    #[test]
    fn yes_with_every_required_option_is_answerable() {
        let args = parse(&["--yes", "--name", "My App", "--project-type", "rust:cli"]);
        assert!(args.ensure_answerable().is_ok());
    }
}
