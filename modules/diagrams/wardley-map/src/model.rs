//! A parsed Wardley map, in the document's own vocabulary (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host's `src/diagrams/wardley-map/backend/EtAlii.Adp.Diagram.WardleyMap/_Model` at
//! `8b6ccfc`, with the three enums its `api/wardley-map.proto` declares (`WardleyElementKind`, `WardleyDecorator`,
//! `WardleyAttitudeKind`). It is a reading, not a resolved model: links, `evolve` statements and pipelines name what
//! they refer to by name, and nothing here is checked for consistency; the rules judge that (`crate::rules`).
//! Coordinates stay in the document's `[visibility, maturity]` order; the one conversion to the canvas is
//! [`crate::axis`]'s.

/// A position as the document writes it: `[visibility, maturity]`. Visibility is drawn vertically, the user need at
/// the top; maturity horizontally, genesis at the left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WardleyCoordinate {
    pub visibility: f64,
    pub maturity: f64,
}

impl WardleyCoordinate {
    pub fn new(visibility: f64, maturity: f64) -> Self {
        WardleyCoordinate { visibility, maturity }
    }
}

/// The DSL's three positioned statement kinds; `market` and `ecosystem` read like kinds and are decorators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WardleyElementKind {
    Component,
    Anchor,
    Submap,
}

impl WardleyElementKind {
    /// The keyword the document writes.
    pub fn keyword(self) -> &'static str {
        match self {
            WardleyElementKind::Component => "component",
            WardleyElementKind::Anchor => "anchor",
            WardleyElementKind::Submap => "submap",
        }
    }
}

/// The five decorators, in the order the DSL documents them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WardleyDecorator {
    Market,
    Ecosystem,
    Build,
    Buy,
    Outsource,
}

impl WardleyDecorator {
    /// The five, in the order the DSL documents them (`WardleyDecorators.All`).
    pub const ALL: [WardleyDecorator; 5] = [
        WardleyDecorator::Market,
        WardleyDecorator::Ecosystem,
        WardleyDecorator::Build,
        WardleyDecorator::Buy,
        WardleyDecorator::Outsource,
    ];

    /// The word the document writes (`WardleyDecorators.Spell`).
    pub fn spell(self) -> &'static str {
        match self {
            WardleyDecorator::Market => "market",
            WardleyDecorator::Ecosystem => "ecosystem",
            WardleyDecorator::Build => "build",
            WardleyDecorator::Buy => "buy",
            WardleyDecorator::Outsource => "outsource",
        }
    }

    /// The decorator a word names, in any case; none for a word that is not one of the five.
    pub fn of(word: &str) -> Option<WardleyDecorator> {
        Self::ALL.into_iter().find(|d| d.spell().eq_ignore_ascii_case(word))
    }
}

/// The decorators a component carries, as one comma-separated line for a reader (`WardleyDecorators.Join`).
pub fn join_decorators(decorators: &[WardleyDecorator]) -> String {
    decorators.iter().map(|d| d.spell()).collect::<Vec<_>>().join(", ")
}

/// A comma-separated line of decorators read back, or the first word in it that is not one of the five
/// (`WardleyDecorators.Parse`): a misspelling is refused rather than silently dropped.
pub fn parse_decorators(value: &str) -> Result<Vec<WardleyDecorator>, String> {
    let mut result = Vec::new();
    for word in value.split(',').map(str::trim).filter(|w| !w.is_empty()) {
        let decorator = WardleyDecorator::of(word).ok_or_else(|| word.to_owned())?;
        if !result.contains(&decorator) {
            result.push(decorator);
        }
    }
    Ok(result)
}

/// The five, listed for an error message (`WardleyDecorators.Vocabulary`).
pub fn decorator_vocabulary() -> String {
    join_decorators(&WardleyDecorator::ALL)
}

/// Pioneers, settlers or town planners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WardleyAttitudeKind {
    Pioneers,
    Settlers,
    TownPlanners,
}

impl WardleyAttitudeKind {
    /// The enum's name as .NET writes it (`ToString()`), which the identity key and the rules' messages use.
    pub fn name(self) -> &'static str {
        match self {
            WardleyAttitudeKind::Pioneers => "Pioneers",
            WardleyAttitudeKind::Settlers => "Settlers",
            WardleyAttitudeKind::TownPlanners => "TownPlanners",
        }
    }

    /// The kind as a person reads it (`WardleyElementDescriptions.Label`).
    pub fn label(self) -> &'static str {
        match self {
            WardleyAttitudeKind::Pioneers => "Pioneers",
            WardleyAttitudeKind::Settlers => "Settlers",
            WardleyAttitudeKind::TownPlanners => "Town Planners",
        }
    }
}

/// How a link is drawn: `A->B` a dependency, `A+>B` a flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WardleyLinkKind {
    Dependency,
    Flow,
}

impl WardleyLinkKind {
    /// The enum's name as .NET writes it, which a link's identity key carries.
    pub fn name(self) -> &'static str {
        match self {
            WardleyLinkKind::Dependency => "Dependency",
            WardleyLinkKind::Flow => "Flow",
        }
    }

    /// The arrow the document writes.
    pub fn arrow(self) -> &'static str {
        match self {
            WardleyLinkKind::Dependency => "->",
            WardleyLinkKind::Flow => "+>",
        }
    }
}

/// A `label [dx, dy]` offset, in pixels rather than map coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WardleyLabelOffset {
    pub x: f64,
    pub y: f64,
}

/// A positioned element: a `component`, an `anchor` or a `submap`.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyComponent {
    pub name: String,
    pub kind: WardleyElementKind,
    pub position: WardleyCoordinate,
    /// The 1-based line that declares it.
    pub line: u32,
    pub decorators: Vec<WardleyDecorator>,
    pub inertia: bool,
    pub label_offset: Option<WardleyLabelOffset>,
    /// The `url(...)` reference it carries, or empty.
    pub url: String,
}

/// A link between two named elements, held by name.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyLink {
    pub source: String,
    pub target: String,
    pub kind: WardleyLinkKind,
    pub line: u32,
    /// The text after `;`, or empty.
    pub context: String,
}

/// Which of the two pipeline syntaxes a document used, so it is written back the way it was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WardleyPipelineForm {
    Nested,
    Legacy,
}

/// A component inside a pipeline: an evolution position only, its visibility the parent's.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyPipelineChild {
    pub name: String,
    pub maturity: f64,
    pub line: u32,
}

/// A pipeline: a component holding children that differ only in evolution.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyPipeline {
    pub parent: String,
    pub children: Vec<WardleyPipelineChild>,
    pub line: u32,
    pub form: WardleyPipelineForm,
    /// The two coordinates a legacy pipeline carries on its own line.
    pub legacy_extent: Option<WardleyCoordinate>,
}

/// An `evolve` statement: where a component is heading.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyEvolve {
    pub name: String,
    pub maturity: f64,
    pub line: u32,
    /// The name it takes when it arrives (`evolve Name->NewName x`), or empty.
    pub override_name: String,
}

/// A region of the map given an attitude: `[v1, m1, v2, m2]`, visibility first.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyAttitude {
    pub kind: WardleyAttitudeKind,
    pub from: WardleyCoordinate,
    pub to: WardleyCoordinate,
    pub line: u32,
}

/// An `accelerator` or a `deaccelerator`.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyAccelerator {
    pub name: String,
    pub position: WardleyCoordinate,
    pub line: u32,
    pub is_deaccelerator: bool,
}

/// A `note`: free text at a position.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyNote {
    pub text: String,
    pub position: WardleyCoordinate,
    pub line: u32,
}

/// A numbered annotation, pinned at one place or several; never fewer than one.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyAnnotation {
    pub number: i32,
    pub occurrences: Vec<WardleyCoordinate>,
    pub text: String,
    pub line: u32,
}

/// A `url name [address]` definition.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyUrlDefinition {
    pub name: String,
    pub address: String,
    pub line: u32,
}

/// The `size [w, h]` a map asks to be drawn at, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WardleyMapSize {
    pub width: f64,
    pub height: f64,
}

/// A parsed map: what the document says. Statements the module does not model are absent here and present in the
/// document, which keeps them by never splicing them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WardleyMap {
    pub title: String,
    pub components: Vec<WardleyComponent>,
    pub links: Vec<WardleyLink>,
    pub pipelines: Vec<WardleyPipeline>,
    pub evolves: Vec<WardleyEvolve>,
    pub attitudes: Vec<WardleyAttitude>,
    pub accelerators: Vec<WardleyAccelerator>,
    pub notes: Vec<WardleyNote>,
    pub annotations: Vec<WardleyAnnotation>,
    pub urls: Vec<WardleyUrlDefinition>,
    pub size: Option<WardleyMapSize>,
    pub style: String,
    pub annotations_position: Option<WardleyCoordinate>,
}

/// A line the parser passed over, with why: its 0-based line and the sentence (`ParseProblem`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProblem {
    pub line: usize,
    pub message: String,
}
