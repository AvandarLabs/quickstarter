# Scaffolder architecture

This repo is a scaffolder: it builds a new project by composing template
layers. This document explains how the pieces fit together and how to extend
it.

## The problem it solves

Supporting N combinations by keeping a full template repo per combination is
O(N) to maintain: a change to a shared dependency (Mantine, the theme, lint
config) has to be repeated in every copy. Instead, every concern is stored once
and projects are **composed** from layers. The maintenance cost of a shared
change is O(1), and adding a choice is additive rather than multiplicative.

## The tag model

A generated repo carries **tags**, and a tag is one of two types.

A **project type** is the tag every project has exactly one of. Its key is
`<language>:<product>`, and it decides the build system, the files the project
starts from, and the skills that come with the language. A **capability** is a
library or framework layered on top, and a project has any number of them.

| | Project type | Capability |
| --- | --- | --- |
| Per project | exactly one | zero or more |
| Key | `<language>:<product>` | a plain name |
| Declared in | `project-types/<slug>/` | `capabilities/<slug>/` |
| Decides | build system, starting files, language skills | one library |
| Today | `typescript:web`, `rust:cli` | `tanstack-router`, `tanstack-start` |

Which combinations are legal is declared data, not code. A capability names the
`projectTypes` and the `languages` it fits (an empty list restricts nothing)
and the capabilities it `conflictsWith`. The rules that read those fields are
pure and live in `cli/src/catalog/compatibility.rs`, where a conflict counts
even when only one side declares it, so a single missing entry cannot let an
impossible pair through.

Capabilities that exclude one another are one question asked once, not several
independent yes/no answers, so they are grouped: a **choice group** is a
connected component of the conflict relation (`cli/src/catalog/groups.rs`).
Grouping is transitive, because if A excludes B and B excludes C then no two of
the three can be chosen together. A group whose members fit the chosen project
type is a **required** choice; a capability that excludes nothing is an
independent optional one.

Today that produces exactly one question. Both TanStack capabilities require
`typescript:web` and exclude each other, so a web project picks exactly one
router, and `rust:cli` has no capabilities at all.

A slug is a directory name (`rust-cli`) and a key is the tag id (`rust:cli`).
They differ because a key carries a `:`, which a directory name cannot, so the
slug is read from the directory rather than declared.

## Layers

```
templates/
├── base/
│   └── files/                 # what every project type needs
├── project-types/<slug>/
│   ├── project-type.json      # key, name, language, order, tokens
│   ├── package.json           # the merge base, if the language has one
│   └── files/                 # what this kind of project starts from
└── capabilities/<slug>/
    ├── capability.json        # key, name, order, tokens, and the
    │                          #   compatibility: projectTypes,
    │                          #   languages, conflictsWith
    ├── package.json           # fragment merged onto the project type's
    └── files/                 # what this library adds or overrides
```

- **base** holds only what every project type needs, whatever its language:
  `AGENTS.md` (with the `CLAUDE.md` and `.cursor/rules/agents.mdc` symlinks
  that point at it), `README.md`, `.gitignore`, `docs/skills.md`, and
  `scripts/skills/update-skills.sh`. Every line in it that differs between
  project types is a `{{TOKEN}}`.
- **project-types** each contribute one language and product: its source,
  config, task runner, and `docs/rules/*.md`. The deep language rules are
  shipped as files rather than squeezed into token values.
- **capabilities** each contribute only what one library adds: its files, its
  dependencies, and the tokens that name it.

A layer may leave any part out. A capability with no `files/` is a
`package.json` fragment and a token or two; a project type with no
`package.json` (the Rust one) composes without a manifest at all.

## Composition techniques

A project is assembled with three complementary techniques (see
`cli/src/compose/`). `plan.rs` is the one place that knows where each layer
lives and how the token map stacks, so `mod.rs` stays a list of steps.

1. **Overlay** (`overlay.rs`): copy `base/files`, then the project type's
   `files`, then each chosen capability's `files` in `order`. Later layers win,
   so a capability owns a whole divergent file (each router ships its own
   `vite.config.ts`). Symlinks are recreated as links, not dereferenced, so the
   generated `CLAUDE.md -> AGENTS.md` link is preserved.
2. **Deep-merge** (`package_json.rs`): the project type's `package.json` is the
   merge base, and each chosen capability's `package.json` is a fragment merged
   onto it. `dependencies` blocks combine rather than replace; a fragment can
   override a scalar such as a script command. A project type that ships no
   `package.json` skips the step entirely rather than failing it, because a
   Rust project must not be handed an empty manifest.
3. **Token substitution** (`tokens.rs`): after overlay and merge, `{{TOKEN}}`
   placeholders are filled in across the composed tree. Values come from the
   project type's `tokens`, then each capability's (so a capability wins, and a
   later capability wins over an earlier one), then the tokens the CLI computed
   itself, then the built-in `PROJECT_NAME` (display name) and `PACKAGE_NAME`
   (npm-safe name), which nothing may override.

Substitution reads every UTF-8 text file rather than a list of known
extensions, because `.gitignore` and `justfile` carry tokens and have no
extension at all: a list would silently ship a `{{TOKEN}}` to the user. Known
binary extensions are skipped without being read, symlinks are left alone, and
a file is rewritten only when a token actually changed it. A token with no
value is left in place so the mistake is visible, and
`cli/tests/real_templates_test.rs` fails on any unfilled token in a composed
project.

Choosing the right technique: whole-file differences use overlay, dependency
differences use the merge, and small in-file differences use tokens. Never copy
a shared file into a tag's layer just to change one line.

### Which tag owns a token

A token belongs to whichever tag knows the answer, which is not always the same
kind of tag. `PROJECT_TYPE_RULES`, `BUILD_TEST_BLOCK`, `PROJECT_TYPE_IGNORES`
and the `SKILLS_*` values are project-type tokens, because they describe a
build system. `STACK_LINE`, `ROUTING_NOTES` and `DEV_URL` are capability
tokens, because only the router knows them, and they land in a file
(`docs/rules/routing.md`) that the project type ships. `STACK_SUMMARY` and
`STACK_DESCRIPTION` come from the capability on a web project and from the
project type on a Rust one, because that is where the answer lives in each
case.

`NEXT_STEPS` is the exception that never reaches a file: it is a project-type
token the CLI itself reads to print what to run once the project exists
(`cli/src/app.rs`), because a Rust project is not started with `pnpm install`.

## The command-line surface

Every answer the scaffolder needs has a flag, and every flag is optional:
anything the user leaves out is asked for. `--yes` (alias `--no-input`) removes
the questions, at which point the options with no sensible default become
required.

| Answer | Flag | Missing under `--yes` |
| --- | --- | --- |
| Project name | `-n`, `--name <NAME>` | error |
| Location | `-d`, `--dir <DIR>` | defaults to `.` |
| Project type | `-p`, `--project-type <KEY>` | error |
| Capability | `-c`, `--capability <KEY>` | error, if a required choice is unmade |
| Template repo | `--repo <URL>` | defaults to `DEFAULT_TEMPLATE_REPO` |

`--capability` repeats (`-c a -c b`) and also takes one comma-separated value
(`-c a,b`). Passing it at all is the whole answer rather than a first draft the
questions top up, so a command that names its capabilities behaves the same
with and without `--yes`.

Three modules implement that contract:

- `cli/src/cli/args.rs` defines the flags (clap derive) and `ensure_answerable`,
  which rejects an incomplete `--yes` command up front, naming every missing
  flag at once. It checks `--name` and `--project-type` only: whether a
  capability is required depends on the project type, which is not known until
  the templates are cloned.
- `cli/src/cli/resolve.rs` resolves each answer from the same three sources in
  order: the flag, a question, a default. It is the only place that knows
  prompting can be off, and it is where the whole combination is validated, so
  nothing downstream has to wonder whether the tags it was handed go together.
- `cli/src/cli/prompts.rs` holds the questions themselves (`dialoguer`).
  Enum-shaped answers are lists, never free text: a `Select` for the project
  type, one `Select` per required choice group, and a `MultiSelect` for
  whatever is optional (there is nothing optional today, so that question is
  skipped). A user therefore cannot type a combination the catalog would
  reject, and a value passed as a flag is validated by the same code that
  validates a typed one.

Adding an answer means adding a field in `args.rs`, a resolver method, and
(when it is not free text) a list prompt. Adding a *required* one also means
adding it to `ensure_answerable`.

Every refusal names the real choices: an unknown `--project-type` lists the
project types this template repository ships, an unknown `--capability` lists
the ones the chosen project type can use, a capability that does not fit says
which project types or languages it does fit, a conflicting pair names both
tags, and an unmade required choice lists that group's members and the flag to
pass.

## Runtime flow

The binary is a thin client (`cli/src/app.rs`):

1. Check `git` is available, and reject an incomplete `--yes` command. Both
   happen before anything is asked, created, or fetched.
2. Resolve the project name and location; resolve and reject an existing target.
3. Clone the template repo shallowly into a temp dir (`template/fetch.rs`).
   Clone failures are classified as offline, access/config, or generic.
4. Discover the catalog from the clone (`catalog/`): every `project-type.json`
   under `templates/project-types/` and every `capability.json` under
   `templates/capabilities/`, each sorted by `order` then `name`. A repository
   that declares no project type is malformed and fails here; one that declares
   no capability is fine. Discovery comes after the clone because the valid
   choices are whatever the template repository ships, which is also why an old
   binary can build a project type it has never heard of.
5. Resolve the selection: the project type, then the capabilities, then the
   check that the combination holds together (every capability exists and fits,
   no two exclude each other, every required choice is made). Building the
   `Selection` is the check: one that exists is a combination that can be built,
   so everything downstream reads it without revalidating.
6. Select the agent skills those tags name. This happens before composition,
   because the new project's update script has to be told which of them install
   themselves.
7. Compose into the target directory; on error, remove the partial output.
8. Install the selected skills by running `npx skills add` once per spec inside
   the new project (`skills/install.rs`). This is best-effort: a failure prints
   a warning naming the commands to re-run and keeps the project.
9. Initialize the target as a git repository on `main` with a single initial
   commit (`git_init.rs`). The branch is named explicitly, because `git init`
   otherwise falls back to `master` on a machine with no `init.defaultBranch`.
   This is best-effort too: if git init or the commit fails (for example, no
   configured identity), the run prints a warning and keeps the project rather
   than discarding it. A generic fallback identity is used for the commit only
   when the user has none configured.
10. Print what was built (the project type's name plus each capability's) and
    the next steps that project type declares. The temp clone is deleted when
    the run ends, so the user only ever sees the finished project.

Because the templates are fetched at runtime, an old binary still builds from
the newest templates.

`cli/tests/` covers that flow end to end. `non_interactive_test.rs` drives the
real binary with `--yes` against a fixture template repo cloned from a local
path, so the non-interactive path and every refusal above are exercised without
a network. `real_templates_test.rs` composes the **real** templates, one test
per combination: `typescript:web` with each router, and `rust:cli` on its own.

## Adding a project type

1. Create `templates/project-types/<slug>/` with:
   - `project-type.json` declaring `key` (`<language>:<product>`), `name`,
     `description`, `language`, `order`, and its `tokens`;
   - `files/` holding everything a project of this type starts from, its
     `docs/rules/*.md` included;
   - `package.json` only if the language has one. A Rust project type ships
     none and composes without a manifest.
2. Give a value to every `{{TOKEN}}` the base files use, or
   `real_templates_test` fails on an unfilled token. Today that means
   `STACK_SUMMARY`, `STACK_DESCRIPTION`, `PROJECT_TYPE_RULES`,
   `BUILD_TEST_BLOCK`, `PROJECT_TYPE_IGNORES`, `SKILLS_LIST_COMMAND`,
   `SKILLS_INSTALL_COMMAND`, `SKILLS_UPDATE_COMMAND` and `SKILLS_RESTORE_NOTE`,
   minus any that a capability the project type requires supplies instead. Add
   `NEXT_STEPS` as well, so a run ends by printing how to start this kind of
   project.
3. Add the key to `projectTypes` in
   [`skills-manifest.json`](../skills-manifest.json) with the skills its
   language deserves. A tag with no entry fails `skills_manifest_test`.
4. Wire `scripts/skills/update-skills.sh` to the language's task runner, so
   people type the name they expect. See "Wiring the update script into a
   project type" below.
5. Run `cargo test`. Discovery is data-driven, so no Rust change is needed to
   make the new project type selectable.

## Adding a capability

1. Create `templates/capabilities/<slug>/` with:
   - `capability.json` declaring `key`, `name`, `description`, `order`, and its
     compatibility: the `projectTypes` and/or `languages` it fits (leave both
     out to fit everything) and the capabilities it `conflictsWith`;
   - `files/` with whatever it adds or overrides, if anything;
   - `package.json` with only its own dependencies, if any.
2. Declare a conflict on both sides. One side is enough for the rule, but the
   pair only reads as one question when each names the other.
3. Give a value to every token it owns, and add the `{{TOKEN}}` to the shared
   file rather than copying that file into the capability.
4. Add the key to `capabilities` in `skills-manifest.json`, with an empty list
   when it brings no skill of its own. The entry is required either way: an
   undeclared tag fails `skills_manifest_test`.
5. Run `cargo test`. A capability that conflicts with an existing one joins its
   choice group automatically, becoming one more option in that group's
   `Select` rather than a new question.

## Skills: one list per tag

A generated project receives the skills its tags name in
[`skills-manifest.json`](../skills-manifest.json). The manifest has three
sections, mirroring the tag model:

| Section | Keyed by | Who gets it |
| --- | --- | --- |
| `global` | nothing | every generated project |
| `projectTypes` | a project type key | the projects of that type |
| `capabilities` | a capability key | the projects that chose it |

Selection is additive and deduplicated: `global`, then the project type's list,
then each chosen capability's list in order, each spec kept at its first
position so it is installed once. `typescript:web` brings the TypeScript, web
testing, and Mantine skills; `rust:cli` brings the Rust set. Both TanStack
capabilities currently bring none of their own, which is a normal state for a
tag: the entry has to exist so an undeclared tag is caught, and an empty list
says the tag adds nothing.

Compatibility is not repeated here. Which tags exist, and which of them
combine, is declared in `templates/`; this file only says what each tag
installs. `cli/tests/skills_manifest_test.rs` checks that the two agree in both
directions (every tag the templates declare has an entry, every entry names a
tag the templates declare) and guards the entries themselves: every spec well
formed, no spec listed twice under one tag or repeated from `global`, and
`global` present and non-empty.

Each entry is a **source spec** handed to `npx skills add` verbatim, so it has
to resolve to exactly one skill: `owner/repo` when the repository's SKILL.md is
at its root, `owner/repo/path/to/skill` otherwise.

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
generated project's own skills tooling keeps working unchanged: a teammate
cloning the project later restores exactly the same set, with `pnpm install` on
a web project (through its `postinstall` hook) or `just skills-install` on a
Rust one. Think "node_modules installed on creation", not "bundled".

The code lives in `cli/src/skills/`: `manifest.rs` selects the specs for a
project type and its capabilities, deduplicated; `commands.rs` builds the
commands; `install.rs` runs them. Installing is best-effort like the git
initialization: a failure prints a warning naming the commands to re-run, and
never costs the user the project they just created.

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
`SELF_INSTALLING_SKILLS="impeccable"`, and in the TypeScript project type's
skills constants. A project whose tags select no self-installing skill gets an
empty list and skips that step.

That script is the generated project's single "update every skill" entry point:
`npx skills update --project` for everything in the lock, then each
self-installing skill through its own CLI. It is plain `sh` rather than part of
the TypeScript tooling because every generated project needs it, including the
ones that are not TypeScript at all.

Adding another self-installing skill means teaching
`cli/src/skills/commands.rs` about it (so the scaffolder installs it and the
token names it) and `scripts/skills/update-skills.sh` how to update it.

### Wiring the update script into a project type

The script is the implementation; a project type only gives it the name people
working in that language expect to type. It stays directly runnable
(`./scripts/skills/update-skills.sh`), so the wiring is convenience, never a
dependency.

| Project type | Entry point | Where it is declared |
| --- | --- | --- |
| `typescript:web` | `pnpm skills:update` | `scripts` in `package.json` |
| `rust:cli` | `just skills-update` | the `justfile` at the project root |

Rust has no built-in task registry, which is exactly why `just` and
`cargo xtask` exist. `just` is the choice here: it is itself a Rust tool
(`cargo install just`), it is the task runner most Rust projects reach for, a
task costs one line, and `just --list` makes the tasks discoverable the way
`package.json` scripts are. The Rust project type's `justfile` is its whole
task surface (`build`, `run`, `test`, `lint`, `fmt`, `check`, `install`), and
the skills wiring is two more recipes in it:

```just
# Restore the agent skills pinned in skills-lock.json (run once after cloning).
skills-install:
    npx --yes skills experimental_install

# Update every agent skill this project has.
skills-update:
    ./scripts/skills/update-skills.sh
```

`cargo xtask` is the alternative, and it earns its keep when a task needs real
Rust code (codegen, packaging, release). A whole workspace crate whose only job
is to exec a shell script does not. A `Makefile` needs nothing installed, but it
is not what a modern Rust project reaches for, and its tab rules are a trap for
a one-line recipe.
