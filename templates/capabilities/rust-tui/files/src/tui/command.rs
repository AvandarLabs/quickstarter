//! The command vocabulary: everything the terminal UI can be asked to do.
//!
//! One enum, one ordered catalog, one label per row. The palette shows the
//! whole catalog, which is the rule this UI is built on: a keyboard shortcut
//! is a faster way to reach a command, never the only way. `shortcuts.rs`
//! carries the test that fails when the two surfaces drift apart.
//!
//! Adding a command is a variant, an arm in [`Command::label`], a line in
//! [`CATALOG`], and an arm in `state::run_command`. The compiler asks for
//! three of those four, and the catalog test asks for the last one.

/// One user-visible action of the terminal UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Show or hide the details panel beside the main pane.
    ToggleDetails,
    /// Open the modal that lists every keyboard shortcut.
    ShowShortcuts,
    /// Leave the terminal UI and return to the shell.
    Quit,
}

/// Every command, in the order the palette lists them. The order is the one
/// people read, so put the commands they reach for first, first.
pub const CATALOG: &[Command] = &[
    Command::ToggleDetails,
    Command::ShowShortcuts,
    Command::Quit,
];

impl Command {
    /// The wording of this command's palette row: an imperative phrase,
    /// because the row is a thing you are about to do rather than a heading.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ToggleDetails => "Show or hide the details panel",
            Self::ShowShortcuts => "Show every keyboard shortcut",
            Self::Quit => "Quit the terminal UI",
        }
    }

    /// The keyboard shortcut that runs this command, shown dim on its palette
    /// row so the palette teaches the shortcut rather than competing with it.
    /// `None` means the palette is the only way there, which is fine: the
    /// shortcuts are a subset of the commands, not the other way round.
    #[must_use]
    pub const fn shortcut(self) -> Option<&'static str> {
        match self {
            Self::ToggleDetails => Some("Ctrl+D"),
            Self::ShowShortcuts => Some("Alt+S"),
            Self::Quit => Some("Ctrl+Q"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_lists_every_command_once() {
        for command in [
            Command::ToggleDetails,
            Command::ShowShortcuts,
            Command::Quit,
        ] {
            let listed = CATALOG.iter().filter(|row| **row == command).count();
            assert_eq!(listed, 1, "{command:?} is listed {listed} times");
        }
        assert_eq!(CATALOG.len(), 3, "a new command needs a catalog entry");
    }

    #[test]
    fn every_label_is_a_distinct_phrase() {
        let mut labels: Vec<&str> = CATALOG.iter().map(|command| command.label()).collect();
        assert!(labels.iter().all(|label| !label.is_empty()));
        labels.sort_unstable();
        let total = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), total, "two commands read the same");
    }

    #[test]
    fn a_label_reads_as_an_instruction_not_a_heading() {
        // The palette is a list of things to do, so a row never ends in a full
        // stop and never starts lowercase.
        for command in CATALOG {
            let label = command.label();
            assert!(!label.ends_with('.'), "{label:?} ends like a sentence");
            assert!(
                label.starts_with(|first: char| first.is_uppercase()),
                "{label:?} does not start with a capital"
            );
        }
    }
}
