//! Deep-merge of `package.json` fragments.
//!
//! A project type ships the `package.json` with the dependencies every project
//! of that type shares: the merge base. Each capability ships a small fragment
//! with only the dependencies (and any scripts) it adds. Merging them keeps a
//! shared dependency defined once, in the project type.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{Map, Value};

/// Reads the base manifest and deep-merges each fragment onto it in order,
/// returning the result serialized as pretty JSON with a trailing newline.
///
/// A fragment that does not exist is treated as an empty object, so a
/// capability that adds no dependency needs no `package.json` at all. Whether
/// there is a base manifest to merge onto is the caller's question: a project
/// type whose language has none composes without one.
pub fn merge_files(base_path: &Path, fragment_paths: &[PathBuf]) -> Result<String> {
    let base_text = std::fs::read_to_string(base_path)
        .with_context(|| format!("reading base manifest {}", base_path.display()))?;
    let mut base: Value = serde_json::from_str(&base_text)
        .with_context(|| format!("parsing base manifest {}", base_path.display()))?;

    for fragment_path in fragment_paths {
        if let Some(fragment) = read_fragment(fragment_path)? {
            deep_merge(&mut base, &fragment);
        }
    }

    let mut serialized = serde_json::to_string_pretty(&base)?;
    serialized.push('\n');
    Ok(serialized)
}

/// Reads one fragment, or `None` when the layer ships no manifest.
fn read_fragment(fragment_path: &Path) -> Result<Option<Value>> {
    if !fragment_path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(fragment_path)
        .with_context(|| format!("reading fragment {}", fragment_path.display()))?;
    let fragment = serde_json::from_str(&text)
        .with_context(|| format!("parsing fragment {}", fragment_path.display()))?;
    Ok(Some(fragment))
}

/// Recursively merges `overlay` into `target`. Two JSON objects are merged key
/// by key (so `dependencies` blocks combine rather than replace). Any other
/// value in `overlay` replaces the value in `target`, which lets a module
/// override a scalar such as a script command.
pub fn deep_merge(target: &mut Value, overlay: &Value) {
    match (target, overlay) {
        (Value::Object(target_map), Value::Object(overlay_map)) => {
            merge_objects(target_map, overlay_map);
        }
        (target_slot, overlay_value) => {
            *target_slot = overlay_value.clone();
        }
    }
}

fn merge_objects(target_map: &mut Map<String, Value>, overlay_map: &Map<String, Value>) {
    for (key, overlay_value) in overlay_map {
        match target_map.get_mut(key) {
            Some(target_value) => deep_merge(target_value, overlay_value),
            None => {
                target_map.insert(key.clone(), overlay_value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merges_dependency_blocks_instead_of_replacing() {
        let mut base = json!({
            "dependencies": { "react": "^19.0.0", "clsx": "^2.1.1" }
        });
        let fragment = json!({
            "dependencies": { "@tanstack/react-router": "^1.170.0" }
        });

        deep_merge(&mut base, &fragment);

        assert_eq!(base["dependencies"]["react"], "^19.0.0");
        assert_eq!(base["dependencies"]["clsx"], "^2.1.1");
        assert_eq!(base["dependencies"]["@tanstack/react-router"], "^1.170.0");
    }

    #[test]
    fn fragment_scalar_overrides_base_scalar() {
        let mut base = json!({ "scripts": { "build": "vite build" } });
        let fragment = json!({ "scripts": { "build": "tsc -b && vite build" } });

        deep_merge(&mut base, &fragment);

        assert_eq!(base["scripts"]["build"], "tsc -b && vite build");
    }

    #[test]
    fn every_fragment_is_merged_in_order() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("base.json");
        std::fs::write(&base, r#"{ "dependencies": { "react": "^19.0.0" } }"#).unwrap();
        let first = temp.path().join("first.json");
        std::fs::write(&first, r#"{ "dependencies": { "a": "1" }, "type": "module" }"#).unwrap();
        let second = temp.path().join("second.json");
        std::fs::write(&second, r#"{ "dependencies": { "b": "2" }, "type": "commonjs" }"#).unwrap();
        let absent = temp.path().join("absent.json");

        let merged =
            merge_files(&base, &[first, second, absent]).unwrap();

        let parsed: Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(parsed["dependencies"]["react"], "^19.0.0");
        assert_eq!(parsed["dependencies"]["a"], "1");
        assert_eq!(parsed["dependencies"]["b"], "2");
        // A later fragment wins, which is what makes the order meaningful.
        assert_eq!(parsed["type"], "commonjs");
        assert!(merged.ends_with("\n"));
    }

    #[test]
    fn a_base_with_no_fragments_is_returned_as_it_is() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("base.json");
        std::fs::write(&base, r#"{ "name": "app" }"#).unwrap();

        let merged = merge_files(&base, &[]).unwrap();

        let parsed: Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(parsed["name"], "app");
    }

    #[test]
    fn fragment_adds_new_top_level_keys() {
        let mut base = json!({ "name": "app" });
        let fragment = json!({ "scripts": { "start": "node server.mjs" } });

        deep_merge(&mut base, &fragment);

        assert_eq!(base["scripts"]["start"], "node server.mjs");
        assert_eq!(base["name"], "app");
    }
}
