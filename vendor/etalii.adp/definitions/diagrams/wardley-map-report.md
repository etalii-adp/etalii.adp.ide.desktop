# Wardley map: the conversion's delivery report for stage one

The Wardley map of etalii.adp.ide.standalone (origin `wardley/map`) now derives its toolbox, menus, rows, operations, findings and deletion confirmation from [wardley-map.dis](wardley-map.dis), and compiles its canvas from it. It still reads and writes the `.owm` body with its own parser and writer, and keeps its ids in its own identity file. This is the report its specification, `wardley-map-disl-fbl` in that repository's `.spec-workflow/specs/`, asks for when stage one closes (its task 10). It gives every language gap as a candidate change for this repository, every definition defect and module defect repaired on the way, and what still waits. What the module keeps, type by type, is in the companion, [wardley-map.md](wardley-map.md).

## What waits, and on which gaps

Stage two, the conversion's tasks 11 to 13, reads and writes the body through an FBL binding. None of it starts before this repository answers the gaps it needs:

- **Task 11** reads the body through the binding. It waits on gap 1 (a pipeline's brace on the line below its statement), on gaps 2 to 4 as far as the conversion's question Q2 needs them, and on gap 7 (FBL 8.7 describing the identity file as the array it is).
- **Task 12** writes every edit as the binding's splices and keeps the ids through the registration. It waits on task 11 and on gap 8 (FBL 8.6 on an entry whose key changes, and on a stored id for every holder of one key).
- **Task 13** closes the conversion. It waits on task 12.

Until then the module's parser, writer and identity classes keep reading and writing as the tool always has, and the parity transcript pins that they do.

## Candidate changes for this repository

Each is a gap of the companion, numbered as there. The construct, the fixture that shows it and what would resolve it are in the companion's table. None is closed.

| # | Language | Candidate change |
| --- | --- | --- |
| 1 | FBL | A rule option that lets a statement own the block the next statement opens, or a `blocks` option naming the brace style (4.7) |
| 2 | FBL | Only a statement matched by a rule with `opens`, or by a block rule, opens a block; an unclosed block ends at the end of the body with a finding (4.7) |
| 3 | FBL | A binding-level `trailingComment` expression, cut before rules match and kept outside every re-emitted span (4.6) |
| 4 | FBL | A slot that finds a segment of several words anywhere in a group, in any order (2.5) |
| 5 | FBL | A slot whose value is the set of words present, with unknown words kept (3.3) |
| 6 | FBL | A slot that yields a list of groups, each of which can be spliced (`lines` and `blocks` slots) |
| 7 | FBL | FBL 8.7 describing the identity file as the array of `{id, kind, key}` the tool writes, or a way to read and write it |
| 8 | FBL | Keys per rule or kind, an ordinal for a later holder, a rekey when the attribute a key is computed from changes, and a sentence on whether an identity write is part of the edit's history (8.6) |
| 9 | DISL | A base-folder argument or a `project` root for `fs.exists`, and its example rewritten over `Url` addresses (12.4) |
| 10 | DISL | A `formatNumber` that takes an invariant locale, or a separate invariant formatter (12.4) |
| 11 | DISL | A `pickerConfirm` label for `connect` with `target: "pick"`, and a `single: "run"` that runs without asking when there is one candidate (9) |
| 12 | DISL | An ordering option that keeps a built-in's findings in the order of the collections it walks (8.1, 8.6) |

Gaps 1 to 10 are the ten of section C of the conversion's requirements, found by reading the module against the languages. The first half of gap 10, `oncePerGroup` reporting at a later holder, was struck as stated in the companion. Gaps 11 and 12 were found by the implementation, while writing the menus and the constraints against the transcript.

## Definition defects repaired

Each was found by holding the standalone tool's derived answers, and its compiled canvas, to its hand-written ones. Each was repaired here, never patched in the module. The companion's *Definition defects repaired* says what most of them were and are.

- **The rewrite at DISL 0.3** (pull request 514), findings A1 to A4, A7 and A9. DISL 0.1, which the standalone client refuses. Twelve workarounds for what 0.1 lacked (finding A2). `urls`, `decorators` and `AnnotationPin` in the wrong shapes (A3). No wire ids (A4). `wardley.duplicate-name` located on the name (A7). An evolution as a node type of its own, with a warning for a second `evolve` (A9).
- **The next annotation number** (pull request 517). It was taken with `.max()` on a list, which is not a function of DISL's CEL, so the standalone loader refused the definition. It is now `math.greatest`.
- **Five forms** (pull request 528). The add, Rename… and pipeline dialogs accepted an empty name. Evolves to accepted an empty value on an element that is not evolving. Decorators were listed in the notation's order instead of the order they are written in. The label offset was shown as the struct itself. A pipeline's components were on one line.
- **The duplicate-name rule's cost** (pull request 531). `duplicateName` filtered every positioned element once per positioned element: 1.8 s on a map of 1000 components, against the tool's bound of 2 s. It is now scoped to the diagram and takes about 50 ms, with the same findings.
- **What the canvas reads** (pull request 538). The names of components, anchors and submaps are edited inline. Notes, annotation pins, accelerators and attitudes cannot be picked. An accelerator is a rule 32 long, centred on its point.
- **The diagram element** (pull request 541). `x-wardley` recorded an evolution-axis element that the standalone backend no longer sends. It now records the `wardley/map+diagram` element, which carries the attributes `metamodel.diagram` declares and names the map by its title.

## Module defects repaired

Each was repaired in the standalone module, behind a guard seen to fail against the code before it.

- **Findings B9 and B10, identities** (standalone pull request 709). After the first save, every later holder of a repeated key got the first holder's id, and the next drag threw. Renaming an element gave its links new ids, and undoing the rename gave the element a new id, so the redo was refused. Now every holder of a kind and key keeps an id of its own, across save, reopen, undo and redo, under the rulings of 2026-10-11 and 2026-10-12. A rename keeps the ids of the element and of everything attached to it.
- **Finding B13, names a statement cannot hold** (pull request 709). Renaming `Tea [hot]` wrote `Tea [cold] [hot]`. Renaming `CI/CD` left its links behind. "Link to" wrote the same link twice. A name holding `//` was accepted. Such names are now refused with a sentence of their own, on an add, a rename, a pipeline component's name and an evolution's new name.
- **Finding A7, the duplicate-name location** (pull request 709). The finding is located at the first holder.
- **Delete on a selected component** (pull request 724). It did nothing. It now asks the definition's confirmation and removes the component, and one undo puts the file back byte for byte.
- **An accelerator's name** (pull request 724). It was stroked with the rule's width and dash, which drew it smeared. It is now text in the text colour, and `wardley.css` carries the rule for the glyph that it lacked.
- **A right-click on empty map** (standalone pull request 728). The definition declares a menu for the diagram, but the backend refused the placement the canvas sends, so no menu opened. It now opens on the map's seven adds.

## Library work, in the standalone repository

Each was added for every tool, with no module name. Each test was seen to fail first.

- **DISL** (pull request 712): `axisRange`, the `target: "pick"` picker, a constrained primitive's bounds for `std.facets`, and `outOfRange.typed`.
- **DISL and core** (pull request 715): `showAbsent`, a picker's empty text and `via`, the options of a params form, the `call` action with a depth limit, and a `DiagramActionProvider` commit hook.
- **DISL** (pull request 716): `std.facets` checks the fields of a struct by dotted path.
- **The client** (pull request 719): a canvas's bounds, margin and `fit: "stretch"`, an axis's ranges drawn as bands with their boundaries, a drag clamped by both axes, and the seven constructs the definition needed and the library refused.
- **The client** (pull request 724): `typeIds: "specification"`, a node that cannot be picked, an inline editor over a label the element places itself, and an aligned label for a text shape placed by its edge.

## Findings struck or amended

Ruled by the user in chat, each applied to the conversion's requirements as a small amendment:

- **B14 and B15, 2026-10-25**: a statement that cannot be read, and a line that is not a statement, are not passed over in silence. Each is reported as the warning `wardley.line-passed-over` (`out-of-range.owm`, `two-titles.owm`).
- **Requirement 3.3, 2026-10-25**: nine findings, not eight, `wardley.line-passed-over` among them.
No other read finding turned out not to occur.

Ruled by the user in chat on 2026-10-25 as a small amendment to the conversion's amendment 2 (Requirement 1.4, finding A10): the one payload that changes is replaced by the `wardley/map+diagram` payload, which carries the diagram's attributes, in place of the evolution axis's. The canvas names the map by its title.
