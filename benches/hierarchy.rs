//! A large requirement document in the window (etalii.adp spec 036, SC-005, research R10, task T056): how long the
//! shell takes to open the generated document of 100 items and 500 requirements, and the NIST SP 800-53 example of
//! 2,616 entries, to the first frame that shows them, and how long a title another program changed takes to appear.
//!
//! Run with `cargo bench -p adp-shell --bench hierarchy`. The window is headless, its frames drawn at 60 a second while
//! a change is waited for. The bounds are held in a test build by the module's `tests/large_documents.rs`; this prints
//! a release build's times beside them.

#[path = "window.rs"]
mod window;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use adp_designer_hierarchical_requirement::samples::large_document;
use window::{FRAME_BUDGET, Window, median, ms, verdict};

const RUNS: usize = 5;
/// How long a change on disk is waited for before it is called not shown.
const GIVE_UP: Duration = Duration::from_secs(10);

/// A document to open: what it is called in the report, its body as written, the text its first frame shows, the
/// title another program changes and what it changes it to, and SC-005's bound for it.
struct Case {
    name: &'static str,
    body: String,
    registration: Option<String>,
    shown: &'static str,
    title: &'static str,
    changed: &'static str,
    bound: Duration,
}

fn main() {
    let nist = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/designers/hierarchical-requirement/tests/examples/nist-sp-800-53");
    let read = |name: &str| fs::read_to_string(nist.join(name)).expect("the NIST example");
    let cases = [
        Case {
            name: "the generated document of 100 items and 500 requirements",
            body: large_document(),
            registration: None,
            shown: "Part 1 of the subject",
            title: "ITEM-0001 Part 1 of the subject",
            changed: "ITEM-0001 Part 1 changed by an agent",
            bound: Duration::from_secs(2),
        },
        Case {
            name: "the NIST SP 800-53 example of 2,616 entries",
            body: read("nist-sp-800-53.md"),
            registration: Some(read("nist-sp-800-53.adp")),
            shown: "Access Control",
            title: "ITEM-0001 Access Control",
            changed: "ITEM-0001 Access Control, changed by an agent",
            bound: Duration::from_secs(4),
        },
    ];
    println!("SC-005 over a requirement document, headless");
    for case in &cases {
        measure(case);
    }
}

/// Opens `case` in a window of its own `RUNS` times, then changes its title on disk `RUNS` times, and prints both.
fn measure(case: &Case) {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let root = folder.path().join("requirements");
    fs::create_dir_all(&root).expect("the project folder");
    let body = root.join("requirements.md");
    fs::write(&body, &case.body).expect("the body");
    if let Some(registration) = &case.registration {
        fs::write(root.join("requirements.adp"), registration).expect("the registration");
    }
    let state = folder.path().join("state");

    let opened: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let mut window = Window::new(&state);
            window.frame(Vec::new());
            window.shell.open_path(&root);
            window.frame(Vec::new());
            let began = Instant::now();
            window.shell.open_path(&body);
            window.frame(Vec::new());
            let took = began.elapsed();
            assert!(window.shows(case.shown), "the page shows {}", case.shown);
            took
        })
        .collect();
    let opened = median(&opened);
    println!(
        "  {} opened and first drawn: median {} of {RUNS}, threshold {}: {}",
        case.name,
        ms(opened),
        ms(case.bound),
        verdict(opened, case.bound)
    );

    let mut window = Window::new(&state);
    window.frame(Vec::new());
    window.shell.open_path(&root);
    window.shell.open_path(&body);
    window.frames(30);
    // The watcher's first events may still be on their way; let them come and go.
    std::thread::sleep(Duration::from_millis(500));
    window.frames(10);
    let mut shown = Vec::new();
    for run in 0..RUNS {
        let marker = format!("{} {run}", case.changed);
        let text = case.body.replacen(case.title, &marker, 1);
        let wanted = marker.trim_start_matches("ITEM-0001 ").to_owned();
        shown.push(shown_after(&mut window, &wanted, || {
            fs::write(&body, &text).expect("the body is changed")
        }));
        std::thread::sleep(Duration::from_millis(200));
        window.frames(5);
    }
    let times: Vec<Duration> = shown.iter().flatten().copied().collect();
    if times.len() < shown.len() {
        println!(
            "  {} changed on disk: shown {} of {} times within {}: MISSES (threshold {})",
            case.name,
            times.len(),
            shown.len(),
            ms(GIVE_UP),
            ms(case.bound)
        );
    } else {
        let slowest = times.iter().max().copied().unwrap_or_default();
        println!(
            "  {} changed on disk: median {}, slowest {} of {RUNS}, threshold {}: {}",
            case.name,
            ms(median(&times)),
            ms(slowest),
            ms(case.bound),
            verdict(slowest, case.bound)
        );
    }
}

/// How long after `change` the window first painted `text`, drawing a frame every 16.7 ms, or `None` when it did not
/// within [`GIVE_UP`].
fn shown_after(window: &mut Window, text: &str, change: impl FnOnce()) -> Option<Duration> {
    let began = Instant::now();
    change();
    let mut drawn_at = Instant::now();
    while began.elapsed() < GIVE_UP {
        if drawn_at.elapsed() >= FRAME_BUDGET {
            window.frame(Vec::new());
            drawn_at = Instant::now();
            if window.shows(text) {
                return Some(began.elapsed());
            }
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    None
}
