//! Choice groups: the capabilities that exclude one another.
//!
//! Capabilities that cannot be combined are the same question asked once, not
//! several independent yes/no answers: a `typescript:web` project has to pick
//! a router, it does not tick routers off a list. Grouping them is what lets
//! the CLI ask that question with a `Select` and reject a `--yes` run that
//! never answered it.
//!
//! A group is a connected component of the exclusion relation, so it is
//! transitive: if A excludes B and B excludes C, all three are one question,
//! because no two of them can be chosen together. A capability that excludes
//! nothing is not in a group at all, and is an independent optional choice.

use crate::catalog::Capability;
use crate::catalog::compatibility;

/// The question asked for a group whose members share no word to name.
const GENERIC_QUESTION: &str = "Which one?";

/// A set of capabilities that exclude one another, so exactly one of them is
/// chosen.
#[derive(Debug)]
pub struct ChoiceGroup<'catalog> {
    /// The members, in the order they were given (catalog order).
    pub members: Vec<&'catalog Capability>,
}

impl ChoiceGroup<'_> {
    /// The question this group asks, built from what its members have in
    /// common: a group of TanStack frameworks asks "Which TanStack?". A group
    /// whose names share no word falls back to a generic question, because any
    /// question beats none.
    pub fn question(&self) -> String {
        match self.shared_word() {
            Some(word) => format!("Which {word}?"),
            None => GENERIC_QUESTION.to_string(),
        }
    }

    /// Whether one of `chosen` answers this group.
    pub fn is_covered_by(&self, chosen: &[&Capability]) -> bool {
        chosen
            .iter()
            .any(|one| self.members.iter().any(|member| member.key == one.key))
    }

    /// The first word every member's name contains, in the first member's own
    /// casing.
    fn shared_word(&self) -> Option<String> {
        let (first, rest) = self.members.split_first()?;
        let mut shared = words_of(&first.name);
        for member in rest {
            let theirs = words_of(&member.name);
            shared.retain(|word| theirs.iter().any(|one| one.eq_ignore_ascii_case(word)));
        }
        shared.into_iter().next()
    }
}

/// The required choices among `capabilities`: every group of two or more
/// capabilities that exclude one another, in catalog order.
pub fn choice_groups<'catalog>(
    capabilities: &[&'catalog Capability],
) -> Vec<ChoiceGroup<'catalog>> {
    components(capabilities)
        .into_iter()
        .filter(|members| members.len() > 1)
        .map(|members| ChoiceGroup { members })
        .collect()
}

/// The capabilities that exclude nothing, each of which is an independent
/// optional choice, in catalog order.
pub fn optional_capabilities<'catalog>(
    capabilities: &[&'catalog Capability],
) -> Vec<&'catalog Capability> {
    components(capabilities)
        .into_iter()
        .filter(|members| members.len() == 1)
        .map(|members| members[0])
        .collect()
}

/// Partitions `capabilities` into connected components of the exclusion
/// relation, each component in catalog order.
fn components<'catalog>(capabilities: &[&'catalog Capability]) -> Vec<Vec<&'catalog Capability>> {
    let mut grouped = vec![false; capabilities.len()];
    let mut components = Vec::new();

    for start in 0..capabilities.len() {
        if grouped[start] {
            continue;
        }
        grouped[start] = true;
        let mut members = vec![start];
        let mut next = 0;
        while next < members.len() {
            let current = capabilities[members[next]];
            next += 1;
            for (index, candidate) in capabilities.iter().enumerate() {
                if !grouped[index] && compatibility::conflict(current, candidate) {
                    grouped[index] = true;
                    members.push(index);
                }
            }
        }
        members.sort_unstable();
        components.push(members.into_iter().map(|index| capabilities[index]).collect());
    }
    components
}

/// The alphanumeric words of `name`.
fn words_of(name: &str) -> Vec<String> {
    name.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_tags;

    /// Two capabilities that name each other, plus one that names nobody.
    fn routers_and_a_loner() -> Vec<Capability> {
        let mut router = test_tags::capability("tanstack-router");
        router.name = "TanStack Router (SPA)".to_string();
        router.conflicts_with = test_tags::keys(&["tanstack-start"]);
        let mut start = test_tags::capability("tanstack-start");
        start.name = "TanStack Start (SSR)".to_string();
        start.conflicts_with = test_tags::keys(&["tanstack-router"]);
        vec![router, start, test_tags::capability("prettier")]
    }

    fn group_keys<'members>(group: &ChoiceGroup<'members>) -> Vec<&'members str> {
        group.members.iter().map(|one| one.key.as_str()).collect()
    }

    #[test]
    fn capabilities_that_exclude_each_other_are_one_group() {
        let capabilities = routers_and_a_loner();

        let groups = choice_groups(&test_tags::all(&capabilities));

        assert_eq!(groups.len(), 1);
        assert_eq!(group_keys(&groups[0]), vec!["tanstack-router", "tanstack-start"]);
    }

    #[test]
    fn a_capability_that_excludes_nothing_is_optional_rather_than_grouped() {
        let capabilities = routers_and_a_loner();

        let optional = optional_capabilities(&test_tags::all(&capabilities));

        let keys: Vec<&str> = optional.iter().map(|one| one.key.as_str()).collect();
        assert_eq!(keys, vec!["prettier"]);
    }

    #[test]
    fn exclusion_groups_transitively() {
        // A excludes B and B excludes C, so all three are the same question:
        // no two of them can be chosen together.
        let mut first = test_tags::capability("a");
        first.conflicts_with = test_tags::keys(&["b"]);
        let mut second = test_tags::capability("b");
        second.conflicts_with = test_tags::keys(&["c"]);
        let capabilities = vec![first, second, test_tags::capability("c")];

        let groups = choice_groups(&test_tags::all(&capabilities));

        assert_eq!(groups.len(), 1);
        assert_eq!(group_keys(&groups[0]), vec!["a", "b", "c"]);
        assert!(optional_capabilities(&test_tags::all(&capabilities)).is_empty());
    }

    #[test]
    fn two_independent_pairs_are_two_groups() {
        let mut router = test_tags::capability("tanstack-router");
        router.conflicts_with = test_tags::keys(&["tanstack-start"]);
        let start = test_tags::capability("tanstack-start");
        let mut postgres = test_tags::capability("postgres");
        postgres.conflicts_with = test_tags::keys(&["sqlite"]);
        let sqlite = test_tags::capability("sqlite");
        let capabilities = vec![router, start, postgres, sqlite];

        let groups = choice_groups(&test_tags::all(&capabilities));

        assert_eq!(groups.len(), 2);
        assert_eq!(group_keys(&groups[0]), vec!["tanstack-router", "tanstack-start"]);
        assert_eq!(group_keys(&groups[1]), vec!["postgres", "sqlite"]);
    }

    #[test]
    fn a_group_asks_about_what_its_members_share() {
        let capabilities = routers_and_a_loner();
        let groups = choice_groups(&test_tags::all(&capabilities));

        assert_eq!(groups[0].question(), "Which TanStack?");
    }

    #[test]
    fn a_group_that_shares_no_word_still_asks_something() {
        let mut postgres = test_tags::capability("postgres");
        postgres.name = "Postgres".to_string();
        postgres.conflicts_with = test_tags::keys(&["sqlite"]);
        let mut sqlite = test_tags::capability("sqlite");
        sqlite.name = "SQLite".to_string();
        let capabilities = vec![postgres, sqlite];

        let groups = choice_groups(&test_tags::all(&capabilities));

        assert_eq!(groups[0].question(), GENERIC_QUESTION);
    }

    #[test]
    fn a_group_is_covered_by_any_of_its_members() {
        let capabilities = routers_and_a_loner();
        let all = test_tags::all(&capabilities);
        let groups = choice_groups(&all);

        assert!(groups[0].is_covered_by(&[all[1]]));
        assert!(!groups[0].is_covered_by(&[all[2]]));
        assert!(!groups[0].is_covered_by(&[]));
    }
}
