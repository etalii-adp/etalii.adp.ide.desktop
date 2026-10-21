//! The metamodel of a loaded specification (DISL section 4; the standalone host's `DislMetamodel`): its attributes,
//! enumerations, node types and relation types, inheritance flattened (section 4.7), and the user function and id
//! rule declarations the rest of the runtime reads (sections 3.4 and 11.5).

use std::rc::Rc;

use adp_cel::Program;
use indexmap::IndexMap;

use crate::json::{self, Json};

/// The primitive types of DISL section 4.2.
pub const PRIMITIVES: [&str; 15] = [
    "string",
    "text",
    "int",
    "number",
    "bool",
    "date",
    "datetime",
    "yearMonth",
    "time",
    "duration",
    "color",
    "uri",
    "expression",
    "json",
    "binary",
];

/// One attribute of a node type, a relation type or the diagram (DISL section 4.3), after inheritance is flattened:
/// the declaration nearest in the type's linearisation (section 4.7).
#[derive(Debug, Clone)]
pub struct DislAttribute {
    /// The attribute's name, a simple identifier.
    pub name: String,
    /// Its type as written: a primitive (section 4.2), an enum, a data type, or a node or relation type.
    pub type_name: String,
    /// Whether it holds a list.
    pub many: bool,
    /// Its literal default, when it declares one that is not CEL.
    pub default: Option<Json>,
    /// The type whose declaration this is: the type itself, or the supertype it inherits it from.
    pub declared_by: String,
    /// The declaration itself, for what this view does not type.
    pub json: Json,
}

impl DislAttribute {
    /// Whether the attribute is computed by a `derived` expression (section 4.3).
    pub fn is_derived(&self) -> bool {
        json::has(&self.json, "derived")
    }
}

/// One value of an enumeration (DISL section 4.5): its key, which CEL sees, and its stored form, which is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DislEnumValue {
    pub key: String,
    pub stored: String,
    pub label: Option<String>,
}

/// An enumeration (DISL section 4.5), its values in declaration order, which is the display order.
#[derive(Debug, Clone)]
pub struct DislEnum {
    pub name: String,
    pub values: Vec<DislEnumValue>,
    pub ordered: bool,
}

impl DislEnum {
    /// The value whose key is `key`.
    pub fn value_of(&self, key: &str) -> Option<&DislEnumValue> {
        self.values.iter().find(|v| v.key == key)
    }

    /// The value whose stored form is `stored`.
    pub fn stored_as(&self, stored: &str) -> Option<&DislEnumValue> {
        self.values.iter().find(|v| v.stored == stored)
    }
}

/// One end of a relation type (DISL section 4.9): the types it accepts, those it excludes, and whether it may be
/// missing.
#[derive(Debug, Clone, Default)]
pub struct DislRelationEnd {
    pub types: Vec<String>,
    pub exclude: Vec<String>,
    pub optional: bool,
}

/// A node type (DISL section 4.6) or a relation type (section 4.9), inheritance flattened (section 4.7): its C3
/// linearisation, every attribute it has, and what its own declaration says.
#[derive(Debug, Clone)]
pub struct DislType {
    pub name: String,
    /// Whether this is a relation type, drawn as an edge, rather than a node type.
    pub is_relation: bool,
    /// The declaration itself, for what this view does not type.
    pub json: Json,
    /// The supertypes it names, in the order it names them.
    pub extends: Vec<String>,
    /// The type and its supertypes, most specific first: its C3 linearisation (section 4.7).
    pub linearisation: Vec<String>,
    /// Every attribute, inherited ones included, the most general type's first and each in declaration order.
    pub attributes: IndexMap<String, DislAttribute>,
    /// The types a node of this type may contain (section 4.8); empty when it contains none.
    pub child_types: Vec<String>,
    /// The attribute used as the element's name (section 4.6): declared, or the default rule's choice.
    pub label_attribute: Option<String>,
    /// A relation type's source end; none for a node type.
    pub source: Option<DislRelationEnd>,
    /// A relation type's target end; none for a node type.
    pub target: Option<DislRelationEnd>,
}

impl DislType {
    pub(crate) fn new(name: &str, is_relation: bool, json: Json) -> Self {
        DislType {
            name: name.to_owned(),
            is_relation,
            json,
            extends: Vec::new(),
            linearisation: Vec::new(),
            attributes: IndexMap::new(),
            child_types: Vec::new(),
            label_attribute: None,
            source: None,
            target: None,
        }
    }

    pub fn is_abstract(&self) -> bool {
        json::bool_of(&self.json, "abstract")
    }

    /// The `derived` declaration (section 4.11): an Expression (a relation's 0.1 form) or a DerivedNode or
    /// DerivedRelation object; none for a stored type.
    pub fn derived(&self) -> Option<&Json> {
        json::get(&self.json, "derived")
    }

    /// Whether the type is `ancestor` or one of its subtypes.
    pub fn is_a(&self, ancestor: &str) -> bool {
        self.linearisation.iter().any(|t| t == ancestor)
    }

    /// The extension properties (`x-`) of the declaration, unread and unchanged.
    pub fn extensions(&self) -> Vec<(String, Json)> {
        json::extensions(&self.json)
    }
}

/// The metamodel of a specification (DISL section 4), inheritance flattened.
#[derive(Debug, Clone)]
pub struct DislMetamodel {
    /// The attributes of the diagram root element.
    pub diagram_attributes: IndexMap<String, DislAttribute>,
    /// The enumerations, by name.
    pub enums: Rc<IndexMap<String, DislEnum>>,
    /// The node types, in declaration order.
    pub types: IndexMap<String, Rc<DislType>>,
    /// The relation types, in declaration order.
    pub relations: IndexMap<String, Rc<DislType>>,
    /// The names of the data types.
    pub data_types: Vec<String>,
}

impl DislMetamodel {
    /// The node or relation type named `name`.
    pub fn type_of(&self, name: &str) -> Option<&Rc<DislType>> {
        self.types.get(name).or_else(|| self.relations.get(name))
    }

    /// Whether `type_name` is `ancestor` or one of its subtypes (sections 2.7 and 12.2).
    pub fn is_a(&self, type_name: &str, ancestor: &str) -> bool {
        match self.type_of(type_name) {
            Some(known) => known.is_a(ancestor),
            None => type_name == ancestor,
        }
    }

    /// The types that are `ancestor` or a subtype of it, in declaration order.
    pub fn subtypes_of(&self, ancestor: &str) -> Vec<&Rc<DislType>> {
        self.types
            .values()
            .chain(self.relations.values())
            .filter(|t| t.is_a(ancestor))
            .collect()
    }

    pub fn is_data_type(&self, name: &str) -> bool {
        self.data_types.iter().any(|d| d == name)
    }
}

/// One parameter of a user function (DISL section 3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DislParameter {
    pub name: String,
    pub type_name: String,
}

/// A user function's bounded recursion (DISL section 3.4): how deep it may call itself, and what a call deeper than
/// that gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DislRecursion {
    pub max_depth: i64,
    pub at_max_depth: String,
}

/// A user function as declared (DISL section 3.4), in declaration order.
#[derive(Debug, Clone)]
pub struct DislFunctionDeclaration {
    pub name: String,
    pub parameters: Vec<DislParameter>,
    pub returns: String,
    pub cel: String,
    pub uses: Vec<String>,
    pub recursion: Option<DislRecursion>,
}

/// An id rule (DISL section 11.5): how ids are made, for every type or for one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DislIdRule {
    /// The strategy, `uuid-v7` when none is declared.
    pub strategy: String,
    /// The CEL of the `cel` and `derived` strategies; none for the others.
    pub expression: Option<String>,
    /// The prefix the rule gives the type it was asked for: the type's own rule's, else the top level's for the type
    /// or the nearest type in its linearisation; none when none is declared.
    pub prefix: Option<String>,
}

/// A compiled expression of the specification: where it is, the context it was compiled in (section 12.3), and its
/// program.
#[derive(Debug, Clone)]
pub struct DislExpression {
    pub pointer: String,
    pub context: String,
    pub program: Program,
}
