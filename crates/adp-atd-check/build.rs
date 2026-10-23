//! Embeds the schemas settings are checked against and the capability catalogue (ATD §13.6, §14.6), from
//! `ATD_SPEC_DIR` when set, else from the vendored specifications.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=ATD_SPEC_DIR");
    let atd = match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("../../vendor/etalii.adp/specifications/atd"),
    };
    let atd = atd.canonicalize().unwrap_or(atd);
    let specifications = atd.parent().unwrap().to_path_buf();
    let mut out = String::new();
    for (name, path) in [
        ("ATD_SCHEMA", atd.join("atd.schema.json")),
        ("DISL_SCHEMA", specifications.join("disl/disl.schema.json")),
        ("DESL_SCHEMA", specifications.join("desl/desl.schema.json")),
        ("FBL_SCHEMA", specifications.join("fbl/fbl.schema.json")),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
        writeln!(
            out,
            "pub const {name}: &str = include_str!({:?});",
            path.display().to_string()
        )
        .unwrap();
    }
    let mut contracts = Vec::new();
    if let Ok(entries) = std::fs::read_dir(atd.join("capabilities")) {
        for entry in entries.flatten() {
            let contract = entry.path().join("contract.json");
            if contract.is_file() {
                println!("cargo:rerun-if-changed={}", contract.display());
                contracts.push(contract.display().to_string());
            }
        }
    }
    contracts.sort();
    writeln!(out, "pub const CAPABILITIES: &[&str] = &[").unwrap();
    for contract in contracts {
        writeln!(out, "    include_str!({contract:?}),").unwrap();
    }
    writeln!(out, "];").unwrap();
    std::fs::write(
        PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded.rs"),
        out,
    )
    .unwrap();
}
