import { Acclimate } from "@avandar/acclimate";
import { listSkills } from "./listSkills/listSkills";
import { installSkills, updateSkills } from "./syncSkills/syncSkills";

/**
 * `pnpm skills` - the front door to this project's agent skills.
 *
 * Two tools install skills here and neither knows about the other: `npx
 * skills` handles everything in `skills-lock.json`, and `npx impeccable`
 * installs itself. This CLI wraps both so there is one place to ask what is
 * installed and one place to change it.
 *
 * The skills themselves are not tracked in git. `install` and `update` are
 * separate on purpose, and they mirror how pnpm treats packages: `postinstall`
 * runs `install`, which only adds what the lock asks for and is missing, while
 * `update` is the explicit step that fetches newer versions.
 */

const QUIET_OPTION = {
  name: "--quiet",
  type: "boolean",
  required: false,
  defaultValue: false,
  description:
    "Postinstall mode: minimal output, and never fail the install that " +
    "triggered it.",
} as const;

const InstallSkillsCLI = Acclimate.createCLI("install")
  .description(
    "Install the skills in skills-lock.json that are not on disk yet.",
  )
  .addOption(QUIET_OPTION)
  .action(async ({ quiet }) => {
    await installSkills({ quiet });
  });

const UpdateSkillsCLI = Acclimate.createCLI("update")
  .description("Update every installed skill to its latest version.")
  .addOption(QUIET_OPTION)
  .action(async ({ quiet }) => {
    const result = await updateSkills({ quiet });

    // An update the user asked for is allowed to fail loudly. `install` never
    // is: it runs from `postinstall`, and a skill that could not be fetched
    // must not break `pnpm install`.
    if (result.failedLabels.length > 0 && !quiet) {
      process.exitCode = 1;
    }
  });

const ListSkillsCLI = Acclimate.createCLI("list")
  .description("Show every agent skill this project has, from both managers.")
  .action(async () => {
    await listSkills();
  });

const SkillsCLI = Acclimate.createCLI("skills")
  .description("Manage this project's agent skills.")
  .addCommand("install", InstallSkillsCLI)
  .addCommand("list", ListSkillsCLI)
  .addCommand("update", UpdateSkillsCLI)
  .action(async () => {
    await listSkills();
  });

Acclimate.run(SkillsCLI);
