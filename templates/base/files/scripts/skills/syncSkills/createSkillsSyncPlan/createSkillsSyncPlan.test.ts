import { describe, expect, it } from "vitest";
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

function labelsOf(commands: ReadonlyArray<{ label: string }>): string[] {
  return commands.map((command) => {
    return command.label;
  });
}

describe("createSkillsSyncPlan in install mode", () => {
  it("installs nothing when every locked skill is already on disk", () => {
    const plan = createSkillsSyncPlan({
      mode: "install",
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: EVERY_SKILL_INSTALLED,
      isImpeccableInstalled: true,
    });

    // This is the `pnpm install` path: an installed skill is left alone, the
    // same way `pnpm install` does not re-resolve an installed package.
    expect(plan.commands).toEqual([]);
    expect(plan.missingSkillNames).toEqual([]);
  });

  it("skips sources whose skills are all installed", () => {
    const plan = createSkillsSyncPlan({
      mode: "install",
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: ["mantine-combobox", "brainstorming", "impeccable"],
      isImpeccableInstalled: true,
    });

    expect(labelsOf(plan.commands)).toEqual(["skills add obra/superpowers"]);
  });

  it("asks only for the skills that are missing", () => {
    const plan = createSkillsSyncPlan({
      mode: "install",
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: ["mantine-combobox", "brainstorming", "impeccable"],
      isImpeccableInstalled: true,
    });

    expect(plan.commands[0]?.args).toContain("writing-plans");
    expect(plan.commands[0]?.args).not.toContain("brainstorming");
  });

  it("installs impeccable when it is the only thing missing", () => {
    const plan = createSkillsSyncPlan({
      mode: "install",
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: EVERY_SKILL_INSTALLED,
      isImpeccableInstalled: false,
    });

    expect(labelsOf(plan.commands)).toEqual(["impeccable install"]);
  });
});

describe("createSkillsSyncPlan in update mode", () => {
  it("refreshes every source and impeccable even when nothing is missing", () => {
    const plan = createSkillsSyncPlan({
      mode: "update",
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: EVERY_SKILL_INSTALLED,
      isImpeccableInstalled: true,
    });

    expect(labelsOf(plan.commands)).toEqual([
      "skills add mantinedev/skills",
      "skills add obra/superpowers",
      "impeccable update",
    ]);
  });

  it("asks for every locked skill of a source, not just the missing ones", () => {
    const plan = createSkillsSyncPlan({
      mode: "update",
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: ["brainstorming"],
      isImpeccableInstalled: true,
    });

    const superpowers = plan.commands[1];
    expect(superpowers?.args).toContain("brainstorming");
    expect(superpowers?.args).toContain("writing-plans");
  });

  it("installs impeccable rather than updating it when it is absent", () => {
    const plan = createSkillsSyncPlan({
      mode: "update",
      sourceGroups: [],
      installedSkillNames: [],
      isImpeccableInstalled: false,
    });

    expect(labelsOf(plan.commands)).toEqual(["impeccable install"]);
  });
});

describe("createSkillsSyncPlan", () => {
  it("reports every locked skill that is not on disk, in either mode", () => {
    const options = {
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: ["brainstorming"],
      isImpeccableInstalled: false,
    };

    expect(
      createSkillsSyncPlan({ ...options, mode: "install" }).missingSkillNames,
    ).toEqual(["mantine-combobox", "writing-plans"]);
    expect(
      createSkillsSyncPlan({ ...options, mode: "update" }).missingSkillNames,
    ).toEqual(["mantine-combobox", "writing-plans"]);
  });
});
