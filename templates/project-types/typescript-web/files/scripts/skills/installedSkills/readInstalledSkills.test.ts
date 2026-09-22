import { describe, expect, it } from "vitest";
import { parseInstalledSkills } from "./readInstalledSkills";

const LIST_JSON = `[
  {
    "name": "brainstorming",
    "path": "/app/.agents/skills/brainstorming",
    "scope": "project",
    "agents": ["Claude Code", "Codex", "Cursor", "OpenCode"],
    "source": "obra/superpowers",
    "sourceUrl": null,
    "sourceType": "github"
  },
  {
    "name": "impeccable",
    "path": "/app/.agents/skills/impeccable",
    "scope": "project",
    "agents": ["Claude Code"],
    "source": null,
    "sourceUrl": null,
    "sourceType": null
  }
]`;

describe("parseInstalledSkills", () => {
  it("reads the name, source, and agents of each installed skill", () => {
    const skills = parseInstalledSkills(LIST_JSON);

    expect(skills[0]).toEqual({
      name: "brainstorming",
      manager: "skills",
      source: "obra/superpowers",
      agents: ["Claude Code", "Codex", "Cursor", "OpenCode"],
      path: "/app/.agents/skills/brainstorming",
    });
  });

  it("attributes impeccable to its own manager", () => {
    const skills = parseInstalledSkills(LIST_JSON);
    const impeccable = skills.find((skill) => {
      return skill.name === "impeccable";
    });

    expect(impeccable?.manager).toBe("impeccable");
    expect(impeccable?.source).toBe("pbakaus/impeccable");
  });

  it("ignores anything printed before the JSON payload", () => {
    const noisyOutput = `npm notice run npx\nnpm notice run 'skills' list\n${LIST_JSON}`;

    expect(parseInstalledSkills(noisyOutput)).toHaveLength(2);
  });

  it("treats output with no JSON array as no skills", () => {
    expect(parseInstalledSkills("")).toEqual([]);
    expect(parseInstalledSkills("No skills installed.")).toEqual([]);
  });

  it("sorts skills by name", () => {
    const listJson = `[
      { "name": "writing-plans", "agents": [], "source": "obra/superpowers" },
      { "name": "brainstorming", "agents": [], "source": "obra/superpowers" }
    ]`;

    expect(
      parseInstalledSkills(listJson).map((skill) => {
        return skill.name;
      }),
    ).toEqual(["brainstorming", "writing-plans"]);
  });
});
