# Sankey diagram: the conversion's delivery report

The Sankey diagram of etalii.adp.ide.standalone (origin `etalii/sankey`) now derives its toolbox, menus, rows, findings and deletion confirmation from [sankey.dis](sankey.dis), reads its body through the module's FBL binding, writes every edit as the binding's splices, and compiles its canvas from the definition. This is the report its specification, `sankey-disl-fbl` in that repository's `.spec-workflow/specs/`, asks for at its last task: every language gap as a candidate change for this repository, every definition defect repaired on the way, the four module defects of finding B16, and what still waits. What the module keeps, type by type, is in the companion, [sankey.md](sankey.md).

## Candidate changes for this repository

Each is a gap of the companion, numbered as there. The construct, the evidence and what would resolve it are in the companion's table; the type of the module that keeps the behaviour meanwhile is in its *What remains in the module, and why*.

| # | Language | Candidate change |
| --- | --- | --- |
| 1 | FBL | None: closed by FBL 0.5's written text (4.1.5, 5.2 `read` and `quoted`) |
| 2 | FBL | None: closed by FBL 0.5's `fallback` (5.2, 7.4) |
| 3 | DISL | A built-in finding for a reader finding on an entry that is kept and drawn, worded by the definition from what the reader saw, or host attributes readable by constraints (8.7, 11.2) |
| 4 | FBL and DISL | `builtIn` settings for `fbl.` codes, and a `header` that tells a missing mark, a value that is not a number and another version apart (FBL 5.6, 7.4; DISL 8.7) |
| 5 | FBL | A sentence that inserting an entry into an empty flow sequence writes a block sequence holding it, with a conformance fixture (4.3, 6.1, 6.2) |
| 6 | FBL | A sentence that a new sequence item copies the gap after the dash of its previous sibling, with a conformance fixture (6.2) |
| 7 | DISL | A `missing` and a duplicate policy that keep the entry, report it and leave it undrawn, without writing an id (11.5.4) |
| 8 | DISL | An ordering option that sorts findings by location across codes (8.1, 8.6) |
| 9 | DISL | A ribbon edge measured vertically, a minimum bezier reach, a reorder of a top-level node within a computed column, and a drop that chooses a top-level index and writes an attribute from its point (6.10, 9.1, 9.4) |
| 10 | DISL | A relation member that answers the text an end was written with, also when it names nothing (12.2) |
| 11 | FBL | Key slots that bind the members of an entry written as a flow mapping, and a splice of one member's span (4.3, 5.2) |
| 12 | FBL and DISL | A statement of which id a model change names when the specification derives ids and the binding addresses an entry by its place, and that a runtime keeps the pair (DISL 11.5, FBL 5.3) |

Gaps 11 and 12 were found by the implementation, not by the reading the requirements were written from. Gap 12 is the Azure DevOps pipeline diagram's gap 12 met again.

## Definition defects repaired

Twelve, each found by holding the standalone tool's derived answers to its hand-written ones and repaired here, never patched in the module. The companion's *Definition defects repaired* says what each was and is:

- **The rewrite at DISL 0.3** (pull request 497), findings A1 to A6, A9 and A12 to A14: DISL 0.1, seven workarounds for what 0.1 lacked, no Arrange diagram, a persistence plugin beside a binding, no wire ids, the wrong id strategy and the flow's id in prose, findings called not expressible and rules without guards, a node's value over every flow, the thickness stepped from the value as written, and rows showing the stored value.
- **The layout's diagram-wide functions** (pull request 500): `sankeyColumnCount()` and `sankeyScale()` declared without `uses`, so the Column row offered no choices.
- **The finding sort** (pull request 501): the definition now names the host's sort by line as standing in for gap 8.
- **The right-button drag** (pull request 502): the `Flow` edge states the pointer of its connect gesture.

## The four module defects of B16

The line writer the tool used before the conversion misread four shapes of an entry. In etalii.adp.ide.standalone, `SankeyDamagedShapes.Tests.cs` holds a guard for each, seen to fail against that writer before every edit went through the binding (standalone pull request 688), and an oracle test that keeps the damage on record:

| Shape | Fixture | What the line writer did | Now | Guard |
| --- | --- | --- | --- | --- |
| A line inside a block scalar that starts with a key | `block-scalar-key-line.skv` | Took the line for the key: a rename rewrote the description and the name stayed | The name is renamed | `ABlockScalarLineThatStartsWithAKey_IsNotTakenForTheKey` |
| An entry written as a flow mapping | `flow-mapping-entry.skv` | Left a document that is not YAML ("did not find expected '-' indicator") | Read and drawn; setting a value inside it is refused (gap 11), removing it is written | `AnEntryWrittenAsAFlowMapping_StaysReadable` |
| A wide gap after the dash | `wide-dash-gap.skv` | Wrote the first key a second time ("Duplicate key") | Edited like any other entry | `AFirstKeyAfterAWideDashGap_IsFound_AndNotWrittenTwice` |
| A value holding a line that starts with a section key | `section-key-in-value.skv` | Wrote a new node into the value | The node is written into `nodes` | `ALineThatStartsWithASectionKeyInsideAValue_IsNotTakenForTheSection` |

A fifth shape, `header-no-root-mapping.skv`, a document whose root is not a mapping, was left unreadable by the first Thicker bands. It is now refused with "This file does not begin with a mapping such as `sankey: 1`, so there is nowhere to write `thickness`." and, for a new node, FBL's "The file has no place to create nodes in.". On the way the standalone FBL library's YAML family was corrected, so that a container created at the end of a document whose root is not a mapping is refused rather than written (guard `YamlWritingTests.AContainerCreatedAtTheEndOfTheDocument_IsRefused_WhereTheRootIsNotAMapping`).

## Library work, in the standalone repository

Each was added for every tool, with no module name, and each test was seen to fail first:

- **DISL** (pull request 666): `sum()` on a list and `formatNumber(n, pattern)` (12.4); a derived id held twice reported once, on the later holder's line; a `layout` action's `refusals` handed to the host.
- **The client** (pull request 675): a relation drawn as a filled ribbon as thick as a bound number (`ribbonLine`), a type dragged along one axis, and five additions to `compileNotation`.
- **FBL** (pull request 688): the YAML family's refusal above.

## Findings struck or amended

Ruled by the user in chat, each applied to the conversion's requirements as a small amendment:

- **A11 narrowed, 2026-10-11**: `cel.bind` struck, since the standalone CEL parses and evaluates it.
- **Q6 read so, 2026-10-11**: Requirement 9.3, the library work for gaps 1 and 2, waits with Requirement 4.
- **B8 reworded, 2026-10-24**: a body that is not YAML was offered "Add node here" and the edit refused; the conversion removes the offer.

## Behaviour that changed on purpose

Each named in the requirements and ruled there:

- **What an edit writes** (Q1, B11 to B14): the value's own span in its own style, with a trailing comment kept; a new key in the binding's order, so `thickness` comes after `format`; a new entry with its previous sibling's indentation and dash gap; a leading comment travelling with its entry when it is moved or removed.
- **A body that is not YAML** (Q3, B8): opened empty and read-only, with no context menu, and every edit refused with the store's sentence.
- **The shapes of B16 and `header-no-root-mapping.skv`**, above.

Everything else a user reads is unchanged, as the parity transcript shows: menus, rows, findings, payloads, toolbox and session refusals are reproduced byte for byte outside the documents named here.

## What still waits, and on what

| What | Waits on |
| --- | --- |
| `SankeyReader`'s sentences leaving the module | Gaps 3, 4 and 11 |
| `SankeyFindingOrder` leaving the module | Gap 8 |
| The commands running the definition's operations instead of building their model changes | Gap 12, and the standalone interpreter's `set` with `target: diagram`, which it refuses because its DISL model has no element for the diagram's own attributes, so Thicker and Thinner bands cannot run through it |
| The place a dropped node takes, the column drag and the ribbon in the notation | Gap 9 |
| `writtenEnd` leaving the host | Gap 10 |
| A duplicated key read rather than refused, `fbl.header-mismatch`, and a finding for a value or colour on a flow already dangling | Each a small change of its own (Q4) |
| One rounding for a half in the last kept decimal (finding B15) | A ruling on whether the tool or CEL's `math.round` gives way |
| `formatNumber` as ICU in `env.locale` | A ruling on whether DISL 12.4 says ICU or the invariant culture; the standalone library follows the tool |
| `persistence.ids.missing` and `pattern` read by the standalone loader | Nothing for this tool, whose place ids already behave as `ephemeral` and whose pattern refuses nothing a reader keeps |

## Sources

- Code: [`src/diagrams/sankey/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/sankey) in etalii.adp.ide.standalone, its parity transcript `backend/EtAlii.Adp.Diagram.Sankey.Tests/Parity/sankey.transcript.json` and its fixtures.
- The conversion's requirements, design and tasks: `.spec-workflow/specs/sankey-disl-fbl/` in etalii.adp.ide.standalone.
