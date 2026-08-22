# {{PROJECT_NAME}}

{{STACK_DESCRIPTION}}

## Stack

- **Build tool:** [Vite](https://vite.dev)
- **Language:** TypeScript
- **UI framework:** [React](https://react.dev) v19
- **Component library:** [Mantine](https://mantine.dev) v9
- **Routing:** {{STACK_LINE}}

## Getting started

This project uses [pnpm](https://pnpm.io).

1. Install dependencies:

   ```sh
   pnpm install
   ```

2. Start the dev server:

   ```sh
   pnpm dev
   ```

   The app runs at {{DEV_URL}}.

## Scripts

| Script                | Description                                     |
| --------------------- | ----------------------------------------------- |
| `pnpm dev`            | Start the Vite dev server with hot reload.      |
| `pnpm build`          | Type-check (`tsc -b`) and build for production. |
| `pnpm preview`        | Preview the production build locally.           |
| `pnpm type-check`     | Run the TypeScript compiler without emitting.   |
| `pnpm lint`           | Lint with oxlint.                               |
| `pnpm format`         | Format with oxfmt.                              |
| `pnpm test`           | Run the test suite with vitest.                 |
| `pnpm skills`         | List this project's agent skills.               |
| `pnpm skills:install` | Install any locked agent skill that is missing. |
| `pnpm skills:update`  | Update every agent skill to its latest version. |

## Routing

{{ROUTING_NOTES}}

## Agent skills

Agent skills are not tracked in git. `skills-lock.json` is, and `pnpm install`
installs whatever it lists that is missing, so a fresh clone needs no extra
step. Run `pnpm skills` to see what is installed and `pnpm skills:update` to
upgrade them. See [`docs/skills.md`](docs/skills.md) for how the two skill
managers divide the work.

## Agent rules

Coding conventions live in `AGENTS.md`, which is the single source of truth.
`CLAUDE.md` (Claude Code) and `.cursor/rules/agents.mdc` (Cursor) are symlinks
to it, and it is also the file the Codex CLI reads natively. Update `AGENTS.md`
and every tool stays in sync.
