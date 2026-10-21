//! The IRIs the family speaks of by name (etalii.adp spec 030, task T083), ported from the standalone host's
//! `RdfVocabulary.cs`, `Owl/OwlVocabulary.cs`, `Skos/SkosVocabulary.cs` and `Shacl/ShaclVocabulary.cs` at `8b6ccfc`.

/// RDF and RDFS.
pub mod rdf {
    pub const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    pub const FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
    pub const REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
    pub const NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
    pub const LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
    pub const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
    pub const XSD_INTEGER: &str = crate::scanner::INTEGER;
    pub const XSD_DOUBLE: &str = crate::scanner::DOUBLE;
    pub const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
    pub const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
    pub const COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
    pub const RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
}

/// OWL 2 and the RDFS terms the ontology reading names.
pub mod owl {
    pub const CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
    pub const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
    pub const OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
    pub const DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
    pub const ANNOTATION_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AnnotationProperty";
    pub const NAMED_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#NamedIndividual";
    pub const ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
    pub const THING: &str = "http://www.w3.org/2002/07/owl#Thing";
    pub const NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";
    pub const RDFS_DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
    pub const RDFS_LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";

    pub const SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
    pub const DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
    pub const RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
    pub const EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
    pub const DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
    pub const ALL_DISJOINT_CLASSES: &str = "http://www.w3.org/2002/07/owl#AllDisjointClasses";
    pub const MEMBERS: &str = "http://www.w3.org/2002/07/owl#members";
    pub const INVERSE_OF: &str = "http://www.w3.org/2002/07/owl#inverseOf";

    pub const FUNCTIONAL_PROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
    pub const INVERSE_FUNCTIONAL_PROPERTY: &str = "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
    pub const TRANSITIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
    pub const SYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
    pub const ASYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AsymmetricProperty";
    pub const REFLEXIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ReflexiveProperty";
    pub const IRREFLEXIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#IrreflexiveProperty";

    pub const ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
    pub const SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
    pub const ALL_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
    pub const HAS_VALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
    pub const CARDINALITY: &str = "http://www.w3.org/2002/07/owl#cardinality";
    pub const MIN_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#minCardinality";
    pub const MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
    pub const QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#qualifiedCardinality";
    pub const MIN_QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#minQualifiedCardinality";
    pub const MAX_QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";
    pub const ON_CLASS: &str = "http://www.w3.org/2002/07/owl#onClass";
    pub const ON_DATA_RANGE: &str = "http://www.w3.org/2002/07/owl#onDataRange";

    pub const UNION_OF: &str = "http://www.w3.org/2002/07/owl#unionOf";
    pub const INTERSECTION_OF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
    pub const COMPLEMENT_OF: &str = "http://www.w3.org/2002/07/owl#complementOf";
    pub const ONE_OF: &str = "http://www.w3.org/2002/07/owl#oneOf";

    pub const DEPRECATED: &str = "http://www.w3.org/2002/07/owl#deprecated";
    pub const IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";

    pub const XSD_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema#";

    /// Whether `iri` is vocabulary rather than an ontology's own entity: OWL, RDFS, RDF or XSD.
    pub fn is_built_in(iri: &str) -> bool {
        iri.starts_with("http://www.w3.org/2002/07/owl#")
            || iri.starts_with("http://www.w3.org/2000/01/rdf-schema#")
            || iri.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
            || iri.starts_with(XSD_NAMESPACE)
    }
}

/// SKOS.
pub mod skos {
    pub const NAMESPACE: &str = "http://www.w3.org/2004/02/skos/core#";
    pub const CONCEPT: &str = "http://www.w3.org/2004/02/skos/core#Concept";
    pub const CONCEPT_SCHEME: &str = "http://www.w3.org/2004/02/skos/core#ConceptScheme";
    pub const COLLECTION: &str = "http://www.w3.org/2004/02/skos/core#Collection";
    pub const ORDERED_COLLECTION: &str = "http://www.w3.org/2004/02/skos/core#OrderedCollection";
    pub const IN_SCHEME: &str = "http://www.w3.org/2004/02/skos/core#inScheme";
    pub const TOP_CONCEPT_OF: &str = "http://www.w3.org/2004/02/skos/core#topConceptOf";
    pub const HAS_TOP_CONCEPT: &str = "http://www.w3.org/2004/02/skos/core#hasTopConcept";
    pub const BROADER: &str = "http://www.w3.org/2004/02/skos/core#broader";
    pub const NARROWER: &str = "http://www.w3.org/2004/02/skos/core#narrower";
    pub const RELATED: &str = "http://www.w3.org/2004/02/skos/core#related";
    pub const PREF_LABEL: &str = "http://www.w3.org/2004/02/skos/core#prefLabel";
    pub const ALT_LABEL: &str = "http://www.w3.org/2004/02/skos/core#altLabel";
    pub const HIDDEN_LABEL: &str = "http://www.w3.org/2004/02/skos/core#hiddenLabel";
    pub const NOTATION: &str = "http://www.w3.org/2004/02/skos/core#notation";
    pub const MEMBER: &str = "http://www.w3.org/2004/02/skos/core#member";
    pub const MEMBER_LIST: &str = "http://www.w3.org/2004/02/skos/core#memberList";

    /// The documentation properties the property grid edits, in the grid's own order.
    pub const DOCUMENTATION: [&str; 7] = [
        "http://www.w3.org/2004/02/skos/core#definition",
        "http://www.w3.org/2004/02/skos/core#scopeNote",
        "http://www.w3.org/2004/02/skos/core#example",
        "http://www.w3.org/2004/02/skos/core#note",
        "http://www.w3.org/2004/02/skos/core#historyNote",
        "http://www.w3.org/2004/02/skos/core#editorialNote",
        "http://www.w3.org/2004/02/skos/core#changeNote",
    ];

    /// The mapping properties, drawn only when both ends are in-file concepts.
    pub const MAPPINGS: [&str; 5] = [
        "http://www.w3.org/2004/02/skos/core#exactMatch",
        "http://www.w3.org/2004/02/skos/core#closeMatch",
        "http://www.w3.org/2004/02/skos/core#broadMatch",
        "http://www.w3.org/2004/02/skos/core#narrowMatch",
        "http://www.w3.org/2004/02/skos/core#relatedMatch",
    ];

    /// The SKOS-XL label properties, deliberately unread.
    pub const XL_LABEL_PROPERTIES: [&str; 3] = [
        "http://www.w3.org/2008/05/skos-xl#prefLabel",
        "http://www.w3.org/2008/05/skos-xl#altLabel",
        "http://www.w3.org/2008/05/skos-xl#hiddenLabel",
    ];
}

/// SHACL Core, and SHACL-SPARQL's `sh:sparql`.
pub mod shacl {
    pub const NAMESPACE: &str = "http://www.w3.org/ns/shacl#";

    pub const NODE_SHAPE: &str = "http://www.w3.org/ns/shacl#NodeShape";
    pub const PROPERTY_SHAPE: &str = "http://www.w3.org/ns/shacl#PropertyShape";
    pub const TARGET_CLASS: &str = "http://www.w3.org/ns/shacl#targetClass";
    pub const TARGET_NODE: &str = "http://www.w3.org/ns/shacl#targetNode";
    pub const TARGET_SUBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetSubjectsOf";
    pub const TARGET_OBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetObjectsOf";
    pub const PATH: &str = "http://www.w3.org/ns/shacl#path";
    pub const NODE: &str = "http://www.w3.org/ns/shacl#node";
    pub const PROPERTY: &str = "http://www.w3.org/ns/shacl#property";
    pub const QUALIFIED_VALUE_SHAPE: &str = "http://www.w3.org/ns/shacl#qualifiedValueShape";
    pub const AND: &str = "http://www.w3.org/ns/shacl#and";
    pub const OR: &str = "http://www.w3.org/ns/shacl#or";
    pub const XONE: &str = "http://www.w3.org/ns/shacl#xone";
    pub const NOT: &str = "http://www.w3.org/ns/shacl#not";
    pub const CLASS: &str = "http://www.w3.org/ns/shacl#class";
    pub const DATATYPE: &str = "http://www.w3.org/ns/shacl#datatype";
    pub const NODE_KIND: &str = "http://www.w3.org/ns/shacl#nodeKind";
    pub const MIN_COUNT: &str = "http://www.w3.org/ns/shacl#minCount";
    pub const MAX_COUNT: &str = "http://www.w3.org/ns/shacl#maxCount";
    pub const HAS_VALUE: &str = "http://www.w3.org/ns/shacl#hasValue";
    pub const IN: &str = "http://www.w3.org/ns/shacl#in";
    pub const CLOSED: &str = "http://www.w3.org/ns/shacl#closed";
    pub const IGNORED_PROPERTIES: &str = "http://www.w3.org/ns/shacl#ignoredProperties";
    pub const SPARQL: &str = "http://www.w3.org/ns/shacl#sparql";
    pub const SELECT: &str = "http://www.w3.org/ns/shacl#select";
    pub const DEACTIVATED: &str = "http://www.w3.org/ns/shacl#deactivated";
    pub const SEVERITY: &str = "http://www.w3.org/ns/shacl#severity";
    pub const MESSAGE: &str = "http://www.w3.org/ns/shacl#message";
    pub const NAME: &str = "http://www.w3.org/ns/shacl#name";
    pub const DESCRIPTION: &str = "http://www.w3.org/ns/shacl#description";
    pub const VIOLATION: &str = "http://www.w3.org/ns/shacl#Violation";
    pub const WARNING: &str = "http://www.w3.org/ns/shacl#Warning";
    pub const INFO: &str = "http://www.w3.org/ns/shacl#Info";
    pub const INVERSE_PATH: &str = "http://www.w3.org/ns/shacl#inversePath";
    pub const ALTERNATIVE_PATH: &str = "http://www.w3.org/ns/shacl#alternativePath";
    pub const ZERO_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrMorePath";
    pub const ONE_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#oneOrMorePath";
    pub const ZERO_OR_ONE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrOnePath";

    pub const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
    pub const RDFS_SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

    /// The predicates whose subject is a target-carrying shape.
    pub const TARGET_PREDICATES: [&str; 4] = [TARGET_CLASS, TARGET_NODE, TARGET_SUBJECTS_OF, TARGET_OBJECTS_OF];

    /// The constraint component parameters of SHACL Core, plus `sh:sparql`: a subject of any of them is a shape.
    pub const CONSTRAINT_PARAMETERS: [&str; 33] = [
        CLASS,
        DATATYPE,
        NODE_KIND,
        MIN_COUNT,
        MAX_COUNT,
        "http://www.w3.org/ns/shacl#minExclusive",
        "http://www.w3.org/ns/shacl#minInclusive",
        "http://www.w3.org/ns/shacl#maxExclusive",
        "http://www.w3.org/ns/shacl#maxInclusive",
        "http://www.w3.org/ns/shacl#minLength",
        "http://www.w3.org/ns/shacl#maxLength",
        "http://www.w3.org/ns/shacl#pattern",
        "http://www.w3.org/ns/shacl#flags",
        "http://www.w3.org/ns/shacl#languageIn",
        "http://www.w3.org/ns/shacl#uniqueLang",
        "http://www.w3.org/ns/shacl#equals",
        "http://www.w3.org/ns/shacl#disjoint",
        "http://www.w3.org/ns/shacl#lessThan",
        "http://www.w3.org/ns/shacl#lessThanOrEquals",
        NOT,
        AND,
        OR,
        XONE,
        NODE,
        PROPERTY,
        QUALIFIED_VALUE_SHAPE,
        "http://www.w3.org/ns/shacl#qualifiedMinCount",
        "http://www.w3.org/ns/shacl#qualifiedMaxCount",
        "http://www.w3.org/ns/shacl#qualifiedValueShapesDisjoint",
        CLOSED,
        IGNORED_PROPERTIES,
        HAS_VALUE,
        IN,
    ];

    /// The predicates whose single object is itself a shape.
    pub const SHAPE_EXPECTING_PREDICATES: [&str; 4] = [NODE, PROPERTY, QUALIFIED_VALUE_SHAPE, NOT];

    /// The predicates whose object is a list of shapes.
    pub const SHAPE_LIST_PREDICATES: [&str; 3] = [AND, OR, XONE];

    /// The local names of the further terms the recommendation defines in the namespace.
    const OTHER_TERMS: [&str; 46] = [
        "NodeShape",
        "PropertyShape",
        "Shape",
        "path",
        "deactivated",
        "severity",
        "message",
        "name",
        "description",
        "Violation",
        "Warning",
        "Info",
        "Severity",
        "inversePath",
        "alternativePath",
        "zeroOrMorePath",
        "oneOrMorePath",
        "zeroOrOnePath",
        "IRI",
        "BlankNode",
        "Literal",
        "BlankNodeOrIRI",
        "BlankNodeOrLiteral",
        "IRIOrLiteral",
        "order",
        "group",
        "defaultValue",
        "PropertyGroup",
        "select",
        "ask",
        "construct",
        "update",
        "prefixes",
        "declare",
        "prefix",
        "namespace",
        "SPARQLConstraint",
        "shapesGraph",
        "entailment",
        "ValidationReport",
        "conforms",
        "result",
        "ValidationResult",
        "focusNode",
        "resultPath",
        "resultMessage",
    ];

    const MORE_TERMS: [&str; 6] = [
        "resultSeverity",
        "sourceConstraint",
        "sourceConstraintComponent",
        "sourceShape",
        "value",
        "detail",
    ];

    /// Whether `iri` is a term the recommendation defines in the SHACL namespace (`ShaclVocabulary.AllTerms`).
    pub fn is_known_term(iri: &str) -> bool {
        if TARGET_PREDICATES.contains(&iri) || CONSTRAINT_PARAMETERS.contains(&iri) || iri == SPARQL {
            return true;
        }
        iri.strip_prefix(NAMESPACE)
            .is_some_and(|local| OTHER_TERMS.contains(&local) || MORE_TERMS.contains(&local))
    }

    /// Whether `iri` makes its subject a shape by use: a target or a constraint parameter, `sh:sparql` included.
    pub fn is_constraint_parameter(iri: &str) -> bool {
        CONSTRAINT_PARAMETERS.contains(&iri) || iri == SPARQL
    }
}
