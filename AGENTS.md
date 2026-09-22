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

Two skill sets live here and they are independent of each other.

- **This repo's own skills** (`.agents/skills`, tracked in
  `skills-lock.json`) are local tooling for people working on the scaffolder.
  Install one with `skills add ...` and commit the lock. No manifest change is
  needed, and nothing about generated projects changes.
- **A generated project's skills** come from
  [`skills-manifest.json`](skills-manifest.json), which is keyed by capability
  tag. Adding a skill for new projects means adding its full source spec (the
  string `npx skills add` receives: `owner/repo`, or `owner/repo/path/to/skill`
  when the SKILL.md is not at the repository root) to the right capability. Put
  it in `global` when every project should have it, otherwise in the capability
  that implies it (`typescript`, `tanstack-router`, ...). A new capability is a
  new key here plus the `capabilities` list of each
  `templates/modules/<key>/module.json` that declares it.

The manifest is live: the scaffolder installs the selected specs with `npx
skills add` inside the new project, so a spec added here is installed by the
next run. `pbakaus/impeccable` is the one spec not handed to `npx skills`,
because it installs itself through its own CLI. The generated project is told
which of its skills are self-installing through the `SELF_INSTALLING_SKILLS`
token, so its own `scripts/skills/update-skills.sh` can update them the same
way. Another self-installing skill means teaching
`cli/src/skills/commands.rs` about it and that script how to update it.

`cli/tests/skills_manifest_test.rs` guards the manifest's invariants: every
spec well formed and unique, a non-empty `global`, and every capability a
module declares present in the manifest. See the skills section of
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
