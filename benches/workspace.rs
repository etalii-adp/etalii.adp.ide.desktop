//! A workspace of 5,000 files in the window (etalii.adp spec 030, SC-005 and SC-008, research R12, task T090): how long
//! the shell takes to show its project list and to show a workspace of 5,000 files once opened, and how long a change
//! made on disk by another program takes to appear in the open view.
//!
//! Run with `cargo bench -p adp-shell --bench workspace`. The window is headless, so these are the shell's own parts of
//! SC-005; the start-up probe (`benches/startup.rs`) times the same from the process's start, window and all. A change
//! on disk is timed as the window shows it: frames are drawn when the window asks for one or the watcher wakes it, as
//! eframe draws them, and, separately, at 60 frames a second, as they are while a person moves the pointer.

#[path = "generated.rs"]
mod generated;
#[path = "window.rs"]
mod window;

use std::fs;
use std::time::{Duration, Instant};

use adp_core::state::StateFolder;
use adp_projects::ProjectList;
use window::{Window, median, ms, verdict};

const RUNS: usize = 5;
const LIST_THRESHOLD: Duration = Duration::from_secs(2);
const WORKSPACE_THRESHOLD: Duration = Duration::from_secs(3);
const CHANGE_THRESHOLD: Duration = Duration::from_secs(1);
/// How long a change on disk is waited for before it is called not shown.
const GIVE_UP: Duration = Duration::from_secs(5);

fn main() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    // The folder as the temporary folder names it, which on macOS is through a link (`/var` to `/private/var`): the
    // watcher reports changes by the real name there, and the window maps them back.
    let root = folder.path().join("workspace");
    let files = generated::workspace(&root, generated::FILES);
    let state = folder.path().join("state");
    ProjectList::load(StateFolder::at(&state))
        .expect("the project list")
        .add(&root)
        .expect("the workspace is added to the project list");
    println!("SC-005 and SC-008 over a generated workspace of {} files", files.len());

    // SC-005: the project list, from a window made to its first frame drawn.
    let list: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let began = Instant::now();
            let mut window = Window::new(&state);
            window.frame(Vec::new());
            assert!(window.shows("workspace"), "the project list shows the workspace");
            began.elapsed()
        })
        .collect();
    let list = median(&list);
    println!(
        "  project list shown (headless): median {} of {RUNS}, threshold {}: {}",
        ms(list),
        ms(LIST_THRESHOLD),
        verdict(list, LIST_THRESHOLD)
    );

    // SC-005: the workspace, from being opened to its first frame drawn.
    let opened: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let mut window = Window::new(&state);
            window.frame(Vec::new());
            let began = Instant::now();
            window.shell.open_path(&root);
            window.frame(Vec::new());
            let took = began.elapsed();
            assert!(window.shows("area-09"), "the explorer shows the workspace");
            took
        })
        .collect();
    let opened = median(&opened);
    println!(
        "  workspace of {} files shown once opened (headless): median {} of {RUNS}, threshold {}: {}",
        generated::FILES,
        ms(opened),
        ms(WORKSPACE_THRESHOLD),
        verdict(opened, WORKSPACE_THRESHOLD)
    );

    // SC-008: a change on disk in the open view. One window over the workspace, with a file open in a text tab.
    let mut window = Window::new(&state);
    window.frame(Vec::new());
    window.shell.open_path(&root);
    let open = files
        .iter()
        .find(|f| f.extension().is_some_and(|e| e == "md"))
        .expect("a Markdown file");
    window.shell.open_path(open);
    window.frames(30);
    // The watcher's first events may still be on their way; let them come and go.
    std::thread::sleep(Duration::from_millis(500));
    window.frames(10);

    for (paced, name) in [
        (true, "at 60 frames a second"),
        (false, "as the window asks for frames"),
    ] {
        let mut edited = Vec::new();
        let mut added = Vec::new();
        for run in 0..RUNS {
            let marker = format!("Changed on disk {run} {paced}");
            // Between changes the window waits for input: only the watcher can have it draw the change.
            assert!(
                paced || window.repaint_after.is_none(),
                "the window asks for no frame while nothing happens"
            );
            edited.push(shown_after(&mut window, paced, &marker, || {
                fs::write(open, format!("# {marker}\n")).expect("the file is changed")
            }));
            let name = format!("added-{run}-{paced}.md");
            let path = root.join(&name);
            added.push(shown_after(&mut window, paced, &name, || {
                fs::write(&path, "added\n").expect("a file is added")
            }));
            // Let the window settle before the next change.
            std::thread::sleep(Duration::from_millis(200));
            window.frames(5);
        }
        for (what, times) in [("the open tab's text", edited), ("a file added to the explorer", added)] {
            let shown: Vec<Duration> = times.iter().flatten().copied().collect();
            if shown.len() < times.len() {
                println!(
                    "  {what} changed on disk, shown {name}: shown {} of {} times within {}: MISSES (threshold {})",
                    shown.len(),
                    times.len(),
                    ms(GIVE_UP),
                    ms(CHANGE_THRESHOLD)
                );
            } else {
                let slowest = shown.iter().max().copied().unwrap_or_default();
                println!(
                    "  {what} changed on disk, shown {name}: median {}, slowest {} of {RUNS}, threshold {}: {}",
                    ms(median(&shown)),
                    ms(slowest),
                    ms(CHANGE_THRESHOLD),
                    verdict(slowest, CHANGE_THRESHOLD)
                );
            }
        }
    }
}

/// How long after `change` the window first painted `text`, or `None` when it did not within [`GIVE_UP`]. With `paced`
/// a frame is drawn every 16.7 ms; without, only when the last frame asked for one, after the delay it asked for, or
/// when the watcher woke the window.
fn shown_after(window: &mut Window, paced: bool, text: &str, change: impl FnOnce()) -> Option<Duration> {
    let began = Instant::now();
    change();
    let mut asked_at = Instant::now();
    while began.elapsed() < GIVE_UP {
        let due = if paced {
            Some(window::FRAME_BUDGET)
        } else {
            window.repaint_after
        };
        let woken = !paced && window.woken();
        if woken || due.is_some_and(|due| asked_at.elapsed() >= due) {
            window.frame(Vec::new());
            asked_at = Instant::now();
            if window.shows(text) {
                return Some(began.elapsed());
            }
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    None
}
