# Terminal UI rules

How the terminal UI in `src/tui/` is built. The shape is deliberate: every
decision is a pure function with tests beside it, and the only untested code
is the loop that reads a key and the setup that borrows the terminal.

## The palette is the parent set of every command

Every action the UI can perform is a `Command` in `src/tui/command.rs`, and
`CATALOG` lists all of them. The command palette (`Ctrl+P`) shows that catalog,
so a person who knows nothing about this tool can find everything it does by
typing part of its name.

A keyboard shortcut is therefore a **shortcut to a command**, never the only
way to reach one. Each row of the table in `src/tui/shortcuts.rs` names the
commands its keys run, and the test
`every_shortcut_that_runs_something_has_a_palette_row` fails when a shortcut
runs something the catalog does not list. The exemption is narrow: a binding
that only moves the cursor, types a character, or answers a modal runs no
command, says so in its own description, and is pinned by
`only_movement_and_modal_input_may_declare_no_command`. Do not widen it: a key
that does something the user can name belongs in the catalog.

The relationship shows on screen too. A command that has a shortcut prints it
dim on its palette row, so the palette teaches the keyboard instead of
competing with it, and `the_hint_on_a_palette_row_names_a_real_binding` fails
when that hint stops being true.

Adding a command is four edits, and the compiler or a test asks for each one:
the variant, its wording in `Command::label`, its line in `CATALOG`, and its
arm in `state::run_command`.

## Rendering is a pure function of the state

`src/tui/render.rs` reads the state and draws it. It never mutates anything,
never asks the terminal a question, and never decides what should happen next:
if drawing needs to know something, that something is a field of `State` or a
function of it, worked out before the frame starts.

That is what keeps the UI testable. `State` plus `update` is the whole
behavior of this program, and `tests/tui_surface.rs` plays entire sessions
without ever opening a screen.

## Key handling is a classifier, never an `if` in the loop

`src/tui/keymap.rs` turns a `KeyEvent` and the open overlay into an `Action`.
It is pure, it is exhaustively tested, and it is the only place that knows
what a key means. What a key means **depends on what is open**: `Ctrl+P` opens
the palette from the main view and moves the selection while the palette is
up, and a printable character is a binding on the main view but a letter of
the query in the palette. That is exactly the kind of rule that rots when it
is spread through an event loop, so it lives in one function with a test per
case.

A new binding is an arm in the classifier, a row in the shortcut table, and a
test. It is never a condition inside the loop in `src/tui/mod.rs`, which stays
a shell: read a key, classify it, apply it, draw.

## The terminal is borrowed, and it is always given back

Raw mode and the alternate screen belong to the user's shell. A UI that exits
without restoring them leaves a terminal with no echo, no cursor, and no
scrollback, which a person can only fix by typing `reset` blind.

So every exit path restores: the `TerminalGuard` in `src/tui/mod.rs` restores
when it drops, which covers an early `?`, and the panic hook restores before
the panic message prints, which covers a crash. Both are required. If you add
another way out of the UI, it goes through one of them rather than calling
`disable_raw_mode` itself.

The UI draws on **stderr**, like the rest of this tool's human-facing output,
so stdout stays a clean data channel even while a full-screen UI is up.

## Color is semantic, and the terminal is dark

The UI never names a color. It asks `src/theme.rs` for a `Tone`
(`heading`, `accent`, `value`, `muted`, `success`, `warning`, `error`, `info`,
`prompt`) and renders that tone's palette index, so the terminal UI and the
command-line output stay one design with one palette. `render::style` is the
single place the two meet, and it honors the theme's own decision about
whether to paint at all, so `NO_COLOR` yields a monochrome UI here too.

The palette assumes a **dark** terminal, because terminals do not reliably say
which they are and a dark foreground on a dark background is unreadable. See
`docs/rules/cli-ux.md`.

## One module per file

`src/tui/` is one responsibility per file, and the split is the pure/impure
line drawn in `docs/rules/rust.md`:

| File | What it owns |
| --- | --- |
| `mod.rs` | The terminal lifecycle and the event loop. No decisions. |
| `keymap.rs` | What a keystroke means, given what is open. |
| `command.rs` | The command vocabulary and the catalog's order. |
| `palette.rs` | The palette's query, filter, and selection. |
| `shortcuts.rs` | The keybinding table, and the parity guard. |
| `state.rs` | The state, and `update(state, action)`. |
| `render.rs` | Drawing, as a function of the state. |

A file that grows past roughly 400 lines of production code gets split along a
real seam, the same rule the rest of the codebase follows.
