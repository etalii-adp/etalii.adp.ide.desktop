//! A loaded DESL specification (DESL sections 3 to 8): its language, its metamodel, how its documents are stored and
//! how its surface lays the model out, read once into types a host goes by. What the runtime does not interpret -
//! constraints, operations, functions, documentation - stays in [`DeslSpecification::root`], the JSON as written.
//!
//! The standalone host has no DESL loader of its own: its Knowledge module reads the two things it needs from
//! `knowledge.des` directly (`KnowledgeDefinition`: the attribute types, and the labels of the value types). Those
//! readings are [`DeslSpecification::attribute_type`] and [`DeslSpecification::enum_label`] here.

use indexmap::IndexMap;

use crate::json::{self, Json};

/// The designer type's identity (DESL section 3.2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeslLanguage {
    pub id: String,
    pub version: String,
    /// `vendor/type`: the first line of every registration of the type's documents, and the `designer` of every DED
    /// definition of it.
    pub origin: String,
    /// The display name, its default locale's text; the origin when the specification gives none.
    pub label: String,
    pub icon: String,
}

/// One attribute of an element type or of the document (DESL section 4.3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeslAttribute {
    /// A primitive, an enumeration or an element type, which makes the attribute a reference (DESL section 4.7).
    pub type_name: String,
    pub required: bool,
    pub many: bool,
    pub read_only: bool,
    pub default: Option<Json>,
}

/// Where the children of an element go (DESL section 4.6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeslContainment {
    pub allowed: Vec<String>,
    pub ordered: bool,
    /// Each slot with the types it allows, in the specification's order.
    pub slots: IndexMap<String, Vec<String>>,
}

impl DeslContainment {
    /// Every type a child may be: those allowed directly and those of any slot.
    pub fn all_allowed(&self) -> impl Iterator<Item = &String> {
        self.allowed.iter().chain(self.slots.values().flatten())
    }
}

/// One kind of element (DESL section 4.5), or the document's own attributes (DESL section 4.1).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeslElementType {
    pub label: String,
    pub attributes: IndexMap<String, DeslAttribute>,
    pub children: Option<DeslContainment>,
    pub label_attribute: Option<String>,
}

/// One value of an enumeration, by its key (DESL section 4.4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeslEnumValue {
    pub key: String,
    /// The stored form: the key unless the value says otherwise.
    pub value: String,
    pub label: String,
}

/// An enumeration: its values in display order (DESL section 4.4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeslEnum {
    pub values: Vec<DeslEnumValue>,
    pub ordered: bool,
}

/// What a document holds (DESL section 4).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeslMetamodel {
    pub document: DeslElementType,
    pub types: IndexMap<String, DeslElementType>,
    pub enums: IndexMap<String, DeslEnum>,
}

/// How one type's elements get their ids (DESL section 5.3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeslIdRule {
    pub strategy: String,
    pub expression: Option<String>,
    pub ephemeral: bool,
    pub prefix: Option<String>,
    pub digits: Option<i64>,
    pub mark: Option<String>,
    pub placeholder: Option<String>,
}

/// How documents are stored (DESL section 5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeslPersistence {
    pub format: String,
    /// `<document>#<name>`, resolved against the specification's own location.
    pub bindings: Vec<String>,
    /// The top-level id rule.
    pub ids: DeslIdRule,
    /// `assign` (the default) or `ephemeral` (DESL section 5.3.4).
    pub missing: String,
    pub encoding: String,
    /// The per-type rules, each with what it leaves out taken from the top-level rule.
    pub id_rules: IndexMap<String, DeslIdRule>,
    /// Each binding type the type map names, with what it becomes (DESL section 5.2).
    pub type_map: IndexMap<String, String>,
}

/// The editor a host opens over a cell of a value type (DESL section 7.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Editor {
    Text,
    Number,
    Checkbox,
    Date,
    DateTime,
    Time,
    Choice,
    Choices,
    Rows,
}

impl Editor {
    pub fn parse(text: &str) -> Option<Editor> {
        Some(match text {
            "text" => Editor::Text,
            "number" => Editor::Number,
            "checkbox" => Editor::Checkbox,
            "date" => Editor::Date,
            "dateTime" => Editor::DateTime,
            "time" => Editor::Time,
            "choice" => Editor::Choice,
            "choices" => Editor::Choices,
            "rows" => Editor::Rows,
            _ => return None,
        })
    }

    /// Whether a value of this editor is a moment: a date, a date and time, or a time.
    pub fn is_moment(self) -> bool {
        matches!(self, Editor::Date | Editor::DateTime | Editor::Time)
    }

    /// Whether its values are the options of a column.
    pub fn is_choice(self) -> bool {
        matches!(self, Editor::Choice | Editor::Choices)
    }
}

/// How the values of a value type compare when sorted (DESL section 7.5). Empty values sort last both ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortKind {
    Text,
    Number,
    FalseFirst,
    Chronological,
    OptionOrder,
    Title,
}

impl SortKind {
    pub fn parse(text: &str) -> Option<SortKind> {
        Some(match text {
            "text" => SortKind::Text,
            "number" => SortKind::Number,
            "false-first" => SortKind::FalseFirst,
            "chronological" => SortKind::Chronological,
            "option-order" => SortKind::OptionOrder,
            "title" => SortKind::Title,
            _ => return None,
        })
    }
}

/// One value type a column may have (DESL section 7.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeslValueType {
    pub label: String,
    /// The attribute of the cells' type that holds the value or, with `many`, of the items' type that holds one.
    pub key: String,
    pub many: bool,
    pub editor: Editor,
    /// The comparisons a filter on this type offers, besides `is-empty` and `is-not-empty`, in their order.
    pub comparisons: Vec<String>,
    pub sort: SortKind,
    /// The conversion to each other value type (DESL section 7.6).
    pub converts: IndexMap<String, String>,
}

/// The type and attributes that list the values a choice may take (DESL section 7.3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeslOptions {
    pub type_name: String,
    pub name: String,
    pub colour: Option<String>,
}

/// A `table` surface (DESL sections 7.2 to 7.6), by the types and attributes that play each part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeslTable {
    pub columns_type: String,
    pub column_name: String,
    pub column_value_type: String,
    pub column_title: Option<String>,
    pub column_parent: Option<String>,
    pub options: Option<DeslOptions>,
    pub rows_type: String,
    pub cells_type: String,
    pub cell_column: String,
    pub cell_items: Option<String>,
    pub views_type: String,
    pub view_name: String,
    pub view_active: Option<String>,
    /// The element type of each role of a view's settings: `columns`, `sorts`, `conditions`, `filterGroups`, `groups`.
    pub view_settings: IndexMap<String, String>,
    /// Every value type, by its key, in the specification's order.
    pub value_types: IndexMap<String, DeslValueType>,
}

/// How a specification lays its model out (DESL section 7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeslSurface {
    Table(Box<DeslTable>),
    /// A `hierarchy` surface (DESL section 7.7). Its roles are checked when it loads; the standalone host has no
    /// designer of this kind, so its layout is not interpreted here, and it is kept as written.
    Hierarchy(Json),
}

/// A specification that loaded without errors.
#[derive(Debug, Clone)]
pub struct DeslSpecification {
    root: Json,
    /// The DESL version it is written in.
    pub(crate) version: String,
    pub language: DeslLanguage,
    pub metamodel: DeslMetamodel,
    pub persistence: DeslPersistence,
    pub surface: DeslSurface,
}

impl DeslSpecification {
    pub(crate) fn new(
        root: Json,
        language: DeslLanguage,
        metamodel: DeslMetamodel,
        persistence: DeslPersistence,
        surface: DeslSurface,
    ) -> Self {
        let version = json::string_of(&root, "desl").unwrap_or_default().to_owned();
        DeslSpecification {
            root,
            version,
            language,
            metamodel,
            persistence,
            surface,
        }
    }

    /// The specification's JSON, as it was read.
    pub fn root(&self) -> &Json {
        &self.root
    }

    /// The DESL version it is written in (DESL section 2.7).
    pub fn desl_version(&self) -> &str {
        &self.version
    }

    /// The table its surface lays out, when its kind is `table`.
    pub fn table(&self) -> Option<&DeslTable> {
        match &self.surface {
            DeslSurface::Table(table) => Some(table),
            DeslSurface::Hierarchy(_) => None,
        }
    }

    /// A value type of the table surface, by its key.
    pub fn value_type(&self, name: &str) -> Option<&DeslValueType> {
        self.table().and_then(|t| t.value_types.get(name))
    }

    /// The type the specification gives an attribute of an element type, or none where it gives none: the reading
    /// `KnowledgeDefinition.AttributeType` makes, which FBL's reading needs to tell a value that does not convert.
    pub fn attribute_type(&self, type_name: &str, attribute: &str) -> Option<&str> {
        self.metamodel
            .types
            .get(type_name)
            .and_then(|t| t.attributes.get(attribute))
            .map(|a| a.type_name.as_str())
            .filter(|t| !t.is_empty())
    }

    /// How many attributes the metamodel's element types type: none would mean it was not read.
    pub fn typed_attribute_count(&self) -> usize {
        self.metamodel
            .types
            .values()
            .flat_map(|t| t.attributes.values())
            .filter(|a| !a.type_name.is_empty())
            .count()
    }

    /// The label of an enumeration's value, by the enumeration and the key (`KnowledgeDefinition.ValueTypeLabel`).
    pub fn enum_label(&self, enumeration: &str, key: &str) -> Option<&str> {
        self.metamodel
            .enums
            .get(enumeration)
            .and_then(|e| e.values.iter().find(|v| v.key == key))
            .map(|v| v.label.as_str())
    }

    /// The id rule of a type: its own, or the top-level rule.
    pub fn id_rule(&self, type_name: &str) -> &DeslIdRule {
        self.persistence
            .id_rules
            .get(type_name)
            .unwrap_or(&self.persistence.ids)
    }
}
