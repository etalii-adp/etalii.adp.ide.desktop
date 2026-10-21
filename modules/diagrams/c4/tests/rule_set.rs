//! One test per C4 rule (etalii.adp spec 030, task T061), ported from the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4RuleSet.Tests.cs` at `8b6ccfc`: the rule set is a pure
//! function, so each is a string of DSL in and a list of problems out. The last test reaches the rules through a
//! view's problems, the seam the standalone host's `C4Validator` was.

use std::fs;

use adp_diagram::{DiagramDocuments, DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity, ModuleView};
use adp_diagram_c4::model::{C4ElementKind, C4ViewKind};
use adp_diagram_c4::rules::{self, codes};
use adp_diagram_c4::view::C4DiagramView;
use adp_diagram_c4::{parser, store};

fn validate(dsl: &str) -> Vec<DiagramProblem> {
    rules::validate(&parser::parse_text(dsl))
}

fn rule_ids(dsl: &str) -> Vec<String> {
    validate(dsl).into_iter().map(|p| p.rule_id).collect()
}

/// The one problem of `dsl` with rule `rule`; fails when there are none or several.
fn single(dsl: &str, rule: &str) -> DiagramProblem {
    let matching: Vec<DiagramProblem> = validate(dsl).into_iter().filter(|p| p.rule_id == rule).collect();
    assert_eq!(matching.len(), 1, "{matching:?}");
    matching.into_iter().next().unwrap()
}

/// A workspace with everything C4 asks for, so a rule firing here is a false positive.
const CLEAN: &str = r#"workspace "Clean" {
    model {
        u = person "User" "Someone who uses the system."
        s = softwareSystem "System" "Does the thing." {
            web = container "Web App" "Serves pages." "React"
            db = container "Database" "Stores things." "PostgreSQL"
            web -> db "Reads from and writes to" "SQL/TCP"
        }
        u -> web "Visits" "HTTPS"
    }
    views {
        systemContext s "context" {
            include *
        }
        container s "containers" {
            include *
        }
    }
}"#;

#[test]
fn a_clean_model_has_no_problems() {
    assert!(validate(CLEAN).is_empty(), "{:?}", validate(CLEAN));
}

#[test]
fn an_element_without_a_description_is_reported() {
    let dsl = CLEAN.replace("person \"User\" \"Someone who uses the system.\"", "person \"User\"");
    let problem = single(&dsl, codes::MISSING_DESCRIPTION);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains("User"));
    assert_eq!(problem.location, Some(DiagramProblemLocation::Element("u".into())));
}

#[test]
fn a_container_without_a_technology_is_reported() {
    let dsl = CLEAN.replace(
        "container \"Web App\" \"Serves pages.\" \"React\"",
        "container \"Web App\" \"Serves pages.\"",
    );
    let problem = single(&dsl, codes::MISSING_TECHNOLOGY);
    assert!(problem.message.to_lowercase().contains("technology"));
}

#[test]
fn a_component_without_a_technology_is_reported() {
    let dsl = r#"workspace {
    model {
        s = softwareSystem "S" "desc" {
            c = container "C" "desc" "Kotlin" {
                comp = component "Comp" "desc"
            }
        }
    }
}"#;
    assert!(rule_ids(dsl).contains(&codes::MISSING_TECHNOLOGY.to_owned()));
}

#[test]
fn a_person_without_a_technology_is_not_reported() {
    assert!(!rule_ids(CLEAN).contains(&codes::MISSING_TECHNOLOGY.to_owned()));
}

#[test]
fn an_unlabelled_relationship_is_reported() {
    let dsl = CLEAN.replace("u -> web \"Visits\" \"HTTPS\"", "u -> web");
    let problem = single(&dsl, codes::UNLABELLED_RELATIONSHIP);
    assert!(matches!(problem.location, Some(DiagramProblemLocation::Line(_))));
}

#[test]
fn a_relationship_between_containers_with_no_protocol_is_reported() {
    let dsl = CLEAN.replace(
        "web -> db \"Reads from and writes to\" \"SQL/TCP\"",
        "web -> db \"Reads from and writes to\"",
    );
    assert!(rule_ids(&dsl).contains(&codes::MISSING_PROTOCOL.to_owned()));
}

#[test]
fn a_relationship_from_a_person_with_no_protocol_is_reported() {
    let dsl = CLEAN.replace("u -> web \"Visits\" \"HTTPS\"", "u -> web \"Visits\"");
    assert!(rule_ids(&dsl).contains(&codes::MISSING_PROTOCOL.to_owned()));
}

/// One software system, nothing else, and no view: a file someone has just started.
const LONELY: &str = r#"workspace "Lonely" {
    model {
        s = softwareSystem "System" "Does the thing."
    }
}"#;

#[test]
fn a_lonely_system_in_a_model_with_no_views_is_not_reported() {
    assert!(!rule_ids(LONELY).contains(&codes::DISCONNECTED_ELEMENT.to_owned()));
}

#[test]
fn a_lonely_system_in_a_model_with_a_view_is_reported() {
    let dsl = r#"workspace "Lonely" {
    model {
        s = softwareSystem "System" "Does the thing."
    }
    views {
        systemLandscape "landscape" {
            include *
        }
    }
}"#;
    let problem = single(dsl, codes::DISCONNECTED_ELEMENT);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains("System"));
}

#[test]
fn a_system_whose_containers_are_connected_is_not_reported_as_disconnected() {
    assert!(!rule_ids(CLEAN).contains(&codes::DISCONNECTED_ELEMENT.to_owned()));
}

/// A deployed model with nothing missing.
const DEPLOYED: &str = r#"workspace "Deployed" {
    model {
        u = person "User" "Someone who uses the system."
        s = softwareSystem "System" "Does the thing." {
            web = container "Web App" "Serves pages." "React"
        }
        u -> web "Visits" "HTTPS"

        live = deploymentEnvironment "Live" {
            server = deploymentNode "Application Server" "Runs the web app." "Ubuntu 24.04" {
                containerInstance web
            }
            lb = infrastructureNode "Load Balancer" "Spreads traffic across servers." "nginx"
            lb -> server "Forwards to" "HTTPS"
        }
    }
    views {
        systemContext s "context" {
            include *
        }
        container s "containers" {
            include *
        }
        deployment s "Live" "live" {
            include *
        }
    }
}"#;

#[test]
fn a_fully_deployed_model_has_no_problems() {
    assert!(validate(DEPLOYED).is_empty(), "{:?}", validate(DEPLOYED));
}

#[test]
fn a_deployment_node_with_no_description_is_reported() {
    let dsl = DEPLOYED.replace("\"Runs the web app.\"", "\"\"");
    let problem = single(&dsl, codes::MISSING_DEPLOYMENT_DESCRIPTION);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains("Application Server"));
}

#[test]
fn a_deployment_node_with_no_technology_is_reported() {
    let dsl = DEPLOYED.replace("\"Ubuntu 24.04\"", "\"\"");
    let problem = single(&dsl, codes::MISSING_DEPLOYMENT_TECHNOLOGY);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains("Application Server"));
}

#[test]
fn an_infrastructure_node_with_no_description_is_reported() {
    let dsl = DEPLOYED.replace("\"Spreads traffic across servers.\"", "\"\"");
    let problem = single(&dsl, codes::MISSING_INFRASTRUCTURE_DESCRIPTION);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains("Load Balancer"));
}

#[test]
fn an_infrastructure_node_with_no_technology_is_reported() {
    let dsl = DEPLOYED.replace("\"nginx\"", "\"\"");
    let problem = single(&dsl, codes::MISSING_INFRASTRUCTURE_TECHNOLOGY);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains("Load Balancer"));
}

#[test]
fn a_deployment_node_is_not_asked_to_participate_in_a_relationship() {
    let dsl = DEPLOYED.replace("lb -> server \"Forwards to\" \"HTTPS\"", "");
    assert!(!dsl.contains("lb -> server"));
    assert!(!rule_ids(&dsl).contains(&codes::DISCONNECTED_ELEMENT.to_owned()));
}

#[test]
fn an_element_no_view_draws_is_reported() {
    let dsl = r#"workspace "Undrawn" {
    model {
        u = person "User" "Someone who uses the system."
        s = softwareSystem "System" "Does the thing." {
            web = container "Web App" "Serves pages." "React"
            db = container "Database" "Stores things." "PostgreSQL"
            web -> db "Reads from and writes to" "SQL/TCP"
        }
        u -> web "Visits" "HTTPS"
    }
    views {
        systemContext s "context" {
            include *
        }
    }
}"#;
    let undrawn: Vec<DiagramProblem> = validate(dsl)
        .into_iter()
        .filter(|p| p.rule_id == codes::ELEMENT_NOT_ON_ANY_VIEW)
        .collect();
    assert_eq!(undrawn.len(), 2);
    assert!(undrawn.iter().all(|p| p.severity == DiagramProblemSeverity::Warning));
}

#[test]
fn an_element_every_level_draws_is_not_reported() {
    assert!(!rule_ids(CLEAN).contains(&codes::ELEMENT_NOT_ON_ANY_VIEW.to_owned()));
}

#[test]
fn an_undrawn_element_in_a_model_with_no_views_is_not_reported() {
    assert!(!rule_ids(LONELY).contains(&codes::ELEMENT_NOT_ON_ANY_VIEW.to_owned()));
}

#[test]
fn a_relationship_naming_something_undeclared_is_reported() {
    let dsl = CLEAN.replace("u -> web \"Visits\" \"HTTPS\"", "u -> ghost \"Visits\" \"HTTPS\"");
    let problem = single(&dsl, codes::DANGLING_RELATIONSHIP);
    assert!(problem.message.contains("ghost"));
}

#[test]
fn a_container_on_a_context_view_is_reported() {
    let dsl = r#"workspace {
    model {
        s = softwareSystem "S" "desc" {
            web = container "Web" "desc" "React"
        }
    }
    views {
        systemContext s "context" {
            include web
        }
    }
}"#;
    let problem = single(dsl, codes::KIND_NOT_PERMITTED);
    assert!(problem.message.to_lowercase().contains("container"));
    assert_eq!(problem.location, Some(DiagramProblemLocation::Element("web".into())));
}

#[test]
fn a_container_on_a_container_view_is_not_reported() {
    let dsl = r#"workspace {
    model {
        s = softwareSystem "S" "desc" {
            web = container "Web" "desc" "React"
        }
    }
    views {
        container s "containers" {
            include web
        }
    }
}"#;
    assert!(!rule_ids(dsl).contains(&codes::KIND_NOT_PERMITTED.to_owned()));
}

#[test]
fn a_dynamic_view_mixing_levels_is_not_reported_because_the_worked_example_mixes_them() {
    let dsl = r#"workspace {
    model {
        a = softwareSystem "A" "desc" {
            web = container "Web" "desc" "React"
        }
        b = softwareSystem "B" "desc"
        web -> b "Calls" "HTTPS"
        a -> b "Calls" "HTTPS"
    }
    views {
        dynamic a "scenario" {
            web -> b "Calls"
            a -> b "Calls"
        }
    }
}"#;
    assert!(!rule_ids(dsl).contains(&"c4.mixed-abstraction-levels".to_owned()));
}

#[test]
fn a_view_scoped_to_something_undeclared_is_reported() {
    let dsl = CLEAN.replace("systemContext s \"context\"", "systemContext ghost \"context\"");
    assert!(rule_ids(&dsl).contains(&codes::UNKNOWN_VIEW_SCOPE.to_owned()));
}

#[test]
fn an_empty_view_is_reported() {
    let dsl = r#"workspace {
    model {
    }
    views {
        systemLandscape "all" {
        }
    }
}"#;
    assert!(rule_ids(dsl).contains(&codes::EMPTY_VIEW.to_owned()));
}

#[test]
fn a_component_declared_outside_a_container_is_reported() {
    let dsl = r#"workspace {
    model {
        stray = component "Stray" "desc" "Kotlin"
    }
}"#;
    let problem = single(dsl, codes::MISPLACED_ELEMENT);
    assert!(problem.message.to_lowercase().contains("part of a container"));
    assert_eq!(problem.location, Some(DiagramProblemLocation::Element("stray".into())));
}

#[test]
fn a_container_inside_another_container_is_reported() {
    let dsl = r#"workspace {
    model {
        s = softwareSystem "S" "desc" {
            outer = container "Outer" "desc" "Kotlin" {
                inner = container "Inner" "desc" "Kotlin"
            }
        }
    }
}"#;
    let problem = single(dsl, codes::MISPLACED_ELEMENT);
    assert_eq!(problem.location, Some(DiagramProblemLocation::Element("inner".into())));
}

#[test]
fn properly_nested_elements_are_not_reported() {
    assert!(!rule_ids(CLEAN).contains(&codes::MISPLACED_ELEMENT.to_owned()));
}

#[test]
fn an_include_is_reported_because_what_it_declares_is_missing_from_the_diagram() {
    let dsl = r#"workspace {
    !include shared/model.dsl
    model {
        s = softwareSystem "S" "desc"
    }
}"#;
    let problem = single(dsl, codes::INCLUDE_NOT_FOLLOWED);
    assert!(problem.message.contains("shared/model.dsl"));
    assert_eq!(problem.location, None);
}

#[test]
fn a_document_with_no_includes_is_not_reported() {
    assert!(!rule_ids(CLEAN).contains(&codes::INCLUDE_NOT_FOLLOWED.to_owned()));
}

#[test]
fn every_problem_is_a_warning_so_an_unfinished_model_still_saves() {
    let dsl = r#"workspace {
    model {
        u = person "User"
        s = softwareSystem "System" {
            web = container "Web"
        }
        u -> web
    }
    views {
        systemContext s "context" {
            include *
        }
    }
}"#;
    let problems = validate(dsl);
    assert!(!problems.is_empty());
    assert!(problems.iter().all(|p| p.severity == DiagramProblemSeverity::Warning));
}

#[test]
fn every_rule_id_is_prefixed_with_the_modules_name() {
    let dsl =
        "workspace {\n  model {\n    u = person \"U\"\n  }\n  views {\n    systemLandscape \"all\" {\n    }\n  }\n}\n";
    let problems = validate(dsl);
    assert!(!problems.is_empty());
    assert!(problems.iter().all(|p| p.rule_id.starts_with("c4.")));
}

#[test]
fn permitted_kinds_match_what_each_c4_view_shows() {
    use C4ElementKind::*;
    assert_eq!(
        rules::permitted_kinds(C4ViewKind::SystemContext),
        &[Person, SoftwareSystem]
    );
    assert_eq!(
        rules::permitted_kinds(C4ViewKind::SystemLandscape),
        &[Person, SoftwareSystem]
    );
    assert!(rules::permitted_kinds(C4ViewKind::Container).contains(&Container));
    assert!(!rules::permitted_kinds(C4ViewKind::Container).contains(&Component));
    assert!(rules::permitted_kinds(C4ViewKind::Component).contains(&Component));
    assert!(rules::permitted_kinds(C4ViewKind::Deployment).contains(&DeploymentNode));
    assert!(!rules::permitted_kinds(C4ViewKind::Deployment).contains(&Person));
}

#[test]
fn the_validator_reaches_cores_seam() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("clean.dsl");
    fs::write(&body, CLEAN).unwrap();
    let mut documents = DiagramDocuments::default();
    store::get_or_load(&mut documents, &body);
    let view = C4DiagramView::new("c4/context", &body, None);
    assert!(view.problems(&documents).is_empty());
}
