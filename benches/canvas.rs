//! A canvas of 1,000 elements in the window (etalii.adp spec 030, SC-006, research R12, task T090): a generated
//! dependency graph opened in the shell's diagram tab and panned, zoomed and dragged on, every frame timed, against a
//! frame at 60 frames a second, which is when a person would see a stall.
//!
//! Run with `cargo bench -p adp-shell --bench canvas`. The canvas is fitted to the window, so all 1,000 elements and
//! their relations are drawn in every frame of the pan and the drag, the most a frame of this canvas draws (the view
//! culls what it does not show, and text too small to read is not drawn). Each gesture is checked to have done what it
//! does, so a fast frame that did nothing does not count: the pan moved the drawing, the zoom changed its scale and the
//! drag wrote the element's new place to the file.

#[path = "generated.rs"]
mod generated;
#[path = "window.rs"]
mod window;

use std::collections::{BTreeSet, HashMap};
use std::time::Instant;

use eframe::egui::{self, Event, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, Rect, TouchPhase};
use window::{FRAME_BUDGET, Spread, Window, ms, verdict};

/// How many frames each way a pan and a drag take: a second of pointer movement at 60 frames a second.
const STEPS: usize = 60;

fn main() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let root = folder
        .path()
        .canonicalize()
        .expect("the temporary folder")
        .join("canvas");
    std::fs::create_dir_all(&root).expect("the project folder");
    let body = generated::canvas(&root, generated::ELEMENTS);
    let original = std::fs::read_to_string(&body).expect("the diagram");
    let mut window = Window::new(&folder.path().join("state"));
    window.frame(Vec::new());
    window.shell.open_path(&root);
    window.frame(Vec::new());

    let began = Instant::now();
    window.shell.open_path(&body);
    let first = window.frame(Vec::new());
    let opened = began.elapsed();
    let settle = window.frames(30);
    let fitted = elements(&window);
    assert_eq!(
        fitted.len(),
        generated::ELEMENTS,
        "the fitted canvas draws every element"
    );
    println!(
        "SC-006 on a generated dependency graph of {} elements, all drawn",
        generated::ELEMENTS
    );
    println!(
        "  opened and first drawn in {} (its first frame {}); then {} frames: {}",
        ms(opened),
        ms(first),
        settle.len(),
        report(&Spread::of(&settle))
    );

    // Pan: a left drag on the empty canvas between two elements of the middle row, a second there and back.
    let middle = fitted.len() / 2;
    let (left, right) = (fitted[middle].center(), fitted[middle + 1].center());
    assert!((left.y - right.y).abs() < 0.5, "two elements of one row");
    let empty = left + (right - left) / 2.0;
    let away = egui::vec2(300.0, 150.0);
    let mut pan = vec![window.press(empty, PointerButton::Primary)];
    for step in 1..=STEPS {
        pan.push(window.frame(vec![Event::PointerMoved(empty + away * (step as f32 / STEPS as f32))]));
    }
    let held = nearest(&window, left + away);
    assert!(
        (held - (left + away)).length() < 1.0,
        "the drawing moved with the pointer: {left:?} -> {held:?}"
    );
    for step in (0..STEPS).rev() {
        pan.push(window.frame(vec![Event::PointerMoved(empty + away * (step as f32 / STEPS as f32))]));
    }
    pan.push(window.release(empty, PointerButton::Primary));
    let pan_spread = Spread::of(&pan);

    // Zoom: the wheel turned a notch at a time over the middle element, in ten notches and out ten, each spread over the
    // frames that follow.
    let width = fitted[middle].width();
    let mut zoom = Vec::new();
    for notch in 0..20 {
        let delta = if notch < 10 { 1.0 } else { -1.0 };
        zoom.push(window.frame(vec![
            Event::PointerMoved(left),
            Event::MouseWheel {
                unit: MouseWheelUnit::Line,
                delta: egui::vec2(0.0, delta),
                phase: TouchPhase::Move,
                source: MouseWheelSource::Wheel,
                modifiers: Modifiers::NONE,
            },
        ]));
        zoom.extend(window.frames(5));
        if notch == 9 {
            let under = element_at(&window, left).expect("the element under the pointer stays under it");
            assert!(
                under.width() > width * 2.0,
                "ten notches in zoom in: {width} -> {}",
                under.width()
            );
        }
    }
    zoom.extend(window.frames(30));
    let zoom_spread = Spread::of(&zoom);

    // Drag: the middle element, a second there and back, dropped a little off where it began so that the drop writes it.
    let element = elements(&window)[middle].center();
    let mut drag = vec![window.press(element, PointerButton::Primary)];
    for step in 1..=STEPS {
        drag.push(window.frame(vec![Event::PointerMoved(element + away * (step as f32 / STEPS as f32))]));
    }
    let carried = nearest(&window, element + away);
    assert!(
        (carried - (element + away)).length() < 2.0,
        "the element moved with the pointer: {element:?} -> {carried:?}"
    );
    for step in (0..STEPS).rev() {
        drag.push(window.frame(vec![Event::PointerMoved(element + away * (step as f32 / STEPS as f32))]));
    }
    let dropped = element + egui::vec2(4.0, 0.0);
    drag.push(window.frame(vec![Event::PointerMoved(dropped)]));
    drag.push(window.release(dropped, PointerButton::Primary));
    drag.extend(window.frames(10));
    let written = std::fs::read_to_string(&body).expect("the diagram");
    assert_ne!(written, original, "the drag wrote the element's new place");
    let drag_spread = Spread::of(&drag);

    for (gesture, spread) in [("pan", pan_spread), ("zoom", zoom_spread), ("drag", drag_spread)] {
        println!(
            "  {gesture}: {}; threshold a frame at 60 frames a second ({}): {}",
            report(&spread),
            ms(FRAME_BUDGET),
            verdict(spread.p95, FRAME_BUDGET)
        );
    }
}

/// The elements the last frame drew, top to bottom and left to right: the generated elements are alike, so their
/// bodies are the outlines of the size drawn most often, each drawn once filled and once stroked.
fn elements(window: &Window) -> Vec<Rect> {
    let key = |r: &Rect| ((r.width() * 10.0).round() as i32, (r.height() * 10.0).round() as i32);
    let mut sizes: HashMap<(i32, i32), usize> = HashMap::new();
    for outline in &window.outlines {
        *sizes.entry(key(outline)).or_default() += 1;
    }
    let Some((&size, _)) = sizes.iter().max_by_key(|(_, count)| **count) else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut found: Vec<Rect> = window
        .outlines
        .iter()
        .filter(|r| key(r) == size)
        .filter(|r| seen.insert(((r.min.x * 10.0).round() as i64, (r.min.y * 10.0).round() as i64)))
        .copied()
        .collect();
    found.sort_by(|a, b| a.min.y.total_cmp(&b.min.y).then(a.min.x.total_cmp(&b.min.x)));
    found
}

/// The centre of the element the last frame drew nearest to `at`.
fn nearest(window: &Window, at: Pos2) -> Pos2 {
    elements(window)
        .into_iter()
        .map(|r| r.center())
        .min_by(|a, b| (*a - at).length().total_cmp(&(*b - at).length()))
        .expect("an element is drawn")
}

/// The element the last frame drew under `at`.
fn element_at(window: &Window, at: Pos2) -> Option<Rect> {
    elements(window).into_iter().find(|r| r.contains(at))
}

fn report(spread: &Spread) -> String {
    format!(
        "{} frames, mean {}, 95th percentile {}, slowest {}",
        spread.count,
        ms(spread.mean),
        ms(spread.p95),
        ms(spread.slowest)
    )
}
