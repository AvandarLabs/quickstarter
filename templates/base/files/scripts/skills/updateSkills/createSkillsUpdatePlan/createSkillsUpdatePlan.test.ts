import { describe, expect, it } from "vitest";
import { createSkillsUpdatePlan } from "./createSkillsUpdatePlan";
import type { SkillSourceGroup } from "../../skills.types";

const SOURCE_GROUPS: SkillSourceGroup[] = [
  { source: "mantinedev/skills", skillNames: ["mantine-combobox"] },
  {
    source: "obra/superpowers",
    skillNames: ["brainstorming", "writing-plans"],
  },
];

function labelsOf(commands: ReadonlyArray<{ label: string }>): string[] {
  return commands.map((command) => {
    return command.label;
  });
}

describe("createSkillsUpdatePlan", () => {
  it("refreshes every source and impeccable on a full update", () => {
    const plan = createSkillsUpdatePlan({
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: [
        "mantine-combobox",
        "brainstorming",
        "writing-plans",
        "impeccable",
      ],
      isImpeccableInstalled: true,
    });

    expect(labelsOf(plan.commands)).toEqual([
      "skills add mantinedev/skills",
      "skills add obra/superpowers",
      "impeccable update",
    ]);
  });

  it("installs impeccable rather than updating it when it is absent", () => {
    const plan = createSkillsUpdatePlan({
      sourceGroups: [],
      installedSkillNames: [],
      isImpeccableInstalled: false,
    });

    expect(labelsOf(plan.commands)).toEqual(["impeccable install"]);
  });

  it("asks for every locked skill of a source on a full update", () => {
    const plan = createSkillsUpdatePlan({
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: ["brainstorming"],
      isImpeccableInstalled: true,
    });

    const superpowers = plan.commands[1];
    expect(superpowers?.args).toContain("brainstorming");
    expect(superpowers?.args).toContain("writing-plans");
  });

  describe("when only restoring what is missing", () => {
    it("skips sources whose skills are all installed", () => {
      const plan = createSkillsUpdatePlan({
        sourceGroups: SOURCE_GROUPS,
        installedSkillNames: ["mantine-combobox", "brainstorming"],
        isImpeccableInstalled: true,
        onlyMissing: true,
      });

      expect(labelsOf(plan.commands)).toEqual(["skills add obra/superpowers"]);
    });

    it("asks only for the skills that are actually missing", () => {
      const plan = createSkillsUpdatePlan({
        sourceGroups: SOURCE_GROUPS,
        installedSkillNames: ["mantine-combobox", "brainstorming"],
        isImpeccableInstalled: true,
        onlyMissing: true,
      });

      expect(plan.commands[0]?.args).toContain("writing-plans");
      expect(plan.commands[0]?.args).not.toContain("brainstorming");
    });

    it("plans nothing when every skill is already installed", () => {
      const plan = createSkillsUpdatePlan({
        sourceGroups: SOURCE_GROUPS,
        installedSkillNames: [
          "mantine-combobox",
          "brainstorming",
          "writing-plans",
        ],
        isImpeccableInstalled: true,
        onlyMissing: true,
      });

      expect(plan.commands).toEqual([]);
      expect(plan.missingSkillNames).toEqual([]);
    });

    it("still installs impeccable when it is the only thing missing", () => {
      const plan = createSkillsUpdatePlan({
        sourceGroups: SOURCE_GROUPS,
        installedSkillNames: [
          "mantine-combobox",
          "brainstorming",
          "writing-plans",
        ],
        isImpeccableInstalled: false,
        onlyMissing: true,
      });

      expect(labelsOf(plan.commands)).toEqual(["impeccable install"]);
    });
  });

  it("reports every locked skill that is not on disk", () => {
    const plan = createSkillsUpdatePlan({
      sourceGroups: SOURCE_GROUPS,
      installedSkillNames: ["brainstorming"],
      isImpeccableInstalled: false,
    });

    expect(plan.missingSkillNames).toEqual([
      "mantine-combobox",
      "writing-plans",
    ]);
  });
});
