//! The VS Code theme mapped onto egui (etalii.adp spec 030, FR-042, FR-044, task T020).
//!
//! Every colour, stroke width, spacing and font size the shell draws with comes from an [`adp_theme::Theme`]; this is
//! the one place that turns those values into egui's `Style`. Both themes are installed, and egui follows the
//! system's light or dark setting while the window is open.

use adp_theme::{Kind, Rgba, Theme};
use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

/// A theme colour as egui's, keeping its alpha.
pub fn color32(rgba: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(rgba.r, rgba.g, rgba.b, rgba.a)
}

/// The colour of the first id the theme defines, or `fallback` when it defines none of them.
pub fn color(theme: &Theme, ids: &[&str], fallback: Color32) -> Color32 {
    theme.color_of(ids).map(color32).unwrap_or(fallback)
}

/// The workspace's roles, resolved once per theme so drawing code names a role, never a colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub foreground: Color32,
    pub editor_background: Color32,
    pub side_bar_background: Color32,
    pub side_bar_foreground: Color32,
    pub side_bar_title: Color32,
    pub activity_bar_background: Color32,
    pub status_bar_background: Color32,
    pub status_bar_foreground: Color32,
    pub border: Color32,
    pub focus: Color32,
    /// A notice over the editor, such as a file changed on disk under unsaved edits.
    pub notification_background: Color32,
    pub notification_foreground: Color32,
}

impl Palette {
    pub fn of(theme: &Theme) -> Self {
        let foreground = color(theme, &["foreground"], Color32::GRAY);
        let border = color(
            theme,
            &["panel.border", "sideBar.border", "contrastBorder"],
            Color32::TRANSPARENT,
        );
        Self {
            foreground,
            editor_background: color(theme, &["editor.background"], Color32::BLACK),
            side_bar_background: color(theme, &["sideBar.background"], Color32::BLACK),
            side_bar_foreground: color(theme, &["sideBar.foreground", "foreground"], foreground),
            side_bar_title: color(
                theme,
                &["sideBarTitle.foreground", "sideBar.foreground", "foreground"],
                foreground,
            ),
            activity_bar_background: color(theme, &["activityBar.background", "sideBar.background"], Color32::BLACK),
            status_bar_background: color(theme, &["statusBar.background"], Color32::BLACK),
            status_bar_foreground: color(theme, &["statusBar.foreground", "foreground"], foreground),
            border,
            focus: color(theme, &["focusBorder"], Color32::LIGHT_BLUE),
            notification_background: color(
                theme,
                &["notifications.background", "editorWidget.background"],
                Color32::DARK_GRAY,
            ),
            notification_foreground: color(theme, &["notifications.foreground", "foreground"], foreground),
        }
    }
}

/// egui's style for one theme.
pub fn style(theme: &Theme) -> egui::Style {
    let palette = Palette::of(theme);
    let sizes = theme.sizes;
    let mut style = egui::Style::default();
    let mut visuals = match theme.kind {
        Kind::Dark => egui::Visuals::dark(),
        Kind::Light => egui::Visuals::light(),
    };

    visuals.override_text_color = Some(palette.foreground);
    visuals.panel_fill = palette.editor_background;
    visuals.window_fill = color(
        theme,
        &["editorWidget.background", "menu.background"],
        palette.editor_background,
    );
    visuals.window_stroke = Stroke::new(
        sizes.border_width,
        color(theme, &["widget.border", "menu.border"], palette.border),
    );
    visuals.extreme_bg_color = color(theme, &["input.background"], palette.editor_background);
    visuals.faint_bg_color = color(theme, &["list.hoverBackground"], palette.editor_background);
    visuals.code_bg_color = color(theme, &["textCodeBlock.background"], palette.editor_background);
    visuals.hyperlink_color = color(theme, &["textLink.foreground"], palette.focus);
    visuals.warn_fg_color = color(theme, &["editorWarning.foreground"], Color32::YELLOW);
    visuals.error_fg_color = color(theme, &["errorForeground"], Color32::RED);
    visuals.selection.bg_fill = color(
        theme,
        &["list.activeSelectionBackground", "editor.selectionBackground"],
        palette.focus,
    );
    visuals.selection.stroke = Stroke::new(sizes.focus_outline_width, palette.focus);
    visuals.window_corner_radius = CornerRadius::same(4);
    visuals.menu_corner_radius = CornerRadius::same(4);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = palette.editor_background;
    widgets.noninteractive.weak_bg_fill = palette.editor_background;
    widgets.noninteractive.bg_stroke = Stroke::new(sizes.border_width, palette.border);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.foreground);
    let button = color(
        theme,
        &["button.secondaryBackground", "button.background"],
        palette.editor_background,
    );
    let button_text = color(
        theme,
        &["button.secondaryForeground", "button.foreground"],
        palette.foreground,
    );
    // A button fills with its weak colour; a checkbox's box with the strong one, in VS Code's checkbox colour, and
    // the border is the one VS Code draws round a secondary button and an input: a box that is not ticked is seen.
    widgets.inactive.bg_fill = color(theme, &["checkbox.background", "input.background"], button);
    widgets.inactive.weak_bg_fill = button;
    widgets.inactive.bg_stroke = Stroke::new(
        sizes.border_width.max(1.0),
        color(
            theme,
            &["button.secondaryBorder", "checkbox.border", "input.border"],
            palette.border,
        ),
    );
    widgets.inactive.fg_stroke = Stroke::new(1.0, button_text);
    let hover = color(
        theme,
        &["button.secondaryHoverBackground", "list.hoverBackground"],
        button,
    );
    widgets.hovered.bg_fill = hover;
    widgets.hovered.weak_bg_fill = hover;
    widgets.hovered.bg_stroke = Stroke::new(sizes.focus_outline_width, palette.focus);
    widgets.hovered.fg_stroke = Stroke::new(1.0, button_text);
    widgets.active.bg_fill = hover;
    widgets.active.weak_bg_fill = hover;
    widgets.active.bg_stroke = Stroke::new(sizes.focus_outline_width, palette.focus);
    widgets.active.fg_stroke = Stroke::new(1.0, button_text);
    widgets.open.bg_fill = hover;
    widgets.open.weak_bg_fill = hover;
    style.visuals = visuals;

    let body = sizes.workbench_font;
    style.text_styles = [
        (TextStyle::Small, FontId::new(body - 2.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(body, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(body, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(body + 1.0, FontFamily::Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(sizes.editor_font, FontFamily::Monospace),
        ),
    ]
    .into();
    style.spacing.interact_size.y = sizes.row_height;
    style.spacing.indent = 8.0;
    style
}

/// How far a notch of the mouse wheel scrolls, in points: the step of the browser the standalone host runs in,
/// 100 pixels (egui's own is 40 on a desktop).
pub const WHEEL_STEP: f32 = 100.0;

/// Installs both themes and the icons' font, lets egui follow the system's light or dark setting, and gives the wheel the
/// browser's step.
pub fn install(ctx: &egui::Context) {
    crate::icons::install(ctx);
    ctx.set_style_of(egui::Theme::Dark, style(&Theme::dark()));
    ctx.set_style_of(egui::Theme::Light, style(&Theme::light()));
    ctx.set_theme(egui::ThemePreference::System);
    ctx.options_mut(|o| o.input_options.line_scroll_speed = WHEEL_STEP);
}

/// The theme for egui's current light or dark setting.
pub fn current(ctx: &egui::Context) -> Theme {
    match ctx.theme() {
        egui::Theme::Dark => Theme::dark(),
        egui::Theme::Light => Theme::light(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_style_takes_its_colours_and_sizes_from_the_theme() {
        for theme in [Theme::dark(), Theme::light()] {
            let style = style(&theme);
            let expect = |id: &str| color32(theme.color(id).unwrap());
            assert_eq!(style.visuals.panel_fill, expect("editor.background"), "{}", theme.name);
            assert_eq!(style.visuals.override_text_color, Some(expect("foreground")));
            assert_eq!(style.visuals.extreme_bg_color, expect("input.background"));
            assert_eq!(
                style.visuals.selection.bg_fill,
                expect("list.activeSelectionBackground")
            );
            assert_eq!(style.visuals.selection.stroke.color, expect("focusBorder"));
            assert_eq!(style.visuals.selection.stroke.width, theme.sizes.focus_outline_width);
            assert_eq!(style.text_styles[&TextStyle::Body].size, theme.sizes.workbench_font);
            assert_eq!(style.text_styles[&TextStyle::Monospace].size, theme.sizes.editor_font);
            assert_eq!(style.spacing.interact_size.y, theme.sizes.row_height);
            assert_eq!(style.visuals.dark_mode, theme.kind == Kind::Dark);
        }
    }

    /// Walk 4, K8: a checkbox that is not ticked draws its box, in both themes.
    #[test]
    fn an_unticked_checkbox_draws_its_box() {
        for theme in [Theme::dark(), Theme::light()] {
            let ctx = egui::Context::default();
            ctx.set_style_of(egui::Theme::Dark, style(&theme));
            ctx.set_style_of(egui::Theme::Light, style(&theme));
            let background = Palette::of(&theme).editor_background;
            let mut shapes = Vec::new();
            for _ in 0..2 {
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    ui.checkbox(&mut false, "Hide empty groups");
                });
                output.textures_delta.clear();
                shapes = output.shapes;
            }
            let seen = shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect) => {
                    (rect.fill.a() > 0 && rect.fill != background)
                        || (rect.stroke.width > 0.0 && rect.stroke.color.a() > 0 && rect.stroke.color != background)
                }
                _ => false,
            });
            assert!(seen, "{}: the box of an unticked checkbox is drawn", theme.name);
        }
    }

    /// The window follows the system's light or dark setting, also when it changes while the window is open.
    #[test]
    fn the_window_follows_the_systems_light_or_dark_setting() {
        let ctx = egui::Context::default();
        install(&ctx);
        for system in [egui::Theme::Light, egui::Theme::Dark, egui::Theme::Light] {
            let input = egui::RawInput {
                system_theme: Some(system),
                ..Default::default()
            };
            let mut panel = None;
            ctx.run_ui(input, |ui| panel = Some(ui.visuals().panel_fill))
                .textures_delta
                .clear();
            let expected = match system {
                egui::Theme::Dark => Theme::dark(),
                egui::Theme::Light => Theme::light(),
            };
            assert_eq!(current(&ctx).kind, expected.kind, "{system:?}");
            assert_eq!(panel, Some(color32(expected.color("editor.background").unwrap())));
        }
    }

    #[test]
    fn the_palette_names_vscodes_roles() {
        let theme = Theme::dark();
        let palette = Palette::of(&theme);
        assert_eq!(
            palette.side_bar_background,
            color32(theme.color("sideBar.background").unwrap())
        );
        assert_eq!(
            palette.status_bar_background,
            color32(theme.color("statusBar.background").unwrap())
        );
        assert_eq!(palette.focus, color32(theme.color("focusBorder").unwrap()));
    }
}
