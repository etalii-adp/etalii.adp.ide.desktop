//! A scene painted with egui (etalii.adp spec 030, FR-018, SC-006, task T055): every primitive of an
//! `adp-scene` scene, in its order, through the view's pan and zoom, culled to what the view shows.
//!
//! What the standalone host's `DiagramCanvas.tsx` at `8b6ccfc` left to the browser's SVG renderer is done here:
//! curves and arcs are flattened into polylines at a step that follows the zoom, a fill is a convex polygon where
//! the outline is convex and an ear-clipped mesh where it is not, dashes are scaled with the zoom, and a text run is
//! set on its baseline at its anchor. Colours are the scene's, resolved for the notation's theme mode; a colour the
//! scene leaves open is the editor's foreground, as an SVG's `currentColor` is.

use std::collections::HashSet;
use std::f64::consts::TAU;

use adp_scene::definition::labels::TextAnchor;
use adp_scene::scene::{EdgePrimitive, HookKind, Owner, Primitive, ShapePrimitive, TextRun};
use adp_scene::style::{Colour, Stroke as SceneStroke};
use adp_scene::{Path, PathCommand, Point, Rect, Scene};
use eframe::egui::{self, Color32, FontId, Pos2, Shape, Stroke, epaint, text::LayoutJob, text::TextFormat};

use super::metrics::FAMILY;

/// The smallest font size, in screen points, still worth setting: below it a text is a smudge.
const SMALLEST_TEXT: f32 = 2.5;

/// How far apart, in screen points, the points of a flattened curve are at most.
const CURVE_STEP: f64 = 4.0;

/// Where the canvas is looked at from: the canvas point at the top-left corner of the surface, and the screen
/// points one canvas unit takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub origin: Point,
    pub zoom: f64,
}

impl Default for View {
    fn default() -> Self {
        View {
            origin: Point::new(0.0, 0.0),
            zoom: 1.0,
        }
    }
}

impl View {
    pub fn to_screen(self, surface: egui::Rect, point: Point) -> Pos2 {
        Pos2::new(
            surface.min.x + ((point.x - self.origin.x) * self.zoom) as f32,
            surface.min.y + ((point.y - self.origin.y) * self.zoom) as f32,
        )
    }

    pub fn to_canvas(self, surface: egui::Rect, pos: Pos2) -> Point {
        Point::new(
            self.origin.x + (pos.x - surface.min.x) as f64 / self.zoom,
            self.origin.y + (pos.y - surface.min.y) as f64 / self.zoom,
        )
    }

    /// The canvas rectangle the surface shows.
    pub fn visible(self, surface: egui::Rect) -> Rect {
        Rect::new(
            self.origin.x,
            self.origin.y,
            surface.width() as f64 / self.zoom,
            surface.height() as f64 / self.zoom,
        )
    }

    /// The screen rectangle of a canvas rectangle.
    pub fn rect_to_screen(self, surface: egui::Rect, rect: Rect) -> egui::Rect {
        egui::Rect::from_min_max(
            self.to_screen(surface, Point::new(rect.x, rect.y)),
            self.to_screen(surface, Point::new(rect.right(), rect.bottom())),
        )
    }
}

/// What the painter falls back on and adds: the editor's colours, and the elements whose anchors are shown.
pub struct Chrome<'a> {
    pub foreground: Color32,
    pub focus: Color32,
    pub anchors_of: &'a HashSet<String>,
}

/// Parses a CSS colour the way a notation writes one: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`,
/// and the names `white`, `black` and `transparent`. `None` for anything else, `none` and `currentColor` included.
pub fn parse_colour(text: &str) -> Option<Color32> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        let digit = |i: usize| u8::from_str_radix(hex.get(i..i + 1)?, 16).ok();
        let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        return match hex.len() {
            3 | 4 => {
                let channel = |i: usize| digit(i).map(|d| d * 17);
                let alpha = if hex.len() == 4 { channel(3)? } else { 255 };
                Some(Color32::from_rgba_unmultiplied(
                    channel(0)?,
                    channel(1)?,
                    channel(2)?,
                    alpha,
                ))
            }
            6 | 8 => {
                let alpha = if hex.len() == 8 { byte(6)? } else { 255 };
                Some(Color32::from_rgba_unmultiplied(byte(0)?, byte(2)?, byte(4)?, alpha))
            }
            _ => None,
        };
    }
    let lower = text.to_ascii_lowercase();
    if let Some(inner) = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.split([',', ' ', '/']).filter(|p| !p.is_empty()).collect();
        let channel = |p: &str| p.trim().parse::<f64>().ok().map(|v| v.clamp(0.0, 255.0) as u8);
        let (r, g, b) = (
            channel(parts.first()?)?,
            channel(parts.get(1)?)?,
            channel(parts.get(2)?)?,
        );
        let alpha = match parts.get(3) {
            Some(a) if a.ends_with('%') => a.trim_end_matches('%').parse::<f64>().ok()? / 100.0,
            Some(a) => a.parse::<f64>().ok()?,
            None => 1.0,
        };
        return Some(Color32::from_rgba_unmultiplied(
            r,
            g,
            b,
            (alpha.clamp(0.0, 1.0) * 255.0) as u8,
        ));
    }
    match lower.as_str() {
        "white" => Some(Color32::WHITE),
        "black" => Some(Color32::BLACK),
        "transparent" => Some(Color32::TRANSPARENT),
        _ => None,
    }
}

fn colour_of(colour: Option<&Colour>, opacity: f64) -> Option<Color32> {
    let parsed = parse_colour(colour?.value.as_deref()?)?;
    Some(parsed.gamma_multiply(opacity.clamp(0.0, 1.0) as f32))
}

/// A path as screen polylines, one per subpath, each with whether it is closed.
pub fn flatten(path: &Path, view: &View, surface: egui::Rect) -> Vec<(Vec<Pos2>, bool)> {
    let mut out: Vec<(Vec<Point>, bool)> = Vec::new();
    let mut current = Point::new(0.0, 0.0);
    let mut start = current;
    let steps = |length_in_canvas: f64| ((length_in_canvas * view.zoom / CURVE_STEP).ceil() as usize).clamp(2, 96);
    for command in &path.commands {
        match *command {
            PathCommand::MoveTo(p) => {
                out.push((vec![p], false));
                current = p;
                start = p;
            }
            PathCommand::LineTo(p) => {
                push(&mut out, current, p);
                current = p;
            }
            PathCommand::QuadTo { control, to } => {
                let n = steps(current.distance(control) + control.distance(to));
                for i in 1..=n {
                    let t = i as f64 / n as f64;
                    let u = 1.0 - t;
                    let p = Point::new(
                        u * u * current.x + 2.0 * u * t * control.x + t * t * to.x,
                        u * u * current.y + 2.0 * u * t * control.y + t * t * to.y,
                    );
                    push(&mut out, current, p);
                }
                current = to;
            }
            PathCommand::CubicTo { first, second, to } => {
                let n = steps(current.distance(first) + first.distance(second) + second.distance(to));
                let from = current;
                for i in 1..=n {
                    let t = i as f64 / n as f64;
                    let u = 1.0 - t;
                    let p = Point::new(
                        u * u * u * from.x + 3.0 * u * u * t * first.x + 3.0 * u * t * t * second.x + t * t * t * to.x,
                        u * u * u * from.y + 3.0 * u * u * t * first.y + 3.0 * u * t * t * second.y + t * t * t * to.y,
                    );
                    push(&mut out, from, p);
                }
                current = to;
            }
            PathCommand::ArcTo {
                rx,
                ry,
                rotation,
                large_arc,
                sweep,
                to,
            } => {
                for p in arc_points(current, to, rx, ry, rotation, large_arc, sweep, view.zoom) {
                    push(&mut out, current, p);
                }
                current = to;
            }
            PathCommand::Close => {
                if let Some((points, closed)) = out.last_mut() {
                    *closed = true;
                    if points.len() > 1 && points.last() == Some(&start) {
                        points.pop();
                    }
                }
                current = start;
            }
        }
    }
    out.into_iter()
        .filter(|(points, _)| points.len() > 1)
        .map(|(points, closed)| (points.into_iter().map(|p| view.to_screen(surface, p)).collect(), closed))
        .collect()
}

fn push(out: &mut Vec<(Vec<Point>, bool)>, current: Point, p: Point) {
    match out.last_mut() {
        Some((points, false)) => points.push(p),
        _ => out.push((vec![current, p], false)),
    }
}

/// The points of an SVG elliptical arc from `from` to `to`, `to` included, by its centre parameterisation (SVG 1.1,
/// appendix F.6.5).
#[allow(clippy::too_many_arguments)]
fn arc_points(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
    large_arc: bool,
    sweep: bool,
    zoom: f64,
) -> Vec<Point> {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx == 0.0 || ry == 0.0 || from == to {
        return vec![to];
    }
    let phi = rotation.to_radians();
    let (sin, cos) = phi.sin_cos();
    let (dx, dy) = ((from.x - to.x) / 2.0, (from.y - to.y) / 2.0);
    let x1 = cos * dx + sin * dy;
    let y1 = -sin * dx + cos * dy;
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let numerator = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let sign = if large_arc != sweep { 1.0 } else { -1.0 };
    let coefficient = sign * (numerator / denominator).max(0.0).sqrt();
    let (cx1, cy1) = (coefficient * rx * y1 / ry, -coefficient * ry * x1 / rx);
    let cx = cos * cx1 - sin * cy1 + (from.x + to.x) / 2.0;
    let cy = sin * cx1 + cos * cy1 + (from.y + to.y) / 2.0;
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let theta = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
    if !sweep && delta > 0.0 {
        delta -= TAU;
    } else if sweep && delta < 0.0 {
        delta += TAU;
    }
    let n = ((delta.abs() * rx.max(ry) * zoom / CURVE_STEP).ceil() as usize).clamp(2, 96);
    (1..=n)
        .map(|i| {
            if i == n {
                return to;
            }
            let t = theta + delta * i as f64 / n as f64;
            Point::new(
                cos * rx * t.cos() - sin * ry * t.sin() + cx,
                sin * rx * t.cos() + cos * ry * t.sin() + cy,
            )
        })
        .collect()
}

fn signed_area(points: &[Pos2]) -> f32 {
    let mut area = 0.0;
    for (i, a) in points.iter().enumerate() {
        let b = points[(i + 1) % points.len()];
        area += a.x * b.y - b.x * a.y;
    }
    area / 2.0
}

fn is_convex(points: &[Pos2]) -> bool {
    let n = points.len();
    if n < 4 {
        return true;
    }
    let mut sign = 0.0f32;
    for i in 0..n {
        let (a, b, c) = (points[i], points[(i + 1) % n], points[(i + 2) % n]);
        let cross = (b - a).x * (c - b).y - (b - a).y * (c - b).x;
        if cross.abs() > 1e-3 {
            if sign != 0.0 && cross.signum() != sign {
                return false;
            }
            sign = cross.signum();
        }
    }
    true
}

/// A simple polygon's triangles by ear clipping, as a mesh of one colour.
fn ear_clipped(points: &[Pos2], fill: Color32) -> epaint::Mesh {
    let mut mesh = epaint::Mesh::default();
    for p in points {
        mesh.colored_vertex(*p, fill);
    }
    let mut indices: Vec<usize> = (0..points.len()).collect();
    if signed_area(points) < 0.0 {
        indices.reverse();
    }
    let cross = |a: Pos2, b: Pos2, c: Pos2| (b - a).x * (c - a).y - (b - a).y * (c - a).x;
    let inside =
        |p: Pos2, a: Pos2, b: Pos2, c: Pos2| cross(a, b, p) >= 0.0 && cross(b, c, p) >= 0.0 && cross(c, a, p) >= 0.0;
    let mut guard = 0;
    while indices.len() > 3 && guard < points.len() * points.len() {
        guard += 1;
        let n = indices.len();
        let mut clipped = false;
        for i in 0..n {
            let (ia, ib, ic) = (indices[(i + n - 1) % n], indices[i], indices[(i + 1) % n]);
            let (a, b, c) = (points[ia], points[ib], points[ic]);
            if cross(a, b, c) <= 0.0 {
                continue;
            }
            let blocked = indices
                .iter()
                .any(|&j| j != ia && j != ib && j != ic && inside(points[j], a, b, c));
            if blocked {
                continue;
            }
            mesh.add_triangle(ia as u32, ib as u32, ic as u32);
            indices.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if indices.len() == 3 {
        mesh.add_triangle(indices[0] as u32, indices[1] as u32, indices[2] as u32);
    }
    mesh
}

fn stroke_shapes(points: Vec<Pos2>, closed: bool, stroke: Stroke, dash: &[f64], zoom: f64, out: &mut Vec<Shape>) {
    if dash.len() >= 2 && dash.iter().all(|d| *d > 0.0) {
        let mut line = points;
        if closed && let Some(first) = line.first().copied() {
            line.push(first);
        }
        out.extend(Shape::dashed_line(
            &line,
            stroke,
            (dash[0] * zoom) as f32,
            (dash[1] * zoom) as f32,
        ));
    } else if closed {
        out.push(Shape::closed_line(points, stroke));
    } else {
        out.push(Shape::line(points, stroke));
    }
}

fn egui_stroke(stroke: &SceneStroke, zoom: f64, fallback: Color32) -> Stroke {
    let colour = colour_of(stroke.colour.as_ref(), stroke.opacity).unwrap_or(fallback);
    Stroke::new(((stroke.width * zoom) as f32).max(0.5), colour)
}

fn shape(primitive: &ShapePrimitive, view: &View, surface: egui::Rect, chrome: &Chrome, out: &mut Vec<Shape>) {
    let subpaths = flatten(&primitive.path, view, surface);
    if let Some(fill) = colour_of(primitive.fill.as_ref(), primitive.opacity) {
        for (points, _) in &subpaths {
            if points.len() < 3 {
                continue;
            }
            if is_convex(points) {
                out.push(Shape::convex_polygon(points.clone(), fill, Stroke::NONE));
            } else {
                out.push(Shape::mesh(ear_clipped(points, fill)));
            }
        }
    }
    if let Some(stroke) = &primitive.stroke
        && stroke.width > 0.0
    {
        let paint = egui_stroke(stroke, view.zoom, chrome.foreground);
        let paint = Stroke::new(paint.width, paint.color.gamma_multiply(primitive.opacity as f32));
        for (points, closed) in subpaths {
            stroke_shapes(points, closed, paint, &stroke.dash, view.zoom, out);
        }
    }
}

fn edge(primitive: &EdgePrimitive, view: &View, surface: egui::Rect, chrome: &Chrome, out: &mut Vec<Shape>) {
    let stroke = egui_stroke(&primitive.stroke, view.zoom, chrome.foreground);
    for (points, closed) in flatten(&primitive.path, view, surface) {
        stroke_shapes(points, closed, stroke, &primitive.stroke.dash, view.zoom, out);
    }
    for marker in [&primitive.start, &primitive.end].into_iter().flatten() {
        for (points, closed) in flatten(&marker.path, view, surface) {
            if marker.filled && points.len() >= 3 {
                out.push(Shape::convex_polygon(
                    points,
                    stroke.color,
                    Stroke::new(stroke.width.min(1.0), stroke.color),
                ));
            } else {
                stroke_shapes(points, closed, stroke, &[], view.zoom, out);
            }
        }
    }
}

fn text(run: &TextRun, view: &View, surface: egui::Rect, chrome: &Chrome, ctx: &egui::Context, out: &mut Vec<Shape>) {
    let size = (run.font.size * view.zoom) as f32;
    if size < SMALLEST_TEXT || run.text.is_empty() {
        return;
    }
    let colour = colour_of(run.font.colour.as_ref(), 1.0).unwrap_or(chrome.foreground);
    let mut format = TextFormat::simple(FontId::new(size, FAMILY), colour);
    format.italics = run.font.italic;
    if run.font.underline {
        format.underline = Stroke::new((size / 14.0).max(1.0), colour);
    }
    let mut job = LayoutJob::default();
    job.append(&run.text, 0.0, format);
    let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
    let baseline = galley
        .rows
        .first()
        .and_then(|row| row.row.glyphs.first().map(|g| row.pos.y + g.pos.y))
        .unwrap_or(size * 0.8);
    let at = view.to_screen(surface, run.position);
    let width = galley.size().x;
    let left = match run.anchor {
        TextAnchor::Start => at.x,
        TextAnchor::Middle => at.x - width / 2.0,
        TextAnchor::End => at.x - width,
    };
    out.push(Shape::galley(Pos2::new(left, at.y - baseline), galley, colour));
}

/// The shapes `scene` paints on `surface` through `view`, culled to what the view shows. Anchors are drawn for the
/// elements `chrome` names and where the notation shows them at rest.
pub fn paint(scene: &Scene, view: &View, surface: egui::Rect, chrome: &Chrome, ctx: &egui::Context) -> Vec<Shape> {
    let mut out = Vec::new();
    for primitive in &scene.primitives {
        match primitive {
            Primitive::Shape(s) => shape(s, view, surface, chrome, &mut out),
            Primitive::Edge(e) => edge(e, view, surface, chrome, &mut out),
            Primitive::Text(t) => text(t, view, surface, chrome, ctx, &mut out),
            Primitive::Hook(hook) => {
                if let HookKind::Anchor { visible, .. } = &hook.kind {
                    let shown = match &hook.owner {
                        Owner::Element(id) => *visible || chrome.anchors_of.contains(id),
                        _ => false,
                    };
                    if shown {
                        let centre = view.to_screen(surface, hook.area.centre());
                        let radius = (hook.area.width / 2.0 * view.zoom).clamp(2.0, 6.0) as f32;
                        out.push(Shape::circle_filled(centre, radius, chrome.focus));
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_are_read_as_a_notation_writes_them() {
        assert_eq!(parse_colour("#FFF"), Some(Color32::WHITE));
        assert_eq!(parse_colour("#0F172A"), Some(Color32::from_rgb(0x0F, 0x17, 0x2A)));
        assert_eq!(
            parse_colour("#00000080"),
            Some(Color32::from_rgba_unmultiplied(0, 0, 0, 0x80))
        );
        assert_eq!(parse_colour("rgb(255, 0, 0)"), Some(Color32::from_rgb(255, 0, 0)));
        assert_eq!(parse_colour("none"), None);
    }

    #[test]
    fn an_arc_and_a_curve_are_flattened_onto_their_ends_and_the_view_maps_both_ways() {
        let view = View {
            origin: Point::new(-10.0, -10.0),
            zoom: 2.0,
        };
        let surface = egui::Rect::from_min_size(Pos2::new(100.0, 50.0), egui::vec2(400.0, 300.0));
        let path = Path::new()
            .move_to(Point::new(0.0, 0.0))
            .arc_to(10.0, 10.0, 0.0, false, true, Point::new(20.0, 0.0))
            .cubic_to(Point::new(30.0, 0.0), Point::new(30.0, 10.0), Point::new(40.0, 10.0));
        let flat = flatten(&path, &view, surface);
        assert_eq!(flat.len(), 1);
        let points = &flat[0].0;
        assert_eq!(points.first(), Some(&Pos2::new(120.0, 70.0)));
        assert_eq!(points.last(), Some(&view.to_screen(surface, Point::new(40.0, 10.0))));
        // The half circle bulges up (y runs down, the sweep is clockwise).
        assert!(points.iter().any(|p| p.y < 60.0));
        let back = view.to_canvas(surface, view.to_screen(surface, Point::new(3.0, 4.0)));
        assert!((back.x - 3.0).abs() < 1e-4 && (back.y - 4.0).abs() < 1e-4);
    }

    #[test]
    fn a_concave_outline_is_filled_by_triangles_that_cover_it() {
        let l = [
            Pos2::new(0.0, 0.0),
            Pos2::new(20.0, 0.0),
            Pos2::new(20.0, 10.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(10.0, 20.0),
            Pos2::new(0.0, 20.0),
        ];
        assert!(!is_convex(&l));
        let mesh = ear_clipped(&l, Color32::RED);
        assert_eq!(mesh.indices.len(), 3 * 4, "a hexagon is four triangles");
    }
}
