//! Syntax highlighting with TextMate grammars, coloured by VS Code's token colours (etalii.adp spec 030, research R6,
//! FR-044, task T029). VS Code's `tokenColors` are TextMate scope selectors, so they become a syntect theme as they
//! are, and a JSON, XML, YAML or Markdown file looks as it does in VS Code.

use std::ops::Range;
use std::str::FromStr;

use adp_theme::{Kind, Rgba, Theme};
use eframe::egui::Color32;
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color, FontStyle, Highlighter as ScopeHighlighter, ScopeSelectors, StyleModifier, Theme as SyntectTheme, ThemeItem,
    ThemeSettings,
};
use syntect::parsing::{Scope, SyntaxDefinition, SyntaxSet};

/// The TextMate grammar of ATD, generated from its grammar in etalii.adp and vendored here (spec 035, FR-052, T105).
pub const ATD_SYNTAX: &str =
    include_str!("../../../vendor/etalii.adp/specifications/atd/highlighting/atd.sublime-syntax");

/// Files longer than this are shown without colour, so a large file never makes typing wait.
const MAX_HIGHLIGHTED_LINES: usize = 20_000;

/// One coloured run within a line, by byte range.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub range: Range<usize>,
    pub color: Color32,
    pub italic: bool,
}

pub struct Highlighter {
    syntaxes: SyntaxSet,
    dark: SyntectTheme,
    light: SyntectTheme,
}

impl Default for Highlighter {
    fn default() -> Self {
        let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
        if let Ok(atd) = SyntaxDefinition::load_from_str(ATD_SYNTAX, true, Some("atd")) {
            builder.add(atd);
        }
        Self {
            syntaxes: builder.build(),
            dark: syntect_theme(&Theme::dark()),
            light: syntect_theme(&Theme::light()),
        }
    }
}

impl Highlighter {
    /// The runs of every line of `text`, for a file with this extension; plain text has none.
    pub fn highlight(&self, text: &str, extension: &str, kind: Kind) -> Vec<Vec<Run>> {
        let syntax = match extension {
            "md" | "markdown" => self.syntaxes.find_syntax_by_extension("md"),
            "yml" | "yaml" => self.syntaxes.find_syntax_by_extension("yaml"),
            other => self.syntaxes.find_syntax_by_extension(other),
        };
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let Some(syntax) = syntax.filter(|_| lines.len() <= MAX_HIGHLIGHTED_LINES) else {
            return vec![Vec::new(); lines.len().max(1)];
        };
        let theme = match kind {
            Kind::Dark => &self.dark,
            Kind::Light => &self.light,
        };
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut all = Vec::with_capacity(lines.len());
        for line in lines {
            let mut runs = Vec::new();
            let mut at = 0;
            if let Ok(styles) = highlighter.highlight_line(line, &self.syntaxes) {
                for (style, piece) in styles {
                    let end = at + piece.len();
                    let c = style.foreground;
                    runs.push(Run {
                        range: at..end,
                        color: Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a),
                        italic: style.font_style.contains(FontStyle::ITALIC),
                    });
                    at = end;
                }
            }
            all.push(runs);
        }
        if text.ends_with('\n') || text.is_empty() {
            all.push(Vec::new());
        }
        all
    }

    /// The colour and italics the theme gives a TextMate scope, for colouring a semantic token by the scope VS Code
    /// maps its type to; none when the theme leaves the scope in the foreground.
    pub fn scope_style(&self, scope: &str, kind: Kind) -> Option<(Color32, bool)> {
        let theme = match kind {
            Kind::Dark => &self.dark,
            Kind::Light => &self.light,
        };
        let scope = Scope::new(scope).ok()?;
        let style = ScopeHighlighter::new(theme).style_mod_for_stack(&[scope]);
        let c = style.foreground?;
        let italic = style.font_style.is_some_and(|f| f.contains(FontStyle::ITALIC));
        Some((Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a), italic))
    }
}

/// VS Code's token colours as a syntect theme: each rule's scopes become a selector, in the theme's order, and JSON
/// keys take the property-name colour.
pub fn syntect_theme(theme: &Theme) -> SyntectTheme {
    let color = |rgba: Rgba| Color {
        r: rgba.r,
        g: rgba.g,
        b: rgba.b,
        a: rgba.a,
    };
    let scopes = theme
        .token_colors
        .iter()
        .filter(|rule| !rule.scopes.is_empty())
        .filter_map(|rule| {
            let scope = ScopeSelectors::from_str(&rule.scopes.join(", ")).ok()?;
            let mut font_style = FontStyle::empty();
            for word in rule.font_style.split_whitespace() {
                match word {
                    "italic" => font_style |= FontStyle::ITALIC,
                    "bold" => font_style |= FontStyle::BOLD,
                    "underline" => font_style |= FontStyle::UNDERLINE,
                    _ => {}
                }
            }
            Some(ThemeItem {
                scope,
                style: StyleModifier {
                    foreground: rule.foreground.map(color),
                    background: None,
                    font_style: Some(font_style),
                },
            })
        })
        .collect::<Vec<_>>();
    // Sublime's JSON grammar, which syntect ships, scopes a key as a string inside `meta.structure.dictionary.key`; VS
    // Code's grammars scope it as `support.type.property-name` and colour it as such. The key takes that colour here.
    let mut scopes = scopes;
    let property_name = theme
        .token_colors
        .iter()
        .find(|r| r.scopes.iter().any(|s| s.starts_with("support.type.property-name")) && r.foreground.is_some())
        .and_then(|r| r.foreground);
    if let (Some(foreground), Ok(scope)) = (
        property_name,
        ScopeSelectors::from_str("meta.structure.dictionary.key.json string"),
    ) {
        scopes.push(ThemeItem {
            scope,
            style: StyleModifier {
                foreground: Some(color(foreground)),
                background: None,
                font_style: None,
            },
        });
    }
    SyntectTheme {
        name: Some(theme.name.clone()),
        author: None,
        settings: ThemeSettings {
            foreground: theme.color_of(&["editor.foreground", "foreground"]).map(color),
            background: theme.color("editor.background").map(color),
            ..ThemeSettings::default()
        },
        scopes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_coloured_with_vscodes_token_colours() {
        let highlighter = Highlighter::default();
        let lines = highlighter.highlight("{ \"name\": \"ADP\", \"count\": 3 }\n", "json", Kind::Dark);
        assert_eq!(lines.len(), 2);
        let colours: std::collections::HashSet<_> = lines[0].iter().map(|r| r.color).collect();
        assert!(colours.len() >= 3, "keys, strings and numbers differ: {colours:?}");
    }

    #[test]
    fn an_unknown_extension_has_no_colour_and_one_entry_per_line() {
        let lines = Highlighter::default().highlight("a\nb\nc", "unknown-extension", Kind::Light);
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(Vec::is_empty));
    }

    #[test]
    fn atd_is_coloured_by_its_vendored_grammar() {
        let highlighter = Highlighter::default();
        let lines = highlighter.highlight("atd 0.1\n// a comment\ndiagram x/y \"Y\" {}\n", "atd", Kind::Dark);
        let colours: std::collections::HashSet<_> = lines.iter().flatten().map(|r| r.color).collect();
        assert!(colours.len() >= 3, "keywords, comments and strings differ: {colours:?}");
        assert!(highlighter.scope_style("entity.name.type", Kind::Dark).is_some());
    }

    #[test]
    fn every_token_rule_of_both_themes_becomes_a_selector() {
        for theme in [Theme::dark(), Theme::light()] {
            let rules = theme.token_colors.iter().filter(|r| !r.scopes.is_empty()).count();
            assert_eq!(syntect_theme(&theme).scopes.len(), rules + 1, "{}", theme.name);
        }
    }
}
