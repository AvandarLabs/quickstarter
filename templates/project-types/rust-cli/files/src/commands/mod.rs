//! Subcommand dispatch, and the narration every command shares.
//!
//! One thin module per command, each one a shell around a pure function. This
//! file only routes: it never decides anything a test would want to assert.

pub mod greet;
pub mod info;

use anyhow::Result;

use crate::cli::{Cli, Command};
use crate::theme::Theme;
use crate::{APP_NAME, tui};

/// Runs the command the arguments selected. With no subcommand, the terminal
/// UI opens: the tool is useful before you have read its help.
pub fn run(cli: &Cli, theme: Theme) -> Result<()> {
    match &cli.command {
        Some(Command::Greet { name }) => greet::run(name.as_deref(), theme),
        Some(Command::Info) => info::run(),
        Some(Command::Ui) | None => {
            narrate(theme, "Opening the terminal UI.");
            detail(
                cli.verbose,
                theme,
                "Rendering to stderr, so stdout stays pipeable.",
            );
            tui::run(APP_NAME)
        }
    }
}

/// Says what the tool is about to do, on stderr, always. A person should never
/// wonder whether the tool is hung.
pub fn narrate(theme: Theme, message: &str) {
    eprintln!("{}", theme.info(message));
}

/// Says the same thing in more detail, only under `--verbose`. Reassurance is
/// the default; detail is opt-in.
pub fn detail(verbose: bool, theme: Theme, message: &str) {
    if verbose {
        eprintln!("{}", theme.muted(message));
    }
}
