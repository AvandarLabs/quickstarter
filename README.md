# quickstarter

A **project scaffolder**. It builds a fresh project by composing reusable
template layers, rather than shipping a single clone-and-go template. Choose
what kind of project you want (a TypeScript web app, or a Rust command-line
tool) plus any libraries layered on top, and it assembles that combination for
you.

## Why a scaffolder instead of a template to clone

A single template forces one combination. Supporting many by keeping a full
copy of the repo per combination does not scale: the day you bump Mantine or
change the theme, you have to edit every copy. This repo avoids that by storing
each concern **once** and composing:

- **`templates/base/`** - what every generated project gets whatever its
  language: the agent rules, the README, the shared `.gitignore`, and the
  skills tooling.
- **`templates/project-types/<slug>/`** - one folder per language and product,
  holding its source, config, docs, and dependencies. Bump Mantine in
  `typescript-web` and every web project gets it.
- **`templates/capabilities/<slug>/`** - one folder per library layered on top,
  holding only what that library adds.

See [`docs/scaffolder.md`](docs/scaffolder.md) for the architecture.

## Project types and capabilities

A generated repo carries **tags**. It has exactly one **project type**, which
decides its language, its build system, and the files it starts from, and any
number of **capabilities**, each a library or framework layered on top.

| Project type | What you get |
| --- | --- |
| `typescript:web` | Vite + React + Mantine in TypeScript, with oxlint, oxfmt, vitest, and the `pnpm` scripts that run them. Needs a router. |
| `rust:cli` | A command-line tool with a terminal UI: cargo, clap, ratatui over crossterm, anyhow, a `justfile` of tasks, and an `./install.sh`. No capabilities yet. |

| Capability | What it adds |
| --- | --- |
| `tanstack-router` | A client-side single-page app, no server. Needs `typescript:web`. |
| `tanstack-start` | Server-side rendering and server functions. Needs `typescript:web`. |

Those two exclude each other, so a `typescript:web` project chooses exactly one
of them. That is the question the scaffolder asks, and the one `--yes` requires
you to have answered.

## Usage

```sh
./bootstrapNewRepo.sh
```

It asks for whatever you did not pass on the command line:

1. **Project name** (`--name`, required).
2. **Where to create it** (`--dir`, defaults to the current directory).
3. **Which project type** (`--project-type`): TypeScript web app or Rust CLI.
4. **Which capabilities** (`--capability`): one question per set of mutually
   exclusive ones, so a web project is asked which TanStack it wants. A Rust
   CLI is asked nothing here, because it has no capabilities.

Every list is arrow-key (or `j`/`k`) selectable. Then it clones the latest
templates, composes your project, and writes only the finished files to
`<dir>/<name>`.

### Passing answers up front

Every question has a flag, so you can answer some, all, or none of them:

```sh
# Answer one question up front, get asked the rest.
./bootstrapNewRepo.sh --name "My App"

# A TypeScript web app, with no questions at all.
./bootstrapNewRepo.sh --name "My App" --dir ~/src \
  --project-type typescript:web --capability tanstack-router --yes

# A Rust command-line tool.
./bootstrapNewRepo.sh --name "My Tool" --dir ~/src \
  --project-type rust:cli --yes
```

| Flag | Meaning |
| --- | --- |
| `-n`, `--name <NAME>` | Project name. Required under `--yes`. |
| `-d`, `--dir <DIR>` | Where to create it. Defaults to the current directory. |
| `-p`, `--project-type <KEY>` | `typescript:web` or `rust:cli`. Required under `--yes`. |
| `-c`, `--capability <KEY>` | A capability, such as `tanstack-router`. Repeatable, and takes a comma-separated list. Required under `--yes` when a choice is left to make. |
| `--repo <URL>` | Template repository to clone. |
| `-y`, `--yes` | Never prompt. Also spelled `--no-input`. |

`--yes` takes the options exactly as given: optional ones fall back to their
defaults, and a missing required one is an error (naming every missing flag at
once) rather than a question. Without `--yes`, nothing is required up front,
because anything missing is simply asked for.

Every refusal names the real choices, so an unknown project type, an unknown
capability, a capability that does not fit the project type, or two that
exclude each other all tell you what you can pass instead.

Run `./bootstrapNewRepo.sh --help` for the full list.

The Rust binary is a thin client: it clones this repository fresh on every run
into a temporary directory (never into your project), so it always builds from
the newest templates. The launcher rebuilds the binary every run too (cargo is
incremental, so an unchanged tree costs a fraction of a second), which means a
change under `cli/src` can never be silently ignored. It requires `git` and an
internet connection; it will tell you if either is missing.

The generated project also arrives with its agent skills already installed.
Which ones it gets depends on the tags you chose: the scaffolder keeps a
manifest of skills keyed by tag (the global ones, the project type's, and each
capability's) and runs `npx skills` for the ones your project selects, right
after composing it. The `skills-lock.json` those installs write is part of the
project's first commit, so a later clone restores the same set: `pnpm install`
on a web project, `just skills-install` on a Rust one. See the skills section
of [`docs/scaffolder.md`](docs/scaffolder.md).

To point at a different template repository:

```sh
./bootstrapNewRepo.sh --repo https://github.com/AvandarLabs/quickstarter.git
```

## Repository layout

```
.
├── bootstrapNewRepo.sh      # launcher: builds and runs the CLI
├── cli/                     # the Rust scaffolder (see cli/src)
├── templates/
│   ├── base/files/          # what every project gets, any language
│   ├── project-types/
│   │   ├── typescript-web/  # Vite + React + Mantine + its package.json
│   │   └── rust-cli/        # cargo + clap + ratatui, no package.json
│   └── capabilities/
│       ├── tanstack-router/ # TanStack Router (SPA)
│       └── tanstack-start/  # TanStack Start (SSR)
├── skills-manifest.json     # which agent skills each tag installs
└── docs/scaffolder.md       # architecture, and how to add a tag
```

## Development

```sh
cd cli
cargo test        # unit + integration tests (composes the real templates)
cargo clippy
```
