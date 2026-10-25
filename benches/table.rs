//! Scrolling a table of 10,000 rows (etalii.adp spec 030, Story 4 scenario 2, SC-006, task T066): a generated
//! Knowledge designer document opened in the shell's designer tab and scrolled from top to bottom a screen per frame,
//! headless, timing every frame and counting the bytes the process holds after each screen.
//!
//! It is a test of `adp-shell` (`cargo test -p adp-shell --release --test table -- --nocapture` prints the numbers),
//! not a criterion benchmark: the workspace has no criterion, and the bound is what the story asks. It holds two
//! things: no frame takes longer than a frame may without a person seeing it stall, and the memory held does not grow
//! with each screen scrolled - the tab holds the window's lines and nothing else (`TableSurface.tsx`: "a table of ten
//! thousand rows is a few dozen elements"). The bytes are counted by `stats_alloc`'s instrumented system allocator,
//! which this test binary alone installs.
//!
//! The shell is a binary crate, so the test includes the modules it drives - the table, the accessibility names its options editor gives, the theme and the icons' font the theme installs - by path.

// What of the table and the theme the benchmark does not drive is the shell's.
#[allow(dead_code)]
#[path = "../crates/adp-shell/src/a11y.rs"]
mod a11y;
#[allow(dead_code)]
#[path = "../crates/adp-shell/src/icons.rs"]
mod icons;
#[allow(dead_code)]
#[path = "../crates/adp-shell/src/table/mod.rs"]
mod table;
#[allow(dead_code)]
#[path = "../crates/adp-shell/src/theme.rs"]
mod theme;

use std::alloc::System;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const ROWS: usize = 10_000;
/// How far one frame scrolls: about a screen of rows at the window's size.
const SCREEN: f32 = 600.0;
/// Screens scrolled before the memory is first counted, so what is made once - glyphs, the view's lines, egui's
/// caches - is made before the count starts.
const WARM_UP: usize = 20;

/// A table of `rows` rows by rule, as the module's `large_table` test makes it: a name, a number, one of three sizes
/// and a tick on every other row.
fn generated(rows: usize, folder: &Path) -> PathBuf {
    let mut text = String::from("ded: \"0.1\"\ndesigner: etalii/knowledge\nname: Places\nproperties:\n");
    text += "  - id: p1\n    name: Name\n    type: text\n    title: true\n";
    text += "  - id: p2\n    name: People\n    type: number\n";
    text += "  - id: p3\n    name: Size\n    type: selection\n    options:\n      - id: o1\n        name: Small\n        colour: green\n      - id: o2\n        name: Medium\n        colour: blue\n      - id: o3\n        name: Large\n        colour: red\n";
    text += "  - id: p4\n    name: Seen\n    type: checkbox\n";
    text += "views:\n  - id: v1\n    name: All\nrows:\n";
    for index in 0..rows {
        text += &format!("  - id: r{index}\n    cells:\n");
        text += &format!("      - property: p1\n        text: Place {index}\n");
        text += &format!("      - property: p2\n        number: {}\n", (index * 7919) % 100_003);
        text += &format!("      - property: p3\n        option: o{}\n", 1 + (index % 3));
        if index % 2 == 0 {
            text += "      - property: p4\n        checked: true\n";
        }
    }
    let path = folder.join("places.yaml");
    std::fs::write(&path, text).expect("written");
    path
}

/// The bytes the process holds now, as the instrumented allocator counts them since `region` began.
fn held(region: &Region<'_, System>) -> isize {
    let change = region.change();
    change.bytes_allocated as isize - change.bytes_deallocated as isize
}

#[test]
fn ten_thousand_rows_scroll_top_to_bottom_without_a_stall_and_without_growing() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let path = generated(ROWS, folder.path());
    let opening = Instant::now();
    let opened_region = Region::new(GLOBAL);
    let mut tab = table::TableTab::open(&path, folder.path()).expect("the table opens");
    let opened = opening.elapsed();
    let opened_held = held(&opened_region) as f64 / 1_048_576.0;
    let ctx = egui::Context::default();
    theme::install(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    let theme = adp_theme::Theme::dark();
    let palette = theme::Palette::of(&theme);
    let frame = |tab: &mut table::TableTab| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            ..Default::default()
        };
        let started = Instant::now();
        ctx.run_ui(input, |ui| table::show(ui, tab, &theme, &palette))
            .textures_delta
            .clear();
        started.elapsed()
    };
    frame(&mut tab);
    assert_eq!(tab.row_count, ROWS + 1, "every row and the line a row is added at");

    let region = Region::new(GLOBAL);
    let screens = ((ROWS + 1) as f32 * table::ROW_HEIGHT / SCREEN).ceil() as usize;
    let mut times = Vec::with_capacity(screens);
    let mut counted = Vec::with_capacity(screens);
    let mut most_held_lines = 0;
    for screen in 0..=screens {
        tab.scroll_to = Some(screen as f32 * SCREEN);
        times.push(frame(&mut tab));
        most_held_lines = most_held_lines.max(tab.held.len());
        counted.push(held(&region));
    }
    let last = tab.window.expect("a window was asked for");
    assert_eq!(last.0 + last.1, ROWS + 1, "the last screen shows the end of the table");

    let total: Duration = times.iter().sum();
    let mean = total / times.len() as u32;
    let slowest = times.iter().max().copied().unwrap_or_default();
    let mut sorted = times.clone();
    sorted.sort();
    let p95 = sorted[sorted.len() * 95 / 100];
    let start = counted[WARM_UP];
    let end = *counted.last().expect("counted");
    let per_screen = (end - start) as f64 / (counted.len() - 1 - WARM_UP) as f64;
    let peak = counted.iter().max().copied().unwrap_or_default();
    println!(
        "opened {ROWS} rows in {} ms, holding {opened_held:.1} MiB",
        opened.as_millis()
    );
    println!(
        "{} screens: mean {:.2} ms, 95th percentile {:.2} ms, slowest {:.2} ms per frame",
        times.len(),
        mean.as_secs_f64() * 1000.0,
        p95.as_secs_f64() * 1000.0,
        slowest.as_secs_f64() * 1000.0
    );
    println!(
        "memory held after the warm-up {} KiB, at the end {} KiB, peak {} KiB: {per_screen:.1} bytes per screen scrolled; at most {most_held_lines} lines held",
        start / 1024,
        end / 1024,
        peak / 1024
    );

    // What opening holds (walk 4: the window's private bytes grew by about 360 MB for the 1.7 MB file). The FBL
    // reading kept for the byte-preserving splices is most of it: its lists are compacted once read, the id, source
    // and target reads and the large kinds of entry are boxed, and the session shares the body's model rather than
    // copying it - about 308 MiB before, 182 MiB after.
    assert!(opened_held < 220.0, "opening holds {opened_held:.1} MiB");
    // Only a window of lines is ever held.
    assert!(most_held_lines <= 80, "{most_held_lines} lines held");
    // The memory held does not grow with each screen scrolled: less than a kibibyte a screen, which a leak of one
    // row's cells per screen would already pass.
    assert!(per_screen < 1024.0, "{per_screen:.1} bytes more per screen scrolled");
    // No frame stalls: within one frame at 60 frames a second when optimised; an unoptimised build, which the
    // workspace's tests run, is held to a bound thirty times that.
    let bound = if cfg!(debug_assertions) {
        Duration::from_millis(500)
    } else {
        Duration::from_micros(16_667)
    };
    assert!(p95 < bound, "the 95th percentile frame took {p95:?}");
}
