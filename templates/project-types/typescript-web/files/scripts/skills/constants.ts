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
 * The raw list the scaffolder substituted when this project was created:
 * space separated npm package names.
 */
const SELF_INSTALLING_SKILLS_TOKEN = "{{SELF_INSTALLING_SKILLS}}";

/**
 * The skills this project has that `npx skills` does not manage. Each one
 * ships a CLI that installs and updates itself, so it never appears in
 * `skills-lock.json` and has to be driven through that CLI instead.
 *
 * The scaffolder wrote this list from its capability manifest, which is why it
 * is data rather than an assumption: a project whose capabilities select no
 * such skill gets an empty list, and that is a normal state, not a fault.
 */
export const SELF_INSTALLING_SKILL_NAMES: readonly string[] =
  SELF_INSTALLING_SKILLS_TOKEN.split(" ").filter((skillName) => {
    return skillName !== "";
  });

/**
 * The skill behind the `impeccable` CLI. Whether this project has it is not
 * assumed: it is in {@link SELF_INSTALLING_SKILL_NAMES} only when the
 * scaffolder selected it.
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
 * The script that updates every skill this project has, whatever installed it.
 *
 * Updating lives in shell rather than in TypeScript so every generated
 * project, including one with no TypeScript tooling at all, updates its skills
 * the same way. The path is relative to the project root, which is where the
 * package scripts run.
 */
export const UPDATE_SKILLS_SCRIPT_PATH = "./scripts/skills/update-skills.sh";

/**
 * Environment variables that turn the `postinstall` restore off. `CI` is
 * included because no agent frontend runs in CI, so the network cost there
 * buys nothing.
 */
export const SKIP_SKILLS_ENV_VAR_NAMES = ["CI", "SKIP_SKILLS_INSTALL"] as const;
