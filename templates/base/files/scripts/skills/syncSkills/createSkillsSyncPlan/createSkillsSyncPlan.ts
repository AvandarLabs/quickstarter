import {
  createImpeccableInstallCommand,
  createImpeccableUpdateCommand,
  createSkillsAddCommand,
} from "../../skillCommands/skillCommands";
import type { CommandSpec, SkillSourceGroup } from "../../skills.types";

/**
 * Whether to bring the project up to the lock, or up to the latest.
 *
 * - `install` touches only what is absent, mirroring `pnpm install`: an
 *   already-installed skill is left exactly as it is.
 * - `update` refreshes everything, mirroring an explicit dependency update.
 */
export type SkillsSyncMode = "install" | "update";

/** What a sync will do, worked out before anything is executed. */
export type SkillsSyncPlan = {
  commands: CommandSpec[];

  /** Locked skills that are not on disk, whatever the mode. */
  missingSkillNames: string[];
};

type CreateSkillsSyncPlanOptions = {
  mode: SkillsSyncMode;

  /** The locked skills, grouped by the repository they come from. */
  sourceGroups: readonly SkillSourceGroup[];

  /** Names of the skills currently present under `.agents/skills`. */
  installedSkillNames: readonly string[];

  isImpeccableInstalled: boolean;
};

function _createSkillsCommands(
  options: Readonly<CreateSkillsSyncPlanOptions>,
): CommandSpec[] {
  const { mode, sourceGroups, installedSkillNames } = options;
  const installedNames = new Set(installedSkillNames);

  return sourceGroups.flatMap((group) => {
    const wantedNames =
      mode === "update"
        ? group.skillNames
        : group.skillNames.filter((skillName) => {
            return !installedNames.has(skillName);
          });

    if (wantedNames.length === 0) {
      return [];
    }
    return [createSkillsAddCommand({ ...group, skillNames: wantedNames })];
  });
}

function _createImpeccableCommands(
  options: Readonly<CreateSkillsSyncPlanOptions>,
): CommandSpec[] {
  const { mode, isImpeccableInstalled } = options;
  if (!isImpeccableInstalled) {
    return [createImpeccableInstallCommand()];
  }
  return mode === "update" ? [createImpeccableUpdateCommand()] : [];
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
 * Works out which commands a sync needs to run.
 *
 * Keeping the decision separate from running it is what makes both modes
 * testable without touching the network.
 *
 * @param options.mode Install what is missing, or update everything.
 * @param options.sourceGroups Locked skills grouped by source repository.
 * @param options.installedSkillNames Skills currently on disk.
 * @param options.isImpeccableInstalled Whether impeccable is on disk.
 * @returns The commands to run and the locked skills that are missing.
 */
export function createSkillsSyncPlan(
  options: Readonly<CreateSkillsSyncPlanOptions>,
): SkillsSyncPlan {
  return {
    commands: [
      ..._createSkillsCommands(options),
      ..._createImpeccableCommands(options),
    ],
    missingSkillNames: _findMissingSkillNames(options),
  };
}
