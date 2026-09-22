import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  IMPECCABLE_SKILL_NAME,
  SELF_INSTALLING_SKILL_NAMES,
} from "../constants";
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

/**
 * The impeccable install an install plans in this project, if any. The
 * scaffolder decides which skills install themselves, so what an empty project
 * expects follows that list rather than assuming impeccable is in it.
 */
const IMPECCABLE_INSTALL_LABELS = SELF_INSTALLING_SKILL_NAMES.includes(
  IMPECCABLE_SKILL_NAME,
)
  ? ["impeccable install"]
  : [];

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
        ...IMPECCABLE_INSTALL_LABELS,
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

    it("reports every command that failed without stopping", async () => {
      const { specs, runner } = createRecordingRunner(1);

      const result = await installSkills({ projectRootPath, runner });

      expect(specs).toHaveLength(2 + IMPECCABLE_INSTALL_LABELS.length);
      expect(result.failedLabels).toEqual([
        "skills add mantinedev/skills",
        "skills add obra/superpowers",
        ...IMPECCABLE_INSTALL_LABELS,
      ]);
    });
  });

  describe("updateSkills", () => {
    it("runs the update script and nothing else", async () => {
      await installOnDisk("brainstorming");
      const { specs, runner } = createRecordingRunner();

      const result = await updateSkills({ projectRootPath, runner });

      // Updating is the script's job in full: it drives both managers, so
      // neither the lock nor what is on disk is consulted here.
      expect(specs).toHaveLength(1);
      expect(specs[0]?.command).toBe("./scripts/skills/update-skills.sh");
      expect(specs[0]?.args).toEqual([]);
      expect(result.didRun).toBe(true);
    });

    it("runs the script even in a project with nothing installed", async () => {
      const { specs, runner } = createRecordingRunner();

      await updateSkills({ projectRootPath, runner });

      expect(
        specs.map((spec) => {
          return spec.label;
        }),
      ).toEqual(["skills update"]);
    });

    it("reports a failing script instead of swallowing it", async () => {
      const { runner } = createRecordingRunner(1);

      const result = await updateSkills({ projectRootPath, runner });

      expect(result.failedLabels).toEqual(["skills update"]);
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
