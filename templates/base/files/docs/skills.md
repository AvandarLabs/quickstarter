# Agent skills

This project ships a curated set of agent skills so every coding agent working
here starts with the same tooling. This document explains how they get onto
disk and who owns what.

## Two managers, one wrapper

Skills come from two tools that know nothing about each other:

- **`npx skills`** installs everything listed in `skills-lock.json`. It writes
  the real skill directories to `.agents/skills/`, symlinks them into
  `.claude/skills/` for Claude Code, and relies on Cursor, OpenCode, and Codex
  reading `.agents/skills/` natively.
- **`npx impeccable`** installs the `impeccable` skill. It ships its own
  installer and writes a copy per frontend (`.agents`, `.claude`, `.cursor`,
  `.opencode`) plus its hook manifests, so it is never in `skills-lock.json`.

`scripts/skills/SkillsCli.ts` wraps both so there is one place to ask what is
installed and one place to bring it up to date:

| Command              | What it does                                                  |
| -------------------- | ------------------------------------------------------------- |
| `pnpm skills`        | Merged listing of every skill, flagging any that are missing. |
| `pnpm skills:update` | Refresh every locked source plus impeccable to their latest.  |

Never create the per-frontend symlinks or copies by hand. Each manager owns the
layout its own frontends expect, and hand-made links drift the moment either
tool changes.

## What git tracks

**Only `skills-lock.json`.** The installed directories (`.agents/`,
`.claude/skills/`, `.cursor/skills/`, `.opencode/`, `.codex/`) are gitignored:
together they are around 14MB and 600+ files of vendored content, most of it
impeccable's four copies of itself.

The lock is therefore the manifest of what this project wants, and
`pnpm install` is what makes the working tree match it:

- `postinstall` runs `SkillsCli update --only-missing`, which installs whatever
  the lock asks for that is not already on disk. A complete project makes no
  network calls at all, so adding a dependency stays fast.
- The analogy is a package lockfile: `pnpm install` materializes what is
  locked, and `pnpm skills:update` is the deliberate "go get the latest" step.
- `CI=true` or `SKIP_SKILLS_INSTALL=1` turns the restore off. No agent frontend
  runs in CI, so the network cost there buys nothing.

## Changing the skill set

Add or remove skills with the manager, not by editing directories:

```sh
npx skills add <owner>/<repo> --skill <name> --agent claude-code cursor opencode codex
npx skills remove <name>
```

Both commands update `skills-lock.json`. Commit that change: it is what every
other clone and CI checkout installs from.

`skills-lock.json` is seeded by the scaffolder from quickstarter's `produced`
skill set, so a fresh project starts with the curated list rather than an
empty one.
