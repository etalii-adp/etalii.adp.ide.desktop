//! The standalone host's contrast tests, run against both VS Code themes (etalii.adp spec 030, FR-042, research R9,
//! task T086).
//!
//! Ported from `src/client/src/theme.contrast.test.ts` and `src/client/src/moduleThemeContrast.test.ts` at `8b6ccfc`.
//! VS Code's look replaces only the standalone host's chrome tokens, so each of those is read from the VS Code theme:
//! `--color-text` is `editor.foreground`, and `--color-surface` and `--color-bg` are `editor.background`, which the
//! canvas and the editors are drawn on. A tool's own colours stay the tool's; where a pair fails, the tool's colour is
//! adjusted, never the VS Code value.
//!
//! <b>Why the colours are read from the files that draw with them.</b> The standalone host read `index.css` rather
//! than a rendered component, because jsdom applies no CSS and a rendered assertion measured nothing. The same holds
//! here for another reason: a copy of a colour in this test would be checked while the drawn one drifted. So each
//! tool's colours are read where the desktop host declares them: each module's `theme.rs`, which gives the tool its
//! colours in the light and the dark theme (task T087, FR-045), the selection colour of `adp-shell`'s canvas and, for
//! the tools drawn from their notation, the vendored specification's `notation.theme` with the tokens its module adds
//! over it. Every family is counted before a ratio is taken, so a renamed or moved declaration makes this test say
//! "found none" loudly instead of passing over an empty set.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adp_theme::contrast::{contrast_ratio, delta_e};
use adp_theme::{Kind, Rgba, Theme};

/// WCAG 2.x AA for normal-size text.
const AA: f64 = 4.5;

/// The margin the functional decomposition graph's dark fills are chosen for, so that a value cannot fall below AA on
/// the next small edit.
const MARGIN: f64 = 5.0;

/// WCAG 2.x's line for a graphical object, such as a circle or a stroke, against what it is drawn on.
const GRAPHICAL: f64 = 3.0;

/// Far enough apart that two hues never read as one, by eye, at a glance (CIE76).
const DISTINCT: f64 = 20.0;

const KINDS: [Kind; 2] = [Kind::Light, Kind::Dark];

// ---- reading the colours where they are declared --------------------------------------------------------------------

fn file(path: &str) -> String {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()))
}

fn scene(name: &str) -> String {
    file(&format!("crates/adp-shell/src/canvas/{name}"))
}

/// The `theme.rs` of a diagram's module, where the tool's colours per theme are declared.
fn module_theme(module: &str) -> String {
    file(&format!("modules/diagrams/{module}/src/theme.rs"))
}

/// The body of the palette a `theme.rs` declares for `kind`.
fn palette_of(source: &str, kind: Kind) -> &str {
    item(
        source,
        match kind {
            Kind::Light => "const LIGHT: Palette",
            Kind::Dark => "const DARK: Palette",
        },
    )
}

fn rgba(text: &str) -> Rgba {
    Rgba::parse(text).unwrap_or_else(|| panic!("{text} is not a colour"))
}

/// The braced body of the first item `header` opens, such as `const DARK: Palette` or `fn fill(`.
fn item<'a>(source: &'a str, header: &str) -> &'a str {
    let start = source
        .find(header)
        .unwrap_or_else(|| panic!("nothing declares `{header}`"));
    let open = start + source[start..].find('{').expect("the item has a body");
    let mut depth = 0;
    for (at, c) in source[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[open..=open + at];
                }
            }
            _ => {}
        }
    }
    panic!("`{header}` is not closed")
}

/// The string literals of `line`, in order.
fn literals(line: &str) -> Vec<&str> {
    line.split('"').skip(1).step_by(2).collect()
}

/// One line of a colour table: what it names (a quoted key, a field's name, or `_` for the arm that catches the rest)
/// and the colours on it, in order.
struct Entry {
    key: String,
    colours: Vec<Rgba>,
    line: String,
}

fn entries(body: &str) -> Vec<Entry> {
    body.lines()
        .filter_map(|line| {
            let found = literals(line);
            let colours: Vec<Rgba> = found.iter().filter(|l| l.starts_with('#')).map(|l| rgba(l)).collect();
            if colours.is_empty() {
                return None;
            }
            let trimmed = line.trim_start();
            let key = if let Some(quoted) = found.iter().find(|l| !l.starts_with('#')) {
                (*quoted).to_owned()
            } else if trimmed.starts_with('_') || trimmed.starts_with("(_") {
                "_".to_owned()
            } else {
                trimmed.split(':').next().unwrap_or_default().trim().to_owned()
            };
            Some(Entry {
                key,
                colours,
                line: line.trim().to_owned(),
            })
        })
        .collect()
}

/// The colour `const NAME: &str = "#..."` declares.
fn constant(source: &str, name: &str) -> Rgba {
    let header = format!("const {name}: &str = ");
    let line = source
        .lines()
        .find(|l| l.contains(&header))
        .unwrap_or_else(|| panic!("nothing declares `{header}`"));
    rgba(literals(line)[0])
}

/// The one colour the entry `key` of `body` gives.
fn field(body: &str, key: &str) -> Rgba {
    let found: Vec<Entry> = entries(body).into_iter().filter(|e| e.key == key).collect();
    assert_eq!(found.len(), 1, "one `{key}` in {body}");
    found[0].colours[0]
}

/// A vendored diagram specification.
fn notation(name: &str) -> serde_json::Value {
    serde_json::from_str(&file(&format!("vendor/etalii.adp/definitions/diagrams/{name}"))).expect("a specification")
}

/// The theme token `name` in the mode the canvas draws in under `kind`: the mode's own value where the notation
/// declares one, else the token's base value.
fn token(notation: &serde_json::Value, kind: Kind, name: &str) -> Rgba {
    let theme = &notation["notation"]["theme"];
    let value = theme["modes"][mode_of(kind)][name]
        .as_str()
        .or_else(|| theme["tokens"][name].as_str())
        .unwrap_or_else(|| panic!("the notation declares no {name}"));
    rgba(value)
}

fn mode_of(kind: Kind) -> &'static str {
    match kind {
        Kind::Light => "light",
        Kind::Dark => "dark",
    }
}

/// The tokens a module's `theme.rs` adds over its notation in `kind` (`fn tokens`, one arm per theme).
fn module_tokens(module: &str, kind: Kind) -> BTreeMap<String, Rgba> {
    let source = module_theme(module);
    let body = item(&source, "fn tokens(");
    let arm = match kind {
        Kind::Light => "Kind::Light =>",
        Kind::Dark => "Kind::Dark =>",
    };
    let start = body.find(arm).unwrap_or_else(|| panic!("{module} names no {arm} arm"));
    let rest = &body[start + arm.len()..];
    let end = rest.find("Kind::").unwrap_or(rest.len());
    rest[..end]
        .lines()
        .filter_map(|line| match literals(line)[..] {
            [name, value] => Some((name.to_owned(), rgba(value))),
            _ => None,
        })
        .collect()
}

/// The token `name` a notation-drawn tool paints with in `kind`: what its module adds, else the notation's own.
fn drawn_token(notation: &serde_json::Value, module: &str, kind: Kind, name: &str) -> Rgba {
    module_tokens(module, kind)
        .get(name)
        .copied()
        .unwrap_or_else(|| token(notation, kind, name))
}

// ---- what VS Code gives -----------------------------------------------------------------------------------------------

fn vscode(kind: Kind, id: &str) -> Rgba {
    let colour = Theme::for_kind(kind)
        .color(id)
        .unwrap_or_else(|| panic!("VS Code's {kind:?} theme gives no {id}"));
    assert_eq!(colour.a, 255, "{id} is opaque, so a ratio taken on it is the one drawn");
    colour
}

/// The text colour, which replaces the standalone host's `--color-text`.
fn text(kind: Kind) -> Rgba {
    vscode(kind, "editor.foreground")
}

/// What the canvas and the editors are drawn on, which replaces `--color-surface` and `--color-bg`.
fn surface(kind: Kind) -> Rgba {
    vscode(kind, "editor.background")
}

fn at_least(ratio: f64, line: f64, what: impl FnOnce() -> String) {
    assert!(ratio >= line, "{} at {ratio:.4}:1, below {line}:1", what());
}

// ---- theme.contrast.test.ts ---------------------------------------------------------------------------------------------

/// The functional decomposition graph's five fills, in light and dark (its module's `theme.rs`).
fn fdg(kind: Kind) -> (Rgba, Vec<(String, Rgba)>) {
    let source = module_theme("functional-decomposition-graph");
    let body = palette_of(&source, kind);
    let fills: Vec<(String, Rgba)> = entries(body)
        .into_iter()
        .filter(|e| e.key != "text" && e.key != "border")
        .map(|e| (e.key, e.colours[0]))
        .collect();
    // The liveness proof: an instrument reporting "nothing wrong" has to show it was looking.
    assert_eq!(fills.len(), 5, "{kind:?} declares the five fills");
    (field(body, "text"), fills)
}

#[test]
fn a_diagram_fill_clears_aa_under_the_text_in_both_themes() {
    for kind in KINDS {
        let (own, fills) = fdg(kind);
        for (name, fill) in fills {
            for (reference, label) in [
                (text(kind), "VS Code's editor.foreground"),
                (own, "the text the scene draws"),
            ] {
                at_least(contrast_ratio(fill, reference), AA, || {
                    format!("{name} is {fill} in {kind:?}, against {label} {reference}")
                });
            }
        }
    }
}

#[test]
fn the_dark_diagram_fills_clear_the_five_to_one_margin() {
    // The light fills are the user's own hex and are not held to this line; the dark values are chosen, and chosen
    // to this number, against VS Code's dark text.
    let (own, fills) = fdg(Kind::Dark);
    for (name, fill) in fills {
        for reference in [text(Kind::Dark), own] {
            at_least(contrast_ratio(fill, reference), MARGIN, || {
                format!("{name} is {fill}, against {reference}: below the margin the dark fills are chosen for")
            });
        }
    }
}

#[test]
fn the_light_diagram_fills_are_the_users_hex_exactly() {
    // The user ruled the light values are theirs verbatim: tuning them for contrast would undo a decision.
    let (_, fills) = fdg(Kind::Light);
    let fills: BTreeMap<String, String> = fills.into_iter().map(|(n, c)| (n, c.to_string())).collect();
    let expected: BTreeMap<String, String> = [
        ("ui-element", "#AAED92"),
        ("data-element", "#EDEDED"),
        ("action", "#9EDCFA"),
        ("function", "#86E6D9"),
        ("comment", "#FCF281"),
    ]
    .into_iter()
    .map(|(n, c)| (n.to_owned(), c.to_owned()))
    .collect();
    assert_eq!(fills, expected);
}

#[test]
fn the_hype_cycles_trigger_stands_out_from_the_canvas_and_a_note_reads_on_its_box() {
    let ghg = notation("gartner-hype-cycle-graph.dis");
    for kind in KINDS {
        let trigger = token(&ghg, kind, "ghg.trigger");
        at_least(contrast_ratio(trigger, surface(kind)), GRAPHICAL, || {
            format!("the trigger ({trigger}) on editor.background in {kind:?}")
        });
        let (note_text, note) = (token(&ghg, kind, "ghg.noteText"), token(&ghg, kind, "ghg.note"));
        at_least(contrast_ratio(note_text, note), AA, || {
            format!("the note's text ({note_text}) on its box ({note}) in {kind:?}")
        });
    }
}

/// The kind of token each of the standalone host's nine syntax colours painted, as a TextMate scope its editors'
/// grammars give it, so the colour is VS Code's for that scope.
const SYNTAX: [(&str, &str); 9] = [
    ("name", "support.type.property-name.json"),
    ("string", "string.quoted.double.json"),
    ("number", "constant.numeric.json"),
    ("keyword", "constant.language.json"),
    ("comment", "comment.line.number-sign.yaml"),
    ("punctuation", "punctuation.separator.dictionary.key-value.json"),
    ("tag", "entity.name.tag.xml"),
    ("attribute", "entity.other.attribute-name.xml"),
    ("invalid", "invalid.illegal"),
];

/// VS Code's colour for `scope`: the rule whose selector matches it most specifically, the later on a tie, else the
/// editor's text. A selector that names ancestors (`a b`) needs the ancestors, which a lone scope has not.
fn token_colour(theme: &Theme, scope: &str) -> Rgba {
    let mut best: Option<(usize, Rgba)> = None;
    for rule in &theme.token_colors {
        let Some(colour) = rule.foreground else {
            continue;
        };
        for selector in rule.scopes.iter().filter(|s| !s.contains(' ')) {
            let matches = scope == selector || scope.starts_with(&format!("{selector}."));
            let depth = selector.split('.').count();
            if matches && best.is_none_or(|(d, _)| depth >= d) {
                best = Some((depth, colour));
            }
        }
    }
    best.map(|(_, c)| c).unwrap_or_else(|| text(theme.kind))
}

#[test]
fn an_editors_syntax_colour_reads_on_the_editors_surface() {
    for kind in KINDS {
        let theme = Theme::for_kind(kind);
        assert!(theme.token_colors.len() > 50, "{} has its token colours", theme.name);
        for (name, scope) in SYNTAX {
            let colour = token_colour(&theme, scope);
            at_least(contrast_ratio(colour, surface(kind)), AA, || {
                format!("{name} ({scope}) is {colour} in {}, on editor.background", theme.name)
            });
        }
    }
}

// ---- moduleThemeContrast.test.ts -------------------------------------------------------------------------------------

#[test]
fn causal_loops_variable_pill_carries_a_readable_label() {
    let cld = notation("causal-loop-diagram.dis");
    let style = &cld["notation"]["styles"]["variable"];
    // The label is painted from a theme token, so it cannot inherit a colour the theme does not choose.
    let fill = style["fill"]["token"]
        .as_str()
        .expect("the pill is filled from a token");
    let label = style["font"]["color"]["token"]
        .as_str()
        .expect("the label is painted from a token");
    for kind in KINDS {
        let (fill, label) = (token(&cld, kind, fill), token(&cld, kind, label));
        at_least(contrast_ratio(label, fill), AA, || {
            format!("the label ({label}) on the pill ({fill}) in {kind:?}")
        });
    }
}

#[test]
fn azure_pipelines_stage_carries_a_readable_name() {
    // The scene draws a stage in the palette of the active theme.
    assert!(
        scene("pipeline_scene.rs").contains("Some(palette.stage_fill)"),
        "a stage is filled with the palette's stage_fill"
    );
    let source = module_theme("azure-devops-pipeline");
    for kind in KINDS {
        let body = palette_of(&source, kind);
        let (fill, name) = (field(body, "stage_fill"), field(body, "text"));
        at_least(contrast_ratio(name, fill), AA, || {
            format!("the stage name ({name}) on the stage box ({fill}) in {kind:?}")
        });
    }
}

#[test]
fn the_hype_cycles_chevrons_read_against_every_phase() {
    // A chevron is a stroke, not text, and a divider sits between two phases, so every fill is its own pair.
    let ghg = notation("gartner-hype-cycle-graph.dis");
    for kind in KINDS {
        let chevron = token(&ghg, kind, "ghg.chevron");
        for phase in ["peak", "trough", "slope", "plateau"] {
            let fill = token(&ghg, kind, &format!("ghg.{phase}"));
            at_least(contrast_ratio(chevron, fill), GRAPHICAL, || {
                format!("the chevron ({chevron}) on the {phase} fill ({fill}) in {kind:?}")
            });
        }
    }
}

/// The agent activity diagram's fill per kind of element, in `kind`'s mode (its module's `theme.rs`, `fn fill`); the
/// arm that catches the rest is the environment's.
fn aad_fills(kind: Kind) -> Vec<(String, Rgba)> {
    let source = module_theme("agent-activity-diagram");
    let arm = format!("Kind::{kind:?})");
    let fills: Vec<(String, Rgba)> = entries(item(&source, "fn fill("))
        .into_iter()
        .filter(|e| e.line.contains(&arm))
        .map(|e| {
            let name = if e.key == "_" { "environment".to_owned() } else { e.key };
            (name, e.colours[0])
        })
        .collect();
    let names: Vec<&str> = fills.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["project", "specification", "agent", "location", "environment"],
        "{kind:?}"
    );
    fills
}

#[test]
fn agent_activitys_text_reads_on_every_kind_of_element() {
    let source = module_theme("agent-activity-diagram");
    for kind in KINDS {
        let own = field(palette_of(&source, kind), "text");
        for (name, fill) in aad_fills(kind) {
            for (reference, label) in [
                (text(kind), "VS Code's editor.foreground"),
                (own, "the text the scene draws"),
            ] {
                at_least(contrast_ratio(reference, fill), AA, || {
                    format!("{label} ({reference}) on the {name} fill ({fill}) in {kind:?}")
                });
            }
        }
    }
}

#[test]
fn agent_activity_writes_on_an_element_in_the_text_colour_never_the_muted_one_or_a_status() {
    // Everything written on an element is small, so it is all held to the text bar; the muted colour and the status
    // colours measured between 2.4:1 and 3.8:1 on these fills. A status is told by its word, as the standalone host's.
    let source = scene("aad_scene.rs");
    let fonts: Vec<&str> = source.match_indices("font(").map(|(at, _)| &source[at..]).collect();
    assert!(fonts.len() >= 5, "the scene writes its texts with font()");
    for call in fonts {
        let call = &call[..call.find(')').unwrap_or(call.len())];
        assert!(
            call.contains("Some(palette.text"),
            "`{call})` writes on an element in something other than the text colour"
        );
    }
}

/// The supply chain's seven stage hues (its module's `theme.rs`, `fn stage_colour`), one set for both themes; the arm
/// that catches the rest is the consumer's.
fn stages() -> Vec<(String, Rgba)> {
    let found: Vec<(String, Rgba)> = entries(item(&module_theme("supply-chain"), "fn stage_colour("))
        .into_iter()
        .map(|e| {
            let name = if e.key == "_" { "Consumer".to_owned() } else { e.key };
            (name, e.colours[0])
        })
        .collect();
    let names: Vec<&str> = found.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        [
            "Source",
            "Processor",
            "Producer",
            "Integrator",
            "Hub",
            "Outlet",
            "Consumer"
        ]
    );
    found
}

/// The colour a selected element is outlined in on the canvas in `kind`.
fn selected(kind: Kind) -> Rgba {
    let source = scene("module_scene.rs");
    match kind {
        Kind::Light => constant(&source, "SELECTED"),
        Kind::Dark => constant(&source, "SELECTED_DARK"),
    }
}

/// The pairs of `colours` closer than [`DISTINCT`], to each other or to the selection in `kind`.
fn too_close(colours: &[(String, Rgba)], kind: Kind) -> Vec<String> {
    let selection = selected(kind);
    let mut close = Vec::new();
    for (at, (one, a)) in colours.iter().enumerate() {
        for (other, b) in &colours[at + 1..] {
            let distance = delta_e(*a, *b);
            if distance < DISTINCT {
                close.push(format!("{one} and {other} are {distance:.1} apart"));
            }
        }
        let distance = delta_e(*a, selection);
        if distance < DISTINCT {
            close.push(format!("{one} is {distance:.1} from the selection"));
        }
    }
    close
}

#[test]
fn supply_chains_stages_are_seven_distinct_readable_hues() {
    // The stage hues are the same in both themes; what is written on them is each theme's own.
    let stages = stages();
    let source = scene("supply_chain_scene.rs");
    let title_line = source
        .lines()
        .find(|l| l.contains("(\"stage\", node.attribute(\"stageTitle\")"))
        .expect("the stage title is written");
    assert!(
        title_line.contains("palette.on_stage"),
        "the stage title is written in on_stage"
    );
    let theme = module_theme("supply-chain");
    for kind in KINDS {
        assert_eq!(too_close(&stages, kind), Vec::<String>::new(), "{kind:?}");
        let title = field(palette_of(&theme, kind), "on_stage");
        for (stage, hue) in &stages {
            at_least(contrast_ratio(title, *hue), AA, || {
                format!("the title ({title}) on {stage} ({hue}) in {kind:?}")
            });
        }
    }
}

/// Sankey's twelve palette words in `kind`'s mode (its module's `theme.rs`, `fn palette`, light then dark on each
/// line); the arm that catches the rest is grey, the word a node that names none is drawn in.
fn sankey(kind: Kind) -> Vec<(String, Rgba)> {
    let found: Vec<(String, Rgba)> = entries(item(&module_theme("sankey"), "fn palette("))
        .into_iter()
        .map(|e| {
            assert_eq!(e.colours.len(), 2, "`{}` gives a light and a dark colour", e.line);
            let name = if e.key == "_" { "grey".to_owned() } else { e.key };
            (name, e.colours[usize::from(matches!(kind, Kind::Dark))])
        })
        .collect();
    assert_eq!(found.len(), 12, "twelve words");
    found
}

#[test]
fn sankeys_twelve_words_read_as_text_on_the_canvas_in_both_themes() {
    // Each word is the bar, the band and the label of what it colours, so it must read as text on the canvas.
    for kind in KINDS {
        for (word, colour) in sankey(kind) {
            at_least(contrast_ratio(colour, surface(kind)), AA, || {
                format!("{word} ({colour}) on editor.background in {kind:?}")
            });
        }
    }
}

#[test]
fn sankeys_twelve_words_stay_apart_and_away_from_the_selection_in_both_themes() {
    for kind in KINDS {
        assert_eq!(too_close(&sankey(kind), kind), Vec::<String>::new(), "{kind:?}");
    }
}

/// Every token a specification's notation reads: `{"token": name}` and `token('name')`.
fn tokens_read(value: &serde_json::Value, into: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if key == "token"
                    && let Some(name) = value.as_str()
                {
                    into.push(name.to_owned());
                }
                tokens_read(value, into);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| tokens_read(v, into)),
        serde_json::Value::String(text) => {
            for (at, _) in text.match_indices("token('") {
                let rest = &text[at + 7..];
                if let Some(end) = rest.find('\'') {
                    into.push(rest[..end].to_owned());
                }
            }
        }
        _ => {}
    }
}

#[test]
fn neither_causal_loop_nor_azure_pipeline_reads_a_token_its_theme_does_not_declare() {
    for name in ["causal-loop-diagram.dis", "azure-devops-pipeline.dis"] {
        let spec = notation(name);
        let mut read = Vec::new();
        tokens_read(&spec["notation"], &mut read);
        assert!(!read.is_empty(), "{name} reads its theme");
        let declared = &spec["notation"]["theme"]["tokens"];
        let missing: Vec<&String> = read.iter().filter(|t| declared.get(t.as_str()).is_none()).collect();
        assert!(missing.is_empty(), "{name} reads {missing:?}");
    }
}

// ---- every tool's colours per theme (task T087, FR-045) ---------------------------------------------------------------

/// Each canvas `adp-shell` draws by hand, with the module whose `theme.rs` gives it its colours.
const SCENES: [(&str, &str); 13] = [
    ("aad_scene.rs", "agent-activity-diagram"),
    ("ansible_scene.rs", "ansible-structure"),
    ("databricks_scene.rs", "databricks"),
    ("dotnet_scene.rs", "dotnet-dependency-graph"),
    ("fdg_scene.rs", "functional-decomposition-graph"),
    ("helm_scene.rs", "helm-chart"),
    ("pipeline_scene.rs", "azure-devops-pipeline"),
    ("rdf_scene.rs", "rdf"),
    ("sankey_scene.rs", "sankey"),
    ("sparql_scene.rs", "sparql"),
    ("supply_chain_scene.rs", "supply-chain"),
    ("wardley_scene.rs", "wardley-map"),
    ("module_scene.rs", "c4"),
];

/// The crate name of a diagram module, as `adp-shell` uses it.
fn crate_of(module: &str) -> String {
    match module {
        "agent-activity-diagram" => "agent_activity".to_owned(),
        other => other.replace('-', "_"),
    }
}

/// Every colour literal of `source` outside its tests.
fn hex_literals(source: &str) -> Vec<String> {
    let code = source.split("#[cfg(test)]").next().unwrap_or_default();
    code.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .flat_map(|l| literals(l).into_iter().filter(|x| Rgba::parse(x).is_some()))
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_canvas_drawn_by_hand_takes_its_colours_from_its_modules_theme_in_both_themes() {
    for (name, module) in SCENES {
        let source = scene(name);
        let found = hex_literals(&source);
        let allowed: &[&str] = if name == "module_scene.rs" {
            // The selection colour of every canvas, light and dark.
            &["#7c3aed", "#a78bfa"]
        } else {
            &[]
        };
        let stray: Vec<&String> = found.iter().filter(|c| !allowed.contains(&c.as_str())).collect();
        assert!(stray.is_empty(), "{name} draws in colours of its own: {stray:?}");
        assert!(
            source.contains(&format!("adp_diagram_{}::theme", crate_of(module))),
            "{name} reads its module's theme"
        );
        if module == "c4" {
            // A C4 element is painted in its model's colours, the same in both themes, as Structurizr paints it.
            continue;
        }
        let theme = module_theme(module);
        let both = (theme.contains("const LIGHT: Palette") && theme.contains("const DARK: Palette"))
            || (theme.contains("Kind::Light") && theme.contains("Kind::Dark"));
        assert!(
            both,
            "{module}'s theme.rs gives a value for the light and the dark theme"
        );
        let mut per_theme = KINDS.map(|kind| {
            if theme.contains("const LIGHT: Palette") {
                entries(palette_of(&theme, kind))
                    .into_iter()
                    .flat_map(|e| e.colours)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        });
        if per_theme[0].is_empty() {
            // A theme written as functions: light then dark on each line, or one arm per theme.
            let all = hex_literals(&theme);
            assert!(all.len() >= 2, "{module}'s theme.rs declares its colours");
            per_theme = [vec![rgba(&all[0])], vec![rgba(&all[1])]];
        }
        assert!(!per_theme[0].is_empty(), "{module} has light colours");
        assert_ne!(per_theme[0], per_theme[1], "{module}'s dark colours are its own");
    }
}

#[test]
fn a_tools_text_reads_on_its_own_surface_in_both_themes() {
    for (module, pairs) in [
        (
            "azure-devops-pipeline",
            &[("text", "surface"), ("muted", "surface"), ("text", "deployment_fill")][..],
        ),
        ("databricks", &[("text", "surface"), ("muted", "surface")][..]),
        ("supply-chain", &[("text", "surface"), ("muted", "surface")][..]),
        ("wardley-map", &[("text", "surface"), ("muted", "surface")][..]),
        ("rdf", &[("text", "surface"), ("muted", "surface")][..]),
        ("sparql", &[("text", "surface"), ("muted", "surface")][..]),
        ("ansible-structure", &[("text", "surface"), ("muted", "surface")][..]),
    ] {
        let source = module_theme(module);
        for kind in KINDS {
            let body = palette_of(&source, kind);
            for (fore, back) in pairs {
                let (f, b) = (field(body, fore), field(body, back));
                at_least(contrast_ratio(f, b), AA, || {
                    format!("{module}'s {fore} ({f}) on its {back} ({b}) in {kind:?}")
                });
            }
        }
    }
}

#[test]
fn a_tools_muted_writing_on_the_canvas_reads_on_editor_background_in_both_themes() {
    // A Wardley map's stage labels and a pipeline's job counts are written straight on the canvas.
    for (module, name) in [
        ("wardley-map", "muted"),
        ("azure-devops-pipeline", "muted"),
        ("databricks", "muted"),
    ] {
        let source = module_theme(module);
        for kind in KINDS {
            let colour = field(palette_of(&source, kind), name);
            at_least(contrast_ratio(colour, surface(kind)), AA, || {
                format!("{module}'s {name} ({colour}) on editor.background in {kind:?}")
            });
        }
    }
}

#[test]
fn a_sankey_note_reads_on_the_canvas_in_both_themes() {
    let source = module_theme("sankey");
    let notes: Vec<Rgba> = entries(item(&source, "fn note("))
        .into_iter()
        .flat_map(|e| e.colours)
        .collect();
    assert_eq!(notes.len(), 2, "a note's colour, light and dark");
    for (kind, note) in KINDS.into_iter().zip(notes) {
        at_least(contrast_ratio(note, surface(kind)), AA, || {
            format!("a note ({note}) on editor.background in {kind:?}")
        });
    }
}

#[test]
fn a_causal_loops_caption_reads_on_the_canvas_in_both_themes() {
    let cld = notation("causal-loop-diagram.dis");
    for kind in KINDS {
        for loop_kind in ["reinforcing", "balancing"] {
            let name = format!("color.loop.{loop_kind}");
            let caption = drawn_token(&cld, "causal-loop-diagram", kind, &name);
            at_least(contrast_ratio(caption, surface(kind)), AA, || {
                format!("a {loop_kind} loop's caption ({caption}) on editor.background in {kind:?}")
            });
        }
    }
    assert!(
        !module_tokens("causal-loop-diagram", Kind::Dark).is_empty(),
        "the dark theme has loop colours of its own"
    );
}

#[test]
fn the_notation_drawn_tools_without_a_dark_mode_get_one_from_their_module() {
    for (spec, module) in [
        ("dependency-graph.dis", "dependency-graph"),
        ("timeline.dis", "timeline"),
    ] {
        let declared = notation(spec);
        assert!(
            declared["notation"]["theme"]["modes"]["dark"].is_null(),
            "{spec} declares a dark mode of its own: its module need not"
        );
        let added = module_tokens(module, Kind::Dark);
        assert!(
            added.contains_key("color.surface"),
            "{module} gives the dark theme a surface"
        );
        assert!(
            module_tokens(module, Kind::Light).is_empty(),
            "{module} leaves the light theme to its notation"
        );
    }
    let graph = notation("dependency-graph.dis");
    for kind in KINDS {
        let back = drawn_token(&graph, "dependency-graph", kind, "color.surface");
        for fore in ["color.text", "color.text.muted"] {
            let colour = drawn_token(&graph, "dependency-graph", kind, fore);
            at_least(contrast_ratio(colour, back), AA, || {
                format!("the dependency graph's {fore} ({colour}) on its surface ({back}) in {kind:?}")
            });
        }
    }
    let moment = module_tokens("timeline", Kind::Dark)["color.accent"];
    at_least(contrast_ratio(moment, surface(Kind::Dark)), GRAPHICAL, || {
        format!("a timeline's moment ({moment}) on editor.background in the dark theme")
    });
}
