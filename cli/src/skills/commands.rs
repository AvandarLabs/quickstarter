//! The external commands that install the selected skills.
//!
//! Every skill is installed by its own command, so one failure costs exactly
//! one skill. Both tools are run through `npx` rather than added to the new
//! project's dependencies: they manage agent tooling, not application code.

/// The program every install command runs.
const NPX_PROGRAM: &str = "npx";

/// Stops `npx` from prompting before it fetches a missing package.
const NPX_YES_FLAG: &str = "-y";

/// The package that installs a skill from its source spec.
const SKILLS_PACKAGE_NAME: &str = "skills";

/// The agent frontends every skill is installed for. `npx skills` symlinks
/// `.claude/skills` for Claude Code; Cursor, OpenCode, and Codex read
/// `.agents/skills` directly.
pub const SKILL_AGENT_NAMES: &[&str] = &["claude-code", "cursor", "opencode", "codex"];

/// The one spec `npx skills` does not install: impeccable ships its own CLI.
pub const IMPECCABLE_SPEC: &str = "pbakaus/impeccable";

/// The npm package behind [`IMPECCABLE_SPEC`], run as its own installer.
pub const IMPECCABLE_PACKAGE_NAME: &str = "impeccable";

/// Provider names impeccable uses for the same four agent frontends.
pub const IMPECCABLE_PROVIDER_NAMES: &[&str] = &["claude", "cursor", "opencode", "codex"];

/// One external command that installs exactly one skill.
#[derive(Debug, Clone)]
pub struct SkillCommand {
    /// The spec of the skill this command installs, used in progress and
    /// warning output.
    pub label: String,
    /// Program to execute.
    pub program: String,
    /// Arguments passed to the program.
    pub args: Vec<String>,
}

impl SkillCommand {
    /// Renders the command as a copy-pasteable shell line.
    ///
    /// Nothing is quoted: a source spec cannot contain whitespace, which
    /// `cli/tests/skills_manifest_test.rs` enforces for the real manifest.
    pub fn shell_line(&self) -> String {
        let mut line = self.program.clone();
        for arg in &self.args {
            line.push(' ');
            line.push_str(arg);
        }
        line
    }
}

/// Builds the install command for every spec, in the order given.
pub fn install_commands(specs: &[String]) -> Vec<SkillCommand> {
    specs.iter().map(|spec| install_command(spec)).collect()
}

/// The npm package of every self-installing skill among `specs`, in order.
///
/// The generated project needs these names, not the specs: its own
/// `scripts/skills/update-skills.sh` updates them through their own CLI, while
/// `npx skills` updates everything else. A project that selects none gets an
/// empty list and the script skips that step.
pub fn self_installing_skill_names(specs: &[String]) -> Vec<String> {
    specs
        .iter()
        .filter_map(|spec| self_installing_package(spec))
        .map(str::to_string)
        .collect()
}

/// Builds the one command that installs `spec`, routing the self-installing
/// skills to their own installer.
fn install_command(spec: &str) -> SkillCommand {
    match self_installing_package(spec) {
        Some(IMPECCABLE_PACKAGE_NAME) => impeccable_install_command(),
        _ => skills_add_command(spec),
    }
}

/// The npm package that installs `spec` itself, for the specs `npx skills`
/// does not manage.
fn self_installing_package(spec: &str) -> Option<&'static str> {
    (spec == IMPECCABLE_SPEC).then_some(IMPECCABLE_PACKAGE_NAME)
}

/// `npx skills add <spec>`, for the agent frontends this project supports.
///
/// `--agent` is variadic, so the frontends are passed as separate arguments: a
/// comma-joined list reads as one unknown agent name. The trailing `-y`
/// accepts the security prompt, which is what makes the run non-interactive.
fn skills_add_command(spec: &str) -> SkillCommand {
    let mut args = vec![
        NPX_YES_FLAG.to_string(),
        SKILLS_PACKAGE_NAME.to_string(),
        "add".to_string(),
        spec.to_string(),
        "--agent".to_string(),
    ];
    args.extend(SKILL_AGENT_NAMES.iter().map(|agent| (*agent).to_string()));
    args.push(NPX_YES_FLAG.to_string());
    SkillCommand {
        label: spec.to_string(),
        program: NPX_PROGRAM.to_string(),
        args,
    }
}

/// Impeccable's own installer. Passing both `--providers` and `--scope` is
/// what keeps it from asking any questions.
fn impeccable_install_command() -> SkillCommand {
    SkillCommand {
        label: IMPECCABLE_SPEC.to_string(),
        program: NPX_PROGRAM.to_string(),
        args: vec![
            NPX_YES_FLAG.to_string(),
            IMPECCABLE_PACKAGE_NAME.to_string(),
            "install".to_string(),
            // Impeccable's own flag, not npx's: without it the installer stops
            // to ask whether to install its design hook, in the middle of a
            // scaffolding run that is supposed to ask nothing.
            "--yes".to_string(),
            format!("--providers={}", IMPECCABLE_PROVIDER_NAMES.join(",")),
            "--scope=project".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn specs(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn a_normal_spec_is_installed_with_npx_skills_add() {
        let commands = install_commands(&specs(&["obra/superpowers/skills/brainstorming"]));

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].program, "npx");
        assert_eq!(
            commands[0].args,
            specs(&[
                "-y",
                "skills",
                "add",
                "obra/superpowers/skills/brainstorming",
                "--agent",
                "claude-code",
                "cursor",
                "opencode",
                "codex",
                "-y",
            ])
        );
    }

    #[test]
    fn every_agent_frontend_is_installed_for() {
        let commands = install_commands(&specs(&["owner/repo"]));
        for agent in SKILL_AGENT_NAMES {
            assert!(
                commands[0].args.iter().any(|arg| arg == agent),
                "missing {agent}"
            );
        }
    }

    #[test]
    fn impeccable_is_installed_with_its_own_cli() {
        let commands = install_commands(&specs(&[IMPECCABLE_SPEC]));

        assert_eq!(commands[0].program, "npx");
        assert_eq!(
            commands[0].args,
            specs(&[
                "-y",
                "impeccable",
                "install",
                "--yes",
                "--providers=claude,cursor,opencode,codex",
                "--scope=project",
            ])
        );
        assert!(!commands[0].args.iter().any(|arg| arg == "skills"));
    }

    #[test]
    fn every_command_is_labelled_with_the_skill_it_installs() {
        let wanted = specs(&["owner/repo", IMPECCABLE_SPEC, "owner/repo/skills/one"]);
        let commands = install_commands(&wanted);

        let labels: Vec<&str> = commands
            .iter()
            .map(|command| command.label.as_str())
            .collect();
        assert_eq!(labels, wanted);
    }

    #[test]
    fn the_self_installing_skills_of_a_project_are_named_by_package() {
        let names = self_installing_skill_names(&specs(&["owner/repo", IMPECCABLE_SPEC]));
        assert_eq!(names, vec![IMPECCABLE_PACKAGE_NAME]);
    }

    #[test]
    fn a_project_with_no_self_installing_skill_names_none() {
        assert!(self_installing_skill_names(&specs(&["owner/repo"])).is_empty());
    }

    #[test]
    fn a_command_renders_as_a_shell_line() {
        let commands = install_commands(&specs(&["owner/repo"]));
        assert_eq!(
            commands[0].shell_line(),
            "npx -y skills add owner/repo --agent claude-code cursor opencode codex -y"
        );
    }
}
