# Dependency graph

[`dependency-graph.dis`](dependency-graph.dis) specifies ADP's dependency graph in DISL 0.3: its node and relation types, its two authored axes, its notation, toolbox, context menus, forms, findings, operations and persistence settings, and, in its `x-dependencies` block, the ids the standalone tool gives each palette entry, menu entry and property row. This document holds everything about the diagram type that DISL cannot express, and records where the `.dis` had to approximate. Each statement is tied to the code or document that shows it.

| | |
|---|---|
| Kind | Diagram |
| Origin | `generic/dependencies` |
| Display name | Dependency graph |
| File | `.dgr`, ADP's own YAML schema, owned outright by this type |
| State | ✅ Implemented in standalone; no IntelliJ, VS Code or Eclipse implementation |
| Source read | [etalii.adp.ide.standalone `src/diagrams/dependency-graph/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/dependency-graph) at `b5192d47e` |

The type is a disciplined fork of the timeline diagram with every time-related aspect removed (no dates, no ruler, no time units on zoom, no moments) and one addition: its edges mean something. A relation `from: a, to: b` reads "a depends on b", and the arrowhead is drawn at `b` ([`DependencyGraphRelation.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/_Model/DependencyGraphRelation.cs); the original requirements in standalone's history before `d573ad87`, `.spec-workflow/archive/specs/dependency-graph/requirements.md`).

Its catalogue entries agree: standalone [`docs/tools.md`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/docs/tools.md) lists it as implemented, and the Notion "Tools" row gives the purpose "Draw named things and what depends on what, freely placed and connected", the reason for specialising "Dependencies are about direction and order; a dedicated tool keeps every edge directed and readable, in ADP's own .dgr format", the focus areas "Planning and roadmapping" and "Systems and strategy", and the family "Business process & workflow notation".

## 1. The `.dgr` file format

DISL's persistence layer (section 11) configures a DID definition. A `.dgr` file is not one, so the `.dis` names an FBL binding: `persistence.format` is `fbl`, `persistence.binding` is the binding `dgr` of the standalone module's [`dependency-graph.fbl`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/dependency-graph.fbl), and `persistence.typeMap` maps the binding's types `Node`, `Dependency` and `Unreadable` onto `Node`, `DependsOn` and an unreadable entry. The standalone module reads and writes every `.dgr` through that binding, embedded in its backend project, and no longer has a parser or a writer of its own; the same binding is offered beside the FBL examples as [`dependency-graph.fbl`](../../specifications/fbl/dependency-graph.fbl). This section is what the binding says about the file.

### 1.1 Grammar

A `.dgr` file is a YAML mapping with three keys the type reads, in this order by convention:

```yaml
dependencies: 1
elements:
  - id: api-gateway
    label: API gateway
    x: 270
    row: 4
  - id: identity-service
    label: Identity service
    x: 797.5
    row: 4
relations:
  - id: r-gateway-identity
    from: api-gateway
    to: identity-service
    label: verifies tokens with
```

| Key | Content | Read by |
|---|---|---|
| `dependencies` | The version marker, `1`. Written into every new file by the binding's `template`; **never checked when reading**: the binding declares no `header` (see section 9). | the binding's `template` |
| `elements` | A sequence of nodes. Each is a mapping with `id`, `label`, `x` and `row`. | the binding's rule `node` |
| `relations` | A sequence of dependencies. Each is a mapping with `id`, `from` (the dependent), `to` (the dependency) and an optional `label`. May be absent. | the binding's rule `dependency` |

- Every value is read as the text it was written with (`read: "text"`, FBL 0.5 section 5.2): `id: 007`, `label: true`, `label: ~` and `from: 0x1F` are those texts, and `id: 7` and `id: 007` are two ids. An id and a reference are compared as written text.
- `x` and `row` are read as their text too, and typed by the `.dis` (`number` and `int`) with .NET's invariant rules, so a file says the same thing on every machine; a value that will not read is 0 (section 9). FBL's `fallback` would also keep such an entry readable, but it converts by the YAML core schema first, so `0x10` would be 16 and `.inf` infinity where the tool reads 0.
- Anchors, aliases, merge keys, block scalars and every quoting style are read as FBL section 4.3 reads them. A value reached through an alias or a merge key is read-only, and an edit to it is refused with FBL's sentence. A node or a dependency written as a one-line flow mapping is one value whose members a slot does not read (FBL section 4.1): it is read as an entry with no id and no values (section 12).
- An entry of `elements` or `relations` that is not a mapping is read as `Unreadable`: kept, not drawn, and silent, because `std.unreadableEntry` is off. A missing key reads as empty (`id`, `label`, `from`, `to`) or zero (`x`, `row`).
- An empty file, or a file whose root is not a mapping, is an empty graph rather than an error; nothing can be added to it ("The file has no place to create elements in."). A mapping that repeats a key is read, its first value counting, and reported as `fbl.duplicate-key`, a warning on a graph that stays editable.
- The file holds no date, duration or instant anywhere. A `.dgr` that did would be a defect ([fixtures readme](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph.Tests/Fixtures/readme.md)).

### 1.2 Mapping to DID

A runtime that works on DID definitions sees a `.dgr` file through this mapping. DISL expresses the DID side; the binding and the `.dis`'s `typeMap` state the mapping. The binding reads a dependency as an element with reference attributes, so one whose end names no node is kept and reported, and reads each entry's id and each end a second time as written, into the host attributes `storedId`, `writtenFrom` and `writtenTo`.

| `.dgr` | DID |
|---|---|
| `elements[i]` | `elements[]` record with `type: "Node"`, `id` from `id`, `attributes.label`, `attributes.x`, `attributes.row` |
| `relations[i]` | `relations[]` record with `type: "DependsOn"`, `id`, `source` from `from`, `target` from `to`, `attributes.label` when present |
| `dependencies: 1` | `language.version` 1.x of `net.etalii.adp.generic.dependencies` |
| any other key, at any level | preserved verbatim, never modelled (section 1.4) |
| comments, blank lines, indentation, quoting style, line endings | preserved verbatim; DID has no place for them |

There is no view record: `x` and `row` are bound placement attributes, so the position lives only in the model, exactly as DID requires of bound values. Nothing else is stored about the view (`persistence.view.store` is empty): no viewport, no sizes, no waypoints, no label offsets.

### 1.3 Writing: splices, never serialisation

Every write is a model change the binding plans as splices of the file's own bytes (FBL section 6), applied in one place, the module's body ([`DependencyGraphBody.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphBody.cs)): a rename, a relabel or a move is a set on the entry found by the id it is written with, a removal is a remove, and an add or a connect is the DISL change the definition makes, mapped onto the binding by its `typeMap`. DISL's persistence settings assume a canonical writer that regenerates the file; this type does the opposite. Beside an FBL binding DISL 11.2 forbids those settings (`encoding`, `newline`, `finalNewline`, `files`, `ordering`, `omitDefaults`), so the `.dis` has none of them, and the binding's own text settings and insert keys say what new content looks like.

- **Round trip.** Opening and saving an unchanged file returns it byte for byte, the pair of line-ending fixtures and the one without a final newline included, and editing one node changes only that node's bytes.
- **Setting a value** (rename, move, relabel) replaces the value's own span in its own style: the author's quotes, a trailing comment and an anchor on the value survive, and a block scalar is replaced whole.
- **Clearing a relation's label** removes its `label` key, with a comment directly above it. Clearing a node's label writes `label: ""`; a node always keeps its `label` key (`empty` of each label in the binding).
- **Adding** appends after the last existing entry of the section, copying the indentation and the gap after the dash of the entry before it (FBL leaves the gap unstated; section 12). An empty flow list, `elements: []`, is opened into a block list and a comment after it is kept; a graph with no `elements` key gets the key, before `relations:`. A new node is written as `id`, `label`, `x`, `row`; a new relation as `id`, `from`, `to`, then `label` only when it has one, where `insert.keys` orders it.
- **The `relations:` key is created on demand** at the end of the file with its first entry, and the undo of that insert removes the key again (section 6).
- **Removing a node** removes its entry and every dependency whose end resolves to it, each with the comments directly above it. A dependency that names the node's id but resolves to an earlier holder of that id stays, and so does one whose end is not written.
- **Numbers** are written in FBL's form, ECMAScript's shortest round-trippable text: a whole `320` stays `320`, `0.00001` is `0.00001` and `1e21` is `1e+21`.
- **Quoting.** A new value is written plain when it is plain-safe, and in the binding's `quote` (double) otherwise, so a label that would read as another type (`123`, `true`) or an id such as `007` is quoted. Existing quoting is never changed on a value the edit does not touch.
- **Line endings.** A file keeps whatever it already uses, including a missing final newline. A new file is the binding's template, `dependencies: 1\r\nelements: []\r\n`: CRLF because that is standalone's house style, which is why the binding says `newline: "crlf"`.
- **A file that does not read is never written.** Every command refuses with a sentence naming the edit ("… could not be read, so nothing can be edited until it can: {reason}"), so a broken file is never made worse.

### 1.4 What the file may carry beyond the model

Keys the type does not model survive every edit verbatim, at the document, node and relation level, including nested sequences, so a later version of the type or a human can put data there without ADP eating it. The same holds for comments in every position YAML allows and for blank lines. The round-trip corpus proves each case: `simple`, `shared-rows`, `coordinates`, `relations`, `comments`, `lf-line-endings`/`crlf-line-endings`, `no-trailing-newline`, `indentation`, `scalars` and `unmodelled-keys` ([fixtures readme](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph.Tests/Fixtures/readme.md)). The fixtures are byte-compared, so a repository that stores `.dgr` files must exempt them from line-ending normalisation (`*.dgr -text` in standalone's `.gitattributes`).

### 1.5 Files and registration

A graph is one `.dgr` file. In standalone it may also have an `.adp` registration beside it whose first line is the MIME type `generic/dependencies`; the registration carries nothing else the type reads, and a bare `.dgr` opens as a dependency graph with no registration at all, because no other tool writes `.dgr` ([`Diagram.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/Diagram.cs), [`DependencyGraphSessionFactory.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphSessionFactory.cs)). The base name of a new diagram is not written into the file: the schema has no title line. A new graph has no sample content.

## 2. Identity and loading

- **Ids come from the file.** A hand-written file may use readable ids (`api-gateway`); ids ADP generates are ShortGuids, a version 4 GUID written as 25 characters of base 36 (`ShortGuid.cs` in standalone's `src/backend/EtAlii.Adp/`). The `.dis` says `uuid-v4` with `encoding: "base36"`, as the hype cycle graph does; an earlier version of this document said 22 URL-safe base64 characters, which the code never wrote.
- **Missing and duplicate ids are tolerated on load and reported** (section 3). Wherever an id is looked up, the first node with it wins, else the first dependency, found by the binding's `storedId`; FBL addresses a later holder by its place ([`DependencyGraphEdits.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/Commands/DependencyGraphEdits.cs)). DID requires unique, present ids, so these states exist only in a `.dgr` file.
- **A file that is not YAML still opens.** The canvas shows the diagram as unavailable, naming the file and line, the Errors and Warnings panel carries the same message, and every edit is withheld ([`DependencyGraphDocumentStore.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphDocumentStore.cs)).
- **An unreadable `x` or `row` reads as 0**, silently (section 9).
- **One document per path.** Two diagrams on the same file share one loaded document, so an edit through one appears in the other. An edit made outside ADP is picked up and sent to every open view; a reload that cannot read the file keeps the last good document; deleting the file clears it ([`IDependencyGraphDocumentStore.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/IDependencyGraphDocumentStore.cs), [`DependencyGraphDocumentReloader.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphDocumentReloader.cs)). A save writes exactly the document the command edited, so a reload that lands between an edit and its save cannot discard the edit while reporting success.

## 3. Validation

Every rule reports a **warning** naming its element, because the rest of the graph still draws. The one **error** is a file that is not YAML, reported as a single problem at the parser's line instead of running the rules over an empty model ([`DependencyGraphFindings.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphFindings.cs), [`DependencyGraphValidator.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphValidator.cs)).

| ADP rule id | Severity | Message | Located at | In the `.dis` |
|---|---|---|---|---|
| `dependencies.unparseable` | error | "This is not YAML that can be read: {parser message}" | the parser's line (at least 1) | `std.unparseable`, with this code and message |
| `dependencies.missing-id` | warning | "'{label}' has no id, so nothing can select, connect or edit it." (a relation: "A dependency has no id, …") | the declaration's first line | rule `missingId` |
| `dependencies.duplicate-id` | warning | "The id '{id}' is declared more than once, which makes every reference to it ambiguous." | the second and later declarations' first line | `std.duplicateId`, with this code and message. Nodes and relations share one id space |
| `dependencies.dangling-relation` | warning | "A dependency names '{id}', and no node with that id is in this graph." | the relation, by id; once per missing end | rules `danglingSource` and `danglingTarget` |
| `dependencies.self-dependency` | warning | "'{id}' is declared to depend on itself, which says nothing about what needs what." | the relation, by id | rule `selfDependency` |

The standalone tool evaluates these constraints with the DISL runtime ([`DependencyGraphFindings.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphFindings.cs)). Two things about them need the ids as the file writes them, which the `.dis` reads through a CEL function of its own, and one stays code:

- **An empty end is not a reference.** A dependency whose `from` or `to` is absent or empty is reported by no rule. `std.references` would report any end that resolves to no node, so it is switched off and `danglingSource` and `danglingTarget` report an end only `when` it names an id.
- **A self-dependency is judged on the ids as written.** `from: ghost, to: ghost` is reported as dangling twice and as a self-dependency. `std.endpoints` judges only a relation whose two ends both resolve, so it is switched off and `selfDependency` compares the ids as written.
- **Both read `writtenEnd(self, 'source' | 'target')`**, a function of the `net.etalii.adp.generic.dependencyEnds` plugin: an end that names no node is null in the model (4.9), so CEL cannot read the id it names. Its fallback reads the resolved end's id, which is right for every end that names a node. The standalone tool implements it over its own reading.
- **The order stays code.** The findings come in two passes: first every missing and duplicate id, interleaved in declaration order, nodes before dependencies whatever the file's order; then, per dependency in declaration order, its dangling `from`, its dangling `to` and its self-dependency. `constraints.order` sorts by code, which cannot interleave two codes, and DISL's default order (8.6) does not group by element. The tool sorts the runtime's findings into that order, and hands the runtime each declaration's place in it as its line, so that `std.duplicateId` calls the node the first holder of an id even where the dependencies are written above the elements.

The missing-id message names a node by its label, which is empty when it has none ("'' has no id, …"), because the code falls back to the id and the id is the thing missing. The `.dis` says the same with `self.label`; the runtime mints an ephemeral id for the element and still names it by its label.

Rule ids are prefixed with the origin's type name. A self-dependency can only come from a hand-edited file: connecting a node to itself is refused at the command with "A node cannot depend on itself." There is no cycle rule: the design names cycle detection as something the type will want one day, and the code deliberately does not invent it ([`DependencyGraphRules.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphRules.cs)). The `.dis` therefore leaves `acyclic` unset.

A dangling relation is still sent to the canvas with the ids as written, so the canvas can show the loose end and the validator can report it ([`DependencyGraphElementMapper.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphElementMapper.cs)). Labels are never validated: every value asked for is a label, and a label has no wrong answer.

## 4. Placement arithmetic

The `.dis` binds `x` and `row` to a linear x axis and a linear row axis with a scale of 60 and whole-row snapping. These numbers and rules sit beside it:

- **Row height 60, node box 160 × 36.** A node's top-left corner sits at (`x`, `row × 60`), leaving a gutter of 24 between rows. All three constants are rendering constants mirrored in the backend and the client ([`DependencyGraphRows.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphRows.cs), [`DependencyGraphCanvas.tsx`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/client/DependencyGraphCanvas.tsx)); if they disagree, every vertical drag lands on the wrong row. Changing the row height re-spaces every graph without touching a file.
- **Row rounding.** A y becomes a row by rounding `y / 60` to the nearest integer with halves away from zero, never producing negative zero. The same rule is shared by every row canvas and pinned by a golden fixture both sides read. DISL's snap `direction: "nearest"` does not say how ties break.
- **Why a linear row axis, not an ordinal one.** A DISL ordinal axis has declared bands. Rows here have none: any integer is a row, negative rows are valid, several nodes share one and gaps are allowed ([`shared-rows.dgr`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph.Tests/Fixtures/shared-rows.dgr)). A row is a placement grid, never a lane, so no band is drawn and nothing is stacked.
- **Moving.** A drag moves freely on x and snaps to the nearest row; the drop sends the top-left corner, and the move is one command. Nothing on x is snapped, and the canvas offers no way to switch row snapping off, which the `.dis` states with `userToggle: false` and `bypassModifier: "none"`.
- **Dropping from the toolbox** places the node's top-left corner at the pointer's x, on the row nearest the pointer's y. The id the canvas sends is a placement id `new:{x},{row}` that lives only for that one call ([`DependencyGraphNewPlacement.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphNewPlacement.cs)).
- **Tab and Enter** add a node 240 to the right on the same row, or at the same x one row down. The `.dis` expresses both as operations, `addRight` and `addBelow`, which the standalone tool runs with the DISL runtime and writes as the transaction they make ([`RunDependencyGraphOperationCommand.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/Commands/RunDependencyGraphOperationCommand.cs)); the 240 step is chosen so the new node does not land under its source at the zoom levels the canvas opens at ([`DependencyGraphContextActionProvider.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphContextActionProvider.cs)).
- **Adding from a menu** with no placement asks for a label ("Add node", field "Label", prefilled "New node") and places the node 240 right of the node the action was invoked on, or at (0, 0) with no node.
- **Property grid.** X accepts any invariant number; Row accepts only an integer, and a fractional row is refused ("'{value}' is not a number this graph can place a node at.") rather than truncated. The `.dis` states both as `validate` patterns; the code parses with .NET's invariant rules, which also accept `NaN` and `Infinity` for X and refuse a row outside the 32-bit range. The X row shows the number as the file writes it, .NET's shortest invariant text (`1000000`, `1E-05`), where CEL's `string(double)` would write `1e+06` and `1e-05` ([`DependencyGraphContextPropertyProvider.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphContextPropertyProvider.cs)).

## 5. Gestures and interactions

- **Drawing a dependency.** A node has two anchors, one at the middle of each side. Dragging from the **right** anchor to another node makes the dragged node depend on the one it lands on. Dragging from the **left** anchor reverses it: the node landed on depends on the dragged node, because what depends on a node arrives at its left side. The `.dis` says so with the edge's `connect.from`, which maps the `right` anchor to the relation's source and the `left` to its target.
- **Releasing on empty canvas** creates a node labelled "New node" at the release point and the dependency, as one command and one undo step. From the right anchor the new node is the dependency; from the left anchor it is the dependent. The `.dis` says this with `createSource` and `createTarget` on a library tool, `dependsOn`, that no palette group shows and the edge's `connect.tool` names: the gesture starts from a node's anchor, never from the palette. The standalone tool runs that tool's `createTarget` or `createSource` with the DISL runtime and writes the transaction it makes as it is.
- **A whole gesture is one call.** The canvas sends source and landing together as `rel:{from}->{to}`, where either end may be a placement id. An earlier two-call protocol kept an armed source between calls, and a stale arm related the wrong pair in the running app ([`DependencyGraphRelationGesture.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphRelationGesture.cs)). The split takes the first `->`, so a node id containing `->` would be misread.
- **Refusals** come back as sentences shown on the canvas's refusal line: "A node cannot depend on itself.", "Both ends of a dependency must be nodes of this graph.", "The node this dependency starts from is no longer in this graph.", "That node is no longer in this graph.".
- **Removing a node** removes every dependency that resolves to it. With none, it goes at once; with some, the user is asked first ("Removing this node also removes the 1 dependency attached to it." or "… the {n} dependencies attached to it.", button "Remove", marked dangerous). The `.dis` says so with DISL 0.3's `deletion.confirm` `count` and `threshold`.
- **Keys.** On a node: F2 renames, Delete removes, Tab adds to the right, Enter adds below, and a double-click renames, as `doubleClick: "editLabel"` says. On a dependency: F2 relabels, Delete removes it. The client takes these from the `.dis`'s context menus. Labels of nodes and dependencies also edit inline on the canvas.
- **Empty canvas.** A right-click there opens its menu, "Add node here", which places the node as a drop does: its left edge at the x under the pointer, on the row nearest it.
- **No reparenting.** A node has no parent; the host's move-to-parent request is refused with "A node has no parent to move it under; dragging changes where it sits and which row it is on."
- **Not supported:** copy and paste (the `.dis` sets `copyable: false`), resizing, reconnecting a dependency's end (`reconnectable: false`), selecting several elements at once (standalone's multi-select specification is not built yet), and automatic layout. The canvas pans and zooms; zoom and pan never reach the file.
- **No ruler, no dates, no moments.** A ruler, a date in a drag hint or a moment marker appearing anywhere in this type is a defect (original requirement 3.3).

## 6. Undo and redo

Every edit is one command on the project's history. What DISL's single "one transaction, one undo step" rule does not say is how the inverse restores the file:

- **Every undo is the host's shared restore command**, which puts the file's text back as it was before the edit, byte for byte: the comments a removal took, the style of a changed value, and a `relations:` key the edit created included. A create and its connect are one edit, so one undo takes both.
- **Redo reuses the ids** the first execution generated, because the ids are generated once, where the gesture happens, and carried by the command the history holds. A redo that would duplicate an id is refused.
- Every handler re-locates its target in the current document, because undo and redo dispatch the same command instance again later.

## 7. The dependency curve

The `.dis` names a routing plugin, `net.etalii.adp.generic.dependencyCurve`, with `bezier` as its fallback. Its geometry ([`DependencyGraphCanvas.tsx`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/client/DependencyGraphCanvas.tsx)):

- When the dependency's left edge lies to the right of the dependent's right edge, the curve is a horizontal bezier between the two facing side anchors.
- Otherwise, when the dependency sits behind or overlaps the dependent, the curve is a forward loop: out of the dependent's right side and back into the dependency's left side.
- While a dependency is being drawn, the preview is a plain horizontal bezier to the pointer.
- The target end attaches on the side facing its source rather than by true outline intersection, because the notation reads left to right and an edge through the top of a box would read as a different relation.
- The arrowhead is a filled triangle at the dependency end, coloured with its line. Two dependencies between the same pair are drawn on the same curve; nothing separates them.

## 8. Rendering and the wire

- **The look is the host's shared canvas theme**: surface fill, border stroke 1.5, corner radius 6, a 12 px centred label truncated to the box, the label falling back to the node's id when empty. The `.dis` records those values as theme tokens with the standalone light values; the host supplies dark mode. Selection and connect-target highlighting are the canvas library's, not the type's.
- **Viewport culling.** The canvas reports its viewport, and the backend sends only what it admits: every node whose box touches the viewport (touching edges count), and every dependency whose span, the hull of its two end boxes, touches it, together with both of its end nodes so a line always has two boxes to draw between. Until a viewport is reported the whole graph is sent. A view change sends removals first, then additions ([`DependencyGraphSession.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphSession.cs)).
- **Wire payloads.** A node travels with its authored `x`, `row` and `label`; a dependency with `from_element_id`, `to_element_id` and `label`, in that order, because the order is the only thing carrying the type's meaning across the wire and nothing may normalise it ([`dependency-graph.proto`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/api/dependency-graph.proto)). Element types are `generic/dependencies+node` and `generic/dependencies+relation`; an update arrives as an add of the element in its new state, never as a remove and add, so the selection survives it.

## 9. Known gaps and discrepancies

Found while tracing the code for this document; each is a candidate for its own change. The conversion to FBL (dependency-graph-disl-fbl) repaired what was listed here beside them: adding a node to a graph with no `elements` key, which threw, now creates the key; undoing every edit returns the file it started from for every document of the parity transcript; a label that reads as another type is quoted; a removal takes only the dependencies that resolve to the node; and a key written on a dash line with a wide gap is found.

- **The version marker is never read.** `dependencies: 1` is written into every new file, but the parser ignores it, so a file with another value or none opens the same way. The `.dis` maps it to language version 1.x; nothing enforces that.
- **An unreadable `x` or `row` becomes 0 without a problem**, as it always has (Q4 of dependency-graph-disl-fbl, "as today"): the `.dis` types the text the binding reads, and a value that does not fit reads as its default. No rule reports it ([`DependencyGraphRules.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphRules.cs)).
- **The property grid says reconnecting is done on the canvas**, by dragging a dependency's end to another node, but no reconnect gesture or command exists. The `.dis` follows the code: it sets `reconnectable: false` and keeps the sentence as the ends' read-only reason.
- **There is no cycle rule**, by decision rather than by accident (section 3).

## 10. Choices made in the `.dis`

Where DISL had more than one reasonable reading, these were chosen:

- **Language id `net.etalii.adp.generic.dependencies`**, version `1.1.0` since the DISL 0.3 rewrite: the reverse-DNS form of etalii.net followed by the origin tag, which `language.origin` now carries.
- **Node label shown with a fallback.** The label is a CEL text (`label`, or the id when empty) with a `parse` write-back, so it stays editable inline even though its text is computed.
- **`DependsOn` label defaults to `""`**, and an empty relation label has no key while a node always keeps its `label`; with persistence through an FBL binding, the binding says so (`empty` of each label), not `omitDefaults`.
- **`std.unreadableEntry` is switched off.** An entry of `elements` or `relations` that is not a mapping is kept as it is and not drawn, silently, as the tool has always skipped it; the binding reads it as an `Unreadable` element so that it is kept.
- **The palette tool uses `mode: "drop"`** and `after: "none"`: the node is dropped where it belongs and appears labelled "New node" without starting a label edit. Its initial values, and the `addNodeHere` operation that "Add node here" and a drop both run, put its left edge at the point's x and round the point's row to the nearest.
- **Menus.** A node's menu has two groups, `edit` (Rename…, Remove) and `add` (Add node to the right, Add node below), a dependency's one (Relabel…, Remove dependency); empty canvas offers "Add node here" and a finished drag "Depends on". No entry is hidden or disabled on a graph that could not be read, because the code offers them there too and its edits refuse instead.
- **Forms.** Every item states its label; the two read-only ends of a dependency are computed items with the ids `from` and `to` and the read-only reason as `readOnlyReasons`. A dependency end the file names but no node holds has no id CEL can read (4.9), so a runtime shows it as the file wrote it, as the code does.
- **Operations.** `addRight` and `addBelow` create a node and `connect` the selected node to it in one transaction.
- **Findings.** Written as rules over the ids as written, with the `writtenEnd` function of a plugin of this type's own, rather than as `std.references` and `std.endpoints`; see section 3.
- **Icons** are named as the standalone tool names them (`mdi-graph-outline`, `mdi-rectangle-outline`, `mdi-ray-start-arrow` and the menu icons).
- **`x-dependencies`** maps the specification's names to the standalone tool's wire ids: the palette tool `node` to `dependencies.toolbox.node`, dropping `dependencies.add-element`; each menu entry by its operation or kind (`DependsOn/editLabel` to `dependencies.relabel`, `DependsOn/delete` to `dependencies.disconnect`); each form row by its id or attribute.
- **`x-adp`** at the top level records the kind, catalogue state and implementations, which DISL has no property for.

## 11. What remains in the module, and why

The module reads and writes through the binding and derives everything it can from the `.dis`. What is left is either a language gap (section 12) or a concern of the host the type plugs into:

| Class | Stays because |
|---|---|
| [`DependencyGraphBody`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphBody.cs) | One `.dgr` body opened through the binding, the DISL model built from its reading, and the one place every change is written: the shape every converted module has. |
| [`DependencyGraphDefinition`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphDefinition.cs) | The `.dis`, loaded once, and what is derived from it: palette, menus, rows, the operations' transactions and `Apply`. It finds an element by the id it is written with, and writes the X row as the file writes the number (`DslNumber.Format`), where CEL's `string(double)` would write `1e+06`. |
| [`DependencyGraphReading`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphReading.cs) | Gap 3: nodes are read before dependencies whatever the file's order, which no binding can state. |
| [`DependencyGraphFindings`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphFindings.cs), [`DependencyGraphRules`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphRules.cs) | The order of the findings, which `constraints.order` cannot state (section 3); `writtenEnd`, fed from the binding's host attributes, which CEL cannot see (gap 6); `fbl.duplicate-key` passed on as the one FBL finding the tool reports. |
| [`DependencyGraphElementMapper`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphElementMapper.cs), [`DependencyGraphRows`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphRows.cs) | The host's wire: payloads with the ids as written, the viewport filter, and the row height the client mirrors. |
| [`DependencyGraphSession`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphSession.cs), [`DependencyGraphDocumentStore`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphDocumentStore.cs), [`DependencyGraphValidator`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/DependencyGraphValidator.cs), the context action and property providers, the source resolver, `Diagram.cs` and the registration | The host's seams and its shared document lifecycle. |
| The command records and handlers, [`DependencyGraphEdits`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/Commands/DependencyGraphEdits.cs), [`RunDependencyGraphOperationCommand`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/Commands/RunDependencyGraphOperationCommand.cs) | The history and the wire: each handler checks its refusal, builds a change or takes a transaction, and applies it. |

The parser, the writer and the module's model records left the product code with the conversion; they are the test project's oracle, `Parity/Oracle/` ([`DependencyGraphParser.cs`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph.Tests/Parity/Oracle/DependencyGraphParser.cs)), against which the thirteen `.dgr` files of the repository are cross-checked.

## 12. Language gaps

Every place FBL or DISL fell short for this type, with the construct concerned, the document of standalone's parity corpus (`DependencyGraphCorpus.cs`) or fixture that shows it, and what resolves it.

| # | Gap | Construct | Shown by | Resolution |
|---|---|---|---|---|
| 1 | A slot that reads a YAML scalar as written (B1) | FBL 4.1.4, 5.2, 5.3, 5.7 | `inline/b1-scalars-as-written` | **Resolved** by FBL 0.5 `read: "text"`, which the binding uses; an id and a reference are compared as written text. |
| 2 | A value that does not convert, read as the slot's default (B2) | FBL 5.2, 7.4 | `inline/unreadable-values`, `inline/b2-numbers-read-otherwise` | **Answered without module code**: the binding reads `x` and `row` as text and the `.dis` types them, a value that does not fit reading as its default. FBL 0.5's `fallback` converts by the YAML core schema first, which would read `0x10` as 16. |
| 3 | A reading order other than document order (B5), for lists that may be written in either order and share one id space | FBL 4.1 ("Reading order is document order") | `inline/relations-before-elements` | **Open.** `DependencyGraphReading` stands in. A binding-level order of rules (rule order before document order) would resolve it. |
| 4 | Opening an empty flow collection (B6) | FBL 6.2, 6.3 | `inline/b6-empty-list-with-comment`; the conformance fixture `dependency-graph-empty-flow-sequence` | **Text gap.** The library opens `elements: [] # …` into a block list and keeps the comment; FBL does not say that it may, with which splices, or what happens to the comment. The fixture shows the splices. |
| 5 | A new sequence item copies its previous sibling's gap after the dash (B9) | FBL 6.3 | `inline/b9-entries-indented-apart` | **Text gap.** The library does it; 6.3 does not state it. |
| 6 | The written value of an end that names nothing, and whether an end written empty names nothing (A2) | DISL 4.9, 11.2 | `inline/dangling-relation` | **Open.** The `.dis` reads it through `writtenEnd`, a plugin function the module feeds from the binding's host attributes `writtenFrom` and `writtenTo`, which DISL keeps out of CEL. |
| 7 | A relation with a dangling end, and an id held twice or not at all (B3, B4) | FBL 5.4; DISL 11.5 | `inline/dangling-relation`, `inline/duplicate-id`, `inline/missing-id`, `inline/a4-id-held-twice` | **Answered by a detour**: a dependency is read as an element with reference attributes, and every entry carries its `storedId`. This is the third binding that needs it. |
| 8 | The members of a one-line flow mapping that is an entry (B16) | FBL 4.1 (a flow collection is one value) | `inline/b16-merge-key-and-flow-entry` | **Accepted** by the owner's ruling (2026-10-17): such an entry reads with no id and no values. A slot that reads a member of a flow mapping would resolve it. |
| 9 | An order of findings that interleaves two codes by element (A3) | DISL 8.6, `constraints.order` | `inline/every-rule-interleaved` | **Open**, already recorded: `DependencyGraphFindings` orders them. |

## Sources

- Code: [etalii.adp.ide.standalone `src/diagrams/dependency-graph/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/dependency-graph), backend, client, api, tests, fixtures and example, at `b5192d47e`, after the conversion to DISL and FBL (dependency-graph-disl-fbl); shared helpers `GestureIds` and `RowRounding` in `src/backend/EtAlii.Adp.Documents/`, and the FBL and DISL libraries `EtAlii.Adp.Specification.Fbl` and `EtAlii.Adp.Specification.Disl`.
- Binding: [`dependency-graph.fbl`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/backend/EtAlii.Adp.Diagram.DependencyGraph/dependency-graph.fbl), embedded in the backend project, and its copy beside the FBL examples, [`dependency-graph.fbl`](../../specifications/fbl/dependency-graph.fbl), with the conformance fixture [`dependency-graph-empty-flow-sequence`](../../specifications/fbl/fixtures/dependency-graph-empty-flow-sequence/fixture.json).
- Example: [`examples/example-1/services.dgr`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/src/diagrams/dependency-graph/examples/example-1/services.dgr), a storefront's service dependencies.
- The original specification, removed from standalone's tree and readable at the commit before `d573ad87`: `.spec-workflow/archive/specs/dependency-graph/requirements.md` and `design.md`.
- Catalogue: standalone [`docs/tools.md`](https://github.com/etalii-adp/etalii.adp.ide.standalone/blob/develop/docs/tools.md), row `generic/dependencies`; the Notion "Tools" database, row "Dependency graph".
- Specifications: [DISL](../../specifications/disl/DISL-specification.md) and [DID](../../specifications/did/DID-specification.md) in this repository.
- Project conversations: no recorded decision about this type's design beyond the implementation; the catalogue seeding and the backend-centralization work that moved its document store onto the shared lifecycle are the only mentions.
