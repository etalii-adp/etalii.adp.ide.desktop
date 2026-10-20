# SKOS concept scheme: what DISL cannot express

This file accompanies [w3c-skos.dis](w3c-skos.dis), the DISL specification of the SKOS concept scheme diagram (`w3c/skos`, a SKOS vocabulary stored as Turtle `.ttl` or N-Triples `.nt`). The `.dis` says what DISL can say: the three node types and three relation types, the label chooser as CEL functions, the notation, the one toolbox entry, the gestures, the findings, and where layout and persistence hand over to plugins. This file holds everything else: how triples become elements, the exact tie-breaks, the registration header, what each edit writes into the vocabulary file, the layout algorithm, the budget, the examples, and where the prototype falls short of its own specification. Each point names the code, specification or conversation that shows it.

The tool is one of four readings over one shared RDF engine in standalone's `src/diagrams/rdf/` module. The engine itself (parser, store, writer, splice discipline, line endings, the drawn-element budget) is described in [w3c-rdf.md](w3c-rdf.md); the sibling readings are [w3c-owl.dis](w3c-owl.dis) and [w3c-shacl.dis](w3c-shacl.dis). This file repeats the engine only where SKOS uses it differently.

Sources, as read on 2026-09-30:

- The standalone implementation on `develop` of etalii.adp.ide.standalone (commit `b2a2692`): `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/Skos/` (abbreviated `Skos/` below), the family files beside it (`Diagram.cs`, `RdfContextActionProvider.cs`, `RdfContextPropertyProvider.cs`, `RdfSelection.cs`, `Commands/RdfEdits.cs`), `src/diagrams/rdf/api/skos.proto`, the client `src/diagrams/rdf/client/SkosCanvas.tsx`, `skos.css` and `skosModel.ts`, the tests in `EtAlii.Adp.Diagram.Rdf.Tests/`, the examples in `src/diagrams/rdf/examples/stw/`, `docs/tools.md` line 139 and `docs/screenshots/readme.md`.
- The spec-workflow specification `skos-diagram` (requirements, design, tasks), removed from the tree by commit `53f68611` and read from its parent. Requirement numbers below (R1.1 and so on) are that specification's. It consumes the family specification `rdf-diagram` by named pieces ("the triplestore document store", "the splice discipline", "the drawn-element budget" and so on).
- The Notion "Tools" database row "SKOS concept scheme" (properties only; the page body is blank).
- This project's conversations from 2026-09-26 to 2026-09-30. Peter never ruled on SKOS behaviour itself. The one SKOS-specific item is the screenshot session of 2026-09-28 (section 11); the rulings that apply are general ones: the `.dis` extension (Peter, 2026-09-30), one DISL file per standalone tool in `definitions/diagrams/`, and definition-driven tools with per-host plugin code for what a definition cannot cover (Peter, 2026-09-26).

## 1. What the tool is for

A SKOS concept scheme diagram draws a thesaurus, taxonomy or controlled vocabulary as what it states: schemes heading their concepts, broader and narrower as a top-down hierarchy, `skos:related` and mapping links across it. Standalone's description: "A thesaurus or controlled vocabulary: concepts under their schemes, broader and narrower drawn as a hierarchy, related links across it." (`Diagram.cs:59`).

Catalogue data (Notion "Tools" row, `docs/tools.md:139`): kind Diagram, origin `w3c/skos`, family "Ontologies & semantic web", extension `.ttl` (and `.nt`), rarity "Adoption from standard", theory the W3C SKOS reference, example SKOS Play, state ⚗️ Prototype in Standalone ("Implemented as a working prototype; usable, not yet hardened to full quality", the `docs/tools.md` legend) and not yet in IntelliJ, VS Code or Eclipse. Focus areas: Knowledge and semantics, Clarity in textual data. One-line purpose: "Browse and edit a SKOS vocabulary as a hierarchy of concepts with their labels and relations." Why specialized: "A thesaurus is a hierarchy with cross-links; a specialized view shows the broader and narrower trees and related links that a flat file hides." Standalone's icon is `mdi-file-tree`; the `.dis` writes it `mdi:file-tree`.

The research behind the drawing rules (old requirements, "How SKOS schemes are visualized"):

- **The tradition surveyed:** Skosmos presents a hierarchy tree plus an alphabetical index, multilingual; SKOS Play renders alphabetical and hierarchical indexes and D3 tree, partition and sunburst views; VocBench edits in a tree with a graph only as a secondary view; print thesauri (the ISO 25964 heritage) repeat a polyhierarchical concept under each parent. The verdict: "a vocabulary is read as a hierarchy, not explored as a graph."
- **Adopted:** a top-down layered hierarchy, drawn as a DAG, so a concept with several broader concepts is drawn once with one hierarchy edge per parent (repetition "would break selection, positions and editing, which all need one element per concept"); `skos:related` as a dashed cross-link that never influences layering; labels as the content, chosen by an explicit, deterministic language rule, with the IRI one selection away in the property grid.
- **Rejected:** repeating polyhierarchical concepts per parent; sunburst, partition and treemap overviews (no stable per-concept addressability); a built-in graph mode (a `w3c/rdf` registration beside this one *is* the graph view, "two `.adp` files instead of a mode switch"); alphabetical-index panes; any inference; dereferencing mapping IRIs over the network; stub nodes for concepts outside the file; concept nodes as VOWL circles (the design deliberately draws rectangles).

## 2. The vocabulary file and the shared engine

DISL's persistence layer describes a file a runtime writes. Here the model file is a foreign RDF serialization that other tools own, which DISL can only name as `format: "plugin:net.etalii.adp.w3c.turtle"`. SKOS adds no parser, writer or store of its own (old design, "Code Reuse Analysis"): it reuses the family's `IRdfDocumentStore`, `RdfParser`, `RdfModel`, `RdfWriter`, the `RdfEdits` command pattern and the family commands, `RdfValidator`, `RdfDocumentReloader`, the context provider trio, `RdfSelection`, `RdfViewport`, and core's `RegistrationLayout` and `SetRegistrationLayoutCommand` (`Skos/ServiceCollection.AddSkos.cs:18-40`). What that means for SKOS:

- Several readings over one file share one store entry and one undo history (`BothReadings_ShareOneStoreEntry_ForOneFile`, `SkosSession.Tests.cs:123`; R2.5). Opening the same file as `w3c/rdf` and `w3c/skos` side by side is the intended way to get the graph view.
- Every edit goes through `RdfEdits.Run`: it refuses when the file does not parse ("This file does not parse, so nothing can be edited until it is fixed."), captures the text, applies the writer's splices, saves, and keeps the captured bytes as the inverse (`RestoreDocumentCommand`), so undo is byte-exact, including for LF files (`Commands/RdfEdits.cs:19-43`; `SkosActions.Tests.cs:88, 111, 137, 163`).
- External changes to the file reach an open diagram through the family reloader registered for this origin (`ServiceCollection.AddSkos.cs:35-37`). The session listens to the store's `Changed` event, drops its cached layout, re-renders and sends only the differences (`SkosSession.cs:67, 226-249`).
- Nothing of ADP's is ever written into the SKOS file (`SkosSession.cs:110-113`; old design "Diagram storage").

**Routing.** A bare `.ttl` or `.nt` file always opens as the anchor `w3c/rdf`; `w3c/skos` never claims one. It is chosen only through Add, where it is suggested when the file's text contains the literal `skos:ConceptScheme` or the full IRI `http://www.w3.org/2004/02/skos/core#ConceptScheme` (`Diagram.cs:50-69`; R2.2, R2.3; test `TheDefinition_NeverClaimsABareBody_AndDeclaresTheFamilyExtensions`, `SkosSession.Tests.cs:61`). The test is textual on purpose: "it judges whether the reading is worth offering, not whether the file is good SKOS - the parse decides that once the reading opens. A file with concepts but no scheme stays registrable by explicit choice" (`Diagram.cs:64-67`). The `.dis` records these as `x-adp.claimsBareFiles` and `x-adp.suggestWhenBodyContains`, which no DISL runtime is required to read.

**The new-document template** (`Skos/SkosDocumentFactory.cs:17-46`), written with CRLF line endings, where `{local}` is the base name with every character outside ASCII letters, digits, `-` and `_` replaced by `_`, trimmed of `_`, or `untitled` when nothing is left:

```turtle
@prefix skos: <http://www.w3.org/2004/02/skos/core#> .
@prefix ex: <http://example.org/> .

ex:{local} a skos:ConceptScheme ;
    skos:prefLabel "{baseName}"@en ;
    skos:hasTopConcept ex:{local}_top .

ex:{local}_top a skos:Concept ;
    skos:prefLabel "Top concept"@en ;
    skos:topConceptOf ex:{local} .
```

Its purpose is "one scheme, one top concept with a preferred label, parsing and validating clean (nothing above info)" (lines 7-9). `{baseName}` is inserted into the literal unescaped (line 26), so a base name containing `"` produces a file that does not parse. DISL's `templates` describe element templates, not the bytes of a new foreign file, so the template lives here.

## 3. The registration, its `language:` header, and where positions live

The registration is a `.adp` file beside the vocabulary:

```
w3c/skos
body: business-economics.ttl
language: de
```

The `language:` header is this reading's own and sets the display language labels are chosen in. `SkosRegistrationLanguage.Read` (`Skos/SkosRegistrationLanguage.cs:34-93`) reads it as follows:

- It scans at most 8 lines after the MIME line, which is core's own header-region limit (line 26), skipping blank lines, and stops at `layout:` (case-insensitive).
- The first `language:` line wins; the value is trimmed and lower-cased, and an empty value counts as absent.
- A `body:` or `view:` line seen *after* a `language:` line marks the header as misplaced. Core's pairing scan stops at the first line it does not recognise, so a `language:` above `body:` would sever the body pairing; such a header is "ignored and reported, never honoured and never guessed at" (lines 10-16), and its 1-based line number (the MIME line is line 1) feeds the `skos.misplaced-header` finding. The placement rule for every family header, after `body:`/`view:` and before `layout:`, is documented in `RdfRegistrationHeaders.cs:9-14`.
- Other keys are ignored as "another reading's header or prose - not ours to interpret" (line 82).
- The file is read through `SharedDocumentReader.OpenText`, so the read cannot contend with a rename rewriting the registration in place (lines 17-18, 47); an IO or permission error reads as no header.
- Without a header the display language is `en` (`SkosSessionFactory.cs:42-47`, `SkosLabels.cs:12`).

Tests: `SkosRegistrationLanguage.Tests.cs:35, 45, 59, 70, 80, 88`. DISL has diagram attributes but no notion of a header line in a sidecar file, so the `.dis` exposes the result as the read-only diagram attributes `language` and `languageHeaderLine`, and names the header under `x-adp.registrationHeaders`.

**Positions** are stored only in the registration's `layout:` block, keyed by element id (`res:{iri}`), and overlay the computed layout element by element (`RegistrationLayout.Read` and `Apply`, `SkosSession.cs:159-183`; R4.3). A move writes `SetRegistrationLayoutCommand(registrationPath, elementId, x, y)` and never touches the vocabulary (`SkosSession.cs:114-144`). A move is refused with, in order:

- no history: "This diagram is read-only."
- no registration: "This diagram was opened without a registration, so there is nowhere to store a position. Register the file to arrange it."
- an edge or the banner: "That element is not something this diagram can move."
- a blank node: "That is a blank node, whose identity does not survive a reparse, so a stored position could not be trusted. Name it with an IRI to arrange it." (`SkosSession.cs:119-137`; `SkosSession.Tests.cs:140`)

A drag that would change a concept's parent is refused with "A concept is filed under a parent with the hierarchy gesture; dragging changes where it sits on the canvas." (`SkosSession.cs:100-108`). The `.dis` carries the blank-node and reparent refusals as placement constraints; the other three depend on whether a registration exists, which DISL cannot see.

## 4. From triples to drawn elements

The projection is a pure function over the parsed triples: no parsing of its own, no completion, nothing written (`Skos/SkosProjection.cs:3-23`; R1). Everything is by assertion: a concept is whatever the file types `skos:Concept`, and typing is read only from `rdf:type` triples with an IRI object (`SkosProjection.cs:9-10, 289-295`). DISL's metamodel describes elements a runtime stores; it has no way to say "an element exists because a triple of this shape exists", so the whole mapping below is the plugin's.

**Element ids** (`SkosProjection.cs:277-282`; `_Model/SkosEdge.cs:22`): an IRI term is `res:{iri}`, a blank term `blank:{ordinal}` (its position in the parse, so not stable across edits), an edge `edge:{fromId}|{predicateIri}|{toId}`, the banner `truncation`. Literals are never elements.

**Wire types** (`Skos/SkosElementMapper.cs:14-26`; `skos.proto`, package `etalii.adp.skos`): `w3c/skos+concept`, `w3c/skos+scheme`, `w3c/skos+collection`, `w3c/skos+edge`, `w3c/skos+truncation`. Payloads travel as protobuf `Any`. The label, its tag, its kind and the language chip are computed in the backend; the canvas never recomputes a name (`SkosElementMapper.cs:7-9`).

**Schemes.** Every subject of `rdf:type skos:ConceptScheme`. Its top concepts are the objects of its `skos:hasTopConcept` plus the subjects of `skos:topConceptOf` pointing at it, distinct, in document order (`SkosProjection.cs:77-85, 265-266`). Its member count is the number of drawn concepts filed in it (`SkosElementMapper.cs:49`).

**Concepts.** Every subject, IRI or blank, of `rdf:type skos:Concept` (`SkosProjection.cs:32`). Scheme membership is the union of `skos:inScheme` objects, `skos:topConceptOf` objects and the subjects of a scheme's `skos:hasTopConcept`, restricted to IRIs the file types as schemes, distinct, in document order; an empty set means the unfiled band (`SkosProjection.cs:73-85, 214`). Notations are every `skos:notation` literal in document order; the canvas badges the first (`SkosProjection.cs:69-71`).

**Collections.** Every subject of `rdf:type skos:Collection` or `skos:OrderedCollection` (`SkosProjection.cs:34-36`). Members are the objects of `skos:member` for an unordered collection (ignored on an ordered one), and for `skos:memberList` the RDF list is walked cell by cell (`rdf:first`/`rdf:rest`) in list order (`SkosProjection.cs:102-124, 297-320`); the list is walked for any subject, not only ordered collections. Members are limited to elements kept under the budget (line 223). Collections are "never conflated with the hierarchy, per SKOS S13's disjoint families" (`_Model/SkosCollection.cs:4-5`; R1.5).

**Hierarchy edges.** `A skos:broader B` and `B skos:narrower A` state the same pair; the projection draws one edge per (broader, narrower) pair however many triples state it, in whichever directions, and keeps every stating triple on the edge (`SkosProjection.cs:87-95, 227-237`). The edge id always names the `skos:broader` direction, `edge:{narrowerId}|http://www.w3.org/2004/02/skos/core#broader|{broaderId}`, whichever triple actually exists. `assertedBothWays` is true when the pair is stated by more than one distinct predicate or subject (`SkosProjection.cs:333-348`). The inverse is never synthesised, never handed to the store and never written (`SkosProjection.cs:12-15`; test `EitherAssertedDirection_IsOneEdge_AndNothingIsCompleted`, `SkosProjection.Tests.cs:24`). An edge is emitted only when both ends are drawn elements (line 229). A polyhierarchical concept is drawn once with one edge per parent (R1.6; `SkosProjection.Tests.cs:140`).

**Related edges.** `skos:related` in either direction gives one undirected edge per pair; the id orders the two ends ordinally, `edge:{a}|http://www.w3.org/2004/02/skos/core#related|{b}` with a before b (`SkosProjection.cs:97-100, 239-246`). They take no part in layering.

**Mapping edges.** `skos:exactMatch`, `closeMatch`, `broadMatch`, `narrowMatch` and `relatedMatch` are drawn only when both subject and object are typed `skos:Concept` in this file, one edge per triple, subject to object, labelled with the predicate's local name (`SkosProjection.cs:126-141`; `SkosElementMapper.cs:92, 112-122`). Any other mapping triple, with a literal object or an IRI outside the file, becomes a property-grid row, "never a stub node" (R1.3; `SkosProjection.Tests.cs:80`). `broadMatch` and `narrowMatch` carry no hierarchy meaning in the drawing.

**Never drawn:** the documentation properties (grid only), notations beyond the first, out-of-file mappings, SKOS-XL label resources (detected, never resolved, R3.6), membership as edges (scheme membership is expressed only by placement; collection membership not at all, section 11), any resource not typed as one of the four classes, and the transitive properties `skos:broaderTransitive`, `narrowerTransitive` and `semanticRelation`, which the reading does not know (`Skos/SkosVocabulary.cs`).

### The label chooser, exactly

The `.dis` states the chooser as the CEL functions `textsIn`, `tagFor`, `chooseLabel` and `chosenTag`. The reference is `SkosLabels.Choose` (`Skos/SkosLabels.cs:12, 24-66`):

1. Preferred labels first. Within them, the display language, then `en`, then untagged, then the ordinally smallest remaining tag. Within one tag, the ordinally smallest literal: "the deliberate, arbitrary-but-stable S14 tie-break".
2. Only when there is no `skos:prefLabel` in *any* language, the same order over `skos:altLabel`, and the name is marked alternate. A preferred label in any language beats every alternate label (`SkosLabels.Tests.cs:32-33`).
3. Otherwise the IRI fallback: the part of the IRI after its last `#` or `/` (the whole IRI when there is none, or when it ends in one), and an empty name for a blank concept (`SkosElementMapper.cs:109-122`).

Hidden labels never display. Language tags are compared lower-cased (`SkosProjection.cs:65`). The language chip shows exactly when the chosen tag is non-empty and differs from the display language; untagged labels wear none (`_Model/SkosChosenLabel.cs:16-21`; `SkosLabels.Tests.cs:69-80`). Only concepts carry the chip; schemes and collections have a tag but no chip field (`skos.proto:44-60`).

Where the CEL approximation differs: the DISL `sort()` over strings is assumed ordinal, as .NET's `StringComparer.Ordinal` is; a runtime whose CEL sorts by locale would pick a different label among equals. The validator's messages always name concepts with the chooser run in `en`, not the display language (`SkosValidator.cs:149-158`).

### The budget

The family budget is 1000 drawn elements (`RdfProjection.DefaultBudget`, `RdfProjection.cs:24`), counted here over schemes, concepts and collections (`_Model/SkosProjectionResult.cs:14-26`; R8.1). The order in which elements are kept is hierarchy-aware (`SkosProjection.cs:147-203`; R8.2; `SkosProjection.Tests.cs:118`): schemes by IRI; per scheme, the scheme, then breadth-first layers from its asserted top concepts plus its members with no broader inside the scheme, each layer in ordinal order, then its remaining members by id; then every unfiled concept by id; then collections by id. DISL's `limits.maxElements` states the number but not which elements survive, nor that edits are withheld on a truncated view. How the live session actually applies the budget differs from this (section 10, item 10).

## 5. Layout

The `.dis` names the layout `plugin:net.etalii.adp.w3c.skosBands` with a layered fallback. The plugin is `SkosLayout.Layout` (`Skos/SkosLayout.cs:21-247`): pure and deterministic, no physics and no randomness (R4.1). Constants: column width 240, row height 110, region gap 90, region header 60. Coordinates are the top-left of the element, y downward.

1. Only hierarchy edges count. Cycles are broken first (below), then each edge gives a broader-to-narrower child link.
2. Starting at y = 0, each scheme in ordinal IRI order is placed at (0, y), and its band is every drawn concept filed in it that is not yet placed, so a concept filed in several schemes lands in the first by IRI. The band starts 60 below the scheme and the next scheme starts 90 below the band.
3. Every concept still unplaced forms the unfiled band, with no header offset.
4. Collections go in one row below everything, x = column × 240, in IRI order. They are not placed around their members.
5. Inside a band (`PlaceBand`, lines 75-131): layer 0 is every member with no broader inside the band, in ordinal order; each following layer is the children of the previous one, and a child visited again from a deeper parent is moved down, so every concept sits one row below its deepest broader (longest-path depth). The loop stops after as many rounds as the band has members, a guard against a knot the cycle breaking did not unwind. Each depth is one row, filled left to right in ordinal id order at x = i × 240, y = top + depth × 110; members no layer reached go in one extra row below. Rows are left-aligned, not centred under their parents, there is no crossing minimisation, and a row is as wide as its layer.
6. **Cycle breaking** (`BreakCycles` and `FindKnot`, lines 139-247): repeatedly find a self-loop, or else the first strongly connected component of more than one concept (iterative Tarjan, starting from concepts in ordinal order); exclude from layering the knot's edge with the ordinally smallest (broader id, narrower id) pair; record the cycle as its sorted concept ids, the excluded edge and every triple of the knot's edges; repeat until no knot remains (R4.2; `SkosLayout.Tests.cs:65`). Excluded edges are still drawn. The validator consumes these recorded cycles rather than running a second detector, which DISL's `inCycle()` in the `.dis` approximates per concept.
7. The session lays out the unbudgeted projection so every concept has a stable position, then overlays authored positions from the registration (`SkosSession.cs:159-183`). The client uses the positions verbatim as top-left corners and converts them to centres, with the canvas in manual layout mode (`SkosCanvas.tsx:225, 247-263`).

Asserted top concepts get no special place: layer 0 is simply "no broader inside the band", although R4.1 says top concepts sit on the first row (only the budget order uses asserted tops). Tests: `SkosLayout.Tests.cs:26, 48, 93`; `SkosSession.Tests.cs:308`.

The DISL `layered` algorithm with `respect: all` is the closest built-in, but it cannot state per-scheme bands, first-scheme-wins placement, the trailing collections row or the cycle-breaking tie-break.

## 6. Interaction

**Toolbox** (`Skos/SkosToolboxProvider.cs:15-23`): one entry, id `skos.toolbox.concept`, label "Concept", icon `mdi-tag-plus-outline`, description "A concept, filed into the file's first scheme. Drop it where it should sit; you will be asked for its preferred label.", drop action `skos.add-concept`. "Schemes and collections are modeled deliberately, not dropped" (lines 5-7; R6.1). A drop calls the action with the placement id `new:{x},{y}` (`SkosCanvas.tsx:308-309`), but the coordinates are not used: the new concept lands where the layout puts it.

**Canvas gestures** (`SkosCanvas.tsx:83-89, 293-311`): a concept has two anchors, `file` at the top centre and `relate` at the right centre. A connection drawn from `file` runs `skos.file-under`, one from `relate` runs `skos.relate`; both send the relation gesture id `rel:` between the two element ids. Dragging a node moves it (section 3). The canvas declares `rename` (F2, on elements) and `delete` (Delete, on elements and connections), which the backend maps to the entries below by shortcut. Inline renaming on the canvas is exempt for all four family readings, because the drawn labels are derived: a concept's preferred label is changed as a property edit (`src/diagrams/rdf/client/readme.md`, "Inline renaming").

**Context actions.** SKOS offers its actions through the family's `RdfContextActionProvider`, which orders them with the family's own (`RdfContextActionProvider.cs:97-189`; `SkosActions.cs:18-70`):

| Target | Entries, in order |
| --- | --- |
| A file that does not parse, or a truncated view | none at all |
| A resource (`res:`) | the family's "Rename…" (`rdf.rename-resource`, `mdi-pencil-outline`, F2) and "Remove" or "Remove (with {n} statements)" (`rdf.remove-resource`, `mdi-delete-outline`, Delete) |
| A SKOS hierarchy or related edge with asserted triples | hierarchy: "Disconnect (both directions)" when more than one triple states it, else "Disconnect"; related: "Remove related link"; all `skos.disconnect`, `mdi-link-off`, Delete |
| Any other family edge (for example a mapping) | the family's "Remove statement" |
| A placement `new:x,y` | "Add concept here…" (`skos.add-concept`, `mdi-tag-plus-outline`) when the file asserts any scheme, then the family's "Add resource here…" and "Declare prefix…" (plus OWL's entries when the file is an ontology) |
| A relation gesture between two IRI concepts | "File under (broader)" (`skos.file-under`, `mdi-file-tree`) and "Relate (skos:related)" (`skos.relate`, `mdi-swap-horizontal`), then OWL's "Subclass of" when both ends are classes, then the family's "Relate…" |

Discovery reads the file's assertions, not the registration's origin: "the context seam deliberately does not carry" the registration (`SkosActions.cs:8-14`; `SkosSelection.cs:3-9`). The SKOS gestures, the "Add concept here…" entry and the SKOS property grid therefore also appear under the `w3c/rdf` and `w3c/owl` readings of a file that asserts SKOS types. DISL scopes tools to a specification, so this cross-reading offer cannot be written in any one `.dis`.

**Refusals and dialogs** (`SkosActions.cs:73-219`):

- A gesture whose ends are not both IRI concepts: "Both ends of this gesture must be concepts the file types as skos:Concept." Blank concepts are refused here even though the client lets a connection end on one (`SkosSelection.cs:13-17`; `SkosCanvas.tsx:182, 196`).
- A gesture onto itself: "A concept cannot be filed under or related to itself."
- Add concept opens an input dialog titled "Add concept", icon `mdi-tag-plus-outline`, field "Preferred label", button "Add". Its validation: an empty label gives "A concept needs a preferred label."; a file without any prefix gives "This file declares no prefix to mint the concept's IRI under. Declare one first (the family's prefix action)."; a label that slugs to nothing gives "The label leaves nothing usable for an IRI once slugged; use at least one letter or digit."; an IRI already used as a subject or object gives "{iri} already names something in this document. Choose a different label, or rename the existing resource first."
- Disconnecting a pair stated by more than one triple asks first: title "Disconnect", icon `mdi-link-off`, "This pair is asserted in both directions; disconnecting removes both statements as one undo.", button "Disconnect", not styled as dangerous. A single triple is removed without asking. If the pair is no longer in the file: "That connection is not in the file as it stands, so there is nothing to disconnect."
- Removing a resource that appears in more than one statement asks first: title "Remove resource", "Removing this resource also removes the {touching} statements it appears in.", button "Remove", styled as dangerous (`RdfContextActionProvider.cs:255-271`; R5.5).
- Rename opens "Rename resource" with the field "New IRI or prefixed name", prefilled with the compressed IRI (`RdfContextActionProvider.cs:250-253`; R5.4).
- On a truncated view every action answers: "The diagram shows only the first part of this file under the drawn-element budget, so edits through it are withheld - an edit through a partial view could touch what the view does not show. Edit the file as text instead." (`RdfSelection.cs:21-22`).

**IRI minting** (`SkosActions.MintIri`, lines 193-219), stated in the `.dis` only as the plugin function `rdfMintIri`: the namespace is the expansion of the prefix of the file's first prefixed-name subject, or else the first declared prefix's IRI; the local name is the trimmed label with every character outside `[A-Za-z0-9-_]` replaced by `_`, trimmed of `_`.

**Property grid** (`Skos/SkosProperties.cs:15-111`). It replaces the family grid for any element the file types as a scheme, concept or collection (`RdfContextPropertyProvider.cs:65-72`), in the groups Identity, Labels, Documentation, Membership and Mappings:

| Row id | Label | Value | Editable, or the read-only reason |
| --- | --- | --- | --- |
| `skos.iri` | IRI | the full IRI | "Rename through the context menu, so every reference follows the name." |
| `skos.notation` (only when present) | Notation | every notation, joined by ", " | "Notations are codes other systems key off; edit them as triples." |
| `skos.label:{kind}:{lang}`, one per literal | Preferred, Alternate or Hidden, plus " @{lang}" when tagged | the lexical form | preferred labels are editable; otherwise, in priority order, "This concept's labels are stated through SKOS-XL, which this reading does not resolve (Requirement 3.6). Edit them as triples in the graph reading.", the truncation refusal, or "Alternate and hidden labels are edited as triples." |
| `skos.doc:{property}`, always all seven | Definition, ScopeNote, Example, Note, HistoryNote, EditorialNote, ChangeNote | the first literal of that property in any language, or empty | editable unless truncated |
| `skos.schemes` (concepts only) | In schemes | the schemes' display names joined by ", ", or "(unfiled)" | "Membership is stated by triples; file and unfile concepts on the canvas." |
| `skos.mapping:{predicate}:{objectIri}` (concepts only, one per out-of-file mapping) | the predicate's local name, capitalised | the object IRI | "The mapped concept lives outside this file, so the mapping shows here rather than drawing." |

The Membership row reads only `skos:inScheme` and `skos:topConceptOf` from the concept's side and ignores a scheme's `skos:hasTopConcept`, so it can disagree with the projection's membership (lines 84-87). Several alternate or hidden labels in one language share one row id (line 63). DISL forms bind to attributes, so the `.dis` shows labels as tables of `LangString`; the per-literal rows, their ids and the priority of read-only reasons are the plugin's.

## 7. What each edit writes into the vocabulary

Every edit is one undoable command whose inverse restores the exact bytes it replaced (section 2). The writer reuses declared prefixes and splices in place; the splice rules themselves are the engine's (see [w3c-rdf.md](w3c-rdf.md)).

- **File under:** one triple, `<narrower> skos:broader <broader>`, where the concept the gesture started from becomes the narrower one. The inverse `skos:narrower` is never co-written (R5.1; `SkosActions.cs:97-103`; the test asserts `skos:broader ex:tea` appears and `skos:narrower` does not, `SkosActions.Tests.cs:88-108`). There is no duplicate check, so an existing pair can be stated a second time.
- **Relate:** one triple, `<from> skos:related <to>` (R5.2). No inverse, no duplicate check.
- **Add concept** (`AddSkosConceptCommand`, `Skos/SkosCommands.cs:30-75`): three splices under one snapshot inverse, re-parsing between them so no splice works from a stale span: `<iri> rdf:type skos:Concept`, `<iri> skos:prefLabel "label"@en`, and, when the file has a scheme, `<iri> skos:inScheme <first scheme>`. The first scheme is the first subject typed `skos:ConceptScheme` in document order (`SkosSelection.cs:37-43`). The label is always tagged `@en`, because the context channel does not carry the registration and so cannot know its display language (`SkosActions.cs:167-174`). Test: `SkosActions.Tests.cs:163-189`.
- **Disconnect** (`DisconnectSkosPairCommand`, `SkosCommands.cs:78-133`): removes every triple stating the pair, `A skos:broader B` and `B skos:narrower A` for a hierarchy pair, or `A skos:related B` and `B skos:related A`, re-finding them on a fresh parse after each removal, until none remain; one undo restores them all. Only edges whose ends are `res:` ids with the `broader` or `related` predicate resolve to a pair (`SkosSelection.cs:50-83`); a mapping edge falls back to the family's single-statement removal.
- **Preferred label edit:** when the concept has a label in that language, only its lexical form is rewritten and its tag and datatype are kept (`ReplaceRdfObjectLiteralCommand`; R5.4; `SkosActions.Tests.cs:192` keeps `"Zwarte thee"@nl`); otherwise a new `skos:prefLabel` triple in that language is added. Refused when the concept is labelled through SKOS-XL (`SkosProperties.cs:123-135`; `SkosActions.Tests.cs:222`).
- **Documentation edit:** the first existing literal is rewritten with its tag kept, or a new *untagged* literal is added (`SkosProperties.cs:138-151`).
- **Remove:** the family's `RemoveRdfResourceCommand` removes every triple in which the resource is subject or object.
- **Rename:** the family's `RenameRdfTermCommand` rewrites every occurrence of the IRI.
- **Declare prefix:** the family's `rdf.add-prefix`.

DISL operations describe changes to a model; they cannot promise which bytes of a foreign file change, nor that an inverse is never written. The `.dis` delegates rename to the plugin and describes add concept in DISL actions, whose effect on the file is the plugin's.

## 8. Notation the `.dis` approximates

The standalone canvas is drawn through the central canvas library (`SkosCanvas.tsx:42-227, 329-338`, `skos.css`, the shared `canvas.css`). Where DISL's notation is close but not exact:

- **Sizes and label placement.** Concepts are 200 × 44 boxes, schemes and collections 260 × 40 boxes with no fill ("a container, not a node"). The concept's name sits 30 below the top edge, the notation badge 14 below the top and 8 in from the left, the language chip 14 below the top and 8 in from the right. A region's name sits 25 below its top; the collection's "ordered" tag 34 below its top, 8 in. The `.dis` expresses these as label anchors and offsets.
- **Label fitting.** Truncated labels are fitted with the shared client text metric, characters × font size × 0.55 at the label's own font size (the standalone client centralization ruling of 2026-09-27, PR #71). DISL's `overflow: ellipsis` does not fix the metric.
- **Scheme labels versus collection labels.** The code comment says a collection shows "label (memberCount)" and a scheme the label alone, but only schemes carry a member count, so in practice a scheme shows "Label (N)" and a collection its label alone (`SkosCanvas.tsx:143-148, 254`). The `.dis` follows the behaviour.
- **Styling by state.** An alternate label is italic; an IRI fallback is monospace, 11 px, at opacity 0.7; a blank concept is dashed 5 3 at opacity 0.75, "visibly provisional"; the notation is 10 px in the primary colour (the "descriptor-code convention", R1.4); the chip is 10 px, muted, "visible but quiet".
- **Edges.** All straight. Hierarchy solid and arrowless ("the layering carries the direction"), related dashed 6 4 and arrowless, mapping dotted 2 3 with an arrowhead and the predicate's local name 6 above the midpoint. Stroke widths and colours come from the shared canvas classes, not from the module.
- **Blank concepts** are drawn with edge anchors only: other concepts can connect to them but not from them (`SkosCanvas.tsx:36-40, 92-133`).
- **The truncation banner** is an HTML box centred 8 px below the top of the surface, 1 px border and text in the warning colour on the surface colour, 4 px radius, 12 px text: "Showing {shown} of {total} terms — edits are withheld on this truncated view" (`SkosCanvas.tsx:341`; `skos.css`). The `.dis` approximates it as a canvas watermark.
- **Colours** are standalone's theme variables (`src/client/src/index.css`); the module itself has no hex colour. The `.dis` copies the light and dark values as tokens.
- **Selection** is the library's centralized selection, which highlights a pushed concept and a broader edge (`SkosCanvas.test.tsx:267`).
- The canvas's accessible name is "SKOS concept scheme" (`SkosCanvas.tsx:328, 335`).

## 9. Validation

`SkosValidator` (`Skos/SkosValidator.cs`) first runs the family's `RdfValidator`; when that reports the file as unparseable, its one finding is returned and nothing else (lines 60-65; `SkosValidator.Tests.cs:55`), because rules over an empty model would bury the reason. DISL constraints run over a model, so a parse failure has no place among them. The family's file-level rules (parse errors, serializations it cannot read) are described in [w3c-rdf.md](w3c-rdf.md). Then:

| Rule id (standalone) | Severity | Message | Where |
| --- | --- | --- | --- |
| `skos.hierarchy-cycle` | error | "The broader/narrower hierarchy runs in a circle through {names}. The diagram draws every edge and breaks the circle for layering only." | one per knot the layout found; names are the knot's concepts sorted by id; line of the knot's first triple |
| `skos.duplicate-preflabel` | error | "{name} has {n} preferred labels with {tag}; SKOS allows one (S14). The lexicographically first is drawn." where tag is "no language tag" or "language '{lang}'" | per drawn concept and language with more than one; line of the first label |
| `skos.missing-preflabel` | warning | "{name} has no skos:prefLabel in any language, so the diagram falls back to {an alternate label or its IRI}." | drawn concepts; no line |
| `skos.related-overlap` | warning | "{a} and {b} are both hierarchy-linked and skos:related, which SKOS S27 forbids. Only this direct case is checked: the transitive case would need inference, which this diagram never does." | line of the first related triple |
| `skos.nonconcept-target` | warning | "{name} takes part in a {predicate} assertion but is not typed skos:Concept. Typing is never inferred, so only the file's own silence is reported." | each end of `broader`, `narrower` and `related`, the subject of `inScheme` and `topConceptOf`, the object of `hasTopConcept`; line of the triple |
| `skos.misplaced-header` | warning | "The registration's language: header (line {n}) sits above a body: or view: line, where core's header scan would stop before reaching them. It is ignored; move it below body:/view: and above layout:." | the registration |
| `skos.unfiled-concept` | info | "{name} is in no concept scheme and under no top concept - legal SKOS, and usually a mistake. It draws in the unfiled band." | drawn concepts not filed and not reachable from any top concept; no line |
| `skos.xl-labels` | info | "This file labels concepts through SKOS-XL, which this reading does not resolve; the plain-SKOS fallbacks apply (Requirement 3.6)." | once per file |

What the `.dis` constraints cannot carry:

- Findings attach to lines of the vocabulary file (1-based, the start of the triple's span, `SkosValidator.cs:228-229`), which the model does not keep.
- The cycle finding names the whole knot in one message; the `.dis` rule reports per concept.
- The duplicate finding counts labels per language; DISL's `isUnique()` over tags cannot say how many or which tag in the message.
- `skos.nonconcept-target` is about IRIs that are *not* elements, which a DISL rule scoped to elements cannot reach. It reads the file's typing, not the budget-cut projection: a test guards the 1,517 STW concepts once misreported past the budget (`SkosValidator.Tests.cs:168`; `SkosProjection.cs:286-288`). The `.dis` has only the built-in endpoint check for it.
- "an alternate label" is chosen when the concept has any label at all, hidden labels included, although hidden labels never display (line 106).
- Names in messages use the label chooser in `en`, whatever the display language (line 154).

Deliberately not checked: the transitive case of S27, which needs entailment (lines 15-19), and every other SKOS integrity condition (S13's disjointness of the SKOS classes, the pairwise disjointness of the label properties, S46's exactMatch and broadMatch disjointness). The validator never fetches anything and never infers (lines 5-6). Coverage: `SkosValidator.Tests.cs:41-192`; the shipped examples validate clean (`SkosExamples.Tests.cs:38`; R9.5).

## 10. Where the implementation and its specification differ

The tool is a Prototype. Recorded so a port to another IDE knows which one to follow; unless a line says otherwise, the `.dis` follows the implementation.

1. **Drop target.** R5.3 files a dropped concept into "the scheme region it was dropped into"; the code always uses the file's first scheme and ignores the drop point (`SkosActions.cs:173`), as the toolbox description admits.
2. **Label language on add.** R5.3 tags the new label in the display language; the code always writes `@en` (`SkosActions.cs:167-173`).
3. **Collections as groups.** R1.5 and the design draw a collection as a group around its members' positions; the code places 260 × 40 header boxes in a trailing row and draws no membership at all. The member ids travel to the client, which stores them and never uses them (`SkosLayout.cs:63-68`; `skosModel.ts:190`; `SkosCanvas.tsx:244-258`).
4. **Scheme regions** are 260 × 40 header strips at the left of their band, not boxes enclosing their concepts (`SkosCanvas.tsx:244-251`).
5. **Member count** shows on schemes, not collections, against the code comment (section 8).
6. **Grid for schemes and collections.** R6.3 asks for member count and membership rows; they get identity, labels and documentation only (`SkosProperties.cs:82-108`).
7. **Fallback name.** R3.4 says the IRI's prefixed name; the code uses its local name, and a blank concept without labels draws an empty name (`SkosElementMapper.cs:109-122`). The design also calls a missing label "the defect it is"; the warning exists, the canvas shows the dimmed local name.
8. **Cycle tie-break.** R4.2 excludes the edge "whose subject-object IRI pair sorts lowest"; the code sorts by (broader id, narrower id) with the `res:` prefix, that is object then subject of the `skos:broader` triple.
9. **Top concepts on the first row.** R4.1 puts them there; the layout ignores asserted top concepts (section 5).
10. **The budget is split three ways.** The banner compares the SKOS element count with 1000 (`SkosSession.cs:205`); the withholding of edits uses the anchor's projection, which counts the anchor's resources rather than SKOS elements (`RdfSelection.IsTruncated`, `RdfSelection.cs:103-104`); and the session streams every element the viewport admits, not the first 1000, while the banner says "Showing 1000 of N" (`SkosSession.cs:185-224`). The shipped fixture confirms it: `business-economics.adp` sends all 1,150 concepts and 2,634 edges plus the banner (`src/fixtures/cross-tier/example-models/showcase-skos.json`). R8.1 counts N over this reading's elements and draws only the first N. The `.dis` follows R8.1.
11. **Validator language.** Messages name concepts in `en`, not in the registration's language (`SkosValidator.cs:154`).
12. **Readings are not scoped by origin** in the context channel, so SKOS actions and the SKOS grid appear under the RDF and OWL readings too (section 6).
13. **Mapping edges** cannot be created by a gesture and have no SKOS removal of their own; `broadMatch` and `narrowMatch` are drawn as plain arrows without hierarchy meaning.
14. **No duplicate check** on file under or relate (section 7), unlike OWL's subclass gesture, which refuses a duplicate (`RdfContextActionProvider.cs:231-239`).
15. **Membership editing.** The grid says "file and unfile concepts on the canvas", but no gesture files a concept into or out of a scheme or makes it a top concept; only add concept writes `skos:inScheme` (`SkosProperties.cs:22`).
16. **Documentation rows** show only the first literal of each property whatever its language, and new ones are written untagged (`SkosProperties.cs:73, 149`).
17. **Alternate labels, hidden labels and notations** are read-only by design ("edited as triples").
18. **SKOS-XL** is detected and deliberately not resolved (R3.6); an XL-labelled concept's preferred labels refuse edits.
19. **Integrity conditions** beyond the rules in section 9 are unchecked, by design.
20. **Examples.** R9.3 asks for a multilingual example with an incomplete translation and one with collections and notations; the vendored data has neither an incomplete translation nor a collection, so the language chip and collections are covered only by test fixtures. The NALT, EuroVoc and W3C reference examples planned in R9.1 never shipped.
21. **Manual checks** planned for `tests.md` (old task 5.2: the chip on EuroVoc in a non-English language, the truncation banner, file under and undo byte identity) lack the EuroVoc file for the chip check.

## 11. Known rendering gap: the hierarchy reads as one line

On 2026-09-28 the standalone screenshot session left SKOS out of the catalogue screenshots: "`geographic-names.adp` draws every concept on one horizontal line, which reads as a rule across the canvas at any zoom; `business-economics.adp` draws nothing within 20 seconds." (`docs/screenshots/readme.md`, "Not captured, 2026-09-28"; PR #105, which Peter merged without comment on SKOS). The site therefore shows no SKOS screenshot.

The cause has not been investigated. An inference from the layout code, not a verified diagnosis: rows are as wide as their layer and never wrap (section 5), so a band of 435 concepts a few layers deep is tens of thousands of units wide and a few hundred tall, and fitted to a window it reads as a line. The second symptom fits the budget split in section 10, item 10: the over-budget file streams all 1,150 concepts and 2,634 edges. A port should not copy either behaviour; a layout that wraps or centres wide layers, and a session that honours the budget, are what R4.1 and R8.1 describe.

## 12. Examples

`src/diagrams/rdf/examples/stw/`, replicated identically in `src/examples/diagrams/skos/stw/`: two extracts of the STW Thesaurus for Economics (ZBW), licensed CC BY 4.0, verified from the dataset's own `cc:license` triple, with the licence text vendored verbatim as `LICENSE.md` and the provenance in the readme (source `https://zbw.eu/stw/version/latest/download/stw.ttl.zip`, retrieved 2026-09-04).

- `geographic-names.ttl`: subthesaurus G, 435 concepts, under the budget and drawn whole; 272 concepts have more than one broader concept, so it exercises polyhierarchy; registered with `language: en`.
- `business-economics.ttl`: subthesaurus B, a breadth-first extract of 1,150 concepts, past the budget, so it shows the banner; 725 `skos:related` links; registered with `language: de`.

Both are single-rooted, bilingual (German and English) and notated. STW's preferred labels for its classification nodes embed the notation in the label text (for example `B.03  Betriebswirtschaftliches Rechnungswesen` beside the notation `B.03`), so those nodes show the code twice. What the pair does not demonstrate, as the readme records: collections, an incomplete translation, and a vocabulary with several roots. Sources considered and refused: the UNESCO Thesaurus (CC BY-SA IGO, share-alike), AGROVOC (labels through SKOS-XL), EuroVoc (served through a portal handler, not fetchable).

## 13. What DISL 0.1 could not say, in short

- Elements that exist because triples of a given shape exist, and edges that merge several triples in either direction into one (sections 4 and 7). The whole projection is the `net.etalii.adp.w3c.turtle` plugin's.
- A model file in a foreign format that other tools own, written only by minimal byte splices with byte-exact undo, and the promise that an inverse is never written.
- A header in the sidecar registration with its own placement rule (section 3).
- Offering a tool's gestures and property rows under another specification's reading of the same file (section 6).
- Which elements survive a drawn-element budget, and withholding edits on a truncated view as one rule rather than one constraint per edit kind.
- Findings located at lines of the model file, and a finding about an IRI that is not an element.
- The routing rules: never claiming a bare file, and being suggested by marker text. Recorded under `x-adp`.
- A property grid with one row per literal and read-only reasons in priority order.
- The ordering semantics of CEL's `sort()` on strings, which the label chooser's tie-break depends on.
- The new-document template as the bytes of a foreign file.
