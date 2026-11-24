# Timeline diagram

[timeline.dis](timeline.dis) specifies ADP's **Timeline diagram** (`generic/timeline`) in DISL 0.3. The specification holds everything DISL 0.3 can state: the metamodel, the time axis and the row axis, the notation, the toolbox, the context menus, the forms with their checks, the constraints with the tool's own codes and sentences, the operations, the removal's confirmation and the persistence. Reading and comparing times is done by the CEL functions of the plugin `net.etalii.adp.generic.timelineTimes`, because DISL's CEL has no timestamps, and the row arrangement is the layout plugin `net.etalii.adp.generic.timelineRowArrange`. The wire ids are declared under `x-timeline`. This page holds the rest, so that an IDE implementing the tool from the specification ends up with the tool that exists today, and says, in [What DISL 0.3 cannot express](#what-disl-03-cannot-express), where the specification stops. Each aspect names the code or document that shows it.

The reference implementation is the standalone module [`src/diagrams/timeline/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/timeline) in etalii.adp.ide.standalone, which bundles this specification, derives its toolbox, context menus, property rows, findings, placement additions and removal confirmation from it (`backend/EtAlii.Adp.Diagram.Timeline/TimelineDefinition.cs`), and reads and writes its body through its own FBL binding (`backend/EtAlii.Adp.Diagram.Timeline/timeline.fbl`, `TimelineBody.cs`). The conversion that brought the module onto the binding is the standalone repository's spec-workflow specification `timeline-disl-fbl` (`.spec-workflow/specs/timeline-disl-fbl/`); finding numbers below (B1, B11 and so on) are those of its requirements and design documents, and its parity transcript, `backend/EtAlii.Adp.Diagram.Timeline.Tests/Parity/timeline.transcript.json`, records what the tool does over the fixtures beside it. Its original requirements live in that repository's history (`.spec-workflow/archive/specs/timeline-diagram/`, removed in `ece03c36`); requirement numbers below (R3.4 and so on) refer to that document. The Notion "Tools" row for the tool says: kind Diagram, focus areas *Planning and roadmapping* and *Technology assessment*, "Time is the axis that matters; a dedicated timeline tool snaps elements to rows and dates, which a generic canvas leaves to the eye." The project's conversations contain no further decisions about this tool, apart from the owner's ruling for this definition that the code wins: where the specification and the code disagreed, the specification was changed to say what the code does.

Paths below are relative to `src/diagrams/timeline/` in the standalone repository unless they start with `src/`.

## Contents

1. [How the specification maps the tool](#how-the-specification-maps-the-tool)
2. [The document on disk](#the-document-on-disk)
3. [Times and the time plugin](#times-and-the-time-plugin)
4. [Rows](#rows)
5. [The time axis](#the-time-axis)
6. [The time ruler](#the-time-ruler)
7. [Appearance](#appearance)
8. [Gestures](#gestures)
9. [Actions and keys](#actions-and-keys)
10. [Arrange diagram](#arrange-diagram)
11. [Properties](#properties)
12. [Diagnostics](#diagnostics)
13. [Wire ids](#wire-ids)
14. [What DISL 0.3 cannot express](#what-disl-03-cannot-express)
15. [Language gaps](#language-gaps)
16. [Definition defects repaired](#definition-defects-repaired)
17. [Other differences a runtime will meet](#other-differences-a-runtime-will-meet)
18. [Pending with the owner](#pending-with-the-owner)
19. [What stays code](#what-stays-code)
20. [Scale and performance](#scale-and-performance)
21. [Known gaps](#known-gaps)

## How the specification maps the tool

| Tool concept | In timeline.dis | Notes |
| --- | --- | --- |
| Element with an end (a period) | node type `Period` | Wire type `generic/timeline+period`. Labelled "Element" in the toolbox. |
| Element without an end (a moment) | node type `Moment` | Wire type `generic/timeline+moment`. |
| Both | abstract `Element` | Holds `label`, `begin`, `row`. |
| Connection | relation `Connection` | Wire type `generic/timeline+connection`; the UI calls it a "relation" and its type label is "Relation". |
| x axis | time axis `time` | Seconds since the Unix epoch in the implementation. |
| y axis | linear axis `rows`, 60 canvas units per row | See *Rows* below. |

The split into `Period` and `Moment` is a modelling choice of this specification. In the file there is one kind of element, and whether it is a period or a moment is decided only by whether an `end` key holding a scalar is present, even an empty one or `null`; an entry whose `end` holds a mapping or a sequence is a moment (the `when` of the binding's `period` and `moment` rules, finding B11, fixture `findings/end-shapes.tml`). Giving a moment an end, or removing a period's end, is a retype in DISL (`giveEnd`, `removeEnd`) and a single-key edit in the file.

**The model is read through an FBL binding.** `persistence` names the timeline module's own binding, `timeline.fbl#timeline` (see [The binding](#the-binding)), with a `typeMap` that maps its types onto this metamodel. A YAML reading under FBL 0.5 keeps every value as the text it was written with (FBL 4.1.5, 5.2), which the timeline needs: a label `yes`, `1.0` or `null` is that text. Each element and relation is given a unique model id: the first declaration written with an id keeps it, and a later one, or one written without an id, is addressed by its place; the written id is kept beside the model as the host attribute `storedId`, and findings name an element by it (DISL 11.5.4). A relation end resolves to the first **element** written with that id, never to a relation.

The standalone module reads every `.tml` through that binding (`backend/EtAlii.Adp.Diagram.Timeline/TimelineBody.cs`) and builds the DISL model from the reading with the DISL runtime's model builder under the `typeMap`; `TimelineReading.cs` projects the same reading onto the model its mapper, session, scale and arrangement read.

## The document on disk

The timeline does not store DID: it stores its own YAML format. `persistence.format` is `fbl`, and `binding` names the module's own binding, which states the file's shape and how it is written (FBL 6); the account below is what the tool does, and [The binding](#the-binding) says how the module's binding states it.

**Two files.** A timeline is `<name>.adp`, whose first line is the MIME-style origin `generic/timeline`, plus a sibling body `<name>.tml` (Timeline Markup Language) holding the timeline itself (R1.1). `.tml` is owned outright by this type: a bare `.tml` routes to it with no registration step, and the extension is not shared with any other tool (`backend/EtAlii.Adp.Diagram.Timeline/Diagram.cs`, R1.2).

**Why its own format.** The Gantt family (`mermaid/gantt`, PlantUML gantt) was considered and rejected: they compute their layout, express links only as scheduling dependencies and have nowhere to put an authored row. YAML is the serialization; schema and extension are ADP's own (R1.4).

**Layout of the file.**

```yaml
timeline: 1
elements:
  - id: discovery
    label: Discovery
    begin: 2025-04-27
    end: 2025-06-05
    row: 11
  - id: launch
    label: Launch
    begin: 2025-12-04
    row: 8
connections:
  - id: c1
    from: discovery
    to: launch
    label: informs
```

- `timeline: 1` is the schema marker. `elements` is a sequence of mappings with `id`, `label`, `begin`, optional `end` and `row`. `connections` is a sequence of mappings with `id`, `from`, `to` and optional `label`. Keys are read with those exact names; a missing `connections` key means no connections, and a missing key in an entry reads as the empty text.
- A new document is exactly `timeline: 1\r\nelements: []\r\n`: CRLF, a trailing newline, no title and no sample content (the binding's `template.text`, and the empty document `backend/EtAlii.Adp.Diagram.Timeline/ServiceCollection.AddTimeline.cs` registers, R1.3). The first element opens `elements: []` into a block sequence (fixture `findings/first-element.tml`, finding B4).
- An empty file, or one whose root is not a mapping, is an empty timeline, not an error: the binding's selectors match nothing in it.

**Only the affected bytes change (R2.1, R2.2).** Nothing ever serializes a model back to the file. Every edit is a model change that the binding plans as splices (FBL 6), run in one place (`backend/EtAlii.Adp.Diagram.Timeline/Commands/TimelineEdits.cs`), and undoing it puts back the text before it byte for byte, through the host's shared restore. Consequences an implementation must reproduce:

- A document opened and saved with no edit is byte-identical: line endings (LF or CRLF), indentation, key order, comments, blank lines, quoting style and a missing final newline all survive.
- Keys the tool does not model survive verbatim, at document, element and connection level, including nested sequences (R2.3; fixture `unmodelled-keys.tml`). DISL's `x-*` rule covers the specification, not user files.
- A changed value replaces its own span, in its own style: a trailing comment on its line and the author's quotes survive (fixtures `trailing-comment.tml`, `quoted-values.tml`, finding B5). A row is the exception: one written quoted is written plain when it changes (see [Other differences](#other-differences-a-runtime-will-meet)).
- A new element is appended after the last existing one, of either kind, with indentation copied from the file, the gap after the `-` included (fixture `indentation.tml`, findings B7 and B16); only an empty document falls back to two spaces. Keys are written in the order `id`, `label`, `begin`, `end` (only for a period), `row`, and a key given to an entry that lacks it goes where that order puts it, so an end given to a moment goes after its `begin` (fixture `give-end.tml`, finding B6). A label is quoted only when it needs to be.
- A new connection is appended after the last existing one; when the file has no `connections:` key, the key is created at the end of the document together with the first entry (the binding's `insert.create`), and undoing that insert removes the key again so the file returns byte for byte. Removing the last connection keeps the `connections:` key (`remove.container: "keep"`). The `label` key is omitted when the label is empty, and clearing a connection's label removes the key.
- Removing a period's end removes the `end` key and its value (R11.3). Giving a moment an end and removing a period's end are each one change of the `end` key, never a removal and an addition of the entry, so the entry keeps its place and its relations ([gap 6](#6-fbl-a-retype-between-two-rules-that-match-one-entry)).
- Removing an element removes the relations that name its written id, in the same undo step (the binding's `remove.cascade`); a removed entry takes its own comment lines with it.
- The fixture corpus in `backend/EtAlii.Adp.Diagram.Timeline.Tests/Fixtures/` (comments, CRLF/LF pair, no trailing newline, odd indentation, quoting, shared rows, unmodelled keys) is the test of all this; `*.tml -text` in `.gitattributes` keeps git from rewriting it.

**The binding.** The module's binding is `backend/EtAlii.Adp.Diagram.Timeline/timeline.fbl` in the standalone repository, binding `timeline`, at FBL 0.5. It starts from the example `specifications/fbl/timeline.fbl` of this repository, which is not changed, because the conformance fixtures `timeline-edits`, `timeline-bom-lf` and `timeline-written-text` rest on its relation rule and its header. It differs from the example in these ways:

| Difference | Why |
| --- | --- |
| A relation is an element rule `connection`, with `from` and `to` as reference attributes `by: "storedId"`, which the `typeMap` maps to `source` and `target`. | A relation whose end names no element is kept, not drawn, and reported as `timeline.dangling-connection`; an FBL relation whose end names nothing is not created at all (FBL 5.4). |
| Every rule reads `storedId` from the `id` key, read-only, a host attribute. | A later holder of an id is addressed by its place, and the tool reports it as `timeline.duplicate-id` and finds an element by what is written. |
| No `header`. | A missing `timeline:` key, or another version, raises nothing, as the tool raises nothing today; the key stays unbound and is kept, and the template still writes it. |
| `row` is read as text, with `default: 0`, without `quoted` and `fallback`. | The tool reads a row with .NET's integer parse of the scalar's text: `1.0`, `0x10` and `0o17` are row 0 and `"3"` is 3, where FBL's conversion of a typed value would make `1.0` row 1 and `0x10` row 16. |
| `begin` has `default: ""`. | A missing `begin` reads as the empty text, which the tool reports as an unreadable time; an absent slot would leave the attribute unset, and no rule would report it. |
| The `connection` rule has `when: "entry.all(k, true)"`. | A sequence entry that is not a mapping is taken by no rule and passed over, as the tool passes it over; without the condition the rule would read a scalar item as a relation without an id. |
| The `period` rule's `when` is `entry.all(k, true) && has(entry.end) && !(type(entry.end) in [map, list])`, and the `moment` rule's its complement, where the example asks `has(entry.end)`. | The tool reads an entry as a period only when its `end` holds a scalar, the empty one and `null` included; an `end` holding a mapping or a sequence makes a moment (standalone fixture `findings/end-shapes.tml`, finding B11). `entry.all(k, true)` holds of a mapping only, so a sequence item that is not a mapping is taken by no element rule either (finding B8). |
| Both element rules insert with `place: "end"`, where the example has `after-last`. | The tool appends a new element after the last entry of `elements`; `after-last` is right after the last entry of the same rule (FBL 6.2), so once the last entry is a period a new moment would go after the last moment, in the middle of the list (standalone fixture `indentation.tml`, `Parity/timeline.findings.json` B16). |

`label`, `from`, `to` and `storedId` need no `read`: under FBL 0.5 a string attribute, a reference and a host attribute read their written text unasked. `begin` and `end` keep `"read": "text"` and `time: "keep-precision"` as the example has them, and the period rule's `end` keeps `empty: "remove"`.

**Ids.** Ids live in the file because the schema is ADP's own, and they are unique across elements and relations together. New ids are ShortGuids: a version 4 GUID written as 25 lowercase characters of base 36 (`src/backend/EtAlii.Adp/ShortGuid.cs`), for example `467lu0vgaiudevamnkpl6k052`, which `persistence.ids` states as `uuid-v4` with `encoding: "base36"`. Hand-written ids (`discovery`, `c1`) are equally valid. A command that creates an element generates its ids once, when the gesture happens, so a redo re-creates the element under the same ids.

**An unreadable file** (not YAML) opens in the unavailable state naming YamlDotNet's message and line, and no edit is accepted until it is fixed, so a broken file is never made worse (R2.4). The module asks YamlDotNet first, through its stand-in `backend/EtAlii.Adp.Diagram.Timeline/TimelineYamlSyntax.cs`, so that the sentence, the line and which files are refused stay what they were ([gap 7](#7-fbl-the-wording-and-the-acceptance-of-an-unreadable-body)); a body it accepts is read by the binding alone. The store keeps the last good document across a reload that cannot read, and a deleted body closes rather than lingering (the host's shared document lifecycle, `TimelineDocumentStore.cs`). Changes made to the file outside ADP are picked up and pushed to every open view.

## Times and the time plugin

`begin` and `end` are normally ISO 8601, either date-only (`2026-01-05`) or date-time without offset (`2026-01-05T14:30:00`). The implementation keeps the **text** as written and its **precision** (date or date-time), decided by whether the text contains a `T` (`_Model/TimelineInstant.cs`, `_Model/TimelinePrecision.cs`). timeline.dis declares both as `datetime` with `timezone: "preserve"`, and every value in the model is the text as written, also one that is not a time at all.

DISL's CEL has no timestamps, so the specification reads times through the CEL functions of its plugin `net.etalii.adp.generic.timelineTimes` (DISL 13.1.1). The standalone module implements them with the very calls it reads and places times with (`backend/EtAlii.Adp.Diagram.Timeline/TimelineTimes.cs`, `TimelineInstants.cs`); a host without the plugin falls back to the declared fallbacks, which read ISO 8601 only.

| Function | What it does | Fallback |
| --- | --- | --- |
| `readableTime(text)` | Whether `text` is a time the tool can read: .NET's `DateTimeOffset.TryParse` under the invariant culture, surrounding whitespace ignored, a value without an offset taken as UTC. That reads more than ISO 8601: `2026/07/01` and `July 5, 2026` are readable, `2026-02-30` and the empty text are not. | An ISO 8601 date or date-time pattern. |
| `compareTimes(a, b)` | -1, 0 or 1 as the instant `a` names is before, at or after `b`'s, offsets applied. | Lexical comparison. |
| `dateAfter(time, days)` | The date `days` days after the instant `time` names, written `yyyy-MM-dd`, in UTC. | None. |
| `today()` | Today's date in UTC, `yyyy-MM-dd`; not deterministic. | None. |
| `writtenEnd(relation, end)` | The id a relation's `source` or `target` end is written with (`from`, `to`): the end the relation keeps, which the `typeMap` maps from `from` and `to`, also when it names no element and the end is unset in the model (DISL 11.2). It stays a function, rather than `std.references`, because `std.references` reports every unset end and the tool does not report an end written empty or not written at all. | The resolved end's id, or the empty text. |

- A value is read at face value and pinned to offset zero; it is never interpreted in the machine's local time zone (`TimelineInstants.cs`). A value written is the value drawn (R3.7).
- One element never mixes a date-only value with a date-time value (R3.2). The check is the tool's own: whether each text contains a `T`, so `Tuesday 7 July 2026` counts as a date-time. DISL's `std.mixedPrecision` compares written precisions, which a runtime does not know for a text kept as written, so the specification states the tool's test as the rule `mixedPrecision` and the form checks.
- A value the tool writes back after a drag or resize takes the precision of the value it replaces: `2026-01-05` stays `2026-01-05`, never `2026-01-05T00:00:00` (`TimelineScale.ToText`). Date-times are written as `yyyy-MM-ddTHH:mm:ss`, rounded to the whole second.
- Values created by the tool (toolbox drops, create-and-relate, add after or below, the menu's additions) are always date-only.

## Rows

A row is an integer the author owns: negative rows are valid, gaps are allowed, and several elements share a row (R3.5). It is a placement grid and **nothing more**: no identity, label, membership or meaning (R3.6). That is what keeps this a timeline rather than a swimlane, so an implementation must not add lane headers, lane labels or lane packing. DISL's ordinal axis would give rows identity, so timeline.dis uses a linear axis in row units with grid snapping of one row instead.

- A row is 60 canvas units high (`TimelineRows.Height`, mirrored as `ROW_HEIGHT` in `client/timelineBindings.ts`). The height is a rendering constant, not data.
- A vertical drag snaps to the nearest row; halves round **away from zero**, so the midpoint between two rows resolves the same way on both sides of the origin (`src/backend/EtAlii.Adp.Documents/RowRounding.cs`, shared by the client's `snapToStep`). DISL's `nearest` does not say how halves round.
- A row that does not read as an integer is read as row 0.
- Overlapping elements are drawn overlapping; the tool never moves anything to avoid overlap (R4.5), except when the reader asks for [Arrange diagram](#arrange-diagram).

## The time axis

- The implementation's x is seconds since the Unix epoch, as a double (`TimelineScale.cs`); the time-to-coordinate conversion lives in exactly one function pair.
- The client freezes a seconds-to-canvas-units scale when the first non-empty model arrives: the span of all elements plus 10 % on each side, fitted to about 1200 units; for an empty timeline, one day is 20 units (`timelineScaleOf` in `client/TimelineCanvas.tsx`). It is frozen so an edit never rescales the drawing under the user. DISL's fixed `scale` of 20 units per day in timeline.dis is the empty-timeline value; the fitted initial zoom is this implementation's.
- The same document always draws identically (R4.7).

**x snapping depends on the element.** A date-only element's begin snaps to the start of a day when dragged, and its edges snap to days when resized. An element whose times carry a time of day does not snap in x at all and keeps whole seconds (`snapAxes` in `client/timelineBindings.ts`, which carries a day's width in each date-only element's payload; `TimelineScale.ToTime`). DISL cannot make snapping depend on a value's written precision, so timeline.dis declares day snapping, which is the case for every element the tool itself creates.

## The time ruler

The ruler is chrome fixed to the view, not to the diagram (R5). DISL's `ruler` declares a position and a size; the behaviour below is the tool's own (`client/TimelineRuler.tsx`, `client/timelineTicks.ts`, `client/timeline.css`):

- It sits along the **bottom** of the view, 24 px high, stays in sight whatever is scrolled vertically, and is visually subordinate: muted 10 px labels on a mostly transparent band, no pointer events.
- Its labels slide with the content, because their x comes from the same view transform as the elements.
- The label interval adapts to the visible span so labels neither crowd nor vanish: at least 80 px per label, choosing from 1 s, 15 s, 1 min, 5 min, 15 min, 1 h, 6 h, 1 day and 1 week, then calendar months, quarters and years (any whole number of years). Labels fall on round UTC boundaries, never on offsets from the viewport edge.
- Label text: `HH:mm:ss` below a minute, `HH:mm` below a day, `Mon d` for days and weeks (`Mon yyyy` on the first of a month), `Mon yyyy` for months and quarters, and the bare year on 1 January and for year ticks.
- It is measured against the surface's real width, so a label sits over the elements it dates.

## Appearance

- A period is a box 36 units high, its left edge at begin and its right edge at end, at least 2 units wide. It sits at the top of its row, leaving a 24-unit gutter to the next row. Its label is inside, on one line, truncated.
- A moment is a diamond with a radius of 9 units, drawn with its left tip at begin and its centre 18 units below the top of its row, so it lines up with the middle of the periods on the same row. Its label is beside it, to the right. It is filled with the accent colour and has no outline (`client/timeline.css`, the library's `moment` shape). timeline.dis expresses the vertical alignment as a placement offset of 0.3 row.
- An element with an empty label shows its id instead (the specification's function `shownLabel`).
- A connection leaves the **end** anchor (right-side midpoint) of its source and arrives at the **begin** anchor (left-side midpoint) of its target, always; anchors never move to the top, bottom or a corner (R8.1). It is a cubic bezier that leaves and arrives horizontally. When the target begins left of where the source ends, the curve loops forward out of the source and back into the target (`forwardBezierPath` versus `horizontalBezierPath` in `src/client/src/canvas/connectors.ts`); DISL's `bezier` routing has no such rule. It ends in the library's arrowhead at its target, which timeline.dis states as `targetMarker: "arrowFilled"`; an earlier version of this document and of timeline.dis said it had none, which the canvas has not drawn since it moved onto the canvas library. Its label sits at the midpoint, 6 units above the line.
- Connections are redrawn continuously while an element is dragged or resized, not only on release (R6.5, R7.7, R8.6).
- The shared canvas styling (`src/client/src/canvas/canvas.css`) supplies colours and selection states; the timeline adds only the moment's fill and the ruler.

## Gestures

- **Move.** A horizontal drag shifts begin and end by the same amount, keeping the duration; a vertical drag changes only the row; a drag with both does both, as **one** edit with one undo entry (R6.1 to R6.4). A moment never gains an end from a drag. While dragging, a hint above the element shows the time under its left edge and the row it would land on, as `yyyy-MM-ddTHH:mm:ss · row N`. `Escape` returns the element and records nothing (R6.6).
- **Resize.** A selected period shows adorners on both edges; the left one changes only begin, the right one only end. The moving edge stops at the other instead of crossing it, on the client and again on the server (R7.4, `SetTimelinePlacementCommandHandler`; the placement rule `noInversionByGesture`). A moment offers no right adorner (R7.5). A resize is sent as a property edit of begin or end, formatted in the element's own precision.
- **Relate.** Dragging from an element's **end** anchor to another element relates source to target. Dragging from its **begin** anchor runs the relation the other way: the element dropped on becomes the source, because what precedes an element points into it. The whole gesture travels as one stateless call (`TimelineContextActionProvider.cs`, through the shared `src/backend/EtAlii.Adp.Documents/GestureIds.cs`), deliberately replacing a two-call protocol whose stale state once related the wrong pair. The context menu `for: ["connection"]` with `runSingle` is that call's one action, "Relate".
- **Create and relate.** Releasing a relation on empty canvas creates a new element there (begin at the start of the dropped day, row nearest the drop, 14 days long, labelled "New element") and relates it, in one command and one undo. From the begin anchor, the new element is the source and the existing one the target.
- **Self relations are refused** ("An element cannot be connected to itself.", the connect rule `noSelfRelation`); two relations between the same pair are allowed, because each carries its own label (R8.7, R8.8). A self relation written by hand is drawn and not reported.
- **Toolbox drop.** The palette has two items, *Element* and *Moment* (R9). A drop creates the element with its begin at the start of the day under the drop and its row nearest the drop; an element is 14 days long. It is labelled "New element" or "New moment". The drop and the menu run the same operation, `addElementHere` or `addMomentHere`, and the toolbox's `initial` values say the same.
- There is no connection tool in the palette; relations are made only by dragging from an anchor (`contextTools`).

## Actions and keys

Actions reach the ribbon, the right-click menu and the keyboard through one path (R11.2). The specification's `contextMenus` give them, in three groups for an element (the edits, the additions, Arrange) and two for a relation; the standalone module derives its menus from them (`TimelineContextActionProvider.cs`).

| On | Action | Key | Behaviour |
| --- | --- | --- | --- |
| Element | Rename… | F2 | Asks for the label. |
| Moment | Give it an end… | | Asks for the end, prefilled with the begin as written (titled "Give it an end", field "End", button "Set"); refuses an unreadable time and an end before the begin as typed. |
| Period | Remove its end | | Removes the `end` key; the period becomes a moment. |
| Element | Remove | Delete | Removes the element and every relation attached to it. With no relations it happens at once; otherwise it asks first, naming the count ("Removing this element also removes the 1 relation attached to it." / "…the N relations…"), as a danger confirmation titled "Remove" with the button "Remove" (R2.5). The specification's `deletion.Element.confirm` counts the relations with a threshold of 1, a relation from the element to itself counted once. |
| Element | Add element after | Tab | A new element 6 days after this one's end (or begin, for a moment; or today when neither reads), 14 days long, same row, related from this one. |
| Element | Add element below | Enter | A new element with this one's begin as a date (or today), one row down, 14 days long, related from this one. |
| Relation | Relabel… | F2 | Asks for the label; an empty label removes the key. |
| Relation | Remove relation | Delete | |
| Empty canvas | Add element here, Add moment here | | Placed at the clicked time and row. From a menu without a position, asks for the begin (prefilled with today, dialog titled "Add element" or "Add moment", field "Begin", button "Add") and uses the row of the element the gesture anchored on, or row 0. |
| Anywhere | Arrange diagram | | See [Arrange diagram](#arrange-diagram). |

The client's keys come from the same menus (`compileNotation` over timeline.dis): F2, Delete, Tab and Enter on an element, F2 and Delete on a relation, each sent to the backend as that key, and a double-click on an element renames it, as the nodes' `doubleClick: "editLabel"` says. An action that does not apply to the selection is reported as not applicable rather than failing silently (R11.8). Every edit is one command with an inverse on the project's history (R11.1).

## Arrange diagram

"Arrange diagram" puts every element on the row that keeps the diagram least cluttered, as one undoable step; only rows change, never a time (`TimelineArrangement.cs`, `ArrangeTimelineCommandHandler.cs`). It is offered on an element, a relation and empty canvas alike, which keeps it in the ribbon. It is unavailable while the timeline has no element, with the reason "There is nothing to arrange until this timeline has an element."; the menu shows that sentence beside the entry also while it is available, as the hand-written menu always did.

It is not DISL 0.3's standard `rows` algorithm (10.2), so the specification names the layout plugin `net.etalii.adp.generic.timelineRowArrange`:

- Extents are taken at the client's first fit: the timeline's span, at least one day, over 1000 of its 1200 units. A period covers its begin to its end, at least 2 units; a moment covers 9 units before its time to 15 units after it plus its shown label's width at 12 units.
- Elements without an id, without a readable begin, or with an id an earlier element has, take no part and keep their row.
- The shared row packing (`src/backend/EtAlii.Adp.Layout/RowPacking.cs`) puts elements onto as few rows as their extents allow, 12 units apart, drawing related elements to nearby rows.
- A moment and an element later than it that it is related to are kept on different rows, so their relation does not run through the moment's label. `rows` has no such rule.

## Properties

The property grid shows, for an element: **Label** (group Identity), **Begin** and **End** (group Timing; End only for a period) and **Row** (group Placement). For a relation: **Label**, and **From** and **To** read-only with the reason "Reconnecting is done on the canvas, by dragging the relation's end to another element.", showing the ids the relation is written with, also one that names no element (R10). The specification's `forms` give these rows; the standalone module derives them (`TimelineContextPropertyProvider.cs`).

- Begin and End are plain line editors validated on commit; there is no date picker yet (R10.6). A value that does not read as a time is refused ("'…' is not a time this timeline can read."), a value in the other precision than the element's other time is refused ("This element uses the other time form; begin and end must both be dates, or both carry a time."), and an end before the begin is refused ("An element cannot end before it begins."). A moment shows no empty End row: giving it an end is the menu's job.
- Row is a line editor too; a value that is not an integer is refused.
- Editing Begin moves only the begin; unlike a drag, it does not keep the duration.
- Every edit travels as a command onto the history and out to other views, never written by the panel directly (R10.5).

## Diagnostics

Problems reach the Errors and Warnings panel, not a canvas overlay (R12). All are **warnings**, and the rest of the diagram still draws; only an unparseable file is an **error**, reported as one problem with the same message the unavailable state shows. The specification's `constraints` state every one of them with the tool's own code and sentence, and the standalone module evaluates them (`TimelineValidator.cs`, `TimelineDefinition.cs`).

| Code | When | In timeline.dis | Located at |
| --- | --- | --- | --- |
| `timeline.unparseable` | The file is not YAML (error): "This is not YAML that can be read: " and YamlDotNet's message. Nothing else is reported. | `std.unparseable`, its reason YamlDotNet's message. | YamlDotNet's line. |
| `timeline.missing-id` | An element or relation has no id, or an empty one: "'Label' has no id, so nothing can select, connect or edit it." (or "A relation has no id, …"). | Rule `missingId`. | Its line. |
| `timeline.duplicate-id` | A declaration reuses an id an earlier one holds: "The id 'x' is declared more than once, which makes every reference to it ambiguous." The first keeps it. | `std.duplicateId`. | Its line. |
| `timeline.unreadable-time` | A begin or an end does not read as a time, the empty text included: "'Label' begins at '…', which is not a time this timeline can read." (or "ends at"). | Rules `unreadableBegin`, `unreadableEnd`. | The element. |
| `timeline.end-before-begin` | End lies before begin; only a hand edit can cause it. | Rule `endBeforeBegin`. | The element. |
| `timeline.mixed-precision` | One element mixes a date and a date-time. | Rule `mixedPrecision`. | The element. |
| `timeline.dangling-connection` | A relation end names no element: "A relation names 'x', and no element with that id is on this timeline." An end written empty, or not at all, is not reported. | Rules `danglingSource`, `danglingTarget`. | The relation. |

An element is named in a finding by its label in quotes, or by its id in quotes when the label is empty (the specification's function `displayName`). `std.references`, `std.endpoints` and `std.required` are turned off, because the rules above report what the tool reports and nothing more.

**The reader's findings the module forwards are one named set: `std.unparseable` alone** (`ReportedReaderFindings` in `TimelineDefinition.cs`). Every other finding the FBL reading gives is not forwarded:

| Reader finding | Why it is not forwarded |
| --- | --- |
| `std.missingId`, `std.duplicateId`, `fbl.dangling-reference` | The definition's own rules report the same cases under the tool's codes and sentences (`timeline.missing-id`, `timeline.duplicate-id`, `timeline.dangling-connection`). Under the module's binding a relation is an element rule, so `fbl.dangling-reference` is the library's note on a `from` or `to` that names no element, not a relation left unread. |
| `fbl.header-mismatch` | Finding B9: the tool reports nothing for a missing or another `timeline:` line. The module's binding declares no header, so it does not arise either. |
| `std.unreadableEntry` | Finding B8: a sequence entry that is not a mapping is passed over in silence, as the tool always passed it over. |
| `fbl.duplicate-key` | Finding B10: a key written twice in one mapping is refused before the binding reads the body, as YamlDotNet refuses it (`TimelineYamlSyntax`). |
| The rest (`fbl.unbound-key`, `fbl.unconverted-value` and the other notes of the library) | The tool reports none of them; a key the binding does not bind is kept as written. |

The last three are the findings FBL and DISL would add that the tool does not give; switching them on is a decision the owner has still to take (see [Pending with the owner](#pending-with-the-owner)).

**The order of the findings** is the tool's: first the ids (missing and duplicate, in document order, elements before relations), then the times per element (unreadable begin, unreadable end, end before begin, mixed precision), then the relations' ends per relation (source, then target). `constraints.order` cannot state a grouping, so the standalone module sorts what the evaluator reports. Its duplicate-id finding orders the holders by their line, where the tool orders elements before relations; the two differ only for a file whose `connections:` come before its `elements:` and that gives a relation an element's id.

Elements with problems stay on the canvas: an element whose begin cannot be read is drawn at the epoch as a moment, so it stays selectable, and a relation to a missing element is still sent so the canvas can show it (`TimelineElementMapper.cs`).

## Wire ids

The standalone client and backend share their own ids for the toolbox entries, the actions and the property rows. DISL 0.3 has no place for them, so the specification declares them under `x-timeline`, as `x-ghg` does for the hype cycle graph: a tool is keyed by its id, an action by its entry's operation or kind (with `"<Type>/<kind>"` overriding that for entries on that type, so `Connection/editLabel` is `timeline.relabel` and `Connection/delete` is `timeline.disconnect`), and a property by its form item's row id.

## What DISL 0.3 cannot express

- **The file format and its writing discipline** are not DISL's: `persistence.format: "fbl"` hands them to the module's binding, and [The document on disk](#the-document-on-disk) describes the `.tml` layout, the splice-only writes, the indentation copied from the file and the `connections:` key created with its first entry. Where FBL itself stops is in [Language gaps](#language-gaps).
- **Reading times.** CEL has no timestamps; the plugin functions above carry the tool's reading, and their fallbacks are an approximation.
- **Precision-dependent snapping.** The x snapping depends on whether an element's times carry a time of day; the specification declares the day snapping of every element the tool creates.
- **The connection route.** A forward loop when the target begins before the source ends is the tool's own route (`client/timelineBindings.ts`). The named begin and end anchors are the specification's fixed points, compiled as named points on the sides; that a period is drawn as the shared span and a moment as the library's marker is the client's.
- **The drag hint's text and the ruler's tick choice.** The snapping's `feedback.showValue` asks a drag to show the value it would rest on; that the value is the time under the left edge and the row under the top, computed from the live bounds over the frozen scale, is the client's (`client/timelineBindings.ts`), and the ruler is drawn beside the canvas.
- **The row arrangement.** A layout plugin, described in [Arrange diagram](#arrange-diagram).
- **A menu addition without a position** asks for the begin and takes its row from the element the menu was opened on; DISL's operation reads `position` and has no fallback for a menu opened without one.
- **The grouping of findings.** Described in [Diagnostics](#diagnostics).
- **The wire ids**, under `x-timeline`.

**What the standalone module runs from this specification, and what stays code.** The DISL runtime's operation interpreter runs `addAfter` and `addBelow` (a `create` and a `connect`) and `giveEnd` and `removeEnd` (a `retype` with its `set` or `unset`), and a relation released on empty canvas runs the `relate` tool's `createTarget` or `createSource`, the edge's `connect.from` saying which from the anchor the gesture started at. The transaction each produces is carried by the module's own command record, which the history stack and the wire carry (`backend/EtAlii.Adp.Diagram.Timeline/TimelineDefinition.cs`), and the command writes it as model changes through the binding (`Commands/TimelineEdits.cs`): `giveEnd` and `removeEnd` as one change of the `end` key ([gap 6](#6-fbl-a-retype-between-two-rules-that-match-one-entry)). Relating two existing elements and Remove are commands of their own. The client's canvas is compiled from this specification (`client/TimelineCanvas.tsx`), with what it cannot read in `client/timelineBindings.ts`.

## Language gaps

Each gap is a candidate change to FBL, found by the standalone conversion. None is worked around in the specification or the binding by stating something the tool does not do; where the standalone library does more than FBL's text says, the tool relies on it, and the gap is that another host may not do the same. Fixtures are those of the standalone module (`backend/EtAlii.Adp.Diagram.Timeline.Tests/Fixtures/`), and the transcript is its parity transcript.

| # | Gap | Construct | Evidence | Would resolve it | Kept in the standalone module by |
| --- | --- | --- | --- | --- | --- |
| 1 | **Closed by FBL 0.5.** A slot that reads a YAML scalar as written (B1) | FBL 4.1.5 written text, 5.2 `read: "text"` | `written-text.tml` (labels `yes`, `1.0`, `null`, `~`, an id `007`, a reference `7`); the conformance fixture `timeline-written-text` | Done: a string attribute, an id and a reference read their written text unasked, and `begin`, `end` and `row` ask for it | Nothing |
| 2 | Opening an empty flow collection (B4) | FBL 4.3 (a flow collection's only writable span is the whole), 6.1 and 6.2 (no splice opens one) | `findings/first-element.tml`, the tool's own new document; the transcript's first add | A sentence in 4.3 or 6.2 that inserting an entry into an empty flow sequence writes a block sequence holding it, and a conformance fixture adding a first entry to `elements: []` | Nothing: the standalone library does it |
| 3 | The gap after the dash of a new sequence item (B7) | FBL 6.2, *Indentation* ("Keys of a new mapping item align with the column after `- `") | `indentation.tml`, written `-   id:`; the transcript's adds | A sentence that a new item copies its previous sibling's spacing between the `-` and its first key, and a conformance fixture | Nothing: the standalone library does it |
| 4 | A relation whose end names no element (B2) | FBL 5.4 (a relation with a dangling end is not created) | `findings/dangling.tml`, `findings/combined.tml`, `written-text.tml`; three entries of the standalone `RealFiles/divergences.json` | A relation rule that reads an entry whose end does not resolve, keeping the end unset with the text it is written with, so a host reports it under its own code | The binding's detour: a relation is an element rule with reference attributes that the `typeMap` maps to its ends, and `writtenEnd` reads the end it keeps |
| 5 | An id held twice (B3) | FBL 5.3 (a stored id goes to one holder; a later one is addressed by its place) | `findings/identity.tml`; an entry of the standalone `divergences.json` | A reading that keeps a repeated stored id beside the place it falls back to, as a standard host attribute | The host attribute `storedId` on every rule, and `TimelineDefinition`'s `ElementOf` and `WrittenIdOf` |
| 6 | A retype between two rules that match one entry | FBL 6.4 (a `retype` is planned as a `remove` and an `add`) | `give-end.tml`, `findings/end-shapes.tml`, `findings/no-begin.tml`; the transcript's Give it an end and Remove its end steps | A sentence in 6.4 that where the two rules bind the same container and differ only by a `when` on one key, a retype is planned as the set or removal of that key, keeping the entry's place and its relations | `Commands/SetTimelineEndCommandHandler`, which plans one change of `end`; the standalone library writes it through the rule that matches the entry |
| 7 | The wording and the acceptance of an unreadable body | FBL 7.5 (the reason is the host's YAML reader's), 7.4 (a repeated key is read with `fbl.duplicate-key`) | `findings/unparseable.tml` ("While scanning a plain scalar value, found invalid mapping." at line 4), `findings/duplicate-key.tml` | A statement in FBL of the reason and the line a host gives for a body that is not YAML, or the owner's ruling that the timeline gives the library's reason and opens a file with a repeated key (Q2) | `TimelineYamlSyntax`, the stand-in |
| 8 | Removing the last entries of a body without a final newline | FBL 6.2, *Line endings* (an insertion after a last line without an ending keeps the body without one; a removal is not covered) | `no-trailing-newline.tml`; the transcript's removal of its last relations, which leaves a line ending the file never had | The same sentence for a removal: a removal that takes the last line of a body without a final newline takes the line ending before it, so the body still has none | Nothing: the tool writes the library's bytes |

### 1. FBL: a slot that reads a YAML scalar as written (closed)

The requirements' gap 1 and the reason the first half of the timeline's conversion stopped. FBL 0.5 (etalii.adp pull request 115) gives every YAML scalar a written text (section 4.1.5) and lets a slot ask for it with `"read": "text"` (section 5.2); a string attribute, an id and a reference read it unasked. The module's binding asks for it on `begin`, `end` and `row`, and `written-text.tml` reads as the tool read it.

### 2. FBL: opening an empty flow collection

A new timeline is `elements: []`, and the binding's `template` writes it. FBL 4.3 makes a flow collection one value whose only writable span is the whole collection, and the splice catalogue names no operation that opens one into block form. The standalone library does it all the same, replacing an empty flow sequence by a block sequence when an entry is inserted, so the first element of `findings/first-element.tml` is added as the tool adds it. **Would resolve it:** the sentence, and a conformance fixture, without which another host may refuse the first element of every new timeline.

### 3. FBL: the gap after the dash

`indentation.tml` is written `    -   id: …`. A new entry copies the indentation and the spaces after the `-` of the entries already there, and the standalone library does the same. FBL 6.2 says a new item is indented like its previous sibling and that its keys "align with the column after `- `", which does not say whether the spacing between the dash and the key is copied. **Would resolve it:** a sentence that it is, and a conformance fixture.

### 4 and 5. FBL: a relation with an unset end, and an id held twice

The timeline reports a relation whose end names no element (`timeline.dangling-connection`), keeps it and does not draw it; and it reports an id written twice (`timeline.duplicate-id`) and finds an element by the id it is written with. FBL 5.4 creates no relation with a dangling end, and FBL 5.3 gives a stored id to its first holder only. DISL 0.3 answers both from its side, but only when the binding reads a relation as an element rule with reference attributes and every rule carries the written id as a host attribute, as this binding and the hype cycle graph's do. **Would resolve it:** FBL 5.4 and 5.3 saying it once, so every binding of a hand-edited format does not need the same two detours.

### 6. FBL: a retype between two rules that match one entry

In the file, giving a moment an end and removing a period's end change one key; in DISL they are a `retype` (`giveEnd`, `removeEnd`). FBL 6.4 plans a retype as a `remove` and an `add` under the new rule, which would move the entry to the end of the list and take its relations with it through `remove.cascade`. The standalone module plans the change as a `set` of `end` instead, and the library writes a set of an attribute the entry's own rule does not bind through the other rule that matches the entry. **Would resolve it:** a sentence in 6.4 for rules over one container that differ only by a `when` on one key.

### 7. FBL: the wording and the acceptance of an unreadable body

FBL 7.5 leaves the reason of `std.unparseable`, and which bodies are refused, to the host's YAML reader. The timeline's sentence is YamlDotNet's ("This is not YAML that can be read: While scanning a plain scalar value, found invalid mapping." at line 4 of `findings/unparseable.tml`), and YamlDotNet refuses a mapping with a key written twice, where FBL 7.4 reads the body with the first key bound and `fbl.duplicate-key` (`findings/duplicate-key.tml`, finding B10). So the module asks YamlDotNet first, through `TimelineYamlSyntax`, which was ruled by the owner in chat on 2026-10-11 as a stand-in that goes when the Q2 follow-ups are ruled on. **Would resolve it:** either a statement in FBL of the reason and line a host reports, or the owner's ruling that the tool takes the library's.

### 8. FBL: removing the last entries of a body without a final newline

Found by the standalone conversion. `no-trailing-newline.tml` ends without a line ending, and its edit script removes the relations at its end; the standalone library leaves the body ending in a line ending it never had. FBL 6.2 says that an insertion after a last line without an ending keeps the body without a final newline, and says nothing of a removal. **Would resolve it:** the same sentence for a removal, and a conformance fixture; until then the standalone library's bytes are a candidate defect of the library as much as a gap of the text.

## Definition defects repaired

A definition defect is the specification or the binding saying something the tool does not do. Each was found by the standalone conversion and repaired, in `timeline.dis` here or in the module's binding:

| Finding | Was | Is |
| --- | --- | --- |
| A1 | `persistence` declared `format: "yaml"` with `files`, `encoding`, `indent`, `newline`, `ordering`, `omitDefaults` and `timestamps`, which DISL 11.2 forbids beside a binding | `format: "fbl"`, the module's binding by URL, `ids`, `view` and a `typeMap` |
| A2 | `writtenEnd` read the host attributes `writtenFrom` and `writtenTo` the module supplied | It reads the end the relation keeps, which the `typeMap` maps from `from` and `to`; it stays a function because `std.references` reports every unset end, and the tool does not report one written empty or not written |
| B2 | The example reads a connection as an FBL relation, which is not created when an end names nothing | An element rule `connection` with `from` and `to` as references `by: "storedId"`, mapped to the relation's ends |
| B3 | A later holder of an id lost its written id | `storedId`, read-only, on every rule |
| B8, B15 | A sequence entry that is not a mapping would be read: by the relation rule as a relation without an id, reported as `timeline.missing-id` | `entry.all(k, true)` in every rule's `when`, so such an entry is taken by no rule and kept as written |
| B9 | The example's `header` reports `fbl.header-mismatch` for a missing or another `timeline:` line | No `header`; the key is unbound and kept, and the template still writes it |
| B11 | `has(entry.end)` made a period of an entry whose `end` holds a mapping or a sequence | The `when` of the period and moment rules tests that `end` holds a scalar |
| B13 | A row was converted from its typed value: `1.0` row 1, `0x10` row 16, `0o17` row 15, `1e1` row 10 | `row` is read as text with `default: 0`, without `quoted` and `fallback`, and parsed as the tool parses it, so all four are row 0 |
| B14 | A missing `begin` left the attribute unset, which no rule reports | `default: ""`, which the tool reports as an unreadable time |
| B16 | `insert.place: "after-last"` put a new moment after the last moment, in the middle of the list, once the last entry was a period | `place: "end"` on both element rules |

Four findings that were known only by reading did not occur when their fixtures were read or edited through the example binding: B4 (`findings/first-element.tml`), B7 (`indentation.tml`), B12 (`dotnet-times.tml`: a move over a time written `2026/07/01` or `July 5, 2026` writes what the tool writes) and B15 (`findings/scalar-connection.tml`). B4 and B7 remain gaps of FBL's text (gaps 2 and 3); B12 needs nothing; B15 is held by the `when` above all the same.

## Other differences a runtime will meet

These are not language gaps. Each is a place the transcript shows the tool doing something a runtime of this specification might do otherwise, or the tool's behaviour changing with the conversion, recorded so that a host does not have to find it again.

- **What an edit writes changed with the conversion** (ruled by the owner, Q1: what FBL writes). A value's own span in its own style, a trailing comment and the author's quotes kept, a new key where the binding orders it, a removed entry's comment lines removed with it, a value set to what it holds writing nothing, and undo putting back the exact bytes. The standalone pull request 301 lists every kind of changed hunk.
- **A row written quoted is written plain** when it changes: `row: "0"` set to 3 becomes `row: 3` (`quoted-values.tml`, the standalone `Parity/timeline.findings.json`, finding B5). The binding reads `row` as text and the command gives it an integer, and the standalone library writes an integer given to a text slot plain. FBL 6.2 keeps a replaced value's style "when the new value can be written in it"; whether an integer can be written double-quoted into a slot read as text is not settled, so this is a candidate for FBL's text or a defect of the library. The tool wrote it plain before too.
- **Giving an end to an entry whose `end` holds a mapping** is refused with the library's sentence, "The value is a block collection, which a slot does not write." (`findings/end-shapes.tml`). Such an entry is a moment whose `end` key is kept as written. Before the conversion an earlier step of the same script had left that file unreadable, so the transcript recorded another refusal there (standalone pull request 301).
- **Times in a form only .NET reads** (`2026/07/01`, `July 5, 2026`) are read by the time plugin, and a move writes over them in the date form the tool writes (`dotnet-times.tml`, finding B12); the binding's `time: "keep-precision"` agrees.

## Pending with the owner

The Q2 follow-ups of `timeline-disl-fbl`: findings FBL and DISL would add that the tool does not give. Each was switched off in this conversion (Q2's default) and is to be raised as a small change of its own.

- **A missing or another header** (B9, `no-header.tml`, `other-version.tml`): `fbl.header-mismatch`, if the binding declares the example's `header`.
- **A sequence entry that is not a mapping** (B8, `findings/combined.tml`, `findings/scalar-connection.tml`): `std.unreadableEntry`, if the module forwards it.
- **A key written twice in one mapping** (B10, `findings/duplicate-key.tml`): the file opens with the first key bound and `fbl.duplicate-key`, where it is refused today.
- **The wording of `timeline.unparseable`** (gap 7): the library's reason and line in place of YamlDotNet's. Ruling this and the previous one removes `TimelineYamlSyntax` and the module's YamlDotNet reference.

## What stays code

Every class left in the standalone module, `src/diagrams/timeline/`, beside the gap or the host concern that keeps it. A class with neither was removed: the module's hand-written reading and writing of the body, and the code that built the DISL model from that reading, went to the test project's `Parity/` as the oracle the transcript was recorded against, or were deleted; the rule set that only forwarded the definition's findings went before them.

| Class | Kept by |
| --- | --- |
| `TimelineBody` | Host: the shared document lifecycle's body, opened through the embedded binding, with the DISL model built from it, and the edits planned on it, one batch for Arrange diagram. |
| `TimelineReading`, `_Model/TimelineModel`, `TimelineElement`, `TimelineConnection`, `TimelineInstant`, `TimelinePrecision` | Host: the binding's reading as the mapper, the session, the scale, the arrangement and the commands read it; a time is kept as its text and its precision. |
| `TimelineYamlSyntax` | Gap 7: the stand-in of the standalone Requirement 4.2, ruled by the owner in chat on 2026-10-11. It goes when the Q2 follow-ups are ruled on. |
| `TimelineTimes`, `TimelineInstants` | The plugin `net.etalii.adp.generic.timelineTimes`: DISL's CEL has no timestamps, and the tool reads every form .NET reads. |
| `TimelineScale` | Host: the only conversion between a time and x, and the precision a value written after a drag keeps. |
| `TimelineArrangement`, `TimelineRows`, `Commands/ArrangeTimelineCommand`, `SetTimelineRowsCommand` | The layout plugin `net.etalii.adp.generic.timelineRowArrange`, and Arrange diagram as one undo step; `TimelineRows` is the row height and the nearest-row snap. |
| `TimelineDefinition` | Host: the bundled specification loaded with its plugins through the shared `DislDiagramDefinition`, which maps the derivations onto the host's types and the `x-timeline` wire ids; the one named set of reader findings forwarded, and the grouping of findings, which `constraints.order` cannot state. |
| `TimelineRules` | Host: the tool's finding codes as constants, which the grouping of findings and the choice between reporting at a line and at an element name. The codes themselves are the specification's. |
| `TimelineValidator` | Host: the specification's findings as the host's problems, and today's one finding for a body that is not YAML. |
| `TimelineContextActionProvider`, `TimelineContextPropertyProvider`, and the shared `DislToolboxProvider` | Host: handing out the derived toolbox, menus and rows, and dispatching a chosen entry, a gesture or an edited row as the module's command on the history stack. |
| `TimelineContextSourceResolver` | Host: resolving a selection, a relation included whose end names nothing, to an element of the document. |
| `TimelineElementMapper` | Host: the `.proto` payloads and the viewport filter; a relation is sent only when both its ends are. |
| `TimelineSession`, `TimelineDocumentStore`, `ITimelineDocumentStore` | Host: the shared document lifecycle and its reload. |
| `Commands/*Command` and their handlers, `Commands/TimelineEditHandler`, `Commands/TimelineEdits` | Host: the command records the history stack and the wire carry; each handler is its checks, with the tool's refusal sentences, and the model change it plans, answered with the shared restore. `SetTimelineEndCommandHandler` is gap 6 as well. |
| `Diagram`, `ServiceCollection.AddTimeline` | Host: the tool's registration, its services and its empty document. |
| `client/TimelineCanvas.tsx`, `client/timelineBindings.ts` | Host: the canvas compiled from this specification, with what the library cannot read from it (precision-dependent snapping, the forward loop of a relation, the drag hint). |
| `client/TimelineRuler.tsx`, `client/timelineTicks.ts`, `client/timeline.css` | Host: the time ruler beside the canvas, its tick choice and the moment's fill. |
| `client/timelineModel.ts`, `client/useTimelineStream.ts`, `client/register.ts` | Host: decoding the payloads, the stream and the module's registration. |

## Scale and performance

- The client asks the backend only for what is in view; a period is culled on its whole span, not its begin, and a relation is sent only when both its ends are (`TimelineElementMapper.Visible`).
- Scrolling and zooming never round-trip to the backend (NFR performance).

## Known gaps

- Several requirements stated in the original specification are not visible in the code: choosing *Relate* from a menu and then clicking a target (R11.6) and adorners withheld in read-only mode (R7.8) were not verified here.
- There is no date editor in the property grid (R10.6).
- The only real `.tml` documents are the examples under `examples/`; the fixtures were written by the module's author, which the fixtures' own readme names as a weakness.
- A defect of the standalone module found while freezing its behaviour for this specification is left as it is: a document that YamlDotNet rejects with an `InvalidOperationException` rather than a `YamlException` (for example `label: [unclosed` at the end of the file) is not reported as `timeline.unparseable` but fails the validation (`TimelineYamlSyntax` catches only the latter). A second, that undoing a run of edits did not always return the file byte for byte, is repaired: an edit is undone by putting back the text before it, and the standalone round-trip tests undo every edit of the transcript's script and compare bytes (`Parity/TimelineRoundTrip.Tests.cs`).
