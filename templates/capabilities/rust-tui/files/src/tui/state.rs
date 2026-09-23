//! The whole state of the terminal UI, and the one pure function that moves
//! it forward.
//!
//! [`update`] takes the state and an [`Action`] and returns the next state. It
//! touches no terminal and reads no clock, so every behavior of this UI is a
//! test that calls a function: the event loop in `mod.rs` stays a shell that
//! reads a key, classifies it, applies it, and draws.

use super::command::Command;
use super::keymap::Action;
use super::palette::Palette;

/// Which overlay, if any, is covering the main view. The keymap reads this to
/// decide what a key means, which is why it is a plain enum and not a stack of
/// booleans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overlay {
    /// Nothing is covering the main view.
    #[default]
    None,
    /// The command palette is open.
    Palette,
    /// The modal listing every keyboard shortcut is open.
    Shortcuts,
}

/// Everything the UI knows.
#[derive(Debug, Clone)]
pub struct State {
    overlay: Overlay,
    palette: Palette,
    details_visible: bool,
    status: Option<String>,
    running: bool,
}

impl State {
    /// The state the UI opens on: the main view, nothing covering it, and a
    /// palette holding every command.
    #[must_use]
    pub fn new() -> Self {
        Self {
            overlay: Overlay::None,
            palette: Palette::from_catalog(),
            details_visible: true,
            status: None,
            running: true,
        }
    }

    /// Which overlay is open.
    #[must_use]
    pub const fn overlay(&self) -> Overlay {
        self.overlay
    }

    /// The command palette, open or not.
    #[must_use]
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Whether the details panel is showing.
    #[must_use]
    pub const fn details_visible(&self) -> bool {
        self.details_visible
    }

    /// What the last command did, for the footer to report. `None` until
    /// something has happened.
    #[must_use]
    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    /// Whether the event loop should keep going.
    #[must_use]
    pub const fn running(&self) -> bool {
        self.running
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Applies one action and returns the state it leads to.
#[must_use]
pub fn update(mut state: State, action: Action) -> State {
    match action {
        Action::Ignore => {}
        Action::Run(command) => return run_command(state, command),
        Action::OpenPalette => {
            state.palette.clear();
            state.overlay = Overlay::Palette;
        }
        Action::CloseOverlay => state.overlay = Overlay::None,
        Action::RunSelected => {
            if let Some(command) = state.palette.selected_command() {
                state.overlay = Overlay::None;
                return run_command(state, command);
            }
        }
        Action::SelectPrevious => state.palette.select_previous(),
        Action::SelectNext => state.palette.select_next(),
        Action::Type(typed) => state.palette.type_character(typed),
        Action::DeleteCharacter => state.palette.delete_character(),
    }
    state
}

/// Runs one command. Every command arrives here, whether it was picked in the
/// palette or reached by its shortcut, so the two can never mean different
/// things.
fn run_command(mut state: State, command: Command) -> State {
    match command {
        Command::ToggleDetails => {
            state.details_visible = !state.details_visible;
            state.status = Some(
                if state.details_visible {
                    "Details shown."
                } else {
                    "Details hidden."
                }
                .to_owned(),
            );
        }
        Command::ShowShortcuts => state.overlay = Overlay::Shortcuts,
        Command::Quit => state.running = false,
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(state: State, actions: &[Action]) -> State {
        let mut state = state;
        for action in actions {
            state = update(state, *action);
        }
        state
    }

    #[test]
    fn the_ui_opens_on_the_main_view_and_keeps_running() {
        let state = State::new();
        assert_eq!(state.overlay(), Overlay::None);
        assert!(state.running());
        assert_eq!(state.status(), None);
    }

    #[test]
    fn an_ignored_key_changes_nothing() {
        let state = update(State::new(), Action::Ignore);
        assert_eq!(state.overlay(), Overlay::None);
        assert!(state.running());
    }

    #[test]
    fn opening_the_palette_starts_it_empty_again() {
        let state = apply(
            State::new(),
            &[
                Action::OpenPalette,
                Action::Type('q'),
                Action::CloseOverlay,
                Action::OpenPalette,
            ],
        );
        assert_eq!(state.overlay(), Overlay::Palette);
        assert_eq!(state.palette().query(), "");
        assert_eq!(state.palette().visible_rows().len(), 3);
    }

    #[test]
    fn typing_filters_the_palette_and_backspace_puts_it_back() {
        let state = apply(
            State::new(),
            &[Action::OpenPalette, Action::Type('q'), Action::Type('u')],
        );
        assert_eq!(state.palette().query(), "qu");
        assert_eq!(state.palette().visible_rows().len(), 1);
        let state = update(state, Action::DeleteCharacter);
        assert_eq!(state.palette().query(), "q");
    }

    #[test]
    fn running_the_selected_row_closes_the_palette_and_does_the_thing() {
        let state = apply(
            State::new(),
            &[
                Action::OpenPalette,
                Action::Type('d'),
                Action::Type('e'),
                Action::Type('t'),
                Action::RunSelected,
            ],
        );
        assert_eq!(state.overlay(), Overlay::None);
        assert!(!state.details_visible());
        assert_eq!(state.status(), Some("Details hidden."));
    }

    #[test]
    fn running_a_row_that_does_not_exist_leaves_the_palette_alone() {
        let state = apply(
            State::new(),
            &[Action::OpenPalette, Action::Type('z'), Action::RunSelected],
        );
        assert_eq!(state.overlay(), Overlay::Palette);
        assert!(state.running());
    }

    #[test]
    fn a_shortcut_and_a_palette_row_run_the_same_command() {
        let shortcut = update(State::new(), Action::Run(Command::ToggleDetails));
        let palette = apply(
            State::new(),
            &[
                Action::OpenPalette,
                Action::Type('d'),
                Action::Type('e'),
                Action::Type('t'),
                Action::RunSelected,
            ],
        );
        assert_eq!(shortcut.details_visible(), palette.details_visible());
        assert_eq!(shortcut.status(), palette.status());
    }

    #[test]
    fn showing_the_shortcuts_opens_the_modal_and_escape_closes_it() {
        let state = update(State::new(), Action::Run(Command::ShowShortcuts));
        assert_eq!(state.overlay(), Overlay::Shortcuts);
        let state = update(state, Action::CloseOverlay);
        assert_eq!(state.overlay(), Overlay::None);
    }

    #[test]
    fn quitting_stops_the_loop() {
        let state = update(State::new(), Action::Run(Command::Quit));
        assert!(!state.running());
    }

    #[test]
    fn toggling_the_details_twice_leaves_them_as_they_were() {
        let state = apply(
            State::new(),
            &[
                Action::Run(Command::ToggleDetails),
                Action::Run(Command::ToggleDetails),
            ],
        );
        assert!(state.details_visible());
        assert_eq!(state.status(), Some("Details shown."));
    }
}
