# etalii.adp.ide.desktop

The desktop host: ADP's tools as one native desktop application in Rust, at parity with `etalii.adp.ide.standalone`. Every ADP tool is a diagram, a designer or an editor; these words, and the specification and definition languages (DISL/DID, DESL/DED, EDSL/EDD, FBL), mean what [ADP terminology](https://github.com/etalii-adp/etalii.adp/blob/develop/docs/terminology.md) says they mean, which is their single source.

## How work is done here: spec-driven development (GitHub Spec Kit)

Every change starts as a specification, written in [etalii.adp](https://github.com/etalii-adp/etalii.adp) rather than here: this repository has no Spec Kit setup of its own. Its first feature is [`specs/030-desktop-native-rust/`](https://github.com/etalii-adp/etalii.adp/tree/develop/specs/030-desktop-native-rust), whose plan and tasks say how this repository is built. A feature's tasks name files here as `etalii.adp.ide.desktop/...`, and the code arrives here in a pull request of its own, on a branch named as the feature's. That pull request's description links the feature's folder in etalii.adp and names the etalii.adp commit its tasks were taken from, and the tasks are ticked in etalii.adp only once it is merged. Work on it from etalii.adp's folder, with this repository's clone beside it, or set `SPECIFY_INIT_DIR` to etalii.adp's folder.

This repository's principles are in etalii.adp, in [`.specify/memory/repositories/etalii.adp.ide.desktop.md`](https://github.com/etalii-adp/etalii.adp/blob/develop/.specify/memory/repositories/etalii.adp.ide.desktop.md): no .NET, JavaScript, browser, web view or listener at run time; the same bytes as the standalone host for the same edit; the core never depends on the UI toolkit; tests first; no clippy warnings.

Specs say *what* and *why*; plans say *how*. Do not put implementation choices in a spec.

## Branches and delivery

- `develop` is the integration branch.
- Feature work happens on its own branch named `features/<name>`, in its own worktree; a Spec Kit feature's branch is named as etalii.adp's Spec Kit named it, `features/<number>-<name>`. The one exception is `claude/<name>`, which Claude's cloud sessions are handed by their harness.
- A feature branch is never merged locally into `develop`. When its work is done, push the branch from the worktree it was built in to `origin` and open a pull request into `develop`; nothing reaches `develop` except through a pull request, merged with a merge commit.
- When the pull request is merged or closed, delete the branch locally and on `origin`, and remove the worktree.

## Checks

Before a pull request, run what the `Build` workflow runs:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python tools/sync-etalii-adp.py --check
```

Never edit `vendor/etalii.adp/` by hand: change the files in etalii.adp and sync them here.

## Conventions

- End commit messages written by an agent with a `Co-Authored-By:` trailer naming the model.
- When writing markdown files do not split lines to ensure a maximum line length is honored.
