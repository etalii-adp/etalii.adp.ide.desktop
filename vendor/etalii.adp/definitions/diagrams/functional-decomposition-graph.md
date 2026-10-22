# Functional decomposition graph: what DISL does not express

This document accompanies [functional-decomposition-graph.dis](functional-decomposition-graph.dis), the DISL 0.3 specification of the functional decomposition graph diagram. The specification holds everything DISL 0.3 can state: the metamodel with its fixed and clamped sizes, the superellipse, parallelogram, trapezoid and diode shapes, the toolbox of dropped element tools, the menus with their groups, the placement and connect menus, the forms with their row ids and the width's `parse`, the right-button body drag that connects, the constraints with the tool's own codes, sentences and order, the conditional removal confirmation, base-36 ids, and the FBL binding the document is read and written through. What DISL 0.3 cannot state, the wire ids, is declared in the specification under `x-fdg` and described here in [The wire ids](#the-wire-ids); the language gaps are in [What DISL 0.3 cannot express](#what-disl-03-cannot-express).

The tool it describes is the standalone module `src/diagrams/functional-decomposition-graph/` in [etalii-adp/etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/functional-decomposition-graph), origin `etalii/functional-decomposition-graph`, documents with the extension `.fdg`. Every label, sentence and code in the specification is copied from the module's parity transcript, `backend/EtAlii.Adp.Diagram.FunctionalDecompositionGraph.Tests/Parity/functional-decomposition-graph.transcript.json`, read at standalone commit `6d1e0a24`. The work that rewrote it is the standalone specification `.spec-workflow/specs/functional-decomposition-graph-disl-fbl/`, task 2. Unless a path says otherwise, it is relative to the module: `backend/` is `backend/EtAlii.Adp.Diagram.FunctionalDecompositionGraph/`, and `client/` is the module's client folder.

## Contents

1. [What the diagram is for](#what-the-diagram-is-for)
2. [The types](#the-types)
3. [The document](#the-document)
4. [Interaction](#interaction)
5. [Rules, findings and refusals](#rules-findings-and-refusals)
6. [Notation](#notation)
7. [The wire ids](#the-wire-ids)
8. [What DISL 0.3 cannot express](#what-disl-03-cannot-express)
9. [The example](#the-example)
10. [Rulings from the owner](#rulings-from-the-owner)
11. [Sources](#sources)

## What the diagram is for

A functional decomposition graph breaks a product down from what the user sees to what the system does: UI elements contain UI elements, actions and data; actions own data and functions; data owns data and functions; functions own functions. An action can also *show* a UI element, which is how navigation is drawn without making it ownership.

- The notation is ADP's own rather than a standards body's, which is why the origin is `etalii/` (standalone `docs/tools.md`, the row for `etalii/functional-decomposition-graph`).
- The Notion "Tools" database row: kind Diagram, family "Business process & workflow notation", focus areas "Software delivery" and "Humans and agents together", rarity "Unique to ADP", state Prototype.
- State: ⚗️ Prototype, recorded under `x-adp.state`, which is catalogue data rather than DISL (see [Rulings from the owner](#rulings-from-the-owner)).

## The types

| DISL name | Document type | Wire id of its tool | Kind |
| --- | --- | --- | --- |
| `UiElement` | `ui-element` | `fdg.toolbox.ui-element` | element |
| `Action` | `action` | `fdg.toolbox.action` | element |
| `DataElement` | `data-element` | `fdg.toolbox.data-element` | element |
| `Function` | `function` | `fdg.toolbox.function` | element |
| `Comment` | `comment` | `fdg.toolbox.comment` | element |
| `UiChild` | `ui-child` | `fdg.connect.ui-child` | relation |
| `OwnsAction` | `owns-action` | `fdg.connect.owns-action` | relation |
| `OwnsData` | `owns-data` | `fdg.connect.owns-data` | relation |
| `OwnsFunction` | `owns-function` | `fdg.connect.owns-function` | relation |
| `Shows` | `shows` | `fdg.connect.shows` | relation |

- The abstract types `FdgElement`, `NamedElement`, `FdgRelation` and `Ownership` exist only in the specification, to share attributes, menus and forms and to state acyclicity once. The module has no such types.
- Each concrete type carries its document spelling as the fixed attribute `documentType`, and the function `documentTypeOf` maps a DISL name to it inside sentences.
- **`UnknownElement` and `UnknownConnection`** are two concrete types for an entry whose `type:` the notation does not have. They have no tool, no notation and no `create`, and the binding's catch-all rules are mapped to them rather than `unreadable`, so they stay in the model: their ids are counted by `std.duplicateId`, a connection to one is reported by the type check as `fdg.forbidden-link`, and `uniqueName` counts an unknown element's name. `UnknownElement` extends `NamedElement` because the transcript shows the tool giving such an entry the named menu ("Rename…", "Remove") and the rows Name, Description and Width; an unknown connection gets the connection menu and rows. The two types stand in for FBL gap 2 of the standalone specification (a rule chosen by a value cannot write that value), which would otherwise need ten typed rules.

## The document

The document is read and written through the FBL binding named in `persistence.binding`, `backend/functional-decomposition-graph.fbl#fdg`. That binding is written by a later task of the standalone specification and does not exist on standalone `develop` yet; until it does, the tool reads with `FdgParser` and writes with `FdgWriter`, and the menus, rows, findings and refusals are already this specification's.

- **Shape.** A line-oriented YAML subset: a header line `functional-decomposition-graph: 1`, then an `elements:` list and a `connections:` list. It is ADP's own schema, not DID. The header is mapped to the element type `Graph` (`typeMap`, `as: "header"`).
- **Element keys:** `id`, `type`, `name`, `description`, `text`, `x`, `y`, `width`, `height`. **Connection keys:** `id`, `type`, `from`, `to`, `name`, `description`; `typeMap` maps `from` and `to` to `source` and `target`.
- **Position is the top-left corner** (`x`, `y`). The node notations place the node by its top-left (`placement`), so no conversion is stated.
- **Height is stored only for a Comment.** The four named types share one height of 48, stated as a `fixed` attribute on `NamedElement` with the row reason "Only a Comment has its own height; the others share one.". A fixed attribute is not written.
- **A Comment has `text`, never `name`.** A `name` written on a Comment is kept as the read-only attribute `name` (`typeMap` maps the binding's `strayName` to it), reported as `fdg.unreadable-entry`, and counted by `uniqueName`, as the transcript's `names-counted` case shows ("New UI element 3").
- **Ids.** New entries get a ShortGuid, a version 4 GUID written as 25 characters of base 36 (`ids.encoding: "base36"`). Hand-written ids such as `task-detail` are kept as written. Ids are unique across elements and connections together.
- **Round trip.** An unchanged document comes back byte-identical, and every edit is a splice that leaves the rest of the file, its line endings and its comments untouched. Because the bytes are the test subject, standalone's `.gitattributes` marks `*.fdg -text`.
- **A body that is not YAML opens with an empty model and every edit refused** (finding B8 of the standalone specification, ruled in chat 2026-10-21). The one finding is `std.unparseable` with the code `fdg.unreadable-entry`, a warning: "The document could not be read as YAML: " and the parser's own message. An edit is refused with "<file> could not be read, so it cannot be edited.", or "…so nothing can be edited until it can: <finding>". The refusals are host sentences, worded by the host rather than by the specification or the binding (FBL gap 6 of the standalone specification).
- **Registration.** A diagram is added to an ADP project by a `.adp` file whose origin line is `etalii/functional-decomposition-graph` and whose body is `body: <name>.fdg` (`examples/field-service/field-service.adp`).

## Interaction

- **Adding** is a drop from the toolbox (`mode: "drop"`), with the new element's centre where it is dropped: a named element is 160 by 48, a Comment 240 by 96 with the text "New comment". There is no "Links" group. A click on a tool answers "An element is added by dropping it where it belongs.".
- **Placement menu.** The canvas menu has five entries, "Add ui element here", "Add data element here", "Add action here", "Add function here" and "Add comment here", in the order of `FdgElementTypes.All`.
- **New names are unique**: "New UI element", "New action", "New data element", "New function", then the same with " 2", " 3" and so on, counting every `name` in the graph (`uniqueName`, built from `positionIn` because DISL has no `lists.range`, finding A14).
- **Connecting** is a right-button drag from one element's body to another's (`connect.pointer`, `button: "secondary"`, `start: "body"`). The connect menu offers five entries labelled "ui child", "owns action", "owns data", "owns function" and "shows". Which relation one drag creates is [gap 7](#gap-7-which-relation-a-pointer-gesture-creates).
- **Rename** is F2 (`fdg.rename`); on a Comment the entry is "Edit text…", and on a connection "Rename…" (`fdg.rename-connection`).
- **Remove** is the Delete key: "Remove" for an element (`fdg.remove`), "Remove connection" for a connection (`fdg.disconnect`). Removing an element asks for confirmation only when it has connections, naming how many: "Removing this element also removes the N connection(s) to or from it." (`Confirmation` with `count` and `threshold`; a self-loop is counted once).
- **Menus** all sit in one group and show no standard entries; they are the same when the diagram is read-only.
- **Rows.** A named element has Name and Description in "Identity" and Width in "Size"; a Comment has Text, Description, Width and Height; a connection has Name (row id `connectionName`) and Description.
- **Width and Height show four decimals** ("140.1235"), as the file holds them (finding A6, ruled in chat 2026-10-21). A typed value that is not a positive number is refused with "'<value>' is not a size this graph can draw.".
- **Sizes below the minimum** (width 80, a Comment's height 48) are clamped, not refused, whether dragged or typed (`outOfRange` with `clamp` both ways). A stored size below the minimum is kept, drawn and reported.

## Rules, findings and refusals

- **Codes** (`constraints.order`): `fdg.duplicate-id`, `fdg.unreadable-entry`, `fdg.dangling-reference`, `fdg.self-link`, `fdg.forbidden-link`, `fdg.second-parent`, `fdg.second-shows`, `fdg.ownership-cycle`. `fdg.unreadable-entry` is a warning, because the lines are kept and the document still draws; every other code is an error.
- **Connect refusals** run in a fixed order: type, then cardinality, then cycle, each naming its check: "Refused by the type check: …", "Refused by the cardinality check: `X` already has its `owns-data` parent, and may have 1.", "Refused by the cycle check: `T` already owns `S`, so this link would make an ownership loop.". A duplicate id answers "That id is already used in this graph.".
- **The four ownership relations are acyclic together; Shows is not.** `acyclic` on the abstract `Ownership` covers the union of its subtypes (DISL 0.2, *Changes from 0.1*). An action may show the UI element that owns it.

## Notation

- **UI element**: the built-in `superellipse` with `exponent` 4. **Data**: `parallelogram`, `skew` 0.2, `direction` right. **Action**: `trapezoid`, `inset` 0.15, `direction` down, which differ from the built-in defaults and so are stated. **Function**: a custom diode shape whose corner radius is `min(h/2, w)`. **Comment**: a note.
- **Colours** are light and dark theme tokens with a `contrast` requirement of 4.5 against the text.
- **Connections** are cubic béziers with a filled triangle at the target and the name at the midpoint. Placement is free and manual; there is no snapping and no automatic layout.

## The wire ids

The standalone client (`client/fdgIds.ts`) and backend share wire ids that DISL has no place for. `x-fdg` maps them from the specification's names, as `x-ghg` does for the hype cycle:

- `tools`: each tool's id and the action its drop runs.
- `actions`: keyed by the entry's operation, else its kind; a key `<Type>/<kind>` overrides that for entries offered on `<Type>` or its subtypes (`FdgRelation/editLabel` is `fdg.rename-connection`, `FdgRelation/delete` is `fdg.disconnect`), and a `<name>` in an id is replaced by the entry's argument of that name, mapped through `types` (`fdg.connect.<relation>`).
- `properties`: keyed by the form item's id, else its attribute (`connectionName` is `fdg.connection-name`).
- `types`: DISL name to document type. An element's wire id is the id written in the document, with no prefix.

## What DISL 0.3 cannot express

These are the DISL items of section C of the standalone specification's requirements, and the one its design and this rewrite add. Each is a candidate for the DISL specification, recorded with its evidence; the tool's behaviour is kept by a named stand-in in the standalone module, never by a silent workaround. The FBL items of section C (gaps 1 to 6 and 10) belong to the binding and are recorded with it.

### Gap 7: which relation a pointer gesture creates

Five relation types claim the same right-button body drag. The tool creates the one relation whose ends admit both elements, and there is exactly one for every allowed pair because each relation is identified by its target type; the connect menu offers all five. DISL 0.3's `connect.pointer` says how a gesture starts, but not how a runtime chooses among relation types that claim the same gesture (finding A9). Evidence: the transcript's connect cases. Would resolve it: a rule in 7.x or 9.x that a gesture claimed by several relation types creates the one whose endpoint constraints admit the pair, and offers a choice otherwise.

### Gap 8: an order of findings that is not by code and then by line

`constraints.order` orders findings by code, then by line. The tool's order differs in three places (finding A13, worded for all three by Amendment 1 of the standalone design):

1. **Within `fdg.unreadable-entry`**, the findings come in reading order: elements, then connections, then the header.
2. **Across the link codes**, one finding per connection in document order, mixing `fdg.dangling-reference`, `fdg.self-link` and `fdg.forbidden-link`, where code-then-line would sort three connections that fail as forbidden, dangling, forbidden into another order.
3. **For cardinality**, by relation in the order of `FdgRelations.All`, not by line.

The owner ruled in chat on 2026-10-11 that today's order stays. It is held by the stand-in `FdgFindingOrder`, with a fixture for each place. Would resolve it: an `order` per code, or a `by` of `document`, `relation` or `line` on `constraints.order`.

### Gap 11: the id an unresolved end was written with

When a connection's end names no element, the tool still counts it for cardinality and names it: "`ghost` has 2 `ui-child` parents; it may have 1.". A CEL expression sees an unresolved end as absent, so it cannot read the id it was written with, and `std.multiplicity` cannot count connections whose shared end resolves to nothing. The tool also reports one link finding per connection (dangling, then self, then unknown relation, then pair) where `std.endpoints` and `std.references` would each raise one. Both are held by the stand-in `FdgLinkChain` (finding A11, under Q4 of the standalone requirements). Would resolve it: a `sourceId` and `targetId` on a relation in the CEL context (12.x), holding the ids as written.

### Gap 12: the detail and place of a cycle and of a cardinality finding

Added by this rewrite. Section 8.7 binds no `detail` keys for `std.acyclic` findings, only `violation.cycle` in refusals, and gives no location for `std.multiplicity` or `std.acyclic` findings. The tool reports:

- a loop as "Ownership loops: a -> b -> a.", the members rotated to start at the least id, one finding per loop, on the line of that least-id member;
- a cardinality breach on the line of the last connection over the limit.

The specification's message reads `detail.?cycle` optionally, so it is correct once a runtime supplies it; until then the standalone runtime supplies it itself. Would resolve it: `detail.cycle` for `std.acyclic` findings, and a stated location for both built-ins, in 8.7.

### Answered by DISL: gap 9

Gap 9 of section C, a size below its minimum that is clamped on commit rather than refused (finding A6), is answered by `outOfRange` (4.3, DISL 0.2) with `drag` and `typed` both `clamp`, and the specification states it so. The standalone stand-in `FdgSizeClamp` therefore stands in for a runtime that does not yet implement `outOfRange.typed`, not for a language gap.

## The example

`examples/field-service/field-service.fdg` in the standalone module is a field-service app decomposed from its screens down to functions, with two Comments and two Shows links. Those links form a navigation round trip (Task list, Open task, Task detail, Back to list, Task list), a loop that is legal because it closes only through Shows. Its readme (`examples/field-service/readme.md`, "What it does not demonstrate") lists what it leaves out.

The example in the DISL specification, [`specifications/disl/functional-decomposition.dis`](../../specifications/disl/functional-decomposition.dis), is a DISL 0.2 excerpt that shows 0.2 constructs; where it differs from the tool, its own `doc.description` says so.

The standalone client readme (`client/readme.md`, "What is not settled yet") still says only one of Shows and the ownership relations can be reached from an Action. That was resolved when the relation came to be inferred from the pair; the readme was not updated.

## Rulings from the owner

- The tool stays a Prototype (Peter, 2026-09-26: "no, keep it as is as it is still a prototype").
- A body that is not YAML opens empty with every edit refused, the Width and Height rows show four decimals, and finding B12 is struck because it does not occur (chat, 2026-10-21).
- The order of findings stays as today, held by `FdgFindingOrder` (chat, 2026-10-11; [gap 8](#gap-8-an-order-of-findings-that-is-not-by-code-and-then-by-line)).
- DISL gaps are proposed as additions to the DISL specification rather than kept as plugins (2026-10-05), which is why the earlier plugin persistence is gone and only the wire ids remain under an `x-` key.

## Sources

- Module code, tests and the parity transcript: `src/diagrams/functional-decomposition-graph/` in etalii.adp.ide.standalone.
- Specifications: `.spec-workflow/specs/functional-decomposition-graph/` and `.spec-workflow/specs/functional-decomposition-graph-disl-fbl/` in etalii.adp.ide.standalone.
- Catalogue row: `docs/tools.md` in etalii.adp.ide.standalone.
- Notion: the "Tools" database row for Functional decomposition graph.
