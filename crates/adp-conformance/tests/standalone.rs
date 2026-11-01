//! The standalone host's conformance harness, run against the Rust port of its FBL library (etalii.adp spec 030,
//! FR-034, SC-003): `ConformanceFixtures.Tests.cs`, `InsertEmptyFixtures.Tests.cs`, `KnowledgeFixtures.Tests.cs` and
//! `KnowledgeFixtures.Steps.Tests.cs` of `EtAlii.Adp.Specification.Fbl.Tests` at 8b6ccfc, over the fixtures that
//! host vendored, which `vendor/etalii.adp` holds byte for byte the same. For every fixture, reading the input gives
//! what `read` lists, and every step plans exactly its splices and its document, or refuses and writes nothing.
//! Splices are compared in body order, those at one offset as listed: FBL gives an order only to splices at the
//! same offset (section 6.5), and a fixture written since 0.3 lists a cascaded removal bottom-up, as it is planned,
//! where the library lists every edit in body order. The two say the same edit.
//!
//! What FBL leaves to the host or to a tool type's specification is supplied here, as the standalone host's test
//! support does: the DISL id derivations ([`natural_id`], [`knowledge_id`]), the attribute types read from
//! `timeline.dis` and `knowledge.des`, the attributes typed as a date-time, and DISL's default deletion policy,
//! which clears the references to a removed element, and of the Wardley map, whose specification reads its body by a
//! plugin yet: the types of its attributes ([`wardley_type`], from `wardley-map.dis`), that decorators are many, and its
//! random ids, by which a fixture names each element at its place ([`named_ids`]). None of them is an edit to a fixture.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;

use adp_cel::{Map, Value};
use adp_fbl::documents::{self, FblBinding, ProblemSeverity};
use adp_fbl::{
    BodyReading, FblModel, FblOptions, IdRequest, ModelChange, OpenBody, OpenDocument, Operation, PlanResult,
    PlannedEdit, Setting, SettingPosition, Splice, UndoResult,
};
use serde_json::Value as Json;

#[path = "standalone/hierarchical_requirement.rs"]
mod hierarchical_requirement;

/// The fixtures the standalone host vendored at 8b6ccfc under `Conformance/fixtures`, which its
/// `ConformanceFixturesTests` runs, and those the standalone host runs from `a7fdf03` that this port has the features
/// of: `sankey-written-text` and `supply-chain-written-text`, which etalii.adp added with the Sankey and supply chain
/// bindings, and the bullets family's (FBL 0.7 section 4.8, etalii.adp spec 036), with `behavior-tree-read/first-add`,
/// which etalii.adp holds and the standalone host does not vendor.
const STANDALONE_FIXTURES: [&str; 33] = [
    "behavior-tree-read/first-add",
    "insert-constant-json-read",
    "insert-constant-yaml-read",
    "bullets-edits",
    "bullets-edits/no-section",
    "bullets-read",
    "sankey-written-text",
    "supply-chain-written-text",
    "agent-activity-edits",
    "agent-activity-new",
    "agent-activity-read",
    "causal-loop-rename",
    "causal-loop-flow-default",
    "causal-loop-flow-legacy/findings",
    "causal-loop-flow-legacy/on-call",
    "causal-loop-flow-legacy/reference",
    "causal-loop-flow-older",
    "causal-loop-flow-range",
    "causal-loop-flow-read",
    "causal-loop-flow-set",
    "causal-loop-flow-set/lf",
    "causal-loop-flow-unknown",
    "databricks-job-dependencies",
    "databricks-pipeline-json",
    "dependency-graph-empty-flow-sequence",
    "insert-empty-json",
    "insert-empty-yaml",
    "mindmap-nodes",
    "registration-blocks",
    "registration-layout",
    "timeline-bom-lf",
    "timeline-edits",
    "timeline-written-text",
];

/// The fixtures of the `grammar` family (FBL 0.9 section 4.9), which no host but this one is required to read: run
/// as the standalone host's are, by [`the_grammar_family_fixtures_pass`].
const GRAMMAR_FIXTURES: [&str; 33] = [
    "grammar-comment-start",
    "grammar-remove-part",
    "grammar-split-slot",
    "grammar-reach",
    "grammar-expand",
    "grammar-expand-undeclared",
    "grammar-ordinals",
    "grammar-string-sequences",
    "w3c-turtle-triples",
    // FBL 0.16: Turtle's writes.
    "grammar-blank-line",
    "grammar-blank-line/empty",
    "grammar-continue",
    "grammar-continue/last-line",
    "grammar-remove-parts",
    "grammar-remove-parts/last-line",
    "structurizr-open-block",
    "structurizr-absent-arguments",
    "structurizr-add-view",
    "structurizr-block-comments",
    "structurizr-comments",
    "structurizr-escapes",
    "structurizr-gathered-tags",
    "structurizr-legacy-layout",
    "structurizr-recover-edges",
    "structurizr-recover-nested",
    "structurizr-rename",
    "structurizr-split-tags",
    "structurizr-trailing-comments",
    "structurizr-view-statements",
    "structurizr-view-without-key/default-key",
    "structurizr-view-without-key/no-view",
    "w3c-turtle-grammar",
    "w3c-turtle-grammar-partial",
];

/// The fixtures of one binding that several tool types read (FBL 0.10 sections 5.1, 8.2, 9.1): the readings of one
/// Databricks file, inserts into a flow sequence, a binding's refusals, a secondary slot, a level written on an add and
/// a template's key, run by [`the_fixtures_of_one_binding_with_several_readings_pass`].
const READINGS_FIXTURES: [&str; 23] = [
    "databricks-flow-empty/job",
    "databricks-flow-empty/read-back",
    "databricks-flow-one-line/job",
    "databricks-flow-one-line/refused/mapping",
    "databricks-flow-one-line/refused/several-lines",
    "databricks-library-add/pipeline",
    "databricks-library-add/read-back",
    "databricks-readings/bundle",
    "databricks-readings/job",
    "databricks-readings/pipeline",
    "databricks-refusals/job",
    "databricks-refusals/pipeline",
    "databricks-rename-shared/bundle",
    "databricks-rename-shared/job",
    "databricks-resources/bundle",
    "databricks-resources/job-first",
    "databricks-resources/job-named",
    "databricks-resources/missing",
    "databricks-resources/pipe-first",
    "databricks-resources/pipe-named",
    "databricks-secondary/both",
    "databricks-secondary/legacy",
    "databricks-template-key",
];

/// The fixtures the library departs from, which [`the_yaml_family_departs_from_the_fixture_only_as_recorded`] and
/// [`the_json_family_departs_from_the_fixture_only_as_recorded`] run with the departure written out, as the
/// standalone host's `InsertEmptyFixturesTests.Departing` does: FBL 0.6 section 6.2 applies each attribute binding's
/// `empty` rule on insert, which the standalone host's yaml and json families do not yet (its spec 025 task T216).
const DEPARTING: [&str; 2] = ["insert-empty-json", "insert-empty-yaml"];

/// The Knowledge designer's fixtures the standalone host vendored under `Conformance/etalii.adp`, which its
/// `KnowledgeFixturesTests` reads, saves and checks for byte coverage.
const KNOWLEDGE_FIXTURES: [&str; 7] = [
    "knowledge-equivalence/json",
    "knowledge-equivalence/xml",
    "knowledge-equivalence/yaml",
    "knowledge-json",
    "knowledge-kept",
    "knowledge-xml",
    "knowledge-yaml",
];

/// How many steps of each Knowledge fixture pass, and of how many (`KnowledgeFixtureStepsTests.StepsThatPass`).
const KNOWLEDGE_STEPS: [(&str, usize, usize); 4] = [
    ("knowledge-kept", 11, 11),
    ("knowledge-yaml", 150, 150),
    ("knowledge-json", 150, 150),
    ("knowledge-xml", 150, 150),
];

/// The fixtures of the Wardley map's binding (FBL 0.12): segments in any order, items, a brace on its own line, and
/// identities in the legacy file, run by [`the_wardley_map_fixtures_pass`].
const WARDLEY_FIXTURES: [&str; 14] = [
    "wardley-annotation-positions",
    "wardley-brace-ending",
    "wardley-comments/comments-everywhere",
    "wardley-comments/trailing-comments",
    "wardley-duplicate-names/duplicate-names",
    "wardley-duplicate-names/same-key-twice",
    "wardley-legacy-identities",
    "wardley-pipelines",
    "wardley-rename-identity",
    "wardley-strategy-identities",
    "wardley-trailer/decorators",
    "wardley-trailer/label-twice",
    "wardley-trailer/lenient-trailer",
    "wardley-unclosed-pipeline",
];

/// The fixtures of a new member of a mapping (FBL 0.14 section 6.2), named by its key, in both families that have one,
/// run by [`the_fixtures_of_a_new_member_pass`].
const MEMBER_FIXTURES: [&str; 4] = [
    "databricks-resource-add/bundle",
    "databricks-resource-add/fresh",
    "databricks-resource-add/no-jobs",
    "insert-member-json",
];

/// The fixtures of features this port does not have yet: FBL 0.8's constants an insert writes (section 6.2), and one
/// the standalone host does not vendor either. They are out of scope here, not failing: etalii.adp's splice replay
/// (`fixtures.rs`) still runs every one of them. The hierarchical-requirement tool type's fixtures are run by
/// [`hierarchical_requirement`], through the stand-in for its DESL interpreter.
const NOT_SUPPORTED: [&str; 13] = [
    // The standalone host does not vendor this one at a7fdf03 either: its Retry's attempts is read as the text "1",
    // as every value of a lines or bullets group is, and the fixture reads the number 1, which the specification's
    // type gives it above FBL (agent-behavior-modelling.dis, `Retry.attempts: int`).
    "behavior-tree-read",
    "insert-constant-json",
    "insert-constant-yaml",
    // A causal loop's flow properties belong to etalii.adp spec 074, which the desktop host takes up on its own branch.
    "causal-loop-flow-default",
    "causal-loop-flow-legacy/findings",
    "causal-loop-flow-legacy/on-call",
    "causal-loop-flow-legacy/reference",
    "causal-loop-flow-older",
    "causal-loop-flow-range",
    "causal-loop-flow-read",
    "causal-loop-flow-set",
    "causal-loop-flow-set/lf",
    "causal-loop-flow-unknown",
];

fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp")
}

fn fixtures_root() -> PathBuf {
    vendor().join("specifications/fbl/fixtures")
}

fn read_json(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

// ---- what the tool types' specifications give (the standalone host's Tests/Support) ----

fn attribute_text(attributes: &Map, name: &str) -> String {
    match attributes.get(name) {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => if *b { "True" } else { "False" }.into(),
        Some(Value::Int(i)) => i.to_string(),
        Some(Value::Double(d)) => d.to_string(),
        Some(other) => other.to_string(),
    }
}

fn unprefixed(id: Option<&str>) -> String {
    id.map(|i| i[i.find(':').map(|p| p + 1).unwrap_or(0)..].to_owned())
        .unwrap_or_default()
}

/// The DISL id derivations the vendored fixtures name their elements by (`Support/NaturalIds.cs`), mirroring the
/// `persistence.ids` of the specifications in etalii.adp `definitions/diagrams/`.
fn natural_id(binding: &str, request: &IdRequest<'_>) -> Option<String> {
    let a = |name: &str| attribute_text(request.attributes, name);
    Some(match (binding, request.rule) {
        // The fixture's and Turtle's (FBL 0.15): a number is no name, so a place is added to it, and a triple is its parts.
        ("triples", "prefix" | "sparql-prefix") => format!("prefix:{}", a("name")),
        ("triples", _) => format!("triple:{}|{}|{}", a("s"), a("p"), a("o")),
        ("ordinals", "node") => format!("n{}@{}", a("number"), request.line),
        ("ordinals", "cell") => format!("c{}@{}", a("number"), request.line),
        ("cld", "variable") => format!("variable:{}", a("name")),
        ("cld", "link") => format!("link:{}|{}", a("causeName"), a("effectName")),
        ("cld", "loop") => format!("loop:{}", a("identifier")),
        ("job", "task") => format!("task:{}", a("key")),
        ("job", "cluster") => format!("cluster:{}", a("key")),
        ("job", "dependency") => format!("edge:{}->{}", unprefixed(request.source), unprefixed(request.target)),
        ("settings", "pipeline") => "pipeline".into(),
        ("settings", "library") => format!("library:{}", a("path")),
        // One binding for the three Databricks readings (`persistence.ids` of databricks-bundle.dis, databricks-job.dis
        // and databricks-pipeline.dis). A job is the job reading's diagram, whose id no fixture names.
        ("configuration", "bundle") => "bundle".into(),
        ("configuration", "jobResource" | "pipelineResource") => {
            format!("resource:{}/{}", a("resourceKind"), a("key"))
        }
        ("configuration", "target") => format!("target:{}", a("name")),
        ("configuration", "job") => format!("job:{}", a("jobKey")),
        ("configuration", "task") => format!("task:{}", a("key")),
        ("configuration", "cluster") => format!("cluster:{}", a("key")),
        ("configuration", "dependency") => format!("edge:{}->{}", a("upstreamKey"), unprefixed(request.target)),
        ("configuration", "pipeline" | "settings") => "pipeline".into(),
        ("configuration", "notebookLibrary" | "fileLibrary" | "globLibrary" | "settingsLibrary") => {
            format!("library:{}", a("path"))
        }
        ("freeplane", "branch") => format!(
            "branch:{}->{}",
            request.source.unwrap_or_default(),
            request.target.unwrap_or_default()
        ),
        // The C4 types' `persistence.ids` (c4-*.dis): a relationship's ends as written and its line.
        ("workspace", "relationship") => format!("{}->{}@{}", a("from"), a("to"), request.line),
        ("workspace", "interaction") => format!("interaction:{}->{}@{}", a("from"), a("to"), request.line),
        // The agent activity diagram (DISL 0.4, persistence.view.bind): the root is the diagram, and a locked
        // position and a group state are view data, keyed by the element they name.
        ("aad", "diagram") => "diagram".into(),
        ("aad", "placement") => format!("pinned:{}", a("element")),
        ("aad", "group") => format!("groups:{}#{}", a("element"), a("group")),
        // The agent behavior model (agent-behavior-modelling.dis, persistence.ids, strategy derived): a node's place
        // in the tree, its parent's place, a dot and its position among its siblings counted from 1.
        ("abm", _) => match request.parent_id {
            Some(parent) => format!("{parent}.{}", request.position_in_slot + 1),
            None => (request.position_in_slot + 1).to_string(),
        },
        _ => return None,
    })
}

/// The attributes a binding's tool type gives a date or date-time type, which FBL section 6.3 asks of the host.
fn time_attributes(binding: &str) -> HashSet<String> {
    match binding {
        "aad" => ["task.updated", "pullRequest.updated"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        _ => HashSet::new(),
    }
}

/// Whether a binding's tool type keeps DISL's default deletion policy, which clears the references to a removed
/// element.
fn unsets_references(binding: &str) -> bool {
    binding == "aad"
}

/// The id derivations of the Knowledge designer's specification (`Support/KnowledgeIds.cs`).
fn knowledge_id(request: &IdRequest<'_>) -> Option<String> {
    let a = |name: &str| attribute_text(request.attributes, name);
    let parent = request.parent_id.unwrap_or_default();
    Some(match request.rule {
        "table" => "table".into(),
        "column" => format!("{parent}/columns/{}", a("property")),
        "sort" => format!("{parent}/sorts/{}", a("property")),
        "condition1" | "filterGroup1" => format!("{parent}/filter/{}", request.position_in_slot),
        "condition2" | "filterGroup2" | "condition3" => format!("{parent}/{}", request.position_in_slot),
        "groupOrder" | "hiddenGroup" | "collapsedGroup" => {
            format!("{parent}/{}/{}", request.parent_slot.unwrap_or_default(), a("key"))
        }
        "cell" => format!("{parent}/{}", a("property")),
        "cellOption" => format!("{parent}/{}", a("option")),
        "cellRow" => format!("{parent}/{}", a("row")),
        _ => return None,
    })
}

type Types = BTreeMap<(String, String), String>;

/// The attribute types of a specification's metamodel, following `extends`: a type's own first, then each type it
/// extends, nearest first (`Support/TimelineTypes.cs`, `Support/KnowledgeTypes.cs`).
fn read_types(path: &Path, groups: &[&str]) -> Types {
    let document = read_json(path);
    let metamodel = &document["metamodel"];
    let mut declared: BTreeMap<String, Json> = BTreeMap::new();
    for group in groups {
        if let Some(types) = metamodel.get(*group).and_then(Json::as_object) {
            for (name, value) in types {
                declared.insert(name.clone(), value.clone());
            }
        }
    }
    let mut result = Types::new();
    for name in declared.keys() {
        let mut current = Some(name.clone());
        while let Some(type_name) = current {
            let Some(definition) = declared.get(&type_name) else {
                break;
            };
            if let Some(attributes) = definition.get("attributes").and_then(Json::as_object) {
                for (attribute, value) in attributes {
                    if let Some(t) = value.get("type").and_then(Json::as_str) {
                        result
                            .entry((name.clone(), attribute.clone()))
                            .or_insert_with(|| t.to_owned());
                    }
                }
            }
            current = definition.get("extends").and_then(Json::as_str).map(str::to_owned);
        }
    }
    result
}

fn timeline_types() -> &'static Types {
    static TYPES: OnceLock<Types> = OnceLock::new();
    TYPES.get_or_init(|| {
        read_types(
            &vendor().join("definitions/diagrams/timeline.dis"),
            &["types", "relations"],
        )
    })
}

fn knowledge_types() -> &'static Types {
    static TYPES: OnceLock<Types> = OnceLock::new();
    TYPES.get_or_init(|| read_types(&vendor().join("definitions/designers/knowledge.des"), &["types"]))
}

fn type_of(types: &'static Types) -> adp_fbl::model::AttributeType {
    Rc::new(move |element: &str, attribute: &str| types.get(&(element.to_owned(), attribute.to_owned())).cloned())
}

// ---- fixtures ----

/// A fixture's binding reference: one whose file is not where it points is read as naming the file of that name
/// two folders up, as the standalone host's corpus keeps a tool type's binding beside the others.
fn binding_reference(reference: &str, folder: &Path) -> String {
    let hash = reference.rfind('#').expect("a binding reference");
    let file = &reference[..hash];
    if file.is_empty() || folder.join(file).exists() {
        return reference.to_owned();
    }
    let name = Path::new(file).file_name().unwrap().to_string_lossy();
    format!("../../{name}{}", &reference[hash..])
}

fn load_binding(fixture: &Json, folder: &Path) -> Result<(Rc<FblBinding>, String), String> {
    let reference = binding_reference(
        fixture["binding"].as_str().ok_or("the fixture names no binding")?,
        folder,
    );
    let (binding, problems) = documents::resolve_reference(&reference, &folder.join("fixture.json"))?;
    if let Some(error) = problems.iter().find(|p| p.severity == ProblemSeverity::Error) {
        return Err(format!("the binding has an error: {error}"));
    }
    let file = &reference[..reference.rfind('#').unwrap()];
    let text = fs::read_to_string(folder.join(file)).unwrap_or_default();
    Ok((Rc::new(binding), text))
}

fn json_value(value: &Json) -> Value {
    match value {
        Json::String(s) => Value::from(s.as_str()),
        Json::Number(n) => n
            .as_i64()
            .map(Value::Int)
            .unwrap_or_else(|| Value::Double(n.as_f64().unwrap_or(0.0))),
        Json::Bool(b) => Value::Bool(*b),
        Json::Array(items) => Value::list(items.iter().map(json_value).collect()),
        Json::Object(_) => Value::map(attributes(value)),
        Json::Null => Value::Null,
    }
}

fn attributes(json: &Json) -> Map {
    json.as_object()
        .map(|o| o.iter().map(|(k, v)| (k.clone(), json_value(v))).collect())
        .unwrap_or_default()
}

fn read_splices(step: &Json) -> Vec<Splice> {
    serde_json::from_value(step["splices"].clone()).expect("a fixture's splices")
}

/// The expected splices in body order, those at one offset as listed.
fn in_body_order(mut splices: Vec<Splice>) -> Vec<Splice> {
    let mut indexed: Vec<(usize, Splice)> = splices.drain(..).enumerate().collect();
    indexed.sort_by_key(|(i, s)| (s.start, *i));
    indexed.into_iter().map(|(_, s)| s).collect()
}

/// A step's splices by the files they name, sorted by name, each in body order.
fn files_of(step: &Json) -> Vec<(String, Vec<Splice>)> {
    let mut files: BTreeMap<String, Vec<Splice>> = BTreeMap::new();
    for splice in step["splices"].as_array().into_iter().flatten() {
        if let Some(file) = splice["file"].as_str() {
            let splice = serde_json::from_value(splice.clone()).expect("a fixture's splice");
            files.entry(file.to_owned()).or_default().push(splice);
        }
    }
    files
        .into_iter()
        .map(|(file, splices)| (file, in_body_order(splices)))
        .collect()
}

fn describe(splices: &[Splice]) -> String {
    splices
        .iter()
        .map(|s| format!("{} {}-{} {:?}", s.operation, s.start, s.end, s.text))
        .collect::<Vec<_>>()
        .join("\n  ")
}

/// A fixture's subject: a body read through its binding, or a registration (an `.adp` input).
enum Subject {
    Body {
        body: Box<OpenBody>,
        binding: Rc<FblBinding>,
    },
    /// A registration opened through its binding (FBL 0.7 section 8.1). When its body is beside it, the body is read
    /// too, as the document the registration opens: its elements are the reading's, and the ids the registration's
    /// stale entries are told by (FBL section 8.5); the findings are the body's, the registration's and those of a
    /// legacy layout (FBL 0.11 section 8.7). An edit goes to the file that holds what it changes.
    Registration(Box<OpenDocument>),
}

/// The ids a fixture names the elements of a tool type by whose ids are random (the Wardley map's `persistence.ids`,
/// wardley-map.dis, `uuid-v4`), by each element's type and line: the fixture's `read` lists them in reading order, and an
/// id the strategy would make is one no fixture can state (FBL 0.12, wardley-pipelines' `doc`).
type NamedIds = Rc<HashMap<(String, usize), String>>;

/// The attributes wardley-map.dis gives a number, an int or a bool, and those of a position's items, by name: the binding
/// names its types otherwise than the specification, which has no `typeMap` while it stays a plugin. FBL leaves a value's
/// type to the specification (section 5.2).
fn wardley_type(_: &str, attribute: &str) -> Option<String> {
    let name = attribute.rsplit('.').next().unwrap_or(attribute);
    match name {
        "visibility" | "maturity" | "visibility1" | "maturity1" | "visibility2" | "maturity2" => Some("number".into()),
        "number" | "width" | "height" => Some("int".into()),
        "inertia" => Some("bool".into()),
        _ => None,
    }
}

/// What a binding's tool type gives a body's reading, as FBL leaves it to the specification (DISL section 11.5).
fn options_of(binding_name: &str, file_name: &str, named: Option<NamedIds>) -> FblOptions {
    let derive_name = binding_name.to_owned();
    let wardley = binding_name == "map";
    FblOptions {
        file_name: file_name.to_owned(),
        derive_id: Some(Rc::new(move |request: &IdRequest<'_>| match &named {
            Some(named) => named.get(&(request.type_name.to_owned(), request.line)).cloned(),
            None => natural_id(&derive_name, request),
        })),
        time_attributes: time_attributes(binding_name),
        attribute_type: if binding_name == "timeline" {
            Some(type_of(timeline_types()))
        } else if wardley {
            let types: adp_fbl::model::AttributeType = Rc::new(wardley_type);
            Some(types)
        } else {
            None
        },
        // The C4 types' `persistence.ids.missing`, and the attributes of their `ViewInclude` that hold lists.
        ephemeral_missing_ids: binding_name == "workspace",
        many: (binding_name == "workspace" || wardley).then(|| {
            // A component's decorators are one set (FBL 0.12 section 4.9), which wardley-map.dis holds as five flags.
            let many: adp_fbl::model::Many = Rc::new(|element: &str, attribute: &str| {
                element == "ViewInclude" && matches!(attribute, "members" | "excludes") || attribute == "decorators"
            });
            many
        }),
        ..FblOptions::default()
    }
}

impl Subject {
    /// `origin` is the reading a body without a registration is read as: a fixture's `template` names it.
    fn open(input: Vec<u8>, name: &str, binding: Rc<FblBinding>, folder: &Path, origin: Option<&str>) -> Subject {
        Self::open_named(input, name, binding, folder, origin, None)
    }

    fn open_named(
        input: Vec<u8>,
        name: &str,
        binding: Rc<FblBinding>,
        folder: &Path,
        origin: Option<&str>,
        named: Option<NamedIds>,
    ) -> Subject {
        if name.to_lowercase().ends_with(".adp") {
            let options = options_of(&binding.name, name, named);
            return Subject::Registration(Box::new(OpenDocument::open(
                input,
                &folder.join(name),
                binding,
                options,
                &fixtures_root(),
            )));
        }
        let options = FblOptions {
            origin: origin.map(str::to_owned),
            ..options_of(&binding.name, name, named)
        };
        Subject::Body {
            body: Box::new(OpenBody::open(input, binding.clone(), Rc::new(options))),
            binding,
        }
    }

    fn bytes(&self) -> Vec<u8> {
        match self {
            Subject::Body { body, .. } => body.bytes().to_vec(),
            Subject::Registration(document) => document.registration().bytes().to_vec(),
        }
    }

    fn model(&self) -> Option<FblModel> {
        match self {
            Subject::Body { body, .. } => Some(body.model()),
            Subject::Registration(document) => document.model(),
        }
    }

    fn change(&mut self, change: &ModelChange) -> PlanResult {
        match self {
            Subject::Registration(document) => document.change(change),
            Subject::Body { body, binding } => {
                if let ModelChange::Remove { id } = change {
                    let unsets = unsets(body, binding, id);
                    if !unsets.is_empty() {
                        return remove_with(body, change, unsets);
                    }
                }
                body.change(change)
            }
        }
    }

    fn undo(&mut self) -> UndoResult {
        match self {
            Subject::Body { body, .. } => body.undo(),
            Subject::Registration(document) => document.undo(),
        }
    }

    fn redo(&mut self) -> UndoResult {
        match self {
            Subject::Body { body, .. } => body.redo(),
            Subject::Registration(document) => document.redo(),
        }
    }

    /// The names of the files the last edit wrote (FBL 0.10 section 15.3).
    fn edited_files(&self) -> Vec<String> {
        match self {
            Subject::Body { .. } => Vec::new(),
            Subject::Registration(document) => document.last_files().into_iter().map(str::to_owned).collect(),
        }
    }

    /// The splices the last edit wrote to a file of the document (FBL 0.12 section 8.6: one edit may write several).
    fn edited_splices(&self, name: &str) -> Vec<Splice> {
        match self {
            Subject::Body { .. } => Vec::new(),
            Subject::Registration(document) => document.last_splices(name).map(<[Splice]>::to_vec).unwrap_or_default(),
        }
    }

    /// A file of the document by the name a fixture's `expectFiles` gives it.
    fn file(&self, name: &str) -> Option<Vec<u8>> {
        match self {
            Subject::Body { .. } => None,
            Subject::Registration(document) => document.file(name).map(<[u8]>::to_vec),
        }
    }
}

/// The keys DISL's default deletion policy clears when an element is removed: every reference to it from an entry
/// that does not go with it. A fixture of a tool type gives a removal as one edit with those splices in it; FBL
/// itself only cascades (section 6.2), so the runner supplies the policy as it supplies the derived ids.
fn unsets(body: &OpenBody, binding: &FblBinding, id: &str) -> Vec<ModelChange> {
    let model = body.model();
    if !unsets_references(&binding.name) {
        return Vec::new();
    }
    let Some(removed) = model.elements.iter().find(|e| e.id == id) else {
        return Vec::new();
    };
    let cascade: Vec<String> = binding
        .rule(&removed.rule)
        .and_then(|r| r.remove.as_ref())
        .map(|r| r.cascade.clone())
        .unwrap_or_default();
    let mut unsets = Vec::new();
    for other in &model.elements {
        if other.id == id || cascade.contains(&other.rule) || other.parent_id.as_deref() == Some(id) {
            continue;
        }
        let rule = binding.rule(&other.rule).expect("the element's rule");
        let mut cleared = Map::default();
        for (key, attribute) in &rule.attributes {
            if attribute
                .reference
                .as_ref()
                .is_some_and(|r| r.to.contains(&removed.rule))
                && other.attributes.get(key).and_then(Value::as_str) == Some(id)
            {
                cleared.insert(key.clone(), Value::Null);
            }
        }
        if !cleared.is_empty() {
            unsets.push(ModelChange::Set {
                id: other.id.clone(),
                attributes: cleared,
            });
        }
    }
    unsets
}

/// Plans the unsets and the removal against the body as it is, and applies them as one edit.
fn remove_with(body: &mut OpenBody, remove: &ModelChange, unsets: Vec<ModelChange>) -> PlanResult {
    let mut splices: Vec<Splice> = Vec::new();
    for change in unsets.iter().chain(std::iter::once(remove)) {
        match body.plan(change) {
            PlanResult::Planned(part) => splices.extend(part.splices),
            other => return other,
        }
    }
    let mut indexed: Vec<(usize, Splice)> = splices.into_iter().enumerate().collect();
    indexed.sort_by_key(|(i, s)| (s.start, *i));
    let edit = PlannedEdit {
        splices: indexed.into_iter().map(|(_, s)| s).collect(),
        snapshot: false,
    };
    if let Err(problem) = body.apply(&edit) {
        return PlanResult::Invalid(problem);
    }
    PlanResult::Planned(edit)
}

/// A fixture's edit as a model change, as the standalone host's `ConformanceFixturesTests.ReadChange` reads it at
/// `a7fdf03`. A position (FBL 0.7 section 11.2) is converted to the model change's index, which counts the siblings as
/// they are before the change, the moved element included; an `after` or `before` is the place of that sibling, found
/// in `model` among the elements with the same parent and type.
fn read_change(edit: &Json, model: Option<&FblModel>) -> Result<ModelChange, String> {
    if let Some(setting) = edit.get("setting") {
        return Ok(ModelChange::Setting(read_setting(setting)));
    }
    if let Some(moved) = edit.get("move") {
        let id = moved["element"].as_str().unwrap_or_default();
        let parent = moved["parent"].as_str();
        let type_name = model.and_then(|m| m.find(id)).map(|e| e.type_name.as_str());
        return Ok(ModelChange::Move {
            id: id.to_owned(),
            new_parent_id: parent.map(str::to_owned),
            index: position_index(&moved["position"], model, parent, type_name)?.unwrap_or(-1),
        });
    }
    if edit.get("save").is_some() {
        return Ok(ModelChange::Save);
    }
    if let Some(set) = edit.get("set") {
        return Ok(ModelChange::Set {
            id: set["element"].as_str().unwrap_or_default().to_owned(),
            attributes: attributes(&set["attributes"]),
        });
    }
    if let Some(add) = edit.get("add") {
        // A position after an entry of another type is no index among siblings (etalii.adp spec 074).
        if let Some(after) = add["position"]["after"].as_str()
            && model
                .and_then(|m| m.find(after))
                .is_some_and(|e| add["type"].as_str() != Some(e.type_name.as_str()))
        {
            return Ok(ModelChange::AddAfter {
                type_name: add["type"].as_str().unwrap_or_default().to_owned(),
                attributes: attributes(&add["attributes"]),
                after: after.to_owned(),
            });
        }
        return Ok(ModelChange::Add {
            type_name: add["type"].as_str().unwrap_or_default().to_owned(),
            id: add.get("id").and_then(Json::as_str).map(str::to_owned),
            attributes: attributes(&add["attributes"]),
            parent_id: add.get("parent").and_then(Json::as_str).map(str::to_owned),
            index: match add.get("position") {
                Some(position) => position_index(
                    position,
                    model,
                    add.get("parent").and_then(Json::as_str),
                    add["type"].as_str(),
                )?,
                None => None,
            },
            slot: add.get("slot").and_then(Json::as_str).map(str::to_owned),
        });
    }
    if let Some(remove) = edit.get("remove") {
        return Ok(ModelChange::Remove {
            id: remove["element"].as_str().unwrap_or_default().to_owned(),
        });
    }
    if let Some(place) = edit.get("place") {
        return Ok(ModelChange::Place {
            id: place["element"].as_str().unwrap_or_default().to_owned(),
            x: place["x"].as_f64().unwrap_or(0.0),
            y: place["y"].as_f64().unwrap_or(0.0),
        });
    }
    Err(format!("Unknown fixture edit: {edit}."))
}

/// A fixture's change to an entry of a block the binding declares (FBL 0.7 sections 8.4, 15.3).
fn read_setting(setting: &Json) -> Setting {
    let text = |name: &str| setting.get(name).and_then(Json::as_str).map(str::to_owned);
    Setting {
        block: text("block").unwrap_or_default(),
        key: text("key").unwrap_or_default(),
        value: text("value"),
        position: setting.get("position").map(|at| SettingPosition {
            index: at.get("index").and_then(Json::as_i64),
            after: at.get("after").and_then(Json::as_str).map(str::to_owned),
            before: at.get("before").and_then(Json::as_str).map(str::to_owned),
        }),
        rename: text("rename"),
        remove_entry: setting.get("remove").and_then(Json::as_bool) == Some(true),
    }
}

/// The model change's index for a fixture's position among `parent`'s children of `type_name`.
fn position_index(
    position: &Json,
    model: Option<&FblModel>,
    parent: Option<&str>,
    type_name: Option<&str>,
) -> Result<Option<i64>, String> {
    if let Some(index) = position.get("index") {
        return Ok(index.as_i64());
    }
    let siblings: Vec<&str> = model
        .map(|m| {
            m.elements
                .iter()
                .filter(|e| e.parent_id.as_deref() == parent && Some(e.type_name.as_str()) == type_name)
                .map(|e| e.id.as_str())
                .collect()
        })
        .unwrap_or_default();
    let place = |sibling: &Json| {
        let sibling = sibling.as_str().unwrap_or_default();
        siblings
            .iter()
            .position(|s| *s == sibling)
            .map(|at| at as i64)
            .ok_or_else(|| format!("The position names '{sibling}', which is no sibling here."))
    };
    if let Some(after) = position.get("after") {
        return Ok(Some(place(after)? + 1));
    }
    if let Some(before) = position.get("before") {
        return Ok(Some(place(before)?));
    }
    Err(format!("Unknown fixture position: {position}."))
}

fn written(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::String(s) => format!("the text {s:?}"),
        Value::Bool(b) => format!("the bool {b}"),
        Value::Int(i) => format!("the Int64 {i}"),
        Value::Double(d) => format!("the Double {d}"),
        other => format!("the value {other}"),
    }
}

fn same_value(actual: &Value, expected: &Json) -> bool {
    match expected {
        Json::String(s) => actual.as_str() == Some(s.as_str()),
        Json::Number(n) => actual.as_number().is_some_and(|a| Some(a) == n.as_f64()),
        Json::Bool(b) => matches!(actual, Value::Bool(x) if x == b),
        Json::Null => actual.is_null(),
        Json::Array(items) => match actual {
            Value::List(list) => list.len() == items.len() && list.iter().zip(items).all(|(a, e)| same_value(a, e)),
            _ => false,
        },
        Json::Object(fields) => match actual {
            Value::Map(map) => fields
                .iter()
                .all(|(name, e)| map.get(name).is_some_and(|a| same_value(a, e))),
            _ => false,
        },
    }
}

/// Each attribute a fixture's element names has that value and that JSON type, a string for a text (FBL 0.5
/// section 15.3); an attribute it does not name is not compared.
fn check_attributes(element: &adp_fbl::FblElement, attributes: &Json, name: &str) -> Result<(), String> {
    for (attribute, expected) in attributes.as_object().into_iter().flatten() {
        let label = format!("{name}: '{attribute}' of '{}'", element.id);
        let Some(actual) = element.attributes.get(attribute) else {
            // A slot that is absent, with no default, is stated as null.
            if expected.is_null() {
                continue;
            }
            return Err(format!("{label} was not read."));
        };
        let same = same_value(actual, expected);
        if !same {
            return Err(format!(
                "{label} is {}, where the fixture reads {expected}.",
                written(actual)
            ));
        }
    }
    Ok(())
}

/// The attributes of the C4 types' diagram (`persistence.typeMap` of c4-*.dis, `as: "diagram"`) and of the job reading's
/// (databricks-job.dis, `Job`): a binding type's attribute under the name the diagram has it by, its own unless the map
/// renames it, with every value it was read with, as a fixture's `document` lists them (FBL 0.11 section 9.3).
fn check_document(model: &FblModel, read: &Json, name: &str) -> Result<(), String> {
    let Some(attributes) = read.pointer("/document/attributes").and_then(Json::as_object) else {
        return Ok(());
    };
    for (attribute, expected) in attributes {
        let wire = match attribute.as_str() {
            "workspaceName" => "name",
            "includes" => "members",
            other => other,
        };
        let mut values: Vec<&Value> = Vec::new();
        for element in model
            .elements
            .iter()
            .filter(|e| matches!(e.type_name.as_str(), "Workspace" | "ViewInclude" | "Job"))
        {
            values.extend(element.attributes.get(wire));
        }
        // A number is compared as written: the type comes from the specification, which this runner does not read.
        let written = |value: &Value| match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        let same = match expected {
            Json::Null => values.is_empty(),
            Json::Array(items) => {
                let read: Vec<String> = values
                    .iter()
                    .flat_map(|v| match v {
                        Value::List(list) => list.iter().map(written).collect::<Vec<_>>(),
                        other => vec![written(other)],
                    })
                    .collect();
                let items: Vec<String> = items
                    .iter()
                    .map(|i| i.as_str().unwrap_or_default().to_owned())
                    .collect();
                read == items
            }
            Json::Number(n) => values.len() == 1 && written(values[0]) == n.to_string(),
            other => values.len() == 1 && same_value(values[0], other),
        };
        if !same {
            return Err(format!(
                "{name}: the document's {attribute} is {values:?}, where the fixture reads {expected}."
            ));
        }
    }
    Ok(())
}

fn check_read(model: &FblModel, read: &Json, name: &str) -> Result<(), String> {
    check_document(model, read, name)?;
    if let Some(unreadable) = read.get("unreadable").and_then(Json::as_bool)
        && unreadable != model.unreadable
    {
        return Err(format!(
            "{name}: unreadable is {}, where the fixture says {unreadable}.",
            model.unreadable
        ));
    }
    for element in read.get("elements").and_then(Json::as_array).into_iter().flatten() {
        let id = element["id"].as_str().unwrap_or_default();
        let Some(found) = model.find(id) else {
            let all: Vec<String> = model
                .elements
                .iter()
                .map(|e| format!("{} ({})", e.id, e.type_name))
                .collect();
            return Err(format!("{name}: no element '{id}' was read; read: {}.", all.join(", ")));
        };
        if Some(found.type_name.as_str()) != element["type"].as_str() {
            return Err(format!(
                "{name}: '{id}' is a {}, where the fixture reads {}.",
                found.type_name, element["type"]
            ));
        }
        if let Some(parent) = element.get("parent")
            && parent.as_str() != found.parent_id.as_deref()
        {
            return Err(format!(
                "{name}: '{id}' is read in '{}', where the fixture reads {parent}.",
                found.parent_id.as_deref().unwrap_or_default()
            ));
        }
        if let Some(attributes) = element.get("attributes") {
            check_attributes(found, attributes, name)?;
        }
    }
    if let Some(findings) = read.get("findings").and_then(Json::as_array) {
        // A fixture that lists no finding says the reading has none.
        if findings.is_empty() && !model.findings.is_empty() {
            let all: Vec<String> = model
                .findings
                .iter()
                .map(|f| format!("{} {}", f.code, f.message))
                .collect();
            return Err(format!(
                "{name}: the fixture expects no finding; read: {}",
                all.join("; ")
            ));
        }
        for finding in findings {
            let code = finding["rule"].as_str().unwrap_or_default();
            let line = finding.get("line").and_then(Json::as_u64).map(|l| l as usize);
            if !model
                .findings
                .iter()
                .any(|f| f.code == code && line.is_none_or(|l| f.location.as_ref().map(|x| x.line) == Some(l)))
            {
                let all: Vec<String> = model.findings.iter().map(ToString::to_string).collect();
                return Err(format!(
                    "{name}: no finding {code} at line {line:?}; read: {}",
                    all.join("; ")
                ));
            }
        }
    }
    Ok(())
}

/// The ids of a fixture of a tool type whose ids are random: read once to learn each element's type and line, whose id
/// is the one at its place in the fixture's `read`.
fn named_ids(input: &[u8], name: &str, binding: &Rc<FblBinding>, folder: &Path, fixture: &Json) -> Option<NamedIds> {
    if binding.name != "map" {
        return None;
    }
    let model = Subject::open(input.to_vec(), name, binding.clone(), folder, None).model()?;
    let listed = fixture.pointer("/read/elements")?.as_array()?;
    let named = model
        .elements
        .iter()
        .zip(listed)
        .filter_map(|(element, listed)| {
            let id = listed["id"].as_str()?;
            Some(((element.type_name.clone(), element.line), id.to_owned()))
        })
        .collect();
    Some(Rc::new(named))
}

/// Runs a fixture's reading and steps as `ConformanceFixturesTests.Run` does.
fn run(name: &str, folder: &Path, fixture: &Json) -> Result<usize, String> {
    let (binding, binding_text) = load_binding(fixture, folder)?;
    let input_name = fixture["input"].as_str().unwrap_or_default();
    let input = fs::read(folder.join(input_name)).map_err(|e| e.to_string())?;
    // A fixture of a new document: what the template writes is the input (FBL 0.10 section 15.3).
    let template = fixture.get("template");
    let origin = template.and_then(|t| t["origin"].as_str());
    if let Some(template) = template {
        let written =
            adp_fbl::routing::produce(&binding, origin, template["name"].as_str().unwrap_or_default(), &|_| {
                String::new()
            });
        if written.as_deref() != Some(input.as_slice()) {
            return Err(format!("{name}: the template does not write the input."));
        }
    }
    let named = named_ids(&input, input_name, &binding, folder, fixture);
    let mut subject = Subject::open_named(input, input_name, binding, folder, origin, named);
    if let Some(read) = fixture.get("read")
        && let Some(model) = subject.model()
    {
        check_read(&model, read, name)?;
    }
    let steps = fixture["steps"].as_array().cloned().unwrap_or_default();
    for (index, step) in steps.iter().enumerate() {
        let label = format!("{name} step {}", index + 1);
        let before = subject.bytes();
        let expected = in_body_order(read_splices(step));
        let actual = if step.get("undo").and_then(Json::as_bool) == Some(true) {
            match subject.undo() {
                UndoResult::Done(splices) => splices,
                UndoResult::Refused(reason) => return Err(format!("{label}: refused with '{reason}'.")),
            }
        } else if step.get("redo").and_then(Json::as_bool) == Some(true) {
            match subject.redo() {
                UndoResult::Done(splices) => splices,
                UndoResult::Refused(reason) => return Err(format!("{label}: refused with '{reason}'.")),
            }
        } else {
            let model = subject.model();
            let result = subject.change(&read_change(&step["edit"], model.as_ref())?);
            if let Some(refused) = step.get("refused").and_then(Json::as_str) {
                let PlanResult::Refused(reason) = result else {
                    return Err(format!("{label}: planned, where the fixture refuses with '{refused}'."));
                };
                // FBL words a refusal only where the binding gives the words; elsewhere the reason is the host's own.
                if binding_text.contains(refused) && reason != refused {
                    return Err(format!(
                        "{label}: refused with '{reason}', where the binding says '{refused}'."
                    ));
                }
                if reason.trim().is_empty() {
                    return Err(format!("{label}: refused without a reason."));
                }
                if before != subject.bytes() {
                    return Err(format!("{label}: a refused edit changed the document."));
                }
                Vec::new()
            } else {
                match result {
                    PlanResult::Planned(edit) => edit.splices,
                    PlanResult::Refused(reason) | PlanResult::Invalid(reason) => {
                        return Err(format!("{label}: refused with '{reason}'."));
                    }
                }
            }
        };
        // A fixture's splices that name their files are compared file by file (FBL 0.12 section 8.6).
        let named = files_of(step);
        if named.is_empty() {
            if expected != actual {
                return Err(format!(
                    "{label}: expected splices\n  {}\nbut planned\n  {}",
                    describe(&expected),
                    describe(&actual)
                ));
            }
        } else {
            let mut wrote = subject.edited_files();
            wrote.sort();
            if wrote != named.iter().map(|(file, _)| file.clone()).collect::<Vec<_>>() {
                return Err(format!(
                    "{label}: the splices are of {:?}, and the edit wrote {wrote:?}.",
                    named.iter().map(|(file, _)| file).collect::<Vec<_>>()
                ));
            }
            for (file, splices) in &named {
                let actual = subject.edited_splices(file);
                if *splices != actual {
                    return Err(format!(
                        "{label}: expected splices of {file}\n  {}\nbut planned\n  {}",
                        describe(splices),
                        describe(&actual)
                    ));
                }
            }
        }
        for (file, bytes) in step.get("expectFiles").and_then(Json::as_object).into_iter().flatten() {
            let held = subject.file(file);
            if held.as_deref() != bytes.as_str().map(str::as_bytes) {
                return Err(format!(
                    "{label}: {file} differs from 'expectFiles'.\nExpected:\n{}\nActual:\n{}",
                    bytes.as_str().unwrap_or_default(),
                    String::from_utf8_lossy(&held.unwrap_or_default())
                ));
            }
        }
        // A step that names no document leaves the one it started from.
        let expect = match (
            step.get("expect").and_then(Json::as_str),
            step.get("expectFile").and_then(Json::as_str),
        ) {
            (Some(text), _) => text.as_bytes().to_vec(),
            (None, Some(file)) => fs::read(folder.join(file)).map_err(|e| e.to_string())?,
            (None, None) => before,
        };
        if expect != subject.bytes() {
            return Err(format!(
                "{label}: the document differs from 'expect'.\nExpected:\n{}\nActual:\n{}",
                String::from_utf8_lossy(&expect),
                String::from_utf8_lossy(&subject.bytes())
            ));
        }
    }
    Ok(steps.len())
}

fn fixture_folder(name: &str) -> PathBuf {
    fixtures_root().join(name)
}

fn every_vendored_fixture() -> Vec<String> {
    let root = fixtures_root();
    let mut found = Vec::new();
    let mut folders = vec![root.clone()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
            } else if path.file_name().is_some_and(|n| n == "fixture.json") {
                let relative = path.parent().unwrap().strip_prefix(&root).unwrap();
                found.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn every_vendored_fixture_is_run_or_named_as_not_supported() {
    let designer = hierarchical_requirement::fixtures();
    let known: HashSet<&str> = STANDALONE_FIXTURES
        .iter()
        .chain(KNOWLEDGE_FIXTURES.iter())
        .chain(GRAMMAR_FIXTURES.iter())
        .chain(READINGS_FIXTURES.iter())
        .chain(WARDLEY_FIXTURES.iter())
        .chain(MEMBER_FIXTURES.iter())
        .chain(NOT_SUPPORTED.iter())
        .copied()
        .chain(designer.iter().map(String::as_str))
        .collect();
    let unknown: Vec<String> = every_vendored_fixture()
        .into_iter()
        .filter(|f| !known.contains(f.as_str()))
        .collect();
    assert!(
        unknown.is_empty(),
        "fixtures neither run nor named as not supported: {unknown:?}"
    );
}

#[test]
fn the_timeline_specifications_attribute_types_are_read() {
    assert!(
        timeline_types().len() >= 10,
        "only {} attribute types were read",
        timeline_types().len()
    );
    assert_eq!(
        timeline_types()
            .get(&("Period".into(), "label".into()))
            .map(String::as_str),
        Some("string")
    );
    assert_eq!(timeline_types().get(&("Moment".into(), "end".into())), None);
    assert!(knowledge_types().len() > 20);
    assert_eq!(
        knowledge_types()
            .get(&("Cell".into(), "number".into()))
            .map(String::as_str),
        Some("number")
    );
}

#[test]
fn every_fixture_the_standalone_host_passes_passes() {
    let mut failures = Vec::new();
    let mut steps = 0;
    for name in STANDALONE_FIXTURES.iter().filter(|n| !DEPARTING.contains(n)) {
        let folder = fixture_folder(name);
        let fixture = read_json(&folder.join("fixture.json"));
        match run(name, &folder, &fixture) {
            Ok(count) => steps += count,
            Err(problem) => failures.push(problem),
        }
    }
    assert!(
        failures.is_empty(),
        "{} fixtures failed:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
    assert!(steps > 100, "only {steps} steps were run");
}

#[test]
fn the_grammar_family_fixtures_pass() {
    let mut failures = Vec::new();
    for name in GRAMMAR_FIXTURES {
        let folder = fixture_folder(name);
        let fixture = read_json(&folder.join("fixture.json"));
        if let Err(problem) = run(name, &folder, &fixture) {
            failures.push(problem);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn the_fixtures_of_one_binding_with_several_readings_pass() {
    let mut failures = Vec::new();
    for name in READINGS_FIXTURES {
        let folder = fixture_folder(name);
        let fixture = read_json(&folder.join("fixture.json"));
        if let Err(problem) = run(name, &folder, &fixture) {
            failures.push(problem);
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn the_wardley_map_fixtures_pass() {
    let mut failures = Vec::new();
    for name in WARDLEY_FIXTURES {
        let folder = fixture_folder(name);
        let fixture = read_json(&folder.join("fixture.json"));
        if let Err(problem) = run(name, &folder, &fixture) {
            failures.push(problem);
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn the_fixtures_of_a_new_member_pass() {
    let mut failures = Vec::new();
    for name in MEMBER_FIXTURES {
        let folder = fixture_folder(name);
        let fixture = read_json(&folder.join("fixture.json"));
        if let Err(problem) = run(name, &folder, &fixture) {
            failures.push(problem);
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// FBL section 5.7: a rename to a name an element of another rule holds, which one reference names with it, is refused.
#[test]
fn a_rename_to_the_name_of_an_anchor_is_refused() {
    let name = "wardley-rename-identity";
    let folder = fixture_folder(name);
    let fixture = read_json(&folder.join("fixture.json"));
    let (binding, _) = load_binding(&fixture, &folder).unwrap();
    let input = fs::read(folder.join("annotations-and-labels.adp")).unwrap();
    let mut subject = Subject::open(input, "annotations-and-labels.adp", binding, &folder, None);
    let rename = read_change(
        &serde_json::json!({ "set": {
            "element": "avuvz18xgys3ql9t62tp3rghb",
            "attributes": { "name": "Customer" }
        }}),
        None,
    )
    .unwrap();

    let result = subject.change(&rename);

    assert!(matches!(result, PlanResult::Refused(_)), "{result:?}");
}

/// The documents a host must refuse for naming an origin the binding does not claim (FBL 0.10 section 14.1).
#[test]
fn a_binding_that_names_an_origin_it_does_not_claim_is_refused() {
    for name in ["origins-not-claimed.fbl", "resource-by-origin-not-claimed.fbl"] {
        let path = vendor().join("specifications/fbl/refused").join(name);
        let (problems, _) = documents::load_file(&path);
        assert!(
            problems
                .iter()
                .any(|p| p.severity == ProblemSeverity::Error
                    && p.message.contains("is not one of the binding's claims")),
            "{name}: {problems:?}"
        );
    }
}

/// The documents a host must refuse for what a term or a number reads through (FBL 0.15 section 14.1).
#[test]
fn a_binding_whose_term_has_nothing_to_read_through_is_refused() {
    for name in ["expand-without-prefixes.fbl", "ordinal-unknown.fbl"] {
        let path = vendor().join("specifications/fbl/refused").join(name);
        let (problems, _) = documents::load_file(&path);
        assert!(
            problems
                .iter()
                .any(|p| p.severity == ProblemSeverity::Error && p.message.contains("FBL 0.15")),
            "{name}: {problems:?}"
        );
    }
}

/// The documents a host must refuse for what they say of a new member (FBL 0.14 section 14.1).
#[test]
fn a_binding_that_says_otherwise_of_a_new_member_is_refused() {
    for name in ["insert-member-without-capture.fbl", "insert-refusals-not-member.fbl"] {
        let path = vendor().join("specifications/fbl/refused").join(name);
        let (problems, _) = documents::load_file(&path);
        assert!(
            problems
                .iter()
                .any(|p| p.severity == ProblemSeverity::Error && p.message.contains("FBL 0.14")),
            "{name}: {problems:?}"
        );
    }
}

/// The runner fails on a fixture whose reading names a typed value where a text is read (Requirement 5.5).
#[test]
fn the_runner_fails_on_an_attribute_value_of_another_type() {
    let folder = fixture_folder("timeline-written-text");
    let original = fs::read_to_string(folder.join("fixture.json")).unwrap();
    let changed = original.replace("\"label\": \"yes\"", "\"label\": true");
    assert_ne!(original, changed);
    let fixture: Json = serde_json::from_str(&changed).unwrap();
    let failure = run("timeline-written-text", &folder, &fixture).unwrap_err();
    assert!(failure.contains("'label'"), "{failure}");
}

fn binding_and_input(name: &str) -> (Rc<FblBinding>, Vec<u8>, String, Json) {
    let folder = fixture_folder(name);
    let fixture = read_json(&folder.join("fixture.json"));
    let (binding, _) = load_binding(&fixture, &folder).unwrap();
    let input_name = fixture["input"].as_str().unwrap().to_owned();
    let input = fs::read(folder.join(&input_name)).unwrap();
    (binding, input, input_name, fixture)
}

/// The byte-coverage invariant of FBL section 4.1: every byte of the input belongs to the reading.
#[test]
fn every_byte_of_the_input_belongs_to_the_reading() {
    let mut failures = Vec::new();
    for name in STANDALONE_FIXTURES {
        let (binding, input, input_name, _) = binding_and_input(name);
        if input_name.to_lowercase().ends_with(".adp") {
            continue;
        }
        let options = FblOptions {
            file_name: input_name,
            ..FblOptions::default()
        };
        let reading = BodyReading::read(input, binding, Rc::new(options));
        if let Some(message) = reading.unreadable_message() {
            failures.push(format!("{name}: unreadable: {message}"));
        } else if !reading.unaccounted().is_empty() {
            failures.push(format!(
                "{name}: bytes outside the reading: {:?}",
                reading.unaccounted()
            ));
        }
    }
    for name in KNOWLEDGE_FIXTURES {
        let (_, body, input) = open_knowledge(name);
        let reading = body.reading();
        if let Some(message) = reading.unreadable_message() {
            failures.push(format!("{name}: unreadable: {message}"));
        } else if !reading.unaccounted().is_empty() {
            failures.push(format!(
                "{name}: bytes outside the reading: {:?}",
                reading.unaccounted()
            ));
        }
        assert!(!input.is_empty());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// An unchanged save of the input plans no splice and keeps its bytes (FBL section 15.3).
#[test]
fn an_unchanged_save_of_the_input_plans_no_splice_and_keeps_its_bytes() {
    let mut failures = Vec::new();
    for name in STANDALONE_FIXTURES {
        let (binding, input, input_name, _) = binding_and_input(name);
        let mut subject = Subject::open(input.clone(), &input_name, binding, &fixture_folder(name), None);
        match subject.change(&ModelChange::Save) {
            PlanResult::Planned(edit) if edit.splices.is_empty() && subject.bytes() == input => {}
            other => failures.push(format!("{name}: {other:?}")),
        }
    }
    for name in KNOWLEDGE_FIXTURES {
        let (_, mut body, input) = open_knowledge(name);
        match body.change(&ModelChange::Save) {
            PlanResult::Planned(edit) if edit.splices.is_empty() && body.bytes() == input => {}
            other => failures.push(format!("{name}: {other:?}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---- the departures (InsertEmptyFixtures.Tests.cs) ----

/// One fixture's body, opened through the binding beside it, and the fixture's steps.
struct Departure {
    steps: Vec<Json>,
    body: OpenBody,
}

impl Departure {
    fn new(name: &str) -> Departure {
        let (binding, input, input_name, fixture) = binding_and_input(name);
        let options = FblOptions {
            file_name: input_name,
            ..FblOptions::default()
        };
        Departure {
            steps: fixture["steps"].as_array().unwrap().clone(),
            body: OpenBody::open(input, binding, Rc::new(options)),
        }
    }

    fn expected(&self, step: usize) -> &Json {
        &self.steps[step - 1]
    }

    fn expected_splices(&self, step: usize) -> Vec<Splice> {
        read_splices(self.expected(step))
    }

    fn expected_document(&self, step: usize) -> Vec<u8> {
        self.expected(step)["expect"].as_str().unwrap().as_bytes().to_vec()
    }

    /// Plans and applies the add of a step, which must be planned, and returns its splices.
    fn add(&mut self, step: usize) -> Vec<Splice> {
        let add = &self.expected(step)["edit"]["add"];
        let attributes: Map = add
            .get("attributes")
            .and_then(Json::as_object)
            .map(|o| o.iter().map(|(k, v)| (k.clone(), json_value(v))).collect())
            .unwrap_or_default();
        let change = ModelChange::Add {
            type_name: add["type"].as_str().unwrap().to_owned(),
            id: add.get("id").and_then(Json::as_str).map(str::to_owned),
            attributes,
            parent_id: None,
            index: None,
            slot: None,
        };
        match self.body.change(&change) {
            PlanResult::Planned(edit) => edit.splices,
            other => panic!("step {step} was not planned: {other:?}"),
        }
    }

    fn undo(&mut self) -> Vec<Splice> {
        match self.body.undo() {
            UndoResult::Done(splices) => splices,
            UndoResult::Refused(reason) => panic!("the undo was refused: {reason}"),
        }
    }

    /// Runs a step the family already passes, and compares its splices and its document with the fixture's.
    fn step_as_the_fixture(&mut self, step: usize) {
        let actual = if self.expected(step).get("undo").is_some() {
            self.undo()
        } else {
            self.add(step)
        };
        assert_eq!(self.expected_splices(step), actual);
        assert_eq!(self.expected_document(step), self.body.bytes());
    }
}

fn splice(operation: Operation, start: usize, end: usize, text: &str) -> Splice {
    Splice {
        operation,
        start,
        end,
        text: text.to_owned(),
    }
}

/// The keys a new yaml or json entry's text names, in order.
/// `(?:^|[\s{,])(?:"(?<key>[a-z]+)"|(?<key>[a-z]+)): `, the standalone host's pattern.
fn keys(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let boundary = i == 0 || chars[i - 1].is_whitespace() || chars[i - 1] == '{' || chars[i - 1] == ',';
        if boundary {
            let quoted = chars[i] == '"';
            let mut j = if quoted { i + 1 } else { i };
            let start = j;
            while j < chars.len() && chars[j].is_ascii_lowercase() {
                j += 1;
            }
            if j > start {
                let mut k = j;
                if quoted {
                    if chars.get(k) != Some(&'"') {
                        i += 1;
                        continue;
                    }
                    k += 1;
                }
                if chars.get(k) == Some(&':') && chars.get(k + 1) == Some(&' ') {
                    found.push(chars[start..j].iter().collect());
                    i = k + 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    found
}

#[test]
fn the_yaml_family_departs_from_the_fixture_only_as_recorded() {
    let mut run = Departure::new("insert-empty-yaml");

    // Step 1: FBL 0.6 refuses an add that gives refused (empty refuse) its empty value. Today it is planned, and
    // every empty value but kept's (empty keep) is left out.
    assert!(run.expected(1).get("refused").is_some());
    assert_eq!(
        run.add(1),
        vec![splice(
            Operation::InsertEntry,
            100,
            100,
            "  - id: c2\r\n    kept: \"\"\r\n"
        )]
    );
    run.undo();

    // Step 2: FBL 0.6 writes undeclared (no empty, so keep, the default for a string) with the empty string. Today
    // it is left out, because the family keeps an empty value only under an explicit empty keep.
    let second = run.add(2);
    assert_eq!(second.len(), 1);
    let expected = run.expected_splices(2);
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].text.replace("    undeclared: \"\"\r\n", ""), second[0].text);
    assert_ne!(expected[0].text, second[0].text);
    assert_eq!(
        (expected[0].operation, expected[0].start, expected[0].end),
        (second[0].operation, second[0].start, second[0].end)
    );

    // Step 3: the undo takes back what step 2 wrote, which is shorter than the fixture's, and leaves its document.
    assert_eq!(run.undo(), vec![splice(Operation::InsertEntry, 100, 126, "")]);
    assert_eq!(run.expected_document(3), run.body.bytes());

    // Steps 4 and 5: an add that gives no attribute, and its undo, are the fixture's.
    run.step_as_the_fixture(4);
    run.step_as_the_fixture(5);
}

#[test]
fn the_json_family_departs_from_the_fixture_only_as_recorded() {
    let mut run = Departure::new("insert-empty-json");

    // Step 1: FBL 0.6 refuses an add that gives refused its empty value. Today it is planned, with every value.
    assert!(run.expected(1).get("refused").is_some());
    assert_eq!(
        run.add(1),
        vec![splice(
            Operation::InsertEntry,
            155,
            155,
            ",\r\n    { \"id\": \"c2\", \"kept\": \"\", \"removed\": \"\", \"refused\": \"\", \"undeclared\": \"\" }"
        )]
    );
    run.undo();

    // Step 2: FBL 0.6 leaves removed (empty remove) out. Today it is written, and the new entry is written on one
    // line where the fixture expects it on several (not D9: the layout of a new json entry).
    let second = run.add(2);
    assert_eq!(
        second,
        vec![splice(
            Operation::InsertEntry,
            155,
            155,
            ",\r\n    { \"id\": \"c2\", \"kept\": \"\", \"removed\": \"\", \"undeclared\": \"\" }"
        )]
    );
    assert_ne!(run.expected_splices(2), second);

    // Step 3: the undo takes back what step 2 wrote and leaves the fixture's document.
    assert_eq!(run.undo(), vec![splice(Operation::InsertEntry, 155, 221, "")]);
    assert_eq!(run.expected_document(3), run.body.bytes());

    // Step 4: the keys are the fixture's (the id alone); only the one-line layout departs.
    let fourth = run.add(4);
    assert_eq!(
        fourth,
        vec![splice(Operation::InsertEntry, 155, 155, ",\r\n    { \"id\": \"c3\" }")]
    );
    assert_eq!(keys(&run.expected_splices(4)[0].text), keys(&fourth[0].text));

    // Step 5: the undo leaves the fixture's document.
    assert_eq!(run.undo(), vec![splice(Operation::InsertEntry, 155, 176, "")]);
    assert_eq!(run.expected_document(5), run.body.bytes());
}

/// D9 itself: the same add (step 2 of each fixture) gives the two families different keys, where the fixtures give
/// both the same: id, kept and undeclared.
#[test]
fn the_two_families_write_different_keys_for_the_same_add() {
    let mut yaml = Departure::new("insert-empty-yaml");
    let mut json = Departure::new("insert-empty-json");
    assert_eq!(yaml.expected(2)["edit"], json.expected(2)["edit"]);
    let yaml_keys = keys(&yaml.add(2)[0].text);
    let json_keys = keys(&json.add(2)[0].text);
    assert_eq!(yaml_keys, ["id", "kept"]);
    assert_eq!(json_keys, ["id", "kept", "removed", "undeclared"]);
    assert_eq!(keys(&yaml.expected_splices(2)[0].text), ["id", "kept", "undeclared"]);
    assert_eq!(keys(&json.expected_splices(2)[0].text), ["id", "kept", "undeclared"]);
}

// ---- the Knowledge designer's fixtures (KnowledgeFixtures.Tests.cs, KnowledgeFixtures.Steps.Tests.cs) ----

fn knowledge_options(file_name: String) -> Rc<FblOptions> {
    Rc::new(FblOptions {
        file_name,
        derive_id: Some(Rc::new(knowledge_id)),
        attribute_type: Some(type_of(knowledge_types())),
        ..FblOptions::default()
    })
}

fn open_knowledge(name: &str) -> (Json, OpenBody, Vec<u8>) {
    let folder = fixture_folder(name);
    let fixture = read_json(&folder.join("fixture.json"));
    let (binding, _) = load_binding(&fixture, &folder).unwrap();
    let input_name = fixture["input"].as_str().unwrap();
    let input = fs::read(folder.join(input_name)).unwrap();
    let file_name = Path::new(input_name)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let body = OpenBody::open(input.clone(), binding, knowledge_options(file_name));
    (fixture, body, input)
}

#[test]
fn the_knowledge_fixtures_are_read_as_they_say() {
    let mut failures = Vec::new();
    for name in KNOWLEDGE_FIXTURES {
        let (fixture, body, _) = open_knowledge(name);
        let model = body.model();
        let read = &fixture["read"];
        if model.unreadable {
            failures.push(format!("{name}: unreadable"));
            continue;
        }
        for element in read["elements"].as_array().into_iter().flatten() {
            let id = element["id"].as_str().unwrap();
            match model.find(id) {
                None => failures.push(format!("{name}: no element '{id}' was read.")),
                Some(found) if Some(found.type_name.as_str()) != element["type"].as_str() => {
                    failures.push(format!("{name}: '{id}' is a {}.", found.type_name));
                }
                _ => {}
            }
        }
        // The findings it lists and no others: a clean file is read without one.
        let mut expected: Vec<(String, usize)> = read
            .get("findings")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
            .map(|f| {
                (
                    f["rule"].as_str().unwrap().to_owned(),
                    f["line"].as_u64().unwrap() as usize,
                )
            })
            .collect();
        let mut actual: Vec<(String, usize)> = model
            .findings
            .iter()
            .map(|f| (f.code.clone(), f.location.as_ref().map(|l| l.line).unwrap_or(0)))
            .collect();
        expected.sort();
        actual.sort();
        if expected != actual {
            failures.push(format!(
                "{name}: findings {actual:?}, where the fixture lists {expected:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A model as what a format must not change: each element's id, type, parent, slot and values, a value compared by
/// its written form.
fn shape(model: &FblModel) -> Vec<String> {
    model
        .elements
        .iter()
        .map(|e| {
            let mut attributes: Vec<(&String, &Value)> = e.attributes.iter().filter(|(_, v)| !v.is_null()).collect();
            attributes.sort_by(|a, b| a.0.cmp(b.0));
            let values: Vec<String> = attributes
                .iter()
                .map(|(k, v)| {
                    let text = match v {
                        Value::Bool(b) => b.to_string(),
                        Value::Int(i) => i.to_string(),
                        Value::Double(d) => d.to_string(),
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    format!("{k}={text}")
                })
                .collect();
            format!(
                "{} {} in {}/{}: {}",
                e.id,
                e.type_name,
                e.parent_id.as_deref().unwrap_or("-"),
                e.parent_slot.as_deref().unwrap_or("-"),
                values.join(", ")
            )
        })
        .collect()
}

#[test]
fn the_same_table_in_yaml_json_and_xml_is_one_model() {
    let yaml = shape(&open_knowledge("knowledge-equivalence/yaml").1.model());
    let json = shape(&open_knowledge("knowledge-equivalence/json").1.model());
    let xml = shape(&open_knowledge("knowledge-equivalence/xml").1.model());
    assert!(!yaml.is_empty());
    assert_eq!(yaml, json);
    assert_eq!(yaml, xml);
}

/// A fixture's move index as `ModelChange::Move` counts it: a fixture counts among the siblings once the element is
/// taken out (FBL section 11.2); the library counts among them as they are before the move, the element included.
fn index_before_the_move(model: &FblModel, id: &str, index_after_removal: i64) -> Result<i64, String> {
    let element = model
        .find(id)
        .ok_or_else(|| format!("The fixture moves '{id}', which the model does not have."))?;
    let siblings: Vec<&adp_fbl::FblElement> = model
        .elements
        .iter()
        .filter(|o| {
            o.parent_id == element.parent_id
                && o.parent_slot == element.parent_slot
                && (element.parent_id.is_some() || o.type_name == element.type_name)
        })
        .collect();
    let current = siblings.iter().position(|o| o.id == id).unwrap_or(0) as i64;
    Ok(if index_after_removal >= current {
        index_after_removal + 1
    } else {
        index_after_removal
    })
}

fn read_knowledge_change(edit: &Json, model: &FblModel) -> Result<ModelChange, String> {
    if let Some(add) = edit.get("add") {
        return Ok(ModelChange::Add {
            type_name: add["type"].as_str().unwrap_or_default().to_owned(),
            id: add.get("id").and_then(Json::as_str).map(str::to_owned),
            attributes: attributes(&add["attributes"]),
            parent_id: add.get("parent").and_then(Json::as_str).map(str::to_owned),
            index: add.get("position").and_then(|p| p["index"].as_i64()),
            slot: add.get("slot").and_then(Json::as_str).map(str::to_owned),
        });
    }
    if let Some(moved) = edit.get("move") {
        let id = moved["element"].as_str().unwrap_or_default();
        return Ok(ModelChange::Move {
            id: id.to_owned(),
            new_parent_id: moved["parent"].as_str().map(str::to_owned),
            index: index_before_the_move(model, id, moved["position"]["index"].as_i64().unwrap_or(0))?,
        });
    }
    read_change(edit, Some(model))
}

/// Runs one Knowledge step, and says what went wrong, or none when it passed.
fn run_knowledge_step(step: &Json, body: &mut OpenBody, label: &str) -> Option<String> {
    let before = body.bytes().to_vec();
    let expected = in_body_order(read_splices(step));
    let actual = if step.get("undo").and_then(Json::as_bool) == Some(true) {
        match body.undo() {
            UndoResult::Done(splices) => splices,
            UndoResult::Refused(_) => return Some(format!("{label}: the undo was refused.")),
        }
    } else {
        let change = match read_knowledge_change(&step["edit"], &body.model()) {
            Ok(change) => change,
            Err(problem) => return Some(format!("{label}: {problem}")),
        };
        let result = body.change(&change);
        if let Some(refused) = step.get("refused") {
            // Refused, and nothing written. The fixture's sentence is the designer's own rule's, not compared.
            if !matches!(result, PlanResult::Refused(_)) {
                return Some(format!("{label}: planned, where the fixture refuses with {refused}."));
            }
            return (before != body.bytes()).then(|| format!("{label}: a refused edit changed the document."));
        }
        match result {
            PlanResult::Planned(edit) => edit.splices,
            PlanResult::Refused(reason) | PlanResult::Invalid(reason) => {
                return Some(format!("{label}: refused with '{reason}'."));
            }
        }
    };
    if expected != actual {
        return Some(format!(
            "{label}: expected splices\n  {}\nbut planned\n  {}",
            describe(&expected),
            describe(&actual)
        ));
    }
    // A step that names no document expects the one it started from: a save without an edit.
    let expect = step
        .get("expect")
        .and_then(Json::as_str)
        .map(|t| t.as_bytes().to_vec())
        .unwrap_or(before);
    (expect != body.bytes()).then(|| {
        format!(
            "{label}: the document differs from 'expect'.\nExpected:\n{}\nActual:\n{}",
            String::from_utf8_lossy(&expect),
            String::from_utf8_lossy(body.bytes())
        )
    })
}

#[test]
fn the_knowledge_fixtures_steps_pass_as_far_as_recorded() {
    let mut failures = Vec::new();
    for (name, passing, of) in KNOWLEDGE_STEPS {
        let folder = fixture_folder(name);
        let fixture = read_json(&folder.join("fixture.json"));
        let (binding, _) = load_binding(&fixture, &folder).unwrap();
        let input_name = fixture["input"].as_str().unwrap();
        let mut body = OpenBody::open(
            fs::read(folder.join(input_name)).unwrap(),
            binding,
            knowledge_options(input_name.to_owned()),
        );
        let steps = fixture["steps"].as_array().unwrap();
        assert_eq!(of, steps.len(), "{name} has {} steps", steps.len());
        let mut passed = 0;
        let mut failure = None;
        for step in steps {
            let label = format!("{name} step {}", passed + 1);
            failure = run_knowledge_step(step, &mut body, &label);
            if failure.is_some() {
                break;
            }
            passed += 1;
        }
        if passed != passing {
            failures.push(format!(
                "{name}: {passed} of {of} steps pass, where {passing} did. The first failure:\n{}",
                failure.unwrap_or_default()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// A planted defect in one expected splice is caught (SC-003).
#[test]
fn a_planted_defect_in_a_splice_is_caught() {
    let folder = fixture_folder("timeline-edits");
    let mut fixture = read_json(&folder.join("fixture.json"));
    let step = fixture["steps"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["splices"].as_array().is_some_and(|a| !a.is_empty()))
        .unwrap();
    step["splices"][0]["text"] = Json::String("XX".into());
    let failure = run("timeline-edits", &folder, &fixture).unwrap_err();
    assert!(failure.contains("expected splices"), "{failure}");
}

/// Every binding the fixtures above name loads without an error (FBL section 14.1), and routes its fixture's
/// input to itself (FBL section 12.3).
#[test]
fn the_bindings_load_and_route_their_fixtures_inputs() {
    let mut failures = Vec::new();
    for name in STANDALONE_FIXTURES.iter().chain(KNOWLEDGE_FIXTURES.iter()) {
        let (binding, input, input_name, _) = binding_and_input(name);
        // A registration-only binding is opened through a registration, never by routing (FBL section 12.3).
        if input_name.to_lowercase().ends_with(".adp") || binding.claims.registration_only {
            continue;
        }
        let routed = adp_fbl::routing::candidates(&input_name, &input, [binding.as_ref()]);
        if routed.is_empty() {
            failures.push(format!("{name}: {input_name} does not route to {}", binding.name));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
