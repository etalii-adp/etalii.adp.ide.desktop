//! The table of which C4 rule mirrors one of Structurizr's (etalii.adp spec 030, task T061), ported from the
//! standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4StructurizrMirror.Tests.cs` at `8b6ccfc`:
//! it accounts for every rule id, its halves are a partition, it names no rule that does not exist, and every reason
//! it gives is one.

use adp_diagram_c4::mirror::{
    ADP_ONLY_RULES, MIRRORED_RULES, NOT_IMPLEMENTED_RULES, is_adp_only, mirrored, mirrored_structurizr_rules,
};
use adp_diagram_c4::rules::codes;

#[test]
fn every_rule_is_either_mirrored_or_recorded_as_adps_own() {
    let undecided: Vec<&str> = codes::ALL
        .into_iter()
        .filter(|rule| mirrored(rule).is_none() && !is_adp_only(rule))
        .collect();
    assert!(undecided.is_empty(), "{undecided:?}");
}

#[test]
fn no_rule_is_both_mirrored_and_recorded_as_adps_own() {
    let both: Vec<&str> = MIRRORED_RULES
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| is_adp_only(id))
        .collect();
    assert!(both.is_empty(), "{both:?}");
}

#[test]
fn the_table_describes_no_rule_that_does_not_exist() {
    let strangers: Vec<&str> = MIRRORED_RULES
        .iter()
        .map(|(id, _)| *id)
        .chain(ADP_ONLY_RULES.iter().map(|(id, _)| *id))
        .filter(|id| !codes::ALL.contains(id))
        .collect();
    assert!(strangers.is_empty(), "{strangers:?}");
}

#[test]
fn every_mirrored_rule_names_at_least_one_structurizr_rule() {
    assert!(MIRRORED_RULES.iter().all(|(_, rules)| !rules.is_empty()));
}

#[test]
fn every_reason_given_is_a_reason() {
    assert!(ADP_ONLY_RULES.iter().all(|(_, reason)| !reason.is_empty()));
    assert!(NOT_IMPLEMENTED_RULES.iter().all(|(_, reason)| !reason.is_empty()));
}

#[test]
fn no_structurizr_rule_is_both_mirrored_and_recorded_as_not_implemented() {
    let both: Vec<&str> = mirrored_structurizr_rules()
        .into_iter()
        .filter(|rule| NOT_IMPLEMENTED_RULES.iter().any(|(id, _)| id == rule))
        .collect();
    assert!(both.is_empty(), "{both:?}");
}

#[test]
fn the_flattened_set_matches_the_table_it_is_built_from() {
    let mut expected: Vec<&str> = MIRRORED_RULES
        .iter()
        .flat_map(|(_, rules)| rules.iter().copied())
        .collect();
    expected.sort_unstable();
    expected.dedup();
    assert_eq!(mirrored_structurizr_rules(), expected);
}

#[test]
fn the_six_rules_this_spec_added_are_all_mirrored() {
    for rule in [
        codes::MISSING_DEPLOYMENT_DESCRIPTION,
        codes::MISSING_DEPLOYMENT_TECHNOLOGY,
        codes::MISSING_INFRASTRUCTURE_DESCRIPTION,
        codes::MISSING_INFRASTRUCTURE_TECHNOLOGY,
        codes::DISCONNECTED_ELEMENT,
        codes::ELEMENT_NOT_ON_ANY_VIEW,
    ] {
        assert!(
            mirrored(rule).is_some(),
            "'{rule}' closes a gap against Structurizr and must say which rule it mirrors."
        );
    }
}
