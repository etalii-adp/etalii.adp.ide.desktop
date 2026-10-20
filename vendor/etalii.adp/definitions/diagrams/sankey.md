# Sankey diagram

The companion to [sankey.dis](sankey.dis). The `.dis` specifies ADP's **Sankey diagram** (`etalii/sankey`) in DISL 0.1 as far as DISL reaches; this page holds everything about the tool that DISL cannot express, or can express only approximately, so that an IDE implementing the diagram type from the specification ends up with the tool that exists today. Each point names the code that shows it.

The diagram type is implemented in the standalone IDE as the module `src/diagrams/sankey/` of [etalii-adp/etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone), origin `etalii/sankey`. Paths below without a repository are relative to that module folder; `standalone:` marks a path relative to the standalone repository root. It arrives in standalone with the pull request from branch `claude/sankey-diagram-8m13aq`. There is no specification document for it under `standalone:.spec-workflow/specs/`; the code is the source of truth.

## What the diagram is for

A Sankey diagram follows a quantity that is conserved as it moves, and shows how it divides and combines on its way: revenue by segment into gross profit and the cost of revenue, energy from its sources through conversion to where it is used, graduates by major into work, unemployment and the rest. Its question is not *what is connected* but *how much goes each way*: a bar is as tall as what passes through it and a band as thick as what it carries, on one scale, so the proportions are read off the picture.

- **Domain agnostic.** The diagram knows nodes and flows and nothing else. What a node stands for, and what unit a value is in, is the document's business: the palette words name colours rather than meanings, and a value's unit is written into its **format** (`€{value}M`, `{value} TWh`, `{value}k`). The `client/sankeyIds.ts` header says so in as many words.
- **Inspiration.** The [Sankey diagram](https://en.wikipedia.org/wiki/Sankey_diagram) generally, and the income-statement style of published annual reports in particular (revenue by segment on the left, what is left of it after each kind of cost to the right). The `standalone:docs/tools.md` row says so.
- **Origin.** The notation and its file format are defined by the user of the repository rather than by a standards body or a vendor, which is why the origin is `etalii/` and the schema is ADP's own (`backend/EtAlii.Adp.Diagram.Sankey/Diagram.cs`).
- **State.** ⚗️ Prototype, the state its `standalone:docs/tools.md` row carries. The `.dis` records this under `x-adp.state`, which is catalogue data rather than DISL.

## Mapping from the `.dis` to the implementation

| `.dis` name | Document spelling | Wire type | Where |
| --- | --- | --- | --- |
| `Node` | an entry under `nodes:` | `etalii/sankey+node` | `backend/EtAlii.Adp.Diagram.Sankey/_Model/SankeyModel.cs`, `SankeyNode`; `SankeyElementMapper.NodeType` |
| `Flow` | an entry under `flows:` | `etalii/sankey+flow` | `_Model/SankeyModel.cs`, `SankeyFlow`; `SankeyElementMapper.FlowType` |
| diagram `format` | `format:` at the top level | — | `SankeySettings.Format` |
| diagram `flowColor` | `flow-color:` at the top level | — | `SankeySettings.FlowColor` |
| diagram `thickness` | `thickness:` at the top level | — | `SankeySettings.Thickness` |

- **The media type is the origin.** The client registers its canvas for the MIME type `etalii/sankey` (`client/sankeyIds.ts`, `SANKEY_MIME`; `client/register.ts`), and the wire types are that MIME type with `+node` and `+flow`. The `.dis` records it as `persistence.mediaType`, the nearest DISL has; DISL's `mediaType` is meant for DID files, and here it names the body the host routes to the canvas.
- **One element type, one relation type.** The palette offers one item, "Node" (`SankeyToolboxProvider.cs`), because what a node stands for is not the diagram's to decide.
- **A node's value is derived.** It is never stored; it is the larger of the node's inflow and outflow over the drawn flows (`SankeyLayout.Values`). The `.dis` makes it a `derived`, `readOnly` attribute.
- **A flow's id is usually derived.** A flow states an `id` only when it needs one; otherwise its id is `from->to` (`SankeyFlow.Id`). DISL 0.1 has no derived id strategy (DISL 0.2's `derived` IdStrategy has), so the `.dis` describes it in prose under `persistence.ids`.
- **The colour** is one attribute in the `.dis`, a string holding a palette word or a hex, because that is what the document stores; the property grid shows it as two rows (see *Interaction*).

## The file format (`.skv`)

DISL's persistence layer describes a DID definition, and this diagram does not store DID, so the `.dis` names the format `plugin:net.etalii.adp.etalii.skv` and declares a plugin that provides `persistenceFormat`. The format itself, from `backend/…/SankeyParser.cs`, `SankeyWriter.cs` and `SankeyDocumentFactory.cs`, here from `examples/brightwater-coffee/brightwater-coffee.skv`:

```yaml
# Brightwater Coffee Co. - a fictional coffee company's income statement for fiscal year 2026.
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
  - id: cost-of-revenue
    name: Cost of revenue
    color: red
    format: "(€{value}M)"
flows:
  - from: cafes
    to: revenue
    value: 1840
  - from: revenue
    to: cost-of-revenue
    value: 1512
```

- **Shape.** YAML with a header `sankey: 1`, three optional document-wide keys `format`, `flow-color` and `thickness`, then two lists, `nodes` and `flows`. YamlDotNet reads; nothing ever serialises a model back.
- **Document-wide keys.** `format` is any text, default `{value}`. `flow-color` is `source` or `target`, default `target`; any other value is reported and read as `target`. `thickness` is any number above zero, default 1, clamped to 0.1..10; zero, a negative number or a word is reported and ignored.
- **Node keys:** `id`, `name`, `color`, `note`, `format`, `column`, `description`. **Flow keys:** `id`, `from`, `to`, `value`, `step`, `color`, `description`. There is no position and no size anywhere: every coordinate is computed (see *Layout*).
- **`column`** is one-based. It must be a whole number from 1; anything else is reported and ignored, and values above 1000 are read as 1000.
- **Numbers** are read with invariant culture as any finite double, and written in the format `0.####` (`SankeyWriter.Number`), so four decimals at most; the `.dis` says `precision.numbers: 4`.
- **Text** is written plain where YAML reads it back unchanged, and double-quoted where a plain scalar would mean something else: a colon, a hash or a leading dash (the shared `LineSplice.Quote`), and also a leading `[`, `]`, `{`, `}`, `&`, `*`, `!`, `|`, `>`, `%`, `@`, `` ` ``, `,`, `?`, `'` or `"`, a number, `true`, `false`, `yes`, `no`, `null` and `~` (`SankeyWriter.Text`). A format such as `{value} TWh` is therefore always quoted.
- **The empty document** is `sankey: 1`, `nodes: []`, `flows: []`, each on its own line, with CRLF and a final newline, because a new file has no style to preserve and CRLF is the standalone repository's house style. The lists are written flow-empty because a bare `nodes:` reads as null rather than an empty list; the first entry added opens the flow form into a block (`SankeyDocumentFactory.cs`). DISL's `newline: "crlf"` in the `.dis` describes only new files.
- **Round trip.** An unchanged document comes back byte-identical, and every edit is a splice of the line range the parser recorded for that entry (`LineSplice`), so comments, blank lines, indentation, quoting, unknown keys, line endings and a missing final newline survive. Because the bytes are the test subject, standalone's `.gitattributes` marks `*.skv -text`. Fixtures: `backend/EtAlii.Adp.Diagram.Sankey.Tests/Fixtures/crlf-line-endings.skv`, `lf-line-endings.skv`, `no-trailing-newline.skv`, `malformed-entries.skv`, `not-yaml.skv` and `rules-broken.skv`.
- **What an edit writes.**
  - A new node is `id` and `name`, then `color` and `column` when they have values, with indentation copied from the file. It is inserted directly before the node it was dropped above, so it lands at its place in its column, and appended after the last node otherwise. When the document has no `nodes` list, the list's key is created at the end of the document together with the first entry.
  - A new flow is `from`, `to` and `value`, with no id: its ends are its name. It is appended after the last flow, creating `flows:` at the end when there is none.
  - A document-wide key is rewritten on its own line, or inserted directly under the `sankey:` header when the document does not state it yet (`SankeyWriter.SetRootKey`).
  - Moving a node in its column moves its entry's own lines, comments inside it included, to directly before or after another node's entry; nothing else is touched (`SankeyWriter.MoveNode`).
  - Removing a node removes every flow to or from it and then the node, bottom-up so earlier line numbers stay valid.
  - Setting `name` keeps the key and refuses an empty one. Setting `color`, `note`, `format` or `description` to blank removes the key, as does clearing a number or setting the column to auto.
- **Tolerance.** The parser never throws. A document that is not YAML opens empty and says why, with its line. A missing header, a version that is not a number and a version other than 1 are reported, and the document still opens. An unknown key is reported and its line kept; an entry that is not a mapping is reported and passed over; a number that will not read is reported and ignored.
- **An unreadable body is not a missing one.** Both open empty, but a document that could not be read refuses every edit ("… could not be read, so nothing can be edited until it can: …") so that its emptiness is never saved over the only copy on disk (`_Model/SankeyDocumentEntry.cs`, `SankeyDocumentStore.cs`, `Commands/SankeyEdits.cs`). Changes made to the file outside ADP are picked up and pushed to every open view (`SankeyDocumentReloader.cs`).
- **Ids.** New node ids are `ShortGuid`s, 25-character lowercase base-36 renderings of a GUID; hand-written ids (`gross-profit`) are equally valid. DISL has no ShortGuid strategy, so the `.dis` says `nanoid`, the nearest shape. **Nodes and flows share one id space**, a flow's being its stated id or its `from->to`: the first entry with an id is drawn, and every later entry reusing it is not (`SankeyLayout.Compute`). So two flows between the same pair in the same direction are drawn only when each states an id of its own.
- **Registration.** A diagram is added to an ADP project by an `.adp` file whose first line is the origin `etalii/sankey` and whose second is `body: <name>.skv` (`examples/brightwater-coffee/brightwater-coffee.adp`). `.skv` is owned by this type (`Diagram.IsBody`).
- **Undo** restores the whole document text as it was before the command (`RestoreDocumentCommand<ISankeyDocumentStore>`), with the command itself as the redo, rather than inverting a model edit. An edit is made on a copy of the cached document, so a refusal halfway through leaves nothing behind.

## Layout

A Sankey diagram's geometry is its data, so nothing in it is a position anybody chose. The `.dis` names the layout `plugin:net.etalii.adp.etalii.sankeyLayout`, with `trigger: "always"` and `respect: "none"`, and declares it **required with no fallback**, because a bar's height and a band's thickness are on the scale it computes: its CEL functions `sankeyColumn`, `sankeyColumnCount`, `sankeyScale`, `sankeyTop` and `bandAt` feed the notation. The algorithm is `SankeyLayout.Compute`:

- **It is computed over the whole document**, never over a view, and cached per model instance, so a pan reuses it and an edit recomputes it. Only drawable entries take part: nodes with an id not taken earlier; flows whose id is not taken earlier, whose two ends are drawable and different.
- **Values.** A flow carries its value, with a missing or negative value counting as 0. A node's value is the larger of the sum of what flows into it and the sum of what flows out of it.
- **Columns.** A node that states `column` is in that column (one-based in the file, zero-based here). Every other node is one column right of the furthest node that flows into it, over the flows that close no cycle, found by a depth-first walk in document order that leaves out each flow reaching a node still on the walk; a node nothing flows into starts in the first column. Columns are therefore by longest path from the sources, and a sink is **not** pushed to the last column: a node reached in one step sits in the second column however long the other paths are.
- **Order within a column** is the order the nodes are written in the document, top to bottom. It is the one thing a reader arranges (see *Reordering a column*), and nothing in the layout ever changes it.
- **Scale.** One scale for every bar and band: the column whose node values sum highest fills `560 × thickness` canvas units, so the scale is `560 × thickness / fullest`, or 0 when nothing flows. A bar is `max(6, value × scale)` tall and 24 wide; a band is `max(1, value × scale)` thick, so a flow without a value is a hairline. Bars in one column are at least 36 apart; columns are 320 apart, left edge to left edge.
- **Start.** The extent is the tallest column, bars plus gaps. Every column starts stacked from the top, centred on the extent.
- **Bands at a bar** are stacked edge to edge, the outgoing ones in the order of the centres of the nodes they reach and the incoming ones in the order of the centres of the nodes they leave, ties broken by document order, so bands do not cross at the bar. A band's centre is given to the client as a fraction 0..1 of each end's height from its top (`SankeyBand.SourceAt`, `TargetAt`; 0.5 on a bar that carries nothing), so a band moves with its node.
- **Relaxation.** 24 passes, the n-th weighted `0.99^n`. Each pass goes right to left, moving each node towards the value-weighted mean of the tops at which its outgoing forward bands would run level, and then left to right likewise for its incoming forward bands; after each column, the column is resolved. Only forward flows (to a later column) pull.
- **Resolve** keeps a column's nodes within the extent, in their order and never closer than the gap: pushed down from the top, then up from the bottom, then down once more in case the column is as tall as the extent. A node pulled past its neighbour stops at the gap: relaxation **never reorders**.
- **Rounding.** Tops and heights are rounded to two decimals, so a recompute does not send a delta for a change in the fifth decimal.
- **What the backend sends.** Only what is in view: a node whose bar overlaps the viewport, plus every node a sent node shares a flow with, so no flow arrives with one end missing; a flow when both its ends are sent (`SankeyElementMapper.Visible`). Scrolling and zooming never change the layout.

## Reordering a column

The order within a column is the only thing a reader arranges, and it is stored as the order of the document's entries. DISL 0.1 has no reorder gesture, so the `.dis` makes a node's `y` a computed placement whose `write` calls a plugin operation, `moveInColumn`, with the drop's middle; this is how it actually behaves (`SankeySession.MoveElementToAsync`, `client/SankeyCanvas.tsx`, `client/sankeyModel.ts`):

- **A node moves only up and down its column.** The client's definition snaps x with a step of 1,000,000 from the node's own left edge, so x always rests where it was. A column is decided by the flows or by the Column property; a drag sideways would be a third way the document could not keep.
- **While it is dragged**, the nodes it passes step aside by its height plus the gap (36), so the reader sees where it will land (`columnPlaceOf`). Nothing is written until the release.
- **On release** the backend takes the drop's middle and moves the node's entry directly before the first other node of its column whose middle is below it, or after the last one; when that is where it already is, nothing is written and nothing is refused. Until the new layout arrives, the node is drawn in the slot it was given.
- **Refusals:** "This diagram is read-only." without a history, "That is not something this diagram can move." for anything that is not a drawn node. Reparenting is refused: "A node takes its place by being dragged up or down its column."

## Notation details DISL approximates

**Bars** (`client/SankeyCanvas.tsx`, `NODE_ELEMENT_TYPE`): a rectangle 24 wide and as tall as the layout says, filled with the node's colour, no outline. Its labels sit 10 beyond the bar (`insetX` 34 from the far edge, the bar's 24 plus 10), centred on its middle: before the bar, right-aligned, for a node in the first column, and after it, left-aligned, everywhere else (`side` in the payload, `left` for column 0). The **name** at −5 (13 px, bold) and the **value** at +11 (12 px), both in the bar's colour; the **note**, only when there is one, at +26 (11 px, italic, muted). Every label has a 3 px halo in the surface colour so it stays legible where a band runs behind it (`client/sankey.css`). The `.dis` writes two label sets, each visible on its side.

**The tooltip** of a node is `name: value` with ` (note)` when it has one, the value in its format. A flow's is `source → target: value`, the value through the **document's** format.

**Bands** (`centrePath`, `bandPath`, `bandAdornment`):

- **A filled ribbon, not a stroke.** Its top and bottom edges are the same cubic S, `thickness` apart **vertically** all the way along, so a band is as thick as its value at every x and the bands at a bar meet edge to edge. A stroked line is thick across the curve and narrows on the slope. DISL 0.1 has only stroked edges, so the `.dis` draws a stroke as wide as the thickness; an implementation copying the tool draws the ribbon.
- **Route.** From the source's right edge, at `SourceAt` of its height, to the target's left edge, at `TargetAt` of its height; both control points lie horizontally from the ends, `max(40, |dx| / 2)` away. The `.dis` says `routing: "bezier"` leaving and arriving to the right, with side anchors; DISL 0.1 cannot pin the attach point to a computed fraction along an edge (DISL 0.2's `part` anchoring with a bound `at` can), cannot restrict anchors to the left and right sides (AnchorSpec `sides` is DISL 0.2), and cannot pin a bezier's reach (DISL 0.2's `BezierSpec`, whose `auto` is half the horizontal distance but at least 30, not 40).
- **A backward flow** (to a column at or left of its source's) is drawn with the same rule, so it doubles back from the source's right edge to the target's left edge. The payload carries `backward`, but the client does not use it (see *Known gaps*).
- **Translucent**, at 0.42 fill opacity, 0.62 on hover, so a band crossing another shows both and the bars stay the strongest mark. The library's own line is the band's centre, kept at opacity 0 for hit-testing and the connect preview. A selected flow's band is outlined 2 wide in the selection colour; the `.dis` approximates that outline with a casing.
- **No arrowhead and no label on the band**: the direction is the reading direction, and the values are written at the bars.

**Colours.** The twelve palette words - grey, slate, purple, blue, teal, green, lime, yellow, orange, red, pink, brown, in the order the property grid offers them (`SankeyColors.Palette`) - are the host theme's (`--color-diagram-sankey-*` in `standalone:src/client/src/index.css`), declared once for the light mode and once, lifted, for the dark; the `.dis` carries both as theme tokens. A hex colour (`#rgb` or `#rrggbb`) is the document's own choice and is painted as written in both modes, lower-cased. A node with no colour or an unknown one is grey. A flow with no colour, or an unknown one, takes the resolved colour of its target, or of its source when `flow-color: source` (`SankeyElementMapper.Flow`). A palette word is painted through its theme token and a hex as itself; the `.dis` does the same with `paint()` and `token()`.

**Numbers** are written with `#,0.##` in invariant culture: thousands grouped, two decimals at most, none when whole (`SankeyFormat.Number`). A format replaces every `{value}` with the number; a format without the placeholder is shown after the number with a space (`TWh` reads `54 TWh`); an empty format shows the number alone (`SankeyFormat.Write`). A node's own `format` overrides the document's for its value only.

## Interaction

**The palette** has one item, "Node" (`sankey.toolbox.node`, icon `mdi-chart-sankey`, "Something a quantity flows into, out of, or through. Drop it in the column it belongs to."), described by the backend as data. **Adding** is a drop from the palette onto the canvas, or "Add node here" on empty canvas, both answered by `sankey.add` on the target `new:x,y` (`AddSankeyNodeCommandHandler`):

- The column is the one nearest the drop (`round((x − 12) / 320)`, never left of the first). When it is not the first column, it is **written** as the node's `column`, because a node with no flows would otherwise fall to the first.
- Its place is directly above the first node of that column whose middle is below the drop, or after the last node.
- It is named "New node", then "New node 2", "New node 3" when a node's label already says so, and gets no colour.

DISL's toolbox has no drop-from-palette mode and a tool cannot write the column a drop implies, so the `.dis` uses `click` and leaves both to this page.

**Drawing a flow** is a right-button drag from one node to another (`connectOnRightDrag`); the node dragged from is the source. A node's whole left and right edges are its anchors, and no handle is drawn. It travels as `sankey.connect` on the target `rel:from->to`. A new flow's value is what its source has received and not yet passed on, rounded to four decimals, or 1 when nothing is left over (`ConnectSankeyNodesCommandHandler.Remainder`); the `.dis` writes that as a `connect` hook. Refusals: "A flow runs from one node to another.", "A node does not flow into itself." and "These two are already joined by a flow in this direction; change its value instead." (also when another entry already has the id `from->to`).

**Values** are stepped on a selected flow only; a node's value is what flows through it and is never stepped or edited ("A node's value is what flows through it; change a flow's value instead."). A step adds or takes the flow's own `step`, or, when it states none or one that is not above zero, a tenth of the value's order of magnitude and 0.1 below 10 - so 3.3 steps by 0.1, 54 by 1 and 1,302 by 100 (`StepSankeyValueCommandHandler.DefaultStep`); the default is recomputed from the current value at every press. The result is rounded to four decimals so ten steps of 0.1 come back to a whole number, and never goes below zero: a decrease that would pass zero lands on zero, and a decrease at zero is refused ("The value is already zero."). A value that was not stated, or is negative, counts as zero. The keyboard: `+` with or without Shift, and `=`, increase; `-`, and `_` with Shift, decrease. There are no on-canvas stepper buttons.

**Thickness.** "Thicker bands" and "Thinner bands" multiply or divide the document's `thickness` by 1.25, round it to two decimals and clamp it to 0.1..10 (`ScaleSankeyThicknessCommandHandler`). They are offered on every menu - a node's, a flow's and empty canvas - disabled at their end of the range ("The bands are already as thick as they go." / "… thin as they go."), and refused when the rounded result equals the current value ("The bands are already as thick as this diagram draws them." / "… thin …"). One number for the whole diagram, because making one band thicker than its value says would make the diagram lie.

**Actions and keys** come from the backend as data and reach the right-click menu and the keyboard through one path (`SankeyContextActionProvider.cs`). Each row group below is a separate menu group.

| On | Action | Key | Behaviour |
| --- | --- | --- | --- |
| Node | Rename… | F2, double-click | Asks for "Name" in a prompt prefilled with the current label (confirm "Rename"), refuses a blank one ("A node needs a name."). Not edited in place. |
| Node | Remove | Delete | Removes the node and every flow to or from it. With no flows it happens at once; otherwise it asks first, as a danger confirmation, naming the count: "Removing this node also removes the 1 flow to or from it." / "… the N flows to or from it." |
| Flow | Increase value, Decrease value | +, - | As above; Decrease is disabled at zero with "The value is already zero." |
| Flow | Remove flow | Delete | At once. |
| Node, flow, empty canvas | Thicker bands, Thinner bands | | As above. |
| Empty canvas | Add node here | | As a drop at that point. |

DISL 0.1's deletion `confirm` is a fixed message asked every time; the count and the condition are this page's (DISL 0.2's Confirmation has `count` and `threshold`). DISL 0.1 also has no menu for empty canvas (DISL 0.2 reserves `diagram` in `contextMenus[].for`), so the `.dis` offers the thickness operations from the node and flow menus only, and Add node here not at all. A flow has no rename: it has no name.

**Properties** (`SankeyContextPropertyProvider.cs`), every row an edit through `SetSankeyPropertyCommand` on the project's history:

- A node: **Name** and **Description** (group Identity); **Colour**, **Custom colour** and **Column** (group Look); **Value** (read-only), **Note** and **Format** (group Amount).
- A flow: **Description** (Identity); **Colour** and **Custom colour** (Look); **Value** and **Step** (Amount). Step shows the default step for the current value when the flow states none.
- **Colour is offered twice**, both writing the one `color` key: as a choice of "(default)", the twelve words and, while a hex is in use, "(custom)"; and as a Custom colour text for a hex. Choosing "(default)" removes the key; choosing "(custom)" keeps the hex. The `.dis` has one combobox, which allows free entry.
- **Column** is a choice of "auto" and 1 to one past the last column (or the node's own column, if higher); "auto" removes the key.
- **Checks.** Values are trimmed. A value or step must read with invariant culture as a finite number of zero or more ("'…' is not an amount this diagram can hold: a number of zero or more."), and a step must not be zero ("A step of zero would make + and − do nothing."); an empty number removes the key. A colour must be a palette word, a hex or empty ("`…` is not a colour: one of grey, slate, …, brown, or #rrggbb."). A column must be a whole number from 1 ("'…' is not a column: a whole number from 1, or nothing to let the flows decide."). A name must not be empty.

## Rules

Breaches reach the Errors and Warnings panel through `SankeyValidator`, the module's `IDiagramValidator`, each located by its line counted from 1. **What is lost is an error, what is merely untidy a warning**: an error names something that is not drawn, a warning something that still draws what the author meant. Fixture: `backend/…Tests/Fixtures/rules-broken.skv`.

| Rule id | Severity | When | In sankey.dis |
| --- | --- | --- | --- |
| `sankey.unreadable` | warning | Anything the parser reported: not YAML, a missing or wrong header, an unknown key, a number that will not read, an entry that is not a mapping, a `flow-color` that is neither word, a `thickness` not above zero, a `column` that is not a whole number from 1. | Not expressible; the reader's. |
| `sankey.missing-id` | error | A node has no id ("A node has no id and is not drawn."). A flow needs none. | Not expressible; ids are persistence. |
| `sankey.duplicate-id` | error | An id is used again by a node or a flow; only its first entry is drawn. Two flows between the same pair without ids read "A flow from `a` to `b` is written twice; only the first is drawn - add their values, or give each an id." | Not expressible. |
| `sankey.dangling-flow` | error | A flow names a node that is not there, or is missing an end. | `std.references`, `std.endpoints`. |
| `sankey.self-flow` | error | A flow runs from a node to itself. | `selfFlow`, and `allowSelfLoops: false`. |
| `sankey.missing-value` | warning | A drawable flow states no value; it is drawn as a hairline. | `missingValue`. |
| `sankey.negative-value` | warning | A flow's value is negative; it is drawn as zero. | `negativeValue`. |
| `sankey.unknown-color` | warning | A node's or a flow's colour is neither a palette word nor a hex; it is drawn as if it stated none. | `unknownColor`. |
| `sankey.backward-flow` | warning | A drawn flow runs to a column at or left of its source's: a cycle, or a stated column. | `backwardFlow`, through the layout's `sankeyColumn`. |

Negative values and zero steps are refused by every edit but only negative values are reported in a file, so the `.dis` uses `change` rules rather than `min` facets, whose built-in check would report them. A second flow between the same pair in the same direction is refused while drawing, so the `.dis` sets `allowParallel: true` and states the refusal as a `connect` rule.

## Known gaps

- **DISL 0.1 gaps this diagram exposed:** no filled ribbon whose thickness is measured vertically (edges are stroked lines); no edge end attached at a computed fraction along a side; no anchor sides restricted to left and right; no pinned bezier reach; no reorder gesture (the `.dis` routes a drag's `y` through a plugin operation); no drop-from-palette toolbox mode, and no tool that writes what its drop position implies; no conditional deletion confirmation with a count; no menu on empty canvas; no derived id (`from->to`); no form field offered twice for one attribute (palette choice and hex). Several of these are answered by DISL 0.2, as noted where they occur.
- **Inconsistencies in the implementation**, described here as they are rather than smoothed over:
  - **`backward` is sent and not drawn.** The validator says a backward flow "loops round instead of reading left to right", and the payload carries `backward`, but the client ignores the flag and draws the same S, which doubles back across the columns between its ends.
  - **A dropped node is pinned to its column.** A node dropped anywhere but the first column has its `column` written, so it stays there after flows are drawn to it, even where the flows would put it elsewhere, and a flow into it from a later column is then a backward flow. A node dropped in the first column has no `column` and moves with its flows.
  - **`#rgb` is accepted but `#rrggbb` is what is said.** `SankeyColors.IsHex` accepts three and six digits; the messages, the proto comment and the property grid speak only of `#rrggbb`.
  - **A flow's value ignores its nodes' formats.** A flow's display value (its tooltip) uses the document's format only, so in `brightwater-coffee` a flow into a cost node reads `€…M` while the node reads `(€…M)`.
  - **Two wordings at the thickness limits**: the disabled menu entry says "as thick as they go", the command's refusal "as thick as this diagram draws them".
  - **A new flow's starting value counts every flow in the file**, undrawable and duplicate ones included, while a node's value counts only drawn flows.

## The examples

Three examples under `examples/` (also under `standalone:src/examples/diagrams/sankey/`), each with an `.adp` registration and a readme.

- **`examples/brightwater-coffee/brightwater-coffee.skv`**: the income statement of a fictional coffee company for fiscal year 2026, written for ADP, because the format is the repository's own; every figure is invented. 14 nodes in five columns decided by the flows, 13 flows coloured by their target, the document format `€{value}M` and `(€{value}M)` on the cost nodes, notes for year-on-year change and margins, four palette words.
- **`examples/uk-energy/uk-energy.skv`**: a possible UK energy system in 2050 in TWh, vendored from the plotly.js test mock `sankey_energy.json` (MIT, `LICENSE.md` beside it), itself after the d3-sankey energy example and the UK DECC 2050 Calculator. 48 nodes, 68 flows coloured by their source (`flow-color: source`), format `{value} TWh`, flows from under 1 to over 500 TWh.
- **`examples/recent-graduates/recent-graduates.skv`**: recent US college graduates by major category into work and kinds of job, in thousands, derived from FiveThirtyEight's `college-majors/recent-grads.csv` (CC BY 4.0, `LICENSE.md` beside it), from the American Community Survey 2010-2012. 22 nodes in three columns, 51 flows coloured by their target, format `{value}k`.

Their readmes list what they do not demonstrate: a custom colour, a stated column, a cycle or a backward flow, notes and per-node formats outside the coffee company, and a document that breaks the rules (the fixtures cover that).

## Sources

- Module code and tests: `src/diagrams/sankey/` in etalii.adp.ide.standalone.
- Catalogue row: `docs/tools.md` in etalii.adp.ide.standalone, row for `etalii/sankey`.
- Palette tokens: `--color-diagram-sankey-*` in `src/client/src/index.css` in etalii.adp.ide.standalone.
- Canvas library: `src/client/src/canvas/library/` and `src/client/src/canvas/canvas.css` in etalii.adp.ide.standalone.
