import {
  printError,
  printHeading,
  printInfo,
  printSuccess,
  printWarning,
} from "../cliOutput/cliOutput";
import { IMPECCABLE_SKILL_NAME, SKIP_SKILLS_ENV_VAR_NAMES } from "../constants";
import { readInstalledSkillNamesFromDisk } from "../installedSkills/readInstalledSkills";
import { runCommand } from "../runCommand/runCommand";
import { readSkillSourceGroups } from "../skillsLock/readSkillsLock";
import { createSkillsSyncPlan } from "./createSkillsSyncPlan/createSkillsSyncPlan";
import type { CommandRunner, CommandSpec } from "../skills.types";
import type { SkillsSyncMode } from "./createSkillsSyncPlan/createSkillsSyncPlan";

/** Options shared by both sync verbs. */
export type SkillsSyncOptions = {
  /**
   * Postinstall mode: stay quiet unless something needs saying, and let the
   * environment turn the run off entirely.
   */
  quiet?: boolean;

  runner?: CommandRunner;
  projectRootPath?: string;
  environment?: Record<string, string | undefined>;
};

/** What a sync actually did. */
export type SkillsSyncResult = {
  /** True when at least one command ran. */
  didRun: boolean;

  /** True when an environment variable turned the run off. */
  wasSkipped: boolean;

  /** Labels of the commands that failed; the rest still ran. */
  failedLabels: string[];
};

/** Number of trailing output lines to show for a command that failed. */
const FAILURE_DETAIL_LINE_COUNT = 5;

const HEADINGS: Record<SkillsSyncMode, string> = {
  install: "Installing agent skills",
  update: "Updating agent skills",
};

function _findSkipReason(
  environment: Readonly<Record<string, string | undefined>>,
): string | undefined {
  return SKIP_SKILLS_ENV_VAR_NAMES.find((variableName) => {
    const value = environment[variableName];
    return value !== undefined && value !== "" && value !== "false";
  });
}

/**
 * Runs the planned commands one at a time, collecting the labels that failed.
 *
 * The reduce chains the promises so the commands stay sequential: both
 * managers write `.agents/skills` and the same per-frontend link directories,
 * so parallel installs would race each other. A failure is recorded and the
 * run continues, because one unreachable repository should not cost the user
 * every other skill.
 */
function _runCommands(
  commands: readonly CommandSpec[],
  options: Readonly<{ runner: CommandRunner; quiet: boolean }>,
): Promise<string[]> {
  const { runner, quiet } = options;

  return commands.reduce(async (previousFailedLabels, command) => {
    const failedLabels = await previousFailedLabels;
    if (!quiet) {
      printInfo(`- ${command.label}`);
    }

    const result = await runner(command, { streamOutput: !quiet });
    if (result.exitCode === 0) {
      return failedLabels;
    }

    printError(`Failed: ${command.label}`);
    const detail = (result.stderr || result.stdout).trim();
    if (detail !== "") {
      printInfo(
        detail.split("\n").slice(-FAILURE_DETAIL_LINE_COUNT).join("\n"),
      );
    }
    return [...failedLabels, command.label];
  }, Promise.resolve<string[]>([]));
}

/**
 * Brings this project's agent skills in line with `skills-lock.json`.
 *
 * Both managers are driven here: `npx skills` for everything in the lock, and
 * `npx impeccable` for the skill that installs itself. Nothing is symlinked by
 * hand, because each tool owns the layout it expects.
 */
async function _syncSkills(
  mode: SkillsSyncMode,
  options: Readonly<SkillsSyncOptions>,
): Promise<SkillsSyncResult> {
  const {
    quiet = false,
    runner = runCommand,
    projectRootPath = process.cwd(),
    environment = process.env,
  } = options;

  if (quiet && _findSkipReason(environment) !== undefined) {
    return { didRun: false, wasSkipped: true, failedLabels: [] };
  }

  const [sourceGroups, installedSkillNames] = await Promise.all([
    readSkillSourceGroups({ projectRootPath }),
    readInstalledSkillNamesFromDisk({ projectRootPath }),
  ]);

  const plan = createSkillsSyncPlan({
    mode,
    sourceGroups,
    installedSkillNames,
    isImpeccableInstalled: installedSkillNames.includes(IMPECCABLE_SKILL_NAME),
  });

  if (plan.commands.length === 0) {
    if (!quiet) {
      printSuccess("Every skill in skills-lock.json is already installed.");
    }
    return { didRun: false, wasSkipped: false, failedLabels: [] };
  }

  if (quiet) {
    printInfo(
      `Installing agent skills (${plan.commands.length} command(s)). ` +
        "Set SKIP_SKILLS_INSTALL=1 to turn this off.",
    );
  } else {
    printHeading(HEADINGS[mode]);
  }

  const failedLabels = await _runCommands(plan.commands, { runner, quiet });

  if (failedLabels.length > 0) {
    printWarning(
      `${failedLabels.length} of ${plan.commands.length} skill commands failed.`,
    );
  } else if (!quiet) {
    printSuccess("Agent skills are up to date.");
  }

  return { didRun: true, wasSkipped: false, failedLabels };
}

/**
 * Installs the locked skills that are not on disk, and leaves the rest alone.
 *
 * This is what `pnpm install` runs, and it deliberately behaves the way
 * `pnpm install` does for packages: it makes the working tree match the lock
 * without upgrading anything already installed. A complete project makes no
 * network calls at all.
 *
 * @param options.quiet Postinstall mode: minimal output, never fails.
 * @param options.runner Command runner, overridden in tests.
 * @param options.projectRootPath Directory holding `skills-lock.json`.
 * @param options.environment Environment to read the skip flags from.
 * @returns What ran, what was skipped, and what failed.
 */
export function installSkills(
  options: Readonly<SkillsSyncOptions> = {},
): Promise<SkillsSyncResult> {
  return _syncSkills("install", options);
}

/**
 * Updates every skill to the latest version its source offers.
 *
 * Unlike {@link installSkills} this always reaches the network: it is the
 * deliberate "go get the newest" step, so it re-fetches each locked source and
 * refreshes impeccable even when nothing is missing.
 *
 * @param options.quiet Minimal output, never fails.
 * @param options.runner Command runner, overridden in tests.
 * @param options.projectRootPath Directory holding `skills-lock.json`.
 * @param options.environment Environment to read the skip flags from.
 * @returns What ran, what was skipped, and what failed.
 */
export function updateSkills(
  options: Readonly<SkillsSyncOptions> = {},
): Promise<SkillsSyncResult> {
  return _syncSkills("update", options);
}
