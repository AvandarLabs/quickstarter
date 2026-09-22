import {
  printBlock,
  printHeading,
  printInfo,
  printWarning,
} from "../cliOutput/cliOutput";
import { readInstalledSkills } from "../installedSkills/readInstalledSkills";
import { runCommand } from "../runCommand/runCommand";
import { readSkillSourceGroups } from "../skillsLock/readSkillsLock";
import { buildSkillTable } from "./buildSkillTable/buildSkillTable";
import { createSkillListingRows } from "./createSkillListingRows/createSkillListingRows";
import type { CommandRunner, SkillListingRow } from "../skills.types";

/** Collects the agent frontends every installed skill was linked into. */
function _findAgentNames(rows: readonly SkillListingRow[]): string[] {
  return Array.from(
    new Set(
      rows.flatMap((row) => {
        return row.agents;
      }),
    ),
  ).sort((left, right) => {
    return left.localeCompare(right);
  });
}

function _printSummary(rows: readonly SkillListingRow[]): void {
  const installedRows = rows.filter((row) => {
    return row.isInstalled;
  });
  const agentNames = _findAgentNames(installedRows);
  const agentSummary =
    agentNames.length > 0 ? ` for ${agentNames.join(", ")}` : "";

  printInfo(
    `\n${installedRows.length} of ${rows.length} installed${agentSummary}`,
  );

  const missingCount = rows.length - installedRows.length;
  if (missingCount > 0) {
    printWarning(
      `${missingCount} skill(s) are not installed. ` +
        "Run `pnpm install` to restore them.",
    );
  }
}

/**
 * Prints every agent skill this project has, from both managers.
 *
 * The listing merges three sources: what `npx skills` reports on disk, what
 * `skills-lock.json` says the project should have, and impeccable, which
 * neither of the other two fully describes.
 *
 * @param options.runner Command runner, overridden in tests.
 * @param options.projectRootPath Directory holding `skills-lock.json`.
 * @returns Nothing; the listing is printed.
 */
export async function listSkills(
  options: Readonly<{
    runner?: CommandRunner;
    projectRootPath?: string;
  }> = {},
): Promise<void> {
  const { runner = runCommand, projectRootPath = process.cwd() } = options;

  const [installedResult, sourceGroups] = await Promise.all([
    readInstalledSkills({ runner }),
    readSkillSourceGroups({ projectRootPath }),
  ]);

  printHeading("Agent skills");

  if (installedResult.errorMessage !== undefined) {
    printWarning("Could not read the installed skills from `npx skills`.");
    printInfo(installedResult.errorMessage);
  }

  const rows = createSkillListingRows({
    installedSkills: installedResult.skills,
    sourceGroups,
  });

  printBlock(buildSkillTable(rows));
  _printSummary(rows);
}
