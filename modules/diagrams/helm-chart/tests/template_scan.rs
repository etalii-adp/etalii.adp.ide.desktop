//! The literal/templated boundary, pinned case by case (etalii.adp spec 030, task T080): the standalone host's
//! `TemplateScan.Tests.cs` at `8b6ccfc`, with the same expectations.

use adp_diagram_helm_chart::template_scan::scan;

#[test]
fn a_literal_kind_is_discovered() {
    let facts = scan("apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: x\n");
    assert_eq!(facts.kinds, ["Deployment"]);
    assert_eq!(facts.api_versions, ["apps/v1"]);
}

#[test]
fn a_templated_kind_is_unknown_by_design() {
    let facts = scan("kind: {{ .Values.kind }}\napiVersion: {{ include \"x.apiVersion\" . }}\n");
    // Never guessed, but the include's literal name is still a reference.
    assert!(facts.kinds.is_empty());
    assert!(facts.api_versions.is_empty());
    assert_eq!(facts.references, ["x.apiVersion"]);
}

#[test]
fn a_quoted_kind_loses_its_quotes() {
    assert_eq!(scan("kind: \"Service\"\n").kinds, ["Service"]);
}

#[test]
fn a_trailing_comment_is_not_part_of_the_value() {
    assert_eq!(scan("kind: Ingress # the optional one\n").kinds, ["Ingress"]);
}

#[test]
fn an_indented_kind_belongs_to_something_else() {
    let facts = scan("kind: ClusterRole\nrules:\n  - kind: Pod\n# kind: Commented\n");
    assert_eq!(facts.kinds, ["ClusterRole"]);
}

#[test]
fn a_multi_document_template_lists_every_kind_once_sorted() {
    let facts = scan("kind: Service\n---\nkind: Deployment\n---\nkind: Service\n");
    assert_eq!(facts.kinds, ["Deployment", "Service"]);
}

#[test]
fn a_define_block_exports_its_literal_name() {
    assert_eq!(
        scan("{{- define \"mychart.labels\" -}}\napp: x\n{{- end }}\n").defines,
        ["mychart.labels"]
    );
}

#[test]
fn include_and_template_both_reference() {
    let facts = scan("  labels: {{ include \"mychart.labels\" . | nindent 4 }}\n  {{ template \"mychart.name\" . }}\n");
    assert_eq!(facts.references, ["mychart.labels", "mychart.name"]);
}

#[test]
fn a_computed_include_name_is_no_reference() {
    assert!(
        scan("{{ include (printf \"%s.labels\" .Chart.Name) . }}\n")
            .references
            .is_empty()
    );
}

#[test]
fn prose_like_notes_yields_nothing() {
    let facts = scan("Thank you for installing {{ .Chart.Name }}.\n\nYour release is named {{ .Release.Name }}.\n");
    assert_eq!(facts, Default::default());
}

#[test]
fn several_facts_on_one_line_all_count() {
    assert_eq!(
        scan("{{ include \"a.x\" . }}{{ include \"b.y\" . }}\n").references,
        ["a.x", "b.y"]
    );
}

#[test]
fn crlf_content_scans_the_same_as_lf() {
    let lf = scan("kind: Deployment\n{{- define \"x.y\" }}\n");
    let crlf = scan("kind: Deployment\r\n{{- define \"x.y\" }}\r\n");
    assert_eq!(lf, crlf);
    assert_eq!(crlf.kinds, ["Deployment"]);
    assert_eq!(crlf.defines, ["x.y"]);
}
