//! Finding the tag declarations a template repository ships.
//!
//! Project types and capabilities are both stored the same way: one directory
//! per tag, holding the JSON file that declares it. Both are therefore found
//! the same way, and the difference between them (a repository must ship a
//! project type, but need ship no capability) is left to the caller.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The subdirectories of `root` that hold a `file_name`, in directory-name
/// order.
///
/// A missing `root` yields no directories rather than an error: a template
/// repository that ships no capabilities at all is a valid one, and the caller
/// that does need a tag says so with its own message.
pub fn declaring_dirs(root: &Path, file_name: &str) -> Result<Vec<PathBuf>> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut dirs = Vec::new();
    for entry in
        std::fs::read_dir(root).with_context(|| format!("reading {}", root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        if entry.path().join(file_name).exists() {
            dirs.push(entry.path());
        }
    }

    dirs.sort();
    Ok(dirs)
}

/// The directory name of `dir`, which is a tag's slug.
pub fn slug_of(dir: &Path) -> String {
    dir.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_declaration(root: &Path, slug: &str, file_name: &str) {
        let dir = root.join(slug);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(file_name), "{}").unwrap();
    }

    #[test]
    fn finds_only_the_directories_that_declare_a_tag() {
        let temp = tempfile::tempdir().unwrap();
        write_declaration(temp.path(), "one", "tag.json");
        std::fs::create_dir_all(temp.path().join("not-a-tag")).unwrap();
        std::fs::write(temp.path().join("loose.json"), "{}").unwrap();

        let dirs = declaring_dirs(temp.path(), "tag.json").unwrap();

        assert_eq!(dirs.len(), 1);
        assert_eq!(slug_of(&dirs[0]), "one");
    }

    #[test]
    fn a_missing_directory_declares_nothing_and_is_not_an_error() {
        let temp = tempfile::tempdir().unwrap();
        assert!(declaring_dirs(&temp.path().join("absent"), "tag.json").unwrap().is_empty());
    }
}
