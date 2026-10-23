//! Embeds the grammar's modules, so that `atd tables` builds the language's tables without a checkout
//! (contracts/checker.md): from `ATD_SPEC_DIR` when set, else the vendored specifications.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=ATD_SPEC_DIR");
    let atd = match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("../../vendor/etalii.adp/specifications/atd"),
    };
    let grammar = atd.canonicalize().unwrap_or(atd).join("grammar");
    let mut out = String::from("pub const MODULES: &[(&str, &str)] = &[\n");
    for module in [
        "atd", "core", "cel", "script", "canvas", "form", "text", "binding", "notation",
    ] {
        let path = grammar.join(format!("{module}.lark"));
        println!("cargo:rerun-if-changed={}", path.display());
        writeln!(out, "    ({module:?}, include_str!({:?})),", path.display().to_string()).unwrap();
    }
    out.push_str("];\n");
    std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("modules.rs"), out).unwrap();
}
