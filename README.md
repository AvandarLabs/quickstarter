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

It asks for whatever you did not pass on the command line:

1. **Project name** (`--name`, required).
2. **Where to create it** (`--dir`, defaults to the current directory).
3. **Which stack** (`--stack`): TanStack Router (SPA) or TanStack Start
   (SSR + server functions). The list is arrow-key (or `j`/`k`) selectable.

Then it clones the latest templates, composes your project, and writes only the
finished files to `<dir>/<name>`.

### Passing answers up front

Every question has a flag, so you can answer some, all, or none of them:

```sh
# Answer one question up front, get asked the rest.
./bootstrapNewRepo.sh --name "My App"

# Answer everything and skip the interview entirely.
./bootstrapNewRepo.sh --name "My App" --dir ~/src --stack router --yes
```

| Flag | Meaning |
| --- | --- |
| `-n`, `--name <NAME>` | Project name. Required under `--yes`. |
| `-d`, `--dir <DIR>` | Where to create it. Defaults to the current directory. |
| `-s`, `--stack <STACK>` | Stack key: `router` or `start`. Required under `--yes`. |
| `--repo <URL>` | Template repository to clone. |
| `-y`, `--yes` | Never prompt. Also spelled `--no-input`. |

`--yes` takes the options exactly as given: optional ones fall back to their
defaults, and a missing required one is an error (naming every missing flag at
once) rather than a question. Without `--yes`, nothing is required up front,
because anything missing is simply asked for.

Run `./bootstrapNewRepo.sh --help` for the full list.

The Rust binary is a thin client: it clones this repository fresh on every run
into a temporary directory (never into your project), so it always builds from
the newest templates. The launcher rebuilds the binary every run too (cargo is
incremental, so an unchanged tree costs a fraction of a second), which means a
change under `cli/src` can never be silently ignored. It requires `git` and an
internet connection; it will tell you if either is missing.

The generated project also arrives with a `skills-lock.json` holding the
curated agent skills a new project should have. Its first `pnpm install`
installs them; see the skills section of
[`docs/scaffolder.md`](docs/scaffolder.md).

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
