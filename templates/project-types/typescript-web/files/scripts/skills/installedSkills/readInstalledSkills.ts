import { readdir } from "node:fs/promises";
import path from "node:path";
import {
  AGENT_SKILLS_DIR_PATH,
  IMPECCABLE_SKILL_NAME,
  IMPECCABLE_SKILL_SOURCE,
} from "../constants";
import { runCommand } from "../runCommand/runCommand";
import { createSkillsListCommand } from "../skillCommands/skillCommands";
import type { CommandRunner, InstalledSkill } from "../skills.types";

/** What the `skills` CLI reported, or why it could not be asked. */
export type InstalledSkillsResult = {
  skills: InstalledSkill[];

  /** Set when the CLI could not be run, so "no skills" is not mistaken for
   * "nothing installed". */
  errorMessage: string | undefined;
};

type SkillsListRow = {
  name: string;
  source?: unknown;
  agents?: unknown;
  path?: unknown;
};

function _isSkillsListRow(row: unknown): row is SkillsListRow {
  return (
    typeof row === "object" &&
    row !== null &&
    typeof (row as { name?: unknown }).name === "string"
  );
}

/**
 * Pulls the JSON array out of a command's output. `npx` prints notices of its
 * own around the payload, so the array is located rather than assumed to be
 * the whole of stdout.
 */
function _extractJsonRows(commandOutput: string): unknown[] {
  const arrayStart = commandOutput.indexOf("[");
  const arrayEnd = commandOutput.lastIndexOf("]");
  if (arrayStart === -1 || arrayEnd < arrayStart) {
    return [];
  }
  const parsed = JSON.parse(
    commandOutput.slice(arrayStart, arrayEnd + 1),
  ) as unknown;
  return Array.isArray(parsed) ? parsed : [];
}

function _createInstalledSkillFromRow(
  row: Readonly<SkillsListRow>,
): InstalledSkill {
  const isImpeccable = row.name === IMPECCABLE_SKILL_NAME;
  const reportedSource =
    typeof row.source === "string" ? row.source : undefined;
  return {
    name: row.name,
    manager: isImpeccable ? "impeccable" : "skills",
    // `npx skills` knows nothing about where impeccable came from, because
    // the impeccable CLI installed it. Fill that gap in for the reader.
    source: isImpeccable ? IMPECCABLE_SKILL_SOURCE : reportedSource,
    agents: Array.isArray(row.agents)
      ? row.agents.filter((agent): agent is string => {
          return typeof agent === "string";
        })
      : [],
    path: typeof row.path === "string" ? row.path : undefined,
  };
}

/**
 * Parses the output of `skills list --json` into installed skills.
 *
 * Output with no JSON array in it means no skills are installed, which is
 * also what an unconfigured project looks like.
 *
 * @param commandOutput Raw stdout from the `skills` CLI.
 * @returns The installed skills, ordered by name.
 */
export function parseInstalledSkills(commandOutput: string): InstalledSkill[] {
  return _extractJsonRows(commandOutput)
    .filter(_isSkillsListRow)
    .map((row) => {
      return _createInstalledSkillFromRow(row);
    })
    .sort((left, right) => {
      return left.name.localeCompare(right.name);
    });
}

/**
 * Asks `npx skills` what is installed in this project.
 *
 * The `skills` CLI is the authority here because it is the only thing that
 * knows which agent frontends each skill reached.
 *
 * @param options.runner Command runner, overridden in tests.
 * @returns The installed skills, or the reason they could not be read.
 */
export async function readInstalledSkills(
  options: { runner?: CommandRunner } = {},
): Promise<InstalledSkillsResult> {
  const { runner = runCommand } = options;
  const result = await runner(createSkillsListCommand());
  if (result.exitCode !== 0) {
    return {
      skills: [],
      errorMessage: (result.stderr || result.stdout).trim(),
    };
  }
  return {
    skills: parseInstalledSkills(result.stdout),
    errorMessage: undefined,
  };
}

/**
 * Reads the skill names present under `.agents/skills`.
 *
 * This is a plain directory read rather than a call to either manager: it has
 * to work offline and cost nothing, because it is what decides whether the
 * `postinstall` restore needs to do any work at all.
 *
 * @param options.projectRootPath Directory holding `.agents/skills`.
 * @returns The installed skill names, or none when nothing is installed yet.
 */
export async function readInstalledSkillNamesFromDisk(
  options: { projectRootPath?: string } = {},
): Promise<string[]> {
  const { projectRootPath = process.cwd() } = options;
  const skillsDirPath = path.join(projectRootPath, AGENT_SKILLS_DIR_PATH);
  try {
    const entries = await readdir(skillsDirPath, { withFileTypes: true });
    return entries
      .filter((entry) => {
        return entry.isDirectory() || entry.isSymbolicLink();
      })
      .map((entry) => {
        return entry.name;
      });
  } catch {
    return [];
  }
}
