//! The start-up probe (etalii.adp spec 030, SC-005, research R12, task T090): the `adp-desktop` program started as a
//! person starts it, timed from the moment it is started to the moment its first frame is on the screen, once showing
//! the project list and once showing a workspace of 5,000 files named on its command line.
//!
//! Run with `cargo bench -p adp-shell --bench startup`, which builds the program optimised. The program is told to
//! report by `ADP_STARTUP_PROBE`, a file it writes the time its first frame was shown to and then closes, and keeps its
//! state in a folder of the probe's by `ADP_STATE`, so the person's own project list is neither read nor changed. It
//! opens a real window: on a machine without a screen the probe says it could not measure, and does not fail.

#[path = "generated.rs"]
mod generated;
#[path = "window.rs"]
mod window;

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use adp_core::state::StateFolder;
use adp_projects::ProjectList;
use window::{median, ms, verdict};

/// Runs of each start: the first is reported apart, as a start after the program was built or the machine woke is
/// slower than the ones after it.
const RUNS: usize = 6;
/// How long a start is waited for before it is called failed.
const GIVE_UP: Duration = Duration::from_secs(60);

fn main() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let root = folder
        .path()
        .canonicalize()
        .expect("the temporary folder")
        .join("workspace");
    generated::workspace(&root, generated::FILES);
    let state = folder.path().join("state");
    ProjectList::load(StateFolder::at(&state))
        .expect("the project list")
        .add(&root)
        .expect("the workspace is added to the project list");
    let program = Path::new(env!("CARGO_BIN_EXE_adp-desktop"));
    println!("SC-005: {} started, its first frame shown", program.display());

    let starts: [(&str, Option<&Path>, Duration); 2] = [
        ("the project list", None, Duration::from_secs(2)),
        ("a workspace of 5,000 files", Some(&root), Duration::from_secs(3)),
    ];
    for (what, argument, threshold) in starts {
        let mut times = Vec::new();
        for run in 0..RUNS {
            match start(program, &state, argument, &folder.path().join(format!("probe-{run}"))) {
                Ok(took) => times.push(took),
                Err(reason) => {
                    println!("  {what}: could not be measured: {reason}");
                    break;
                }
            }
        }
        if times.len() == RUNS {
            let warm = &times[1..];
            let slowest = warm.iter().max().copied().unwrap_or_default();
            println!(
                "  {what} shown: first start {}, then median {} and slowest {} of {}; threshold {}: {}",
                ms(times[0]),
                ms(median(warm)),
                ms(slowest),
                warm.len(),
                ms(threshold),
                verdict(times[0].max(slowest), threshold)
            );
        }
    }
}

/// Starts the program with `argument` and waits for it to report its first frame shown; how long that took.
fn start(program: &Path, state: &Path, argument: Option<&Path>, report: &Path) -> Result<Duration, String> {
    let _ = std::fs::remove_file(report);
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor");
    let mut command = Command::new(program);
    command
        .env("ADP_STATE", state)
        .env("ADP_STARTUP_PROBE", report)
        .env("ADP_VENDOR", vendor)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(argument) = argument {
        command.arg(argument);
    }
    let started = SystemTime::now();
    let waiting = Instant::now();
    let mut child = command.spawn().map_err(|e| format!("the program did not start: {e}"))?;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if waiting.elapsed() > GIVE_UP {
            let _ = child.kill();
            return Err(format!("no frame within {}", ms(GIVE_UP)));
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let shown = std::fs::read_to_string(report).map_err(|_| {
        let mut error = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            let _ = std::io::Read::read_to_string(&mut stderr, &mut error);
        }
        format!("the program ended ({status}) without showing a frame: {}", error.trim())
    })?;
    let shown: u128 = shown
        .trim()
        .parse()
        .map_err(|_| format!("the probe's report is not a time: {shown}"))?;
    let started = started
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    Ok(Duration::from_nanos(shown.saturating_sub(started) as u64))
}
