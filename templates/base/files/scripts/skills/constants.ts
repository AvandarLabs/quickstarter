/** Where `npx skills` puts the real skill directories. */
export const AGENT_SKILLS_DIR_PATH = ".agents/skills";

/** The only skills file this project tracks in git. */
export const SKILLS_LOCK_FILE_NAME = "skills-lock.json";

/**
 * The agent frontends every skill is installed for. `npx skills` symlinks
 * `.claude/skills` for Claude Code; Cursor, OpenCode, and Codex read
 * `.agents/skills` directly.
 */
export const SKILL_AGENT_NAMES = [
  "claude-code",
  "cursor",
  "opencode",
  "codex",
] as const;

/**
 * The one skill `npx skills` does not manage: it ships an `impeccable` CLI
 * that installs and updates itself, so the wrapper drives that tool instead.
 */
export const IMPECCABLE_SKILL_NAME = "impeccable";

/** Repository behind the `impeccable` CLI, shown when listing skills. */
export const IMPECCABLE_SKILL_SOURCE = "pbakaus/impeccable";

/** Provider names `impeccable` uses for the same four frontends. */
export const IMPECCABLE_PROVIDER_NAMES = [
  "claude",
  "cursor",
  "opencode",
  "codex",
] as const;

/**
 * Environment variables that turn the `postinstall` restore off. `CI` is
 * included because no agent frontend runs in CI, so the network cost there
 * buys nothing.
 */
export const SKIP_SKILLS_ENV_VAR_NAMES = ["CI", "SKIP_SKILLS_INSTALL"] as const;
