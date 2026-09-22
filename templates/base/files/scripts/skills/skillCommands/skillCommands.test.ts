import { describe, expect, it } from "vitest";
import {
  createImpeccableInstallCommand,
  createSkillsAddCommand,
  createSkillsListCommand,
  createSkillsUpdateCommand,
} from "./skillCommands";

describe("createSkillsListCommand", () => {
  it("asks for machine-readable output", () => {
    const command = createSkillsListCommand();

    expect(command.command).toBe("npx");
    expect(command.args).toEqual(["-y", "skills", "list", "--json"]);
  });
});

describe("createSkillsAddCommand", () => {
  const command = createSkillsAddCommand({
    source: "obra/superpowers",
    skillNames: ["brainstorming", "writing-plans"],
  });

  it("passes each skill and agent as its own argument", () => {
    // Both flags are variadic: a comma-joined list is read as one unknown
    // skill name and the install finds nothing.
    expect(command.args).toEqual([
      "-y",
      "skills",
      "add",
      "obra/superpowers",
      "--skill",
      "brainstorming",
      "writing-plans",
      "--agent",
      "claude-code",
      "cursor",
      "opencode",
      "codex",
      "--full-depth",
      "-y",
    ]);
  });

  it("searches the whole repository", () => {
    // Without --full-depth a shallow SKILL.md stops the search, so skills
    // nested deeper in a monorepo are reported as not found.
    expect(command.args).toContain("--full-depth");
  });

  it("names the source repository in its label", () => {
    expect(command.label).toBe("skills add obra/superpowers");
  });
});

describe("createImpeccableInstallCommand", () => {
  it("installs non-interactively for every frontend", () => {
    const command = createImpeccableInstallCommand();

    expect(command.args).toEqual([
      "-y",
      "impeccable",
      "install",
      "--providers=claude,cursor,opencode,codex",
      "--scope=project",
    ]);
  });
});

describe("createSkillsUpdateCommand", () => {
  it("runs the update script with no arguments", () => {
    // The script is the single implementation of "update every skill", so
    // there is nothing for this side to decide or pass along.
    const command = createSkillsUpdateCommand();

    expect(command.command).toBe("./scripts/skills/update-skills.sh");
    expect(command.args).toEqual([]);
  });

  it("labels itself for progress output", () => {
    expect(createSkillsUpdateCommand().label).toBe("skills update");
  });
});
