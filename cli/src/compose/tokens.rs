//! `{{TOKEN}}` substitution for files that are mostly shared but carry a few
//! tag-specific or project-specific lines (for example `AGENTS.md`,
//! `README.md`, and document titles).
//!
//! Substitution runs over the composed output tree in place. Every text file
//! is processed, whatever it is called: a project type puts its ignores in
//! `.gitignore` and its tasks in a `justfile`, neither of which has an
//! extension, so a list of known extensions would silently skip them and ship
//! a `{{TOKEN}}` to the user. Binary files are left byte-for-byte untouched,
//! symlinks are left alone (so the `CLAUDE.md -> AGENTS.md` link is preserved
//! rather than dereferenced), and a file is only rewritten when a token
//! actually changed its contents.

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, Result};

/// Extensions whose contents are never template text. They are skipped rather
/// than read, so a large asset is never loaded just to look for a `{{`.
const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "icns", "pdf", "woff", "woff2", "ttf", "otf",
    "eot", "zip", "gz", "tgz", "mp3", "mp4", "webm", "wasm",
];

/// A map of token name (without the `{{ }}` delimiters) to replacement value.
pub type Tokens = BTreeMap<String, String>;

/// Replaces every `{{KEY}}` occurrence in `text` with its value. Tokens with
/// no matching entry are left in place, which surfaces authoring mistakes
/// instead of silently deleting content.
///
/// A line holding nothing but one token (leading and trailing whitespace
/// allowed) whose value is empty is removed entirely, newline included. That
/// is what lets a shared file carry a **seam**: a line a capability fills in
/// and a project without that capability never sees. Substituting an empty
/// value in place would leave a blank line instead, which `cargo fmt --check`
/// reports as a diff in the generated project. A token that shares its line
/// with anything else is substituted where it stands, empty or not.
pub fn substitute(text: &str, tokens: &Tokens) -> String {
    text.split_inclusive('\n')
        .filter(|line| !is_emptied_seam(line, tokens))
        .map(|line| substitute_in_line(line, tokens))
        .collect()
}

/// Replaces every `{{KEY}}` occurrence in one line, leaving the rest alone.
fn substitute_in_line(line: &str, tokens: &Tokens) -> String {
    let mut result = line.to_string();
    for (key, value) in tokens {
        let placeholder = format!("{{{{{key}}}}}");
        if result.contains(&placeholder) {
            result = result.replace(&placeholder, value);
        }
    }
    result
}

/// Whether `line` is a seam that the chosen tags left empty: nothing but a
/// single token, whose value is the empty string. A token no tag defines is
/// not one, so an authoring mistake stays visible rather than deleting a line.
fn is_emptied_seam(line: &str, tokens: &Tokens) -> bool {
    let trimmed = line.trim();
    let Some(name) = trimmed.strip_prefix("{{").and_then(|rest| rest.strip_suffix("}}")) else {
        return false;
    };
    tokens.get(name).is_some_and(|value| value.is_empty())
}

/// Walks `root` and substitutes tokens in every text file in place.
pub fn substitute_in_tree(root: &Path, tokens: &Tokens) -> Result<()> {
    for entry in walk_files(root)? {
        let Some(original) = read_text(&entry)? else {
            continue;
        };
        let replaced = substitute(&original, tokens);
        if replaced != original {
            std::fs::write(&entry, replaced)
                .with_context(|| format!("writing {}", entry.display()))?;
        }
    }
    Ok(())
}

/// The contents of `path`, or `None` when it is not text: a known binary
/// extension, or bytes that are not UTF-8. Any other read failure is a real
/// problem and is reported.
fn read_text(path: &Path) -> Result<Option<String>> {
    if has_binary_extension(path) {
        return Ok(None);
    }
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::InvalidData => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}

/// Collects the regular files under `root`, skipping symlinks so links are
/// preserved rather than followed and rewritten.
fn walk_files(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    collect_files(root, &mut files)?;
    Ok(files)
}

fn collect_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

/// Whether `path` names a file whose contents are known not to be text.
fn has_binary_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| BINARY_EXTENSIONS.contains(&extension.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens_from(pairs: &[(&str, &str)]) -> Tokens {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn replaces_all_occurrences_of_a_token() {
        let tokens = tokens_from(&[("PROJECT_NAME", "my-app")]);
        let output = substitute("# {{PROJECT_NAME}}\ntitle: {{PROJECT_NAME}}", &tokens);
        assert_eq!(output, "# my-app\ntitle: my-app");
    }

    #[test]
    fn leaves_unknown_tokens_in_place() {
        let tokens = tokens_from(&[("KNOWN", "value")]);
        let output = substitute("{{KNOWN}} and {{UNKNOWN}}", &tokens);
        assert_eq!(output, "value and {{UNKNOWN}}");
    }

    #[test]
    fn substitutes_shell_scripts_too() {
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join("update-skills.sh");
        std::fs::write(&script, "SELF_INSTALLING_SKILLS=\"{{SELF_INSTALLING_SKILLS}}\"").unwrap();

        substitute_in_tree(temp.path(), &tokens_from(&[("SELF_INSTALLING_SKILLS", "impeccable")]))
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(&script).unwrap(),
            "SELF_INSTALLING_SKILLS=\"impeccable\""
        );
    }

    #[test]
    fn substitutes_rust_sources_and_manifests_too() {
        // A Rust project type names its crate in Cargo.toml, so the manifest
        // and the sources are as much template text as a package.json is.
        let temp = tempfile::tempdir().unwrap();
        let manifest = temp.path().join("Cargo.toml");
        std::fs::write(&manifest, "name = \"{{PACKAGE_NAME}}\"").unwrap();
        let source = temp.path().join("src/main.rs");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "// {{PROJECT_NAME}}").unwrap();

        substitute_in_tree(
            temp.path(),
            &tokens_from(&[("PACKAGE_NAME", "my-app"), ("PROJECT_NAME", "My App")]),
        )
        .unwrap();

        assert_eq!(std::fs::read_to_string(&manifest).unwrap(), "name = \"my-app\"");
        assert_eq!(std::fs::read_to_string(&source).unwrap(), "// My App");
    }

    #[test]
    fn substitutes_files_that_have_no_extension_at_all() {
        // A project type puts its ignores in `.gitignore` and its tasks in a
        // `justfile`. Neither has an extension, and both carry tokens.
        let temp = tempfile::tempdir().unwrap();
        let ignores = temp.path().join(".gitignore");
        std::fs::write(&ignores, "node_modules\n{{PROJECT_TYPE_IGNORES}}").unwrap();
        let justfile = temp.path().join("justfile");
        std::fs::write(&justfile, "# {{PROJECT_NAME}}").unwrap();

        substitute_in_tree(
            temp.path(),
            &tokens_from(&[("PROJECT_TYPE_IGNORES", "/target"), ("PROJECT_NAME", "My App")]),
        )
        .unwrap();

        assert_eq!(std::fs::read_to_string(&ignores).unwrap(), "node_modules\n/target");
        assert_eq!(std::fs::read_to_string(&justfile).unwrap(), "# My App");
    }

    #[test]
    fn leaves_binary_files_byte_for_byte_alone() {
        let temp = tempfile::tempdir().unwrap();
        let image = temp.path().join("logo.png");
        let bytes = [0x89, 0x50, 0x4e, 0x47, 0xff, 0xfe, 0x00, 0x01];
        std::fs::write(&image, bytes).unwrap();

        substitute_in_tree(temp.path(), &tokens_from(&[("PROJECT_NAME", "My App")])).unwrap();

        assert_eq!(std::fs::read(&image).unwrap(), bytes);
    }

    #[test]
    fn leaves_a_file_that_is_not_utf8_alone_whatever_it_is_called() {
        let temp = tempfile::tempdir().unwrap();
        let blob = temp.path().join("data.bin");
        let bytes = [0xff, 0xfe, 0x00];
        std::fs::write(&blob, bytes).unwrap();

        substitute_in_tree(temp.path(), &tokens_from(&[("PROJECT_NAME", "My App")])).unwrap();

        assert_eq!(std::fs::read(&blob).unwrap(), bytes);
    }

    #[test]
    fn supports_multiline_values() {
        let tokens = tokens_from(&[("SUMMARY", "line one\nline two")]);
        let output = substitute("{{SUMMARY}}", &tokens);
        assert_eq!(output, "line one\nline two");
    }

    #[test]
    fn a_line_holding_nothing_but_an_empty_token_is_removed_entirely() {
        // A seam a project adds nothing to must leave no blank line behind:
        // `cargo fmt --check` is part of the generated project's own `just
        // check`, and a stray blank line fails it.
        let tokens = tokens_from(&[("EXTRA_MODULES", "")]);

        let output = substitute("pub mod cli;\n{{EXTRA_MODULES}}\npub mod theme;\n", &tokens);

        assert_eq!(output, "pub mod cli;\npub mod theme;\n");
    }

    #[test]
    fn an_indented_line_holding_nothing_but_an_empty_token_is_removed_too() {
        let tokens = tokens_from(&[("EXTRA_DISPATCH", "")]);

        let output =
            substitute("match command {\n        {{EXTRA_DISPATCH}}\n    }\n", &tokens);

        assert_eq!(output, "match command {\n    }\n");
    }

    #[test]
    fn a_line_holding_nothing_but_a_filled_token_substitutes_as_it_always_did() {
        // Including a multi-line value: the indentation is the line's, so only
        // the first line of the value inherits it, exactly as before.
        let tokens = tokens_from(&[("EXTRA_SUBCOMMANDS", "/// Open the terminal UI.\nUi,")]);

        let output = substitute("enum Command {\n    {{EXTRA_SUBCOMMANDS}}\n}\n", &tokens);

        assert_eq!(output, "enum Command {\n    /// Open the terminal UI.\nUi,\n}\n");
    }

    #[test]
    fn an_empty_token_sharing_its_line_with_other_text_keeps_the_old_behavior() {
        let tokens = tokens_from(&[("EXTRA_RULES", "")]);

        let output = substitute("- rules {{EXTRA_RULES}}\n", &tokens);

        assert_eq!(output, "- rules \n");
    }

    #[test]
    fn an_empty_token_on_its_own_line_is_removed_even_without_a_trailing_newline() {
        let tokens = tokens_from(&[("EXTRA_RULES", "")]);

        let output = substitute("- rules\n{{EXTRA_RULES}}", &tokens);

        assert_eq!(output, "- rules\n");
    }

    #[test]
    fn a_line_holding_only_a_token_nothing_defines_is_left_in_place() {
        // An unfilled token is an authoring mistake, and the composed project
        // has to show it rather than quietly losing the line.
        let tokens = tokens_from(&[("KNOWN", "")]);

        let output = substitute("a\n{{UNKNOWN}}\nb\n", &tokens);

        assert_eq!(output, "a\n{{UNKNOWN}}\nb\n");
    }
}
