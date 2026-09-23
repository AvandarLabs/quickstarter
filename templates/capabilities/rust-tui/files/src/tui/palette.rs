//! The command palette: a query, the rows it leaves visible, and where the
//! selection sits. Pure state, with no terminal anywhere in sight.
//!
//! Filtering is **word atoms**: every whitespace-separated word of the query
//! has to appear somewhere in the row, in any order. "open help" and "help
//! open" therefore find the same row, which is what people expect from a
//! palette and what a plain substring match gets wrong.

use super::command::{CATALOG, Command};

/// One row of the palette: the stable number it was given, the wording, the
/// command it runs, and the shortcut hint shown dim beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The 1-based position in the catalog. It never changes as the query
    /// narrows the list, so the number a person learns keeps pointing at the
    /// same command.
    pub number: usize,
    /// What the row says.
    pub label: String,
    /// What running the row does.
    pub command: Command,
    /// The keyboard shortcut for the same command, when it has one.
    pub shortcut: Option<&'static str>,
}

impl Row {
    /// The row as one searchable, displayable line: `"3. Quit the terminal
    /// UI"`. The number is part of the text on purpose, so typing a digit
    /// narrows the list to that row.
    #[must_use]
    pub fn text(&self) -> String {
        format!("{}. {}", self.number, self.label)
    }
}

/// Whether a row matches a query.
///
/// Every whitespace-separated word of the query must appear somewhere in the
/// text, in any order and in any case. An empty query matches everything,
/// because it has no words to satisfy.
#[must_use]
pub fn matches_query(query: &str, text: &str) -> bool {
    let text = text.to_lowercase();
    query
        .split_whitespace()
        .all(|word| text.contains(&word.to_lowercase()))
}

/// The palette's whole state: what was typed, what survives it, and which of
/// the survivors is selected.
#[derive(Debug, Clone)]
pub struct Palette {
    query: String,
    rows: Vec<Row>,
    visible: Vec<usize>,
    selected: usize,
}

impl Palette {
    /// A palette over `rows`, numbering them in the order they arrive.
    #[must_use]
    pub fn new(rows: Vec<Row>) -> Self {
        let mut rows = rows;
        for (index, row) in rows.iter_mut().enumerate() {
            row.number = index + 1;
        }
        let visible = (0..rows.len()).collect();
        Self {
            query: String::new(),
            rows,
            visible,
            selected: 0,
        }
    }

    /// The palette every command in [`CATALOG`] belongs to. This is the only
    /// palette the UI builds: the catalog is the parent set of the shortcuts,
    /// so nothing is reachable by key alone.
    #[must_use]
    pub fn from_catalog() -> Self {
        Self::new(
            CATALOG
                .iter()
                .map(|command| Row {
                    number: 0,
                    label: command.label().to_owned(),
                    command: *command,
                    shortcut: command.shortcut(),
                })
                .collect(),
        )
    }

    /// What has been typed so far.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Every row, filtered or not.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The rows the query leaves, in catalog order.
    #[must_use]
    pub fn visible_rows(&self) -> Vec<&Row> {
        self.visible
            .iter()
            .map(|&index| &self.rows[index])
            .collect()
    }

    /// Where the selection sits among the visible rows.
    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// The command the selection would run, or `None` when the query matched
    /// nothing.
    #[must_use]
    pub fn selected_command(&self) -> Option<Command> {
        self.visible
            .get(self.selected)
            .map(|&index| self.rows[index].command)
    }

    /// Types one character into the query.
    pub fn type_character(&mut self, typed: char) {
        self.query.push(typed);
        self.refilter();
    }

    /// Deletes the last character of the query.
    pub fn delete_character(&mut self) {
        self.query.pop();
        self.refilter();
    }

    /// Forgets the query and the selection, as reopening the palette does. A
    /// palette that remembered the last query would hide most of itself the
    /// next time it opened.
    pub fn clear(&mut self) {
        self.query.clear();
        self.refilter();
    }

    /// Moves the selection one row up, wrapping at the top.
    pub fn select_previous(&mut self) {
        let count = self.visible.len();
        self.selected = if count == 0 {
            0
        } else {
            (self.selected + count - 1) % count
        };
    }

    /// Moves the selection one row down, wrapping at the bottom.
    pub fn select_next(&mut self) {
        let count = self.visible.len();
        self.selected = if count == 0 {
            0
        } else {
            (self.selected + 1) % count
        };
    }

    /// Recomputes the visible rows and puts the selection back on the first
    /// of them: after a keystroke changes the list, the row under the
    /// selection is no longer the row the eye was on.
    fn refilter(&mut self) {
        self.visible = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| matches_query(&self.query, &row.text()))
            .map(|(index, _)| index)
            .collect();
        self.selected = 0;
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::from_catalog()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: &str, command: Command) -> Row {
        Row {
            number: 0,
            label: label.to_owned(),
            command,
            shortcut: None,
        }
    }

    fn sample() -> Palette {
        Palette::new(vec![
            row("Open the help modal", Command::ShowShortcuts),
            row("Show or hide the details panel", Command::ToggleDetails),
            row("Quit the terminal UI", Command::Quit),
        ])
    }

    fn labels(palette: &Palette) -> Vec<String> {
        palette
            .visible_rows()
            .iter()
            .map(|row| row.label.clone())
            .collect()
    }

    fn type_query(palette: &mut Palette, query: &str) {
        for typed in query.chars() {
            palette.type_character(typed);
        }
    }

    #[test]
    fn an_empty_query_shows_everything_in_order() {
        let palette = sample();
        assert_eq!(palette.visible_rows().len(), 3);
        assert_eq!(palette.rows()[0].number, 1);
        assert_eq!(palette.rows()[2].number, 3);
    }

    #[test]
    fn every_word_of_the_query_must_appear_in_any_order() {
        assert!(matches_query("open help", "1. Open the help modal"));
        assert!(matches_query("help open", "1. Open the help modal"));
        assert!(!matches_query("open quit", "1. Open the help modal"));
    }

    #[test]
    fn matching_ignores_case_on_both_sides() {
        assert!(matches_query("OPEN Help", "1. Open the help modal"));
    }

    #[test]
    fn an_empty_query_matches_any_row() {
        assert!(matches_query("", "anything at all"));
        assert!(matches_query("   ", "anything at all"));
    }

    #[test]
    fn typing_narrows_the_visible_rows() {
        let mut palette = sample();
        type_query(&mut palette, "help open");
        assert_eq!(labels(&palette), ["Open the help modal"]);
    }

    #[test]
    fn a_digit_narrows_to_the_row_that_carries_it() {
        let mut palette = sample();
        type_query(&mut palette, "3");
        assert_eq!(labels(&palette), ["Quit the terminal UI"]);
    }

    #[test]
    fn backspace_widens_the_list_again() {
        let mut palette = sample();
        type_query(&mut palette, "quit");
        assert_eq!(palette.visible_rows().len(), 1);
        for _ in 0..4 {
            palette.delete_character();
        }
        assert_eq!(palette.query(), "");
        assert_eq!(palette.visible_rows().len(), 3);
    }

    #[test]
    fn a_query_that_matches_nothing_selects_nothing() {
        let mut palette = sample();
        type_query(&mut palette, "zzz");
        assert!(palette.visible_rows().is_empty());
        assert_eq!(palette.selected_command(), None);
        assert_eq!(palette.selected(), 0);
    }

    #[test]
    fn moving_the_selection_wraps_at_both_ends() {
        let mut palette = sample();
        palette.select_previous();
        assert_eq!(palette.selected(), 2);
        palette.select_next();
        assert_eq!(palette.selected(), 0);
    }

    #[test]
    fn moving_an_empty_list_stays_put_instead_of_panicking() {
        let mut palette = sample();
        type_query(&mut palette, "zzz");
        palette.select_previous();
        palette.select_next();
        assert_eq!(palette.selected(), 0);
    }

    #[test]
    fn the_selection_runs_the_row_it_sits_on() {
        let mut palette = sample();
        palette.select_next();
        assert_eq!(palette.selected_command(), Some(Command::ToggleDetails));
    }

    #[test]
    fn narrowing_moves_the_selection_back_to_the_first_row() {
        let mut palette = sample();
        palette.select_next();
        type_query(&mut palette, "quit");
        assert_eq!(palette.selected(), 0);
        assert_eq!(palette.selected_command(), Some(Command::Quit));
    }

    #[test]
    fn clearing_forgets_the_query_and_the_selection() {
        let mut palette = sample();
        type_query(&mut palette, "quit");
        palette.clear();
        assert_eq!(palette.query(), "");
        assert_eq!(palette.visible_rows().len(), 3);
        assert_eq!(palette.selected(), 0);
    }

    #[test]
    fn the_catalog_palette_numbers_and_labels_every_command() {
        let palette = Palette::from_catalog();
        assert_eq!(palette.rows().len(), CATALOG.len());
        assert_eq!(
            palette.rows()[0].text(),
            "1. Show or hide the details panel"
        );
        assert_eq!(palette.rows()[0].shortcut, Some("Ctrl+D"));
    }
}
