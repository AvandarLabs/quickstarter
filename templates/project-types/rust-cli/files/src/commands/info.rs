//! The `info` command: what this build is, as data.
//!
//! Stdout is the machine channel, so the output is plain `key=value` lines: no
//! color, no table drawing, nothing a caller has to strip before parsing.

use anyhow::Result;

use crate::APP_NAME;

/// The facts, in a stable order. Pure, so the shape of the output is testable.
#[must_use]
pub fn facts() -> Vec<(&'static str, &'static str)> {
    vec![
        ("name", APP_NAME),
        ("package", env!("CARGO_PKG_NAME")),
        ("version", env!("CARGO_PKG_VERSION")),
    ]
}

/// Prints one `key=value` line per fact.
pub fn run() -> Result<()> {
    for (key, value) in facts() {
        println!("{key}={value}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_facts_lead_with_the_name_and_carry_a_version() {
        let facts = facts();
        assert_eq!(facts.first().map(|(key, _)| *key), Some("name"));
        assert!(
            facts
                .iter()
                .any(|(key, value)| *key == "version" && !value.is_empty())
        );
    }

    #[test]
    fn no_fact_is_blank() {
        for (key, value) in facts() {
            assert!(
                !key.is_empty() && !value.is_empty(),
                "{key} has nothing to say"
            );
        }
    }
}
