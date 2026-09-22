//! The command-line surface.
//!
//! Every action has a flag or a subcommand, so an agent can drive the tool
//! without ever meeting a prompt. Values a person may omit are `Option`s: the
//! command asks for them when there is a terminal to ask on. See
//! `docs/rules/cli-ux.md`.

use clap::{Parser, Subcommand};

/// {{PROJECT_NAME}}: a command-line tool with a terminal UI.
#[derive(Debug, Clone, Parser)]
#[command(version, propagate_version = true)]
pub struct Cli {
    /// Print detailed diagnostics to stderr.
    #[arg(short, long, global = true, env = "CLI_VERBOSE")]
    pub verbose: bool,

    /// The command to run. With none, the terminal UI opens.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// The things this tool can be asked to do.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Print a greeting.
    Greet {
        /// Who to greet. Left out on a terminal, you are asked for it.
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Print what this build is, one `key=value` line per fact.
    Info,
    /// Open the terminal UI.
    Ui,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_surface_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn no_subcommand_means_the_ui() {
        let cli = Cli::try_parse_from(["tool"]).expect("a bare invocation parses");
        assert_eq!(cli.command, None);
    }

    #[test]
    fn verbose_is_accepted_after_a_subcommand() {
        let cli = Cli::try_parse_from(["tool", "info", "--verbose"]).expect("global flag parses");
        assert!(cli.verbose);
        assert_eq!(cli.command, Some(Command::Info));
    }

    #[test]
    fn greet_takes_a_name_non_interactively() {
        let cli = Cli::try_parse_from(["tool", "greet", "--name", "Ada"]).expect("a name parses");
        assert_eq!(
            cli.command,
            Some(Command::Greet {
                name: Some("Ada".to_owned())
            })
        );
    }

    #[test]
    fn greet_without_a_name_still_parses() {
        let cli = Cli::try_parse_from(["tool", "greet"]).expect("an omitted name parses");
        assert_eq!(cli.command, Some(Command::Greet { name: None }));
    }

    #[test]
    fn an_unknown_subcommand_is_rejected() {
        assert!(Cli::try_parse_from(["tool", "frobnicate"]).is_err());
    }
}
