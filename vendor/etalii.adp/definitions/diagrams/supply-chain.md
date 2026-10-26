# Supply chain diagram

[supply-chain.dis](supply-chain.dis) specifies ADP's **Supply chain diagram** (`etalii/supply-chain`) in DISL 0.3. The specification holds everything DISL 0.3 can state: the metamodel with its one node type and the seven stages as an enumeration, the notation, the toolbox, the context menus, the forms with their checks, the constraints with their codes and sentences, the operations with their refusals, the trace through the viewer's selection, Arrange diagram, the deletion confirmation, the ids and the persistence through an FBL binding. This page holds what DISL 0.3 cannot state and the places where DISL and FBL fall short, so that a host implementing the diagram type from the specification ends up with the tool that exists today.

The reference implementation is the standalone module [`src/diagrams/supply-chain/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/supply-chain) in etalii.adp.ide.standalone, which that repository's specification `supply-chain-disl-fbl` converts to this definition. Every sentence, label and code in the definition and below is the one its parity transcript records (`backend/EtAlii.Adp.Diagram.SupplyChain.Tests/Parity/supply-chain.transcript.json`), and every fixture named is under `backend/EtAlii.Adp.Diagram.SupplyChain.Tests/Fixtures/`. Paths are relative to `src/diagrams/supply-chain/` in the standalone repository.

## Contents

1. [What the diagram is for](#what-the-diagram-is-for)
2. [The document on disk](#the-document-on-disk)
3. [Layout](#layout)
4. [Interaction DISL cannot state](#interaction-disl-cannot-state)
5. [Notation DISL cannot state](#notation-disl-cannot-state)
6. [Language gaps](#language-gaps)
7. [Definition defects repaired](#definition-defects-repaired)
8. [The examples](#the-examples)
9. [Sources](#sources)

## What the diagram is for

A supply chain diagram follows goods from where they start to where they are used: the sources they enter by, the processors and producers that turn them into material and parts, the integrators that combine those into the finished whole, and the hubs, outlets and consumers it reaches. The stages name roles, not industries, so the same notation reads for goods, energy, data or services. Its question is not only *who supplies whom* but *how much*, and *what else depends on this*: a flow's band is as broad as its volume, and selecting anything lights the whole chain through it.

- **Inspiration.** The notation follows the [yFiles supply chain showcase](https://www.yfiles.com/demos/showcase/supply-chain/): cards per stage with a coloured header, flows drawn as bands whose width follows their volume with goods moving along them, regions as frames, and the selection tracing everything upstream and downstream.
- **Origin.** The notation and its file format are defined by the user of the repository rather than by a standards body or a vendor, which is why the origin is `etalii/`.
- **State.** ⚗️ Prototype, the state its row in the standalone `docs/tools.md` carries, recorded under `x-adp.state`.

## The document on disk

A `.supply` document is YAML: a header `supply-chain: 1`, then three lists, `groups`, `nodes` and `flows`. A group's keys are `id`, `name`, `description`, `x` and `y`; a node's are `id`, `type`, `name`, `description`, `group`, `quantity`, `unit`, `step`, `x` and `y`; a flow's are `id`, `from`, `to`, `product`, `description`, `volume`, `unit` and `step`.

```yaml
supply-chain: 1
groups:
  - id: central-africa
    name: Central Africa
nodes:
  - id: kolwezi-cobalt
    type: source
    name: Kolwezi cobalt mines
    group: central-africa
    quantity: 170
    unit: kt Co
  - id: cobalt-refinery
    type: processor
    name: Cobalt refinery
    x: 320
    y: 40
flows:
  - id: cobalt-ore
    from: kolwezi-cobalt
    to: cobalt-refinery
    product: Cobalt hydroxide
    volume: 120
    unit: kt Co
```

The specification names the module's FBL binding, `backend/EtAlii.Adp.Diagram.SupplyChain/supply-chain.fbl`, in `persistence.binding`, with a `typeMap` that maps the binding's `Document`, `Group`, `Node`, `Flow` and `Unreadable` onto the diagram, `Group`, `Stage`, `Flow` and the unreadable entry. **The binding is not written yet**: the standalone module still reads with its own parser and writes through its own line splices, and its binding waits for the construct of gap 1 to be taken up there. Until it exists, what the binding will have to say is this:

- **A node's `type` is its stage**, one of the seven words, and six older words still read as the stage each became: `raw-material` as source, `supplier` as processor, `manufacturer` as producer, `assembler` as integrator, `distributor` as hub, `retailer` as outlet (`legacy-stages.supply`). A line nobody edits keeps its old word, and any change of stage writes the new word. A word that is none of the thirteen is kept as written, reported ("`quarry` is not a stage this module knows.") and not drawn.
- **A node names its group by the group's id** (`group: central-africa`), and groups do not nest. The definition states it as a reference attribute, not as containment (gap 2).
- **Positions are the top-left corner** (`x`, `y`) in whole canvas units, read as placing an entry only when both are written. Sizes are never stored: every node is 192 by 96, and a group's frame is computed from its members. Positions are written rounded to whole units, with a half rounded to the even neighbour (`half-positions.supply`: 10.5 is written 10, 252.5 is written 252).
- **Every value is read as it is written** (`scalars-as-written.supply`: a name written `yes`, `~`, `12` or `null` is that text), and a number is whatever the invariant culture reads from the text (`numbers-as-text.supply`: `"50"` in quotes is 50, `1e3` and `+7` are numbers, `0x10`, `.inf` and `1_000` are not and are reported as "`quantity: 0x10` is not a number and was ignored.").
- **The empty document** is `supply-chain: 1`, `groups: []`, `nodes: []` and `flows: []`, each on its own line, with CRLF and a final newline. The lists are written flow-empty because a bare `nodes:` reads as null; the first entry added opens the flow form into a block (gap 5).
- **Round trip.** An unchanged document comes back byte-identical, and every edit changes only the lines it must, so comments, blank lines, indentation, quoting, unknown keys, line endings and a missing final newline survive (`crlf-line-endings.supply`, `lf-line-endings.supply`, `no-trailing-newline.supply`). Because the bytes are the test subject, the standalone repository's `.gitattributes` marks `*.supply -text`.
- **An unreadable body is not a missing one.** A body that is not YAML, and one with a key written twice in one mapping, opens empty and refuses every edit ("not-yaml.supply could not be read, so nothing can be edited until it can: The document could not be read as YAML: …", `not-yaml.supply`, `duplicate-key.supply`). A body whose root is not a mapping opens empty with no finding (`root-not-a-mapping.supply`).

## Layout

The specification names the layout `plugin:net.etalii.adp.etalii.supplyChainLayers`, falling back to `layered`, with `trigger: "always"` and `respect: "all"`: a node the document places is drawn there, and every other node is placed by the layout. A document whose nodes carry no positions (both examples) is drawn entirely by it, so a host that reproduces the tool must reproduce it. The module implements it as `SupplyChainLayout`:

- **It is computed over the whole document**, never over a view. Only drawable entries take part: groups, nodes and flows with an id no earlier entry holds, ids taken groups first, then nodes, then flows; nodes of a stage this diagram knows; flows whose two ends are drawable and different.
- **Layers** run left to right and follow the stages. A node's layer is at least its stage's place in the chain (source 0 to consumer 6, an older word ranking as the stage it reads as) and at least one past every node that supplies it, over the flows, after every cycle has been broken at the flow that closes it in a depth-first walk in document order, so a recycling loop does not stretch the chain.
- **Lanes.** A flow forward over more than one layer gets a slot in every layer it crosses, as if a card stood there, in the band of the node it delivers to; the slots take part in ordering and placement exactly as nodes do, and draw nothing. The flow is drawn through the middle of its slots, so it passes between cards rather than through them. Only a flow whose two ends stand where the layout put them uses its lanes.
- **Bands** are the groups, plus one band for the nodes outside every group, each first in document order of its first member. **Order** is by barycentre, eight passes, ties broken by document order. **Placement** drops the bands, in that order, onto a per-layer skyline: layers are 320 apart (the card's 192 plus 128), nodes stacked in one band 124 apart (96 plus 28), a grouped band reserves 40 above its nodes for the frame's title and 20 below them, and two bands sharing a layer are 32 apart.
- **A node is placed by the document only when it states both `x` and `y`.** With one of them missing the layout places it, and DISL's `respect` cannot say *only when both coordinates are stored* (gap 8).
- **Arrange diagram** writes the computed position of **every** node into the document, whatever it stated before, as one undoable step. DISL's `respect` describes the automatic layout only; the definition states Arrange as a `layout` action, and this sentence is what says it overrides stored positions.

## Interaction DISL cannot state

- **Dragging a card into a frame** (finding A13). When the card's centre lands inside **another** group's frame, and not inside its own group's, the node joins that group; where frames overlap, the smallest frame wins. Dropped anywhere else it stays in its own group, whose frame grows to follow it. When the node was the last member of the group it left, that group's `x` and `y` are written as the old frame's top-left, so the group stays where it was, drawn empty. All of it is one edit. **Dragging a group with members** moves every member by the same distance, and shifts the group's own `x` and `y` by that distance too when it has them; **dragging an empty group** writes its `x` and `y`. A hook's context holds `self`, `old`, `event`, `diagram` and `env`, and no frame of another element, and the frames are the layout plugin's, so no hook can state it (gap 8). The canvas library's own reparent gesture is refused with "A node joins a group by being dragged inside its frame, or through its Group property.", a flow is refused with "That is not something this diagram can move.", and a diagram opened without a history with "This diagram is read-only."
- **The steppers.** Beside the one node or flow that is selected, the tool draws a − and a + button: round, 22 across and 4 apart, at the card's right edge level with the quantity (centres 141 and 167 from the card's left, 68 from its top), or for a flow under the volume pill (38 below the middle of its two ends). A press steps the value without moving the selection, so the chain stays lit. At zero the − is drawn at 0.35 opacity with a not-allowed cursor. The definition declares the buttons as context tools; it cannot place one under a flow's label (gap 8).
- **An entry without an id is found by the empty id.** The tool finds an element by the id it is written with, the first node with it, else the first flow, else the first group, and the empty id is no exception: a menu or the property grid asked for `""` shows the first entry written without an id, and a node with no `group` shows in its Group row the name of the first group written without an id, when there is one. The definition keeps both as the tool does: the Group row reads the group by `written`, and the host finds an element by the id it is written with. Neither is reachable from the canvas, which draws no entry without an id.
- **The keys** `+` with or without Shift, and `=`, increase; `-`, and `_` with Shift, decrease. DISL gives an operation one `shortcut`.

## Notation DISL cannot state

- **The goods** are a second line along the band's path, in the surface colour: dashes of 1 and gaps of 13, at least 2 wide and otherwise 0.4 of the band, at 0.85 opacity, moving forward 14 units every 1.4 s, and still when the user prefers reduced motion or the flow is dimmed. DISL's `flow` animates a line's own dashes and an edge has one line, so the definition draws the goods as dots repeated along the band, which do not move (gap 8).
- **The unit beside the quantity** is a separate 11 px muted text just after the number, at an x estimated from the number's characters (12 per digit, 6 per other character, plus 6). DISL cannot place a label by another label's width, so the definition joins the number and the unit in one label (gap 8).
- **The bezier's reach.** Both control points lie horizontally from the ends, `max(48, |dx| / 2)` away. `BezierSpec`'s `auto` is half the distance and at least 30, so the definition's curve is flatter than the tool's for two cards closer than 96 apart. The arrowhead is square to the consumer's side rather than to the curve's tangent.
- **A flow with lanes** is a chain of such curves: from the supplier to the left of its first lane, straight across each lane as wide as a card, and on to the next, the last arriving at the consumer. The lane centres are the layout plugin's and travel on the flow's payload; DISL has no route through computed points other than `routing: "layout"`, which hands the whole route to the layout.
- **The share bar** of a card is its share of the largest quantity **among the nodes the client holds** with the same unit, which, because the backend sends only what is in view plus what the visible nodes trade with, can change as the view is panned. The definition computes the share over the whole diagram.

## Language gaps

Each gap is a candidate change to DISL or FBL, found by the standalone conversion. None is worked around in the specification by stating something the tool does not do. The nine of the requirements' section C come first, and the two the design added after them.

| # | Gap | Construct | Evidence | Would resolve it |
| --- | --- | --- | --- | --- |
| 1 | **Closed by FBL 0.5.** A slot that reads a YAML scalar as written (B1, B2) | FBL 4.1.5 written text, 5.2 `read`, `quoted` and `fallback` | `scalars-as-written.supply`, `numbers-as-text.supply` | Done: an id, a reference and a text attribute read their written text, and a number slot asks for it. The module's binding is written against it when the module takes it up |
| 2 | Containment stated by a reference key (A6, B12) | DISL 4.6 `children`, 11.2 `typeMap` attributes; FBL 5.5 (a child is the entry nested in its parent) | `group-removed.supply`; the examples, where every node names its group | A `typeMap` attribute mapped to `"parent"`, with a parent that names nothing kept at the top level and reported as a dangling end is |
| 3 | An entry that is kept, reported and not drawn (A8) | DISL 11.5.4 `missing`, the second and later holders of an id | `ids-missing-and-repeated.supply`, `rules-broken.supply` | A `missing` and a duplicate policy that keep the entry, report it and leave it undrawn, without writing an id |
| 4 | Findings in one list by line (A9) | DISL 8.6 ordering, `constraints.order` (8.1) | `findings-order.supply`; the standalone module sorts the derived list by line | An ordering option `by: "location"` that sorts by line across codes |
| 5 | Opening an empty flow collection (B6), and a set on an entry written as a flow mapping (B13) | FBL 4.3 (a flow collection is one value whose only writable span is the whole), 6.1, 6.2 | `empty-lists.supply` (`groups: [ ]` is opened; `nodes: []  # nothing here yet` is not, and the document breaks), `flow-mapping-entry.supply` | A sentence in 4.3 or 6.2 that inserting an entry into an empty flow sequence writes a block sequence holding it, key slots that bind the members of a flow mapping, and a conformance fixture each |
| 6 | What `map` reads for a wire value it does not list (A5) | FBL 5.2 `map` | `legacy-stages.supply` (`type: quarry`) | A sentence that a value `map` does not list is kept as written. The other half of the requirements' gap 6, one binding type read as several model types, is not met, because the stages are one attribute (Q2) |
| 7 | Not a gap under Q1's default: a removal that leaves leading comments, and a new key placed after the entry's first line (B8, B9) | FBL 6.1, 6.2 `insert.keys` | `comment-above-entry.supply`, `first-placement.supply` | Nothing, while Q1 keeps FBL's bytes; a whole-line rewrite and both placements, should the ruling change |
| 8 | The notation and gestures of A10 and A13 | DISL 6.10 edges (one line), 6.7 labels, 5.4 context tool positions, 10 `respect`, 9 hooks (no frame of another element) | The examples' bands; the transcript's drag steps and `sessionRefusals` | A second, animated line along an edge; a label placed by another label's width; a context tool placed under an edge's label; a `respect` that needs both coordinates; a move that reparents by where the centre lands |
| 9 | The parser's own message in the sentence of `std.unparseable`, and the reader's sentences for an entry that is still read (A11) | DISL 8.7 `std.unparseable` and `std.unreadableEntry` (kept and not drawn), 11.2 host attributes (invisible to constraints) | `not-yaml.supply`, `unreadable-sentences.supply`, `no-header.supply`, `header-not-a-number.supply`, `header-other-version.supply` | A `detail.reason` whose text is the same in every host, and a built-in for a reader finding on a read entry, worded by the definition from what the reader saw |
| 10 | What a reference or a relation end was written with, once it names nothing | DISL 12.2: CEL reads an unset reference or end as absent | `dangling-flows.supply` ("The flow `to-nowhere` names `nowhere`, which is not a node here; it is not drawn."), `rules-broken.supply` (the unknown group `south`) | A member of an element that answers the text a reference or an end was written with, also when it names nothing; the plugin function `written` stands in for it |
| 11 | A finding on the line of a key rather than of its element | DISL 8.6 locations (an element and an attribute) | `unreadable-sentences.supply` (`quantity: lots` on line 14, its node starting on line 10) | A finding located at the key's line, or an attribute location that a host maps to the key's line |

### 1. FBL: a scalar read as written (closed)

Closed by FBL 0.5 (etalii.adp spec 015, `fbl-written-text`) before this conversion reached it: section 4.1.5 gives every YAML scalar a written text, and section 5.2 lets a slot read it with `read: "text"` and keep its entry readable with `fallback`. The binding the standalone requirements drafted marks these slots with `"as": "text"`, which FBL 0.5 spells `read`; it is rewritten against FBL 0.5 when it is written.

### 2. DISL and FBL: membership by a key

A node names its group with `group:`; FBL makes an element a child of the nearest enclosing entry, and a `typeMap` attribute maps to an attribute name, `source`, `target`, `id` or `null`, never to the parent. The definition therefore states `group` as a reference attribute to `Group`, by Q5's default: removing a group unsets the reference (`references: "unset"`), which removes the key and leaves the nodes where they are, as the tool does; the frame around the members is the layout plugin's, not a container's. Containment would have given the frame that encloses its members and moves them, and `children: "reparent"`, but it would also have lost a node whose group names nothing, which the tool keeps, draws outside every group and reports.

### 3. DISL: an entry that is kept, reported and not drawn

The tool does not draw an entry without an id ("A node has no id and is not drawn.") nor a second holder of an id ("The id `shared` is used more than once; only its first entry is drawn."), and takes "first" as groups, then nodes, then flows, whatever order the sections have in the file (`ids-missing-and-repeated.supply`: the flow `shared` at line 5 is the one reported, because the node `shared` further down is read first). DISL 11.5.4 says of both that they **MUST** be drawn, and takes reading order. The definition keeps the tool's behaviour with `missing: "ephemeral"`, the rules `missingId` and `duplicateId`, the plugin function `laterHolder` and the `visible` of the styles `card`, `groupFrame` and `band`, so it states nothing untrue; a runtime that follows 11.5.4 alone will draw them.

### 4. DISL: findings by line

The tool lists every finding by the line of its entry, whatever its code, ties in the order they arise (`findings-order.supply`). DISL orders findings by reader, built-in and rule, and `constraints.order` by the position of the code and only then by line. The definition declares the rules in the order the tool's findings arise on one entry; across entries the standalone module sorts the derived list by line.

### 9. DISL: the reader's sentences

`supply-chain.unreadable` carries the reader's sentences: "The document does not begin with `supply-chain: 1`.", "The version `one` is not a number.", "This document states version 2; this module reads version 1.", "`colour` is not a key this module reads on a group; the line is kept.", "`quarry` is not a stage this module knows.", "`quantity: lots` is not a number and was ignored.", "A node entry is not a mapping and was passed over." and, for a body that is not YAML, "The document could not be read as YAML: " followed by the parser's own message (`While parsing a flow sequence, did not find expected ',' or ']'.`), which no second host will word the same. Only the entry that is not a mapping is kept and not drawn, which is what `std.unreadableEntry` means; the definition sets `std.unreadableEntry` and `std.unparseable` to give `supply-chain.unreadable` as a warning, and the sentences are the reader's, handed in as reader findings.

## Definition defects repaired

The draft of this definition was written against DISL 0.1, before the standalone conversion; these are the defects the conversion's findings named, and how each was repaired.

- **A1.** The draft was DISL 0.1; it is 0.3.
- **A2.** Each 0.1 workaround is replaced by its 0.3 construct: the trace reads the viewer's selection (`view.selected`, 12.2) and walks the flows with `reachable` in CEL, where the draft called a plugin function `traceOf`; a card's anchors are restricted to its left and right sides (`AnchorSpec.sides`); the bezier's reach is stated (`BezierSpec`); removing a node asks first only when it has a flow, naming the count (`confirm` with `count` and `threshold`); the empty canvas has a menu (`diagram` in `contextMenus[].for`), so Arrange is offered there too; and the palette's tools are dropped (`mode: "drop"`, `drop.elsewhere: "create"`) where the draft could only click.
- **A3.** `persistence` is `format: "fbl"` with `binding` and a `typeMap`, in place of a plugin with `newline`, `files`, `ordering`, `omitDefaults`, `precision` and `definition`; the ids are `uuid-v4` in `base36`, which is the module's ShortGuid, in place of `nanoid`, "the nearest shape"; and `view.store` is empty, because positions are the body's `x` and `y`.
- **A4.** The wire ids are under `x-supply-chain`, as ruled for every converted tool (Peter, 2026-10-06, "Spec all but wire ids"): the 8 toolbox ids, the 15 action ids and the 9 property ids. The draft's `x-supplyChain` keys, which held each type's document spelling and older words, are gone: the binding's `map` holds them.
- **A5.** The seven stages are one node type, `Stage`, with a `stage` attribute whose enumeration `StageKind` carries each stage's colour, by Q2's default; a change of stage is a changed value, as the tool writes it, where the draft retyped. Each of the seven tools creates the one type with `initial` setting the stage, a quantity of zero and the name, because a `create` hook is not run; the draft named a new node in such a hook. The Stage row offers the seven words, as the tool does, rather than the enumeration's labels.
- **A6.** Membership is the reference attribute `group`, by Q5's default (gap 2), where the draft stated containment.
- **A11, B3, B4.** The codes are the tool's: every finding compares what is written, through the plugin functions `written` and `laterHolder`, as the hype cycle graph and the timeline do. A dangling flow is reported by rules over its written ends rather than by `std.references`, which would report a flow from a missing node to itself twice more (ruled 2026-10-11); `std.duplicateId`, `std.references`, `std.endpoints` and `std.facets` are switched off, so that no finding appears that the tool does not give (Q4).

## The examples

Two examples under `examples/` (also under the standalone `src/examples/diagrams/supply-chain/`, without their readmes), each written for ADP, because the notation is the repository's own and no published corpus of `.supply` documents exists to vendor. Their figures are illustrative orders of magnitude, not market data.

- **`examples/automotive/automotive.supply`**: a European maker of electric and combustion cars, from lithium, cobalt, nickel, copper, iron ore and rubber to private buyers and company fleets: 7 groups, 24 nodes in all seven stages, 25 flows.
- **`examples/gpu-memory/gpu-memory.supply`**: the GPU and memory industry, from rare earths, gallium, quartz, neon and process chemicals through wafers, lithography, logic and memory fabs, HBM stacking and packaging to phones, laptops, graphics cards, servers and their markets: 8 groups, 26 nodes, 38 flows.

Neither demonstrates authored positions, a cycle, a node outside every group, an empty group, or a document that breaks the rules; the fixtures cover those.

## Sources

- The standalone module `src/diagrams/supply-chain/` in etalii.adp.ide.standalone, its parity transcript and fixtures.
- The standalone specification `supply-chain-disl-fbl` (`.spec-workflow/specs/supply-chain-disl-fbl/`): its findings A1 to A14 and B1 to B14, its open questions Q1 to Q7 with their defaults, and the amendments ruled on 2026-10-11.
- [DISL-specification.md](../../specifications/disl/DISL-specification.md) (0.3) and [FBL-specification.md](../../specifications/fbl/FBL-specification.md).
- Inspiration: the [yFiles supply chain showcase](https://www.yfiles.com/demos/showcase/supply-chain/).
