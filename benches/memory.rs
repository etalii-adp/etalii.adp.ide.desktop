//! The memory probe of the desktop host (etalii.adp spec 030, SC-007, research R12, task T091): the `adp-desktop`
//! program started as a person starts it, left idle, and its memory read the way Activity Monitor reads it, at three
//! points: the project list, the workspace of 5,000 files open, and the workspace open with three documents in tabs
//! (a Markdown file, a JSON file and the 1,000-element canvas, active), which is SC-007's point.
//!
//! Run with `cargo bench -p adp-shell --bench memory`. It needs macOS, for `footprint`, and a screen, as the program
//! opens a real window. The inputs are written where `ADP_MEMORY_WORKSPACE` says and kept there, so the standalone
//! host is measured on the same files by `parity/memory/standalone.mjs`; without it they go to a temporary folder.
//! The program keeps its state in a folder of the probe's (`ADP_STATE`), so the person's own project list is neither
//! read nor changed. `ADP_MEMORY_SETTLE` is how many seconds each point is left idle before it is read (default 15).

#[path = "generated.rs"]
mod generated;
#[path = "window.rs"]
mod window;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Reads taken at each point, a second apart, of which the median is reported.
const READS: usize = 5;

fn main() {
    if !cfg!(target_os = "macos") {
        println!("SC-007: the memory probe reads memory with macOS's `footprint`; on this system it could not measure");
        return;
    }
    let temporary = tempfile::tempdir().expect("a temporary folder");
    let folder = match std::env::var_os("ADP_MEMORY_WORKSPACE") {
        Some(folder) => PathBuf::from(folder),
        None => temporary.path().to_path_buf(),
    };
    let root = inputs(&folder);
    let documents = documents(&root);
    let settle = std::env::var("ADP_MEMORY_SETTLE")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(15));
    let state = tempfile::tempdir().expect("a state folder");
    let program = Path::new(env!("CARGO_BIN_EXE_adp-desktop"));
    println!("SC-007: {} idle, its memory read by footprint", program.display());
    println!("  inputs in {}", root.display());

    let mut workspace_and_documents = vec![root.clone()];
    workspace_and_documents.extend(documents.iter().cloned());
    opens_its_tabs(&workspace_and_documents);
    let points: [(&str, Vec<PathBuf>); 3] = [
        ("the project list", Vec::new()),
        ("the workspace of 5,000 files open", vec![root.clone()]),
        (
            "the workspace open with three documents in tabs (SC-007)",
            workspace_and_documents,
        ),
    ];
    for (what, arguments) in points {
        match measure(program, state.path(), &arguments, settle) {
            Ok(memory) => println!("  {what}: {memory}"),
            Err(reason) => println!("  {what}: could not be measured: {reason}"),
        }
    }
}

/// Writes the workspace of 5,000 files and the canvas of 1,000 elements into `folder/workspace`, afresh, and returns
/// that folder by its real path, as both hosts are handed it.
fn inputs(folder: &Path) -> PathBuf {
    std::fs::create_dir_all(folder).expect("the inputs' folder");
    let root = folder.canonicalize().expect("the inputs' folder").join("workspace");
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("the previous inputs are removed");
    }
    generated::workspace(&root, generated::FILES);
    generated::canvas(&root, generated::ELEMENTS);
    root
}

/// The three documents SC-007 has open, in the order they are opened: a Markdown file, a JSON file and the canvas of
/// 1,000 elements, which is so the active tab, drawn.
fn documents(root: &Path) -> [PathBuf; 3] {
    [
        root.join("area-00").join("module-00").join("item-0000.md"),
        root.join("area-01").join("module-00").join("item-0001.json"),
        root.join("generated.dgr"),
    ]
}

/// Checks, in the headless window the other benchmarks draw, that the command line the probe gives the program opens
/// the workspace with a tab for each of the three documents, the canvas's active and drawn.
fn opens_its_tabs(arguments: &[PathBuf]) {
    let state = tempfile::tempdir().expect("a state folder");
    let mut window = window::Window::new(state.path());
    for path in arguments {
        window.shell.open_path(path);
    }
    window.frames(30);
    for name in ["generated.dgr", "item-0000.md", "item-0001.json"] {
        assert!(window.shows(name), "a tab shows {name}");
    }
    assert!(window.shows("area-09"), "the explorer shows the workspace");
    assert!(
        window.outlines.len() >= generated::ELEMENTS,
        "the canvas draws its elements"
    );
}

/// What was read at a point: the median of the reads of the process's physical footprint and of its resident size.
struct Memory {
    footprint: u64,
    resident: u64,
}

impl std::fmt::Display for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "footprint {}, resident {} (median of {READS} reads)",
            mib(self.footprint),
            mib(self.resident)
        )
    }
}

fn mib(bytes: u64) -> String {
    format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
}

/// The process ends when the probe is done with it, whatever happened.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Starts the program with `arguments`, leaves it idle for `settle`, and reads its memory.
fn measure(program: &Path, state: &Path, arguments: &[PathBuf], settle: Duration) -> Result<Memory, String> {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor");
    let mut running = Running(
        Command::new(program)
            .args(arguments)
            .env("ADP_STATE", state)
            .env("ADP_VENDOR", vendor)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("the program did not start: {e}"))?,
    );
    std::thread::sleep(settle);
    let pid = running.0.id();
    let mut footprints = Vec::new();
    let mut residents = Vec::new();
    for read in 0..READS {
        if let Some(status) = running.0.try_wait().map_err(|e| e.to_string())? {
            return Err(format!("the program ended ({status})"));
        }
        if read > 0 {
            std::thread::sleep(Duration::from_secs(1));
        }
        footprints.push(footprint(pid)?);
        residents.push(resident(pid)?);
    }
    Ok(Memory {
        footprint: median(footprints),
        resident: median(residents),
    })
}

/// The process's physical footprint in bytes, as `footprint` reports it and Activity Monitor shows it as Memory.
fn footprint(pid: u32) -> Result<u64, String> {
    let output = Command::new("footprint")
        .args(["-f", "bytes", "--noCategories", "-p", &pid.to_string()])
        .output()
        .map_err(|e| format!("footprint did not run: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|line| line.trim().strip_prefix("phys_footprint: "))
        .and_then(|value| value.trim_end_matches(" B").trim().parse().ok())
        .ok_or_else(|| format!("footprint said no footprint: {}", text.trim()))
}

/// The process's resident size in bytes, as `ps` reports it.
fn resident(pid: u32) -> Result<u64, String> {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .map_err(|e| format!("ps did not run: {e}"))?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .map(|kib| kib * 1024)
        .map_err(|_| "ps said no resident size".to_string())
}

fn median(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    values[values.len() / 2]
}
