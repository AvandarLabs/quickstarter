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

## The CLI surface

Every answer has a flag and every flag is optional: what the user omits is
asked for. `--yes` turns the questions off, making the answers with no default
(`--name`, `--stack`) required. Keep those three pieces in step when you add an
answer: the flag in `cli/src/cli/args.rs`, the resolution in
`cli/src/cli/resolve.rs`, and the question in `cli/src/cli/prompts.rs`. An
answer with a fixed set of choices gets a `Select` list, never free text.

## Where things go

- A change that should affect **every** stack goes in `templates/base/`.
- A change specific to one stack goes in that `templates/modules/<stack>/`.
- Never duplicate a shared file into a module just to tweak one line. Add a
  `{{TOKEN}}` in the base file and give it a value in each module's
  `module.json`.

## Skills

Whenever you install a new skill in this repo (`skills add ...`), you MUST, in
the same change, classify it in [`skills-manifest.json`](skills-manifest.json)
by deciding where it belongs:

- **`quickstarterDev`** - only useful for developing the quickstarter CLI (for
  example anything Rust-related). Never installed into generated projects.
- **`produced`** - shipped to every generated project. The `produced` bucket is
  live: composition writes it into the new project's `skills-lock.json`, so
  adding a name here means the next generated project installs that skill.
- **both** - list the skill in *both* arrays when it applies to developing
  quickstarter and to generated projects.

Every installed skill must appear in at least one bucket.
`cli/tests/skills_manifest_test.rs` fails the build if a skill is installed but
unclassified, if the manifest names a skill that is not installed, or if any
Rust-related skill (`rust-*` or `coding-guidelines`) is placed in `produced`.
Removing a skill means removing it from the manifest in the same change.

A produced skill must also be installed here, because the generated lock copies
its source and hash from this repo's `skills-lock.json`. A name in `produced`
with no lock entry fails composition rather than silently shipping less.

`impeccable` is `produced` but excluded from the generated lock: it installs
itself through its own CLI. See the skills section of
[`docs/scaffolder.md`](docs/scaffolder.md).

## Rust conventions

- One module per file; keep files small (~400 lines of production code is the
  smell threshold). Inline tests do not count toward the production threshold,
  but they remain subject to modularity review. Model parent/child
  relationships through the directory tree, mirroring the code's ownership.
- Apply the same modularity standard to unit tests, test-only modules,
  integration tests, test support, harnesses, and fixtures. Split large or
  multi-responsibility suites by behavior or subsystem, and compose fixtures
  from focused helpers instead of building monolithic test infrastructure.
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
