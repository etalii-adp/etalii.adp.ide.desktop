//! The diagram canvas (etalii.adp spec 030, Story 2, FR-014, FR-018, SC-006, tasks T055 and T056): a diagram tab
//! that draws its session's scene with egui, culled to the view, with pan, zoom, fit to view and scroll bars, and
//! the one gesture arbiter that turns presses into selections, moves, connections, drops, renames and actions on
//! the session, each one undo away on the project's history.
//!
//! Ported from the standalone host at `8b6ccfc`: `src/client/src/canvas/library/DiagramCanvas.tsx` (the view box,
//! `zoomBy` about a point by steps of 1.25, fit to view with a padding of 40, the selection, the drop, the
//! keyboard's Escape, declared shortcuts and F2, the context menu, the inline label editor and the toolbox drop),
//! `src/client/src/canvas/scroll/` (scroll bars over an unbounded plane: the content's extent with half its span
//! again on either side, a thumb at least 5% of its track), and `src/client/src/shell/panels/ToolTabsPanel.tsx`
//! (a diagram opens in a tab of its own, its text still a choice away). A scene is rebuilt only when what it shows
//! changes: the document, the selection, the theme or the drag in flight. A selected element's resize handles, the
//! scene's [`HookKind::ResizeHandle`] hooks, carry one edge each (`DiagramCanvas.tsx`, the `resize` gesture, and
//! `gestureFrame.ts`: the preview is the frame's, the edit is written once, at the release, and Escape reverts it).

pub mod aad_scene;
pub mod ansible_scene;
pub mod fdg_scene;
pub mod gestures;
pub mod helm_scene;
pub mod metrics;
pub mod module_scene;
pub mod paint;
pub mod pipeline_scene;
pub mod placement;
pub mod sankey_scene;
pub mod supply_chain_scene;
pub mod toolbox;
pub mod viewpoint;
pub mod wardley_scene;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, Prompt, PromptOption, SelectMode, Shortcut, action_for_shortcut, find_answer};
use adp_core::Catalogue;
use adp_diagram::document::DiagramDocuments;
use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::tool::DiagramTools;
use adp_diagram::workspace::{DiagramWorkspace, OpenedDiagram};
use adp_diagram::{Resize, ResizedEdge};
use adp_disl::behavior::ids;
use adp_disl::{DislEnv, DislPluginFunctions, IdSource};
use adp_fbl::RegistrationDocument;
use adp_scene::connectors::Edge;
use adp_scene::definition::compartments::Folding;
use adp_scene::scene::{HookKind, Owner, Primitive, Role, SceneOptions, StandardRoutePlugins, build_scene};
use adp_scene::text::TextMetrics;
use adp_scene::{DiagramDefinition, Evaluator, Point, Rect, Scene, Theme as NotationTheme, compile_notation};
use adp_theme::{Kind, Theme};
use eframe::egui::{self, Color32, Pos2, RichText, Sense, Stroke};

use crate::theme::{self, Palette};
use gestures::{Arbiter, Button, Hit, Intent, Modifiers};
use metrics::EguiMetrics;
use paint::{Chrome, View};
use placement::{Mapping, Placed};

/// The factor one zoom step changes the view by (`ZOOM_STEP`).
pub const ZOOM_STEP: f64 = 1.25;
/// The narrowest and the widest the view may show, in canvas units (`MIN_VIEW_WIDTH`, `MAX_VIEW_WIDTH`).
const MIN_VIEW_WIDTH: f64 = 40.0;
const MAX_VIEW_WIDTH: f64 = 100_000.0;
/// What fitting the view keeps round the diagram, in canvas units (`FIT_PADDING`).
const FIT_PADDING: f64 = 40.0;
/// The thickness of a scroll bar, in screen points.
const SCROLL_BAR: f32 = 10.0;
/// The smallest share of its track a thumb takes (`thumbOf`).
const MIN_THUMB: f64 = 0.05;
/// What the action id of the question a deletion of the whole selection asks starts with, before the deletion's own.
const DELETE_SELECTION: &str = "canvas.delete-selection:";
/// What stands in for the notation of a diagram its module draws itself: its scene is built from the module's view
/// ([`module_scene`]) rather than compiled from a specification.
pub const MODULE_CANVAS: &str = "This diagram is drawn by its module.";

/// Where the vendored specifications and bindings are: `ADP_VENDOR`, else a `vendor` folder beside the program or
/// above it, else the source tree's.
pub fn vendor_folder() -> PathBuf {
    if let Some(folder) = std::env::var_os("ADP_VENDOR") {
        return PathBuf::from(folder);
    }
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().skip(1) {
            let candidate = dir.join("vendor");
            if candidate.join("etalii.adp").is_dir() {
                return candidate;
            }
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor")
}

/// The diagram tool types of the catalogue and one diagram workspace, with its history, per project.
pub struct DiagramHost {
    pub tools: DiagramTools,
    workspaces: HashMap<PathBuf, DiagramWorkspace>,
}

impl DiagramHost {
    /// The tool types the vendored catalogue at `vendor` knows.
    pub fn with_vendor(vendor: &Path) -> Self {
        DiagramHost {
            tools: adp_diagram_azure_devops_pipeline::register(adp_diagram_ansible_structure::register(
                adp_diagram_agent_activity::register(adp_diagram_wardley_map::register(
                    adp_diagram_supply_chain::register(adp_diagram_sankey::register(adp_diagram_c4::register(
                        adp_diagram_dependency_graph::register(
                            adp_diagram_mindmap::register(DiagramTools::new(Catalogue::load(vendor), |_| {
                                DislPluginFunctions::none()
                            }))
                            .with_module(adp_diagram_timeline::ORIGIN, adp_diagram_timeline::module())
                            .with_module(adp_diagram_hype_cycle::ORIGIN, adp_diagram_hype_cycle::module())
                            .with_module(adp_diagram_causal_loop::ORIGIN, adp_diagram_causal_loop::module())
                            .with_module(adp_diagram_helm_chart::ORIGIN, adp_diagram_helm_chart::module())
                            .with_module(
                                adp_diagram_functional_decomposition_graph::ORIGIN,
                                adp_diagram_functional_decomposition_graph::module(),
                            ),
                        ),
                    ))),
                )),
            )),
            workspaces: HashMap::new(),
        }
    }

    /// The diagrams of the project at `root`.
    pub fn workspace(&mut self, root: &Path) -> &mut DiagramWorkspace {
        self.workspaces
            .entry(root.to_path_buf())
            .or_insert_with(|| DiagramWorkspace::new(root))
    }

    /// Whether a usable diagram tool type claims the file at `path`: a body by its extension, or a registration by
    /// the tool type on its first line.
    pub fn claims(&self, path: &Path) -> bool {
        let catalogue = self.tools.catalogue();
        // A tool type is usable when its specification loads, or when its module opens its documents itself.
        let usable = |origin: &str| {
            catalogue.by_origin(origin).is_some_and(|t| t.is_usable()) || self.tools.opened_by_module(origin)
        };
        if adp_diagram::workspace::is_registration(path) {
            return std::fs::read(path).is_ok_and(|bytes| usable(&RegistrationDocument::read(bytes).origin));
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.tools.for_file(&name).iter().any(|origin| usable(origin))
    }

    /// Opens the diagram at `path`, a body or its registration, in the project at `root`.
    pub fn open(&mut self, path: &Path, root: &Path) -> Result<DiagramTab, String> {
        let workspace = self
            .workspaces
            .entry(root.to_path_buf())
            .or_insert_with(|| DiagramWorkspace::new(root));
        let (opened, session) = workspace.open(&mut self.tools, path)?;
        DiagramTab::new(root, path, opened, session)
    }

    /// Reconciles a file changed on disk with every project's open diagrams; the bodies whose documents changed.
    pub fn reconcile(&mut self, path: &Path) -> Vec<PathBuf> {
        for workspace in self.workspaces.values_mut() {
            workspace.reconcile(path);
        }
        self.take_changed()
    }

    /// The bodies whose documents changed since this was last asked, in every project.
    pub fn take_changed(&mut self) -> Vec<PathBuf> {
        self.workspaces
            .values_mut()
            .flat_map(DiagramWorkspace::take_changed)
            .collect()
    }
}

/// What a toolbox entry dragged onto a canvas carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolboxDrag {
    pub tool_id: String,
}

/// Ids for what a gesture creates: the specification's own strategy where the host can mint it, else the type's
/// name and the first number no element has yet.
struct FreshIds {
    taken: HashSet<String>,
    issued: std::cell::RefCell<HashSet<String>>,
}

impl IdSource for FreshIds {
    fn next(&self, type_name: &str) -> Result<String, String> {
        let stem: String = type_name
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                let lower = c.to_lowercase().collect::<Vec<_>>();
                if c.is_uppercase() && i > 0 {
                    std::iter::once('-').chain(lower).collect::<Vec<_>>()
                } else {
                    lower
                }
            })
            .collect();
        let mut issued = self.issued.borrow_mut();
        let id = (1..)
            .map(|n| format!("{stem}-{n}"))
            .find(|id| !self.taken.contains(id) && !issued.contains(id))
            .expect("some number is free");
        issued.insert(id.clone());
        Ok(id)
    }
}

/// Ids as the standalone host's modules minted them (`ShortGuid.NewShortGuid()`): a version 4 GUID written as 25
/// characters of base 36, for what a module's own view adds.
struct ShortGuids;

impl IdSource for ShortGuids {
    fn next(&self, _type_name: &str) -> Result<String, String> {
        const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let mut value = uuid::Uuid::new_v4().as_u128();
        let mut digits = [b'0'; 25];
        for digit in digits.iter_mut().rev() {
            *digit = DIGITS[(value % 36) as usize];
            value /= 36;
        }
        Ok(String::from_utf8(digits.to_vec()).expect("base 36 digits"))
    }
}

/// What an inline editor writes when it is committed.
#[derive(Debug, Clone, PartialEq)]
pub enum Commit {
    /// The element's label of this id, by its `parse`.
    Label(String),
    /// The answer to the prompt of an action.
    Answer { target: ActionTarget, action_id: String },
}

/// The inline label editor, open over an element.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineEditor {
    pub element: String,
    /// The label it edits, its element's first editable one when none is named.
    pub label: Option<String>,
    pub commit: Commit,
    pub text: String,
    /// Where it is drawn, in canvas units: over the label as the scene last drew it.
    pub area: Rect,
    opened: bool,
}

/// A prompt an action raised, waiting for the person.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingPrompt {
    pub target: ActionTarget,
    pub action_id: String,
    pub prompt: Prompt,
    pub value: String,
}

/// A context menu, open at a screen point.
#[derive(Debug, Clone)]
struct Menu {
    target: ActionTarget,
    at: Pos2,
    groups: Vec<ActionGroup>,
    area: Option<egui::Rect>,
}

/// Where a layout draws every node, by id: its position and its stored size.
type LaidOut = (HashMap<String, (f64, f64)>, HashMap<String, adp_diagram::Extent>);

/// What a scene was built for.
#[derive(Debug, Clone, PartialEq)]
struct SceneKey {
    generation: u64,
    selection: Vec<String>,
    dark: bool,
    viewpoint: Option<String>,
    preview: Option<(Vec<String>, u64, u64)>,
    resizing: Option<(String, u8, u64)>,
}

/// A scene and what the gestures hit-test against, as of one [`SceneKey`].
struct Cached {
    key: SceneKey,
    scene: Scene,
    /// Every node's box and mapping, without the drag in flight.
    placed: Placed,
    /// Each element's box in painting order, the topmost last.
    boxes: Vec<(String, Rect)>,
    /// Each relation's line, in canvas units, and how far from it a press still hits it.
    lines: Vec<(String, Vec<Point>, f64)>,
    anchors: Vec<(String, String, Rect)>,
    /// The resize handles of the selected elements: element, edge, area.
    handles: Vec<(String, ResizedEdge, Rect)>,
    /// The editable labels: element, label id, box.
    labels: Vec<(String, String, Rect)>,
    bounds: Option<Rect>,
    /// The canvas's own fill where the scene draws no background.
    surface: Option<Color32>,
    drop_mapping: Mapping,
    /// The ruler of a time axis: its height in screen points and the axis's canvas units per second.
    ruler: Option<(f64, f64)>,
    /// The ruler of the hype cycle graph's month axis, in the diagram's time unit; none in Compact, whose x is no date.
    month_ruler: Option<adp_diagram_hype_cycle::scale::TimeUnit>,
    /// The legend at the top left of the canvas (`notation.canvas.legend`).
    legend: Vec<viewpoint::LegendEntry>,
}

/// A diagram open in a tab.
pub struct DiagramTab {
    pub root: PathBuf,
    /// The path it was opened by: the body or its registration.
    pub path: PathBuf,
    pub opened: OpenedDiagram,
    pub session: DiagramSession,
    definition: Result<DiagramDefinition, String>,
    /// The viewpoint a switch of the canvas toggles (the hype cycle graph's Compact), when the specification has one.
    pub toggle: Option<viewpoint::Toggle>,
    /// Whether the toggled viewpoint is on: the viewer's, never saved.
    pub toggled: bool,
    /// The notation the toggled viewpoint draws with, compiled when it is first switched on.
    variant: Option<Result<DiagramDefinition, String>>,
    /// Counts the changes to the document this tab has seen; a scene is rebuilt when it moves.
    pub generation: u64,
    cache: Option<Cached>,
    metrics: Option<EguiMetrics>,
    pub view: View,
    /// Whether the view is the diagram fitted to the surface, kept so as either changes, until the person zooms,
    /// pans or scrolls (`DiagramCanvas.tsx`: the view is `null`, the fit box, until it is set).
    follows_fit: bool,
    /// Whether the view followed the fit when the pan in flight began, so Escape puts that back too.
    followed_before_pan: bool,
    /// Whether the canvas has had the keyboard handed to it once, when its tab first drew.
    focused_once: bool,
    /// The scale of a time axis, canvas units per second, frozen when the diagram is first drawn with something on
    /// it, so an edit never rescales the drawing under the person (`timelineScaleOf`).
    time_scale: Option<f64>,
    /// The selected elements and relations, the most recent last.
    pub selection: Vec<String>,
    pub arbiter: Arbiter,
    pub editor: Option<InlineEditor>,
    pub prompt: Option<PendingPrompt>,
    menu: Option<Menu>,
    /// Where new ids come from; a test may set its own.
    pub ids: Option<Box<dyn IdSource>>,
    /// The last refusal or report, for the status bar.
    pub notice: Option<String>,
    /// Whether an edit succeeded since the status bar last looked, so a refusal shown before it is taken away.
    pub cleared: bool,
    hovered: Option<String>,
    /// The surface drawn last frame.
    pub surface: egui::Rect,
    pub grid: crate::properties::Grid,
    /// The selection last reported to the window's one selection.
    pub reported: Option<Vec<String>>,
    id: egui::Id,
}

impl DiagramTab {
    fn new(root: &Path, path: &Path, opened: OpenedDiagram, session: DiagramSession) -> Result<Self, String> {
        Ok(DiagramTab {
            root: root.to_path_buf(),
            path: path.to_path_buf(),
            id: egui::Id::unique(("diagram-canvas", opened.body_path.clone())),
            opened,
            session,
            definition: Err(String::new()),
            toggle: None,
            toggled: false,
            variant: None,
            generation: 0,
            cache: None,
            metrics: None,
            view: View::default(),
            follows_fit: true,
            followed_before_pan: false,
            focused_once: false,
            time_scale: None,
            selection: Vec::new(),
            arbiter: Arbiter::default(),
            editor: None,
            prompt: None,
            menu: None,
            ids: None,
            notice: None,
            cleared: false,
            hovered: None,
            surface: egui::Rect::NOTHING,
            grid: crate::properties::Grid::default(),
            reported: None,
        })
    }

    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// The tab's title; every edit is written as it is made, so a diagram tab is never unsaved.
    pub fn title(&self) -> String {
        self.name()
    }

    /// Whether this tab shows `path`, by the body or the registration.
    pub fn shows(&self, path: &Path) -> bool {
        self.path == path || self.opened.body_path == path || self.opened.registration_path.as_deref() == Some(path)
    }

    /// The canvas's keyboard focus id.
    pub fn canvas_id(&self) -> egui::Id {
        self.id
    }

    /// The scene as last built.
    #[cfg(test)]
    pub fn scene(&self) -> Option<&Scene> {
        self.cache.as_ref().map(|c| &c.scene)
    }

    /// The box an element is drawn in, without the drag in flight.
    pub fn bounds_of(&self, id: &str) -> Option<Rect> {
        let cache = self.cache.as_ref()?;
        cache
            .placed
            .rects
            .get(id)
            .copied()
            .or_else(|| cache.boxes.iter().find(|(e, _)| e == id).map(|(_, r)| *r))
    }

    /// Something this tab shows changed: its scene is rebuilt on the next frame.
    pub fn changed(&mut self) {
        self.generation += 1;
    }

    fn ensure_definition(&mut self, documents: &DiagramDocuments) {
        if self.definition.is_ok() || !self.definition.as_ref().err().is_some_and(String::is_empty) {
            return;
        }
        if self.session.module_view().is_some() {
            // A tool type whose module reads, draws and edits its documents itself (C4) has no notation to compile.
            self.definition = Err(MODULE_CANVAS.to_owned());
            return;
        }
        let Some(document) = documents.get(&self.opened.body_path) else {
            self.definition = Err("The diagram is not open.".to_owned());
            return;
        };
        self.definition = compile_notation(document.tool().definition.specification()).map_err(|e| e.to_string());
        self.toggle = viewpoint::toggle_of(document.tool().definition.specification());
    }

    /// The notation the canvas draws with now: the toggled viewpoint's while it is on, else the specification's.
    fn active(&self) -> Option<&DiagramDefinition> {
        match (&self.variant, self.toggled) {
            (Some(Ok(variant)), true) => Some(variant),
            _ => self.definition.as_ref().ok(),
        }
    }

    /// Switches the toggled viewpoint on or off: its notation compiled the first time, the session's `env.viewpoint`
    /// following it, and the scene drawn again; a viewpoint that does not compile stays off, with its reason.
    pub fn set_toggled(&mut self, documents: &DiagramDocuments, on: bool) {
        let Some(toggle) = self.toggle.clone() else { return };
        let Some(document) = documents.get(&self.opened.body_path) else {
            return;
        };
        let specification = document.tool().definition.specification();
        if on && self.variant.is_none() {
            let plugins = document
                .tool()
                .module
                .as_ref()
                .map(|m| m.plugin_functions())
                .unwrap_or_else(DislPluginFunctions::none);
            self.variant = Some(viewpoint::variant_definition(specification, &toggle.id, &plugins));
        }
        if let (true, Some(Err(reason))) = (on, &self.variant) {
            self.notice = Some(reason.clone());
            return;
        }
        self.toggled = on;
        let default = specification
            .root()
            .get("viewpoints")
            .and_then(|v| v.get(&toggle.id))
            .and_then(|v| v.get("variantOf"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned();
        self.session.set_viewpoint(if on { &toggle.id } else { &default });
        self.selection.clear();
        self.changed();
    }

    /// Where the toggled viewpoint's layout draws every node, centres and sizes in canvas units, when it is on and the
    /// tool type's module lays it out (the hype cycle graph's Compact, `adp_diagram_hype_cycle::compact`).
    fn variant_positions(&self, document: &adp_diagram::DiagramDocument) -> Option<LaidOut> {
        if !self.toggled || self.opened.origin != adp_diagram_hype_cycle::ORIGIN {
            return None;
        }
        let packed = adp_diagram_hype_cycle::compact::positions(&document.model().diagram);
        let positions = packed.iter().map(|(id, p)| (id.clone(), (p.x, p.y))).collect();
        let extents = packed
            .iter()
            .map(|(id, p)| {
                (
                    id.clone(),
                    adp_diagram::Extent {
                        width: Some(adp_diagram::Length::Canvas(p.width)),
                        height: None,
                    },
                )
            })
            .collect();
        Some((positions, extents))
    }

    /// Builds the scene when what it shows changed since it was last built.
    pub fn ensure_scene(&mut self, ctx: &egui::Context, documents: &DiagramDocuments, dark: bool) {
        self.ensure_definition(documents);
        let preview = self
            .arbiter
            .moving()
            .map(|(ids, (dx, dy))| (ids.to_vec(), dx.to_bits(), dy.to_bits()));
        let resizing = self
            .arbiter
            .resizing()
            .map(|(id, edge, delta)| (id.to_owned(), edge as u8, delta.to_bits()));
        let key = SceneKey {
            generation: self.generation,
            selection: self.selection.clone(),
            dark,
            viewpoint: self
                .toggled
                .then(|| self.toggle.as_ref().map(|t| t.id.clone()))
                .flatten(),
            preview,
            resizing,
        };
        if self.cache.as_ref().is_some_and(|c| c.key == key) {
            return;
        }
        if self.session.module_view().is_some() {
            self.ensure_module_scene(ctx, documents, key);
            return;
        }
        let Some(document) = documents.get(&self.opened.body_path) else {
            return;
        };
        let variant_positions = self.variant_positions(document);
        let Some(definition) = self.active() else {
            return;
        };
        let definition = definition.clone();
        let definition = &definition;
        let metrics = self.metrics.get_or_insert_with(|| EguiMetrics::new(ctx));
        let specification = &definition.specification;
        let has_mode = |mode: &str| {
            specification
                .root()
                .get("notation")
                .and_then(|n| n.get("theme"))
                .and_then(|t| t.get("modes"))
                .and_then(|m| m.get(mode))
                .is_some()
        };
        let mode = if dark && has_mode("dark") {
            Some("dark")
        } else if !dark && has_mode("light") {
            Some("light")
        } else {
            None
        };
        let notation_theme = NotationTheme::of(specification, mode);
        let surface = (dark && mode.is_none())
            .then(|| notation_theme.text_of("color.surface"))
            .flatten()
            .and_then(|c| paint::parse_colour(&c));
        let evaluator = Evaluator::new(specification, notation_theme, DislEnv::default());
        let diagram = &document.model().diagram;
        // The viewer's folds, which the corner glyphs read; a layout that places the diagram leaves out what is folded
        // away, which is then not drawn.
        document.bind_views();
        let (laid_out, positions, extents) = match variant_positions {
            Some((positions, extents)) => (Some(positions.clone()), positions, extents),
            None => {
                let laid_out = document.laid_out();
                let positions = laid_out.clone().unwrap_or_else(|| document.positions());
                (laid_out, positions, document.extents())
            }
        };
        let module = document.tool().module.as_deref();
        let time_axis = placement::time_axis(specification);
        if time_axis.is_some() && self.time_scale.is_none() {
            // Frozen at the first drawing with something on it: each element from its begin to its end.
            self.time_scale = placement::fitted_time_scale(positions.iter().map(|(id, (x, _))| {
                let width = match extents.get(id).and_then(|e| e.width) {
                    Some(adp_diagram::Length::Domain(width)) => width,
                    _ => 0.0,
                };
                (*x, x + width)
            }));
        }
        let time_scale = self.time_scale.or(time_axis.map(|a| a.units_per_second));
        let placed = placement::place(
            definition,
            &evaluator,
            diagram,
            &positions,
            &extents,
            metrics,
            laid_out.is_some(),
            module,
            time_scale,
        );
        let mut rects = placed.rects.clone();
        if let Some((ids, (dx, dy))) = self.arbiter.moving() {
            for id in ids {
                if let Some(rect) = rects.get_mut(id) {
                    *rect = Rect::new(rect.x + dx, rect.y + dy, rect.width, rect.height);
                }
            }
        }
        if let Some((id, edge, delta)) = self.arbiter.resizing()
            && let Some(element_type) = placed.types.get(id).and_then(|t| definition.element_type(t))
            && let Some(rect) = rects.get_mut(id)
        {
            *rect = placement::resized(*rect, edge, delta, &element_type.size);
        }
        // A selected element that is gone is no longer selected.
        let known: HashSet<String> = diagram.elements().iter().map(|e| e.id()).collect();
        self.selection.retain(|id| known.contains(id));
        let folding = Folding::default();
        let options = SceneOptions {
            selection: &self.selection,
            folding: &folding,
            metrics,
            plugins: &StandardRoutePlugins,
        };
        let scene = build_scene(definition, &evaluator, diagram, &rects, &options);
        let identity = View::default();
        let origin = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1.0, 1.0));
        let mut boxes: Vec<(String, Rect)> = Vec::new();
        let mut lines = Vec::new();
        let mut anchors = Vec::new();
        let mut handles = Vec::new();
        let mut labels = Vec::new();
        for primitive in &scene.primitives {
            match (primitive, primitive.owner()) {
                (Primitive::Shape(shape), Owner::Element(id)) => {
                    let Some(bounds) = primitive.bounds(metrics) else {
                        continue;
                    };
                    let body = matches!(shape.role, Role::Body | Role::BodyPart | Role::Part(_));
                    match boxes.iter_mut().find(|(e, _)| e == id) {
                        Some((_, rect)) if body || !rects.contains_key(id) => *rect = rect.union(&bounds),
                        Some(_) => {}
                        None => boxes.push((id.clone(), rects.get(id).copied().unwrap_or(bounds))),
                    }
                }
                (Primitive::Text(text), Owner::Element(id)) => {
                    if let (true, Role::Label { id: label, .. }) = (text.editable, &text.role)
                        && let Some(bounds) = primitive.bounds(metrics)
                    {
                        labels.push((id.clone(), label.clone(), bounds));
                    }
                    if !boxes.iter().any(|(e, _)| e == id)
                        && let Some(bounds) = primitive.bounds(metrics)
                    {
                        boxes.push((id.clone(), rects.get(id).copied().unwrap_or(bounds)));
                    }
                }
                (Primitive::Edge(edge), Owner::Relation(id)) if edge.selectable => {
                    let points = paint::flatten(&edge.path, &identity, origin)
                        .into_iter()
                        .flat_map(|(points, _)| points)
                        .map(|p| Point::new(p.x as f64, p.y as f64))
                        .collect();
                    lines.push((id.clone(), points, edge.hit_width.max(edge.stroke.width) / 2.0));
                }
                (Primitive::Hook(hook), Owner::Element(id)) => match &hook.kind {
                    HookKind::Anchor { name, .. } => anchors.push((id.clone(), name.clone(), hook.area)),
                    HookKind::ResizeHandle(edge) => {
                        let edge = match edge {
                            Edge::Left => ResizedEdge::Left,
                            Edge::Right => ResizedEdge::Right,
                            Edge::Top => ResizedEdge::Top,
                            Edge::Bottom => ResizedEdge::Bottom,
                        };
                        handles.push((id.clone(), edge, hook.area));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        let bounds = scene.bounds(metrics);
        let drop_mapping = placement::drop_mapping(
            specification,
            diagram,
            placement::Scales {
                time: time_scale,
                module: module.is_some_and(|m| m.position_at(0.0, 0.0).is_some()),
            },
        );
        let ruler = time_axis
            .and_then(|axis| axis.ruler)
            .zip(time_scale)
            .filter(|(_, scale)| *scale > 0.0);
        let month_ruler = (self.opened.origin == adp_diagram_hype_cycle::ORIGIN && !self.toggled)
            .then(|| adp_diagram_hype_cycle::reading::unit_of(diagram));
        let legend = viewpoint::legend_of(specification, mode);
        self.cache = Some(Cached {
            key,
            scene,
            placed,
            boxes,
            lines,
            anchors,
            handles,
            labels,
            bounds,
            surface,
            drop_mapping,
            ruler,
            month_ruler,
            legend,
        });
    }

    /// Builds the scene of a diagram its module draws itself, from what the module's view renders.
    fn ensure_module_scene(&mut self, ctx: &egui::Context, documents: &DiagramDocuments, key: SceneKey) {
        let elements = match self.session.render(documents) {
            Ok(elements) => elements,
            Err(reason) => {
                self.notice = Some(reason);
                Vec::new()
            }
        };
        let metrics = self.metrics.get_or_insert_with(|| EguiMetrics::new(ctx));
        // A selected element that is gone is no longer selected.
        let known: HashSet<&str> = elements.iter().map(|e| e.id.as_str()).collect();
        self.selection.retain(|id| known.contains(id.as_str()));
        // A Wardley map is drawn in its 0..1 space on a square of 1000 canvas units (T070), its marks mapped back by
        // that scale.
        let (built, scaled) = if self.opened.origin == adp_diagram_wardley_map::ORIGIN {
            wardley_scene::build(&elements, &self.selection, self.arbiter.moving())
        } else if self.opened.origin == adp_diagram_sankey::ORIGIN {
            (
                sankey_scene::build(&elements, &self.selection, self.arbiter.moving()),
                HashMap::new(),
            )
        } else if self.opened.origin == adp_diagram_helm_chart::ORIGIN {
            // A Helm chart's boxes are moved by the top-left the registration stores (`onElementMoved`).
            (
                helm_scene::build(&elements, &self.selection, self.arbiter.moving(), key.dark),
                HashMap::new(),
            )
        } else if self.opened.origin == adp_diagram_ansible_structure::ORIGIN {
            // An Ansible structure's boxes are moved by the top-left the registration stores (`onElementMoved`).
            (
                ansible_scene::build(&elements, &self.selection, self.arbiter.moving(), key.dark),
                HashMap::new(),
            )
        } else if self.opened.origin == adp_diagram_supply_chain::ORIGIN {
            // A supply chain's cards and frames are moved by the top-left the document holds (`onElementMoved`).
            (
                supply_chain_scene::build(&elements, &self.selection, self.arbiter.moving()),
                HashMap::new(),
            )
        } else if self.opened.origin == adp_diagram_azure_devops_pipeline::ORIGIN {
            // A pipeline arranges itself from its dependencies: nothing on it is dragged.
            (pipeline_scene::build(&elements, &self.selection), HashMap::new())
        } else if self.opened.origin == adp_diagram_agent_activity::ORIGIN {
            // An activity file's cards are laid out on their rings by the module, and locked by their centre.
            (
                aad_scene::build(&elements, &self.selection, self.arbiter.moving(), key.dark),
                HashMap::new(),
            )
        } else {
            (
                module_scene::build(&elements, &self.selection, self.arbiter.moving(), key.dark),
                HashMap::new(),
            )
        };
        let bounds = built.scene.bounds(metrics);
        // Where a module writes a move as the element's top-left, the drag hands it the top-left.
        let top_left = Mapping {
            top_left: true,
            ..Mapping::default()
        };
        let mut mappings: HashMap<String, Mapping> = built.top_left.iter().map(|id| (id.clone(), top_left)).collect();
        mappings.extend(scaled);
        self.cache = Some(Cached {
            key,
            scene: built.scene,
            placed: Placed {
                rects: built.rects,
                mappings,
                ..Placed::default()
            },
            boxes: built.boxes,
            lines: built.lines,
            anchors: built.anchors,
            handles: Vec::new(),
            labels: built.labels,
            bounds,
            surface: None,
            drop_mapping: Mapping::default(),
            ruler: None,
            month_ruler: None,
            legend: Vec::new(),
        });
    }

    // ---- the view ---------------------------------------------------------------------------------------------------

    fn zoom_limits(&self) -> (f64, f64) {
        let width = (self.surface.width() as f64).max(1.0);
        (width / MAX_VIEW_WIDTH, width / MIN_VIEW_WIDTH)
    }

    /// Changes the zoom by `factor` about a canvas point, the view's centre when none is given (`zoomBy`).
    pub fn zoom_by(&mut self, factor: f64, about: Option<Point>) {
        let (min, max) = self.zoom_limits();
        let zoom = (self.view.zoom * factor).clamp(min, max);
        let visible = self.view.visible(self.surface);
        let about = about.unwrap_or_else(|| visible.centre());
        let kept = self.view.zoom / zoom;
        self.view.origin = Point::new(
            about.x - (about.x - self.view.origin.x) * kept,
            about.y - (about.y - self.view.origin.y) * kept,
        );
        self.view.zoom = zoom;
        self.follows_fit = false;
    }

    /// Fits the view to everything the diagram draws, with a padding round it, and keeps it fitted as the diagram or
    /// the surface changes until the person zooms, pans or scrolls (`fitToView`: the view is `null` again).
    pub fn fit(&mut self) {
        self.follows_fit = true;
        self.refit();
    }

    /// Sets the view by hand, as a pan or a scroll does: it no longer follows the fit.
    pub fn look_from(&mut self, origin: Point) {
        self.view.origin = origin;
        self.follows_fit = false;
    }

    /// The view fitted to what the diagram draws now.
    fn refit(&mut self) {
        let Some(bounds) = self.cache.as_ref().and_then(|c| c.bounds) else {
            self.view = View {
                origin: Point::new(
                    -(self.surface.width() as f64) / 2.0,
                    -(self.surface.height() as f64) / 2.0,
                ),
                zoom: 1.0,
            };
            return;
        };
        let padded = bounds.inflate(FIT_PADDING);
        let (min, max) = self.zoom_limits();
        let zoom = (self.surface.width() as f64 / padded.width)
            .min(self.surface.height() as f64 / padded.height)
            .clamp(min, max);
        let centre = padded.centre();
        self.view = View {
            origin: Point::new(
                centre.x - self.surface.width() as f64 / zoom / 2.0,
                centre.y - self.surface.height() as f64 / zoom / 2.0,
            ),
            zoom,
        };
    }

    /// The scrollable extent of each axis: the content's bounds with half its span on either side.
    fn extent(&self) -> Rect {
        let content = self
            .cache
            .as_ref()
            .and_then(|c| c.bounds)
            .unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0));
        Rect::new(
            content.x - content.width * 0.5,
            content.y - content.height * 0.5,
            (content.width * 2.0).max(1.0),
            (content.height * 2.0).max(1.0),
        )
    }

    // ---- hit testing ------------------------------------------------------------------------------------------------

    /// What lies at a canvas point: an anchor, else a selected element's resize handle (the anchors are drawn over the
    /// handles, `SpanElement.tsx`), else the topmost element, else a relation's line, else nothing.
    pub fn hit(&self, point: Point) -> Hit {
        let Some(cache) = &self.cache else { return Hit::Empty };
        let tolerance = 3.0 / self.view.zoom;
        if let Some((element, anchor, _)) = cache
            .anchors
            .iter()
            .rev()
            .find(|(_, _, area)| area.inflate(tolerance).contains(point))
        {
            return Hit::Anchor {
                element: element.clone(),
                anchor: anchor.clone(),
            };
        }
        if let Some((element, edge, _)) = cache
            .handles
            .iter()
            .rev()
            .find(|(_, _, area)| area.inflate(tolerance).contains(point))
        {
            return Hit::Resize {
                element: element.clone(),
                edge: *edge,
            };
        }
        if let Some((id, _)) = cache.boxes.iter().rev().find(|(_, rect)| rect.contains(point)) {
            return Hit::Element(id.clone());
        }
        let near = |line: &[Point], reach: f64| {
            line.windows(2)
                .any(|pair| distance_to_segment(point, pair[0], pair[1]) <= reach.max(tolerance))
        };
        if let Some((id, _, _)) = cache.lines.iter().rev().find(|(_, line, reach)| near(line, *reach)) {
            return Hit::Relation(id.clone());
        }
        Hit::Empty
    }

    /// What a drag of a press on `hit` moves: the selected nodes that move, when the press is on one of them, else
    /// the pressed node alone, when it moves.
    fn moves_for(&self, hit: &Hit) -> Vec<String> {
        let Hit::Element(id) = hit else { return Vec::new() };
        let movable = |id: &str| self.movable(id);
        if self.selection.iter().any(|s| s == id) {
            self.selection.iter().filter(|s| movable(s)).cloned().collect()
        } else if movable(id) {
            vec![id.clone()]
        } else {
            Vec::new()
        }
    }

    /// Whether a right drag from an element draws a connection here (`connectOnRightDrag`): where the module that
    /// draws the view says so, else where the notation binds a connect gesture to the secondary button on the body.
    fn connects_on_right_drag(&self) -> bool {
        match self.session.module_view() {
            Some(view) => view.connects_on_right_drag(),
            None => self
                .active()
                .is_some_and(|definition| gestures::connects_on_right_drag(&definition.notation)),
        }
    }

    /// Whether a node may be dragged: it is placed, and its notation lets it move.
    fn movable(&self, id: &str) -> bool {
        let Some(cache) = &self.cache else { return false };
        if !cache.placed.rects.contains_key(id) {
            return false;
        }
        if self.session.module_view().is_some() {
            // Every card of a module's view drags; where it goes is the module's to record.
            return true;
        }
        let Some(definition) = self.active() else { return false };
        cache
            .placed
            .types
            .get(id)
            .and_then(|t| definition.element_type(t))
            .is_some_and(|t| t.draggable)
    }

    // ---- what gestures do -------------------------------------------------------------------------------------------

    fn ensure_ids(&mut self, documents: &DiagramDocuments) {
        if self.ids.is_none() && self.session.module_view().is_some() {
            // A module that draws its view itself mints what it adds as its standalone module did.
            self.ids = Some(Box::new(ShortGuids));
        }
        if self.ids.is_none() {
            let minted = self
                .definition
                .as_ref()
                .ok()
                .and_then(|d| ids::of(&d.specification).ok());
            self.ids = Some(match minted {
                Some(uuids) => Box::new(uuids),
                None => Box::new(FreshIds {
                    taken: documents
                        .get(&self.opened.body_path)
                        .map(|d| d.model().diagram.elements().iter().map(|e| e.id()).collect())
                        .unwrap_or_default(),
                    issued: Default::default(),
                }),
            });
        }
    }

    pub(crate) fn report(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => self.succeeded(),
            Err(refusal) => self.notice = Some(refusal),
        }
    }

    /// An edit went through: the refusal shown before it is taken away, and the scene is drawn again.
    fn succeeded(&mut self) {
        self.notice = None;
        self.cleared = true;
        self.changed();
    }

    /// Carries out what a gesture came to.
    pub fn apply(&mut self, intent: Intent, workspace: &mut DiagramWorkspace) {
        match intent {
            Intent::Select { ids, mode } => self.select(ids, mode),
            Intent::Move { ids, delta } => {
                if !ids.iter().all(|id| self.selection.contains(id)) {
                    self.select(ids.clone(), SelectMode::Replace);
                }
                self.move_by(workspace, &ids, delta);
            }
            Intent::Resize { element, edge, delta } => self.resize_by(workspace, &element, edge, delta),
            Intent::Connect { source, target, at } => self.connect(workspace, &source, target.as_deref(), at),
            Intent::ContextMenu { hit, screen, canvas } => {
                let target = match hit.id() {
                    Some(id) => ActionTarget::Element(id.to_owned()),
                    None => {
                        let (x, y) = self
                            .cache
                            .as_ref()
                            .map(|c| c.drop_mapping.point_to_domain(canvas))
                            .unwrap_or((canvas.x, canvas.y));
                        ActionTarget::Canvas { x, y }
                    }
                };
                let groups = self.session.actions(workspace.documents(), &target);
                if groups.iter().any(|g| !g.actions.is_empty()) {
                    self.menu = Some(Menu {
                        target,
                        at: Pos2::new(screen.0, screen.1),
                        groups,
                        area: None,
                    });
                }
            }
            Intent::Pan { origin } => {
                if self.follows_fit {
                    self.followed_before_pan = true;
                }
                self.look_from(origin);
            }
        }
    }

    /// Changes the selection.
    pub fn select(&mut self, ids: Vec<String>, mode: SelectMode) {
        match mode {
            SelectMode::Replace => {
                self.selection.clear();
                for id in ids {
                    if !self.selection.contains(&id) {
                        self.selection.push(id);
                    }
                }
            }
            SelectMode::Toggle => {
                for id in ids {
                    match self.selection.iter().position(|s| *s == id) {
                        Some(at) => {
                            self.selection.remove(at);
                        }
                        None => self.selection.push(id),
                    }
                }
            }
            SelectMode::Add => {
                for id in ids {
                    if !self.selection.contains(&id) {
                        self.selection.push(id);
                    }
                }
            }
        }
    }

    /// Moves elements by `delta` canvas units, each landing where its placement puts it, as one edit; where a drag
    /// proposes a parent instead, each becomes the last child of the node its box is dropped on.
    pub fn move_by(&mut self, workspace: &mut DiagramWorkspace, ids: &[String], delta: (f64, f64)) {
        if ids
            .iter()
            .any(|id| self.session.drag_reparents(workspace.documents(), id))
        {
            self.reparent_by(workspace, ids, delta);
            return;
        }
        let Some(cache) = &self.cache else { return };
        let moves: Vec<(String, f64, f64)> = ids
            .iter()
            .filter_map(|id| {
                let rect = cache.placed.rects.get(id)?;
                let mapping = cache.placed.mappings.get(id).copied().unwrap_or_default();
                let moved = Rect::new(rect.x + delta.0, rect.y + delta.1, rect.width, rect.height);
                let (x, y) = mapping.to_domain(moved);
                Some((id.clone(), x, y))
            })
            .collect();
        if moves.is_empty() {
            return;
        }
        let result = self.session.move_elements(workspace, &moves);
        self.report(result);
    }

    /// A drag that re-parents (`MindmapCanvas.tsx`, `onElementMoved` and `dropTargetAt`): each node dropped with its
    /// centre on another becomes that node's last child, as one edit; dropped on itself, on a node of its own branch
    /// or on empty canvas, the drag simply ends. Nothing is placed: the layout puts it where it now belongs.
    fn reparent_by(&mut self, workspace: &mut DiagramWorkspace, ids: &[String], delta: (f64, f64)) {
        let Some(cache) = &self.cache else { return };
        let documents = workspace.documents();
        let Some(document) = documents.get(&self.opened.body_path) else {
            return;
        };
        let moves: Vec<(String, String)> = ids
            .iter()
            .filter_map(|id| {
                let rect = cache.placed.rects.get(id)?;
                let dropped = Point::new(rect.centre().x + delta.0, rect.centre().y + delta.1);
                // The topmost node under the drop, where the model has it; the dragged one and its branch are no target.
                let (target, _) = cache.boxes.iter().rev().find(|(_, b)| b.contains(dropped))?;
                let branch = document.element(id)?.descendants();
                (target != id && !branch.iter().any(|d| d.id() == *target)).then(|| (id.clone(), target.clone()))
            })
            .collect();
        if moves.is_empty() {
            return;
        }
        let result = self.session.reparent_elements(workspace, &moves);
        self.report(result);
    }

    /// Carries one edge of an element `delta` canvas units along its axis, the far edge kept, within the notation's
    /// limits, as one edit (`resizedBounds`, `element-resized`).
    pub fn resize_by(&mut self, workspace: &mut DiagramWorkspace, id: &str, edge: ResizedEdge, delta: f64) {
        let Some(cache) = &self.cache else { return };
        let Some(definition) = self.active() else { return };
        let (Some(rect), Some(element_type)) = (
            cache.placed.rects.get(id),
            cache.placed.types.get(id).and_then(|t| definition.element_type(t)),
        ) else {
            return;
        };
        let mapping = cache.placed.mappings.get(id).copied().unwrap_or_default();
        let bounds = placement::resized(*rect, edge, delta, &element_type.size);
        if bounds == *rect {
            return;
        }
        let (x, y) = mapping.to_domain(bounds);
        let resize = Resize {
            edge,
            x,
            y,
            right: bounds.right() / mapping.scale.0,
            bottom: bounds.bottom() / mapping.scale.1,
            width: bounds.width,
            height: bounds.height,
        };
        let result = self.session.resize_element(workspace, id, &resize);
        self.report(result);
    }

    fn connect(&mut self, workspace: &mut DiagramWorkspace, source: &str, target: Option<&str>, at: Point) {
        if let Some(view) = self.session.module_view().cloned() {
            // A module that draws its view itself says which of its actions a connection runs: the functional
            // decomposition graph's, the one relation the pair admits (`relationOnto`). One that names none takes no
            // connection from the canvas: the standalone host's C4 canvas draws its cards' edge anchors and the drag
            // from one, and handles no `connection-drawn`.
            let Some(target) = target.filter(|t| *t != source) else {
                return;
            };
            match view.connect_action(workspace.documents(), source, target) {
                Some(Ok(action)) => {
                    let target = ActionTarget::Connection {
                        source: Some(source.to_owned()),
                        target: Some(target.to_owned()),
                    };
                    self.run_action(workspace, target, &action);
                }
                Some(Err(refusal)) => self.notice = Some(refusal),
                None => self.notice = Some(adp_diagram::session::NOT_OFFERED.to_owned()),
            }
            return;
        }
        let types = self
            .session
            .relation_types_between(workspace.documents(), source, target);
        let Ok(definition) = &self.definition else { return };
        match target {
            Some(target) => {
                let Some(relation) = types
                    .into_iter()
                    .find(|t| target != source || definition.relation_type(t).is_some_and(|r| r.endpoints.allow_self))
                else {
                    if target != source {
                        self.notice = Some("Nothing in this diagram connects these two.".to_owned());
                    }
                    return;
                };
                self.ensure_ids(workspace.documents());
                let ids = self.ids.as_deref().expect("made");
                let result = self.session.connect(workspace, &relation, source, target, ids);
                self.report(result);
            }
            None => {
                // Dropped on empty canvas: only a relation that declares the release meaningful makes its target.
                let Some(relation) = types
                    .into_iter()
                    .find(|t| definition.relation_type(t).is_some_and(|r| r.empty_release))
                else {
                    return;
                };
                let (x, y) = self
                    .cache
                    .as_ref()
                    .map(|c| c.drop_mapping.point_to_domain(at))
                    .unwrap_or((at.x, at.y));
                self.ensure_ids(workspace.documents());
                let ids = self.ids.as_deref().expect("made");
                let result = self.session.connect_to_new(workspace, &relation, source, x, y, ids);
                self.report(result);
            }
        }
    }

    /// Drops toolbox entry `tool_id` at a canvas point.
    pub fn drop_tool(&mut self, workspace: &mut DiagramWorkspace, tool_id: &str, at: Point) {
        if self.session.module_view().is_some() {
            // The entry carries the module's own action: dropped on an element, that element is the target the
            // action adds inside; on empty canvas, the canvas (`onElementDropped`).
            let Some(action) = self
                .session
                .toolbox(workspace.documents())
                .into_iter()
                .find(|item| item.id == tool_id)
                .map(|item| item.drop_action_id)
            else {
                return;
            };
            let target = match self.hit(at) {
                Hit::Element(id) => ActionTarget::Element(id),
                _ => ActionTarget::Canvas { x: at.x, y: at.y },
            };
            // Asked in a dialog rather than over the card it lands on: what is typed names the new element.
            self.ensure_ids(workspace.documents());
            let ids = self.ids.as_deref().expect("made");
            match self.session.execute(workspace, &target, &action, ids) {
                ActionOutcome::Done => self.report(Ok(())),
                ActionOutcome::Refused(refusal) => self.notice = Some(refusal),
                ActionOutcome::Prompt(prompt) => {
                    self.prompt = Some(PendingPrompt {
                        target,
                        action_id: action,
                        prompt,
                        value: String::new(),
                    })
                }
            }
            return;
        }
        if self.session.tool_operation(workspace.documents(), tool_id).is_some() {
            // A tool that runs an operation runs it on the node it lands on, as that node's menu runs it; over empty
            // canvas there is no node and nothing happens (`MindmapCanvas.tsx`, `onElementDropped`).
            let Hit::Element(id) = self.hit(at) else { return };
            let before = self.node_ids(workspace.documents());
            self.ensure_ids(workspace.documents());
            let ids = self.ids.as_deref().expect("made");
            if let Some((action_id, outcome)) = self.session.drop_tool_on(workspace, tool_id, &id, ids) {
                self.outcome(workspace, ActionTarget::Element(id), &action_id, outcome, &before);
            }
            return;
        }
        let at = match workspace.documents().get(&self.opened.body_path) {
            Some(document) if self.toggled && self.opened.origin == adp_diagram_hype_cycle::ORIGIN => {
                let (x, y) = adp_diagram_hype_cycle::compact::inverse(&document.model().diagram, at.x, at.y);
                Point::new(x, y)
            }
            _ => at,
        };
        let (x, y) = self
            .cache
            .as_ref()
            .map(|c| c.drop_mapping.point_to_domain(at))
            .unwrap_or((at.x, at.y));
        let before = self.node_ids(workspace.documents());
        self.ensure_ids(workspace.documents());
        let ids = self.ids.as_deref().expect("made");
        let result = self.session.drop_tool(workspace, tool_id, x, y, ids);
        let done = result.is_ok();
        self.report(result);
        if done {
            self.select_new(workspace.documents(), &before);
        }
    }

    fn node_ids(&self, documents: &DiagramDocuments) -> HashSet<String> {
        documents
            .get(&self.opened.body_path)
            .map(|d| d.model().diagram.nodes().iter().map(|n| n.id()).collect())
            .unwrap_or_default()
    }

    /// Selects the one node an edit added, when it added one; that node.
    fn select_new(&mut self, documents: &DiagramDocuments, before: &HashSet<String>) -> Option<String> {
        let added: Vec<String> = self
            .node_ids(documents)
            .into_iter()
            .filter(|id| !before.contains(id))
            .collect();
        if let [one] = added.as_slice() {
            self.select(vec![one.clone()], SelectMode::Replace);
            return Some(one.clone());
        }
        None
    }

    /// Runs action `action_id` of `target`'s menu: done, refused with its sentence, or a prompt to answer.
    pub fn run_action(&mut self, workspace: &mut DiagramWorkspace, target: ActionTarget, action_id: &str) {
        let before = self.node_ids(workspace.documents());
        self.ensure_ids(workspace.documents());
        let ids = self.ids.as_deref().expect("made");
        let outcome = self.session.execute(workspace, &target, action_id, ids);
        self.outcome(workspace, target, action_id, outcome, &before);
    }

    /// Carries out what action `action_id` of `target` answered, the nodes before it ran being `before`: an edit
    /// drawn and what it added selected, a refusal said, a question asked. A name asked of an element is asked in place,
    /// over the node the action added when it added one (`addChild`: `select: added`, then `editLabel` of it), else
    /// over the element itself.
    fn outcome(
        &mut self,
        workspace: &mut DiagramWorkspace,
        target: ActionTarget,
        action_id: &str,
        outcome: ActionOutcome,
        before: &HashSet<String>,
    ) {
        match outcome {
            ActionOutcome::Done => {
                self.succeeded();
                self.select_new(workspace.documents(), before);
            }
            ActionOutcome::Refused(refusal) => self.notice = Some(refusal),
            ActionOutcome::Prompt(Prompt::Input { initial_value, .. })
                if matches!(target, ActionTarget::Element(_)) =>
            {
                let ActionTarget::Element(element) = &target else {
                    unreachable!()
                };
                // A node made before its name is asked is drawn, and selected, at once.
                let added = self.select_new(workspace.documents(), before);
                if added.is_some() {
                    self.succeeded();
                }
                let element = added.unwrap_or_else(|| element.clone());
                let area = self.label_area(&element, None).unwrap_or_default();
                self.editor = Some(InlineEditor {
                    element,
                    label: None,
                    commit: Commit::Answer {
                        target: target.clone(),
                        action_id: action_id.to_owned(),
                    },
                    text: initial_value,
                    area,
                    opened: false,
                });
            }
            ActionOutcome::Prompt(prompt) => {
                self.prompt = Some(PendingPrompt {
                    target,
                    action_id: action_id.to_owned(),
                    prompt,
                    value: String::new(),
                })
            }
        }
    }

    /// Answers the prompt waiting, with `value`; or drops it.
    pub fn answer(&mut self, workspace: &mut DiagramWorkspace, value: Option<&str>) {
        let Some(pending) = self.prompt.take() else { return };
        let Some(value) = value else { return };
        if let Some(action_id) = pending.action_id.strip_prefix(DELETE_SELECTION) {
            self.delete_selection(workspace, action_id, true);
            return;
        }
        match self
            .session
            .answer(workspace, &pending.target, &pending.action_id, value)
        {
            ActionOutcome::Done => self.succeeded(),
            ActionOutcome::Refused(refusal) => self.notice = Some(refusal),
            ActionOutcome::Prompt(_) => self.notice = Some("That cannot be answered here yet.".to_owned()),
        }
    }

    /// Where an element's label of `label` (its first editable one when none is named) is drawn.
    fn label_area(&self, element: &str, label: Option<&str>) -> Option<Rect> {
        let cache = self.cache.as_ref()?;
        let found = cache
            .labels
            .iter()
            .find(|(e, l, _)| e == element && label.is_none_or(|name| name == l))
            .map(|(_, _, area)| *area);
        let element_box = self.bounds_of(element);
        match (found, element_box) {
            (Some(text), Some(b)) => Some(Rect::new(b.x, text.y - 2.0, b.width, text.height + 4.0)),
            (Some(text), None) => Some(text),
            (None, b) => b,
        }
    }

    /// Opens the inline editor on an element's label: the one under `at`, else its first editable one (F2 and
    /// double-click).
    pub fn open_editor(&mut self, documents: &DiagramDocuments, element: &str, at: Option<Point>) {
        let Some(cache) = &self.cache else { return };
        let candidates: Vec<&(String, String, Rect)> = cache.labels.iter().filter(|(e, _, _)| e == element).collect();
        let Some((_, label, _)) = at
            .and_then(|p| candidates.iter().find(|(_, _, area)| area.contains(p)).copied())
            .or_else(|| candidates.first().copied())
        else {
            self.notice = Some("This has no label to rename here.".to_owned());
            return;
        };
        let label = label.clone();
        let text = self.session.label_text(documents, element, &label).unwrap_or_default();
        let area = self.label_area(element, Some(&label)).unwrap_or_default();
        self.editor = Some(InlineEditor {
            element: element.to_owned(),
            label: Some(label.clone()),
            commit: Commit::Label(label),
            text,
            area,
            opened: false,
        });
    }

    /// Writes what the inline editor holds, and closes it.
    pub fn commit_editor(&mut self, workspace: &mut DiagramWorkspace) {
        let Some(editor) = self.editor.take() else { return };
        match editor.commit {
            Commit::Label(label) => {
                let result = self
                    .session
                    .edit_label(workspace, &editor.element, &label, &editor.text);
                self.report(result);
            }
            Commit::Answer { target, action_id } => {
                match self.session.answer(workspace, &target, &action_id, &editor.text) {
                    ActionOutcome::Done => self.report(Ok(())),
                    ActionOutcome::Refused(refusal) => self.notice = Some(refusal),
                    ActionOutcome::Prompt(_) => {}
                }
            }
        }
    }

    /// Escape: the gesture in flight dissolves, a pan back where it began; whether there was one.
    pub fn cancel_gesture(&mut self) -> bool {
        if let Some(origin) = self.arbiter.pan_origin() {
            self.view.origin = origin;
            self.follows_fit = std::mem::take(&mut self.followed_before_pan);
        }
        self.arbiter.cancel()
    }

    /// A declared shortcut of the selected element: Tab, Enter, Delete, Space, F2 and the rest, by the menus. With
    /// several selected, a deletion deletes all of them as one edit, or none (the multi-select specification's
    /// Requirement 7.3).
    pub fn shortcut(&mut self, workspace: &mut DiagramWorkspace, pressed: &Shortcut) -> bool {
        let Some(primary) = self.selection.last().cloned() else {
            return false;
        };
        let target = ActionTarget::Element(primary);
        let groups = self.session.actions(workspace.documents(), &target);
        let Some(action) = action_for_shortcut(&groups, pressed) else {
            return false;
        };
        let id = action.id.clone();
        if self.selection.len() > 1 && self.session.is_deletion(workspace.documents(), &target, &id) {
            self.delete_selection(workspace, &id, false);
            return true;
        }
        self.run_action(workspace, target, &id);
        true
    }

    /// Deletes every selected element by its deletion `action_id`, as one edit, asking first where the specification
    /// asks before deleting any of them.
    fn delete_selection(&mut self, workspace: &mut DiagramWorkspace, action_id: &str, confirmed: bool) {
        let selection = self.selection.clone();
        match self
            .session
            .delete_elements(workspace, &selection, action_id, confirmed)
        {
            ActionOutcome::Done => self.succeeded(),
            ActionOutcome::Refused(refusal) => self.notice = Some(refusal),
            ActionOutcome::Prompt(prompt) => {
                self.prompt = Some(PendingPrompt {
                    target: ActionTarget::Element(selection.last().cloned().unwrap_or_default()),
                    action_id: format!("{DELETE_SELECTION}{action_id}"),
                    prompt,
                    value: String::new(),
                })
            }
        }
    }

    // ---- drawing and input -------------------------------------------------------------------------------------------

    /// Draws the tab: its view controls, the canvas and its scroll bars, and what is open over them.
    pub fn show(&mut self, ui: &mut egui::Ui, workspace: &mut DiagramWorkspace, theme: &Theme, palette: &Palette) {
        let dark = theme.kind == Kind::Dark;
        let mut zoom: Option<f64> = None;
        let mut fit = false;
        let mut history = None;
        let mut switched = None;
        let muted = theme::color(theme, &["descriptionForeground"], palette.foreground);
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(8, 2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let button = |ui: &mut egui::Ui, text: &str, hint: &str| {
                        ui.add(
                            egui::Button::new(RichText::new(text).size(theme.sizes.workbench_font - 1.0)).frame(false),
                        )
                        .on_hover_text(hint)
                        .clicked()
                    };
                    if button(ui, "Zoom In", "Zoom in (+)") {
                        zoom = Some(ZOOM_STEP);
                    }
                    if button(ui, "Zoom Out", "Zoom out (-)") {
                        zoom = Some(1.0 / ZOOM_STEP);
                    }
                    if button(ui, "Fit to View", "Fit the diagram to the view (0)") {
                        fit = true;
                    }
                    ui.separator();
                    if button(ui, "Undo", "Undo (Ctrl+Z)") {
                        history = Some(true);
                    }
                    if button(ui, "Redo", "Redo (Ctrl+Y)") {
                        history = Some(false);
                    }
                    if let Some(toggle) = &self.toggle {
                        ui.separator();
                        let mut on = self.toggled;
                        if ui
                            .checkbox(
                                &mut on,
                                RichText::new(&toggle.label).size(theme.sizes.workbench_font - 1.0),
                            )
                            .changed()
                        {
                            switched = Some(on);
                        }
                    }
                    ui.label(RichText::new(format!("{:.0}%", self.view.zoom * 100.0)).color(muted));
                    if let Some(notice) = &self.notice {
                        ui.label(RichText::new(notice).color(ui.visuals().warn_fg_color));
                    }
                });
            });
        if let Some(on) = switched {
            self.set_toggled(workspace.documents(), on);
        }
        if let Some(undo) = history {
            // Said as Ctrl+Z says it: a refusal in the status bar, which a step that went through takes away.
            match if undo { workspace.undo() } else { workspace.redo() } {
                adp_history::Outcome::Refused(refusal) => self.notice = Some(refusal.0),
                adp_history::Outcome::Done { .. } => self.succeeded(),
            }
            self.changed();
        }

        let available = ui.available_rect_before_wrap();
        let surface = egui::Rect::from_min_max(available.min, available.max - egui::vec2(SCROLL_BAR, SCROLL_BAR));
        ui.allocate_rect(available, Sense::hover());
        let response = ui.interact(surface, self.id, Sense::click_and_drag());
        self.surface = surface;
        if !self.focused_once {
            // A diagram that opens has the keyboard, as an editor that opens has it.
            self.focused_once = true;
            ui.memory_mut(|m| m.request_focus(self.id));
        }
        self.ensure_scene(ui.ctx(), workspace.documents(), dark);
        self.follow_fit();
        if let Some(factor) = zoom {
            self.zoom_by(factor, None);
        }
        if fit {
            self.fit();
        }

        self.input(ui, workspace, &response);
        self.ensure_scene(ui.ctx(), workspace.documents(), dark);
        self.follow_fit();
        self.paint(ui, palette);
        self.scroll_bars(ui, palette, theme);
        self.show_editor(ui, workspace);
        self.show_menu(ui, workspace, palette);
        self.show_prompt(ui, workspace);
        if self.notice.is_some() || self.cleared {
            // The status bar is drawn before the canvas: one more frame shows what this one said.
            ui.ctx().request_repaint();
        }
    }

    /// Keeps the view fitted while it follows the fit; never under a gesture in flight, whose preview is not yet the
    /// diagram (`fitBoxOf` reads the model, not the drag).
    fn follow_fit(&mut self) {
        if self.follows_fit && self.cache.is_some() && self.arbiter.is_idle() {
            self.refit();
        }
    }

    fn input(&mut self, ui: &mut egui::Ui, workspace: &mut DiagramWorkspace, response: &egui::Response) {
        let ctx = ui.ctx().clone();
        let surface = self.surface;
        let blocked_at = |tab: &Self, pos: Pos2| {
            tab.prompt.is_some()
                || tab.menu.as_ref().and_then(|m| m.area).is_some_and(|a| a.contains(pos))
                || tab
                    .editor
                    .as_ref()
                    .is_some_and(|e| tab.view.rect_to_screen(surface, e.area).contains(pos))
        };
        let (press, release, latest, double, modifiers, zooms) = ui.input(|i| {
            let press = if i.pointer.primary_pressed() {
                i.pointer.press_origin().map(|p| (Button::Primary, p))
            } else if i.pointer.button_pressed(egui::PointerButton::Secondary) {
                i.pointer.press_origin().map(|p| (Button::Secondary, p))
            } else {
                None
            };
            let release = i.pointer.primary_released() || i.pointer.button_released(egui::PointerButton::Secondary);
            (
                press,
                release,
                i.pointer.latest_pos(),
                i.pointer.button_double_clicked(egui::PointerButton::Primary),
                Modifiers {
                    command: i.modifiers.command,
                    shift: i.modifiers.shift,
                },
                wheel_zooms(&i.events),
            )
        });
        let hover = latest.filter(|p| surface.contains(*p));
        let hovering = hover.map(|p| self.hit(self.view.to_canvas(surface, p)));
        self.hovered = hovering.as_ref().and_then(|h| h.id().map(str::to_owned));
        let edge = match (&self.arbiter.gesture, &hovering) {
            (gestures::Gesture::Resizing { edge, .. }, _) | (_, Some(Hit::Resize { edge, .. })) => Some(*edge),
            _ => None,
        };
        if let Some(edge) = edge {
            ctx.set_cursor_icon(if edge.is_horizontal() {
                egui::CursorIcon::ResizeHorizontal
            } else {
                egui::CursorIcon::ResizeVertical
            });
        }

        // A press anywhere but on the open menu closes it.
        if let Some((_, pos)) = press
            && self
                .menu
                .as_ref()
                .is_some_and(|m| m.area.is_none_or(|a| !a.contains(pos)))
        {
            self.menu = None;
        }
        if let Some((button, pos)) = press
            && surface.contains(pos)
            && !blocked_at(self, pos)
        {
            if self.editor.is_some() {
                self.commit_editor(workspace);
            }
            ui.memory_mut(|m| m.request_focus(self.id));
            self.followed_before_pan = false;
            let canvas = self.view.to_canvas(surface, pos);
            let hit = self.hit(canvas);
            let moves = self.moves_for(&hit);
            self.arbiter.connects_on_right_drag = self.connects_on_right_drag();
            self.arbiter
                .press(button, hit, modifiers, (pos.x, pos.y), canvas, moves, self.view.origin);
        }
        if !self.arbiter.is_idle()
            && let Some(pos) = latest
            && let Some(intent) = self
                .arbiter
                .motion((pos.x, pos.y), self.view.to_canvas(surface, pos), self.view.zoom)
        {
            self.apply(intent, workspace);
        }
        if release && let Some(pos) = latest {
            let canvas = self.view.to_canvas(surface, pos);
            if !self.arbiter.is_idle() {
                let hit = self.hit(canvas);
                for intent in self.arbiter.release(canvas, &hit) {
                    self.apply(intent, workspace);
                }
                // A double-click edits a label in place; a module that draws its view itself declares no double-click
                // (`C4Canvas.tsx` declares no `activate` gesture), and its renames are its menu's and F2's.
                if double
                    && self.session.module_view().is_none()
                    && let Hit::Element(id) = &hit
                {
                    let id = id.clone();
                    self.open_editor(workspace.documents(), &id, Some(canvas));
                }
            }
            if surface.contains(pos)
                && let Some(drag) = egui::DragAndDrop::take_payload::<ToolboxDrag>(&ctx)
            {
                self.drop_tool(workspace, &drag.tool_id, canvas);
            }
        }
        if let Some(pos) = hover
            && self.prompt.is_none()
        {
            for factor in zooms {
                let about = self.view.to_canvas(surface, pos);
                self.zoom_by(factor, Some(about));
            }
        }

        // The canvas's keys are the canvas's only while it has the keyboard: an Enter that commits a field of the
        // property grid is that field's (`onKeyDown` is the surface's own, `isTextTarget` aside).
        let focused = ui.memory(|m| m.focused());
        let ours = focused == Some(self.id);
        if focused == Some(self.id) {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    self.id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                )
            });
        }
        if ours && self.editor.is_none() && self.prompt.is_none() {
            self.keys(ui, workspace);
        }
        let _ = response;
    }

    fn keys(&mut self, ui: &mut egui::Ui, workspace: &mut DiagramWorkspace) {
        use egui::{Key, Modifiers as M};
        let pressed = |ui: &mut egui::Ui, modifiers: M, key: Key| ui.input_mut(|i| i.consume_key(modifiers, key));
        if pressed(ui, M::NONE, Key::Escape) {
            if !self.cancel_gesture() {
                self.menu = None;
            }
            return;
        }
        if pressed(ui, M::NONE, Key::Plus) || pressed(ui, M::NONE, Key::Equals) {
            self.zoom_by(ZOOM_STEP, None);
        }
        if pressed(ui, M::NONE, Key::Minus) {
            self.zoom_by(1.0 / ZOOM_STEP, None);
        }
        if pressed(ui, M::NONE, Key::Num0) {
            self.fit();
        }
        if pressed(ui, M::NONE, Key::F2) {
            // F2 is the rename the menu declares on it, where it declares one (C4's Rename…), else the label's own
            // editor.
            if !self.shortcut(workspace, &Shortcut::key("F2"))
                && let Some(id) = self.selection.last().cloned()
            {
                self.open_editor(workspace.documents(), &id, None);
            }
            return;
        }
        for (key, name) in [
            (Key::Tab, "Tab"),
            (Key::Enter, "Enter"),
            (Key::Delete, "Delete"),
            (Key::Backspace, "Backspace"),
            (Key::Insert, "Insert"),
            // A browser's `key` for the space bar, which `Space` in a specification is spelled as.
            (Key::Space, " "),
        ] {
            for shift in [false, true] {
                let modifiers = if shift { M::SHIFT } else { M::NONE };
                if ui.input(|i| i.key_pressed(key) && i.modifiers.matches_exact(modifiers)) {
                    let mut shortcut = Shortcut::key(name);
                    if shift {
                        shortcut = shortcut.with_shift();
                    }
                    if self.shortcut(workspace, &shortcut) {
                        ui.input_mut(|i| i.consume_key(modifiers, key));
                    }
                }
            }
        }
    }

    fn paint(&mut self, ui: &mut egui::Ui, palette: &Palette) {
        let surface = self.surface;
        let painter = ui.painter_at(surface);
        let Some(cache) = &self.cache else {
            let message = match &self.definition {
                Err(reason) if !reason.is_empty() => reason.clone(),
                _ => "The diagram cannot be drawn.".to_owned(),
            };
            painter.rect_filled(surface, 0.0, palette.editor_background);
            painter.text(
                surface.center(),
                egui::Align2::CENTER_CENTER,
                message,
                egui::FontId::proportional(13.0),
                palette.foreground,
            );
            return;
        };
        painter.rect_filled(surface, 0.0, cache.surface.unwrap_or(palette.editor_background));
        let metrics = self.metrics.as_ref().expect("made with the scene");
        let visible = self.view.visible(surface);
        let culled = cache.scene.cull(visible, metrics as &dyn TextMetrics);
        let mut shown: HashSet<String> = self.selection.iter().cloned().collect();
        if let Some(hovered) = &self.hovered {
            shown.insert(hovered.clone());
        }
        let chrome = Chrome {
            foreground: palette.foreground,
            focus: palette.focus,
            anchors_of: &shown,
        };
        let shapes = paint::paint(&culled, &self.view, surface, &chrome, ui.ctx());
        painter.extend(shapes);
        if let Some(text) = &cache.scene.empty_text {
            painter.text(
                surface.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(13.0),
                palette.foreground.gamma_multiply(0.7),
            );
        }
        if let Some((height, scale)) = cache.ruler {
            self.paint_ruler(&painter, palette, height as f32, scale);
        }
        if let Some(unit) = cache.month_ruler {
            self.paint_month_ruler(&painter, palette, unit);
        }
        if !cache.legend.is_empty() {
            self.paint_legend(&painter, palette, &cache.legend);
        }
        if let gestures::Gesture::Connecting { from, to, .. } = &self.arbiter.gesture {
            painter.line_segment(
                [self.view.to_screen(surface, *from), self.view.to_screen(surface, *to)],
                Stroke::new(1.5, palette.focus),
            );
        }
    }

    /// The time ruler along the bottom of the canvas (`TimelineRuler.tsx`): fixed to the view, its labels on the
    /// boundaries `timelineTicks.ts` picks for the time in view, each over the instant it names.
    fn paint_ruler(&self, painter: &egui::Painter, palette: &Palette, height: f32, units_per_second: f64) {
        let surface = self.surface;
        let band = egui::Rect::from_min_max(Pos2::new(surface.min.x, surface.max.y - height), surface.max);
        painter.rect_filled(band, 0.0, palette.editor_background.gamma_multiply(0.92));
        painter.line_segment(
            [band.left_top(), band.right_top()],
            Stroke::new(1.0, palette.foreground.gamma_multiply(0.3)),
        );
        let visible = self.view.visible(surface);
        let start = visible.x / units_per_second;
        let end = visible.right() / units_per_second;
        for tick in adp_diagram_timeline::ruler::ticks(start, end, surface.width() as f64) {
            let x = self
                .view
                .to_screen(surface, Point::new(tick.seconds * units_per_second, 0.0))
                .x;
            painter.line_segment(
                [Pos2::new(x, band.top()), Pos2::new(x, band.top() + 5.0)],
                Stroke::new(1.0, palette.foreground.gamma_multiply(0.5)),
            );
            painter.text(
                Pos2::new(x + 3.0, band.center().y + 2.0),
                egui::Align2::LEFT_CENTER,
                tick.label,
                egui::FontId::proportional(11.0),
                palette.foreground.gamma_multiply(0.8),
            );
        }
    }

    /// The hype cycle graph's ruler along the bottom of the canvas: fixed to the view, its labels on the boundaries of
    /// the coarsest level that fits (`adp_diagram_hype_cycle::ruler`), each over the month it names.
    fn paint_month_ruler(
        &self,
        painter: &egui::Painter,
        palette: &Palette,
        unit: adp_diagram_hype_cycle::scale::TimeUnit,
    ) {
        let surface = self.surface;
        let height = 24.0;
        let band = egui::Rect::from_min_max(Pos2::new(surface.min.x, surface.max.y - height), surface.max);
        painter.rect_filled(band, 0.0, palette.editor_background.gamma_multiply(0.92));
        painter.line_segment(
            [band.left_top(), band.right_top()],
            Stroke::new(1.0, palette.foreground.gamma_multiply(0.3)),
        );
        let visible = self.view.visible(surface);
        for tick in adp_diagram_hype_cycle::ruler::ticks(visible.x, visible.right(), surface.width() as f64, unit) {
            let x = self.view.to_screen(surface, Point::new(tick.x, 0.0)).x;
            painter.line_segment(
                [Pos2::new(x, band.top()), Pos2::new(x, band.top() + 5.0)],
                Stroke::new(1.0, palette.foreground.gamma_multiply(0.5)),
            );
            painter.text(
                Pos2::new(x + 3.0, band.center().y + 2.0),
                egui::Align2::LEFT_CENTER,
                tick.label,
                egui::FontId::proportional(11.0),
                palette.foreground.gamma_multiply(0.8),
            );
        }
    }

    /// The legend at the top left of the canvas: one swatch per entry, in its colour, beside its label.
    fn paint_legend(&self, painter: &egui::Painter, palette: &Palette, legend: &[viewpoint::LegendEntry]) {
        let font = egui::FontId::proportional(12.0);
        let origin = self.surface.min + egui::vec2(8.0, 8.0);
        let row = 18.0;
        let width = legend
            .iter()
            .map(|entry| {
                painter
                    .layout_no_wrap(entry.label.clone(), font.clone(), palette.foreground)
                    .size()
                    .x
            })
            .fold(0.0_f32, f32::max)
            + 30.0;
        let frame = egui::Rect::from_min_size(origin, egui::vec2(width, row * legend.len() as f32 + 8.0));
        painter.rect_filled(frame, 3.0, palette.editor_background.gamma_multiply(0.92));
        painter.rect_stroke(
            frame,
            3.0,
            Stroke::new(1.0, palette.foreground.gamma_multiply(0.3)),
            egui::StrokeKind::Inside,
        );
        for (index, entry) in legend.iter().enumerate() {
            let top = origin.y + 4.0 + row * index as f32;
            let swatch = egui::Rect::from_min_size(Pos2::new(origin.x + 6.0, top + 3.0), egui::vec2(12.0, 12.0));
            let colour = entry
                .colour
                .as_deref()
                .and_then(paint::parse_colour)
                .unwrap_or(palette.foreground);
            painter.rect_filled(swatch, 2.0, colour);
            painter.text(
                Pos2::new(swatch.right() + 6.0, swatch.center().y),
                egui::Align2::LEFT_CENTER,
                &entry.label,
                font.clone(),
                palette.foreground,
            );
        }
    }

    fn scroll_bars(&mut self, ui: &mut egui::Ui, palette: &Palette, theme: &Theme) {
        let surface = self.surface;
        let extent = self.extent();
        let visible = self.view.visible(surface);
        let track = theme::color(
            theme,
            &["scrollbarSlider.background"],
            palette.foreground.gamma_multiply(0.2),
        );
        let active = theme::color(
            theme,
            &["scrollbarSlider.activeBackground"],
            palette.foreground.gamma_multiply(0.4),
        );
        for horizontal in [true, false] {
            let (start, span, extent_start, extent_span, track_rect) = if horizontal {
                (
                    visible.x,
                    visible.width,
                    extent.x.min(visible.x),
                    extent.width.max(visible.right() - extent.x.min(visible.x)),
                    egui::Rect::from_min_max(
                        Pos2::new(surface.min.x, surface.max.y),
                        Pos2::new(surface.max.x, surface.max.y + SCROLL_BAR),
                    ),
                )
            } else {
                (
                    visible.y,
                    visible.height,
                    extent.y.min(visible.y),
                    extent.height.max(visible.bottom() - extent.y.min(visible.y)),
                    egui::Rect::from_min_max(
                        Pos2::new(surface.max.x, surface.min.y),
                        Pos2::new(surface.max.x + SCROLL_BAR, surface.max.y),
                    ),
                )
            };
            let extent_span = extent_span.max(1.0);
            let size = (span / extent_span).clamp(MIN_THUMB, 1.0);
            let offset = ((start - extent_start) / extent_span).clamp(0.0, 1.0 - size);
            let length = if horizontal {
                track_rect.width()
            } else {
                track_rect.height()
            } as f64;
            let thumb = if horizontal {
                egui::Rect::from_min_size(
                    Pos2::new(track_rect.min.x + (offset * length) as f32, track_rect.min.y + 2.0),
                    egui::vec2((size * length) as f32, SCROLL_BAR - 4.0),
                )
            } else {
                egui::Rect::from_min_size(
                    Pos2::new(track_rect.min.x + 2.0, track_rect.min.y + (offset * length) as f32),
                    egui::vec2(SCROLL_BAR - 4.0, (size * length) as f32),
                )
            };
            let response = ui.interact(thumb, self.id.with(horizontal), Sense::drag());
            let colour = if response.dragged() || response.hovered() {
                active
            } else {
                track
            };
            ui.painter().rect_filled(thumb, 4.0, colour);
            if response.dragged() {
                let delta = response.drag_delta();
                let moved = if horizontal { delta.x } else { delta.y } as f64 / length * extent_span;
                let origin = if horizontal {
                    Point::new(self.view.origin.x + moved, self.view.origin.y)
                } else {
                    Point::new(self.view.origin.x, self.view.origin.y + moved)
                };
                self.look_from(origin);
            }
        }
    }

    fn show_editor(&mut self, ui: &mut egui::Ui, workspace: &mut DiagramWorkspace) {
        if let Some((element, label)) = self.editor.as_ref().map(|e| (e.element.clone(), e.label.clone()))
            && let Some(area) = self.label_area(&element, label.as_deref())
            && let Some(editor) = &mut self.editor
        {
            editor.area = area;
        }
        let Some(editor) = &mut self.editor else { return };
        let mut screen = self.view.rect_to_screen(self.surface, editor.area);
        if screen.height() < 22.0 {
            screen = egui::Rect::from_center_size(screen.center(), egui::vec2(screen.width().max(80.0), 22.0));
        }
        let id = self.id.with("inline-editor");
        let response = ui.put(
            screen,
            egui::TextEdit::singleline(&mut editor.text)
                .id(id)
                .align(egui::Align2::CENTER_CENTER),
        );
        if !editor.opened {
            editor.opened = true;
            response.request_focus();
            return;
        }
        let (enter, escape) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        if escape {
            self.editor = None;
            ui.memory_mut(|m| m.request_focus(self.id));
        } else if enter || response.lost_focus() {
            self.commit_editor(workspace);
            ui.memory_mut(|m| m.request_focus(self.id));
        }
    }

    fn show_menu(&mut self, ui: &mut egui::Ui, workspace: &mut DiagramWorkspace, palette: &Palette) {
        let Some(menu) = &mut self.menu else { return };
        let mut chosen = None;
        let id = self.id.with("context-menu");
        let area = egui::Area::new(id)
            .order(egui::Order::Foreground)
            .fixed_pos(menu.at)
            .show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    ui.set_min_width(180.0);
                    menu_entries(ui, &menu.groups, id, &mut chosen);
                });
            });
        menu.area = Some(area.response.rect);
        let _ = palette;
        if let Some(action) = chosen {
            let target = menu.target.clone();
            self.menu = None;
            self.run_action(workspace, target, &action);
        }
    }

    fn show_prompt(&mut self, ui: &mut egui::Ui, workspace: &mut DiagramWorkspace) {
        let Some(pending) = &mut self.prompt else { return };
        let mut answer: Option<Option<String>> = None;
        egui::Modal::new(self.id.with("prompt")).show(ui.ctx(), |ui| {
            ui.set_width(420.0);
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            match &pending.prompt {
                Prompt::Confirm {
                    title,
                    message,
                    confirm_label,
                    ..
                } => {
                    ui.heading(title.as_str());
                    ui.label(message.as_str());
                    ui.horizontal(|ui| {
                        if ui.button(confirm_label.as_str()).clicked() {
                            answer = Some(Some(String::new()));
                        }
                        if ui.button("Cancel").clicked() {
                            answer = Some(None);
                        }
                    });
                }
                Prompt::Input {
                    title,
                    label,
                    confirm_label,
                    ..
                } => {
                    ui.heading(title.as_str());
                    ui.label(label.as_str());
                    ui.text_edit_singleline(&mut pending.value).request_focus();
                    ui.horizontal(|ui| {
                        if ui.button(confirm_label.as_str()).clicked() || enter {
                            answer = Some(Some(pending.value.clone()));
                        }
                        if ui.button("Cancel").clicked() {
                            answer = Some(None);
                        }
                    });
                }
                Prompt::Choice {
                    title,
                    confirm_label,
                    options,
                    empty_message,
                    ..
                } => {
                    ui.heading(title.as_str());
                    answer = choose(ui, &[options], empty_message, confirm_label, &mut pending.value, enter);
                }
                Prompt::File {
                    title,
                    confirm_label,
                    files,
                    pinned,
                    empty_message,
                    ..
                } => {
                    ui.heading(title.as_str());
                    answer = choose(
                        ui,
                        &[pinned, files],
                        empty_message,
                        confirm_label,
                        &mut pending.value,
                        enter,
                    );
                }
                Prompt::Closed { message } => {
                    ui.label(message.as_str());
                    if ui.button("OK").clicked() {
                        answer = Some(None);
                    }
                }
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                answer = Some(None);
            }
        });
        if let Some(value) = answer {
            self.answer(workspace, value.as_deref());
        }
    }
}

/// The entries of a context menu, a separator between groups; a submenu is a heading that opens its entries beneath
/// it, indented, inside the same menu, so a press on them is a press on the menu (an action with `items` instead of a
/// run, as the standalone host's menus nest them). The action chosen is put in `chosen`.
fn menu_entries(ui: &mut egui::Ui, groups: &[ActionGroup], id: egui::Id, chosen: &mut Option<String>) {
    for (index, group) in groups.iter().enumerate() {
        if index > 0 && !group.actions.is_empty() {
            ui.separator();
        }
        for action in &group.actions {
            let available = action.is_available();
            if action.items.is_empty() {
                let response = ui
                    .add_enabled(available, egui::Button::new(&action.label).frame(false))
                    .on_disabled_hover_text(&action.unavailable_reason);
                if response.clicked() {
                    *chosen = Some(action.id.clone());
                }
                continue;
            }
            let nested = id.with(&action.id);
            ui.add_enabled_ui(available, |ui| {
                egui::CollapsingHeader::new(action.label.as_str())
                    .id_salt(nested)
                    .show(ui, |ui| menu_entries(ui, &action.items, nested, chosen));
            })
            .response
            .on_disabled_hover_text(&action.unavailable_reason);
        }
    }
}

/// The options of a choice or a file question, each list in turn, groups as headings over their indented options and
/// an option that may not be chosen greyed with its reason; the one picked is held in `value`, and the confirm button,
/// or Enter, answers with it once it is one that may be chosen (`find_answer`). The empty message stands in for no
/// options at all. A name field the question carries is not offered: an answer here carries the one value.
fn choose(
    ui: &mut egui::Ui,
    lists: &[&Vec<PromptOption>],
    empty_message: &str,
    confirm_label: &str,
    value: &mut String,
    enter: bool,
) -> Option<Option<String>> {
    fn rows(ui: &mut egui::Ui, options: &[PromptOption], value: &mut String) {
        for option in options {
            if option.selectable {
                let available = option.unavailable_reason.is_empty();
                let mut response = ui
                    .add_enabled(
                        available,
                        egui::Button::selectable(*value == option.id, option.label.as_str()),
                    )
                    .on_disabled_hover_text(&option.unavailable_reason);
                if !option.description.is_empty() {
                    response = response.on_hover_text(&option.description);
                }
                if response.clicked() {
                    *value = option.id.clone();
                }
            } else {
                ui.label(RichText::new(option.label.as_str()).strong());
            }
            if !option.children.is_empty() {
                ui.indent(option.id.as_str(), |ui| rows(ui, &option.children, value));
            }
        }
    }
    if lists.iter().all(|l| l.is_empty()) {
        ui.label(empty_message);
    } else {
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            let mut shown = false;
            for options in lists {
                if options.is_empty() {
                    continue;
                }
                if shown {
                    ui.separator();
                }
                rows(ui, options, value);
                shown = true;
            }
        });
    }
    let answerable = lists.iter().any(|options| find_answer(options, value).is_some());
    let mut answer = None;
    ui.horizontal(|ui| {
        if ui.add_enabled(answerable, egui::Button::new(confirm_label)).clicked() || (enter && answerable) {
            answer = Some(Some(value.clone()));
        }
        if ui.button("Cancel").clicked() {
            answer = Some(None);
        }
    });
    answer
}

/// The zoom each turn of the wheel and each pinch of `events` asks for, in order: a notch of the wheel, with or without
/// Ctrl, zooms in or out by one step of [`ZOOM_STEP`] about the pointer (`onWheel`: `deltaY < 0` zooms in), a wheel
/// that reports lines or pages by one step a line, and a pinch by its own factor.
fn wheel_zooms(events: &[egui::Event]) -> Vec<f64> {
    events
        .iter()
        .filter_map(|event| match event {
            egui::Event::MouseWheel { unit, delta, .. } if delta.y != 0.0 => {
                let steps = match unit {
                    egui::MouseWheelUnit::Point => 1,
                    egui::MouseWheelUnit::Line | egui::MouseWheelUnit::Page => (delta.y.abs().round() as i32).max(1),
                };
                // egui's y is positive for a wheel turned away from the person, the browser's deltaY negative.
                let step = if delta.y > 0.0 { ZOOM_STEP } else { 1.0 / ZOOM_STEP };
                Some(step.powi(steps))
            }
            egui::Event::Zoom(factor) if *factor != 1.0 => Some(*factor as f64),
            _ => None,
        })
        .collect()
}

fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    if length == 0.0 {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0.0, 1.0);
    p.distance(Point::new(a.x + t * dx, a.y + t * dy))
}
