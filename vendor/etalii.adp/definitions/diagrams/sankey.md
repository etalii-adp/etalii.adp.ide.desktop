# Sankey diagram

[sankey.dis](sankey.dis) specifies ADP's **Sankey diagram** (`etalii/sankey`) in DISL 0.3. The specification holds everything DISL 0.3 can state: the metamodel, the notation, the toolbox, the context menus, the forms with their checks, the constraints with their codes, sentences and order, the operations with their refusals, Arrange diagram, the deletion confirmation, the ids and the persistence through an FBL binding. This page holds what DISL 0.3 cannot state, the places where DISL and FBL fall short, and what a runtime will meet that neither says, so that a host implementing the diagram type from the specification ends up with the tool that exists today.

The reference implementation is the standalone module [`src/diagrams/sankey/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/sankey) in etalii.adp.ide.standalone, converted to this specification by that repository's specification `sankey-disl-fbl`. Its delivery report, with every gap as a candidate change for this repository, is [sankey-report.md](sankey-report.md). Every sentence, label and code below is the one its parity transcript records (`backend/EtAlii.Adp.Diagram.Sankey.Tests/Parity/sankey.transcript.json`), and every fixture named is under `backend/EtAlii.Adp.Diagram.Sankey.Tests/Fixtures/`. Paths are relative to `src/diagrams/sankey/` in the standalone repository unless they start with `src/`.

## Contents

1. [What the diagram is for](#what-the-diagram-is-for)
2. [The document on disk](#the-document-on-disk)
3. [Layout](#layout)
4. [Reordering a column](#reordering-a-column)
5. [What DISL cannot express](#what-disl-cannot-express)
6. [Language gaps](#language-gaps)
7. [Definition defects repaired](#definition-defects-repaired)
8. [Other differences a runtime will meet](#other-differences-a-runtime-will-meet)
9. [What remains in the module, and why](#what-remains-in-the-module-and-why)
10. [The examples](#the-examples)
11. [Sources](#sources)

## What the diagram is for

A Sankey diagram follows a quantity that is conserved as it moves, and shows how it divides and combines on its way: revenue by segment into gross profit and the cost of revenue, energy from its sources through conversion to where it is used, graduates by major into work. Its question is not *what is connected* but *how much goes each way*: a bar is as tall as what passes through it and a band as thick as what it carries, on one scale, so the proportions are read off the picture.

- **Domain agnostic.** The diagram knows nodes and flows and nothing else. What a node stands for, and what unit a value is in, is the document's business: the palette words name colours rather than meanings, and a value's unit is written into its `format` (`€{value}M`, `{value} TWh`, `{value}k`).
- **Origin.** The notation and its file format are defined by the user of the repository rather than by a standards body or a vendor, which is why the origin is `etalii/`.
- **State.** ⚗️ Prototype, the state its row in the standalone `docs/tools.md` carries, recorded under `x-adp.state`.

## The document on disk

A `.skv` document is YAML: a header `sankey: 1`, three optional document-wide keys `format`, `flow-color` and `thickness`, then two lists, `nodes` and `flows`. A node's keys are `id`, `name`, `color`, `note`, `format`, `column` and `description`; a flow's are `id`, `from`, `to`, `value`, `step`, `color` and `description`. There is no position and no size anywhere: every coordinate is computed (see [Layout](#layout)).

```yaml
sankey: 1
format: "€{value}M"
flow-color: target
nodes:
  - id: cafes
    name: Cafés
    color: blue
    note: "+9% Y/Y"
  - id: revenue
    name: Revenue
    color: grey
flows:
  - from: cafes
    to: revenue
    value: 1840
```

The specification reads and writes it through the module's FBL binding, `backend/EtAlii.Adp.Diagram.Sankey/sankey.fbl`, named by `persistence.binding`, with a `typeMap` that maps the binding's `document`, `node` and `flow` onto the diagram, `Node` and `Flow`, and a flow's `from`, `to` and `value` onto its ends and `quantity` (`value` is a reserved attribute name in DISL). The binding declares no `header`: `sankey` is read as the host attribute `version`, and the three sentences about the header are the module's (see [What DISL cannot express](#what-disl-cannot-express)). Its template, `sankey: 1`, `nodes: []`, `flows: []` with CRLF and a final newline, is the document a new diagram starts from, byte for byte the document the module's factory writes. The module reads every body through the binding and writes every edit as the binding's splices.

- **Ids.** A node's id is the one written in the file; a new node gets `uuid-v4` in `base36`, which is the module's 25-character ShortGuid. A flow states an id only when it needs one, and its id is otherwise `from->to`, which the specification derives (`persistence.ids.types.Flow.derived`). Nodes and flows share one id space and the first holder wins. The pattern is `^.+$`, because the module reads `a->b` and hand-written ids with a space or an accent as written. A node without an id is `missing: "ephemeral"`: nothing ever writes an id for it.
- **An unreadable body is not a missing one.** Both open empty, but a body that is not YAML opens read-only, as FBL 7.5 says: it offers no context menu, and every edit is refused with "not-yaml.skv could not be read, so nothing can be edited until it can: The document could not be read as YAML: While parsing a flow sequence, did not find expected ',' or ']'." (`not-yaml.skv`). A key written twice in one mapping makes the body unreadable in the same way, "Duplicate key name" (`duplicate-key.skv`), where FBL 7.4 binds the first of the two (ruled for this tool, Q4). A document whose root is not a mapping is read, reports nothing and refuses every edit with a sentence of its own (`header-no-root-mapping.skv`, gap 4). An entry written with a wide gap after its dash is read and edited like any other (`wide-dash-gap.skv`), and so is an entry written as a flow mapping, except that a value inside it cannot be set (`flow-mapping-entry.skv`, gap 11).
- **Round trip.** An unchanged document comes back byte-identical, and every edit is a splice, so comments, blank lines, indentation, quoting, unknown keys, line endings and a missing final newline survive (`crlf-line-endings.skv`, `lf-line-endings.skv`, `no-trailing-newline.skv`). Because the bytes are the test subject, the standalone repository's `.gitattributes` marks `*.skv -text`.

## Layout

A Sankey diagram's geometry is its data, so nothing in it is a position anybody chose. The specification names the layout `plugin:net.etalii.adp.etalii.sankeyLayout`, required and with no fallback, because a bar's height and a band's thickness are on the scale it computes: its CEL functions `sankeyColumn`, `sankeyColumnCount`, `sankeyScale`, `sankeyTop` and `bandAt` feed the notation. The module implements it as `SankeyLayout`:

- **It is computed over the whole document**, never over a view. Only drawn entries take part: nodes with an id not taken earlier, and flows whose id is not taken earlier and whose two ends are drawn and different (the user functions `drawnNode` and `drawn`).
- **Values.** A flow carries its value, a missing or negative value counting as 0. A node's value is the larger of the sum of what flows into it and the sum of what flows out of it, over drawn flows only (`value-over-drawn.skv`).
- **Columns.** A node that states `column` is in that column (one-based in the file, zero-based to CEL). Every other node is one column right of the furthest node that flows into it, over the flows that close no cycle, found by a depth-first walk in document order that leaves out each flow reaching a node still on the walk; a node nothing flows into starts in the first column. A sink is not pushed to the last column.
- **Order within a column** is the order the nodes are written in the document, top to bottom. Nothing in the layout changes it; a reader does (see [Reordering a column](#reordering-a-column)), and so does Arrange diagram.
- **Scale.** One scale for every bar and band: the column whose node values sum highest fills `560 × thickness` canvas units. A bar is `max(6, value × scale)` tall and 24 wide; a band is `max(1, value × scale)` thick, so a flow without a value is a hairline. Bars in one column are at least 36 apart; columns are 320 apart, left edge to left edge.
- **Bands at a bar** are stacked edge to edge, the outgoing ones in the order of the centres of the nodes they reach and the incoming ones in the order of the centres of the nodes they leave, ties broken by document order. `bandAt` gives a band's centre as a fraction of each end's height from its top, 0.5 on a bar that carries nothing.
- **Relaxation.** 24 passes, the n-th weighted `0.99^n`, right to left and then left to right, moving each node towards the value-weighted mean of the tops at which its forward bands would run level; each column is then kept within the extent, in its order and never closer than the gap. Relaxation never reorders. Tops and heights are rounded to two decimals.
- **Arrange diagram** (`sankey.arrange`, the `layout` action `arrange`) gives the nodes of each column the order whose bands cross least, weighed by how thick the crossing bands are, and writes it as the order of the entries; it is refused with "There is nothing to arrange until this diagram has a node." and "This diagram is already arranged.", the two `refusals` of the action. The module implements it as `SankeyArrangement`.

## Reordering a column

The order within a column is the only thing a reader arranges, and it is stored as the order of the document's entries. A node moves only up and down its column; the nodes it passes step aside while it is dragged, and on release its entry moves directly before the first other node of its column whose middle is below the drop's middle, or after the last one. When that is where it already is, nothing is written and nothing is refused. The refusals are "This diagram is read-only." without a history, "That is not something this diagram can move." for anything that is not a drawn node, and "A node takes its place by being dragged up or down its column." for a reparent. The specification states the placement as `movable: { "x": false, "y": true }` over a computed `y`; DISL has no gesture for the rest (gap 9).

## What DISL cannot express

- **An entry that is kept, reported and not drawn** (finding A7). A node without an id, and every second and later holder of an id, node or flow, is reported as an error and not drawn, its flows do not count towards any value, and no id is ever written for it (`undrawn-entries.skv`: "A node has no id and is not drawn." at line 5, "The id `a` is used more than once; only its first entry is drawn." at line 10). DISL 11.5.4 draws a later holder and gives an element without an id either an id written with the next save or a drawing under an ephemeral id. The specification declares `missing: "ephemeral"`, keeps the findings through `std.missingId` and `std.duplicateId`, and leaves the entries undrawn through the `visible` of the styles `bar` and `band`, which read the user functions `drawnNode` and `drawn`. That a later holder is not drawn is the specification's notation, not DISL's identity rule (gap 7).
- **Findings in the order of the file** (A8). The module lists every finding by its line, whatever its code (`rules-broken.skv`: `sankey.missing-id` at line 3, `sankey.unknown-color` at line 4, `sankey.duplicate-id` at line 9, …). DISL orders findings by kind, and `constraints.order` by the position of the code and only then by line. The specification's `constraints.order` lists the codes in the order the module finds them arise, which is what a runtime shows for one entry; across entries the module's list is by line (gap 8).
- **The reader's sentences for an entry that is still drawn** (A10, B4). `sankey.unreadable` is nine sentences of the module's reader, and in eight of them the entry is still read and drawn: an unknown key ("`colour` is not a key this module reads on a node; the line is kept.", `unknown-keys.skv`), a number that will not read ("`value: lots` is not a number and was ignored.", `values-that-will-not-read.skv`), a `column` that is not a whole number from 1 ("`column: 0` is not a whole number from 1; the flows place this node."), a `flow-color` that is neither word ("`flow-color: sideways` is neither `source` nor `target`; flows take their target's colour."), a `thickness` not above zero ("`thickness: wide` is not a number above zero and was ignored."), and the three about the header ("The document does not begin with `sankey: 1`.", "The version `one` is not a number.", "This document states version 2; this module reads version 1.", the `header-*.skv` fixtures). `std.unreadableEntry` is for an entry that is kept and not drawn, and host attributes are invisible to constraints, so none of these can be declared; the specification sets `std.unreadableEntry` and `std.unparseable` to `sankey.unreadable`, a warning, for the ninth, an entry that is not a mapping ("A node entry is not a mapping and was passed over.", `malformed-entries.skv`), and the rest stay a module reader over the binding's elements (gap 3).
- **The notation and gestures of A15.** A flow is a filled ribbon whose top and bottom edges are the same cubic curve, `thickness` apart vertically along its whole length, so a band is as thick as its value at every x and the bands at a bar meet edge to edge; DISL has stroked edges only, and the specification draws a stroke as wide as the thickness. Its control points lie horizontally from the ends, `max(40, half the horizontal distance)` away, where `BezierSpec`'s `auto` is at least 30. A node is reordered by dragging it within a column the layout computed (see [Reordering a column](#reordering-a-column)), where DISL's `reorder` gesture is for a child of an ordered container. A dropped node takes its place among the top-level entries by geometry, directly above the first node of the nearest column whose middle is below the drop, and has its `column` written from the drop point when that is not the first column, where a parent placement (DISL 9.1) chooses an index only under a parent (gap 9).
- **The wire ids** are under `x-sankey`, as ruled for every converted tool (Peter, 2026-10-06, "Spec all but wire ids").

## Language gaps

Each gap is a candidate change to DISL or FBL, found by the standalone conversion. None is worked around in the specification or the binding by stating something the tool does not do. Where the standalone library does more than FBL's text says, the tool relies on it, and the gap is that another host may not do the same.

| # | Gap | Construct | Evidence | Would resolve it |
| --- | --- | --- | --- | --- |
| 1 | **Closed by FBL 0.5.** A slot that reads a YAML scalar as written (B1, B2) | FBL 4.1.5 written text, 5.2 `read` and `quoted` | `text-that-reads-as-a-number.skv` (an id `1`, a name `2050`, an id `007`, a flow id `0x1F`), `number-forms.skv` (`value: "12"`, `value: 0x10`, `value: .inf`); an entry of the standalone `RealFiles/divergences.json` | Done: an id, a reference and a text attribute read their written text unasked, and a number slot asks for it and is parsed as the tool parses it |
| 2 | **Closed by FBL 0.5.** A slot whose value will not read, without losing the entry (B3) | FBL 5.2 `fallback`, 7.4 | `values-that-will-not-read.skv`, `malformed-entries.skv` (`thickness: wide`, `flow-color: sideways` on the root mapping, `value: lots` on a flow) | Done: `fallback` keeps the entry readable and its bytes kept, with `report: false` where the tool words the finding itself (gap 3) |
| 3 | Reader findings worded by the definition for an entry that is still drawn (A10, B4) | DISL 8.7 `std.unreadableEntry` (kept and not drawn), 11.2 host attributes (invisible to constraints) | `unknown-keys.skv`, `values-that-will-not-read.skv`, the `header-*.skv` fixtures | A built-in finding for a kept and drawn entry whose sentence the definition words from what the reader saw (the key, the written value), or host attributes readable by constraints |
| 4 | The code and sentence of `fbl.header-mismatch` (B10) | FBL 5.6 `header`, 7.4; DISL 8.7 `constraints.builtIn`, which reaches `std.` codes only | `header-missing.skv`, `header-not-a-number.skv`, `header-other-version.skv`, `header-forms.skv`, `header-no-root-mapping.skv` | `builtIn` settings for `fbl.` codes, and a header that tells a missing mark, a value that is not a number and another version apart |
| 5 | Opening an empty flow sequence (B13) | FBL 4.3 (a flow collection's only writable span is the whole), 6.1, 6.2 | The template's own `nodes: []` and `flows: []`; the standalone test `TheTemplatesEmptyFlowSequences_AreOpenedByTheFirstEntry` | A sentence in 4.3 or 6.2 that inserting an entry into an empty flow sequence writes a block sequence holding it, and a conformance fixture |
| 6 | What a new sequence item copies (B13) | FBL 6.2, *Indentation* (the previous sibling; keys "align with the column after `- `") | `entry-forms.skv` (a node added after `    -   id: c` is written `    -   id:`: the standalone library copies the indentation and the gap after the dash of the previous sibling) | A sentence that the gap after the dash is copied, from the previous sibling, and a conformance fixture |
| 7 | An entry that is kept, reported and not drawn (A7) | DISL 11.5.4 `missing`, the second and later holders of an id | `undrawn-entries.skv`, `rules-broken.skv` | A `missing` and a duplicate policy that keep the entry, report it and leave it undrawn, without writing an id |
| 8 | Findings in the order of the file (A8) | DISL 8.6 ordering, `constraints.order` (8.1) | `rules-broken.skv`, `guarded-findings.skv`; the standalone module sorts the derived list by line (`SankeyFindingOrder`) | An ordering option `by: "location"` that sorts by line across codes |
| 9 | The notation and gestures of A15 | DISL 6.10 edges (stroked only), `BezierSpec`, 9.4 `reorder`, 9.1 placement | The examples' bands; the transcript's drag, reparent and drop steps (`sessionRefusals`; the drops of `crlf-line-endings.skv`) | A ribbon edge measured vertically, a minimum bezier reach, a reorder of a top-level node within a computed column, and a drop that chooses a top-level index and writes an attribute from its point |
| 10 | What a dangling end was written with | DISL 12.2: CEL reads a relation's end that names nothing as null | `guarded-findings.skv` ("A flow names `ghost`, which is not a node here; it is not drawn." at line 13; "The flow from `ghost` runs to itself and is not drawn." at line 22) | A member of a relation that answers the text its end was written with, also when it names nothing |
| 11 | The members of an entry written as a flow mapping (B16) | FBL 4.3 (a flow collection is one value whose only writable span is the whole), 5.2 key slots | `flow-mapping-entry.skv` (`- {id: a, name: A}`); the standalone `RealFiles/divergences.json` entry for it; the transcript's refusals "The skv file has no mapping to write 'value' in." | Key slots that bind the members of a flow mapping, and a splice that replaces one member's span |
| 12 | The id a model change names when the specification derives one (B7) | DISL 11.5 `derived` ids; FBL 5.3 place ids | The standalone test `Parity/SankeyDerivedOperations.Tests.cs`, which finds a flow such as `a->m` by its line before it can write it | A statement of which id a model change names for a derived id, and that a runtime keeps the pair of the derived id and the binding's place id |

### 1 and 2. FBL: written text and a value that will not read (closed)

The requirements' gaps 1 and 2, closed by FBL 0.5 (etalii.adp spec 015, `fbl-written-text`) before this conversion reached them: section 4.1.5 gives every YAML scalar a written text, section 5.2 lets a slot read it with `read: "text"` and lets an attribute keep its entry readable with `fallback`. The module's binding takes them up: its slots read the written text, and its number slots carry a `fallback`.

### 3. DISL: reader findings for an entry that is still drawn

Every binding of a hand-edited format meets it: the hype cycle graph and the timeline keep a module reader for the same reason. `std.unreadableEntry` means *kept and not drawn*, and the eight sentences above are about entries that are drawn, so declaring them there would state something the tool does not do. **Would resolve it:** a built-in for a reader finding on a drawn entry, worded by the definition from what the reader saw.

### 4. FBL and DISL: the header

The module reads `sankey:` wherever it stands in the root mapping, quoted or not (`header-forms.skv` reports nothing), and says one of three sentences on line 1. A document with no root mapping reports nothing at all and refuses every edit: Thicker and Thinner bands with "This file does not begin with a mapping such as `sankey: 1`, so there is nowhere to write `thickness`.", which the module words, and a new node with FBL's "The file has no place to create nodes in." (`header-no-root-mapping.skv`). FBL's `header` reports one `fbl.header-mismatch`, a warning, for all three, and DISL's `builtIn` names no `fbl.` code, so neither its code nor its sentence can be set. The binding therefore declares no `header`.

### 5 and 6. FBL: a new entry

The first entry of a new document goes into the template's `nodes: []`. FBL 4.3 makes a flow collection one value written whole, and no splice of 6.1 opens one into block form; the standalone library does it all the same, so the tool's own new document is edited as the tool edits it. A new entry copies the indentation and the gap after the dash of its previous sibling (`entry-forms.skv`), as the standalone library writes it; before the conversion the tool copied the list's **first** entry, and FBL 6.2 says the previous sibling for the indentation but not that the gap is copied (finding B13, ruled by Q1). **Would resolve them:** the two sentences, and a conformance fixture each, without which another host may write different bytes into the template's own document.

### 7. DISL: an entry that is kept, reported and not drawn

See [What DISL cannot express](#what-disl-cannot-express). The specification keeps the tool's behaviour with `missing: "ephemeral"` and the styles' `visible`, so it states nothing untrue, but the identity section of DISL says otherwise of a later holder, and a runtime that follows 11.5.4 alone will draw it.

### 8. DISL: findings in the order of the file

The Errors and Warnings panel lists a document's findings from top to bottom, whatever their code (`rules-broken.skv`). `constraints.order` can only put whole codes before others. **Would resolve it:** an ordering by location.

### 9. DISL: a ribbon, a bezier reach, a reorder in a computed column, a drop that writes its point

See [What DISL cannot express](#what-disl-cannot-express). The specification draws the nearest DISL has: a stroke as wide as the band, `routing: "bezier"` with the ends attached by `part` anchoring at `bandAt`, a `movable` y over a computed placement, and the toolbox's `mode: "drop"`; the place among the entries and the written `column` are the host's.

### 10. DISL: what a dangling end was written with

Added by the standalone conversion. A flow's derived id is `from->to` as written, also when an end names no node (`a->ghost`); a flow from `ghost` to `ghost` is a self-flow and nothing else; and `sankey.dangling-flow` names the end as written. DISL hands CEL a dangling end as null, with no way to read what was written. The specification declares the plugin function `writtenEnd(flow, end)` of `net.etalii.adp.etalii.sankeyEnds`, with a fallback that reads the end's id, and uses it in the three places. The hype cycle graph and the timeline carry the same stand-in. **Would resolve it:** a relation member, such as `self.writtenSource`, that answers the written text of an end.

### 11. FBL: the members of an entry written as a flow mapping

Found by the implementation (finding B16, the second of its shapes). YAML lets an entry be written `- {id: a, name: A}`, and the tool has always read and drawn such an entry like any other. To FBL's YAML family a flow mapping is one value, whose members CEL sees but no key slot binds (FBL 4.3), so the binding reads no id and neither end. The module reads such an entry's values from the binding's computed `written` attribute, the written text of every member (`SankeyReader`), so it draws it as before. A removal of the whole entry is written, and setting a value inside it is refused with FBL's "The skv file has no mapping to write 'value' in.", where the tool before the conversion rewrote the line and left a document that was not YAML. **Would resolve it:** key slots that bind a flow mapping's members, and a splice of one member's span.

### 12. FBL and DISL: the id a model change names when the specification derives one

Found by the implementation. A flow that states no id is `a->m` to the definition (`persistence.ids.types.Flow.derived`) and `flow@<line>` to the binding, which addresses an entry without an id by its place (FBL 5.3). An operation of the definition run by the standalone interpreter names the flow by its DISL id, which the binding does not know, so the module's commands build their model changes themselves, naming the binding's element, and a test runs `rename`, `increaseValue` and `decreaseValue` through the interpreter, finding each element by its line, to show that both write the same bytes (`Parity/SankeyDerivedOperations.Tests.cs`). The Azure DevOps pipeline diagram meets the same gap. **Would resolve it:** a statement of which id a model change names when the specification derives ids and the binding addresses by place, and that a runtime keeps the pair.

## Definition defects repaired

A definition defect is the specification saying something the tool does not do, or leaving out something it does. Each was found by the standalone conversion and repaired here; the first ten by the rewrite at DISL 0.3 (pull request 497), the last two by the tasks that derived the rows and compiled the canvas:

| Finding | Was | Is |
| --- | --- | --- |
| A1 | DISL 0.1, which the standalone client refuses | DISL 0.3 |
| A2 | Seven workarounds for what DISL 0.1 lacked | `persistence.ids.types.Flow.derived`; a deletion `confirm` with `count` and `threshold`; a `diagram` context menu; the toolbox's `mode: "drop"`; anchors on the `left` and `right` sides; `part` anchoring with `at`; Colour and Custom colour as two form items over `color`, with `id`, `display` and `parse` |
| A3 | No Arrange diagram | The `layout` action `arrange` with the refusals `nothingDrawn` and `unchanged` |
| A4 | A persistence plugin, with the keys DISL 11.2 forbids beside a binding | `format: "fbl"` with the binding, `typeMap` and an empty `view.store` |
| A5 | No wire ids, and `x-sankey` used for `documentList` | The `x-sankey` block of tools, actions and properties |
| A6 | `strategy: "nanoid"` for both types, and the flow's id in prose | `uuid-v4` in `base36`, the pattern `^.+$`, `missing: "ephemeral"`, and the flow's `derived` rule `self.statedId != '' ? self.statedId : writtenEnd(self, 'source') + '->' + writtenEnd(self, 'target')` |
| A9 | `missing-id` and `duplicate-id` called not expressible; value and colour rules without guards; messages reading unset ends | `std.missingId` and `std.duplicateId` set to `sankey.missing-id` and `sankey.duplicate-id`, errors, with the module's sentences; every flow rule guarded so that a self-flow and a dangling flow silence it, as `guarded-findings.skv` shows; `constraints.order` in the order of arising |
| A12 | A node's value over every flow | Over drawn flows, through `drawn` (`value-over-drawn.skv`) |
| A13 | Thicker and Thinner bands computed from the thickness as written | From the drawn thickness, clamped to 0.1 to 10, so `thickness: 50` steps from 10 (`thickness-clamped.skv`) |
| A14 | Step, Column, Colour and Rename showing the stored value; one sentence of the thickness limits | `display` and `parse` on the rows (the default step, `auto`, `(default)` and `(custom)`, numbers as `0.####`, the label as Rename's default); `unavailable` with "The bands are already as thick as they go." and `abort` with "The bands are already as thick as this diagram draws them.", and the same for thin |
| Task 6 | `sankeyColumnCount()` and `sankeyScale()` declared without `uses`, so a host was never told which diagram they answer for, and the Column row offered no choices | Both declare `"uses": ["diagram"]` (pull request 500) |
| Task 8 | The right-button drag that draws a flow stated only in the toolbox's `doc` | The `Flow` edge's connect gesture states `{ "button": "secondary", "start": "body" }` (pull request 502) |

## Other differences a runtime will meet

- **Rounding.** The module rounds a stepped value, a starting value and a thickness with .NET's `Math.Round`, which rounds a half to the even neighbour; CEL's `math.round` and FBL's `number: {decimals: 4}` round a half away from zero. The two differ only on a half in the last kept decimal (`number-rounding.skv`, `thickness: 1.005`): `0.0001` stepped by `0.00015` is written `0.0002` by the tool and `0.0003` by the definition's `increaseValue` (finding B15).
- **`formatNumber` is not ICU.** DISL 12.4 says `formatNumber` formats with ICU in `env.locale`. The standalone library formats with .NET's invariant culture, which is what the tool's labels have always shown (`#,0.##` and `0.####`); ICU rounds a half to the even neighbour and prints a large integer exactly, so `12345678901234567890` (`number-rounding.skv`) can be written differently by a host that follows DISL to the letter.
- **`backward` is not drawn.** `sankey.backward-flow` says a backward flow "loops round instead of reading left to right", and the band is drawn with the same curve as any other, doubling back from the source's right edge to the target's left edge.
- **`#rgb` is read, `#rrggbb` is said.** A three-digit hex is a colour the module reads and paints; the messages speak only of `#rrggbb`.
- **A flow's value ignores its nodes' formats.** A flow's tooltip uses the document's format only.
- **A dropped node is pinned to its column.** A node dropped anywhere but the first column has its `column` written, so it stays there after flows are drawn to it.

## What remains in the module, and why

Every type of the product project `backend/EtAlii.Adp.Diagram.Sankey/` of `src/diagrams/sankey/` in etalii.adp.ide.standalone, checked against `git ls-files` of that project at the end of the conversion (its task 12), beside the gap or the host concern that keeps it. `SankeyToolboxProvider` had neither, since core's `DislToolboxProvider` hands out a bundled definition's palette, and was removed by that task. `SankeyParser`, `SankeyWriter`, `SankeyValidator`'s rules, `SankeyDisl` and the providers' hand-written logic left the product code earlier in the conversion; the ones a test still compares with live in the test project as the oracle (`Parity/HandWrittenSankey.cs`, `Parity/HandWrittenSankeyEdits.cs`).

| Type | Kept by |
| --- | --- |
| `SankeyLayout`, `SankeyBand`, `SankeyGeometry` | The layout the definition names as the required plugin `net.etalii.adp.etalii.sankeyLayout` (finding A16): columns, scale, relaxation and the bands at a bar, computed once per model. |
| `SankeyArrangement` | The `layout` action `arrange`: the order of least crossing that the action writes (finding A16). |
| `SankeyFunctions` | The host's side of the two plugins: the layout's five CEL functions over one layout per model, and `writtenEnd` (gap 10). |
| `SankeyReader` | The reader's nine sentences over the binding's elements (gap 3) and the header's three (gap 4); a flow mapping's values from the written text (gap 11). |
| `SankeyFindingOrder` | Findings in the order of their lines (gap 8). |
| `SankeyModel`, `SankeyNode`, `SankeyFlow`, `SankeySettings` (`_Model/`) | What the reader reads into, and what the layout, the mapper and the commands address. |
| `SankeyFormat`, `SankeyColors` | The `.proto` payloads carry the formatted value and the paint, resolved: a palette word or a hex, and a flow's fallback to its end's colour by `flow-color` (Requirement 7.1). `SankeyColors` stays because the payload carries the colour resolved, which the definition's enum does not give, and the colour check of Set property keeps the tool's sentence. |
| `SankeyElementMapper` | The host's wire: the model in the element vocabulary of `api/sankey.proto`, and the viewport filter that sends a node in view with every node it shares a flow with. |
| `SankeySession` | The host's session seam: the deltas a connection is sent, and "This diagram is read-only." without a history. |
| `ISankeyDocumentStore`, `SankeyDocumentStore` | The shared document lifecycle: one open body per file, read through the binding. |
| `SankeyBody` | The body read and written through the module's binding (`sankey.fbl`), the DISL model built from its reading, a body that is not YAML read-only, and the template a new document starts from (finding B17). |
| `SankeyDefinition` | The definition: the bundled `.dis` loaded once, and the palette, menus, rows, findings and deletion confirmation derived from it, mapped by `x-sankey`, read-only for a body that is not YAML. |
| `SankeyContextSourceResolver` | The host's selection: an element id resolved against the file. |
| `SankeyContextActionProvider`, `SankeyContextPropertyProvider`, `SankeyValidator` | Core's seams, handing out what the definition derives. |
| `AddSankeyNodeCommand`, `ConnectSankeyNodesCommand`, `MoveSankeyNodeCommand`, `RemoveSankeyEntryCommand`, `SetSankeyPropertyCommand`, `SankeyKeys`, `StepSankeyValueCommand`, `ScaleSankeyThicknessCommand`, `ArrangeSankeyCommand`, with their handlers and `SankeyEditHandler` | The host's command records, unchanged on the wire. Each handler builds the binding's model change itself, because the definition names a flow by its derived id (gap 12); the place a dropped node takes and the order Arrange writes come from the layout and the drop point (finding A15, gap 9). |
| `SankeyEdits` | The model changes the handlers ask the binding for, in one place, with the refusals the binding cannot word. |
| `Diagram` | The host's catalog entry: the origin and the `.skv` extension. |
| `ServiceCollectionAddSankeyExtension` | The host's registration of the types above, and of core's `DislToolboxProvider` over the definition. |

Beside them the client keeps `client/sankeyBindings.ts`, which says what the canvas library cannot read from the definition (the values the backend computes, the class names that tie a palette word to its theme colour, the test ids), and `client/sankeyModel.ts` and `client/sankeyIds.ts`, which decode the payloads and state the wire ids once. The hand-written canvas definition is test code only, in `client/sankeyCompiledDefinition.test.tsx`, the oracle the compiled one is held to.

## The examples

Three examples under the standalone `src/examples/diagrams/sankey/`, each with an `.adp` registration and a readme:

- **`brightwater-coffee`**: the income statement of a fictional coffee company for fiscal year 2026, written for ADP; every figure is invented.
- **`uk-energy`**: a possible UK energy system in 2050 in TWh, vendored from the plotly.js test mock `sankey_energy.json` (MIT, `LICENSE.md` beside it).
- **`recent-graduates`**: recent US college graduates by major category into work and kinds of job, in thousands, derived from FiveThirtyEight's `college-majors/recent-grads.csv` (CC BY 4.0, `LICENSE.md` beside it).

Their readmes list what they do not demonstrate; the fixtures cover a document that breaks the rules.

## Sources

- Module code, binding, fixtures and parity transcript: `src/diagrams/sankey/` in etalii.adp.ide.standalone.
- The conversion's findings, A1 to A16 and B1 to B17, and its open questions: `.spec-workflow/specs/sankey-disl-fbl/requirements.md` in etalii.adp.ide.standalone.
- Catalogue row: `docs/tools.md` in etalii.adp.ide.standalone, row for `etalii/sankey`.
