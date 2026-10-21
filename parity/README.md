# Parity

What etalii.adp spec 030's FR-010, FR-032, SC-002 and SC-004 are checked against ([contracts/parity.md](https://github.com/etalii-adp/etalii.adp/blob/develop/specs/030-desktop-native-rust/contracts/parity.md)): the inventory of the standalone host's capabilities ([inventory.md](inventory.md)), the edit scripts ([scripts/](scripts/)), the documents they open ([fixtures/](fixtures/README.md)), the bytes the standalone host wrote for them ([goldens/](goldens/)) and the program that recorded those bytes ([recorder/](recorder/README.md)).

## The check

`cargo test -p adp-parity` replays every script headless in the desktop core and compares every file its golden lists byte for byte (task T050, `crates/adp-parity`). Per script it:

1. copies the fixture's folder into a temporary project, as the recorder does;
2. opens the registration (else the fixture) through `adp-diagram`'s `DiagramWorkspace`, and takes the session's baseline, as `DiagramService.Open` and `UpdateView` did;
3. maps each step onto the `DiagramSession` as the recorder mapped it onto the standalone host's RPCs: `select` sets the selection; `action` runs `execute` on `args.target` (an element, `new:x,y` the canvas, `rel:a->b` a finished connect gesture) or the selection, answers its prompt with `args.value`, or with `args.confirm` for a confirmation, and dismisses it otherwise; `property` finds the row by id or by its label and runs `set_property`; `move` runs `move_element` for a `parent`, else `move_elements` to `to` or to the element's place plus `dx` and `dy`; `undo` and `redo` run on the project's history; `save: true` sends nothing;
4. gives every element or connection a step creates the id the standalone host generated at that step, from `recorded.json`, and resolves `$N` and `$cN` to the N-th generated element and connection, so the bytes can match;
5. compares each file of the golden's `changed` list with the project's file of that path, and checks that each file of `removed` is gone.

A step the standalone host accepted and the desktop host refuses (or the other way round, for `"expect": "refused"`) is reported with the desktop host's sentence, and a file that differs is reported with its first differing byte offset and the line holding it in the golden and in the replay.

### Line endings

A golden is the standalone host's bytes on the platform `recorded.json` names (every golden here was recorded on `windows`). The C4 layout sidecar (`*.layout.json`) is the one file the standalone host writes with the platform's line ending, `System.Text.Json`'s indented output using `Environment.NewLine` ([recorder/README.md](recorder/README.md)), so the check compares it in the line endings of the platform the golden was recorded on: on another platform, the replay's sidecar has its line endings turned into the recorded platform's (CRLF for `windows`) before the comparison. Every other file, and the sidecar on the recorded platform, is compared exactly as written: `roadmap.tml` is CRLF because its fixture is, not because of the platform.

### A script that does not match yet

Every script is a test in `crates/adp-parity/tests/parity.rs`. A script whose tool type cannot open yet in the desktop host, or whose bytes do not match yet, stays a test, ignored with its concrete reason, and its row below says so. `cargo test -p adp-parity -- --ignored` replays them all and prints what differs. The goldens and the scripts are never changed to make a replay pass: when one matches, its `#[ignore]` goes and its row says `matches`.

## State

| Script | State | Why |
|---|---|---|
| `c4/container/courier` | cannot open | The C4 container specification (`vendor/etalii.adp/definitions/diagrams/c4-container.dis`) does not load: `/persistence/typeMap/ViewInclude/attributes/members: 'includes' is not a variable here; available: self, diagram, env. (context element)`. The C4 module's residue (the Structurizr DSL reading and writing of `courier.dsl`, and the layout sidecar `courier.layout.json`) is not ported either. |
| `freeplane/mindmap/example-1` | differs at offset 213 of `mindmap.mm` | The mind map's binding (`mindmap.fbl#mindmap`) reads `.mm` through the persistence plugin `net.etalii.adp.freeplane.mm`, the module's residue, which is not ported. The desktop host opens the map without a refusal but with no elements, so every one of the 28 steps that names a node is refused and nothing is written: the replay keeps the fixture's bytes, which first differ from the golden at the node step 9 renames (`Node lifecycle` against `Node lifecycle &amp; pooling`). |
| `generic/timeline/roadmap` | differs at offset 63 of `roadmap.tml` | 13 of the 32 steps are refused, listed below; the first differing line is the rename of step 1 (`label: Discovery` against `label: Discovery phase`). |

### Where the timeline replay parts from the standalone host

| Step | Script | The desktop host |
|---|---|---|
| 1 | `timeline.rename` "Discovery phase" | Refuses the inline rename: a period's label is computed (`shownLabel(self)`) and has neither `editText` nor `parse`, so `adp-disl`'s `form_commit::label` finds nothing to write. The standalone module's rename was its own. |
| 4 | move `build` by 604800 seconds | Refuses ("An element cannot end before it begins."): the session reads a node's place from its `begin` and `row` attributes as numbers, so a dated `begin` is at x 0, and its place is not in the timeline's own coordinates (seconds since 1970 across, 60 per row down, `coordinates.axes`) that the script and the standalone host use. |
| 7, 8 | `timeline.give-end`, then `timeline.remove-end` | Runs the operation without asking its `params` (`end`), so `set end: p.end` fails ("No such key: 'end'"); `remove-end` is then not offered, since `launch` has no end. |
| 11, 12, 19 | `timeline.add-after`, `timeline.add-below`, `timeline.add-element` | Refuses: `dateAfter` is a function of the plugin `net.etalii.adp.generic.timelineTimes`, which the desktop host does not implement and which declares no fallback. |
| 13, 21, 22 | the label of `$1`, then selecting and removing `$3` | Follow from 11 and 19: the elements were not created. |
| 15 | `timeline.relabel` "informs the beta" | Refuses ("'label' cannot be edited here."): the connection's label is bound to the attribute `label` but has no `parse`, and `form_commit::label` only writes through a `parse`, where DISL section 6.12 says an attribute binding makes a label editable and its text is written to the attribute. |
| 16, 17 | `timeline.connect` on `rel:discovery->launch`, then `timeline.disconnect` of `$c3` | The finished connect gesture's menu entry, the `relate` tool, is not run by `DiagramSession::execute` ("'Relate' cannot be run here yet."), so the connection `$c3` disconnects is not made. |
