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
//! which clears the references to a removed element. None of them is an edit to a fixture.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;

use adp_cel::{Map, Value};
use adp_fbl::documents::{self, FblBinding, ProblemSeverity};
use adp_fbl::{
    BodyReading, FblModel, FblOptions, IdRequest, ModelChange, OpenBody, OpenRegistration, Operation, PlanResult,
    PlannedEdit, Splice, UndoResult,
};
use serde_json::Value as Json;

/// The fixtures the standalone host vendored at 8b6ccfc under `Conformance/fixtures`, which its
/// `ConformanceFixturesTests` runs.
const STANDALONE_FIXTURES: [&str; 15] = [
    "agent-activity-edits",
    "agent-activity-new",
    "agent-activity-read",
    "causal-loop-rename",
    "databricks-job-dependencies",
    "databricks-pipeline-json",
    "dependency-graph-empty-flow-sequence",
    "insert-empty-json",
    "insert-empty-yaml",
    "mindmap-nodes",
    "registration-layout",
    "structurizr-open-block",
    "timeline-bom-lf",
    "timeline-edits",
    "timeline-written-text",
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

/// The fixtures of FBL 0.7 features the standalone host does not have at 8b6ccfc, so neither has this port: the
/// markdown-backed families of the bullets, behavior-tree and hierarchical-requirement tool types, and the
/// registration's own blocks (FBL 0.7 sections 7.1 and 8.1). Their bindings are FBL 0.7 documents whose bodies no
/// family of the library reads. They are out of scope here, not failing: etalii.adp's splice replay
/// (`fixtures.rs`) still runs every one of them. Beside them, the fixtures of FBL 0.8's constants an insert writes (section
/// 6.2), which this port does not have yet either.
const NOT_SUPPORTED: [&str; 33] = [
    "behavior-tree-read",
    "behavior-tree-read/first-add",
    "bullets-edits",
    "bullets-edits/no-section",
    "bullets-read",
    "hierarchical-requirement-assessments",
    "hierarchical-requirement-assessments/first-assessment",
    "hierarchical-requirement-category-state",
    "hierarchical-requirement-category-state/list",
    "hierarchical-requirement-collapsed",
    "hierarchical-requirement-collapsed/stale",
    "hierarchical-requirement-collapsed/unknown-block",
    "hierarchical-requirement-edits",
    "hierarchical-requirement-edits/no-mark",
    "hierarchical-requirement-edits/numbering",
    "hierarchical-requirement-faults",
    "hierarchical-requirement-faults/no-envelope",
    "hierarchical-requirement-faults/not-utf8",
    "hierarchical-requirement-filters",
    "hierarchical-requirement-new",
    "hierarchical-requirement-read",
    "hierarchical-requirement-read/no-section",
    "hierarchical-requirement-references",
    "hierarchical-requirement-references/deleted",
    "hierarchical-requirement-structure",
    "hierarchical-requirement-structure/no-section",
    "hierarchical-requirement-tag-list",
    "hierarchical-requirement-tags",
    "insert-constant-json",
    "insert-constant-json-read",
    "insert-constant-yaml",
    "insert-constant-yaml-read",
    "registration-blocks",
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
        ("cld", "variable") => format!("variable:{}", a("name")),
        ("cld", "link") => format!("link:{}|{}", a("causeName"), a("effectName")),
        ("cld", "loop") => format!("loop:{}", a("identifier")),
        ("job", "task") => format!("task:{}", a("key")),
        ("job", "cluster") => format!("cluster:{}", a("key")),
        ("job", "dependency") => format!("edge:{}->{}", unprefixed(request.source), unprefixed(request.target)),
        ("settings", "pipeline") => "pipeline".into(),
        ("settings", "library") => format!("library:{}", a("path")),
        ("freeplane", "branch") => format!(
            "branch:{}->{}",
            request.source.unwrap_or_default(),
            request.target.unwrap_or_default()
        ),
        ("workspace", "relationship") => format!(
            "{}->{}",
            request.source.unwrap_or_default(),
            request.target.unwrap_or_default()
        ),
        // The agent activity diagram (DISL 0.4, persistence.view.bind): the root is the diagram, and a locked
        // position and a group state are view data, keyed by the element they name.
        ("aad", "diagram") => "diagram".into(),
        ("aad", "placement") => format!("pinned:{}", a("element")),
        ("aad", "group") => format!("groups:{}#{}", a("element"), a("group")),
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
        _ => Value::Null,
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
    Registration(Box<OpenRegistration>),
}

impl Subject {
    fn open(input: Vec<u8>, name: &str, binding: Rc<FblBinding>) -> Subject {
        if name.to_lowercase().ends_with(".adp") {
            return Subject::Registration(Box::new(OpenRegistration::open(input)));
        }
        let binding_name = binding.name.clone();
        let derive_name = binding_name.clone();
        let options = FblOptions {
            file_name: name.to_owned(),
            derive_id: Some(Rc::new(move |request: &IdRequest<'_>| {
                natural_id(&derive_name, request)
            })),
            time_attributes: time_attributes(&binding_name),
            attribute_type: (binding_name == "timeline").then(|| type_of(timeline_types())),
            ..FblOptions::default()
        };
        Subject::Body {
            body: Box::new(OpenBody::open(input, binding.clone(), Rc::new(options))),
            binding,
        }
    }

    fn bytes(&self) -> Vec<u8> {
        match self {
            Subject::Body { body, .. } => body.bytes().to_vec(),
            Subject::Registration(registration) => registration.bytes().to_vec(),
        }
    }

    fn model(&self) -> Option<FblModel> {
        match self {
            Subject::Body { body, .. } => Some(body.model()),
            Subject::Registration(_) => None,
        }
    }

    fn change(&mut self, change: &ModelChange) -> PlanResult {
        match self {
            Subject::Registration(registration) => registration.change(change),
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
            Subject::Registration(registration) => registration.undo(),
        }
    }

    fn redo(&mut self) -> UndoResult {
        match self {
            Subject::Body { body, .. } => body.redo(),
            Subject::Registration(registration) => registration.redo(),
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

fn read_change(edit: &Json) -> Result<ModelChange, String> {
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
        return Ok(ModelChange::Add {
            type_name: add["type"].as_str().unwrap_or_default().to_owned(),
            id: add.get("id").and_then(Json::as_str).map(str::to_owned),
            attributes: attributes(&add["attributes"]),
            parent_id: add.get("parent").and_then(Json::as_str).map(str::to_owned),
            index: None,
            slot: None,
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

/// Each attribute a fixture's element names has that value and that JSON type, a string for a text (FBL 0.5
/// section 15.3); an attribute it does not name is not compared.
fn check_attributes(element: &adp_fbl::FblElement, attributes: &Json, name: &str) -> Result<(), String> {
    for (attribute, expected) in attributes.as_object().into_iter().flatten() {
        let label = format!("{name}: '{attribute}' of '{}'", element.id);
        let Some(actual) = element.attributes.get(attribute) else {
            return Err(format!("{label} was not read."));
        };
        let same = match expected {
            Json::String(s) => actual.as_str() == Some(s.as_str()),
            Json::Number(n) => actual.as_number().is_some_and(|a| Some(a) == n.as_f64()),
            Json::Bool(b) => matches!(actual, Value::Bool(x) if x == b),
            Json::Null => actual.is_null(),
            _ => false,
        };
        if !same {
            return Err(format!(
                "{label} is {}, where the fixture reads {expected}.",
                written(actual)
            ));
        }
    }
    Ok(())
}

fn check_read(model: &FblModel, read: &Json, name: &str) -> Result<(), String> {
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

/// Runs a fixture's reading and steps as `ConformanceFixturesTests.Run` does.
fn run(name: &str, folder: &Path, fixture: &Json) -> Result<usize, String> {
    let (binding, binding_text) = load_binding(fixture, folder)?;
    let input_name = fixture["input"].as_str().unwrap_or_default();
    let input = fs::read(folder.join(input_name)).map_err(|e| e.to_string())?;
    let mut subject = Subject::open(input, input_name, binding);
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
            let result = subject.change(&read_change(&step["edit"])?);
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
        if expected != actual {
            return Err(format!(
                "{label}: expected splices\n  {}\nbut planned\n  {}",
                describe(&expected),
                describe(&actual)
            ));
        }
        let expect = match step.get("expect").and_then(Json::as_str) {
            Some(text) => text.as_bytes().to_vec(),
            None => {
                fs::read(folder.join(step["expectFile"].as_str().unwrap_or_default())).map_err(|e| e.to_string())?
            }
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
    let known: HashSet<&str> = STANDALONE_FIXTURES
        .iter()
        .chain(KNOWLEDGE_FIXTURES.iter())
        .chain(NOT_SUPPORTED.iter())
        .copied()
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
        let mut subject = Subject::open(input.clone(), &input_name, binding);
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
    read_change(edit)
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
