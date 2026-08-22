import {
  createImpeccableInstallCommand,
  createImpeccableUpdateCommand,
  createSkillsAddCommand,
} from "../../skillCommands/skillCommands";
import type { CommandSpec, SkillSourceGroup } from "../../skills.types";

/** What an update run will do, worked out before anything is executed. */
export type SkillsUpdatePlan = {
  commands: CommandSpec[];

  /** Locked skills that are not on disk, whatever the mode. */
  missingSkillNames: string[];
};

type CreateSkillsUpdatePlanOptions = {
  /** The locked skills, grouped by the repository they come from. */
  sourceGroups: readonly SkillSourceGroup[];

  /** Names of the skills currently present under `.agents/skills`. */
  installedSkillNames: readonly string[];

  isImpeccableInstalled: boolean;

  /**
   * Restore what is absent and nothing more. This is the `postinstall` mode:
   * it costs no network at all once a project is complete, so adding a
   * dependency does not re-fetch every skill repository.
   */
  onlyMissing?: boolean;
};

function _createSkillsCommands(
  options: Readonly<CreateSkillsUpdatePlanOptions>,
): CommandSpec[] {
  const { sourceGroups, installedSkillNames, onlyMissing = false } = options;
  const installedNames = new Set(installedSkillNames);

  return sourceGroups.flatMap((group) => {
    const wantedNames = onlyMissing
      ? group.skillNames.filter((skillName) => {
          return !installedNames.has(skillName);
        })
      : group.skillNames;

    if (wantedNames.length === 0) {
      return [];
    }
    return [createSkillsAddCommand({ ...group, skillNames: wantedNames })];
  });
}

function _createImpeccableCommands(
  options: Readonly<CreateSkillsUpdatePlanOptions>,
): CommandSpec[] {
  const { isImpeccableInstalled, onlyMissing = false } = options;
  if (!isImpeccableInstalled) {
    return [createImpeccableInstallCommand()];
  }
  return onlyMissing ? [] : [createImpeccableUpdateCommand()];
}

function _findMissingSkillNames(
  options: Readonly<CreateSkillsUpdatePlanOptions>,
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
 * Works out which commands an update needs to run.
 *
 * Keeping this decision separate from running it is what makes the two modes
 * testable without touching the network: a full update refreshes every locked
 * source and impeccable, while `onlyMissing` restores just what is absent.
 *
 * @param options.sourceGroups Locked skills grouped by source repository.
 * @param options.installedSkillNames Skills currently on disk.
 * @param options.isImpeccableInstalled Whether impeccable is on disk.
 * @param options.onlyMissing Restore absent skills only.
 * @returns The commands to run and the locked skills that are missing.
 */
export function createSkillsUpdatePlan(
  options: Readonly<CreateSkillsUpdatePlanOptions>,
): SkillsUpdatePlan {
  return {
    commands: [
      ..._createSkillsCommands(options),
      ..._createImpeccableCommands(options),
    ],
    missingSkillNames: _findMissingSkillNames(options),
  };
}
