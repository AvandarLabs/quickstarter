# {{PROJECT_NAME}}

{{STACK_DESCRIPTION}}

## Getting started

{{BUILD_TEST_BLOCK}}

## Agent skills

Agent skills are not tracked in git, but `skills-lock.json` is.
{{SKILLS_RESTORE_NOTE}}
Run `{{SKILLS_LIST_COMMAND}}` to see what is installed and
`{{SKILLS_UPDATE_COMMAND}}` to upgrade them. See
[`docs/skills.md`](docs/skills.md) for how the two skill managers
divide the work.

## Agent rules

Coding conventions live in `AGENTS.md`, which is the single source of truth.
`CLAUDE.md` (Claude Code) and `.cursor/rules/agents.mdc` (Cursor) are symlinks
to it, and it is also the file the Codex CLI reads natively. Update `AGENTS.md`
and every tool stays in sync.
