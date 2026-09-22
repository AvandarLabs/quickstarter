//! The `greet` command: both audiences in one place.
//!
//! `--name` makes it fully non-interactive, and omitting it on a terminal asks
//! rather than failing. The greeting itself is a pure function, which is the
//! part worth testing.

use std::io::{IsTerminal, Write};

use anyhow::{Context, Result, bail};

use crate::theme::Theme;

/// Builds the greeting. An empty or blank name greets the world instead.
#[must_use]
pub fn greeting(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        "Hello, world!".to_owned()
    } else {
        format!("Hello, {name}!")
    }
}

/// Prints the greeting on stdout, asking for the name when a person left it
/// out and there is a terminal to ask on.
pub fn run(name: Option<&str>, theme: Theme) -> Result<()> {
    let name = match name {
        Some(given) => given.to_owned(),
        None => ask_for_name(theme)?,
    };
    println!("{}", greeting(&name));
    Ok(())
}

/// Asks who to greet. Without a terminal there is nobody to ask, so the
/// failure names the flag that answers the question instead.
fn ask_for_name(theme: Theme) -> Result<String> {
    if !std::io::stdin().is_terminal() {
        bail!("no name to greet: pass --name <NAME> (stdin is not a terminal, so I cannot ask)");
    }
    eprint!("{} ", theme.prompt("Who should I greet?"));
    std::io::stderr().flush().context("writing the prompt")?;
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("reading the answer")?;
    Ok(answer.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_greeting_names_the_person() {
        assert_eq!(greeting("Ada"), "Hello, Ada!");
    }

    #[test]
    fn surrounding_space_is_not_part_of_the_name() {
        assert_eq!(greeting("  Ada \n"), "Hello, Ada!");
    }

    #[test]
    fn an_empty_name_greets_the_world() {
        assert_eq!(greeting("   "), "Hello, world!");
    }
}
