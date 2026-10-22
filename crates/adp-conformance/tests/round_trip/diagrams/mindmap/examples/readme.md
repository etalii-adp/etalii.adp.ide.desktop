# Mindmap examples

Two Freeplane maps you can open and click around in. They are not fixtures — the fixtures under
`src/diagrams/mindmap/backend/EtAlii.Adp.Diagram.Mindmap.Tests/Fixtures/` exist to pin parser
behaviour one construct at a time, and read like it. These read like maps somebody made.

Paths below are given from the repository root rather than relative, because this file is
shipped in two places (see the last section) and a relative path would resolve in only one.

| File | What it is for |
|---|---|
| [`example 1/mindmap.mm`](example%201/mindmap.mm) | A rendering engine broken down five levels deep, 37 nodes. Read it to see how a wide map lays out and how deep nesting reads. |
| [`example 1/design.mm`](example%201/design.mm) | Developer onboarding, 36 nodes. The same tree with different text — it shares every node ID with `mindmap.mm`, because it was made from it. |

Only `mindmap.mm` carries a `.adp` registration in the module's own copy of this folder;
`design.mm` is registered here in the showcase and not there.

## Authored for this repository

**Both maps were written for ADP.** Neither is derived from, nor attributed to, any external
source, so there is no `LICENSE.md` beside them and none is owed. That is the honest-fallback
path the vendored-data rule leads with, and it is worth saying out loud rather than leaving a
reader to infer it from a missing file — an absent licence otherwise reads as an oversight.

## What this corpus does NOT demonstrate

The rule is that a corpus records what it cannot show, because the gap is invisible from the
inside: everything present looks like everything there is. **Measured 2026-09-09 by counting
attributes across both files, not by reading them.**

The module reads five things from a `.mm` node — `TEXT`, `LINK`, `FOLDED`, `POSITION` and a
`richcontent` note. These maps between them use **two**:

| Construct | Present? | What is therefore unexercised |
|---|---|---|
| `TEXT` | 37 and 36 nodes | — |
| `LINK` | one per file, both on the root | A link on anything but a root; the `↗` indicator anywhere else |
| `FOLDED` | **none** | A collapsed branch, its hidden subtree, and the `⊕` indicator |
| `richcontent` note | **none** | A node with notes, and the `•` indicator |
| `POSITION` | **none** | An author's explicit left/right placement, as against the computed layout |

Nothing here carries an icon, an arrow link, a cloud, per-node styling or an attribute table
either — the module does not read those today, so their absence costs nothing yet, but it will
the day it does.

**The practical consequence, for whoever runs the manual checks**: two of the three corner
indicator glyphs cannot be produced by opening a file we ship. `tests.md`'s shape-family entry
covers this by folding and annotating a node in the session and undoing both afterwards. If that
becomes tiresome, the fix is a third example that has them — not a looser check.

## These files are shipped twice, and for a long time only one copy was tested

The same maps live under both `src/examples/diagrams/mindmap/` (the showcase, which is what a
reader opens) and `src/diagrams/mindmap/examples/` (the module's own copy). Where a module has
example tests at all they read the **module** copy — so the showcase, the tree a user actually
sees, was read by nothing, while being the tree the running application writes to. It drifted:
two nodes reading `sdfsdf` were saved into `mindmap.mm` from the running app and committed, and
stayed until somebody opened the map and looked at it.

`ShippedExamplesTests` now sweeps both trees for placeholder text. It does **not** compare the
two copies, and must not: they are not required to be byte-identical — a presentation choice
made in the app and saved is a legitimate divergence, and the causal loop examples carry one on
purpose — so a byte guard here would fail on correct edits. What the sweep still cannot see is
a name that is real words in the wrong place, and that needs a person opening the diagram,
which is what the `tests.md` step exists for.
