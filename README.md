# etalii.adp.ide.desktop

[![Build](https://github.com/etalii-adp/etalii.adp.ide.desktop/actions/workflows/build.yml/badge.svg?branch=develop)](https://github.com/etalii-adp/etalii.adp.ide.desktop/actions/workflows/build.yml?query=branch%3Adevelop)

The ADP tools as one native desktop application, written in Rust: the standalone host's workspace and tools with no .NET runtime, browser, web view or local server. It is specified in [etalii.adp, spec 030](https://github.com/etalii-adp/etalii.adp/tree/develop/specs/030-desktop-native-rust).

ADP, A Different Perspective, is a range of task-focused tools: diagrams, designers and editors. The site is at <https://etalii.net/adp/>.

## Tools

Each tool type the host has a module for is in `modules/`, and the whole catalogue, with the tool types it lists without one, is `crates/adp-core/catalogue/tools.json`.

- **Diagrams** (`modules/diagrams/`): 19 modules, among them C4, mind maps, timelines, Wardley maps, Sankey diagrams, RDF graphs, SPARQL queries and Databricks bundles, jobs and pipelines.
- **Designers** (`modules/designers/`): the Knowledge designer (`etalii/knowledge`), tables in YAML, JSON or XML; and the Hierarchical requirement designer (`etalii/hierarchical-requirement`), a requirement document in Markdown drawn as one page of nested boxes ([etalii.adp, spec 036](https://github.com/etalii-adp/etalii.adp/tree/develop/specs/036-desktop-hierarchical-requirement)).
- **Editors** (`modules/editors/`): JSON, Markdown, XML and YAML, and plain text for every other file.

A causal loop diagram can be exported from the ribbon: **Export flow page** writes one self-contained web page beside the `.cld` and opens it in your browser, where the diagram moves as flows of influence, with a heatmap of where it piles up and controls for its values ([etalii.adp, spec 074](https://github.com/etalii-adp/etalii.adp/tree/develop/specs/074-desktop-cld-flow-export)). The host itself needs no browser.

## Build

```bash
cargo run -p adp-shell
cargo test --workspace
```

The specifications and definitions the host reads are vendored from etalii.adp under `vendor/etalii.adp/`, at the commit in `vendor/etalii.adp.lock`; refresh them with `python tools/sync-etalii-adp.py <commit>`.

## Licence

[Apache License 2.0](LICENSE).
