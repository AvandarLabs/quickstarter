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
        └── module.json   # metadata: name, order, capabilities, tokens
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
6. Install the agent skills the chosen module's capabilities select, by running
   `npx skills add` once per spec inside the new project (`skills/install.rs`).
   This is best-effort: a failure prints a warning naming the commands to
   re-run and keeps the project.
7. Initialize the target as a git repository on `main` with a single initial
   commit (`git_init.rs`). The branch is named explicitly, because `git init`
   otherwise falls back to `master` on a machine with no `init.defaultBranch`.
   This is best-effort: if git init or the commit fails (for example, no
   configured identity), the run prints a warning and keeps the project rather
   than discarding it. A generic fallback identity is used for the commit only
   when the user has none configured.
8. Print next steps. The temp clone is deleted when the run ends, so the user
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
   - `module.json` with `key`, `name`, `description`, `order`, the
     `capabilities` the stack has, and any `tokens` referenced by base files.
2. Provide a value for every `{{TOKEN}}` that base files use, or the
   `real_templates_test` will fail on an unfilled token.
3. Use only capabilities that exist in
   [`skills-manifest.json`](../skills-manifest.json). An unknown one fails
   `skills_manifest_test`, because it would silently install no skills.
4. Run `cargo test`. Module discovery is data-driven, so no Rust changes are
   needed to make the new stack selectable.

## Skills: capability tags, not repo types

A generated project has no single type, so the skills it gets are not a single
list. It has a **set of capability tags**, and it receives the union of the
lists those tags name in [`skills-manifest.json`](../skills-manifest.json):

- `global` is the special tag every generated project gets: the skills that
  apply whatever the stack is (planning, debugging, code review, and so on).
- Every other tag is a capability a stack module declares in its `module.json`.
  `router` declares `["typescript", "tanstack-router"]` and `start` declares
  `["typescript", "tanstack-start"]`, so both inherit the TypeScript skills
  while each keeps room for skills only it should have.
- `rust` is in the manifest as data only: no module declares it yet. It is
  there because a Rust CLI project type is coming, and a capability that
  nothing claims costs nothing.

Each entry is a **source spec** handed to `npx skills add` verbatim, so it has
to resolve to exactly one skill: `owner/repo` when the repository's SKILL.md is
at its root, `owner/repo/path/to/skill` otherwise.

### Adding a capability

A capability is nothing more than a key in the manifest plus the modules that
declare it. Add the key with its list of specs, then add its name to the
`capabilities` array of every `templates/modules/<key>/module.json` that should
get those skills. No Rust changes are involved.

### This repo's lock is unrelated

`skills-lock.json` here records the skills installed in **this** checkout, so
that engineers working on the scaffolder share the same tooling (all the
Rust-related ones, because the CLI is written in Rust). It is completely
decoupled from the manifest: `npx skills add` or `npx skills remove` in this
repo changes nothing about generated projects, and a spec in the manifest does
not have to be installed here at all. The two files answer different questions:
the lock says what this checkout has, the manifest says what a new project
gets.

### How the skills reach a generated project

After composition and before the git init, the scaffolder runs one command per
selected spec inside the new project directory:

```sh
npx -y skills add <spec> --agent claude-code cursor opencode codex -y
```

`npx skills` does the rest: it writes `.agents/skills`, the `.claude/skills`
symlinks, and the new project's own `skills-lock.json`. That lock is why the
generated project's skills tooling (`scripts/skills`, the `postinstall` hook,
`pnpm skills:*`) keeps working unchanged: a teammate cloning the project later
restores exactly the same set with `pnpm install`. Think "node_modules
installed on creation", not "bundled".

The code lives in `cli/src/skills/`: `manifest.rs` selects the specs for a set
of capabilities, deduplicated; `commands.rs` builds the commands; `install.rs`
runs them. Installing is best-effort like `git init`: a failure prints a
warning naming the commands to re-run, and never discards the project the user
just created.

### Skills that install themselves

`pbakaus/impeccable` is listed under `global` but is the one spec the
scaffolder does not hand to `npx skills`. It ships its own installer, so it is
run as:

```sh
npx -y impeccable install \
  --providers=claude,cursor,opencode,codex --scope=project
```

A generated project has to keep updating that skill the same way, and it cannot
work out which of its skills are self-installing on its own: they are not in
`skills-lock.json`, which is exactly why they need their own installer. So the
scaffolder tells it. `SELF_INSTALLING_SKILLS` is a built-in token, computed
from the selected specs (`cli/src/app.rs`), that the templates substitute:
today it lands in the project's `scripts/skills/update-skills.sh` as
`SELF_INSTALLING_SKILLS="impeccable"`, and in the project's TypeScript skills
constants. A project whose capabilities select no self-installing skill gets an
empty list and skips that step.

That script is the generated project's single "update every skill" entry point:
`npx skills update --project` for everything in the lock, then each
self-installing skill through its own CLI. It is plain `sh` rather than part of
the TypeScript tooling because every generated project needs it, including the
non-TypeScript ones to come; each stack only wires it to its own idiom (a
`skills:update` script in `package.json` for the TypeScript stacks).

Adding another self-installing skill means teaching
`cli/src/skills/commands.rs` about it (so the scaffolder installs it and the
token names it) and `scripts/skills/update-skills.sh` how to update it.

`cli/tests/skills_manifest_test.rs` guards the manifest itself rather than
comparing it against what is installed: every spec is well formed, no spec is
duplicated inside or across capabilities, `global` exists and is non-empty, and
every capability a module declares exists in the manifest.

## Adding a new axis (future)

Today there is a single axis (the stack). A second axis (for example a data
layer) would be modeled as another module group, a second flag, and a second
prompt, composed as an additional overlay + merge pass. The composition engine
already supports stacking more than two layers; only the flag/prompt wiring
would grow.
