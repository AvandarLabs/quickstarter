import { readFile } from "node:fs/promises";
import path from "node:path";
import { SKILLS_LOCK_FILE_NAME } from "../constants";
import type { SkillSourceGroup } from "../skills.types";

type SkillsLockEntry = {
  source?: unknown;
};

type SkillsLockDocument = {
  version?: unknown;
  skills?: Record<string, SkillsLockEntry>;
};

function _parseLockDocument(lockJson: string): SkillsLockDocument {
  try {
    return JSON.parse(lockJson) as SkillsLockDocument;
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    throw new Error(`Could not parse ${SKILLS_LOCK_FILE_NAME}: ${reason}`);
  }
}

function _isFileNotFoundError(error: unknown): boolean {
  return (
    error instanceof Error && (error as NodeJS.ErrnoException).code === "ENOENT"
  );
}

/**
 * Groups the locked skills by the repository that provides them.
 *
 * `npx skills add` takes one repository plus the skills to take from it, so
 * grouping is what turns a lock file into the smallest set of install
 * commands. Entries with no `source` are skipped: nothing can be fetched for
 * them, which is the case for a skill authored inside this project.
 *
 * @param lockJson The raw contents of `skills-lock.json`.
 * @returns One group per source repository, ordered by source then name.
 */
export function createSkillSourceGroupsFromLock(
  lockJson: string,
): SkillSourceGroup[] {
  const document = _parseLockDocument(lockJson);
  const skillNamesBySource = Object.entries(document.skills ?? {}).reduce(
    (groupedNames, [skillName, entry]) => {
      const source = entry?.source;
      if (typeof source !== "string" || source === "") {
        return groupedNames;
      }
      const namesFromSource = groupedNames.get(source) ?? [];
      namesFromSource.push(skillName);
      groupedNames.set(source, namesFromSource);
      return groupedNames;
    },
    new Map<string, string[]>(),
  );

  return Array.from(skillNamesBySource.entries())
    .map(([source, skillNames]) => {
      return {
        source,
        skillNames: skillNames.sort((left, right) => {
          return left.localeCompare(right);
        }),
      };
    })
    .sort((left, right) => {
      return left.source.localeCompare(right.source);
    });
}

/**
 * Reads this project's `skills-lock.json` and groups it by source.
 *
 * A missing lock file yields no groups: a project that tracks no skills is a
 * valid, if unusual, state and must not break `pnpm install`.
 *
 * @param options.projectRootPath Directory holding the lock file.
 * @returns One group per source repository.
 */
export async function readSkillSourceGroups(
  options: { projectRootPath?: string } = {},
): Promise<SkillSourceGroup[]> {
  const { projectRootPath = process.cwd() } = options;
  const lockFilePath = path.join(projectRootPath, SKILLS_LOCK_FILE_NAME);
  try {
    return createSkillSourceGroupsFromLock(
      await readFile(lockFilePath, "utf8"),
    );
  } catch (error) {
    if (_isFileNotFoundError(error)) {
      return [];
    }
    throw error;
  }
}
