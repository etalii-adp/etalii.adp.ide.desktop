//! Embeds the documentation of every keyword, Appendix B of the ATD document, for hover (FR-051): from
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
    let document = atd.join("ATD-specification.md");
    println!("cargo:rerun-if-changed={}", document.display());
    let text = std::fs::read_to_string(&document).unwrap_or_default();
    let appendix = text
        .split("## Appendix B")
        .nth(1)
        .unwrap_or("")
        .split("## Appendix C")
        .next()
        .unwrap_or("");
    let mut out = String::from("pub static KEYWORDS: &[(&str, &str)] = &[\n");
    for line in appendix.lines() {
        let Some(rest) = line.strip_prefix("| `") else { continue };
        let Some((keyword, doc)) = rest.split_once("` | ") else {
            continue;
        };
        let doc = doc.trim_end_matches(" |");
        writeln!(out, "    ({keyword:?}, {doc:?}),").unwrap();
    }
    out.push_str("];\n");
    std::fs::write(
        PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("keywords.rs"),
        out,
    )
    .unwrap();
}
