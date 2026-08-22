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
import { createSkillsUpdatePlan } from "./createSkillsUpdatePlan/createSkillsUpdatePlan";
import type { CommandRunner, CommandSpec } from "../skills.types";

type UpdateSkillsOptions = {
  /**
   * Restore absent skills and nothing else. This is what `postinstall` uses,
   * so a complete project costs no network when a dependency is added.
   */
  onlyMissing?: boolean;

  /**
   * Postinstall mode: stay quiet unless something needs saying, and never
   * fail the surrounding `pnpm install`.
   */
  quiet?: boolean;

  runner?: CommandRunner;
  projectRootPath?: string;
  environment?: Record<string, string | undefined>;
};

/** What an update actually did. */
export type UpdateSkillsResult = {
  /** True when at least one command ran. */
  didRun: boolean;

  /** True when an environment variable turned the run off. */
  wasSkipped: boolean;

  /** Labels of the commands that failed; the rest still ran. */
  failedLabels: string[];
};

function _findSkipReason(
  environment: Readonly<Record<string, string | undefined>>,
): string | undefined {
  return SKIP_SKILLS_ENV_VAR_NAMES.find((variableName) => {
    const value = environment[variableName];
    return value !== undefined && value !== "" && value !== "false";
  });
}

/** Number of trailing output lines to show for a command that failed. */
const FAILURE_DETAIL_LINE_COUNT = 5;

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
 * Installs or refreshes this project's agent skills.
 *
 * Both managers are driven here: `npx skills` for everything in
 * `skills-lock.json`, and `npx impeccable` for the skill that installs
 * itself. Nothing is symlinked by hand, because each tool owns the layout it
 * expects.
 *
 * @param options.onlyMissing Restore absent skills only.
 * @param options.quiet Postinstall mode: minimal output, never fails.
 * @param options.runner Command runner, overridden in tests.
 * @param options.projectRootPath Directory holding `skills-lock.json`.
 * @param options.environment Environment to read the skip flags from.
 * @returns What ran, what was skipped, and what failed.
 */
export async function updateSkills(
  options: Readonly<UpdateSkillsOptions> = {},
): Promise<UpdateSkillsResult> {
  const {
    onlyMissing = false,
    quiet = false,
    runner = runCommand,
    projectRootPath = process.cwd(),
    environment = process.env,
  } = options;

  const skipReason = quiet ? _findSkipReason(environment) : undefined;
  if (skipReason !== undefined) {
    return { didRun: false, wasSkipped: true, failedLabels: [] };
  }

  const [sourceGroups, installedSkillNames] = await Promise.all([
    readSkillSourceGroups({ projectRootPath }),
    readInstalledSkillNamesFromDisk({ projectRootPath }),
  ]);

  const plan = createSkillsUpdatePlan({
    sourceGroups,
    installedSkillNames,
    isImpeccableInstalled: installedSkillNames.includes(IMPECCABLE_SKILL_NAME),
    onlyMissing,
  });

  if (plan.commands.length === 0) {
    if (!quiet) {
      printSuccess("Every skill in skills-lock.json is already installed.");
    }
    return { didRun: false, wasSkipped: false, failedLabels: [] };
  }

  if (quiet) {
    printInfo(
      `Restoring agent skills (${plan.commands.length} command(s)). ` +
        "Set SKIP_SKILLS_INSTALL=1 to turn this off.",
    );
  } else {
    printHeading(
      onlyMissing ? "Restoring agent skills" : "Updating agent skills",
    );
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
