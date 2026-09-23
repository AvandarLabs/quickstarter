//! The keybinding table: one row per binding, in the order the modal shows
//! them.
//!
//! The `commands` field is the parity specification. A shortcut is a faster
//! way to reach a command, so a row that runs one names it here, and the test
//! at the bottom fails when the palette does not list it. That guard is the
//! point of this module: it is what keeps the two surfaces from drifting as
//! the UI grows.
//!
//! A binding that only moves the cursor, types, or answers a modal runs no
//! command and says so in its own description. That is the only exemption,
//! and a second test pins the list of rows that claim it.

use super::command::Command;

/// Where a binding applies. The modal shows one section per group, in
/// [`Group::ORDER`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// Keys that work on the main view.
    MainView,
    /// Keys that work while the command palette is open.
    Palette,
    /// Keys that work wherever you are.
    Anywhere,
}

impl Group {
    /// The groups, in display order.
    pub const ORDER: &'static [Self] = &[Self::MainView, Self::Palette, Self::Anywhere];

    /// The heading the shortcuts modal puts above this group.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::MainView => "Main view",
            Self::Palette => "Command palette",
            Self::Anywhere => "Anywhere",
        }
    }
}

/// One keybinding, as the modal and the footer read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    /// The keys, written the way a person says them out loud.
    pub keys: &'static str,
    /// Two or three words for the footer, where there is no room for prose.
    pub label: &'static str,
    /// The full sentence, for the modal.
    pub description: &'static str,
    /// Where the binding applies.
    pub group: Group,
    /// Whether the footer carries this one. Keep the footer to a handful: it
    /// is a reminder, not a reference.
    pub in_footer: bool,
    /// The commands this key runs. Empty means it runs none, which only
    /// movement, typing, and modal answers may claim.
    pub commands: &'static [Command],
}

/// Every keybinding, in a stable order.
pub const ALL: &[Shortcut] = &[
    Shortcut {
        keys: "Ctrl+P",
        label: "palette",
        description: "Open the command palette: every command lives there",
        group: Group::MainView,
        in_footer: true,
        // Opening the palette is not itself a command: it is the door to all
        // of them.
        commands: &[],
    },
    Shortcut {
        keys: "Ctrl+D",
        label: "details",
        description: "Show or hide the details panel beside the main pane",
        group: Group::MainView,
        in_footer: false,
        commands: &[Command::ToggleDetails],
    },
    Shortcut {
        keys: "Alt+S",
        label: "shortcuts",
        description: "Show this list of keyboard shortcuts",
        group: Group::MainView,
        in_footer: true,
        commands: &[Command::ShowShortcuts],
    },
    Shortcut {
        keys: "Up / Down",
        label: "move",
        description: "Move the palette selection, wrapping around: not a command",
        group: Group::Palette,
        in_footer: false,
        commands: &[],
    },
    Shortcut {
        keys: "Ctrl+P / Ctrl+N",
        label: "move",
        description: "The same move without leaving the home row: not a command",
        group: Group::Palette,
        in_footer: false,
        commands: &[],
    },
    Shortcut {
        keys: "Enter",
        label: "run",
        description: "Run the selected palette row: modal input, not a command",
        group: Group::Palette,
        in_footer: false,
        commands: &[],
    },
    Shortcut {
        keys: "Esc",
        label: "close",
        description: "Close whatever is open: modal input, and never a way out",
        group: Group::Anywhere,
        in_footer: false,
        commands: &[],
    },
    Shortcut {
        keys: "Ctrl+Q",
        label: "quit",
        description: "Leave the terminal UI, whatever is open",
        group: Group::Anywhere,
        in_footer: true,
        commands: &[Command::Quit],
    },
];

/// The bindings the footer reminds people of.
#[must_use]
pub fn footer() -> Vec<&'static Shortcut> {
    ALL.iter().filter(|shortcut| shortcut.in_footer).collect()
}

/// The bindings of one group, in table order.
#[must_use]
pub fn in_group(group: Group) -> Vec<&'static Shortcut> {
    ALL.iter()
        .filter(|shortcut| shortcut.group == group)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::command::CATALOG;
    use crate::tui::palette::Palette;

    #[test]
    fn every_shortcut_that_runs_something_has_a_palette_row() {
        // The invariant this whole module exists for: a key that performs an
        // action must also be reachable from the palette, so nobody has to
        // know a keystroke to find a feature. Bindings that only move the
        // cursor or answer a modal carry no command and are exempt by
        // declaration, which the next test pins down.
        let listed: Vec<Command> = Palette::from_catalog()
            .rows()
            .iter()
            .map(|row| row.command)
            .collect();

        for shortcut in ALL {
            for command in shortcut.commands {
                assert!(
                    listed.contains(command),
                    "{} runs {command:?}, which the command palette does not list",
                    shortcut.keys
                );
            }
        }
    }

    #[test]
    fn only_movement_and_modal_input_may_declare_no_command() {
        // Guards the escape hatch, so "it runs nothing" cannot quietly become
        // the way a real feature gets added without a palette row.
        let exempt: Vec<&str> = ALL
            .iter()
            .filter(|shortcut| shortcut.commands.is_empty())
            .map(|shortcut| shortcut.keys)
            .collect();

        assert_eq!(
            exempt,
            ["Ctrl+P", "Up / Down", "Ctrl+P / Ctrl+N", "Enter", "Esc"]
        );
    }

    #[test]
    fn the_hint_on_a_palette_row_names_a_real_binding() {
        // The palette advertises each command's shortcut. A hint that named a
        // key nothing is bound to would be worse than no hint at all.
        for command in CATALOG {
            let Some(hint) = command.shortcut() else {
                continue;
            };
            assert!(
                ALL.iter()
                    .any(|shortcut| shortcut.keys == hint && shortcut.commands.contains(command)),
                "{command:?} advertises {hint}, which runs something else"
            );
        }
    }

    #[test]
    fn the_footer_is_a_short_flagged_subset() {
        let footer = footer();
        assert!(!footer.is_empty());
        assert!(footer.len() <= 3, "the footer is a reminder, not a table");
        assert!(footer.iter().all(|shortcut| shortcut.in_footer));
    }

    #[test]
    fn every_binding_lands_in_exactly_one_ordered_group() {
        let grouped: usize = Group::ORDER
            .iter()
            .map(|group| in_group(*group).len())
            .sum();
        assert_eq!(grouped, ALL.len());
    }

    #[test]
    fn every_binding_says_what_it_does() {
        for shortcut in ALL {
            assert!(!shortcut.keys.is_empty());
            assert!(!shortcut.label.is_empty());
            assert!(
                shortcut.description.len() > shortcut.label.len(),
                "{} needs a real description",
                shortcut.keys
            );
        }
    }
}
