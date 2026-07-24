# quickstarter

A **project scaffolder**. It builds a fresh front-end project by composing
reusable template layers, rather than shipping a single clone-and-go template.
Choose a stack (currently TanStack Router or TanStack Start) and it assembles
the right combination for you.

## Why a scaffolder instead of a template to clone

A single template forces one stack. Supporting many stacks by keeping a full
copy of the repo per combination does not scale: the day you bump Mantine or
change the theme, you have to edit every copy. This repo avoids that by storing
each concern **once** and composing:

- **`templates/base/`** - the single source of truth for everything shared:
  the Mantine theme, lint/format config, TypeScript config, docs, and the
  shared dependencies. Bump Mantine here and every generated project gets it.
- **`templates/modules/<stack>/`** - one folder per stack choice. Each holds
  only the files and dependencies that differ for that stack.

See [`docs/scaffolder.md`](docs/scaffolder.md) for the architecture.

## Usage

```sh
./bootstrapNewRepo.sh
```

It asks three questions:

1. **Project name** (required).
2. **Where to create it** (defaults to the current directory).
3. **Which stack**: TanStack Router (SPA) or TanStack Start (SSR + server
   functions).

Then it clones the latest templates, composes your project, and writes only the
finished files to `<location>/<name>`.

The Rust binary is a thin client: it clones this repository fresh on every run
into a temporary directory (never into your project), so even an old binary
always builds from the newest templates. It requires `git` and an internet
connection; it will tell you if either is missing.

To point at a different template repository:

```sh
./bootstrapNewRepo.sh --repo https://github.com/AvandarLabs/quickstarter.git
```

## Repository layout

```
.
├── bootstrapNewRepo.sh     # launcher: builds (first run) and runs the CLI
├── cli/                    # the Rust scaffolder (see cli/src)
├── templates/
│   ├── base/               # shared source, config, docs, and package.json
│   │   ├── files/          #   copied verbatim into every project
│   │   └── package.json    #   shared dependencies (the merge base)
│   └── modules/
│       ├── router/         # TanStack Router (SPA) layer
│       └── start/          # TanStack Start (SSR) layer
└── docs/scaffolder.md      # architecture and how to add a stack
```

## Development

```sh
cd cli
cargo test        # unit + integration tests (composes the real templates)
cargo clippy
```
