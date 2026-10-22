//! The window of the benchmarks (etalii.adp spec 030, research R12, task T090): the shell the `adp-desktop` binary runs,
//! drawn in headless egui frames of 1280 by 800, each frame timed from its input to its tessellated triangles, which
//! is all of a frame the processor does; the graphics card's part is not timed.
//!
//! Each benchmark includes this file by path, so what it does not use of it is allowed to be unused.

#![allow(dead_code)]

use std::time::{Duration, Instant};

use adp_core::state::StateFolder;
use adp_shell::{shell::Shell, theme};
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput};

/// One frame at 60 frames a second: what a frame may take without a person seeing it stall.
pub const FRAME_BUDGET: Duration = Duration::from_micros(16_667);

/// The shell in a headless window.
pub struct Window {
    pub ctx: egui::Context,
    pub shell: Shell,
    started: Instant,
    /// Every text the last frame painted, with where.
    pub texts: Vec<(String, egui::Rect)>,
    /// The bounds of every rectangle and closed outline the last frame painted.
    pub outlines: Vec<egui::Rect>,
    /// When the last frame asked for the next one, as eframe reads it: `None` when it asked for none, and the window
    /// then waits for input.
    pub repaint_after: Option<Duration>,
}

impl Window {
    /// A window keeping its project list in `state`, in the dark theme.
    pub fn new(state: &std::path::Path) -> Window {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        Window {
            ctx,
            shell: Shell::with_state(StateFolder::at(state)),
            started: Instant::now(),
            texts: Vec::new(),
            outlines: Vec::new(),
            repaint_after: None,
        }
    }

    /// One frame with `events`, and how long it took.
    pub fn frame(&mut self, events: Vec<Event>) -> Duration {
        let input = RawInput {
            time: Some(self.started.elapsed().as_secs_f64()),
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            events,
            ..Default::default()
        };
        let began = Instant::now();
        let shell = &mut self.shell;
        let mut output = self.ctx.run_ui(input, |ui| shell.show(ui));
        let shapes = std::mem::take(&mut output.shapes);
        let triangles = self.ctx.tessellate(shapes.clone(), output.pixels_per_point);
        let took = began.elapsed();
        std::hint::black_box(triangles);
        output.textures_delta.clear();
        self.repaint_after = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|v| v.repaint_delay)
            .filter(|d| *d < Duration::from_secs(3600));
        self.texts.clear();
        self.outlines.clear();
        for clipped in &shapes {
            painted(&clipped.shape, &mut self.texts, &mut self.outlines);
        }
        took
    }

    /// `count` frames without input.
    pub fn frames(&mut self, count: usize) -> Vec<Duration> {
        (0..count).map(|_| self.frame(Vec::new())).collect()
    }

    /// Whether the last frame painted a text containing `text`.
    pub fn shows(&self, text: &str) -> bool {
        self.texts.iter().any(|(painted, _)| painted.contains(text))
    }

    /// Where the last frame painted `text`, exactly.
    pub fn where_is(&self, text: &str) -> Option<Pos2> {
        self.texts
            .iter()
            .rev()
            .find(|(painted, _)| painted == text)
            .map(|(_, rect)| rect.center())
    }

    /// A press of `button` at `at`; the frame's time.
    pub fn press(&mut self, at: Pos2, button: PointerButton) -> Duration {
        self.frame(vec![
            Event::PointerMoved(at),
            Event::PointerButton {
                pos: at,
                button,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ])
    }

    /// The release of `button` at `at`; the frame's time.
    pub fn release(&mut self, at: Pos2, button: PointerButton) -> Duration {
        self.frame(vec![Event::PointerButton {
            pos: at,
            button,
            pressed: false,
            modifiers: Modifiers::NONE,
        }])
    }

    /// A left drag from `from` to `to` and back in `steps` frames each way, released where it began; every frame's
    /// time, the press's and the release's included.
    pub fn drag_there_and_back(&mut self, from: Pos2, to: Pos2, steps: usize) -> Vec<Duration> {
        let mut times = vec![self.press(from, PointerButton::Primary)];
        for step in (1..=steps).chain((0..steps).rev()) {
            let t = step as f32 / steps as f32;
            times.push(self.frame(vec![Event::PointerMoved(from + (to - from) * t)]));
        }
        times.push(self.release(from, PointerButton::Primary));
        times
    }
}

fn painted(shape: &egui::Shape, texts: &mut Vec<(String, egui::Rect)>, outlines: &mut Vec<egui::Rect>) {
    match shape {
        egui::Shape::Text(text) => texts.push((text.galley.text().to_owned(), text.visual_bounding_rect())),
        egui::Shape::Rect(rect) => outlines.push(rect.rect),
        egui::Shape::Path(path) if path.closed => outlines.push(egui::Rect::from_points(&path.points)),
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| painted(s, texts, outlines)),
        _ => {}
    }
}

/// The mean, the 95th percentile and the slowest of `times`.
pub struct Spread {
    pub count: usize,
    pub mean: Duration,
    pub p95: Duration,
    pub slowest: Duration,
}

impl Spread {
    pub fn of(times: &[Duration]) -> Spread {
        let mut sorted = times.to_vec();
        sorted.sort();
        Spread {
            count: sorted.len(),
            mean: sorted.iter().sum::<Duration>() / sorted.len().max(1) as u32,
            p95: sorted.get(sorted.len() * 95 / 100).copied().unwrap_or_default(),
            slowest: sorted.last().copied().unwrap_or_default(),
        }
    }
}

/// The median of `times`.
pub fn median(times: &[Duration]) -> Duration {
    let mut sorted = times.to_vec();
    sorted.sort();
    sorted[sorted.len() / 2]
}

/// `duration` in milliseconds, for the report.
pub fn ms(duration: Duration) -> String {
    format!("{:.1} ms", duration.as_secs_f64() * 1000.0)
}

/// Whether `measured` keeps within `threshold`, in words, for the report.
pub fn verdict(measured: Duration, threshold: Duration) -> &'static str {
    if measured <= threshold { "meets" } else { "MISSES" }
}
