# Gartner hype cycle graph: what DISL does not express

This document accompanies [gartner-hype-cycle-graph.dis](gartner-hype-cycle-graph.dis), the DISL 0.3 specification of the Gartner hype cycle graph diagram. The specification holds everything DISL 0.3 can state: the metamodel, the `yearMonth` time axis and its ruler, snapping per gesture, the phased banner with its boundary handles, influence ends bound to phase parts, the tag filter, the compact toggle, the menus, the forms, the constraints with the tool's own codes and sentences, and the FBL binding the document is read and written through. What DISL 0.3 cannot state, the wire ids, is declared in the specification under an `x-` key and described here in [What DISL 0.3 cannot express](#what-disl-03-cannot-express). This document also holds the background and the rest of the detail, each point tied to the code, specification or ruling that shows it.

The tool it describes is the standalone module `src/diagrams/gartner-hype-cycle-graph/` in [etalii-adp/etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/gartner-hype-cycle-graph), read on branch `research/hype-cycle-fbl-qa6b70` at commit `466cc56b`. It is also implemented in the Visual Studio Code plug-in, [etalii-adp/etalii.adp.ide.vscode](https://github.com/etalii-adp/etalii.adp.ide.vscode), against this definition; where that host differs is recorded in its `docs/parity.md`. Its tool type origin is `gartner/hypecycle-graph` (`language.origin`) and its documents use the extension `.ghg`. Unless a path says otherwise, it is relative to that module: `backend/` is `backend/EtAlii.Adp.Diagram.GartnerHypeCycleGraph/`, and `client/` is the module's client folder.

## Contents

1. [Sources](#sources)
2. [The document](#the-document)
3. [Time](#time)
4. [Phases](#phases)
5. [Influences](#influences)
6. [Triggers](#triggers)
7. [Notes](#notes)
8. [Canvas chrome: ruler, tag filter, legend](#canvas-chrome-ruler-tag-filter-legend)
9. [Compact mode](#compact-mode)
10. [Toolbox, context actions and the property grid](#toolbox-context-actions-and-the-property-grid)
    - [Arrange diagram](#arrange-diagram)
11. [Validation rules](#validation-rules)
12. [Refusals and confirmations](#refusals-and-confirmations)
13. [What DISL 0.3 cannot express](#what-disl-03-cannot-express)
14. [Fixed on the research branch](#fixed-on-the-research-branch)
15. [Rulings from the owner](#rulings-from-the-owner)
16. [The Notion row](#the-notion-row)

## Sources

- The module's code, as named per point below.
- The module's FBL binding, `backend/gartner-hype-cycle-graph.fbl`.
- The three requirements documents in the standalone repository: `.spec-workflow/specs/gartner-hype-cycle-graph/requirements.md`, `.spec-workflow/specs/ghg-compact-mode/requirements.md` and `.spec-workflow/specs/ghg-triggers-and-notes/requirements.md`.
- The catalog row in the standalone repository's `docs/tools.md`.
- The Notion "Tools" database row for the Gartner Hype Cycle (see [The Notion row](#the-notion-row)).
- The owner's rulings in the project's conversations while the module was built (see [Rulings from the owner](#rulings-from-the-owner)).

## The document

The `.ghg` document is a hand-editable YAML file. The specification declares `persistence.format` `fbl` with `binding` naming the module's own FBL binding, `gartner-hype-cycle-graph.fbl#ghg`, which states the file's shape and writing discipline: the header, the four lists, kebab-case keys, key order, flow tags, a literal block for a note's text, two decimals for an `at`, a width and a height, CRLF line endings, and new `triggers:` and `notes:` lists opened before `influences:` (`backend/gartner-hype-cycle-graph.fbl`). Writing splices only the lines an edit changes, so an unchanged document stays byte-identical, and comments, blank lines and unknown keys survive. The keys DISL 11.2 forbids beside an FBL binding (`encoding`, `mediaType`, `files`, `ordering`, `omitDefaults` and the rest) are therefore gone from the specification.

**Shape.**

- The first key is the header `gartner-hypecycle-graph: 1`. The `1` is the document version; this specification's `language.version` `1.1.0` corresponds to it.
- An optional top-level `unit:` follows, one of `month`, `year`, `decade` or `century`; absent means `month`, and an unknown value is reported and drawn in months. It is the DISL diagram attribute `unit`.
- Then up to four lists, always in this order: `trends:`, `triggers:`, `notes:`, `influences:`. Adding to a list the document cannot have is refused: "The document has no `influences:` section to add to."
- Keys are kebab-case in the file and camelCase in the specification: `peak-end` is `peakEnd`, `from-phase` is `fromPhase`, and so on, as the binding maps them.
- A trend's `row` and `phases` are written even at their defaults when the trend is added.

**The binding's types are not the metamodel's.** The binding reads an influence as an element with the reference attributes `from` and `to` rather than as a relation, because an FBL relation whose end names nothing is not created at all (FBL 5.4) while the module keeps such an influence and reports it as `ghg.dangling-reference`. It also reads the header and the unit as elements of their own (`Graph`, `Unit`), entries that are not mappings as `Unreadable`, and gives every entry the host attributes `storedId` and `unknownKeys`. The specification maps these onto its metamodel with DISL 0.3's `persistence.typeMap`, under which an influence whose end names nothing is kept, not drawn, and reported by `std.references`.

**Reading never fails.** An unknown key, a malformed value or an entry that cannot be read becomes a finding, `ghg.unreadable-entry`, with a line number, and the rest of the document is still drawn (`backend/GhgParser.cs`). The reasons are the module's own sentences, such as "A trend has no `start` date; it cannot be drawn." or "`tags` is not a list of tags; the line is kept."; keys `from-phase`, `from-edge` and `from-at` on an influence that comes from a trigger are kept and reported as ignored. A body that cannot be read at all opens as an empty, read-only diagram; the specification's `behavior.messages` gives the read-only sentence, "The graph could not be read, so it cannot be edited."

**Ids.** New entries get a ShortGuid, a version 4 GUID written as 25 characters of base 36 (`ids.encoding: "base36"`; `ShortGuid.cs` in the standalone backend). Hand-written ids such as `steam-engine` are kept. Ids are unique across all four lists together; adding an entry under an id already in use is refused: "That id is already used in this graph."

**The project entry.** In the standalone tool each diagram sits beside a `.adp` entry file that names the tool type origin `gartner/hypecycle-graph` and the `body:` document (for example `examples/coal-technologies/coal-technologies.adp`). That entry belongs to the IDE host, not to this diagram type.

## Time

The time axis is DISL 0.2's `yearMonth` axis: a month index counted from the origin 1900-01, four canvas units per step of the diagram's `unit` (`scale.unit` bound to the attribute), deliberately linear (`backend/GhgScale.cs`). Dates are signed astronomical years, `0000` being 1 BCE and `-3200-01` being 3201 BCE, with four digits for a year from 0 and four to six after a minus sign; the `eras-of-innovation` example uses such dates. Rows are 56 canvas units apart, a 32-unit trend plus a 24-unit gutter.

**Snapping** (`snapping` on the coordinate system and on each node notation):

- A move or resize snaps the placement anchor to the nearest step start and the nearest row, halves away from zero on both sides of the origin. A trigger's anchor is its centre, which sits on a row's middle, 16 of its 56 units down.
- A drop from the toolbox, or an "Add … here" on empty canvas, lands in the step that contains the drop point. The row differs per type: a trend or trigger takes the row its middle is nearest to (`RowAtMiddle(y) = round((y - 16) / 56)`), a note's top the row that contains the drop point (`floor(y / 56)`). DISL 0.2 states this with `snapping.byGesture.drop` on each node notation.

**Label formats follow the unit.** A trigger's date reads `Dec 1947` in a diagram of months and the year alone (`1947`, or `-3200`) in one of years or coarser (`GhgScale.FormatWhen`); the tooltip uses the full month name (`FormatWhenLong`). The specification's `formatWhen` function mirrors this.

## Phases

The phase boundary rule is in the specification (`phaseBoundaries`), checked against `GhgPhases.BoundariesOf` over 200,000 random trends with no difference. A drawn set of boundaries that is not strictly increasing inside the banner is drawn evenly spread instead (`phaseFractions`).

### Dragging a phase boundary

On a selected trend in true time, each drawn inner boundary is a handle (the shape's `handles`). Dragging it snaps to the step grid counted from 1900-01, halves away from zero, and is kept at least one step from its neighbouring drawn boundaries or the trend's ends, or put halfway between them when there is no room (`boundaryLanding`, mirroring the standalone client's `canvas/library/shapes/segments.ts`). The landed position is written as the month of the nearest step to the boundary's model attribute, `peakEnd`, `troughEnd` or `slopeEnd` (`Commands/SetGhgBoundaryCommandHandler.cs`). Only a boundary between two visible phases can be moved; any other is refused with "Only a boundary between two visible phases can be moved." While a boundary, or a trend's edge, is dragged, the property grid previews the Start, Stop and boundary values it would write (`client/GhgCanvas.tsx`, the property preview); DISL has no such preview and the specification does not state it.

### Keeping every phase at least a month long

Setting a boundary, from a handle or from the grid, pins it and clamps every stored boundary so each visible phase, including those spread evenly around it, stays at least one month long; rescaling boundaries after a resize ends with the same clamp. A stored boundary beyond the last visible phase is only kept inside the span (`KeptAMonthApart` in `backend/GhgPhases.cs`). The specification's `rescaleBoundaries` hook does the proportional rescale (`newStart + round((b - oldStart) * newSpan / oldSpan)`, which is an exact shift on a move); the clamp is an iterative pass over neighbouring anchors that one declarative `set` per attribute cannot express, and is declared as DISL 0.3's `bounds.neighbour` on the `Trend` type.

### Other phase behaviour

- **A changed phase count keeps the stored boundaries** (`GhgWriter.SetPhases`, Requirement 3.4). A boundary beyond the new last visible phase stays in the file, is not drawn and is not a neighbour for spreading.
- **A span shorter than its phases is refused**: a trend showing N phases must be at least N months long. The specification states this as change and placement constraints; the exact refusal text is in [Refusals and confirmations](#refusals-and-confirmations).
- **Per-segment tooltips.** Hovering a phase shows its full Gartner name: "Peak of Inflated Expectations", "Trough of Disillusionment", "Slope of Enlightenment", "Plateau of Productivity" (`GhgPhases.GartnerNames`, Requirement 4.4). The specification declares each on its part, as DISL 0.3's part `tooltip`. The trend itself has no tooltip of its own.
- **Phase titles** in the grid and in influence lists are Peak, Trough, Slope and Plateau (the `Phase` enum labels).
- **Even phases** forgets all three stored boundaries. It is listed only while one is stored; asked for otherwise it is refused with "This trend's phases are already even."

## Influences

An influence end is a phase, an edge and a fraction: it attaches anywhere along the top or bottom edge of one phase segment, at `at` from 0 to 1 along that segment's stretch of the edge. The specification states this with DISL 0.2's `anchoring.mode: "part"`, binding `part`, `side` and `at` to the relation's attributes; the parts `peak` to `plateau` of the phased banner are each segment's stretch, from its left chevron tails to its right ones. Because `at` is a fraction of the segment, a resize or a boundary drag moves the end proportionally. No anchor dots are drawn on trends; the whole edge is the handle.

- **The connect gesture records both ends** as one undo step (`backend/GhgGestures.cs`, `client/GhgCanvas.tsx` `gestureEnd`); a trigger's end carries nothing. The specification's `connection` menu runs its single "Influence" entry.
- **Moving an end.** A selected influence shows a handle on each end (`movable: true`), which slides along the trend's edge and across into other phases, writing `fromPhase`, `fromEdge` and `fromAt` (or the `to` attributes).
- **Default ends.** A gesture that carries no end leaves from the source's last visible phase, bottom edge, 0.5, and arrives at the target's Peak, top edge, 0.5 (`Commands/AddGhgInfluenceCommandHandler.cs`), the anchoring's `default`.
- **Drawing.** A cubic Bézier with a filled arrowhead at the target (`arrowFilled`, the canvas library's `arrow`), leaving and entering perpendicular to the edge it attaches to, so it meets the border at 90°, in the host's muted text colour (`ghg.influence`), 1.5 wide.
- **Hidden, not removed.** An influence attached to a phase its trend does not show is hidden, stays in the document and still counts for the one-per-direction rule (Requirement 7.3). With part anchoring, an end on a part that is not drawn hides the edge.
- **One per direction.** A to B and B to A may both exist; a second A to B is refused. The duplicate check reads the document, not what is drawn, so a hidden influence still blocks a new one.
- **An end on an unknown phase or edge** is drawn to the whole edge of the trend today (`backend/GhgElementMapper.cs`, `attachmentPointOf` in `segments.ts`); with `Phase` and `Edge` as enumerations, such a value is reported by `std.facets` and the specification states nothing about where the end is then drawn.

## Triggers

(`client/GhgCanvas.tsx` `TRIGGER_TYPE`; `.spec-workflow/specs/ghg-triggers-and-notes/requirements.md`)

- A circle 16 across, filled and outlined 1.5 wide in the trigger colour, centred on its step's start and its row's middle. Never resized.
- Its label is written before it, 8 units away, as `{name} · {when}`. Only the name is edited in place: the inline editor opens on the name alone (the label's `editText`), and the edited text is trimmed and written as the name.
- The tooltip reads `Trigger: {name}, {December 1947}`.
- An influence may start at a trigger but never end at one. It leaves from one of three invisible handles, top, right and bottom, and the line leaves the outline facing its target wherever the gesture began (`anchors: { kind: "compass", positions: ["n", "e", "s"], attachDrawnBy: "edge", visible: false }`; `drawnFrom: "outline"` in the specification). The document stores nothing for that end.
- Removing a trigger removes the influences from it, after the confirmation below.

## Notes

(`client/GhgCanvas.tsx` `NOTE_TYPE`; `Commands/SetGhgNoteSizeCommand*.cs`)

- A box outlined 1.5 wide in the border colour, with the author's text, 12 units high, word-wrapped and centred in the box less 6 on every side, edited in place over several lines; overflowing text ends in an ellipsis. A note has no tooltip.
- Dropped from the toolbox it is 160 by 64, its top-left at the step start and row of the drop, and its editor opens at once (`after: "editLabel"`).
- It moves and resizes in both directions. A resize sends one command, `W x H at YYYY-MM row N`, so size and position change in one undo step. In the grid, Size reads `160 x 64` and accepts the same form; anything else is refused with "'…' is not a size; write it as width x height, such as 160 x 64." The grid field is declared with `parse`.
- `width` and `height` are canvas units, stored with at most two decimals, and are not snapped.
- No anchors, no tags, no influences, and it stays visible under every tag filter.
- A note whose position cannot be read cannot be resized: "This note's position cannot be read, so it cannot be resized until it is fixed in the file."

## Canvas chrome: ruler, tag filter, legend

**Ruler.** A horizontal ruler pinned to the bottom of the view, not to the canvas (`attach: "view"`). Its levels, finest first, are month (`MMM yyyy`), quarter (`MMM yyyy`), year, decade, century and millennium (`yyyy`), each shown only while its labels are at least 64 pixels apart (`client/GhgCanvas.tsx`, `RULER_RUNGS`). The specification's formats use `uuuu`, the signed year, because DISL rejects `y` on a `yearMonth` axis. A diagram drawn in a coarser unit keeps only the levels at least one of its steps wide, at any zoom; that floor is the ruler's `minUnit`, bound to the diagram's `unit`.

**Tag filter.** A "Filter by tags" box with tag chips, a lookup of the tags in use and an Any/All switch; matching is case-insensitive (`canvas.filters.tags`). It applies to trends and triggers only. Elements that do not match are hidden, not dimmed, together with every influence to or from them; notes always stay. The filter is viewer state only and is never saved. An earlier and/or expression filter was replaced by the chips at the owner's request.

**Legend.** A key to the phase colours sits under the filter box, one swatch per phase painted by the same rule that paints the phase, so it follows dark mode (`legend` under `filter`, `client/ghg.css`). The specification's legend names the `Phase` enum at the filter's position; each enum value's `color` is the theme token its swatch is painted with (`{ "token": "ghg.peak" }`), which DISL 0.3 resolves in the current theme mode.

**Theme colours** are the specification's `ghg.*` theme tokens, taken from the standalone client's `src/client/src/index.css`, which `client/ghg.css` applies to the phases, triggers and notes; `ghg.border` and `ghg.influence` are the host's border and muted text colours.

## Compact mode

The specification's `compact` viewpoint is a variant of true time with a "Compact" toggle under the legend (`variantOf`, `toggle`). It is viewer state, never saved: every diagram opens in true time. Trends are 24 canvas units per visible phase (96 for all four, twice a new true-time trend) with even phases; nothing moves or resizes, no boundary is dragged, and the coordinate system `compact` has no ruler because a compact x is no date. What DISL cannot state (`client/GhgCanvas.tsx`, `layout`; `.spec-workflow/specs/ghg-compact-mode/requirements.md`):

- **The layout**, declared as DISL 0.3's standard algorithm `rowPacked`. Every element keeps its stored row; within a row, elements keep the order of their start dates and are placed as far left as they fit, 4 canvas units apart. An influence's target starts after the middle of its source, so causes read to the left of their effects. Only trends take the compact width; triggers keep their circle and notes their box, and a note two rows tall keeps both rows clear.
- **The whole document.** Compact places every trend by where all the others start, so while it is on the view reported to the backend is the whole canvas rather than the part on screen.
- **What still works.** Influences (drawn, added and removed), renaming, the property grid, undo and the tag filter.
- **No background menu**, so "Add … here" and Arrange on empty canvas are not offered (the `diagram` menu's `when`); Arrange is reached from an element.
- **Drops.** A toolbox drop in compact still needs a date; it is approximated by running the placement backwards from the drop point. The specification does not state this.

## Toolbox, context actions and the property grid

**Toolbox** (`backend/GhgToolboxProvider.cs`): three items, dragged onto the canvas (`mode: "drop"`), with the code's icons and descriptions and no group heading. A dropped trend is 12 steps of the unit long with all four phases; the default names "New trend" and "Trigger" are numbered "New trend 2", "New trend 3" when taken (`UniqueName.Of` in `EtAlii.Adp.History`), which the specification's `uniqueName` function mirrors. Influences are drawn, not dropped, so they have no toolbox entry.

**Context actions** (`backend/GhgContextActionProvider.cs`), with their icons in the specification:

- Trend: Rename… (F2), Even phases (only while a boundary is stored), Remove (Delete).
- Trigger: Rename… (F2), Remove (Delete).
- Note: Edit text… (F2), Remove (Delete).
- Influence: Remove influence (Delete).
- Every menu, and empty canvas: Arrange diagram (see [Arrange diagram](#arrange-diagram)). The tool shows it as a group of its own after the element's entries; the specification lists it last.
- Empty canvas in true time: Add trend here, Add trigger here, Add note here, each adding at the point that was clicked as a toolbox drop there would.
- Activating an element (double-click) also renames it.
- A read-only diagram offers no menu at all: every entry is hidden, not disabled (`visible: "!env.readOnly"`).

### Arrange diagram

(`Commands/ArrangeGhgCommandHandler.cs`, `GhgArrangement.cs`, and `RowPacking.cs` in the standalone backend's `EtAlii.Adp.Documents`)

"Arrange diagram" puts every trend, trigger and note on the row that leaves the graph least cluttered. **Across is the data; only the row is the tool's to choose**: an element's horizontal extent is its dates, so an arrangement changes nothing but `row` keys, and never a date. The specification declares it as the operation `arrange`, whose `layout` action runs the layout `rowArrange`, declared as DISL 0.3's standard algorithm `rows`, which writes `row` and nothing else; its refusals are on the `layout` action. That a row touched by a tall note stays where it is is part of `rows` too.

- **What takes part.** Every element that can be drawn: a trend with a readable span, a trigger with a date, a note with a position and a size, each id once. An influence is a link between the two elements it names.
- **An element's extent** includes the label written before it. A trend runs from its start, less 8 and the width of its name, to its stop. A trigger runs from its centre, less 8 (half its size), less 8 again and the width of `{name} · {when}`, to its centre plus 8. A note is its own box, and is as many rows tall as its height divided by the row step of 56, rounded up. A text's width is the hosts' shared estimate, the notation's `textMetric`: characters (UTF-16 units) times the font size, 12, times 0.55.
- **Fewest rows, while every element is one row tall.** Elements are taken from left to right (by left edge, then right edge, then document order). Each goes on a row that is free where it starts, keeping 16 clear of what is before it on that row; a row is opened only when every row is busy there, at which point that many elements overlap one point and no arrangement could use fewer. A note taller than a row needs every row it covers free, and may open rows that a different order would have saved.
- **Linked elements close together.** Among the rows that are free, an element takes the one nearest the average row of the elements it is linked to that are already placed, the lower row on a tie; with none placed, the first free row. A chain of linked elements that do not overlap therefore runs along one row.
- **Then whole rows swap** with their neighbour while that shortens the links in total, measured in rows, for at most 64 passes. A row touched by a note taller than one row stays where it is.
- **Deterministic.** Ties fall to the lower row and the original order, so arranging an arranged graph changes nothing.
- **Written as rows.** Only elements whose row changed are written, each as its one `row` line, bottom-up, in one undo step.
- **Refusals.** "There is nothing to arrange until this graph has a trend." when nothing can be drawn; the action is shown disabled with the same sentence for a graph with no trend, trigger or note (`unavailable`). "This graph is already arranged." when no row would change.

**Property grid** (`backend/GhgContextPropertyProvider.cs`): the specification's forms follow it, group by group. Details stated with DISL 0.3's `display` and `parse` on form items:

- The Phases slider's four stops are labelled "Peak", "Peak and Trough", "Peak, Trough and Slope" and "All four" (`widgetOptions.marks`); the row shows and accepts that label, refusing anything else with "'…' is not one of Peak, Peak and Trough, Peak, Trough and Slope, All four."
- "Peak ends", "Trough ends" and "Slope ends", one per drawn boundary, show the drawn month, which may be spread rather than stored, and accept a month that sets the boundary as a drag would.
- Size is one field, `W x H`.
- "From attachment" and "To attachment" are one text field each, written `phase/edge/at` such as `plateau/bottom/0.3`, shown for every influence; anything else is refused with "'…' is not an attachment; write it as phase/edge/at, such as plateau/bottom/0.3."
- The per-phase groups' "Influence" and "Influenced by" lists are read-only text, one `Name · Phase` per line, or a trigger's name alone, or "None"; an empty name shows the id. A hidden phase's group is titled `{Phase} (hidden)` and is shown only while an influence attaches to it. Their read-only reason is "Draw, reattach or delete an influence on the canvas."; that of an influence's From and To is "Where it is attached; drag the end on the canvas to move it."
- Tag suggestions are the tags in use on trends, then triggers.
- Descriptions are shown only in the grid; they are never drawn.

## Validation rules

All twelve rules report as warnings in the Errors and Warnings panel, each with the line of the entry it concerns (`backend/GhgValidator.cs`, which evaluates this specification's constraints; the hand-written rule set they replaced is kept as the tests' oracle, `Parity/GhgRuleSet.cs`). Saving is never blocked. Every finding carries the tool's code, and the messages are the tool's sentences. The findings are listed in the order of `constraints.order`, by code, and within a code by line. How each rule is carried in the specification:

| Rule | Where it is in the specification |
|---|---|
| `ghg.duplicate-influence` | `allowParallel: false` on `Influence` (built-in `std.endpoints`, code per case in its computed `code`, one finding per pair, at the second influence, by `oncePerGroup`). |
| `ghg.self-influence` | `allowSelfLoops: false` on `Influence` (`std.endpoints`). |
| `ghg.influence-into-trigger` | `target: "Trend"` on `Influence` (`std.endpoints`). |
| `ghg.stop-before-start` | Constraint `stopAfterStart`. |
| `ghg.phase-count` | Constraint `phaseCount`. |
| `ghg.boundary-order` | Constraint `boundaryOrder`: stored boundaries strictly inside the span and strictly in order, naming the first that is not. |
| `ghg.bad-attachment` | Constraints `fromAttachment` and `toAttachment`, for every end not at a trigger. |
| `ghg.dangling-reference` | `std.references`, with the code. |
| `ghg.duplicate-id` | `std.duplicateId`, with the code, reported once, at the last entry declaring the id (`oncePerGroup`). |
| `ghg.unreadable-entry` | `std.unreadableEntry` and `std.unparseable` with the code. A trend without a readable `start` or `stop` is the reader's finding, as an unknown key is: a constraint over the model cannot tell a missing date from a malformed one, which the reader also reads as absent. |
| `ghg.trigger-date` | Constraint `triggerDate`. |
| `ghg.note-position` | Constraint `notePosition`. |

## Refusals and confirmations

The backend refuses what the canvas would not offer anyway, because a request is never trusted to come from this canvas. The specification states each refusal as a constraint, a form validation, a handle refusal, an `unavailable` reason or a built-in's `refusal`. The sentences, verbatim:

- "A trend needs a name." and "A trigger needs a name." (a note's text may be emptied)
- "This trend's dates cannot be read, so it cannot be changed until they are fixed in the file."
- "A trend must stop after it starts, at least one month later." and "A trend must be at least one month long."
- "A trend showing N phases must be at least N months long, one per phase." (`Commands/GhgRefusals.cs`)
- "A trend shows 1 to 4 phases."
- "This note's position cannot be read, so it cannot be resized until it is fixed in the file." and "A note needs a width and a height."
- "'…' is not a date; write it as YYYY-MM, such as 2007-06." (for a trigger, "such as 1947-12.")
- "Only a boundary between two visible phases can be moved."
- "This trend's phases are already even."
- "An influence cannot end at a trigger.", "An influence is drawn from one trend to another.", "A trend cannot influence itself.", "This trend already influences that one; a trend influences another once in each direction."
- "An influence has a from end and a to end." and "An influence attaches to a phase, on its top or bottom edge, at a fraction from 0 to 1."
- "That id is already used in this graph." and "That is no longer in this graph."

**Removing with influences.** Removing a trend or trigger that has influences asks first, in a danger-styled confirmation titled "Remove" with the button "Remove": "Removing this trend also removes the 1 influence to or from it." or "… the N influences to or from it." With no influences it removes at once; a note never asks. The specification states this with a `Confirmation` whose `count` is the influences, a trend's influence on itself counted once, and whose `threshold` is 1.

## What DISL 0.3 cannot express

One item is declared in the specification under an `x-` key: the wire ids, which stay the host's own (Peter, 2026-10-06, "Spec all but wire ids"). A runtime that does not know the key ignores it and falls back as the last column says.

| Key | On | What it states | Without it |
|---|---|---|---|
| `x-ghg` | the specification | Today's wire ids of the toolbox items and their drops, the context actions and the property rows (`client/ghgIds.ts`), as `x-abm` states the behavior model's. An action is keyed by its operation or kind, and `"Influence/delete": "ghg.disconnect"` overrides that for one type. A property row is keyed by its form item's row id. | Ids are the specification's own names, which the client does not know. |

The other keys this specification once declared under `x-` are standard DISL 0.3 and renamed. Those DISL 0.3 adopted on 2026-10-05: `bounds.neighbour`, the ruler's `minUnit`, a part's `tooltip`, a label's `editText`, the anchors' `drawnFrom`, an enum value's `color` as a token, a form item's `display` and `parse`, a built-in's computed `code`, the layout algorithms `rowPacked` and `rows` (with `rows` pinning the rows a tall note spans, once `x-layout.rowArrange.pinRowsOfTallNotes`), the `layout` action's `refusals` and `persistence.typeMap`. Those Peter's ruling of 2026-10-06 made standard:

| Key | Was | On | What it states |
|---|---|---|---|
| `id` | `x-field.id` | the per-phase Influence and Influenced by items | The item's row id (DISL 7.5), so `x-ghg.properties` gives each of the eight lists its own row id rather than the ids of their two labels. |
| `group` | `x-menu.group` | each context-menu entry | The group the entry is shown in (DISL 7.3): consecutive entries with one name form one group, so Arrange diagram stands apart from an element's edits and from the canvas's adds. |
| `oncePerGroup` | `x-builtIn.oncePerGroup` | `std.endpoints`, `std.duplicateId` | Which member of a group of parallel relations or of entries sharing an id is reported (DISL 8.1): `second` reports the group once, at its second member; `last` once, at its last. Without it three influences in one direction would be two findings. |
| `order` | `x-order` | `constraints` | The order findings are listed in (DISL 8.6): by code, in the order given, then by line, a pair of influences or a shared id by its first member. |

Three built-in findings read detail keys that DISL 0.2 does not list, with a fallback so the specification stays correct without them; DISL 0.3 lists all three: `std.duplicateId`'s `count` (the message says how many times an id is declared), `std.references`' `end` (the message says whether the `from` or the `to` names nothing) and, since 2026-10-06, `std.endpoints`' `violation` (`target`, `selfLoop` or `parallel`: which of the relation's limits the finding is about, so a duplicated influence into a trigger or into itself is told from a third one in the same direction).

One thing has no key, because no construct could carry it: **the written id of an influence end that names nothing.** DISL hands CEL a dangling end as null (4.9), and the binding cannot map `to` to both the end and an attribute, so `endText` cannot write the end as the property grid does (`x · Peak`, `GhgContextPropertyProvider.Describe`). A runtime shows such a list or From or To row empty; the standalone host writes it from the id the binding read.

The per-type drop rows that once needed a plugin are standard DISL 0.2: snapping declared on a node notation, with `byGesture.drop`, is more specific than the coordinate system's.

## Fixed on the research branch

Three places where the code did not do what this specification states are fixed on the standalone branch `research/hype-cycle-fbl-qa6b70`; the specification states the fixed behaviour:

- **The time unit comes from the diagram's `unit`.** The canvas took it from the first trend's payload (`client/GhgCanvas.tsx`), so a diagram in years with only triggers and notes was drawn in months.
- **A later entry that reuses an id is drawn**, with an ephemeral id, and reported as `ghg.duplicate-id`, as DISL 11.5.4 requires. It was not drawn (`backend/GhgElementMapper.cs`).
- **A date with month 13 is rejected.** The 0.1 specification's `YearMonth` pattern accepted `2020-13`, which `GhgScale.ParseMonth` rejects; the `yearMonth` type rejects it too.

## Rulings from the owner

Decisions Peter made in the project's conversations while the module was built, which the code follows:

- The extension is `.ghg` (2026-09-26).
- Dates before year 1 are written as signed ISO (astronomical) years (2026-09-27).
- The time axis stays linear, never logarithmic (2026-09-27).
- Phase colours: Peak yellow, Trough light grey, Slope orange, Plateau lime green.
- Phases are spread evenly until a boundary is dragged.
- One influence per direction.
- Filtered-out elements are hidden, not dimmed.
- Influence arrows meet the border at 90°.
- Anchor handles on trends are shown only when selected; triggers get handles top, right and bottom but no visible anchor dots (2026-09-27, cited in `client/GhgCanvas.tsx`).
- The phase legend sits under the filter box.
- The property grid lists each phase's influences.
- The filter is tag chips with an Any/All switch.
- Compact trends are twice as wide as a new true-time trend, a share per phase, and causes read left of their effects (2026-09-27, cited in `client/GhgCanvas.tsx`).
- The six open questions of the triggers and notes specification took their proposed defaults.
- DISL gaps are proposed as additions to the DISL specification rather than kept as plugins (2026-10-05); the `x-` keys were shaped as those proposals, and all but the wire ids are standard DISL 0.3 (2026-10-06).

## The Notion row

The "Tools" database row, named "Gartner hype cycle graph" with origin `gartner/hypecycle-graph`: Kind Diagram; description "Gartner Hype Cycle (technologies and trends placed along the curve of expectations over time, from innovation trigger to plateau of productivity, with the relations between them)"; focus areas Technology assessment, Psychological and societal insights, Planning and roadmapping; purpose "Aims to provide insights in how socio-technological trends relate to each other."; rarity Unique to ADP; state 🛠️ Work-in-progress, and per host: Standalone 🛠️ Work-in-progress, VS Code ⚗️ Prototype, Notion ⚗️ Prototype (read on 2026-10-08); theory [Gartner hype cycle](https://en.wikipedia.org/wiki/Gartner_hype_cycle); why specialized "The Hype Cycle's meaning lies in the position on its curve; a dedicated tool keeps each item on the curve and relates them, which free drawing loses."

The Notion page's description of compact widths predates the ruling above; the code and this document follow the ruling.

In the Notion host the tool is a Notion add-on, published at <https://etalii.net/adp-notion/gartner-hype-cycle-graph/> from `etalii.adp.ide.notion` (spec 012-notion-hype-cycle-addon). It interprets this specification and the binding it names, and keeps a graph in a Notion database, one row per element. The nine examples of the row are in the workspace's Showcase, each with its own database and diagram page. Granting access works in a web browser and not yet in the Notion desktop app, which is why the row's state for Notion is Prototype and no further.
