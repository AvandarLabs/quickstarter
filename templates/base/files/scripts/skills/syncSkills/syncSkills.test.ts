import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { installSkills, updateSkills } from "./syncSkills";
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

describe("skills sync", () => {
  let projectRootPath = "";

  beforeEach(async () => {
    vi.restoreAllMocks();
    vi.spyOn(console, "log").mockImplementation(() => {});
    projectRootPath = await mkdtemp(path.join(tmpdir(), "skills-sync-"));
    await writeFile(path.join(projectRootPath, "skills-lock.json"), LOCK_JSON);
  });

  afterEach(async () => {
    await rm(projectRootPath, { force: true, recursive: true });
  });

  async function installOnDisk(
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

  describe("installSkills", () => {
    it("installs every locked skill into an empty project", async () => {
      const { specs, runner } = createRecordingRunner();

      await installSkills({ projectRootPath, runner });

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

    it("leaves installed skills alone", async () => {
      await installOnDisk("brainstorming", "mantine-combobox", "impeccable");
      const { specs, runner } = createRecordingRunner();

      await installSkills({ projectRootPath, runner });

      expect(specs).toHaveLength(1);
      expect(specs[0]?.args).toContain("writing-plans");
    });

    it("runs nothing in a complete project", async () => {
      await installOnDisk(
        "brainstorming",
        "writing-plans",
        "mantine-combobox",
        "impeccable",
      );
      const { specs, runner } = createRecordingRunner();

      const result = await installSkills({ projectRootPath, runner });

      expect(specs).toEqual([]);
      expect(result.didRun).toBe(false);
    });

    it("treats a project with no lock file as nothing to install", async () => {
      await rm(path.join(projectRootPath, "skills-lock.json"));
      await installOnDisk("impeccable");
      const { specs, runner } = createRecordingRunner();

      await installSkills({ projectRootPath, runner });

      expect(specs).toEqual([]);
    });
  });

  describe("updateSkills", () => {
    it("refreshes a complete project", async () => {
      await installOnDisk(
        "brainstorming",
        "writing-plans",
        "mantine-combobox",
        "impeccable",
      );
      const { specs, runner } = createRecordingRunner();

      const result = await updateSkills({ projectRootPath, runner });

      expect(
        specs.map((spec) => {
          return spec.label;
        }),
      ).toEqual([
        "skills add mantinedev/skills",
        "skills add obra/superpowers",
        "impeccable update",
      ]);
      expect(result.didRun).toBe(true);
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
  });

  describe("the quiet postinstall path", () => {
    it("skips entirely in CI", async () => {
      const { specs, runner } = createRecordingRunner();

      const result = await installSkills({
        projectRootPath,
        runner,
        quiet: true,
        environment: { CI: "true" },
      });

      expect(specs).toEqual([]);
      expect(result.wasSkipped).toBe(true);
    });

    it("honors SKIP_SKILLS_INSTALL", async () => {
      const { specs, runner } = createRecordingRunner();

      await installSkills({
        projectRootPath,
        runner,
        quiet: true,
        environment: { SKIP_SKILLS_INSTALL: "1" },
      });

      expect(specs).toEqual([]);
    });

    it("still runs in CI when asked for directly", async () => {
      const { specs, runner } = createRecordingRunner();

      await installSkills({
        projectRootPath,
        runner,
        environment: { CI: "true" },
      });

      expect(specs.length).toBeGreaterThan(0);
    });
  });
});
