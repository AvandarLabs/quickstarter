//! Deep-merge of `Cargo.toml` fragments, comments and all.
//!
//! The Rust half of the manifest merge, mirroring
//! [`crate::compose::package_json`]: the project type ships the manifest every
//! project of that type shares, and each capability ships a fragment with only
//! the dependencies it adds. The semantics are the same, so the two merges
//! read alike: tables merge recursively, `[dependencies]` blocks combine
//! rather than replace, and a later fragment wins on a scalar.
//!
//! What differs is the serialization. Every dependency in these templates
//! carries the one line saying why it is there, and a parse-and-reserialize
//! through `serde` would delete every one of them, so the merge edits the
//! parsed document in place (`toml_edit`) instead: the base keeps its own
//! bytes, and an entry arriving from a fragment brings its comment with it.
//!
//! The merge runs before `{{TOKEN}}` substitution, so a token in a manifest
//! has to sit inside a TOML value (`name = "{{PACKAGE_NAME}}"`), exactly as it
//! already has to sit inside a JSON string in a `package.json`. A manifest is
//! parsed as it was authored, never as it will be rendered.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use toml_edit::{DocumentMut, Item, Key, Table};

/// Reads the base manifest and merges each fragment onto it in order,
/// returning the result serialized with every comment and the base's own
/// formatting intact.
///
/// A fragment that does not exist is treated as empty, so a capability that
/// adds no dependency needs no `Cargo.toml` at all. Whether there is a base
/// manifest to merge onto is the caller's question.
pub fn merge_files(base_path: &Path, fragment_paths: &[PathBuf]) -> Result<String> {
    let base_text = std::fs::read_to_string(base_path)
        .with_context(|| format!("reading base manifest {}", base_path.display()))?;
    let mut base: DocumentMut = base_text
        .parse()
        .with_context(|| format!("parsing base manifest {}", base_path.display()))?;

    for fragment_path in fragment_paths {
        if let Some(fragment) = read_fragment(fragment_path)? {
            deep_merge(base.as_table_mut(), fragment.as_table());
        }
    }

    let mut serialized = base.to_string();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    Ok(serialized)
}

/// Recursively merges `overlay` into `target`.
///
/// Two tables are merged key by key, so `[dependencies]` blocks combine rather
/// than replace. Anything else in `overlay` replaces what `target` holds,
/// which is how a fragment overrides a version. An entry arriving from the
/// overlay brings the comment written above it; one it overrides without a
/// comment of its own keeps the comment it already had.
pub fn deep_merge(target: &mut Table, overlay: &Table) {
    for (key, item) in entries(overlay) {
        if merge_nested(target, key.get(), item) {
            continue;
        }
        let key = merged_key(target, key);
        target.insert_formatted(&key, adopted(item));
    }
}

/// Reads one fragment, or `None` when the layer ships no manifest.
fn read_fragment(fragment_path: &Path) -> Result<Option<DocumentMut>> {
    if !fragment_path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(fragment_path)
        .with_context(|| format!("reading fragment {}", fragment_path.display()))?;
    let fragment = text
        .parse()
        .with_context(|| format!("parsing fragment {}", fragment_path.display()))?;
    Ok(Some(fragment))
}

/// Every entry of `table` with its key, which is where a comment lives.
fn entries(table: &Table) -> Vec<(&Key, &Item)> {
    table.iter().filter_map(|(name, _)| table.get_key_value(name)).collect()
}

/// Merges `item` into the table `target` already stores under `name`, when
/// both sides are tables, and reports whether it did. Anything else is an
/// overwrite, which is the caller's job.
fn merge_nested(target: &mut Table, name: &str, item: &Item) -> bool {
    let Some(overlay_table) = item.as_table() else {
        return false;
    };
    let Some(target_table) = target.get_mut(name).and_then(Item::as_table_mut) else {
        return false;
    };
    deep_merge(target_table, overlay_table);
    true
}

/// The key to store an entry under: the overlay's, so its comment comes with
/// it, unless the overlay wrote no comment and the entry it replaces has one.
/// A fragment bumping a version should not delete the line saying why the
/// dependency is there.
fn merged_key(target: &Table, overlay_key: &Key) -> Key {
    if carries_a_comment(overlay_key) {
        return overlay_key.clone();
    }
    match target.key(overlay_key.get()) {
        Some(existing) if carries_a_comment(existing) => existing.clone(),
        _ => overlay_key.clone(),
    }
}

/// Whether anything was written above `key` in its own layer.
fn carries_a_comment(key: &Key) -> bool {
    key.leaf_decor()
        .prefix()
        .and_then(|prefix| prefix.as_str())
        .is_some_and(|prefix| prefix.contains('#'))
}

/// A clone of `item` ready to live in another document: it keeps its comments,
/// but a table forgets the position it held in the fragment so it renders
/// where it was inserted rather than where it used to sit. Position is what
/// orders the sections of the output, so forgetting it is what makes composing
/// the same project twice byte-identical.
fn adopted(item: &Item) -> Item {
    let mut item = item.clone();
    if let Some(table) = item.as_table_mut() {
        forget_positions(table);
    }
    item
}

fn forget_positions(table: &mut Table) {
    table.set_position(None);
    for (_, item) in table.iter_mut() {
        if let Some(nested) = item.as_table_mut() {
            forget_positions(nested);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A manifest the way the templates author one: a comment above every
    /// dependency, saying why it is there.
    const BASE: &str = "\
[package]
name = \"app\"
edition = \"2024\"

[dependencies]
# Error handling with context.
anyhow = \"1\"
# Argument parsing.
clap = { version = \"4\", features = [\"derive\"] }
";

    fn document(text: &str) -> DocumentMut {
        text.parse().unwrap()
    }

    fn merged(base: &str, fragment: &str) -> String {
        let mut base = document(base);
        deep_merge(base.as_table_mut(), document(fragment).as_table());
        base.to_string()
    }

    fn write(dir: &Path, name: &str, contents: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn a_fragment_adds_a_dependency_and_brings_its_comment_with_it() {
        let output = merged(
            BASE,
            "[dependencies]\n# Terminal UI rendering.\nratatui = \"0.29\"\n",
        );

        assert!(output.contains("# Terminal UI rendering.\nratatui = \"0.29\""), "{output}");
        // The block combined rather than replaced: the base's own entries stay.
        assert!(output.contains("anyhow = \"1\""), "{output}");
        assert!(output.contains("clap = "), "{output}");
    }

    #[test]
    fn a_fragment_overrides_a_dependency_and_keeps_the_reason_it_is_there() {
        let output = merged(BASE, "[dependencies]\nanyhow = \"1.0.99\"\n");

        assert!(output.contains("# Error handling with context.\nanyhow = \"1.0.99\""), "{output}");
        assert!(!output.contains("anyhow = \"1\"\n"), "{output}");
    }

    #[test]
    fn a_fragment_that_comments_its_override_says_why_it_overrode() {
        let output =
            merged(BASE, "[dependencies]\n# Pinned for the TUI.\nanyhow = \"1.0.99\"\n");

        assert!(output.contains("# Pinned for the TUI.\nanyhow = \"1.0.99\""), "{output}");
        assert!(!output.contains("# Error handling with context."), "{output}");
    }

    #[test]
    fn every_comment_in_the_base_survives_a_merge() {
        let output = merged(BASE, "[dependencies]\nratatui = \"0.29\"\n");

        for comment in ["# Error handling with context.", "# Argument parsing."] {
            assert!(output.contains(comment), "{comment} was lost:\n{output}");
        }
        // The base's own layout is untouched, because it is never reserialized.
        assert!(output.starts_with("[package]\nname = \"app\"\nedition = \"2024\"\n"), "{output}");
    }

    #[test]
    fn a_fragment_can_add_a_whole_section_of_its_own() {
        let output = merged(
            BASE,
            "[dev-dependencies]\n# Snapshot assertions.\ninsta = \"1\"\n",
        );

        assert!(output.contains("[dev-dependencies]"), "{output}");
        assert!(output.contains("# Snapshot assertions.\ninsta = \"1\""), "{output}");
        // Appended after the base's sections rather than in front of them.
        let dependencies = output.find("[dependencies]").unwrap();
        let dev = output.find("[dev-dependencies]").unwrap();
        assert!(dependencies < dev, "{output}");
    }

    #[test]
    fn nested_tables_merge_rather_than_replace() {
        let base = "[dependencies.serde]\nversion = \"1\"\nfeatures = [\"derive\"]\n";
        let output = merged(base, "[dependencies.serde]\noptional = true\n");

        assert!(output.contains("version = \"1\""), "{output}");
        assert!(output.contains("features = [\"derive\"]"), "{output}");
        assert!(output.contains("optional = true"), "{output}");
    }

    #[test]
    fn every_fragment_is_merged_in_order() {
        let temp = tempfile::tempdir().unwrap();
        let base = write(temp.path(), "Cargo.toml", BASE);
        let first = write(
            temp.path(),
            "first.toml",
            "[dependencies]\n# The first one.\na = \"1\"\n\n[package]\nedition = \"2021\"\n",
        );
        let second = write(
            temp.path(),
            "second.toml",
            "[dependencies]\n# The second one.\nb = \"2\"\n\n[package]\nedition = \"2024\"\n",
        );
        let absent = temp.path().join("absent.toml");

        let output = merge_files(&base, &[first, second, absent]).unwrap();

        assert!(output.contains("# The first one.\na = \"1\""), "{output}");
        assert!(output.contains("# The second one.\nb = \"2\""), "{output}");
        // A later fragment wins, which is what makes the order meaningful.
        assert!(output.contains("edition = \"2024\""), "{output}");
        assert!(!output.contains("edition = \"2021\""), "{output}");
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn a_base_with_no_fragments_comes_back_byte_for_byte() {
        let temp = tempfile::tempdir().unwrap();
        let base = write(temp.path(), "Cargo.toml", BASE);

        assert_eq!(merge_files(&base, &[]).unwrap(), BASE);
    }

    #[test]
    fn merging_the_same_manifests_twice_produces_the_same_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let base = write(temp.path(), "Cargo.toml", BASE);
        let fragment = write(
            temp.path(),
            "fragment.toml",
            "[dev-dependencies]\ninsta = \"1\"\n\n[dependencies]\n# Terminal UI.\nratatui = \"0.29\"\n",
        );

        let fragments = std::slice::from_ref(&fragment);
        let first = merge_files(&base, fragments).unwrap();
        let second = merge_files(&base, fragments).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn a_manifest_that_is_not_toml_names_the_file_it_could_not_parse() {
        let temp = tempfile::tempdir().unwrap();
        let base = write(temp.path(), "Cargo.toml", "[package\nname =");

        let error = format!("{:#}", merge_files(&base, &[]).unwrap_err());

        assert!(error.contains("parsing base manifest"), "{error}");
        assert!(error.contains("Cargo.toml"), "{error}");
    }
}
