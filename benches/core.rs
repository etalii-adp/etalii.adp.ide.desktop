//! The core paths under SC-005 and SC-008 (etalii.adp spec 030, research R12, task T090), measured with criterion: the
//! workspace tree read from a folder of 5,000 files, which opening a workspace waits for, and a change made on disk by
//! another program reaching the watcher, which every view that follows the disk waits for.
//!
//! Run with `cargo bench -p adp-hierarchy --bench core`. The window's part of the same is in `benches/workspace.rs`.

#[path = "generated.rs"]
mod generated;

use std::fs;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use adp_hierarchy::{Change, Hierarchy, Watcher};
use criterion::{Criterion, criterion_group, criterion_main};

fn scan(c: &mut Criterion) {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let root = folder
        .path()
        .canonicalize()
        .expect("the temporary folder")
        .join("workspace");
    generated::workspace(&root, generated::FILES);
    c.bench_function("SC-005 the tree of a workspace of 5000 files", |b| {
        b.iter(|| Hierarchy::scan(&root).expect("the workspace is read"))
    });
}

fn change(c: &mut Criterion) {
    let folder = tempfile::tempdir().expect("a temporary folder");
    // The folder as the platform names it: the watcher reports changes by that name.
    let root = folder
        .path()
        .canonicalize()
        .expect("the temporary folder")
        .join("workspace");
    let files = generated::workspace(&root, generated::FILES);
    let watcher = Watcher::start(&root).expect("the workspace is watched");
    // The watcher's first events, of the files just written, come and go.
    std::thread::sleep(Duration::from_millis(500));
    let changed = &files[0];
    let mut group = c.benchmark_group("SC-008");
    group.sample_size(20).measurement_time(Duration::from_secs(10));
    group.bench_function("a change on disk reaches the watcher", |b| {
        b.iter_custom(|iterations| {
            let mut total = Duration::ZERO;
            for iteration in 0..iterations {
                while watcher.changes().try_recv().is_ok() {}
                let began = Instant::now();
                fs::write(changed, format!("changed {iteration}\n")).expect("the file is changed");
                loop {
                    match watcher.changes().recv_timeout(Duration::from_secs(5)) {
                        Ok(Change::Modified(path) | Change::Added(path)) if &path == changed => break,
                        Ok(_) => continue,
                        Err(RecvTimeoutError::Timeout) => panic!("the change did not reach the watcher in 5 s"),
                        Err(RecvTimeoutError::Disconnected) => panic!("the watcher stopped"),
                    }
                }
                total += began.elapsed();
            }
            total
        })
    });
    group.finish();
}

criterion_group!(benches, scan, change);
criterion_main!(benches);
