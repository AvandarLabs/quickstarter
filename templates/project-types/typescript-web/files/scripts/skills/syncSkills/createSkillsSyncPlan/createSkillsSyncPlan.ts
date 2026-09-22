import {
  IMPECCABLE_SKILL_NAME,
  SELF_INSTALLING_SKILL_NAMES,
} from "../../constants";
import {
  createImpeccableInstallCommand,
  createSkillsAddCommand,
} from "../../skillCommands/skillCommands";
import type { CommandSpec, SkillSourceGroup } from "../../skills.types";

/** What an install will do, worked out before anything is executed. */
export type SkillsSyncPlan = {
  commands: CommandSpec[];

  /** Locked skills that are not on disk. */
  missingSkillNames: string[];
};

type CreateSkillsSyncPlanOptions = {
  /** The locked skills, grouped by the repository they come from. */
  sourceGroups: readonly SkillSourceGroup[];

  /** Names of the skills currently present under `.agents/skills`. */
  installedSkillNames: readonly string[];

  isImpeccableInstalled: boolean;

  /**
   * The skills that install themselves through their own CLI. It defaults to
   * what the scaffolder wrote for this project so no caller can drift from it;
   * a test passes it to describe a project of a different shape.
   */
  selfInstallingSkillNames?: readonly string[];
};

/**
 * One `skills add` per source that is still owed something. Only the missing
 * skills are asked for: this mirrors `pnpm install`, which does not re-resolve
 * a package that is already there.
 */
function _createSkillsAddCommands(
  options: Readonly<CreateSkillsSyncPlanOptions>,
): CommandSpec[] {
  const installedNames = new Set(options.installedSkillNames);

  return options.sourceGroups.flatMap((group) => {
    const missingNames = group.skillNames.filter((skillName) => {
      return !installedNames.has(skillName);
    });

    if (missingNames.length === 0) {
      return [];
    }
    return [createSkillsAddCommand({ ...group, skillNames: missingNames })];
  });
}

/**
 * The impeccable install, when this project has impeccable at all and it is
 * not on disk yet. Impeccable is never in the lock, so whether it is wanted
 * has to be read from the self-installing list rather than inferred.
 */
function _createImpeccableCommands(
  options: Readonly<CreateSkillsSyncPlanOptions>,
): CommandSpec[] {
  const {
    isImpeccableInstalled,
    selfInstallingSkillNames = SELF_INSTALLING_SKILL_NAMES,
  } = options;

  const usesImpeccable = selfInstallingSkillNames.includes(
    IMPECCABLE_SKILL_NAME,
  );
  if (!usesImpeccable || isImpeccableInstalled) {
    return [];
  }
  return [createImpeccableInstallCommand()];
}

function _findMissingSkillNames(
  options: Readonly<CreateSkillsSyncPlanOptions>,
): string[] {
  const installedNames = new Set(options.installedSkillNames);
  return options.sourceGroups
    .flatMap((group) => {
      return group.skillNames;
    })
    .filter((skillName) => {
      return !installedNames.has(skillName);
    })
    .sort((left, right) => {
      return left.localeCompare(right);
    });
}

/**
 * Works out which commands an install needs to run.
 *
 * Installing is the only thing planned here: updating is the job of
 * `scripts/skills/update-skills.sh`, which drives both managers itself so
 * every generated project updates the same way. Keeping this decision separate
 * from running it is what makes it testable without touching the network.
 *
 * @param options.sourceGroups Locked skills grouped by source repository.
 * @param options.installedSkillNames Skills currently on disk.
 * @param options.isImpeccableInstalled Whether impeccable is on disk.
 * @param options.selfInstallingSkillNames Skills that install themselves.
 * @returns The commands to run and the locked skills that are missing.
 */
export function createSkillsSyncPlan(
  options: Readonly<CreateSkillsSyncPlanOptions>,
): SkillsSyncPlan {
  return {
    commands: [
      ..._createSkillsAddCommands(options),
      ..._createImpeccableCommands(options),
    ],
    missingSkillNames: _findMissingSkillNames(options),
  };
}
