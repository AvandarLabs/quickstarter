import { describe, expect, it } from "vitest";
import { IMPECCABLE_SKILL_NAME } from "../../constants";
import { createSkillsSyncPlan } from "./createSkillsSyncPlan";
import type { SkillSourceGroup } from "../../skills.types";

const SOURCE_GROUPS: SkillSourceGroup[] = [
  { source: "mantinedev/skills", skillNames: ["mantine-combobox"] },
  {
    source: "obra/superpowers",
    skillNames: ["brainstorming", "writing-plans"],
  },
];

const EVERY_SKILL_INSTALLED = [
  "mantine-combobox",
  "brainstorming",
  "writing-plans",
  "impeccable",
];

/**
 * A project the scaffolder gave impeccable to. It is stated rather than taken
 * from the constant so these tests describe one project shape whatever this
 * project's own capabilities selected.
 */
const WITH_IMPECCABLE = [IMPECCABLE_SKILL_NAME];

function labelsOf(commands: ReadonlyArray<{ label: string }>): string[] {
  return commands.map((command) => {
    return command.label;
  });
}

describe("createSkillsSyncPlan", () => {
  it("installs nothing when every locked skill is already on disk", () => {
    const plan = createSkillsSyncPlan({
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: EVERY_SKILL_INSTALLED,
      isImpeccableInstalled: true,
      selfInstallingSkillNames: WITH_IMPECCABLE,
    });

    // This is the `pnpm install` path: an installed skill is left alone, the
    // same way `pnpm install` does not re-resolve an installed package.
    expect(plan.commands).toEqual([]);
    expect(plan.missingSkillNames).toEqual([]);
  });

  it("skips sources whose skills are all installed", () => {
    const plan = createSkillsSyncPlan({
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: ["mantine-combobox", "brainstorming", "impeccable"],
      isImpeccableInstalled: true,
      selfInstallingSkillNames: WITH_IMPECCABLE,
    });

    expect(labelsOf(plan.commands)).toEqual(["skills add obra/superpowers"]);
  });

  it("asks only for the skills that are missing", () => {
    const plan = createSkillsSyncPlan({
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: ["mantine-combobox", "brainstorming", "impeccable"],
      isImpeccableInstalled: true,
      selfInstallingSkillNames: WITH_IMPECCABLE,
    });

    expect(plan.commands[0]?.args).toContain("writing-plans");
    expect(plan.commands[0]?.args).not.toContain("brainstorming");
  });

  it("installs every source and impeccable in an empty project", () => {
    const plan = createSkillsSyncPlan({
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: [],
      isImpeccableInstalled: false,
      selfInstallingSkillNames: WITH_IMPECCABLE,
    });

    expect(labelsOf(plan.commands)).toEqual([
      "skills add mantinedev/skills",
      "skills add obra/superpowers",
      "impeccable install",
    ]);
  });

  it("installs impeccable when it is the only thing missing", () => {
    const plan = createSkillsSyncPlan({
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: EVERY_SKILL_INSTALLED,
      isImpeccableInstalled: false,
      selfInstallingSkillNames: WITH_IMPECCABLE,
    });

    expect(labelsOf(plan.commands)).toEqual(["impeccable install"]);
  });

  it("leaves an installed impeccable alone", () => {
    // Refreshing it is the update script's job, never an install's.
    const plan = createSkillsSyncPlan({
      sourceGroups: [],
      installedSkillNames: ["impeccable"],
      isImpeccableInstalled: true,
      selfInstallingSkillNames: WITH_IMPECCABLE,
    });

    expect(plan.commands).toEqual([]);
  });

  it("plans no impeccable command for a project that does not have it", () => {
    // The scaffolder writes the self-installing list from its capability
    // manifest, so a project that selected no such skill must never be handed
    // an impeccable install it did not ask for.
    const plan = createSkillsSyncPlan({
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: [],
      isImpeccableInstalled: false,
      selfInstallingSkillNames: [],
    });

    expect(labelsOf(plan.commands)).toEqual([
      "skills add mantinedev/skills",
      "skills add obra/superpowers",
    ]);
  });

  it("reports every locked skill that is not on disk", () => {
    const plan = createSkillsSyncPlan({
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: ["brainstorming"],
      isImpeccableInstalled: false,
      selfInstallingSkillNames: WITH_IMPECCABLE,
    });

    expect(plan.missingSkillNames).toEqual([
      "mantine-combobox",
      "writing-plans",
    ]);
  });
});
