//! Deep-merge of `package.json` fragments.
//!
//! The base layer ships a `package.json` with the dependencies every project
//! shares. Each stack module ships a small `package.json` fragment with only
//! the dependencies (and any scripts) that its stack adds. Merging them keeps
//! shared dependencies defined once, in the base.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Map, Value};

/// Reads the base and fragment manifests from disk, deep-merges the fragment
/// onto the base, and returns the merged manifest serialized as pretty JSON
/// with a trailing newline. A missing fragment is treated as an empty object,
/// so a module without extra dependencies needs no `package.json` at all.
pub fn merge_files(base_path: &Path, fragment_path: &Path) -> Result<String> {
    let base_text = std::fs::read_to_string(base_path)
        .with_context(|| format!("reading base manifest {}", base_path.display()))?;
    let mut base: Value = serde_json::from_str(&base_text)
        .with_context(|| format!("parsing base manifest {}", base_path.display()))?;

    if fragment_path.exists() {
        let fragment_text = std::fs::read_to_string(fragment_path)
            .with_context(|| format!("reading fragment {}", fragment_path.display()))?;
        let fragment: Value = serde_json::from_str(&fragment_text)
            .with_context(|| format!("parsing fragment {}", fragment_path.display()))?;
        deep_merge(&mut base, &fragment);
    }

    let mut serialized = serde_json::to_string_pretty(&base)?;
    serialized.push('\n');
    Ok(serialized)
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
    fn fragment_adds_new_top_level_keys() {
        let mut base = json!({ "name": "app" });
        let fragment = json!({ "scripts": { "start": "node server.mjs" } });

        deep_merge(&mut base, &fragment);

        assert_eq!(base["scripts"]["start"], "node server.mjs");
        assert_eq!(base["name"], "app");
    }
}
