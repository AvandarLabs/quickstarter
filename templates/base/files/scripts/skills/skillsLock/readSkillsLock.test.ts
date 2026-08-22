import { describe, expect, it } from "vitest";
import { createSkillSourceGroupsFromLock } from "./readSkillsLock";

const LOCK_JSON = `{
  "version": 1,
  "skills": {
    "writing-plans": {
      "source": "obra/superpowers",
      "sourceType": "github",
      "skillPath": "skills/writing-plans/SKILL.md"
    },
    "mantine-combobox": {
      "source": "mantinedev/skills",
      "sourceType": "github",
      "skillPath": "skills/mantine-combobox/SKILL.md"
    },
    "brainstorming": {
      "source": "obra/superpowers",
      "sourceType": "github",
      "skillPath": "skills/brainstorming/SKILL.md"
    }
  }
}`;

describe("createSkillSourceGroupsFromLock", () => {
  it("groups skills by the repository they come from", () => {
    const groups = createSkillSourceGroupsFromLock(LOCK_JSON);

    expect(groups).toEqual([
      {
        source: "mantinedev/skills",
        skillNames: ["mantine-combobox"],
      },
      {
        source: "obra/superpowers",
        skillNames: ["brainstorming", "writing-plans"],
      },
    ]);
  });

  it("returns no groups for a lock with no skills", () => {
    expect(
      createSkillSourceGroupsFromLock(`{ "version": 1, "skills": {} }`),
    ).toEqual([]);
  });

  it("skips entries that have no source to install from", () => {
    const lockJson = `{
      "version": 1,
      "skills": {
        "hand-written": { "sourceType": "local" },
        "brainstorming": { "source": "obra/superpowers" }
      }
    }`;

    expect(createSkillSourceGroupsFromLock(lockJson)).toEqual([
      { source: "obra/superpowers", skillNames: ["brainstorming"] },
    ]);
  });

  it("throws a readable error when the lock is not valid JSON", () => {
    expect(() => {
      return createSkillSourceGroupsFromLock("{ not json");
    }).toThrow(/skills-lock\.json/);
  });
});
