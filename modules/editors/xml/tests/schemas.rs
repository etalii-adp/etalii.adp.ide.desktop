//! Checking a document against the XSD or DTD it names (etalii.adp spec 030, FR-020, tasks T024 and T032), ported
//! from the standalone host's XML editor tests at `8b6ccfc` (`SchemaValidation.Tests.cs`, `SchemaAssociation.Tests.cs`,
//! `Security.Tests.cs`). The sentences are those the standalone host's validator, .NET's `System.Xml.Schema`, gives for
//! the same documents.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use adp_editor_xml::{Analysis, ReadFile, Severity, analyse};

const EXAMPLES: &[(&str, &str)] = &[
    ("latin1.xml", include_str!("examples/latin1.xml")),
    ("library-broken.xml", include_str!("examples/library-broken.xml")),
    ("library.xml", include_str!("examples/library.xml")),
    ("library.xsd", include_str!("examples/library.xsd")),
    ("notes.xml", include_str!("examples/notes.xml")),
    ("order.xml", include_str!("examples/order.xml")),
    ("order.xsd", include_str!("examples/order.xsd")),
    ("party.xsd", include_str!("examples/party.xsd")),
    ("recipe.dtd", include_str!("examples/recipe.dtd")),
    ("recipe.xml", include_str!("examples/recipe.xml")),
];

/// The examples as a project held in memory, so a test may change a file of it, as the standalone host's
/// `ExampleProject` does with a folder of its own.
struct ExampleProject {
    files: Arc<Mutex<BTreeMap<String, String>>>,
    /// Every path the analysis asked for, read or not.
    asked: Arc<Mutex<Vec<Vec<String>>>>,
}

impl ExampleProject {
    fn new() -> Self {
        let files = EXAMPLES
            .iter()
            .map(|(name, text)| ((*name).to_owned(), text.replace("\r\n", "\n")))
            .collect();
        Self {
            files: Arc::new(Mutex::new(files)),
            asked: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn text(&self, name: &str) -> String {
        self.files.lock().unwrap()[name].clone()
    }

    fn write(&self, name: &str, text: &str) {
        self.files.lock().unwrap().insert(name.to_owned(), text.to_owned());
    }

    fn read(&self) -> ReadFile {
        let files = Arc::clone(&self.files);
        let asked = Arc::clone(&self.asked);
        Arc::new(move |path: &[String]| {
            asked.lock().unwrap().push(path.to_vec());
            files.lock().unwrap().get(&path.join("/")).cloned()
        })
    }

    /// Analyses `name` as typed in `text`, or as it is in the project.
    fn analyse(&self, name: &str, text: Option<&str>) -> Analysis {
        let text = text.map_or_else(|| self.text(name), str::to_owned);
        analyse(&text, &path(name), "UTF-8", self.read())
    }
}

fn path(name: &str) -> Vec<String> {
    name.split('/').map(str::to_owned).collect()
}

// SchemaValidation.Tests.cs

#[test]
fn the_broken_library_is_marked_where_each_mistake_is() {
    let project = ExampleProject::new();
    let text = project.text("library-broken.xml");
    let analysis = project.analyse("library-broken.xml", None);

    assert_eq!(analysis.findings.len(), 4, "{:?}", analysis.findings);
    assert!(analysis.findings.iter().all(|f| f.severity == Severity::Error));
    let find = |part: &str| {
        analysis
            .findings
            .iter()
            .find(|f| f.message.contains(part))
            .unwrap_or_else(|| panic!("no finding says {part}: {:?}", analysis.findings))
    };

    // A required attribute missing: on the start tag of the book that lacks it.
    let attribute = find("required attribute 'isbn'");
    assert_eq!(attribute.message, "The required attribute 'isbn' is missing.");
    assert_eq!(attribute.span.start, text.find("<book format").unwrap() + 1);
    assert_eq!(&text[attribute.span.clone()], "book");

    // A required child missing: on the start tag, not the end tag, of the book that lacks it, naming it.
    let child = find("incomplete content");
    assert_eq!(
        child.message,
        "The element 'book' has incomplete content. List of possible elements expected: 'author, year'."
    );
    assert_eq!(child.span.start, text.find("<book isbn=\"978-0-19").unwrap() + 1);
    assert_eq!(&text[child.span.clone()], "book");

    // A value that does not fit its type: on the value.
    let value = find("'soon'");
    assert_eq!(
        value.message,
        "The 'year' element is invalid - The value 'soon' is invalid according to its datatype 'http://www.w3.org/2001/XMLSchema:gYear' - The string 'soon' is not a valid GYear value."
    );
    assert_eq!(&text[value.span.clone()], "soon");

    // An element not allowed where it stands: on its name, saying what was expected.
    let element = find("'publisher'");
    assert_eq!(
        element.message,
        "The element 'library' has invalid child element 'publisher'. List of possible elements expected: 'book'."
    );
    assert_eq!(element.span.start, text.find("<publisher").unwrap() + 1);
    assert_eq!(&text[element.span.clone()], "publisher");
    assert_eq!(analysis.status, "Schema: library.xsd");
}

#[test]
fn an_attribute_value_that_does_not_fit_is_marked_on_the_value() {
    let project = ExampleProject::new();
    let text = project
        .text("library.xml")
        .replace("format=\"paperback\"", "format=\"vinyl\"");
    let analysis = project.analyse("library.xml", Some(&text));

    assert_eq!(analysis.findings.len(), 1, "{:?}", analysis.findings);
    let finding = &analysis.findings[0];
    assert_eq!(&text[finding.span.clone()], "vinyl");
    assert_eq!(
        finding.message,
        "The 'format' attribute is invalid - The value 'vinyl' is invalid according to its datatype 'format' - The Enumeration constraint failed."
    );
}

#[test]
fn a_text_that_is_not_well_formed_gets_no_schema_finding() {
    let project = ExampleProject::new();
    let text = project.text("library-broken.xml").replace("</library>", "</librar>");
    let analysis = project.analyse("library-broken.xml", Some(&text));

    assert_eq!(analysis.findings.len(), 1, "{:?}", analysis.findings);
    assert_eq!(analysis.findings[0].severity, Severity::Error);
    assert_eq!(
        analysis.status,
        "Not well-formed: the rest was not checked. Schema not checked."
    );
}

#[test]
fn a_schema_that_does_not_compile_is_said_once_and_gives_no_schema_finding() {
    let project = ExampleProject::new();
    project.write(
        "library.xsd",
        &project
            .text("library.xsd")
            .replace("type=\"format\"", "type=\"medium\""),
    );
    let text = project.text("library-broken.xml");
    let analysis = project.analyse("library-broken.xml", None);

    // One finding where the schema is named, naming the file and its first error.
    assert_eq!(analysis.findings.len(), 1, "{:?}", analysis.findings);
    let finding = &analysis.findings[0];
    assert_eq!(finding.severity, Severity::Info);
    assert_eq!(
        finding.message,
        "'library.xsd' does not compile: Type 'medium' is not declared, or is not a simple type. (line 41)"
    );
    assert_eq!(&text[finding.span.clone()], "library.xsd");
    assert_eq!(analysis.status, "Schema: library.xsd does not compile.");
}

#[test]
fn a_dtd_validates() {
    let project = ExampleProject::new();
    // A recipe without its title.
    let text = project.text("recipe.xml").replace("  <title>Soda bread</title>\n", "");
    let analysis = project.analyse("recipe.xml", Some(&text));

    assert_eq!(analysis.findings.len(), 1, "{:?}", analysis.findings);
    let finding = &analysis.findings[0];
    assert_eq!(finding.severity, Severity::Error);
    // The first ingredient is the element not allowed where it stands, and the sentence says the title was expected.
    assert_eq!(finding.span.start, text.find("<ingredient").unwrap() + 1);
    assert_eq!(&text[finding.span.clone()], "ingredient");
    assert_eq!(
        finding.message,
        "The element 'recipe' has invalid child element 'ingredient'. List of possible elements expected: 'title'."
    );
    assert_eq!(analysis.status, "DTD: checked, no suggestions.");
}

#[test]
fn nothing_read_is_written_back_not_even_a_default_the_schema_supplies() {
    let project = ExampleProject::new();
    project.write(
        "library.xsd",
        &project.text("library.xsd").replace(
            "<xs:attribute name=\"format\" type=\"format\"/>",
            "<xs:attribute name=\"format\" type=\"format\" default=\"paperback\"/>",
        ),
    );
    let before: BTreeMap<String, String> = project.files.lock().unwrap().clone();
    for (name, _) in EXAMPLES {
        project.analyse(name, None);
    }
    assert_eq!(*project.files.lock().unwrap(), before);
}

// SchemaAssociation.Tests.cs

#[test]
fn no_namespace_schema_location_names_the_schema_of_elements_without_a_namespace() {
    let project = ExampleProject::new();
    let analysis = project.analyse("library.xml", None);

    assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
    assert_eq!(analysis.status, "Schema: library.xsd");
    assert_eq!(analysis.depends_on, [path("library.xsd")]);
}

#[test]
fn schema_location_is_read_as_pairs_and_every_schema_applies_together() {
    let project = ExampleProject::new();
    let analysis = project.analyse("order.xml", None);

    assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
    assert_eq!(analysis.status, "Schema: urn:example:orders, urn:example:parties");
    let mut depends_on = analysis.depends_on.clone();
    depends_on.sort();
    assert_eq!(depends_on, [path("order.xsd"), path("party.xsd")]);
}

#[test]
fn schema_location_validates_the_elements_of_each_namespace() {
    let project = ExampleProject::new();
    // The party lacks its required name, which only party.xsd knows.
    let text = project
        .text("order.xml")
        .replace("    <p:name>Hendrik de Vries</p:name>\n", "");
    let analysis = project.analyse("order.xml", Some(&text));

    assert_eq!(analysis.findings.len(), 1, "{:?}", analysis.findings);
    let finding = &analysis.findings[0];
    assert_eq!(
        finding.message,
        "The element 'party' in namespace 'urn:example:parties' has invalid child element 'city' in namespace 'urn:example:parties'. List of possible elements expected: 'name' in namespace 'urn:example:parties'."
    );
    assert_eq!(&text[finding.span.clone()], "p:city");
}

#[test]
fn a_doctype_names_the_dtd_and_its_internal_subset_is_read() {
    let project = ExampleProject::new();
    // recipe.xml uses &oven;, declared in its internal subset.
    let analysis = project.analyse("recipe.xml", None);

    assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
    assert_eq!(analysis.status, "DTD: checked, no suggestions.");
    assert_eq!(analysis.depends_on, [path("recipe.dtd")]);
}

#[test]
fn a_document_that_is_a_schema_is_checked_as_one() {
    let project = ExampleProject::new();
    // A type that is declared nowhere.
    let text = project
        .text("library.xsd")
        .replace("type=\"xs:gYear\"", "type=\"year\"");
    let valid = project.analyse("library.xsd", None);
    let broken = project.analyse("library.xsd", Some(&text));

    assert!(valid.findings.is_empty(), "{:?}", valid.findings);
    assert_eq!(valid.status, "XSD: compiles.");
    assert_eq!(broken.findings.len(), 1, "{:?}", broken.findings);
    let finding = &broken.findings[0];
    assert_eq!(finding.severity, Severity::Error);
    assert_eq!(
        finding.message,
        "'library.xsd' does not compile: Type 'year' is not declared. (line 23)"
    );
    assert_eq!(finding.span.start, text.find("<xs:element name=\"year\"").unwrap() + 1);
    assert_eq!(&text[finding.span.clone()], "xs:element");
    assert_eq!(broken.status, "XSD: does not compile.");
}

#[test]
fn an_xml_model_instruction_is_said_once_as_not_supported() {
    for (namespace, language) in [
        ("http://relaxng.org/ns/structure/1.0", "RELAX NG"),
        ("http://purl.oclc.org/dsdl/schematron", "Schematron"),
    ] {
        let project = ExampleProject::new();
        let text = format!(
            "<?xml version=\"1.0\"?>\n<?xml-model href=\"notes.rng\" schematypens=\"{namespace}\"?>\n<notes/>\n"
        );
        let analysis = project.analyse("notes.xml", Some(&text));

        // One information finding where it is named, and no schema findings.
        assert_eq!(analysis.findings.len(), 1, "{:?}", analysis.findings);
        let finding = &analysis.findings[0];
        assert_eq!(finding.severity, Severity::Info);
        assert_eq!(&text[finding.span.clone()], "xml-model");
        assert_eq!(
            finding.message,
            format!(
                "An xml-model instruction names {language}, which this editor does not check: it checks XSD 1.0 and DTD named by the document."
            )
        );
        assert_eq!(analysis.status, "No schema");
    }
}

#[test]
fn an_xsd_that_declares_version_1_1_is_said_once_as_not_supported() {
    let project = ExampleProject::new();
    project.write(
        "library.xsd",
        &project.text("library.xsd").replace(
            "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">",
            "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" xmlns:vc=\"http://www.w3.org/2007/XMLSchema-versioning\" vc:minVersion=\"1.1\">",
        ),
    );
    let text = project.text("library-broken.xml");
    let analysis = project.analyse("library-broken.xml", None);

    assert_eq!(analysis.findings.len(), 1, "{:?}", analysis.findings);
    let finding = &analysis.findings[0];
    assert_eq!(finding.severity, Severity::Info);
    assert_eq!(
        finding.message,
        "'library.xsd' declares XSD 1.1, which this editor does not check: it checks XSD 1.0."
    );
    assert_eq!(&text[finding.span.clone()], "library.xsd");
    assert_eq!(analysis.status, "Schema not supported: XSD 1.1.");
}

#[test]
fn a_reference_that_cannot_be_resolved_is_one_information_finding_where_it_is_named() {
    for (reference, message) in [
        ("../../library.xsd", "'../../library.xsd' leaves the project."),
        (
            "http://example.com/library.xsd",
            "'http://example.com/library.xsd' is a web address; ADP reads schemas from the project.",
        ),
        (
            "missing.xsd",
            "'missing.xsd' is not in the project, so no schema is checked from it.",
        ),
    ] {
        let project = ExampleProject::new();
        let text = project
            .text("library-broken.xml")
            .replace("\"library.xsd\"", &format!("\"{reference}\""));
        let analysis = project.analyse("library-broken.xml", Some(&text));

        // Said where it is named, and nothing from a schema.
        assert_eq!(analysis.findings.len(), 1, "{reference}: {:?}", analysis.findings);
        let finding = &analysis.findings[0];
        assert_eq!(finding.severity, Severity::Info);
        assert_eq!(&text[finding.span.clone()], reference);
        assert_eq!(finding.message, message);
        assert_eq!(analysis.status, "No schema");
    }
}

#[test]
fn a_document_that_names_no_schema_says_so() {
    let project = ExampleProject::new();
    let analysis = project.analyse("notes.xml", None);

    assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
    assert_eq!(analysis.status, "No schema");
    assert!(analysis.depends_on.is_empty());
}

#[test]
fn a_changed_included_file_changes_the_next_answer() {
    let project = ExampleProject::new();
    // library.xsd takes its book type from an include.
    let schema = project.text("library.xsd");
    let book_start = schema.find("  <xs:complexType name=\"book\">").unwrap();
    let book_end = schema.find("</xs:schema>").unwrap();
    project.write(
        "book.xsd",
        &format!(
            "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\n{}</xs:schema>\n",
            &schema[book_start..book_end]
        ),
    );
    // An include comes before every declaration, so it goes straight after the schema's start tag.
    let opened =
        schema[schema.find("<xs:schema").unwrap()..].find('>').unwrap() + schema.find("<xs:schema").unwrap() + 1;
    project.write(
        "library.xsd",
        &format!(
            "{}\n  <xs:include schemaLocation=\"book.xsd\"/>{}{}",
            &schema[..opened],
            &schema[opened..book_start],
            &schema[book_end..]
        ),
    );
    let before = project.analyse("library.xml", None);

    // The include now requires a shelfmark nobody has.
    project.write(
        "book.xsd",
        &project.text("book.xsd").replace(
            "<xs:element name=\"lent\"",
            "<xs:element name=\"shelfmark\" type=\"xs:string\"/>\n      <xs:element name=\"lent\"",
        ),
    );
    let after = project.analyse("library.xml", None);

    assert!(before.findings.is_empty(), "{:?}", before.findings);
    assert!(before.depends_on.contains(&path("book.xsd")));
    assert!(!after.findings.is_empty());
    assert!(
        after.findings.iter().all(|f| f.message.contains("shelfmark")),
        "{:?}",
        after.findings
    );
}

// Security.Tests.cs

const CANARY: &str = "CANARY-5f0c2a-outside-the-project";

#[test]
fn a_file_outside_the_project_is_never_read() {
    for document in [
        "<!DOCTYPE d [<!ENTITY leak SYSTEM \"../canary.txt\">]>\n<d>&leak;</d>",
        "<!DOCTYPE d SYSTEM \"../canary.dtd\">\n<d>&leak;</d>",
    ] {
        let project = ExampleProject::new();
        // What a path that left the project would find, were it ever asked for.
        project.write("../canary.txt", CANARY);
        project.write("../canary.dtd", &format!("<!ENTITY leak \"{CANARY}\">"));
        let analysis = project.analyse("d.xml", Some(document));

        // The reference is refused by name, and the canary's content is nowhere.
        let said: Vec<&str> = analysis.findings.iter().map(|f| f.message.as_str()).collect();
        assert!(!said.iter().any(|m| m.contains(CANARY)) && !analysis.status.contains(CANARY));
        assert!(said.iter().any(|m| m.contains("leaves the project")), "{said:?}");
        assert!(
            project
                .asked
                .lock()
                .unwrap()
                .iter()
                .all(|p| !p.iter().any(|s| s == "..")),
            "{:?}",
            project.asked.lock().unwrap()
        );
    }
}

#[test]
fn a_document_of_nested_entities_is_refused_in_under_a_second() {
    // Each level ten of the one below; six levels expand to three million characters.
    let mut entities = String::from("<!ENTITY l0 \"lol\">");
    for level in 1..=6 {
        entities.push_str(&format!(
            "<!ENTITY l{level} \"{}\">",
            format!("&l{};", level - 1).repeat(10)
        ));
    }
    let document = format!("<!DOCTYPE bomb [{entities}]>\n<bomb>&l6;</bomb>");
    let project = ExampleProject::new();
    let started = Instant::now();
    let analysis = project.analyse("bomb.xml", Some(&document));

    assert!(
        analysis.findings.iter().any(|f| f.severity == Severity::Error),
        "{:?}",
        analysis.findings
    );
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "refusing took {:?}",
        started.elapsed()
    );
}

// The sentences .NET's validator gives, which the standalone host shows as they are, for findings its examples do
// not have (recorded with .NET 10's System.Xml.Schema on 2026-10-22).

#[test]
fn an_xsd_finding_is_said_as_the_standalone_host_says_it() {
    let project = ExampleProject::new();
    project.write(
        "types.xsd",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:simpleType name="pat"><xs:restriction base="xs:string"><xs:pattern value="[a-z]+"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="short3"><xs:restriction base="xs:string"><xs:maxLength value="3"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="small"><xs:restriction base="xs:int"><xs:maxInclusive value="3"/></xs:restriction></xs:simpleType>
  <xs:element name="r">
    <xs:complexType>
      <xs:sequence>
        <xs:element name="eint" type="xs:int"/>
        <xs:element name="elong" type="xs:long"/>
        <xs:element name="einteger" type="xs:integer"/>
        <xs:element name="ebyte" type="xs:byte"/>
        <xs:element name="efloat" type="xs:float"/>
        <xs:element name="eboolean" type="xs:boolean"/>
        <xs:element name="p" type="pat"/>
        <xs:element name="s" type="short3"/>
        <xs:element name="m" type="small"/>
        <xs:element name="empty"><xs:complexType/></xs:element>
      </xs:sequence>
      <xs:attribute name="a" type="xs:int"/>
    </xs:complexType>
  </xs:element>
</xs:schema>
"#,
    );
    let text = "<r xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:noNamespaceSchemaLocation=\"types.xsd\" a=\"zz\" b=\"1\">\n<eint>@1</eint>\n<elong>@2</elong>\n<einteger>@3</einteger>\n<ebyte>@4</ebyte>\n<efloat>@5</efloat>\n<eboolean>@6</eboolean>\n<p>A1</p><s>abcd</s><m>9</m><empty>hi</empty><zz/></r>\n";
    let analysis = project.analyse("types.xml", Some(text));
    let said: Vec<(&str, &str)> = analysis
        .findings
        .iter()
        .map(|f| (&text[f.span.clone()], f.message.as_str()))
        .collect();
    let xs = "http://www.w3.org/2001/XMLSchema";
    let expected = [
        ("zz", format!("The 'a' attribute is invalid - The value 'zz' is invalid according to its datatype '{xs}:int' - The string 'zz' is not a valid Int32 value.")),
        ("b", "The 'b' attribute is not declared.".to_owned()),
        ("@1", format!("The 'eint' element is invalid - The value '@1' is invalid according to its datatype '{xs}:int' - The string '@1' is not a valid Int32 value.")),
        ("@2", format!("The 'elong' element is invalid - The value '@2' is invalid according to its datatype '{xs}:long' - The string '@2' is not a valid Int64 value.")),
        ("@3", format!("The 'einteger' element is invalid - The value '@3' is invalid according to its datatype '{xs}:integer' - The string '@3' is not a valid Integer value.")),
        ("@4", format!("The 'ebyte' element is invalid - The value '@4' is invalid according to its datatype '{xs}:byte' - The string '@4' is not a valid SByte value.")),
        ("@5", format!("The 'efloat' element is invalid - The value '@5' is invalid according to its datatype '{xs}:float' - The string '@5' is not a valid Single value.")),
        ("@6", format!("The 'eboolean' element is invalid - The value '@6' is invalid according to its datatype '{xs}:boolean' - The string '@6' is not a valid Boolean value.")),
        ("A1", "The 'p' element is invalid - The value 'A1' is invalid according to its datatype 'pat' - The Pattern constraint failed.".to_owned()),
        ("abcd", "The 's' element is invalid - The value 'abcd' is invalid according to its datatype 'short3' - The actual length is greater than the MaxLength value.".to_owned()),
        ("9", "The 'm' element is invalid - The value '9' is invalid according to its datatype 'small' - The MaxInclusive constraint failed.".to_owned()),
        ("hi", "The element cannot contain text. Content model is empty.".to_owned()),
        ("zz", "The element 'r' has invalid child element 'zz'.".to_owned()),
    ];
    let expected: Vec<(&str, &str)> = expected.iter().map(|(at, m)| (*at, m.as_str())).collect();
    assert_eq!(said, expected);

    let root = project.analyse(
        "z.xml",
        Some(
            "<z xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:noNamespaceSchemaLocation=\"types.xsd\"/>",
        ),
    );
    assert_eq!(root.findings.len(), 1, "{:?}", root.findings);
    assert_eq!(root.findings[0].message, "The 'z' element is not declared.");
}

#[test]
fn a_dtd_finding_is_said_as_the_standalone_host_says_it() {
    let project = ExampleProject::new();
    let subset = "<!DOCTYPE r [<!ELEMENT r (a, (b|c)+, d?)><!ATTLIST r need CDATA #REQUIRED><!ELEMENT a EMPTY><!ELEMENT b (#PCDATA)><!ELEMENT c (#PCDATA|a)*><!ELEMENT d (a)>]>\n";
    let said = |body: &str| -> Vec<(String, String)> {
        let text = format!("{subset}{body}");
        project
            .analyse("d.xml", Some(&text))
            .findings
            .iter()
            .map(|f| (text[f.span.clone()].to_owned(), f.message.clone()))
            .collect()
    };
    let pairs = |expected: &[(&str, &str)]| -> Vec<(String, String)> {
        expected
            .iter()
            .map(|(a, m)| ((*a).to_owned(), (*m).to_owned()))
            .collect()
    };

    assert_eq!(
        said("<r x=\"1\"><a>t</a></r>"),
        pairs(&[
            (
                "r",
                "The element 'r' has incomplete content. List of possible elements expected: 'b c'."
            ),
            ("r", "The required attribute 'need' is missing."),
            ("x", "The 'x' attribute is not declared."),
            ("t", "The element cannot contain text. Content model is empty."),
        ])
    );
    assert_eq!(
        said("<r need=\"1\"><a><b/></a><b/><d>txt<a/></d><q/></r>"),
        pairs(&[
            (
                "b",
                "The element 'a' cannot contain child element 'b' because the parent element's content model is empty."
            ),
            (
                "txt",
                "The element 'd' cannot contain text. List of possible elements expected: 'a'."
            ),
            ("q", "The 'q' element is not declared."),
            ("q", "The element 'r' has invalid child element 'q'."),
        ])
    );
    assert_eq!(
        said("<r need=\"1\"><a/><c>x<b/></c></r>"),
        pairs(&[(
            "b",
            "The element 'c' has invalid child element 'b'. List of possible elements expected: 'a'."
        )])
    );
}
