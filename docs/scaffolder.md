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

## Runtime flow

The binary is a thin client (`cli/src/app.rs`):

1. Check `git` is available (fail fast).
2. Prompt for project name and location; resolve and reject an existing target.
3. Clone the template repo shallowly into a temp dir (`template/fetch.rs`).
   Clone failures are classified as offline, access/config, or generic.
4. Discover modules from the clone (`catalog/mod.rs`) and prompt for the stack.
5. Compose into the target directory; on error, remove the partial output.
6. Print next steps. The temp clone is deleted when the run ends, so the user
   only ever sees the finished project.

Because the templates are fetched at runtime, an old binary still builds from
the newest templates.

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

There are two conceptually distinct skill sets, even though only the first
exists today:

- **Quickstarter-dev skills** live in this repo (`.agents/skills`, tracked via
  `skills-lock.json`) so engineers working on the scaffolder share the same
  tooling. This set includes **all Rust-related skills**, because the CLI is
  written in Rust: `rust-skills`, `coding-guidelines` (Rust code style, despite
  the generic name), and the `rust-*` tools (`rust-call-graph`,
  `rust-code-navigator`, `rust-daily`, `rust-deps-visualizer`, `rust-learner`,
  `rust-refactor-helper`, `rust-router`, `rust-symbol-analyzer`,
  `rust-trait-explorer`).

- **Produced-repo skills** are the curated set a *generated* project should
  receive. Installing them into generated projects is **not implemented yet**
  (the templates install no skills today), but the split itself is already
  deterministic.

The authoritative, machine-readable split lives in
[`skills-manifest.json`](../skills-manifest.json) at the repo root. It has two
buckets, `quickstarterDev` and `produced`, and every installed skill must
appear in exactly one. `cli/tests/skills_manifest_test.rs` enforces this: the
build fails if a skill is added or removed without updating the manifest, if a
skill is in both buckets, if the manifest names a skill that is not installed,
or if any Rust-related skill (the `rust-*` tools or `coding-guidelines`) leaks
into the `produced` bucket. When the produced-repo install is built, it reads
the `produced` list and resolves each skill's source from `skills-lock.json`.

## Adding a new axis (future)

Today there is a single axis (the stack). A second axis (for example a data
layer) would be modeled as another module group and a second prompt, composed
as an additional overlay + merge pass. The composition engine already supports
stacking more than two layers; only the prompt/selection wiring would grow.
