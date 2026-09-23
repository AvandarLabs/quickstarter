//! What a keystroke means. Pure: an overlay and a key event in, an action out.
//!
//! Key handling is where a terminal UI hides its bugs, so it is kept out of
//! the event loop entirely and tested here instead. What a key means depends
//! on what is open: `Ctrl+P` opens the palette from the main view and moves
//! the selection while the palette is showing, and a printable character is a
//! binding on the main view but a letter of the query in the palette.
//!
//! Adding a binding means adding an arm here and a row in `shortcuts.rs`,
//! never an `if` inside the loop.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::command::Command;
use super::state::Overlay;

/// What the UI should do about a keystroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Nothing is bound to this key in this state.
    Ignore,
    /// Run a command the key is bound to directly.
    Run(Command),
    /// Open the command palette.
    OpenPalette,
    /// Close whichever overlay is open.
    CloseOverlay,
    /// Run the palette row the selection sits on.
    RunSelected,
    /// Move the palette selection one row up.
    SelectPrevious,
    /// Move the palette selection one row down.
    SelectNext,
    /// Type one character into the palette query.
    Type(char),
    /// Delete the last character of the palette query.
    DeleteCharacter,
}

/// Decides what a keystroke means, given what is currently open.
///
/// `Ctrl+Q` is the one unconditional binding: a person who wants out should
/// never have to work out which overlay is swallowing their keys first. Key
/// releases (Windows terminals report them) are never actions, because
/// otherwise every keypress would fire twice.
#[must_use]
pub fn action_for(overlay: Overlay, key: &KeyEvent) -> Action {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return Action::Ignore;
    }
    if is_control(key, 'q') {
        return Action::Run(Command::Quit);
    }
    match overlay {
        Overlay::None => on_the_main_view(key),
        Overlay::Palette => in_the_palette(key),
        Overlay::Shortcuts => in_the_shortcuts_modal(key),
    }
}

/// The main view: every binding here is a shortcut to a command, plus the one
/// key that opens the palette holding all of them.
const fn on_the_main_view(key: &KeyEvent) -> Action {
    if is_control(key, 'p') {
        return Action::OpenPalette;
    }
    if is_control(key, 'd') {
        return Action::Run(Command::ToggleDetails);
    }
    if is_alt(key, 's') {
        return Action::Run(Command::ShowShortcuts);
    }
    Action::Ignore
}

/// The palette: the keyboard belongs to the query, so only the keys that
/// navigate or edit it are bound, and everything printable is text.
fn in_the_palette(key: &KeyEvent) -> Action {
    if is_control(key, 'p') {
        return Action::SelectPrevious;
    }
    if is_control(key, 'n') {
        return Action::SelectNext;
    }
    match key.code {
        KeyCode::Esc => Action::CloseOverlay,
        KeyCode::Enter => Action::RunSelected,
        KeyCode::Up => Action::SelectPrevious,
        KeyCode::Down => Action::SelectNext,
        KeyCode::Backspace => Action::DeleteCharacter,
        KeyCode::Char(typed) if types_text(key) => Action::Type(typed),
        _ => Action::Ignore,
    }
}

/// The shortcuts modal: it is a page to read, so it takes the keyboard and
/// gives back only the key that closes it.
const fn in_the_shortcuts_modal(key: &KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => Action::CloseOverlay,
        _ => Action::Ignore,
    }
}

/// Whether the key is `Ctrl` plus `letter`. Terminals report the letter in
/// either case depending on whether Shift was held, so both count.
const fn is_control(key: &KeyEvent, letter: char) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(typed) if typed.eq_ignore_ascii_case(&letter))
}

/// Whether the key is `Alt` plus `letter`.
const fn is_alt(key: &KeyEvent, letter: char) -> bool {
    key.modifiers.contains(KeyModifiers::ALT)
        && matches!(key.code, KeyCode::Char(typed) if typed.eq_ignore_ascii_case(&letter))
}

/// Whether a printable key is text rather than a binding. Shift is part of
/// typing a capital letter; Ctrl and Alt never are.
fn types_text(key: &KeyEvent) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn control(letter: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(letter), KeyModifiers::CONTROL)
    }

    fn alt(letter: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(letter), KeyModifiers::ALT)
    }

    #[test]
    fn control_q_quits_whatever_is_open() {
        for overlay in [Overlay::None, Overlay::Palette, Overlay::Shortcuts] {
            assert_eq!(
                action_for(overlay, &control('q')),
                Action::Run(Command::Quit),
                "{overlay:?} swallowed the quit key"
            );
        }
        assert_eq!(
            action_for(Overlay::None, &control('Q')),
            Action::Run(Command::Quit)
        );
    }

    #[test]
    fn control_p_opens_the_palette_from_the_main_view() {
        assert_eq!(
            action_for(Overlay::None, &control('p')),
            Action::OpenPalette
        );
    }

    #[test]
    fn control_p_moves_the_selection_once_the_palette_is_open() {
        assert_eq!(
            action_for(Overlay::Palette, &control('p')),
            Action::SelectPrevious
        );
        assert_eq!(
            action_for(Overlay::Palette, &control('n')),
            Action::SelectNext
        );
    }

    #[test]
    fn alt_s_shows_the_shortcuts_from_the_main_view() {
        assert_eq!(
            action_for(Overlay::None, &alt('s')),
            Action::Run(Command::ShowShortcuts)
        );
    }

    #[test]
    fn control_d_toggles_the_details_panel() {
        assert_eq!(
            action_for(Overlay::None, &control('d')),
            Action::Run(Command::ToggleDetails)
        );
    }

    #[test]
    fn escape_closes_an_overlay_and_does_nothing_otherwise() {
        assert_eq!(
            action_for(Overlay::Palette, &press(KeyCode::Esc)),
            Action::CloseOverlay
        );
        assert_eq!(
            action_for(Overlay::Shortcuts, &press(KeyCode::Esc)),
            Action::CloseOverlay
        );
        // Nothing is open, so Escape must not double as a quit key: leaving
        // is Ctrl+Q, and only Ctrl+Q.
        assert_eq!(
            action_for(Overlay::None, &press(KeyCode::Esc)),
            Action::Ignore
        );
    }

    #[test]
    fn enter_runs_the_selected_row_only_in_the_palette() {
        assert_eq!(
            action_for(Overlay::Palette, &press(KeyCode::Enter)),
            Action::RunSelected
        );
        assert_eq!(
            action_for(Overlay::None, &press(KeyCode::Enter)),
            Action::Ignore
        );
    }

    #[test]
    fn the_arrow_keys_move_the_palette_selection() {
        assert_eq!(
            action_for(Overlay::Palette, &press(KeyCode::Up)),
            Action::SelectPrevious
        );
        assert_eq!(
            action_for(Overlay::Palette, &press(KeyCode::Down)),
            Action::SelectNext
        );
    }

    #[test]
    fn a_printable_key_is_a_query_letter_in_the_palette_and_nothing_outside_it() {
        assert_eq!(
            action_for(Overlay::Palette, &press(KeyCode::Char('q'))),
            Action::Type('q')
        );
        assert_eq!(
            action_for(
                Overlay::Palette,
                &KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT)
            ),
            Action::Type('Q')
        );
        assert_eq!(
            action_for(Overlay::None, &press(KeyCode::Char('q'))),
            Action::Ignore
        );
    }

    #[test]
    fn a_modified_letter_is_never_typed_into_the_query() {
        assert_eq!(action_for(Overlay::Palette, &alt('s')), Action::Ignore);
        assert_eq!(action_for(Overlay::Palette, &control('x')), Action::Ignore);
    }

    #[test]
    fn backspace_deletes_a_query_letter() {
        assert_eq!(
            action_for(Overlay::Palette, &press(KeyCode::Backspace)),
            Action::DeleteCharacter
        );
    }

    #[test]
    fn the_shortcuts_modal_takes_the_keyboard_while_it_is_up() {
        for key in [press(KeyCode::Enter), press(KeyCode::Down), control('p')] {
            assert_eq!(
                action_for(Overlay::Shortcuts, &key),
                Action::Ignore,
                "{key:?} should be swallowed by the modal"
            );
        }
    }

    #[test]
    fn a_key_release_is_not_a_second_press() {
        let release = KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::CONTROL,
            KeyEventKind::Release,
        );
        assert_eq!(action_for(Overlay::None, &release), Action::Ignore);
    }
}
