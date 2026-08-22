import {
  IMPECCABLE_SKILL_NAME,
  IMPECCABLE_SKILL_SOURCE,
} from "../../constants";
import type {
  InstalledSkill,
  SkillListingRow,
  SkillSourceGroup,
} from "../../skills.types";

type CreateSkillListingRowsOptions = {
  /** What the managers report as present on disk. */
  installedSkills: readonly InstalledSkill[];

  /** What `skills-lock.json` says this project should have. */
  sourceGroups: readonly SkillSourceGroup[];
};

function _createRowFromInstalledSkill(
  skill: Readonly<InstalledSkill>,
): SkillListingRow {
  return {
    name: skill.name,
    manager: skill.manager,
    source: skill.source,
    agents: skill.agents,
    isInstalled: true,
  };
}

/**
 * Builds the rows for skills the lock asks for that are not on disk. This is
 * the state a freshly cloned project is in before `pnpm install` runs.
 */
function _createMissingLockedRows(
  options: Readonly<CreateSkillListingRowsOptions>,
): SkillListingRow[] {
  const installedNames = new Set(
    options.installedSkills.map((skill) => {
      return skill.name;
    }),
  );

  return options.sourceGroups.flatMap((group) => {
    return group.skillNames
      .filter((skillName) => {
        return !installedNames.has(skillName);
      })
      .map((skillName) => {
        return {
          name: skillName,
          manager: "skills" as const,
          source: group.source,
          agents: [],
          isInstalled: false,
        };
      });
  });
}

/**
 * Builds the impeccable row when it is absent. Impeccable is never in the
 * lock, so its absence has to be inferred rather than read.
 */
function _createMissingImpeccableRows(
  installedSkills: readonly InstalledSkill[],
): SkillListingRow[] {
  const isInstalled = installedSkills.some((skill) => {
    return skill.name === IMPECCABLE_SKILL_NAME;
  });
  if (isInstalled) {
    return [];
  }
  return [
    {
      name: IMPECCABLE_SKILL_NAME,
      manager: "impeccable",
      source: IMPECCABLE_SKILL_SOURCE,
      agents: [],
      isInstalled: false,
    },
  ];
}

/**
 * Merges what is installed with what is locked into one listing.
 *
 * Neither manager alone can answer "what should this project have?": `npx
 * skills` only knows what is on disk and impeccable only knows about itself,
 * so the lock supplies the expected set and the difference becomes the
 * missing rows.
 *
 * @param options.installedSkills Skills the managers report on disk.
 * @param options.sourceGroups Locked skills grouped by source repository.
 * @returns One row per skill, ordered by name.
 */
export function createSkillListingRows(
  options: Readonly<CreateSkillListingRowsOptions>,
): SkillListingRow[] {
  return [
    ...options.installedSkills.map(_createRowFromInstalledSkill),
    ..._createMissingLockedRows(options),
    ..._createMissingImpeccableRows(options.installedSkills),
  ].sort((left, right) => {
    return left.name.localeCompare(right.name);
  });
}
