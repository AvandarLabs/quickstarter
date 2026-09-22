# Scaffolder architecture

This repo is a scaffolder: it builds a new front-end project by composing
template layers. This document explains how the pieces fit together and how to
extend it.

## The problem it solves

Supporting N stack combinations by keeping a full template repo per combination
is O(N) to maintain: a change to a shared dependency (Mantine, the theme, lint
config) has to be repeated in every copy. Instead, every concern is stored
once and projects are **composed** from layers. The maintenance cost of a
shared change is O(1), and adding a stack is additive rather than multiplicative.

## Layers

```
templates/
├── base/
│   ├── files/            # copied verbatim into every generated project
│   └── package.json      # shared dependencies: the merge base
└── modules/
    └── <stack>/
        ├── files/        # files this stack adds or overrides
        ├── package.json  # dependency fragment merged onto the base
        └── module.json   # metadata: name, description, order, tokens
```

- **base** is the single source of truth for everything shared. Change Mantine,
  the theme, or the lint config here and every stack inherits it.
- **modules** each contribute only what differs for that stack.

## Composition techniques

A project is assembled with three complementary techniques (see
`cli/src/compose/`):

1. **Overlay** (`overlay.rs`): copy `base/files` into the destination, then
   copy the chosen module's `files` on top. Later layers win, so a module owns
   a whole divergent file (for example each stack ships its own
   `vite.config.ts`). Symlinks are recreated as links, not dereferenced, so the
   generated `CLAUDE.md -> AGENTS.md` link is preserved.
2. **Deep-merge** (`package_json.rs`): the base `package.json` and the module's
   `package.json` fragment are deep-merged. `dependencies` blocks combine
   rather than replace; a module can override a scalar such as a script command.
3. **Token substitution** (`tokens.rs`): after overlay and merge, every text
   file is scanned for `{{TOKEN}}` placeholders. Values come from the module's
   `module.json` `tokens` map plus the built-in `PROJECT_NAME` (display name)
   and `PACKAGE_NAME` (npm-safe name). Use this for files that are mostly shared
   but carry a stack-specific line, such as `AGENTS.md`'s stack summary or the
   `README.md` routing notes.

Choosing the right technique: whole-file differences use overlay, dependency
differences use the merge, and small in-file differences use tokens. Never copy
a shared file into a module just to change one line.

## The command-line surface

Every answer the scaffolder needs has a flag, and every flag is optional:
anything the user leaves out is asked for. `--yes` (alias `--no-input`) removes
the questions, at which point the options with no sensible default become
required.

| Answer | Flag | Missing under `--yes` |
| --- | --- | --- |
| Project name | `-n`, `--name` | error |
| Location | `-d`, `--dir` | defaults to `.` |
| Stack | `-s`, `--stack` | error |
| Template repo | `--repo` | defaults to `DEFAULT_TEMPLATE_REPO` |

Three modules implement that contract:

- `cli/src/cli/args.rs` defines the flags (clap derive) and
  `ensure_answerable`, which rejects an incomplete `--yes` command up front,
  naming every missing flag at once.
- `cli/src/cli/resolve.rs` resolves each answer from the same three sources in
  order: the flag, a question, a default. It is the only place that knows
  prompting can be off.
- `cli/src/cli/prompts.rs` holds the questions themselves (`dialoguer`).
  Enum-shaped answers use a `Select` list rather than free text, so the stack is
  chosen with the arrow keys or `j`/`k`. A value passed as a flag is validated
  by the same code that validates a typed one.

Adding an answer means adding a field in `args.rs`, a resolver method, and
(when it is not free text) a `Select` prompt. Adding a *required* one also
means adding it to `ensure_answerable`.

## Runtime flow

The binary is a thin client (`cli/src/app.rs`):

1. Check `git` is available, and reject an incomplete `--yes` command. Both
   happen before anything is asked, created, or fetched.
2. Resolve the project name and location; resolve and reject an existing target.
3. Clone the template repo shallowly into a temp dir (`template/fetch.rs`).
   Clone failures are classified as offline, access/config, or generic.
4. Discover modules from the clone (`catalog/mod.rs`) and resolve the stack.
   This comes after the clone because the valid choices are whatever modules
   the template repository ships; an unknown `--stack` is rejected here with
   the real list.
5. Compose into the target directory; on error, remove the partial output.
6. Initialize the target as a git repository on `main` with a single initial
   commit (`git_init.rs`). The branch is named explicitly, because `git init`
   otherwise falls back to `master` on a machine with no `init.defaultBranch`.
   This is best-effort: if git init or the commit fails (for example, no
   configured identity), the run prints a warning and keeps the project rather
   than discarding it. A generic fallback identity is used for the commit only
   when the user has none configured.
7. Print next steps. The temp clone is deleted when the run ends, so the user
   only ever sees the finished project.

Because the templates are fetched at runtime, an old binary still builds from
the newest templates.

`cli/tests/non_interactive_test.rs` drives the real binary through this whole
flow with `--yes`, cloning a fixture template repo from a local path, so the
non-interactive path is covered end to end without a network.

## Adding a new stack

1. Create `templates/modules/<key>/` with:
   - `files/` containing the files that differ for the stack;
   - `package.json` with only the extra dependencies/scripts;
   - `module.json` with `key`, `name`, `description`, `order`, and any `tokens`
     referenced by base files.
2. Provide a value for every `{{TOKEN}}` that base files use, or the
   `real_templates_test` will fail on an unfilled token.
3. Run `cargo test`. Module discovery is data-driven, so no Rust changes are
   needed to make the new stack selectable.

## Skills: quickstarter-dev vs produced repos

There are two distinct skill sets, and the split is data, not convention.

- **Quickstarter-dev skills** live in this repo (`.agents/skills`, tracked via
  `skills-lock.json`) so engineers working on the scaffolder share the same
  tooling. This set includes **all Rust-related skills**, because the CLI is
  written in Rust: `rust-skills`, `coding-guidelines` (Rust code style, despite
  the generic name), the `rust-*` tools, and Rust toolchain skills without the
  prefix (`cargo-workflows`, `fuzzing`).

- **Produced-repo skills** are the curated set a *generated* project receives.

The authoritative split lives in
[`skills-manifest.json`](../skills-manifest.json), which has a
`quickstarterDev` and a `produced` bucket; a skill in both means it applies to
both. `cli/tests/skills_manifest_test.rs` fails the build if a skill is
installed but unclassified, if the manifest names a skill that is not
installed, or if a Rust-related skill leaks into `produced`.

### How produced skills reach a generated project

Composition writes a `skills-lock.json` into the new project
(`cli/src/compose/skills_lock.rs`): the `produced` bucket intersected with this
repo's own lock, entries copied verbatim so the project pins the same sources.
`cli/tests/real_templates_test.rs` asserts the result covers the whole produced
set and that no Rust skill leaks through.

The scaffolder itself installs nothing. The generated project's
`postinstall` calls its own `scripts/skills` CLI, so the first `pnpm install`
materializes `.agents/skills` and the per-frontend links from the lock. That
keeps scaffolding fast and offline-capable, and it means a teammate cloning the
project later goes through exactly the same path.

`impeccable` is the one exception and is deliberately excluded from the
generated lock (`CLI_MANAGED_SKILLS`): it ships its own `impeccable` CLI, which
the project's skills wrapper drives separately. Adding another
self-installing skill means adding it to that constant and teaching
`scripts/skills` about it.

## Adding a new axis (future)

Today there is a single axis (the stack). A second axis (for example a data
layer) would be modeled as another module group, a second flag, and a second
prompt, composed as an additional overlay + merge pass. The composition engine
already supports stacking more than two layers; only the flag/prompt wiring
would grow.
