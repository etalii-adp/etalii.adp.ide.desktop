# etalii.adp.ide.desktop

[![Build](https://github.com/etalii-adp/etalii.adp.ide.desktop/actions/workflows/build.yml/badge.svg?branch=develop)](https://github.com/etalii-adp/etalii.adp.ide.desktop/actions/workflows/build.yml?query=branch%3Adevelop)

The ADP tools as one native desktop application, written in Rust: the standalone host's workspace and tools with no .NET runtime, browser, web view or local server. It is specified in [etalii.adp, spec 030](https://github.com/etalii-adp/etalii.adp/tree/develop/specs/030-desktop-native-rust); there is no window yet.

ADP, A Different Perspective, is a range of task-focused tools: diagrams, designers and editors. The site is at <https://etalii.net/adp/>.

## Build

```bash
cargo run -p adp-shell
cargo test --workspace
```

The specifications and definitions the host reads are vendored from etalii.adp under `vendor/etalii.adp/`, at the commit in `vendor/etalii.adp.lock`; refresh them with `python tools/sync-etalii-adp.py <commit>`.

## Licence

[Apache License 2.0](LICENSE).
