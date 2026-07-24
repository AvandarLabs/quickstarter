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

| Script            | Description                                     |
| ----------------- | ----------------------------------------------- |
| `pnpm dev`        | Start the Vite dev server with hot reload.      |
| `pnpm build`      | Type-check (`tsc -b`) and build for production. |
| `pnpm preview`    | Preview the production build locally.           |
| `pnpm type-check` | Run the TypeScript compiler without emitting.   |
| `pnpm lint`       | Lint with oxlint.                               |
| `pnpm format`     | Format with oxfmt.                              |

## Routing

{{ROUTING_NOTES}}

## Agent rules

Coding conventions live in `AGENTS.md`, which is the single source of truth.
`CLAUDE.md` (Claude Code) and `.cursor/rules/agents.mdc` (Cursor) are symlinks
to it, and it is also the file the Codex CLI reads natively. Update `AGENTS.md`
and every tool stays in sync.
