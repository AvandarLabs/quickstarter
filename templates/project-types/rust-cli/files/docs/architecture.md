# Architecture

{{PROJECT_NAME}} is a single crate with two targets: a library that holds
everything worth testing, and a binary that is a shell around it. Extend this
document as the shape changes; it is meant to grow, not to be replaced.

## The layout

```
src/
├── main.rs          the binary: parse, dispatch, report, exit
├── lib.rs           the library root: module list and the product name
├── cli.rs           the clap types: every flag and subcommand
├── theme.rs         semantic colors for human-facing output
├── commands/
│   ├── mod.rs       dispatch, plus the narration commands share
│   ├── greet.rs     a command that asks when a value is missing
│   └── info.rs      a command whose output is data
└── tui/
    ├── mod.rs       terminal setup, the event loop, teardown
    └── keymap.rs    what a keystroke means
tests/
└── public_api.rs    the library surface, driven from outside the crate
```

## Why a library plus a thin binary

`main.rs` does four things: parse the arguments, call `commands::run`, print a
failure on stderr, and return an exit code. Everything else is in the library,
so a test can reach it by calling a function instead of spawning a process.
`tests/public_api.rs` is that test: it drives the same public surface the
binary does, which is also a check that the surface is usable from outside.

## The pure/impure split

The split runs through every module, and it is the reason the test suite is
fast and boring:

| Pure, tested inline | Impure, deliberately thin |
| --- | --- |
| `cli.rs`: the argument types, and that they parse | `main.rs`: process plumbing |
| `theme.rs`: which role gets which color | `commands/*::run`: printing, prompting |
| `commands::greet::greeting`: the message | `tui::mod`: raw mode, the alternate screen, the event loop |
| `commands::info::facts`: the facts and their order | |
| `tui::keymap::action_for`: which key means what | |

A terminal UI hides its bugs in key handling, so key handling is a pure
function returning an `Action` and the event loop only routes the result. The
loop itself has no tests because it has no decisions: when it grows one, that
decision moves into `keymap.rs` first.

## Output channels

Stdout carries data, stderr carries the conversation with the person: prompts,
progress, failures, and the terminal UI itself. That is why the UI renders to
stderr, and why `theme.rs` decides whether to emit color by looking at stderr.
See `docs/rules/cli-ux.md`.

## The dependencies, and what each is for

| Crate | Why it is here |
| --- | --- |
| `anyhow` | One error type across the program, with a `.context(...)` chain the binary prints as the failure message. |
| `clap` (derive, env) | The command-line surface, declared next to the types it fills. `env` lets any flag also come from the environment. |
| `ratatui` | The terminal UI. Immediate-mode rendering keeps drawing a function of the state it is handed. |
| `crossterm` | Terminal control and key events under ratatui, pinned to the version ratatui builds against so the key types match. |

The set is small on purpose (`docs/rules/rust.md`). Adding to it means saying
here what the crate is for, in one line, in the same change.

## Things to rename once

- `CLI_VERBOSE`, the environment variable behind `--verbose` in `src/cli.rs`.
  Give it a prefix of your own when the tool's name settles.
- `APP_NAME` in `src/lib.rs` is the name people read; the crate name is what
  they type.
