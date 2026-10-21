//! Themes, paints, strokes, fonts and styles (DISL sections 6.2 to 6.6), resolved for one element.
//!
//! The standalone host paints with its modules' stylesheets: `compileNotation.ts` hands an element its classes, and
//! a module's test proves its theme tokens equal the stylesheet's custom properties. This host has no stylesheet, so
//! a style is resolved here, from the notation itself: a named style with its `extends` merged beneath it
//! (`styleOf`), a subtype's notation merged per key over its supertype's (`mergedPerKey`, DISL section 6.6
//! *Resolution of a visual property*), and the node's `conditions` merged over that while they hold. A colour
//! keeps the token it names beside the value the theme gives it in the active mode, so the shell may repaint by
//! token. What is not drawn here: gradients, patterns and images (section 6.3) resolve to no paint, and shadows,
//! filters and blend modes are not read.

use adp_cel::Value;
use adp_disl::DislSpecification;
use adp_disl::expressions::contexts;
use serde_json::{Map, Value as Json};

use crate::evaluation::{Evaluator, Vars};
use crate::text::LABEL_FONT_SIZE;

/// The standard tokens' light values where DISL states one (Appendix B.1) and the specification gives none.
const STANDARD_TOKENS: [(&str, &str); 5] = [
    ("color.surface", "#FFFFFF"),
    ("color.border", "#5F5E5A"),
    ("color.accent", "#534AB7"),
    ("color.danger", "#E24B4A"),
    ("color.warning", "#BA7517"),
];

/// A notation's theme in one mode: its tokens with the mode's overrides merged over them.
#[derive(Debug, Clone, Default)]
pub struct Theme {
    tokens: Map<String, Json>,
    mode: Option<String>,
}

impl Theme {
    /// The theme of `specification` in `mode`; its `defaultMode` when none is given.
    pub fn of(specification: &DislSpecification, mode: Option<&str>) -> Theme {
        let theme = specification
            .root()
            .get("notation")
            .and_then(|n| n.get("theme"))
            .cloned()
            .unwrap_or(Json::Null);
        let mut tokens = theme
            .get("tokens")
            .and_then(Json::as_object)
            .cloned()
            .unwrap_or_default();
        let mode = mode
            .map(str::to_owned)
            .or_else(|| theme.get("defaultMode").and_then(Json::as_str).map(str::to_owned));
        if let Some(overrides) = mode
            .as_deref()
            .and_then(|m| theme.get("modes").and_then(|modes| modes.get(m)))
            .and_then(Json::as_object)
        {
            for (name, value) in overrides {
                tokens.insert(name.clone(), value.clone());
            }
        }
        Theme { tokens, mode }
    }

    pub fn mode(&self) -> Option<&str> {
        self.mode.as_deref()
    }

    fn json_of(&self, name: &str) -> Option<Json> {
        self.tokens.get(name).cloned().or_else(|| {
            STANDARD_TOKENS
                .iter()
                .find(|(token, _)| *token == name)
                .map(|(_, value)| Json::String((*value).to_owned()))
        })
    }

    /// The token's value as CEL reads it from `token(name)`.
    pub fn value_of(&self, name: &str) -> Option<Value> {
        match self.json_of(name)? {
            Json::String(s) => Some(Value::String(s)),
            Json::Number(n) => Some(
                n.as_i64()
                    .map(Value::Int)
                    .unwrap_or(Value::Double(n.as_f64().unwrap_or(0.0))),
            ),
            Json::Bool(b) => Some(Value::Bool(b)),
            _ => None,
        }
    }

    /// The token's value as text, for a colour or a font family.
    pub fn text_of(&self, name: &str) -> Option<String> {
        match self.json_of(name)? {
            Json::String(s) => Some(s),
            other => Some(other.to_string()),
        }
    }

    /// The token's value as a number, for a width or a size.
    pub fn number_of(&self, name: &str) -> Option<f64> {
        self.json_of(name)?.as_f64()
    }
}

/// A colour: the theme token it names, and its value in the active mode where the theme has one, or a value given
/// outright.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Colour {
    pub token: Option<String>,
    pub value: Option<String>,
}

impl Colour {
    pub fn token(theme: &Theme, name: &str) -> Colour {
        Colour {
            token: Some(name.to_owned()),
            value: theme.text_of(name),
        }
    }

    pub fn value(value: &str) -> Colour {
        Colour {
            token: None,
            value: Some(value.to_owned()),
        }
    }
}

/// How an outline or a line is drawn (DISL section 6.4).
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub colour: Option<Colour>,
    pub width: f64,
    /// Dash and gap lengths in diagram units; empty for a solid line.
    pub dash: Vec<f64>,
    pub opacity: f64,
}

/// A font weight from 100 to 900.
pub type FontWeight = u16;

/// How a text is set (DISL section 6.5).
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    pub family: Option<String>,
    pub size: f64,
    pub weight: FontWeight,
    pub italic: bool,
    pub colour: Option<Colour>,
    pub underline: bool,
}

impl Font {
    /// The canvas library's label font: 12 units, normal, in the theme's text colour.
    pub fn label(theme: &Theme) -> Font {
        Font {
            family: theme.text_of("font.body"),
            size: LABEL_FONT_SIZE,
            weight: 400,
            italic: false,
            colour: Some(Colour::token(theme, "color.text")),
            underline: false,
        }
    }
}

/// A style resolved for one element.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedStyle {
    pub fill: Option<Colour>,
    pub stroke: Option<Stroke>,
    pub corner_radius: Option<f64>,
    pub opacity: f64,
    pub font: Font,
}

/// A value DISL binds rather than writes, which a subtype's replaces whole rather than merging into.
fn is_bound(value: &Json) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.contains_key("cel") || o.contains_key("token") || o.contains_key("attribute"))
}

/// `own` merged per key over `inherited`: an object `own` states is merged into the one it inherits, anything else
/// it states replaces it (`mergedPerKey`).
pub fn merged_per_key(inherited: &Json, own: &Json) -> Json {
    match (inherited.as_object(), own.as_object()) {
        (Some(base), Some(over)) if !is_bound(own) && !is_bound(inherited) => {
            let mut merged = base.clone();
            for (key, value) in over {
                let next = match merged.get(key) {
                    Some(existing) => merged_per_key(existing, value),
                    None => value.clone(),
                };
                merged.insert(key.clone(), next);
            }
            Json::Object(merged)
        }
        _ => own.clone(),
    }
}

/// A style as it is drawn: a named style of the notation or one written in place, its `extends` merged beneath it
/// left to right (`styleOf`). Where the standalone refuses a style that extends itself, the `extends` that leads
/// back is left out here, so the style is drawn with what it states of its own.
pub fn style_of(notation: &Json, style: &Json) -> Option<Json> {
    style_within(notation, style, &mut Vec::new())
}

fn style_within(notation: &Json, style: &Json, seen: &mut Vec<String>) -> Option<Json> {
    if let Some(name) = style.as_str() {
        if seen.iter().any(|s| s == name) {
            return None;
        }
        seen.push(name.to_owned());
        let named = notation.get("styles").and_then(|s| s.get(name))?.clone();
        let resolved = style_within(notation, &named, seen);
        seen.pop();
        return resolved;
    }
    let object = style.as_object()?;
    let mut own = object.clone();
    let bases = own.remove("extends");
    let bases: Vec<Json> = match bases {
        Some(Json::Array(list)) => list,
        Some(single) => vec![single],
        None => Vec::new(),
    };
    let inherited = bases
        .iter()
        .filter_map(|base| style_within(notation, base, seen))
        .reduce(|merged, base| merged_per_key(&merged, &base));
    let own = Json::Object(own);
    Some(match inherited {
        Some(inherited) => merged_per_key(&inherited, &own),
        None => own,
    })
}

/// Resolves the values a style binds, for one element: tokens through the theme, CEL in the element context.
pub struct StyleResolver<'a> {
    pub evaluator: &'a Evaluator,
    pub vars: &'a Vars,
}

impl StyleResolver<'_> {
    fn theme(&self) -> &Theme {
        self.evaluator.theme()
    }

    /// A Bindable value (DISL section 2.6) as CEL gives it: a literal, a token, an attribute of `self`, or CEL.
    pub fn bound(&self, value: &Json) -> Option<Value> {
        match value {
            Json::Object(o) => {
                if let Some(token) = o.get("token").and_then(Json::as_str) {
                    return self.theme().value_of(token);
                }
                if let Some(attribute) = o.get("attribute").and_then(Json::as_str) {
                    let element = self.vars.get("self")?;
                    return adp_disl::DislElement::from_value(element).map(|e| e.value_of(attribute));
                }
                if let Some(cel) = o.get("cel").and_then(Json::as_str) {
                    return self.evaluator.evaluate(contexts::ELEMENT, cel, self.vars).ok();
                }
                None
            }
            Json::String(s) => Some(Value::String(s.clone())),
            Json::Number(n) => n.as_f64().map(Value::Double),
            Json::Bool(b) => Some(Value::Bool(*b)),
            _ => None,
        }
    }

    /// A bound number.
    pub fn number(&self, value: &Json) -> Option<f64> {
        self.bound(value)?.as_number()
    }

    /// A paint (DISL section 6.3): none for `none`, a gradient, a pattern or an image.
    pub fn paint(&self, value: &Json) -> Option<Colour> {
        match value {
            Json::String(s) if s == "none" => None,
            Json::String(s) => Some(Colour::value(s)),
            Json::Object(o) => {
                if let Some(token) = o.get("token").and_then(Json::as_str) {
                    return Some(Colour::token(self.theme(), token));
                }
                if o.contains_key("type") {
                    return None;
                }
                match self.bound(value)? {
                    Value::String(s) if s == "none" => None,
                    Value::String(s) => Some(Colour::value(&s)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// A stroke (DISL section 6.4) over the defaults given, which a stated `none` colour removes.
    pub fn stroke(&self, value: Option<&Json>, default: Option<Stroke>) -> Option<Stroke> {
        let Some(value) = value.filter(|v| v.is_object()) else {
            return default;
        };
        let mut stroke = default.unwrap_or(Stroke {
            colour: Some(Colour::token(self.theme(), "color.border")),
            width: 1.0,
            dash: Vec::new(),
            opacity: 1.0,
        });
        if let Some(colour) = value.get("color") {
            stroke.colour = self.paint(colour);
        }
        if let Some(width) = value.get("width").and_then(|w| self.number(w)) {
            stroke.width = width;
        }
        if let Some(opacity) = value.get("opacity").and_then(Json::as_f64) {
            stroke.opacity = opacity;
        }
        if let Some(dash) = value.get("dash") {
            let scale = if value.get("dashScale").and_then(Json::as_str) == Some("absolute") {
                1.0
            } else {
                stroke.width
            };
            stroke.dash = self.dash(dash, scale);
        }
        Some(stroke)
    }

    /// A dash style: an explicit list as written (the standalone draws the numbers it is given), a preset in multiples
    /// of `scale` (DISL section 6.4), or CEL that gives a list.
    pub fn dash(&self, value: &Json, scale: f64) -> Vec<f64> {
        let preset: &[f64] = match value.as_str() {
            Some("dashed") => &[4.0, 3.0],
            Some("long-dash") => &[8.0, 4.0],
            Some("short-dash") => &[2.0, 2.0],
            Some("dotted") => &[0.1, 2.0],
            Some("dash-dot") => &[6.0, 3.0, 0.1, 3.0],
            Some("dash-dot-dot") => &[6.0, 3.0, 0.1, 3.0, 0.1, 3.0],
            Some(_) => &[],
            None => {
                if let Some(list) = value.as_array() {
                    return list.iter().filter_map(Json::as_f64).collect();
                }
                return match self.bound(value) {
                    Some(Value::List(items)) => items.iter().filter_map(Value::as_number).collect(),
                    _ => Vec::new(),
                };
            }
        };
        preset.iter().map(|length| length * scale).collect()
    }

    /// A font (DISL section 6.5) over `base`.
    pub fn font(&self, value: Option<&Json>, base: Font) -> Font {
        let Some(value) = value.filter(|v| v.is_object()) else {
            return base;
        };
        let mut font = base;
        if let Some(family) = value
            .get("family")
            .and_then(|f| self.bound(f))
            .and_then(|f| f.as_str().map(str::to_owned))
        {
            font.family = Some(family);
        }
        if let Some(size) = value.get("size").and_then(|s| self.number(s)) {
            font.size = size;
        }
        match value.get("weight") {
            Some(Json::String(w)) if w == "bold" => font.weight = 700,
            Some(Json::String(w)) if w == "normal" => font.weight = 400,
            Some(Json::Number(n)) => font.weight = n.as_u64().unwrap_or(400) as FontWeight,
            _ => {}
        }
        if let Some(style) = value.get("style").and_then(Json::as_str) {
            font.italic = style == "italic";
        }
        if let Some(colour) = value.get("color") {
            font.colour = self.paint(colour);
        }
        if let Some(decoration) = value.get("decoration").and_then(Json::as_str) {
            font.underline = decoration == "underline";
        }
        font
    }

    /// A style's JSON as drawn, over the defaults of DISL Appendix B.1 for a node (`node`) or a line.
    pub fn resolve(&self, style: Option<&Json>, node: bool) -> ResolvedStyle {
        let theme = self.theme();
        let empty = Json::Object(Map::new());
        let style = style.unwrap_or(&empty);
        let default_stroke = Stroke {
            colour: Some(Colour::token(theme, "color.border")),
            width: 1.0,
            dash: Vec::new(),
            opacity: 1.0,
        };
        let fill = if node {
            match style.get("fill") {
                Some(paint) => self.paint(paint),
                None => Some(Colour::token(theme, "color.surface")),
            }
        } else {
            style.get("fill").and_then(|paint| self.paint(paint))
        };
        ResolvedStyle {
            fill,
            stroke: self.stroke(style.get("stroke"), Some(default_stroke)),
            corner_radius: style.get("cornerRadius").and_then(|r| self.number(r)),
            opacity: style.get("opacity").and_then(Json::as_f64).unwrap_or(1.0),
            font: self.font(style.get("font"), Font::label(theme)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_style_is_merged_over_the_styles_it_extends_left_to_right() {
        let notation = json!({ "styles": {
            "base": { "fill": { "token": "color.surface" }, "stroke": { "width": 1, "color": "#000" } },
            "missing": { "stroke": { "width": 1.5, "dash": [3, 3] } },
            "missingDependency": { "extends": "missing", "stroke": { "width": 2, "dash": [2, 6] } },
            "both": { "extends": ["base", "missing"], "font": { "size": 13 } },
            "loop": { "extends": "loop" }
        }});
        assert_eq!(
            Some(json!({ "stroke": { "width": 2, "dash": [2, 6] } })),
            style_of(&notation, &json!("missingDependency"))
        );
        assert_eq!(
            Some(
                json!({ "fill": { "token": "color.surface" }, "stroke": { "width": 1.5, "color": "#000", "dash": [3, 3] }, "font": { "size": 13 } })
            ),
            style_of(&notation, &json!("both"))
        );
        assert_eq!(Some(json!({})), style_of(&notation, &json!("loop")));
        assert_eq!(
            Some(json!({ "stroke": { "dash": [4, 1] } })),
            style_of(&notation, &json!({ "stroke": { "dash": [4, 1] } }))
        );
    }

    #[test]
    fn a_bound_value_is_replaced_whole_rather_than_merged_into() {
        let merged = merged_per_key(
            &json!({ "fill": { "token": "a" }, "stroke": { "width": 1, "color": "#000" } }),
            &json!({ "fill": { "cel": "'#fff'" }, "stroke": { "width": 2 } }),
        );
        assert_eq!(
            json!({ "fill": { "cel": "'#fff'" }, "stroke": { "width": 2, "color": "#000" } }),
            merged
        );
    }
}
