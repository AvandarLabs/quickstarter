import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { updateSkills } from "./updateSkills";
import type { CommandResult, CommandSpec } from "../skills.types";

const LOCK_JSON = JSON.stringify({
  version: 1,
  skills: {
    brainstorming: { source: "obra/superpowers" },
    "writing-plans": { source: "obra/superpowers" },
    "mantine-combobox": { source: "mantinedev/skills" },
  },
});

type RecordedRun = {
  specs: CommandSpec[];
  runner: (spec: CommandSpec) => Promise<CommandResult>;
};

function createRecordingRunner(exitCode = 0): RecordedRun {
  const specs: CommandSpec[] = [];
  return {
    specs,
    runner: (spec) => {
      specs.push(spec);
      return Promise.resolve({ exitCode, stdout: "", stderr: "boom" });
    },
  };
}

describe("updateSkills", () => {
  let projectRootPath = "";

  beforeEach(async () => {
    vi.restoreAllMocks();
    vi.spyOn(console, "log").mockImplementation(() => {});
    projectRootPath = await mkdtemp(path.join(tmpdir(), "skills-update-"));
    await writeFile(path.join(projectRootPath, "skills-lock.json"), LOCK_JSON);
  });

  afterEach(async () => {
    await rm(projectRootPath, { force: true, recursive: true });
  });

  async function installSkills(
    ...skillNames: readonly string[]
  ): Promise<void> {
    await Promise.all(
      skillNames.map((skillName) => {
        return mkdir(path.join(projectRootPath, ".agents/skills", skillName), {
          recursive: true,
        });
      }),
    );
  }

  it("refreshes every locked source and impeccable", async () => {
    const { specs, runner } = createRecordingRunner();

    await updateSkills({ projectRootPath, runner });

    expect(
      specs.map((spec) => {
        return spec.label;
      }),
    ).toEqual([
      "skills add mantinedev/skills",
      "skills add obra/superpowers",
      "impeccable install",
    ]);
  });

  it("restores only what is missing in only-missing mode", async () => {
    await installSkills("brainstorming", "mantine-combobox", "impeccable");
    const { specs, runner } = createRecordingRunner();

    await updateSkills({ projectRootPath, runner, onlyMissing: true });

    expect(specs).toHaveLength(1);
    expect(specs[0]?.args).toContain("writing-plans");
  });

  it("runs nothing when a complete project is only restoring", async () => {
    await installSkills(
      "brainstorming",
      "writing-plans",
      "mantine-combobox",
      "impeccable",
    );
    const { specs, runner } = createRecordingRunner();

    const result = await updateSkills({
      projectRootPath,
      runner,
      onlyMissing: true,
    });

    expect(specs).toEqual([]);
    expect(result.didRun).toBe(false);
  });

  it("skips entirely in CI when running quietly", async () => {
    const { specs, runner } = createRecordingRunner();

    const result = await updateSkills({
      projectRootPath,
      runner,
      quiet: true,
      environment: { CI: "true" },
    });

    expect(specs).toEqual([]);
    expect(result.wasSkipped).toBe(true);
  });

  it("honors SKIP_SKILLS_INSTALL when running quietly", async () => {
    const { specs, runner } = createRecordingRunner();

    await updateSkills({
      projectRootPath,
      runner,
      quiet: true,
      environment: { SKIP_SKILLS_INSTALL: "1" },
    });

    expect(specs).toEqual([]);
  });

  it("still runs in CI when the update was asked for directly", async () => {
    const { specs, runner } = createRecordingRunner();

    await updateSkills({
      projectRootPath,
      runner,
      environment: { CI: "true" },
    });

    expect(specs.length).toBeGreaterThan(0);
  });

  it("reports every command that failed without stopping", async () => {
    const { specs, runner } = createRecordingRunner(1);

    const result = await updateSkills({ projectRootPath, runner });

    expect(specs).toHaveLength(3);
    expect(result.failedLabels).toEqual([
      "skills add mantinedev/skills",
      "skills add obra/superpowers",
      "impeccable install",
    ]);
  });

  it("treats a project with no lock file as nothing to restore", async () => {
    await rm(path.join(projectRootPath, "skills-lock.json"));
    await installSkills("impeccable");
    const { specs, runner } = createRecordingRunner();

    await updateSkills({ projectRootPath, runner, onlyMissing: true });

    expect(specs).toEqual([]);
  });
});
