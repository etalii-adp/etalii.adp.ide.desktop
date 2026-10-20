# Functional decomposition graph

The companion to [functional-decomposition-graph.dis](functional-decomposition-graph.dis). The `.dis` specifies the diagram type in DISL 0.1 as far as DISL reaches; this page holds everything about the tool that DISL cannot express, each point tied to the code, specification or conversation that shows it.

The diagram type is implemented in the standalone IDE as the module `src/diagrams/functional-decomposition-graph/` of [etalii-adp/etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone), origin `etalii/functional-decomposition-graph`. Paths below without a repository are relative to that module folder; `standalone:` marks a path relative to the standalone repository root. Its specification there is `standalone:.spec-workflow/specs/functional-decomposition-graph/` (requirements, design and 19 tasks, all done).

## What the diagram is for

A functional decomposition graph breaks a product down from what the user sees to what the system does: UI elements contain UI elements, actions and data; actions own data and functions; data owns data and functions; functions own functions. An action can also *show* a UI element, which is how navigation is drawn without making it ownership.

- The notation is ADP's own, defined by the user of the repository rather than a standards body, which is why the origin is `etalii/` (`standalone:docs/tools.md`, row for `etalii/functional-decomposition-graph`).
- The Notion "Tools" database row: kind Diagram, family "Business process & workflow notation", focus areas "Software delivery" and "Humans and agents together", rarity "Unique to ADP", state Prototype.
- State: ⚗️ Prototype. Peter ruled on 2026-09-26 that it stays a prototype ("no, keep it as is as it is still a prototype"). The `.dis` records this under `x-adp.state`, which is catalogue data rather than DISL.

## Mapping from the `.dis` to the implementation

| `.dis` name | Standalone id | Where |
| --- | --- | --- |
| `UiElement` | `ui-element` | `backend/EtAlii.Adp.Diagram.FunctionalDecompositionGraph/_Model/FdgElement.cs` |
| `DataElement` | `data-element` | same |
| `Action` | `action` | same |
| `Function` | `function` | same |
| `Comment` | `comment` | same |
| `UiChild` | `ui-child` | `backend/…/FdgRelations.cs` |
| `OwnsAction` | `owns-action` | same |
| `OwnsData` | `owns-data` | same |
| `OwnsFunction` | `owns-function` | same |
| `Shows` | `shows` | same |

The abstract types `FdgElement`, `NamedElement`, `FdgRelation` and `Owns` exist only in the `.dis`, to share attributes and to state acyclicity once; the implementation has no such types. Each concrete type carries its document spelling in `x-fdg.documentType`.

## The file format (`.fdg`)

DISL's persistence layer cannot describe this format, so the `.dis` names it `plugin:net.etalii.adp.etalii.fdg` and declares a `net.etalii.adp.etalii.fdg` plugin that provides `persistenceFormat`. The format itself, from `backend/…/FdgParser.cs`, `FdgWriter.cs`, `FdgDocumentFactory.cs` and the design document:

- **Shape.** A line-oriented YAML subset: a header line `functional-decomposition-graph: 1`, then an `elements:` list and a `connections:` list. It is ADP's own schema, not DID.
- **Element keys:** `id`, `type`, `name`, `description`, `text`, `x`, `y`, `width`, `height`. **Connection keys:** `id`, `type`, `from`, `to`, `name`, `description`.
- **Position is the top-left corner** (`x`, `y`), while the canvas and the wire work with the centre. The mapper converts (`backend/…/FdgElementMapper.cs`); moves go through `moveElementTo` with the top-left.
- **Height is stored only for a Comment.** The four named types share one height of 48 (`_Model/FdgGeometry.cs:22`), so it is never written for them. DISL's `size` can fix the height at 48, but it cannot say "do not persist this".
- **A Comment has `text`, never `name`.** Its text is written as a `|-` block scalar. A `name` on a Comment is reported as `fdg.unreadable-entry`.
- **Numbers** are written with invariant culture in the format `0.####` (`FdgWriter.cs:323`), so four decimals at most; the `.dis` says `precision.canvas: 4`, which is the nearest DISL has.
- **An empty description removes the key** rather than writing an empty value.
- **The empty document** is the header plus `elements: []` and `connections: []`, written with CRLF because a new file has no style to preserve and CRLF is the repository's house style (`FdgDocumentFactory.cs:34-40`).
- **Round trip.** An unchanged document comes back byte-identical, and every edit is a line splice (`LineSplice`) that leaves the rest of the file, its line endings and its comments untouched. The existing line ending of a file is kept. Because the bytes are the test subject, standalone's `.gitattributes` marks `*.fdg -text` (line 115). DISL's `newline: "crlf"` in the `.dis` describes only new files.
- **Tolerance.** The parser never throws. Unknown keys and unknown types survive a round trip and are reported; a version other than 1 is reported but the document still opens; an unreadable entry is reported and kept. Fixtures: `backend/…Tests/Fixtures/malformed-entries.fdg`, `not-yaml.fdg`, `no-trailing-newline.fdg`, `crlf-line-endings.fdg`, `lf-line-endings.fdg`.
- **Ids.** New ids are `ShortGuid`s (`Commands/AddFdgElementCommandHandler.cs:32`), not UUID v4 strings; hand-authored ids may be any token. The `.dis` says `uuid-v4` because DISL has no other generator.
- **Removal order.** Removing an element also removes every connection to or from it, spliced out bottom-up so earlier line numbers stay valid.
- **Registration.** A diagram is added to an ADP project by a `.adp` file whose origin line is `etalii/functional-decomposition-graph` and whose body is `body: <name>.fdg` (`examples/field-service/field-service.adp`).

## Interaction

- **Connecting is a right-button drag from one element's body to another's** (`connectOnRightDrag`, `client/FdgCanvas.tsx`), not a palette link tool. The relation is inferred from the pair: the canvas library's `relationOnto` picks the one relation whose ends admit both elements, and there is exactly one for every allowed pair because each relation is identified by its target type. DISL's toolbox can only offer a relation per tool, so the `.dis` has a Links group with five drag tools that standalone does not show.
- **The client readme is stale here.** `client/readme.md:59-64`, "What is not settled yet", says only one of Shows and the ownership relations can be reached from an Action. That was resolved by `relationOnto`; the readme was not updated.
- **Adding** is a drop from the toolbox onto the canvas, placing the element's centre where it is dropped (action `fdg.add.<type>` with target `new:x,y`). DISL's toolbox has click and drag modes but no drop-from-palette mode; the `.dis` uses click.
- **New names are unique.** "New UI element", "New action", "New data element", "New function", then the same with " 2", " 3" and so on (`Commands/AddFdgElementCommandHandler.cs:66`). A new Comment's text is "New comment" (line 53). The `.dis` approximates this with the `uniqueName` function and a create hook.
- **Rename** is F2 or a double-click (`fdg.rename`); on a Comment the same gesture is "Edit text…".
- **Delete** is the Delete key (`fdg.remove` for an element, `fdg.disconnect` for a connection). Removing an element asks for confirmation **only when it has connections**, naming how many: "Removing this element also removes the N connections to or from it." (`backend/…/FdgContextActionProvider.cs:195-196`). DISL's deletion policy can say `confirm`, but not conditionally or with a count.
- **Properties** (`backend/…/FdgContextPropertyProvider.cs`): `fdg.name`, `fdg.text` (Comment), `fdg.description`, `fdg.width`, `fdg.height` (Comment) and `fdg.connection-name`. Width and Height are rows in the property grid; DISL forms cover attributes, not geometry.
- **Resize.** The named types resize horizontally only; a Comment resizes both ways. A size below the minimum (width 80, Comment height 48, `_Model/FdgGeometry.cs:25-28`) is clamped, not refused. A resize from the left or top edge is two edits, a size and then a move, so it undoes in two steps.
- **Undo** restores the whole document text (`RestoreDocumentCommand<IFdgDocumentStore>`, registered in `ServiceCollection.AddFunctionalDecompositionGraph.cs:55`), rather than inverting a model edit.
- **Descriptions are never sent to the canvas.** The proto (`api/functional-decomposition-graph.proto`) has no description field; descriptions live in the file and the property grid only.

## Rules and refusals

The `.dis` states the rules as endpoints, multiplicities, `acyclic` on the abstract `Owns` and an invariant; the implementation adds the following.

- **The backend re-runs the connect verdict** the canvas already ran (`backend/…/FdgConnectVerdict.cs`, called from `Commands/ConnectFdgElementsCommandHandler.cs:25`), in a fixed order: type, then cardinality, then cycle. Each refusal is a sentence shown to the person, naming which check failed:
  - "Refused by the cardinality check: `X` already has its `owns-data` parent, and may have 1." (line 54, and line 60 for Shows).
  - "Refused by the cycle check: `X` already owns `Y`, so this link would make an ownership loop." (line 66).
  - "That id is already used in this graph." for a duplicate connection id.
- **Rule ids** for a document that breaks the rules, typically because it was edited outside ADP (`backend/…/FdgRuleSet.cs:13-20`): `fdg.forbidden-link`, `fdg.self-link`, `fdg.second-parent`, `fdg.second-shows`, `fdg.ownership-cycle`, `fdg.dangling-reference`, `fdg.duplicate-id`, `fdg.unreadable-entry`. Each has a fixture `backend/…Tests/Fixtures/rule-*.fdg`, and `rules-clean.fdg` has none.
- **Breaches reach the Errors and Warnings panel** through `FdgValidator`, the module's `IDiagramValidator`, registered in `ServiceCollection.AddFunctionalDecompositionGraph.cs` (standalone PR #109, 2026-09-30). `fdg.unreadable-entry` is a warning, because the lines are kept and the document still draws; every other rule id is an error. Each problem is located by its line, counted from 1.
- **The four ownership relations are acyclic together; Shows is not.** An action may show the UI element that owns it. The `.dis` expresses this with `acyclic: true` on the abstract `Owns`; DISL does not say explicitly whether `acyclic` on an abstract relation type covers the union of its subtypes, so the `.dis` also carries the `ownershipLoop` invariant to state it unambiguously.

## Notation details DISL approximates

- **Superellipse (UI element).** Standalone draws a superellipse with exponent n=4, sampled at 64 points (`standalone:src/client/src/canvas/library/shapes/outline.ts`). DISL has no superellipse, so the `.dis` draws it with four cubic curves (k=0.919), a maximum radial error of 0.46%.
- **Diode (function).** The corner radius is `min(h/2, w)` and the arc is sampled at 24 points. The `.dis` uses a custom shape whose radius is a GeomExpr with `min()`.
- **Parallelogram (data)** has corners (0.2,0), (1,0), (0.8,1), (0,1); **trapezoid (action)** has (0,0), (1,0), (0.85,1), (0.15,1). Both are exact in the `.dis`.
- **Text regions.** Standalone computes the text area as the band inside each outline; the `.dis` gives each custom shape a fixed `textArea`, which is close but not identical for narrow elements.
- **Colours** are CSS variables in `standalone:src/client/src/index.css` and `client/fdg.css`, switched by the app's theme. The `.dis` carries them as light and dark theme tokens. The fills were chosen for text contrast against `--color-text` in both themes (design document, notation section); DISL has no way to state a contrast requirement.
- **Connections** are cubic béziers with a filled triangle at the target and the name at the midpoint, as in the `.dis`. No snapping and no automatic layout: placement is free and manual.

## Known gaps

- **The client readme's "What is not settled yet"** is stale, see *Interaction*.
- **DISL gaps this diagram exposed:** no drop-from-palette toolbox mode; no inferred relation for a body-to-body drag; no superellipse or rounded-end built-in; no conditional deletion confirmation; no way to mark a fixed attribute as not persisted; no explicit statement of how `acyclic` on an abstract relation type applies to its subtypes.

## The example

`examples/field-service/field-service.fdg` is a field-service app decomposed from its screens down to functions, with two Comments and two Shows links. Those links form a navigation round trip (Task list, Open task, Task detail, Back to list, Task list), a loop that is legal because it closes only through Shows. Its readme (`examples/field-service/readme.md`, "What it does not demonstrate") lists what it leaves out: a document that breaks the rules (the `rule-*.fdg` fixtures cover that), a second Shows from one Action, base36 ids, an empty name, a Data element owned by an Action or a Function owned by a UI element, a description on a Comment, and an Action showing its own page.

## Sources

- Module code and tests: `src/diagrams/functional-decomposition-graph/` in etalii.adp.ide.standalone.
- Specification: `.spec-workflow/specs/functional-decomposition-graph/requirements.md` and `design.md` in etalii.adp.ide.standalone.
- Catalogue row: `docs/tools.md` in etalii.adp.ide.standalone.
- Canvas library: `src/client/src/canvas/library/` and `src/client/src/canvas/DiagramCanvas.tsx` in etalii.adp.ide.standalone.
- Notion: the "Tools" database row for Functional decomposition graph.
- Conversation: Peter's ruling of 2026-09-26 that the tool stays a Prototype.
