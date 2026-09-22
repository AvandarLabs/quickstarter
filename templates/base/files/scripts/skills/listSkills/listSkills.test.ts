import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { Acclimate } from "@avandar/acclimate";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  IMPECCABLE_SKILL_NAME,
  SELF_INSTALLING_SKILL_NAMES,
} from "../constants";
import { listSkills } from "./listSkills";
import type { CommandResult } from "../skills.types";

const LOCK_JSON = JSON.stringify({
  version: 1,
  skills: {
    brainstorming: { source: "obra/superpowers" },
    "writing-plans": { source: "obra/superpowers" },
  },
});

const LIST_OUTPUT = JSON.stringify([
  {
    name: "brainstorming",
    path: "/app/.agents/skills/brainstorming",
    agents: ["Claude Code", "Codex", "Cursor", "OpenCode"],
    source: "obra/superpowers",
  },
  {
    name: "impeccable",
    path: "/app/.agents/skills/impeccable",
    agents: ["Claude Code"],
    source: null,
  },
]);

/**
 * Whether the scaffolder gave this project impeccable. The listing only claims
 * impeccable is missing for a project that has it, so the test for that row
 * only applies to such a project.
 */
const USES_IMPECCABLE = SELF_INSTALLING_SKILL_NAMES.includes(
  IMPECCABLE_SKILL_NAME,
);

function createRunner(result: Partial<CommandResult>) {
  return () => {
    return Promise.resolve({
      exitCode: 0,
      stdout: "",
      stderr: "",
      ...result,
    });
  };
}

function loggedOutput(): string {
  const logMock = vi.mocked(Acclimate.log);
  return logMock.mock.calls
    .map((call) => {
      return String(call[0]);
    })
    .join("\n");
}

describe("listSkills", () => {
  let projectRootPath = "";

  beforeEach(async () => {
    vi.restoreAllMocks();
    vi.spyOn(Acclimate, "log").mockImplementation(() => {});
    projectRootPath = await mkdtemp(path.join(tmpdir(), "skills-list-"));
    await writeFile(path.join(projectRootPath, "skills-lock.json"), LOCK_JSON);
  });

  afterEach(async () => {
    await rm(projectRootPath, { force: true, recursive: true });
  });

  it("lists the skills of both managers together", async () => {
    await listSkills({
      projectRootPath,
      runner: createRunner({ stdout: LIST_OUTPUT }),
    });

    const output = loggedOutput();
    expect(output).toContain("brainstorming");
    expect(output).toContain("impeccable");
    expect(output).toContain("obra/superpowers");
    expect(output).toContain("pbakaus/impeccable");
  });

  it("calls out locked skills that are not installed", async () => {
    await listSkills({
      projectRootPath,
      runner: createRunner({ stdout: LIST_OUTPUT }),
    });

    const output = loggedOutput();
    expect(output).toContain("writing-plans");
    expect(output).toContain("pnpm install");
  });

  it("names the agent frontends the skills are installed for", async () => {
    await listSkills({
      projectRootPath,
      runner: createRunner({ stdout: LIST_OUTPUT }),
    });

    expect(loggedOutput()).toContain("Claude Code");
  });

  it("warns instead of claiming nothing is installed when the CLI fails", async () => {
    await listSkills({
      projectRootPath,
      runner: createRunner({ exitCode: 1, stderr: "npx: not found" }),
    });

    const output = loggedOutput();
    expect(output).toMatch(/could not/i);
    expect(output).toContain("npx: not found");
  });

  it.runIf(USES_IMPECCABLE)(
    "reports impeccable as missing in a project with nothing installed",
    async () => {
      await rm(path.join(projectRootPath, "skills-lock.json"));

      await listSkills({
        projectRootPath,
        runner: createRunner({ stdout: "[]" }),
      });

      const output = loggedOutput();
      expect(output).toContain("impeccable");
      expect(output).toContain("not installed");
    },
  );
});
