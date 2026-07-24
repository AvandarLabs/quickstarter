//! `{{TOKEN}}` substitution for files that are mostly shared but carry a few
//! stack-specific or project-specific lines (for example `AGENTS.md`,
//! `README.md`, and document titles).
//!
//! Substitution runs over the composed output tree in place. Only text files
//! with a known extension are processed, symlinks are left untouched (so the
//! `CLAUDE.md -> AGENTS.md` link is preserved rather than dereferenced), and a
//! file is only rewritten when a token actually changed its contents.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

/// File extensions whose contents may contain tokens. Anything else (images,
/// lockfiles, binaries) is left byte-for-byte untouched.
const TEXT_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "cjs", "json", "md", "html", "css", "txt", "yml", "yaml",
];

/// A map of token name (without the `{{ }}` delimiters) to replacement value.
pub type Tokens = BTreeMap<String, String>;

/// Replaces every `{{KEY}}` occurrence in `text` with its value. Tokens with
/// no matching entry are left in place, which surfaces authoring mistakes
/// instead of silently deleting content.
pub fn substitute(text: &str, tokens: &Tokens) -> String {
    let mut result = text.to_string();
    for (key, value) in tokens {
        let placeholder = format!("{{{{{key}}}}}");
        if result.contains(&placeholder) {
            result = result.replace(&placeholder, value);
        }
    }
    result
}

/// Walks `root` and substitutes tokens in every eligible text file in place.
pub fn substitute_in_tree(root: &Path, tokens: &Tokens) -> Result<()> {
    for entry in walk_files(root)? {
        if !is_text_file(&entry) {
            continue;
        }
        let original = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;
        let replaced = substitute(&original, tokens);
        if replaced != original {
            std::fs::write(&entry, replaced)
                .with_context(|| format!("writing {}", entry.display()))?;
        }
    }
    Ok(())
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

fn is_text_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| TEXT_EXTENSIONS.contains(&extension))
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
    fn supports_multiline_values() {
        let tokens = tokens_from(&[("SUMMARY", "line one\nline two")]);
        let output = substitute("{{SUMMARY}}", &tokens);
        assert_eq!(output, "line one\nline two");
    }
}
