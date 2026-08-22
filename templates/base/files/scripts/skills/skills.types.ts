/**
 * Shared types for the skills tooling under `scripts/skills`.
 *
 * Two tools install agent skills into this project and the CLI wraps both:
 * `npx skills` for everything listed in `skills-lock.json`, and `npx
 * impeccable` for the one skill that ships its own installer.
 */

/** Which tool installs and updates a skill. */
export type SkillManagerName = "skills" | "impeccable";

/** A skill that is currently present on disk in this project. */
export type InstalledSkill = {
  name: string;
  manager: SkillManagerName;

  /** Repository the skill came from, when the manager reports one. */
  source: string | undefined;

  /** Display names of the agent frontends that can see the skill. */
  agents: string[];

  /** Absolute path of the real skill directory. */
  path: string | undefined;
};

/** The skills that one repository in `skills-lock.json` provides. */
export type SkillSourceGroup = {
  source: string;
  skillNames: string[];
};

/** One row of `pnpm skills` output. */
export type SkillListingRow = {
  name: string;
  manager: SkillManagerName;
  source: string | undefined;
  agents: string[];

  /**
   * False when `skills-lock.json` asks for the skill but it is not on disk,
   * which is what a project looks like before `pnpm install` restores it.
   */
  isInstalled: boolean;
};

/** A command the CLI shells out to, with a label for progress output. */
export type CommandSpec = {
  label: string;
  command: string;
  args: string[];
};

/** What a finished command reported back. */
export type CommandResult = {
  exitCode: number;
  stdout: string;
  stderr: string;
};

/**
 * Runs a command and resolves with its result rather than throwing on a
 * non-zero exit. Injected in tests so no test ever reaches the network.
 */
export type CommandRunner = (
  spec: Readonly<CommandSpec>,
  options?: Readonly<{ streamOutput?: boolean }>,
) => Promise<CommandResult>;
