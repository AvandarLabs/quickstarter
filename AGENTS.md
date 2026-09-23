# Agent Rules

This repository is a **project scaffolder**, not one of the projects it
builds. It has two parts: the Rust CLI under `cli/`, and the template layers
under `templates/`. The rules that a *generated* project follows live in
`templates/base/files/AGENTS.md` and its `docs/rules/`; the rules below are for
working on the scaffolder.

## Architecture

Read [`docs/scaffolder.md`](docs/scaffolder.md) before changing composition
behavior.

A generated repo carries **tags** of two types: exactly one **project type**
(`typescript:web`, `rust:cli`), which decides the build system, the starting
files, and the language's skills, plus any number of **capabilities**
(`tanstack-router`, `tanstack-start`, `rust-tui`) layered on top. Which
combinations are legal is declared by the tags themselves (`projectTypes`,
`languages`, `conflictsWith`) and read by `cli/src/catalog/`, never hardcoded.
Capabilities that exclude one another form a choice group, and a group that
fits the chosen project type is a required choice; one that excludes nothing is
an optional extra (`rust-tui` on `rust:cli`).

A project is composed with four techniques:

- **Overlay** (`cli/src/compose/overlay.rs`): copy `base/files`, then the
  project type's `files`, then each chosen capability's `files`. Later layers
  win, so a capability owns a whole file (its `vite.config.ts`) that its
  project type also ships.
- **Manifest merge** (`cli/src/compose/package_json.rs`,
  `cli/src/compose/cargo_toml.rs`): the project type's dependency manifest plus
  each capability's fragment. Which merge runs is the `ManifestKind` detected
  from the manifest at the project type's layer root (`package.json` or
  `Cargo.toml`); a project type that ships neither composes without one rather
  than failing, and a capability fragment of the other kind is refused by name
  rather than skipped. The TOML merge is `toml_edit`, because a
  parse-and-reserialize would delete the comment above every dependency. A
  token in a manifest has to sit inside a value (`name = "{{PACKAGE_NAME}}"`):
  the manifest is parsed before substitution runs.
- **Token substitution** (`cli/src/compose/tokens.rs`): `{{TOKEN}}`s for files
  that are mostly shared but carry a few tag-specific lines. Every UTF-8 text
  file is processed, extensionless ones (`.gitignore`, `justfile`) included.
- **Seams** (the part of substitution that lets a capability add code): a line
  holding nothing but a token, declared empty by the project type that owns the
  file and filled by a capability that needs the line. Today `EXTRA_MODULES`,
  `EXTRA_SUBCOMMANDS` and `EXTRA_DISPATCH` in the `rust:cli` source, and
  `EXTRA_RULES` in the base `AGENTS.md`, which every project type declares. A
  seam line whose value is empty is deleted whole, newline included, so a
  project that took no capability has no blank hole and still passes
  `cargo fmt --check`.

## The CLI surface

Every answer has a flag and every flag is optional: what the user omits is
asked for. `--yes` turns the questions off, making the answers with no default
(`--name`, `--project-type`) required and turning an unmade required capability
choice into an error. `-c, --capability` repeats and also takes a
comma-separated list. Keep those three pieces in step when you add an answer:
the flag in `cli/src/cli/args.rs`, the resolution in `cli/src/cli/resolve.rs`,
and the question in `cli/src/cli/prompts.rs`. An answer with a fixed set of
choices gets a `Select` (a `MultiSelect` when several may be ticked), never
free text.

## Where things go

- A change that should affect **every** project, whatever its language, goes
  in `templates/base/`. Keep that layer small: it holds `AGENTS.md`, the
  `README.md`, `.gitignore`, `docs/skills.md`, and the skills update script.
- A change specific to one language and product goes in
  `templates/project-types/<slug>/`. Deep language rules belong in its
  `files/docs/rules/`, not in a token value.
- A change specific to one library goes in `templates/capabilities/<slug>/`,
  along with the compatibility it declares (`projectTypes`, `languages`,
  `conflictsWith`). Declare a conflict on both sides.
- A capability that adds a dependency ships a manifest fragment of the kind its
  project types merge: `Cargo.toml` on `rust:cli`, `package.json` on
  `typescript:web`. The other kind fails composition, so a fragment is never
  silently dropped.
- A capability that has to add a line to a file its project type owns fills a
  **seam** token; it never overlays the file to get one line in. At most one
  capability per project type may fill a given seam, because the token map is
  flat and the second one would silently win:
  `cli/tests/template_tokens_test.rs` fails the build on that pair.
  Capabilities that exclude each other are exempt, since a project chooses
  exactly one of them (both routers define `STACK_LINE`). A seam that does not
  exist yet is a change to the project type: one more empty token in its
  `project-type.json`, and the line that carries it.
- Never duplicate a shared file into a project type or a capability just to
  tweak one line. Add a `{{TOKEN}}` in the shared file and give it a value in
  the `project-type.json` or `capability.json` of whichever tag knows the
  answer.

## Skills

Two skill sets live here and they are independent of each other.

- **This repo's own skills** (`.agents/skills`, tracked in
  `skills-lock.json`) are local tooling for people working on the scaffolder.
  Install one with `skills add ...` and commit the lock. No manifest change is
  needed, and nothing about generated projects changes.
- **A generated project's skills** come from
  [`skills-manifest.json`](skills-manifest.json), which is keyed by tag in
  three sections: `global` (every project), `projectTypes` (keyed by project
  type key), and `capabilities` (keyed by capability key). A project gets the
  union of the three lists that apply to it. Adding a skill for new projects
  means adding its full source spec (the string `npx skills add` receives:
  `owner/repo`, or `owner/repo/path/to/skill` when the SKILL.md is not at the
  repository root) to the right section. A new tag needs an entry here, even
  an empty one, and its compatibility is declared in `templates/` rather than
  here: the manifest only says what each tag installs.

The manifest is live: the scaffolder installs the selected specs with `npx
skills add` inside the new project, so a spec added here is installed by the
next run. `pbakaus/impeccable` is the one spec not handed to `npx skills`,
because it installs itself through its own CLI. The generated project is told
which of its skills are self-installing through the `SELF_INSTALLING_SKILLS`
token, so its own `scripts/skills/update-skills.sh` can update them the same
way. Another self-installing skill means teaching
`cli/src/skills/commands.rs` about it and that script how to update it. A new
project type wires that one script to its own task runner (`pnpm skills:update`
on `typescript:web`, `just skills-update` on `rust:cli`).

`cli/tests/skills_manifest_test.rs` guards the manifest's invariants: every
spec well formed and unique, a non-empty `global`, and the tags the manifest
names being exactly the tags `templates/` declares. See the skills section of
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

When you add or change a project type, a capability, a composition technique,
or the CLI's behavior, update `docs/scaffolder.md` and this file in the same
change.
