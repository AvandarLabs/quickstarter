import { describe, expect, it } from "vitest";
import { createSkillListingRows } from "./createSkillListingRows";
import type { InstalledSkill, SkillSourceGroup } from "../../skills.types";

const SOURCE_GROUPS: SkillSourceGroup[] = [
  {
    source: "obra/superpowers",
    skillNames: ["brainstorming", "writing-plans"],
  },
];

const INSTALLED_BRAINSTORMING: InstalledSkill = {
  name: "brainstorming",
  manager: "skills",
  source: "obra/superpowers",
  agents: ["Claude Code", "Codex"],
  path: "/app/.agents/skills/brainstorming",
};

const INSTALLED_IMPECCABLE: InstalledSkill = {
  name: "impeccable",
  manager: "impeccable",
  source: "pbakaus/impeccable",
  agents: ["Claude Code"],
  path: "/app/.agents/skills/impeccable",
};

function namesOf(rows: ReadonlyArray<{ name: string }>): string[] {
  return rows.map((row) => {
    return row.name;
  });
}

describe("createSkillListingRows", () => {
  it("reports an installed skill with its manager and agents", () => {
    const rows = createSkillListingRows({
      installedSkills: [INSTALLED_BRAINSTORMING, INSTALLED_IMPECCABLE],
      sourceGroups: SOURCE_GROUPS,
    });

    expect(rows[0]).toEqual({
      name: "brainstorming",
      manager: "skills",
      source: "obra/superpowers",
      agents: ["Claude Code", "Codex"],
      isInstalled: true,
    });
  });

  it("reports a locked skill that is not on disk as missing", () => {
    const rows = createSkillListingRows({
      installedSkills: [INSTALLED_BRAINSTORMING, INSTALLED_IMPECCABLE],
      sourceGroups: SOURCE_GROUPS,
    });

    expect(rows).toContainEqual({
      name: "writing-plans",
      manager: "skills",
      source: "obra/superpowers",
      agents: [],
      isInstalled: false,
    });
  });

  it("reports impeccable as missing when it is not installed", () => {
    const rows = createSkillListingRows({
      installedSkills: [INSTALLED_BRAINSTORMING],
      sourceGroups: SOURCE_GROUPS,
    });

    expect(rows).toContainEqual({
      name: "impeccable",
      manager: "impeccable",
      source: "pbakaus/impeccable",
      agents: [],
      isInstalled: false,
    });
  });

  it("lists impeccable once when it is installed", () => {
    const rows = createSkillListingRows({
      installedSkills: [INSTALLED_IMPECCABLE],
      sourceGroups: [],
    });

    expect(namesOf(rows)).toEqual(["impeccable"]);
    expect(rows[0]?.isInstalled).toBe(true);
  });

  it("keeps a skill that is installed but no longer locked", () => {
    const rows = createSkillListingRows({
      installedSkills: [INSTALLED_BRAINSTORMING, INSTALLED_IMPECCABLE],
      sourceGroups: [],
    });

    expect(namesOf(rows)).toEqual(["brainstorming", "impeccable"]);
  });

  it("orders rows by name", () => {
    const rows = createSkillListingRows({
      installedSkills: [INSTALLED_IMPECCABLE],
      sourceGroups: [
        { source: "mantinedev/skills", skillNames: ["mantine-combobox"] },
        { source: "obra/superpowers", skillNames: ["brainstorming"] },
      ],
    });

    expect(namesOf(rows)).toEqual([
      "brainstorming",
      "impeccable",
      "mantine-combobox",
    ]);
  });
});
