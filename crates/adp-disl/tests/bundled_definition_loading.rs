//! Both bundled definitions load with this runtime: nothing refused and nothing warned about, every expression
//! compiled in its own context; a function either calls that the runtime lacks fails the load by name, wherever the
//! call is (the standalone host's `BundledDefinitionLoadingTests`). Beside them, every vendored `.dis` loads with the
//! diagnostics the standalone host's loader gives it, recorded in `fixtures/standalone-diagnostics.txt`.

mod common;

use adp_disl::bundling::BundledDefinition;
use adp_disl::{DislLoader, DislPluginFunctions, contexts};
use common::{BEHAVIOR_MODEL, HYPE_CYCLE, bundled, resource, text};
use serde_json::Value as Json;

#[test]
fn the_bundled_definition_loads_without_a_diagnostic() {
    for (resource, language, at_least) in [
        (HYPE_CYCLE, "net.etalii.adp.gartner.hypecycle-graph", 250),
        (BEHAVIOR_MODEL, "net.etalii.adp.etalii.agent-behavior-modelling", 80),
    ] {
        let bundled = bundled(resource);

        assert!(bundled.diagnostics.is_empty(), "{:?}", bundled.diagnostics);
        assert_eq!(language, bundled.specification.language_id());
        assert_eq!("0.3", bundled.specification.disl());
        let compiled = bundled.specification.expressions().len() + bundled.specification.functions().len();
        assert!(compiled >= at_least, "Only {compiled} expressions were compiled; the walk has lost some.");
    }
}

/// Both bundled definitions are DISL 0.3 and say nothing through the `x-` keys DISL 0.3 adopted: each is renamed to
/// its standard key, which is the only one the runtime reads. The `x-` key that still holds a construct not yet
/// ruled on holds that construct and nothing else.
#[test]
fn the_bundled_definition_is_disl03_without_the_renamed_extension_keys() {
    let renamed = [
        "x-bounds.neighbour",
        "x-enumValue.colorToken",
        "x-ruler.minUnit",
        "x-part.tooltip",
        "x-label.editText",
        "x-anchors.drawnFrom",
        "x-field.display",
        "x-field.parse",
        "x-builtIn.code",
        "x-layout.rowPacked",
        "x-layout.rowArrange",
        "x-persistence.typeMap",
        "x-abm-connectGesture",
        "x-abm-retype",
        "x-abm-place",
        "x-abm-placements",
        "x-abm-tidyTree",
        "x-menu.group",
        "x-field.id",
        "x-order",
        "x-builtIn.oncePerGroup",
    ];
    let kept_for: [(&str, &[&str]); 1] = [("x-abm-plan", &["retype", "doc"])];

    fn walk(node: &Json, pointer: &str, renamed: &[&str], kept_for: &[(&str, &[&str])], found: &mut Vec<String>) {
        match node {
            Json::Object(members) => {
                for (name, value) in members {
                    let at = format!("{pointer}/{name}");
                    if renamed.contains(&name.as_str()) {
                        found.push(at.clone());
                    }
                    if let Some((_, allowed)) = kept_for.iter().find(|(k, _)| k == name)
                        && let Some(kept) = value.as_object()
                    {
                        found.extend(
                            kept.keys()
                                .filter(|k| !allowed.contains(&k.as_str()))
                                .map(|k| format!("{at}/{k}")),
                        );
                    }
                    walk(value, &at, renamed, kept_for, found);
                }
            }
            Json::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    walk(item, &format!("{pointer}/{index}"), renamed, kept_for, found);
                }
            }
            _ => {}
        }
    }

    for resource in [HYPE_CYCLE, BEHAVIOR_MODEL] {
        let root: Json = serde_json::from_str(&text(resource)).unwrap();
        let mut found = Vec::new();
        walk(&root, "", &renamed, &kept_for, &mut found);
        if let Some(disl) = root.get("disl").and_then(Json::as_str)
            && disl != "0.3"
        {
            found.insert(0, format!("/disl is \"{disl}\""));
        }
        assert!(found.is_empty(), "{found:?}");
    }
}

/// The hype cycle graph's enumerations and user functions, as the definition declares them.
#[test]
fn the_hype_cycle_graphs_enumerations_and_functions_are_read_as_declared() {
    let specification = bundled(HYPE_CYCLE).specification;

    let phase = &specification.metamodel().enums["Phase"];
    assert_eq!("Phase", phase.name);
    assert!(phase.ordered);
    assert!(!specification.metamodel().enums["Edge"].ordered);
    let unit_months: Vec<_> = specification.functions().iter().filter(|f| f.name == "unitMonths").collect();
    assert_eq!(1, unit_months.len());
    assert_eq!("int", unit_months[0].returns);
    assert_eq!(
        vec![("u", "string")],
        unit_months[0]
            .parameters
            .iter()
            .map(|p| (p.name.as_str(), p.type_name.as_str()))
            .collect::<Vec<_>>()
    );
}

/// The behavior model's containment: a composite holds any behavior node, a leaf holds nothing.
#[test]
fn the_behavior_models_child_types_are_read_along_the_linearisation() {
    let specification = bundled(BEHAVIOR_MODEL).specification;
    let metamodel = specification.metamodel();

    assert_eq!(vec!["BehaviorNode"], metamodel.type_of("Sequence").unwrap().child_types);
    assert!(metamodel.type_of("Check").unwrap().child_types.is_empty());
}

#[test]
fn the_bundled_definition_carries_its_provenance() {
    for (resource, path) in [
        (HYPE_CYCLE, "definitions/diagrams/gartner-hype-cycle-graph.dis"),
        (BEHAVIOR_MODEL, "definitions/diagrams/agent-behavior-modelling.dis"),
    ] {
        let provenance = bundled(resource).provenance;

        assert_eq!("etalii-adp/etalii.adp", provenance.repository);
        assert_eq!(path, provenance.path);
        assert!(provenance.revision.len() == 40 && provenance.revision.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

#[test]
fn a_bundled_definition_whose_bytes_are_not_its_provenances_is_refused() {
    let (bytes, provenance) = resource(HYPE_CYCLE);
    let mut changed = bytes.to_vec();
    changed.push(b' ');

    let error = BundledDefinition::load(HYPE_CYCLE, &changed, provenance, &DislPluginFunctions::none()).unwrap_err();

    assert!(error.contains("not to the fab0bbdf"), "{error}");
}

/// The contexts the walker gives the hype cycle graph's expressions, spot-checked against DISL sections 12.3 and 8.4.
#[test]
fn an_expression_of_the_hype_cycle_graph_is_compiled_in_its_context() {
    let specification = bundled(HYPE_CYCLE).specification;
    for (pointer, context) in [
        ("/constraints/rules/0/rule", contexts::CONSTRAINT),
        ("/constraints/rules/9/rule", contexts::GESTURE_CHANGE),
        ("/constraints/rules/14/rule", contexts::GESTURE_PLACEMENT),
        ("/constraints/builtIn/std.references/message/cel", contexts::BUILT_IN_MESSAGE),
        ("/constraints/builtIn/std.endpoints/refusal/cel", contexts::BUILT_IN_REFUSAL),
        ("/toolbox/groups/0/tools/0/initial/start/cel", contexts::CREATE),
        ("/toolbox/contextMenus/0/tools/0/visible", contexts::ELEMENT),
        ("/behavior/hooks/0/actions/1/set/peakEnd", contexts::HOOK),
        ("/behavior/operations/addTrendHere/actions/0/create/attributes/start", contexts::OPERATION),
        ("/behavior/deletion/Trend/confirm/message/cel", contexts::GESTURE_DELETE),
        ("/notation/shapes/phasedBanner/handles/0/x", contexts::SHAPE),
        ("/notation/shapes/phasedBanner/handles/0/visible", contexts::ELEMENT),
        ("/notation/shapes/phasedBanner/handles/0/snap/cel", contexts::HANDLE_SNAP),
        ("/notation/shapes/phasedBanner/handles/0/write/0/set/peakEnd", contexts::HANDLE_WRITE),
        ("/notation/canvas/filters/tags/keep", contexts::FILTER),
        ("/notation/canvas/filters/tags/options/cel", contexts::FILTER_OPTIONS),
        ("/notation/nodes/Trigger/labels/0/parse/write/0/set/name", contexts::LABEL_PARSE),
        ("/forms/trend/items/3/items/0/value/cel", contexts::FORM),
    ] {
        let expression = specification.expression_at(pointer);
        assert_eq!(Some(context), expression.map(|e| e.context.as_str()), "{pointer}");
    }
}

#[test]
fn an_expression_of_the_behavior_model_is_compiled_in_its_context() {
    let specification = bundled(BEHAVIOR_MODEL).specification;
    for (pointer, context) in [
        ("/metamodel/relations/Child/derived/from", contexts::DERIVE),
        ("/metamodel/relations/Child/derived/source", contexts::DERIVE_ITEM),
        ("/persistence/ids/expression", contexts::IDENTITY),
        ("/behavior/retype/Sequence/attributeMapping/attempts", contexts::RETYPE),
        ("/toolbox/contextMenus/0/tools/5/label/cel", contexts::ELEMENT),
        ("/forms/notesDialog/items/0/initial", contexts::FORM),
    ] {
        let expression = specification.expression_at(pointer);
        assert_eq!(Some(context), expression.map(|e| e.context.as_str()), "{pointer}");
    }
}

/// A call the runtime lacks, planted in each kind of position of each definition, refuses the load and names the
/// function and where.
#[test]
fn a_call_of_a_function_the_runtime_lacks_refuses_the_definition() {
    for (resource, pointer, planted) in [
        (HYPE_CYCLE, "/constraints/rules/1/rule", "soundex(self.name) == ''"),
        (HYPE_CYCLE, "/functions/monthText/cel", "soundex(string(m))"),
        (HYPE_CYCLE, "/notation/nodes/Trigger/tooltip/cel", "soundex(self.name)"),
        (BEHAVIOR_MODEL, "/functions/shorten/cel", "soundex(s)"),
        (BEHAVIOR_MODEL, "/metamodel/relations/Child/derived/from", "soundex(diagram.nodes)"),
        (BEHAVIOR_MODEL, "/persistence/ids/expression", "soundex(self.label)"),
    ] {
        let mut root: Json = serde_json::from_str(&text(resource)).unwrap();
        *root.pointer_mut(pointer).unwrap() = Json::String(planted.to_owned());

        let result = DislLoader::load(&root.to_string());

        assert!(result.specification.is_none());
        assert!(
            result
                .errors()
                .any(|e| e.pointer == pointer && e.message.contains("'soundex'")),
            "{pointer}: {:?}",
            result.diagnostics
        );
    }
}

/// Every `.dis` the desktop host vendors - the tool types' definitions and the DISL examples - loads with the very
/// diagnostics the standalone host's loader gives it, and the two the standalone host bundles with none.
#[test]
fn every_vendored_definition_has_the_standalone_loaders_diagnostics() {
    let expected = include_str!("fixtures/standalone-diagnostics.txt").replace('\r', "");
    let mut actual = String::new();
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../vendor/etalii.adp");
    for (folder, path) in [("diagrams", "definitions/diagrams"), ("disl", "specifications/disl")] {
        let mut files: Vec<_> = std::fs::read_dir(format!("{root}/{path}"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("dis"))
            .collect();
        files.sort();
        for file in files {
            let result = DislLoader::load_bytes(&std::fs::read(&file).unwrap(), &DislPluginFunctions::none());
            actual.push_str(&format!(
                "# {folder}/{} loaded={}\n",
                file.file_name().unwrap().to_string_lossy(),
                if result.specification.is_some() { "True" } else { "False" }
            ));
            for diagnostic in &result.diagnostics {
                actual.push_str(&format!("{diagnostic}\n"));
            }
        }
    }
    for (line, (want, got)) in expected.lines().zip(actual.lines()).enumerate() {
        assert_eq!(want, got, "line {}", line + 1);
    }
    assert_eq!(expected.lines().count(), actual.lines().count());
}
