import { Acclimate } from "@avandar/acclimate";
import { listSkills } from "./listSkills/listSkills";
import { updateSkills } from "./updateSkills/updateSkills";

/**
 * `pnpm skills` - the front door to this project's agent skills.
 *
 * Two tools install skills here and neither knows about the other: `npx
 * skills` handles everything in `skills-lock.json`, and `npx impeccable`
 * installs itself. This CLI wraps both so there is one place to ask what is
 * installed and one place to bring it up to date.
 *
 * The skills themselves are not tracked in git. `postinstall` calls the
 * `update` command with `--only-missing`, so `pnpm install` restores whatever
 * the lock asks for and a complete project costs nothing.
 */

const UpdateSkillsCLI = Acclimate.createCLI("update")
  .description(
    "Install or refresh every skill in skills-lock.json, plus impeccable.",
  )
  .addOption({
    name: "--only-missing",
    type: "boolean",
    required: false,
    defaultValue: false,
    description:
      "Only install skills that are absent, instead of refreshing all of them.",
  })
  .addOption({
    name: "--quiet",
    type: "boolean",
    required: false,
    defaultValue: false,
    description:
      "Postinstall mode: minimal output, and never fail the install that " +
      "triggered it.",
  })
  .action(async ({ onlyMissing, quiet }) => {
    const result = await updateSkills({ onlyMissing, quiet });

    // A skill that failed to install must not break `pnpm install`: the app
    // itself is unaffected, and the next install tries again.
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
  .addCommand("list", ListSkillsCLI)
  .addCommand("update", UpdateSkillsCLI)
  .action(async () => {
    await listSkills();
  });

Acclimate.run(SkillsCLI);
