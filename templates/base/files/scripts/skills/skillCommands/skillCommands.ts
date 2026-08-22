import {
  IMPECCABLE_PROVIDER_NAMES,
  IMPECCABLE_SKILL_NAME,
  SKILL_AGENT_NAMES,
} from "../constants";
import type { CommandSpec, SkillSourceGroup } from "../skills.types";

/**
 * The exact commands this project runs against the two skill managers.
 *
 * Both tools are invoked through `npx` rather than installed as dependencies:
 * they manage agent tooling, not application code, and neither belongs in the
 * dependency graph of the app being built.
 */

const NPX_COMMAND = "npx";

/** `-y` stops npx from prompting before it fetches a missing package. */
const NPX_FLAGS = ["-y"] as const;

const SKILLS_PACKAGE_NAME = "skills";

/** Builds the command that lists the skills installed in this project. */
export function createSkillsListCommand(): CommandSpec {
  return {
    label: "skills list",
    command: NPX_COMMAND,
    args: [...NPX_FLAGS, SKILLS_PACKAGE_NAME, "list", "--json"],
  };
}

/**
 * Builds the command that installs (or refreshes) every skill taken from one
 * repository.
 *
 * `--skill` and `--agent` are variadic, so the names are passed as separate
 * arguments: a comma-joined list is read as one unknown skill name. `-y`
 * accepts the security prompt, which is what makes this usable from
 * `postinstall`. `--full-depth` is what finds skills nested inside a
 * monorepo: without it a shallow `SKILL.md` ends the search and the deeper
 * skills are reported as not found.
 *
 * @param group The source repository and the skills wanted from it.
 * @returns The `skills add` command for that repository.
 */
export function createSkillsAddCommand(
  group: Readonly<SkillSourceGroup>,
): CommandSpec {
  return {
    label: `skills add ${group.source}`,
    command: NPX_COMMAND,
    args: [
      ...NPX_FLAGS,
      SKILLS_PACKAGE_NAME,
      "add",
      group.source,
      "--skill",
      ...group.skillNames,
      "--agent",
      ...SKILL_AGENT_NAMES,
      "--full-depth",
      "-y",
    ],
  };
}

/**
 * Builds the command that installs impeccable into this project.
 *
 * Passing both `--providers` and `--scope` is what keeps the installer
 * non-interactive.
 */
export function createImpeccableInstallCommand(): CommandSpec {
  return {
    label: `${IMPECCABLE_SKILL_NAME} install`,
    command: NPX_COMMAND,
    args: [
      ...NPX_FLAGS,
      IMPECCABLE_SKILL_NAME,
      "install",
      `--providers=${IMPECCABLE_PROVIDER_NAMES.join(",")}`,
      "--scope=project",
    ],
  };
}

/** Builds the command that refreshes an existing impeccable install. */
export function createImpeccableUpdateCommand(): CommandSpec {
  return {
    label: `${IMPECCABLE_SKILL_NAME} update`,
    command: NPX_COMMAND,
    args: [...NPX_FLAGS, IMPECCABLE_SKILL_NAME, "update"],
  };
}
