//! Recursive, symlink-aware file overlay.
//!
//! Composing a project means copying the base layer's files into the target
//! directory and then copying the chosen module's files on top. Later layers
//! win on conflict, which is how a module owns a whole file (its
//! `vite.config.ts`) that differs from the base. Symlinks are recreated as
//! symlinks (not dereferenced) so the `CLAUDE.md -> AGENTS.md` link survives.

use std::path::Path;

use anyhow::{Context, Result};

/// Copies every entry under `source` into `dest`, creating `dest` and any
/// intermediate directories as needed. Existing files in `dest` are
/// overwritten, giving later layers precedence.
pub fn copy_tree(source: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)
        .with_context(|| format!("creating directory {}", dest.display()))?;

    for entry in
        std::fs::read_dir(source).with_context(|| format!("reading dir {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = std::fs::symlink_metadata(&source_path)?.file_type();

        if file_type.is_symlink() {
            copy_symlink(&source_path, &dest_path)?;
        } else if file_type.is_dir() {
            copy_tree(&source_path, &dest_path)?;
        } else {
            copy_file(&source_path, &dest_path)?;
        }
    }
    Ok(())
}

fn copy_file(source_path: &Path, dest_path: &Path) -> Result<()> {
    std::fs::copy(source_path, dest_path).with_context(|| {
        format!("copying {} to {}", source_path.display(), dest_path.display())
    })?;
    Ok(())
}

/// Recreates a symlink at `dest_path` pointing at the same target as the
/// source link, replacing any existing entry.
fn copy_symlink(source_path: &Path, dest_path: &Path) -> Result<()> {
    let link_target = std::fs::read_link(source_path)
        .with_context(|| format!("reading symlink {}", source_path.display()))?;
    if dest_path.exists() || std::fs::symlink_metadata(dest_path).is_ok() {
        std::fs::remove_file(dest_path).ok();
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&link_target, dest_path).with_context(|| {
        format!("linking {} -> {}", dest_path.display(), link_target.display())
    })?;
    #[cfg(not(unix))]
    anyhow::bail!("symlinks in templates are only supported on Unix");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlays_later_layer_over_earlier() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("base");
        let module = temp.path().join("module");
        let dest = temp.path().join("dest");
        std::fs::create_dir_all(base.join("src")).unwrap();
        std::fs::create_dir_all(&module).unwrap();
        std::fs::write(base.join("shared.txt"), "from base").unwrap();
        std::fs::write(base.join("src/theme.ts"), "theme").unwrap();
        std::fs::write(module.join("shared.txt"), "from module").unwrap();

        copy_tree(&base, &dest).unwrap();
        copy_tree(&module, &dest).unwrap();

        assert_eq!(
            std::fs::read_to_string(dest.join("shared.txt")).unwrap(),
            "from module"
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("src/theme.ts")).unwrap(),
            "theme"
        );
    }

    #[cfg(unix)]
    #[test]
    fn preserves_symlinks_as_links() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("base");
        let dest = temp.path().join("dest");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("AGENTS.md"), "rules").unwrap();
        std::os::unix::fs::symlink("AGENTS.md", base.join("CLAUDE.md")).unwrap();

        copy_tree(&base, &dest).unwrap();

        let link = dest.join("CLAUDE.md");
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read_link(&link).unwrap().to_str().unwrap(), "AGENTS.md");
    }
}
