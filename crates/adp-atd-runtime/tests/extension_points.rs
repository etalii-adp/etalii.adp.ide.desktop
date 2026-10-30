//! The engines' extension points named by capability (etalii.adp spec 035, task T124): a binding whose family is a
//! reader capability reads its bodies through that capability's persistence plugin, and an edge's routing plugin is
//! drawn by the routing capability of its name, with no tool type named.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::{Capabilities, RuntimeHost, diagram_tool};
use adp_cel::{Map, Value};
use adp_diagram::DiagramDocuments;
use adp_fbl::FblElement;
use adp_fbl::model::codes;
use adp_fbl::plugins::{
    PersistencePlugin, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult, PluginTemplateRequest,
};
use adp_fbl::text::Span;
use adp_scene::scene::{RouteEnds, RoutePlugins, StandardRoutePlugins};
use adp_scene::{Point, Rect};
use serde_json::{Value as Json, json};

fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp")
}

/// A reader that reads each line of a body as a resource whose IRI is the line.
struct LinesAsResources;

impl PersistencePlugin for LinesAsResources {
    fn id(&self) -> &str {
        "reader.turtle"
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        let bytes = &request.files[0].bytes;
        let mut elements = Vec::new();
        let mut start = 0;
        for (number, line) in bytes.split(|b| *b == b'\n').enumerate() {
            if !line.is_empty() {
                let mut attributes = Map::default();
                attributes.insert("iri", Value::from(String::from_utf8_lossy(line).as_ref()));
                elements.push(FblElement {
                    id: format!("r{}", number + 1),
                    id_is_stored: false,
                    type_name: "Resource".into(),
                    rule: "resource".into(),
                    is_relation: false,
                    attributes,
                    parent_id: None,
                    parent_slot: None,
                    source: None,
                    target: None,
                    own_span: Span::new(start, start + line.len()),
                    line: number + 1,
                    file: None,
                });
            }
            start += line.len() + 1;
        }
        PluginReadResult::new(elements, Vec::new(), false)
    }

    fn plan(&self, _request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        PluginPlanResult::Refused("This reader writes nothing.".into())
    }

    fn template(&self, _request: &PluginTemplateRequest) -> Vec<u8> {
        Vec::new()
    }
}

/// ATD's example of a capability as the family, with the language id DISL needs to open its documents.
fn described() -> PathBuf {
    described_requiring("^1")
}

/// The example, requiring `reader.turtle` in `range`.
fn described_requiring(range: &str) -> PathBuf {
    let example = std::fs::read_to_string(vendor().join("specifications/atd/binding-capability.atd")).unwrap();
    let example = example.replacen("reader.turtle@^1", &format!("reader.turtle@{range}"), 1);
    let described = example.replacen(
        "  label \"Resource graph\"\n",
        "  label \"Resource graph\"\n  id: \"example.resourceGraph\"\n",
        1,
    );
    assert_ne!(described, example);
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let next = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let folder = std::env::temp_dir().join(format!(
        "adp-atd-runtime-extension-points-{}-{next}",
        std::process::id()
    ));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("resource-graph.atd");
    std::fs::write(&path, described).unwrap();
    path
}

fn opened(capabilities: Capabilities) -> adp_fbl::FblModel {
    let checked = check(&FsFiles, &described());
    let symbols = checked.symbols.as_ref().expect("symbols");
    let host = RuntimeHost {
        capabilities,
        ..RuntimeHost::default()
    };
    let runtime = diagram_tool(&checked.description, symbols, &host).unwrap();
    assert_eq!(runtime.tool.extensions, [".ttl"]);
    let mut documents = DiagramDocuments::default();
    let document = documents.open_bytes(
        runtime.tool.clone(),
        Path::new("/nowhere/graph.ttl"),
        None,
        b"ex:a\nex:b\n".to_vec(),
        None,
    );
    document.fbl().clone()
}

#[test]
fn a_capability_as_the_family_lowers_to_fbls_plugin_reader() {
    let checked = check(&FsFiles, &vendor().join("specifications/atd/binding-capability.atd"));
    let members: Vec<&Json> = checked.description.members().collect();
    let lowered = adp_atd_runtime::lowering::fbl::lower(&members).expect("a binding");
    let binding = &lowered["bindings"]["turtle"];
    assert_eq!(binding["reader"], json!({ "plugin": "reader.turtle" }));
    assert_eq!(binding["body"], json!({ "kind": "file" }));
}

#[test]
fn a_binding_reads_its_body_through_the_reader_capability_it_names() {
    let model = opened(Capabilities::desktop().with_reader("reader.turtle", Rc::new(LinesAsResources)));
    assert!(!model.unreadable, "{:?}", model.findings);
    let iris: Vec<String> = model
        .elements
        .iter()
        .map(|e| {
            e.attributes
                .get("iri")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    assert_eq!(iris, ["ex:a", "ex:b"]);
}

#[test]
fn a_reader_capability_the_host_lacks_opens_the_body_with_a_finding() {
    let model = opened(Capabilities::desktop());
    assert!(model.elements.is_empty());
    assert!(
        model.findings.iter().any(|f| f.code == codes::PLUGIN_MISSING),
        "{:?}",
        model.findings
    );
}

#[test]
fn a_routing_plugin_is_drawn_by_the_routing_capability_of_its_name() {
    let ends = RouteEnds {
        source: Rect::new(0.0, 0.0, 100.0, 40.0),
        target: Rect::new(300.0, 80.0, 100.0, 40.0),
    };
    let origin = Point::new(0.0, 0.0);
    let capabilities = Capabilities::desktop();
    let by_capability = capabilities.route("routing.forwardBezier", origin, origin, &ends);
    assert!(by_capability.is_some());
    assert_eq!(
        by_capability,
        StandardRoutePlugins.route("routing.forwardBezier", origin, origin, &ends)
    );
    assert_eq!(
        by_capability,
        StandardRoutePlugins.route("net.etalii.adp.generic.dependencyCurve", origin, origin, &ends)
    );
    assert!(
        capabilities
            .route("routing.noSuchThing", origin, origin, &ends)
            .is_none()
    );
}

/// The `ATD9010` findings of the example requiring `reader.turtle` in `range`, on a host with `capabilities`.
fn lacking(range: &str, capabilities: Capabilities) -> Vec<String> {
    let checked = check(&FsFiles, &described_requiring(range));
    let symbols = checked.symbols.as_ref().expect("symbols");
    let host = RuntimeHost {
        capabilities,
        ..RuntimeHost::default()
    };
    let runtime = diagram_tool(&checked.description, symbols, &host).expect("the description still opens");
    runtime
        .findings
        .iter()
        .map(|f| format!("{} {}:{} {}", f.code, f.line, f.column, f.message))
        .collect()
}

#[test]
fn a_required_capability_the_host_lacks_is_one_finding_and_the_description_still_opens() {
    let found = lacking("^1", Capabilities::desktop());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("ATD9010 "), "{found:?}");
    assert!(
        found[0].contains("does not provide the capability `reader.turtle`"),
        "{found:?}"
    );

    let reader = || Capabilities::desktop().with_reader("reader.turtle", Rc::new(LinesAsResources));
    assert!(lacking("^1", reader()).is_empty());
    let found = lacking("^2", reader());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("at 1.0.0, which `^2` does not allow"), "{found:?}");
}

#[test]
fn a_range_is_read_as_npms_semver_ranges() {
    use adp_atd_runtime::Version;
    let v = Version(1, 2, 3);
    for (range, holds) in [
        ("^1", true),
        ("^1.2", true),
        ("^1.3", false),
        ("^2", false),
        ("~1.2", true),
        ("~1.1", false),
        ("~1", true),
        (">=1.0", true),
        (">=1.3", false),
        (">1.1", true),
        (">1.2", false),
        (">1.2.3", false),
        ("<2", true),
        ("<1.2.3", false),
        ("<=1.2", true),
        ("<=1.1", false),
        ("1.2.3", true),
        ("=1.2", true),
        ("1.3", false),
        (">=1.0 <2", true),
        (">=1.0 <1.2", false),
        ("", false),
        ("^x", false),
    ] {
        assert_eq!(v.satisfies(range), holds, "{range}");
    }
    assert!(Version(0, 2, 5).satisfies("^0.2"));
    assert!(!Version(0, 3, 0).satisfies("^0.2"));
    assert!(!Version(0, 0, 4).satisfies("^0.0.3"));
}
