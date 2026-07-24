# Agent Rules

This repository is a **project scaffolder**, not the front-end app itself. It
has two parts: the Rust CLI under `cli/`, and the template layers under
`templates/`. The rules that a *generated* project follows live in
`templates/base/files/AGENTS.md`; the rules below are for working on the
scaffolder.

## Architecture

Read [`docs/scaffolder.md`](docs/scaffolder.md) before changing composition
behavior. In short, a project is composed with three techniques:

- **Overlay** (`cli/src/compose/overlay.rs`): copy `base/files`, then the
  chosen module's `files` on top. Later layers win, so a module owns a whole
  file (its `vite.config.ts`) that differs from the base.
- **Deep-merge** (`cli/src/compose/package_json.rs`): the base `package.json`
  plus each module's `package.json` fragment.
- **Token substitution** (`cli/src/compose/tokens.rs`): `{{TOKEN}}`s for files
  that are mostly shared but carry a few stack-specific lines.

## Where things go

- A change that should affect **every** stack goes in `templates/base/`.
- A change specific to one stack goes in that `templates/modules/<stack>/`.
- Never duplicate a shared file into a module just to tweak one line. Add a
  `{{TOKEN}}` in the base file and give it a value in each module's
  `module.json`.

## Rust conventions

- One module per file; keep files small (~400 lines of production code is the
  smell threshold). Model parent/child relationships through the directory
  tree, mirroring the code's ownership.
- Use `anyhow` with `.context(...)` for error messages the user will read.
- Document exported items with `///` doc comments.
- Do not use em dashes in comments or output. Prefer a colon or a hyphen.
- Keep functions short (<= 45 lines).

## TDD

Implement with red/green TDD by default. The pure logic (merge, tokens,
overlay, catalog) is unit-tested inline; `cli/tests/` composes the **real**
templates end-to-end. Run `cargo test` and `cargo clippy` before finishing.

## Keeping docs current

When you add or change a stack module, a composition technique, or the CLI's
behavior, update `docs/scaffolder.md` and this file in the same change.
