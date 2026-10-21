//! What to call a newly added element, read off the elements already beside it (etalii.adp spec 030, FR-018: Tab
//! and Enter for child and sibling); ported from the standalone host's `EtAlii.Adp.Diagram/SiblingNaming.cs` at
//! `8b6ccfc`.
//!
//! A new element is named from its siblings rather than by asking first, so it can be created at once and renamed in
//! place. The rule follows the siblings, in order, the first that applies winning: numbered siblings (`Phase 1`,
//! `Phase 2`) give one past the highest number of the stem most of them share; at least two siblings sharing a last
//! word (`Measure pass`, `Arrange pass`) give `New pass`; otherwise the fallback. Every branch ends in the same
//! uniqueness pass, and case is compared ignoring it while the highest numbered sibling's spelling is kept.

/// The name for a new element among `siblings` (order is irrelevant), `fallback` when they show no pattern.
pub fn next_name<'a>(siblings: impl IntoIterator<Item = &'a str>, fallback: &str) -> String {
    let taken: Vec<&str> = siblings
        .into_iter()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect();
    let name = numbered_continuation(&taken)
        .or_else(|| shared_last_word(&taken))
        .unwrap_or_else(|| fallback.to_owned());
    make_unique(name, &taken)
}

/// A name ending in a number of at most fifteen digits, split into the stem before it and the number: the
/// standalone host's `^(?<stem>.*\S)\s+(?<number>\d{1,15})$`.
fn numbered(name: &str) -> Option<(&str, u64)> {
    let digits_start = name.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    let digits = &name[digits_start..];
    if digits.is_empty() || digits.len() > 15 {
        return None;
    }
    let before = &name[..digits_start];
    let stem = before.trim_end();
    if stem.len() == before.len() || stem.is_empty() {
        return None;
    }
    Some((stem, digits.parse().ok()?))
}

fn numbered_continuation(taken: &[&str]) -> Option<String> {
    let numbered: Vec<(&str, u64)> = taken.iter().filter_map(|name| numbered(name)).collect();
    // The stem the most siblings agree on, so one stray "Draft 2" among five "Phase n" does not decide the answer;
    // ties break on the highest number, the most recently added of the tied stems.
    let mut groups: Vec<(String, Vec<(&str, u64)>)> = Vec::new();
    for entry in numbered {
        let key = entry.0.to_lowercase();
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, members)) => members.push(entry),
            None => groups.push((key, vec![entry])),
        }
    }
    let (_, winner) = groups.into_iter().max_by(|(_, a), (_, b)| {
        a.len()
            .cmp(&b.len())
            .then(highest(a).cmp(&highest(b)))
            .then(std::cmp::Ordering::Greater)
    })?;
    let (stem, number) = winner
        .iter()
        .copied()
        .reduce(|best, entry| if entry.1 > best.1 { entry } else { best })?;
    Some(format!("{stem} {}", number + 1))
}

fn highest(entries: &[(&str, u64)]) -> u64 {
    entries.iter().map(|(_, n)| *n).max().unwrap_or_default()
}

fn shared_last_word(taken: &[&str]) -> Option<String> {
    let mut groups: Vec<(String, &str, usize)> = Vec::new();
    for name in taken {
        let words: Vec<&str> = name.split(' ').filter(|w| !w.is_empty()).collect();
        if words.len() < 2 {
            continue;
        }
        let word = words[words.len() - 1];
        let key = word.to_lowercase();
        match groups.iter_mut().find(|(k, _, _)| *k == key) {
            Some((_, _, count)) => *count += 1,
            None => groups.push((key, word, 1)),
        }
    }
    let mut best: Option<(&str, usize)> = None;
    for (_, word, count) in groups {
        if count >= 2 && best.is_none_or(|(_, c)| count > c) {
            best = Some((word, count));
        }
    }
    best.map(|(word, _)| format!("New {word}"))
}

fn make_unique(name: String, taken: &[&str]) -> String {
    let holds = |candidate: &str| taken.iter().any(|t| t.to_lowercase() == candidate.to_lowercase());
    if !holds(&name) {
        return name;
    }
    (2..)
        .map(|suffix| format!("{name} {suffix}"))
        .find(|candidate| !holds(candidate))
        .expect("some suffix is free")
}

#[cfg(test)]
mod tests {
    use super::next_name;

    fn next(siblings: &[&str]) -> String {
        next_name(siblings.iter().copied(), "Node")
    }

    #[test]
    fn continues_a_numbered_run() {
        assert_eq!(next(&["Phase 1", "Phase 2"]), "Phase 3");
    }

    #[test]
    fn counts_from_the_highest_rather_than_the_count() {
        assert_eq!(next(&["Phase 1", "Phase 3"]), "Phase 4");
    }

    #[test]
    fn takes_the_stem_most_siblings_agree_on() {
        assert_eq!(next(&["Step 1", "Step 2", "Step 3", "Step 4", "Draft 9"]), "Step 5");
    }

    #[test]
    fn follows_a_shared_last_word() {
        assert_eq!(next(&["Measure pass", "Arrange pass"]), "New pass");
    }

    #[test]
    fn needs_two_siblings_to_call_something_a_pattern() {
        assert_eq!(next(&["Measure pass"]), "Node");
    }

    #[test]
    fn falls_back_when_the_siblings_show_no_pattern_at_all() {
        assert_eq!(next(&["Scene graph", "Layout", "Rendering pipeline"]), "Node");
    }

    #[test]
    fn names_the_first_child_of_a_childless_node() {
        assert_eq!(next(&[]), "Node");
    }

    #[test]
    fn never_collides_with_a_sibling_that_already_has_the_name() {
        assert_eq!(next(&["Scene graph", "Node"]), "Node 2");
        assert_eq!(next(&["Scene graph", "Node", "Node 2"]), "Node 3");
    }

    #[test]
    fn does_not_collide_on_a_derived_name() {
        assert_eq!(next(&["Measure pass", "Arrange pass", "New pass"]), "New pass 2");
    }

    #[test]
    fn treats_case_as_the_same_stem_and_keeps_the_authors_spelling() {
        assert_eq!(next(&["Phase 1", "phase 2"]), "phase 3");
    }

    #[test]
    fn ignores_blank_siblings() {
        assert_eq!(next(&["Phase 1", "Phase 2", "", "   "]), "Phase 3");
    }

    #[test]
    fn survives_a_number_no_integer_could_hold() {
        let siblings = ["Build 99999999999999999999999999999999999999"];
        let name = next(&siblings);
        assert!(!name.trim().is_empty());
        assert!(!siblings.contains(&name.as_str()));
    }

    #[test]
    fn takes_the_callers_fallback_when_it_has_one() {
        assert_eq!(next_name(["Discovery", "Build"], "New element"), "New element");
    }
}
