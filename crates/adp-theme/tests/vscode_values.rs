//! Every value of the theme equals VS Code's (etalii.adp spec 030, SC-010, task T019).

use adp_theme::{Kind, Rgba, Sizes, Theme};

/// The colour ids the workspace draws with (contracts/vscode-theme.md); each must be in both themes.
const ROLES: &[&str] = &[
    "titleBar.activeBackground",
    "titleBar.activeForeground",
    "sideBar.background",
    "sideBarSectionHeader.background",
    "list.activeSelectionBackground",
    "list.hoverBackground",
    "tab.activeBackground",
    "tab.inactiveBackground",
    "tab.border",
    "panel.background",
    "panel.border",
    "statusBar.background",
    "statusBar.foreground",
    "menu.background",
    "input.background",
    "button.background",
    "focusBorder",
    "editor.background",
    "editor.foreground",
    "editor.selectionBackground",
];

#[test]
fn every_colour_of_the_extracted_files_is_in_the_theme_unchanged() {
    for kind in [Kind::Dark, Kind::Light] {
        let theme = Theme::for_kind(kind);
        let source: serde_json::Value = serde_json::from_str(Theme::source(kind)).unwrap();
        let colors = source["colors"].as_object().unwrap();
        assert!(colors.len() > 250, "{} has only {} colours", theme.name, colors.len());
        for (id, value) in colors {
            let value = value.as_str().unwrap();
            let parsed = Rgba::parse(value).unwrap_or_else(|| panic!("{id} = {value} is not a colour"));
            assert_eq!(theme.color(id), Some(parsed), "{id}");
            assert_eq!(parsed.to_string().to_lowercase(), normalise(value), "{id}");
        }
        assert_eq!(theme.colors().count(), colors.len());
    }
}

#[test]
fn the_themes_are_vscodes_defaults_and_hold_every_role_the_workspace_draws_with() {
    let (dark, light) = (Theme::dark(), Theme::light());
    assert_eq!((dark.name.as_str(), light.name.as_str()), ("Dark 2026", "Light 2026"));
    assert_eq!(dark.vscode, light.vscode);
    for theme in [&dark, &light] {
        for role in ROLES {
            assert!(theme.color(role).is_some(), "{} has no {role}", theme.name);
        }
        assert!(!theme.token_colors.is_empty());
    }
}

#[test]
fn every_size_equals_its_row_in_sizes_md() {
    let table = include_str!("../vscode/sizes.md");
    let sizes = Sizes::VSCODE;
    let expected = [
        ("workbench_font", sizes.workbench_font),
        ("row_height", sizes.row_height),
        ("tab_height", sizes.tab_height),
        ("title_bar_height", sizes.title_bar_height),
        ("status_bar_height", sizes.status_bar_height),
        ("activity_bar_width", sizes.activity_bar_width),
        ("focus_outline_width", sizes.focus_outline_width),
        ("border_width", sizes.border_width),
        (
            if cfg!(target_os = "macos") {
                "editor_font_macos"
            } else {
                "editor_font"
            },
            sizes.editor_font,
        ),
    ];
    for (name, value) in expected {
        let row = table
            .lines()
            .find(|l| l.starts_with(&format!("| {name} |")))
            .unwrap_or_else(|| panic!("sizes.md has no row {name}"));
        let cell: f32 = row.split('|').nth(2).unwrap().trim().parse().unwrap();
        assert_eq!(value, cell, "{name}");
    }
}

#[test]
fn colours_parse_in_every_form_vscode_writes() {
    assert_eq!(
        Rgba::parse("#fff"),
        Some(Rgba {
            r: 255,
            g: 255,
            b: 255,
            a: 255
        })
    );
    assert_eq!(
        Rgba::parse("#0008"),
        Some(Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0x88
        })
    );
    assert_eq!(
        Rgba::parse("#3994BCB3"),
        Some(Rgba {
            r: 0x39,
            g: 0x94,
            b: 0xBC,
            a: 0xB3
        })
    );
    assert_eq!(Rgba::parse("red"), None);
    assert_eq!(Rgba::parse("#12345"), None);
}

/// VS Code's text, written the way `Rgba`'s `Display` writes it: six or eight hex digits, an opaque alpha dropped.
fn normalise(value: &str) -> String {
    let hex = value.trim_start_matches('#').to_lowercase();
    let full: String = match hex.len() {
        3 | 4 => hex.chars().flat_map(|c| [c, c]).collect(),
        _ => hex,
    };
    let full = if full.len() == 8 && full.ends_with("ff") {
        full[..6].to_owned()
    } else {
        full
    };
    format!("#{full}")
}
