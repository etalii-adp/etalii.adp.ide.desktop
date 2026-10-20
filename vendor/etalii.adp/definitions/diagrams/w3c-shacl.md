# SHACL shapes: what DISL cannot express

This file accompanies [w3c-shacl.dis](w3c-shacl.dis), the DISL specification of the SHACL shapes diagram (`w3c/shacl`, a SHACL shapes graph stored as Turtle `.ttl` or N-Triples `.nt`). The `.dis` says what DISL can say: one card type with its targets and rows as lists, one reference relation, the notation, the toolbox and context menu, the findings, and where persistence, layout and the writes hand over to plugins. This file holds everything else: which triples make a shape and which shapes become cards, how rows, summaries, paths and edges are computed, the layout, the exact dialogs, writes and refusal sentences, where the implementation departs from its own specification, and the places where DISL 0.1 fell short. Each point names the code, specification or conversation that shows it.

Sources, as read on 2026-09-30:

- The standalone implementation on `develop` of etalii.adp.ide.standalone (commit `b2a2692`): `src/diagrams/rdf/` (the `Shacl/` backend folder, the family classes it reuses, `client/ShaclCanvas.tsx`, `client/shaclModel.ts`, `client/shacl.css`, `api/shacl.proto`, `examples/shacl/`, the `Shacl*` tests), the shared canvas library in `src/client/src/canvas/`, `docs/tools.md` line 140, and `tests.md`. Unless a path says otherwise, backend paths below are relative to `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/`, client paths to `src/diagrams/rdf/client/`, and test paths to `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf.Tests/`.
- The approved spec-workflow specification `shacl-diagram` (requirements, design and tasks, all approved 2026-09-03), removed from the tree by commit `53f68611` and read from its parent. Requirement numbers below (R1.1 and so on) are that specification's.
- The Notion "Tools" database row "SHACL shapes".
- This project's conversations from 2026-09-26 to 2026-09-30. One holds a user report about this tool (section 7); the rulings that otherwise apply are general ones: the `.dis` extension (Peter, 2026-09-30) and one DISL file per standalone tool in `definitions/diagrams/`.

## 1. What the tool is for

A SHACL shapes diagram shows what a shapes graph *says*: which constraints its shapes place on which classes, nodes and properties. The standalone description is "The constraints a shapes graph states: node shapes as cards, their property constraints as rows, and what each one targets - drawn, never executed." (`Diagram.cs:82`).

Catalogue data (Notion "Tools" row, `docs/tools.md:140`): kind Diagram, origin `w3c/shacl`, display name "SHACL shapes", family "Ontologies & semantic web", rarity "Adoption from standard", theory "W3C SHACL", example "SHACL Play", state Implemented in standalone. The Notion row's purpose is "See which constraints SHACL shapes place on which classes and properties." and its reason for a specialized tool is "Constraints nested in Turtle are hard to follow; a shapes view puts each target, path and constraint side by side." The row's page body is blank and names no focus areas. Standalone's icon is `mdi-check-decagram-outline` (`Diagram.cs:83`), written `mdi:check-decagram-outline` in the `.dis`. A screenshot, `docs/screenshots/shacl.png`, was added to standalone in PR #105 (merged 2026-09-28).

**A shapes graph is about another graph.** That is the fact the whole tool bends around (requirements introduction). An RDF, OWL or SKOS file mostly describes the terms it contains; a SHACL file describes constraints over data that lives somewhere else, and its targets name that absent data. A naive graph drawing would point edges into nothing or invent nodes for data it has never seen. This tool draws a target as a declaration on its shape and never fabricates an element for a targeted term.

The research behind the drawing rules (requirements, "How SHACL shapes are visualized"):

- **Adopted:** the UML-card convention of SHACL Play (a node shape is a class box, a property shape an attribute line with `[min..max]`), transposed to this repository's card idiom; targets as declarations on the card, as every surveyed tool that draws targets does; shape references as labelled edges, with `sh:class` drawn only where exactly one drawn card claims the class so the mapping stays deterministic; severity and deactivation worn rather than hidden; complex paths printed as SHACL path syntax, "never expanded into plumbing".
- **Rejected:** constraint blank nodes as free-floating nodes (the unreadable naive projection); fabricated nodes for targeted but absent data ("a diagram that lies about what the file contains"); any execution or preview validation (section 9); TopBraid EDG's form-style rendering, whose instinct lands in the property grid instead.

## 2. The file and the shared engine

The tool is one of four readings over one engine in `src/diagrams/rdf/`: the anchor `w3c/rdf` and the siblings `w3c/owl`, `w3c/skos` and `w3c/shacl` (`Diagram.cs:98-104`). The store, the Turtle and N-Triples parser, the line-level document, the writer with its splice discipline and byte-identical round trips, the term-input resolver, the family rename, the drawn-element budget and the reloader are the family's and are described in [w3c-rdf.md](w3c-rdf.md) beside this file. The `.dis` names all of it as the plugin `net.etalii.adp.w3c.turtle`, the same declaration the sibling specifications carry. SHACL owns only a projection, a layout, a toolbox, context cases, a property grid, a validator, two writer operations and seven commands (`Shacl/ServiceCollection.AddShacl.cs:18-55`).

**Routing.** The type declares `.ttl` with `.nt` as its alternate extension and `SharedExtension: true` (`Diagram.cs:84-86`): it never claims a bare body, which the anchor keeps, so a file becomes a shapes diagram only through Add. Add offers it when the body passes a textual marker test, "not whether the file is valid SHACL" (`Diagram.cs:87-93`):

```csharp
SuggestsBody: text => text.Contains("sh:NodeShape", StringComparison.Ordinal)
    || text.Contains("sh:property", StringComparison.Ordinal)
    || text.Contains(ShaclVocabulary.NodeShape, StringComparison.Ordinal),
```

`ShaclVocabulary.NodeShape` is `http://www.w3.org/ns/shacl#NodeShape`. The `.dis` carries the three markers in `x-adp.suggestWhenBodyContains`. A shapes file that spells none of them (for example one using only full-IRI `sh:targetClass`) is not suggested, by construction of the three tests. Several readings may be registered over one file; each is its own view of the one parsed document in the one family store, with one shared history (R2.3; tests `ShaclSession.Tests.cs:211` and `:248`).

**A new file.** `Shacl/ShaclDocumentFactory.cs:23-41` writes this starter, with CRLF line endings, where `{local}` is the base name with every character other than an ASCII letter, digit, `-` or `_` replaced by `_`, trimmed of `_`, or `untitled` when nothing is left (`:43-55`):

```turtle
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <http://example.org/> .

ex:{local}Shape a sh:NodeShape ;
    sh:targetClass ex:{local} ;
    sh:name "{baseName}" ;
    sh:property [
        sh:path ex:name ;
        sh:datatype xsd:string ;
        sh:minCount 1 ;
        sh:maxCount 1 ;
    ] .
```

The starter deliberately targets a class the file does not describe, "the normal case for this medium and not a finding" (`:11-16`). DISL has no notion of a starter document.

**Reload.** The session subscribes to the store's change event for its body path, drops its cached layout, re-renders and sends a diff; a failed re-read is logged at warning level as "Could not re-read {BodyPath} after a change" (`Shacl/ShaclSession.cs:226-249`). The family reloader keeps the last good document through an unreadable reload (`Shacl/ServiceCollection.AddShacl.cs:50-52`).

## 3. The registration and where positions live

The `.dis` declares `files.mode: "split"` with the model in `{name}.ttl` and the view in `{name}.adp`, and `view.store: ["bounds"]`. What that stands for is standalone's registration file, which DISL does not describe:

- The `.adp` beside the shapes file holds the type line `w3c/shacl`, a `body:` header naming the `.ttl` and an optional `layout:` block of `<id>: <x> <y>` lines, for example `res:http://fairdatapoint.org/catNavShape: 406.201 -53.523` (`src/examples/diagrams/shacl/fair-data-point/navigation-shapes.adp`). Only positions are stored, never sizes, and only for IRI-named cards. A registered `.nt` names its body with an explicit `body:` header, because the alternate extension derives no registration sibling (`Diagram.cs:11-16`).
- Nothing about targets, rows, severity or any other drawn content is persisted; it is recomputed from the triples on every render, and nothing of ADP's is ever written into the RDF (R3.2).
- **The registration header facility is deliberately unused** (R4.5; `Shacl/ShaclSessionFactory.cs:9-13`, `Shacl/ShaclSession.cs:12-19`): "a shapes graph constrains any number of data graphs and binding one registration to one of them would misstate the medium". Annotating targets against a chosen data graph is the first step of execution, which a future specification would own whole. The SKOS sibling's `language:` header has no counterpart here, which is why the `.dis` declares no `registrationHeaders`.
- **Moving a card** dispatches core's `SetRegistrationLayoutCommand(registrationPath, elementId, x, y)` through history, never touching the shapes file, and undoes to the byte (`Shacl/ShaclSession.cs:116-152`; test `ShaclSession.Tests.cs:188`). The client sends the raw top-left position (`ShaclCanvas.tsx:228-234`).
- **Move refusals** (`Shacl/ShaclSession.cs:106-145`), each shown on the canvas's refusal line: without a project history "This diagram is read-only."; without a registration "This diagram was opened without a registration, so there is nowhere to store a position. Register the file to arrange it."; for an edge or the banner "That element is not something this diagram can move."; for a `blank:` card the blank-node sentence of section 6; and for dropping a card into another "A shape is not held inside another shape; a reference between them is stated as a triple. Dragging changes where a card sits on the canvas." The `.dis` carries the blank-node and reparent sentences as `placement` constraints; the other three depend on the registration and the history, which DISL has no way to name.

## 4. From triples to drawn elements

DISL's metamodel describes elements a user creates. Here every element is computed from the triples by a pure projection, `Shacl/ShaclProjection.cs`, which emits "node shapes as cards, property shapes as rows, targets as chips, shape-to-shape references as edges - and nothing else" (`:3-9`). In the `.dis` every field those rules fill is a read-only attribute the plugin supplies.

**Which terms are shapes** (`Shacl/ShaclShapeDiscovery.cs:19-82`). Over all triples in document order, a term is marked a shape when:

1. an `rdf:type` triple has `sh:NodeShape` or `sh:PropertyShape` as its object (the subject is marked);
2. the predicate is a target predicate (`sh:targetClass`, `sh:targetNode`, `sh:targetSubjectsOf`, `sh:targetObjectsOf`) or a constraint parameter (the subject is marked);
3. the predicate is `sh:node`, `sh:property`, `sh:qualifiedValueShape` or `sh:not` (the object is marked);
4. the predicate is `sh:and`, `sh:or` or `sh:xone` (every member of the RDF list object is marked, each at the index of its own `rdf:first` triple; a malformed or cyclic list "yields what it has and stops", `:106-110`).

The constraint parameters are `sh:class`, `datatype`, `nodeKind`, `minCount`, `maxCount`, `minExclusive`, `minInclusive`, `maxExclusive`, `maxInclusive`, `minLength`, `maxLength`, `pattern`, `flags`, `languageIn`, `uniqueLang`, `equals`, `disjoint`, `lessThan`, `lessThanOrEquals`, `not`, `and`, `or`, `xone`, `node`, `property`, `qualifiedValueShape`, `qualifiedMinCount`, `qualifiedMaxCount`, `qualifiedValueShapesDisjoint`, `closed`, `ignoredProperties`, `hasValue`, `in` and, from SHACL-SPARQL, `sparql` (`Shacl/ShaclVocabulary.cs:74-86`). A literal is never a shape. Prefixed and full spellings of one IRI collapse to one shape (`Shacl/_Model/ShaclShape.cs:10`). The first marking triple wins and shapes are ordered by it, "the deterministic backbone of the whole reading" (`:13-15`). A shape is a property shape when it is the subject of an `sh:path` triple, "the recommendation's own distinction" (`:59-66`). A shape has an implicit class target when it is an IRI the file types `rdfs:Class`, or types with a class whose stated `rdfs:subClassOf` chain reaches `rdfs:Class`: "Only stated triples count - the no-inference rule holds here as everywhere" (`:152-193`). Subjects that are not shapes never draw: "co-resident ontology or instance content belongs to the sibling readings" (`:8-10`).

**Which shapes become cards** (`Shacl/ShaclProjection.cs:77-87`), verbatim:

```csharp
if (shape.IsPropertyShape)
    return shape.Term is IriTerm && !context.PropertyReferenced.Contains(shape.Key);
return shape.Term is IriTerm
    || context.NodeReferenced.Contains(shape.Key)
    || !context.OperandReferenced.Contains(shape.Key);
```

`PropertyReferenced` holds the objects of `sh:property`, `NodeReferenced` the objects of `sh:node`, and `OperandReferenced` the objects of `sh:not` and `sh:qualifiedValueShape` plus the members of `sh:and`, `sh:or` and `sh:xone` lists (`:484-541`). So every IRI-named node shape is a card (R1.1); an IRI-named property shape is its own card only when no `sh:property` references it, because it "would otherwise be invisible" (R1.6), and is otherwise a row on each referencing card; a blank property shape is always a row; and a blank node shape is an anonymous card when `sh:node` references it or when it is no operand at all, and is otherwise summarized inline where it is used.

**The card** (`Shacl/_Model/ShaclCard.cs:15-26`; wire `ShaclShapePayload`, `api/shacl.proto:94-120`):

- Title: the IRI compressed with the file's own declared prefixes (longest match with a clean local name), else the IRI's last `#` or `/` segment, else the full IRI (`RdfProjection.cs:206-217`); `_:{label}` for a labelled blank node; `anonymous shape` for an unlabelled one (`Shacl/ShaclProjection.cs:113-118`). The client joins it with `sh:name` as `{display} — {name}` when a name exists.
- Flags, read only from the card's own subject triples (`:128`): deactivated when an `sh:deactivated` literal has the lexical form `true` (`:138-140`); closed likewise for `sh:closed` (`:142-144`); severity when `sh:severity` names anything other than `sh:Violation`, shown as its display such as `sh:Warning` (`:146-149`); name and description from the first `sh:name` and `sh:description` literals (`:151-157`).
- **What a card does not show.** Value constraints stated directly on a node shape (`sh:class`, `sh:datatype`, `sh:nodeKind`, `sh:pattern`, `sh:minCount`, `sh:in`, `sh:hasValue`, `sh:qualifiedValueShape`, `sh:ignoredProperties`, `sh:order`, `sh:group` and the like) produce no row and no text; only `sh:message` reaches the property grid (`:131-187`).

**Rows**, in the order of the card's own triples (`:128`; `Shacl/_Model/ShaclRow.cs:15-22`). Four sources:

- **Property row**, one per `sh:property` object (`:159-161`, built at `:229-276`). The path is the property shape's first `sh:path` in path syntax, empty when there is none. The name is its first `sh:name`. The cardinality is `[{min or 0}..{max or *}]` from the first `sh:minCount` and `sh:maxCount` lexical forms, for example `[1..1]` or `[0..*]`. The row's own severity follows the card rule. A literal `sh:property` object draws an empty blank row (`:237-241`).
- **Combinator row** for `sh:and`, `sh:or` or `sh:xone` stated on the card (`:167-169`, `:340-368`). IRI operands become edges. A row exists only when at least one operand is blank, and then it lists every operand, IRI ones by name and blank ones one level deep, for example `or(ex:A, { datatype xsd:date })`, with an empty path and cardinality.
- **Negation row** for `sh:not` on the card with a blank operand, `not({ … })`; an IRI operand is a `not` edge instead (`:171-181`).
- **SPARQL row** for `sh:sparql` on the card, summary `SPARQL constraint` (`:183-186`). The query is extracted, never parsed further and never executed (R1.7).

**The summary** of a property row joins its parts with `, ` in this fixed order (`:20-38`, `:266-272`): `sh:datatype`, `sh:class`, `sh:nodeKind`, `sh:minLength`, `sh:maxLength`, `sh:pattern`, `sh:minInclusive`, `sh:maxInclusive`, `sh:minExclusive`, `sh:maxExclusive`, `sh:uniqueLang`, `sh:hasValue`, `sh:in`, `sh:node`, `sh:qualifiedValueShape`, `sh:sparql`, then the combinators `sh:and`, `sh:or`, `sh:xone`, `sh:not`. Each part (`:278-338`):

- `sh:datatype`: the display only, for example `xsd:string`.
- `sh:class`: `class ex:Person`, or no text and a `class` edge when exactly one drawn card claims the class.
- `sh:nodeKind`: the local name after `sh:`, for example `IRI` or `BlankNodeOrIRI`.
- `sh:in`: `in (a, b, c)`, members as displays, literals by lexical form, blank members as `…`.
- `sh:hasValue`: `has {term}`.
- `sh:node`: an IRI gives no text and a `node` edge labelled with the row's path; a blank node gives `node { … }`.
- `sh:qualifiedValueShape`: `qualified {display}` or `qualified { … }`; never an edge.
- `sh:sparql`: no text; the row is flagged instead.
- Every other listed parameter: `{localName} {term}`, for example `pattern ^[A-Z]` or `minLength 3`.
- Combinators: `and(…)`, `or(…)`, `xone(…)`, `not(…)`, IRI operands by display and blank ones as one-level summaries; operands inside a row never draw edges.

Not in any summary: `sh:minCount` and `sh:maxCount` (they are the cardinality column), `sh:flags`, `sh:languageIn`, `sh:equals`, `sh:disjoint`, `sh:lessThan`, `sh:lessThanOrEquals`, `sh:qualifiedMinCount`, `sh:qualifiedMaxCount`, `sh:qualifiedValueShapesDisjoint`, `sh:closed`, `sh:ignoredProperties`, `sh:description`, `sh:message`, `sh:defaultValue`, `sh:order` and `sh:group`.

**The inline summary** of a blank node (`:382-413`) is one level deep: `{ ` plus, for each `sh:` predicate of the blank node other than `rdf:type`, `local term`, plus ` }`; deeper blank nodes elide to `…`, "with the property grid carrying the full structure" (`:378-381`). The W3C `sh:or` example reads `or({ path ex:firstName, minCount 1 }, { path ex:givenName, minCount 1 })`.

**Path syntax** (`:416-462`; test `ShaclProjection.Tests.cs:169-185`): an IRI is its display; `[ sh:inversePath p ]` is `^p`; `[ sh:alternativePath (a b) ]` is `a|b`; `sh:zeroOrMorePath`, `sh:oneOrMorePath` and `sh:zeroOrOnePath` are `p*`, `p+` and `p?`; an RDF list is the sequence `a/b`; nested alternatives and sequences are parenthesized. Tested outputs include `^ex:parent`, `ex:parent/ex:member`, `ex:a|ex:b` and `ex:next*`.

**Target chips** (`Shacl/_Model/ShaclTargetChip.cs:14-20`; `api/shacl.proto:41-62`). One per target triple on the card's subject, in source order (`Shacl/ShaclProjection.cs:207-227`), then the implicit class chip, whose term is the card's own display (`:190-193`). A literal `sh:targetNode` shows its lexical form and a blank target `_:{label or ordinal}`. `DescribedInFile` records whether the targeted term is a subject anywhere in the file, "a fact the grid states, never a finding". The client words them `targets class {t}`, `targets node {t}`, `targets subjects of {t}`, `targets objects of {t}` and `targets its own instances` (`shaclModel.ts:196-209`). No element or edge is ever made for a target, present or absent, even when it names a drawn shape (tests `ShaclTargetChips.Tests.cs:49`, `:89`).

**Edges** (`Shacl/_Model/ShaclEdge.cs:9`; id `shacl-edge:{fromId}|{kind}|{toId}`, `Shacl/ShaclProjection.cs:202`):

| Source triple | Kind | Label | Where |
| --- | --- | --- | --- |
| `sh:node` on the card | `node` | `node` | `Shacl/ShaclProjection.cs:163-165` |
| `sh:node` IRI in a property row | `node` | the row's path | `:301-309` |
| IRI operand of `sh:and`, `sh:or`, `sh:xone` on the card | `and`, `or`, `xone` | the kind | `:350-355` |
| IRI operand of `sh:not` on the card | `not` | `not` | `:171-175` |
| `sh:class` IRI in a property row, exactly one other drawn card claiming it | `class` | the row's path | `:290-297` |

A class is claimed by a drawn card through `sh:targetClass` or its implicit target; a second claimant voids the slot, and with zero or several claimants the constraint stays as `class X` text (`:89-102`, `:596-610`). Edges are kept only when both ends are drawn cards and are deduplicated by id (`:65-68`), so an `sh:property` reference to an IRI property shape is a row, not an edge. The `.dis` has one relation, `Reference`, with the kind and label as attributes, because the client declares one relation type and classes each connection by kind (`ShaclCanvas.tsx:115-131`, `:198`). An edge is derived, so the family selection cannot name it; `Shacl/ShaclSelection.cs:3-37` describes a `shacl-edge:` id as `{fromDisplay} → {toDisplay}`.

**Ids.** A card is `res:{iri}` or, when anonymous, `blank:{ordinal}`, the family's selection vocabulary (`Shacl/ShaclProjection.cs:74-75`, `RdfSelection.cs:12-16`); blank ordinals "hold for this parse only". Edges are `shacl-edge:…` and the banner is the family's `truncation` element. Only `res:` ids are ever stored. The `.dis` pattern `^(res:.+|blank:[0-9]+|shacl-edge:.+)$` is the nearest DISL says; it cannot say that ids come from IRIs and parse ordinals rather than from key attributes, nor that `blank:` ids are never stored.

**The budget.** Cards beyond 1000, in discovery order, are not drawn (`Shacl/ShaclProjection.cs:17`, `:49-51`); rows travel with their card. The banner is raised from the document's card total, never from a viewport (`Shacl/ShaclSession.cs:195-224`). Edits, however, are withheld by a different measure; see section 10. DISL's `limits.maxElements` only asks a runtime to warn.

**Viewport delivery.** A connection receives only the cards whose 320 by 400 cell touches its viewport, the edges whose two ends it received, the budget applied to that visible set, and the banner decided by the document total (`Shacl/ShaclSession.cs:24-33`, `:90-100`, `:202-224`). This is a runtime concern DISL does not model.

## 5. Layout

DISL names a layout; it does not define one. The `.dis` names `plugin:net.etalii.adp.w3c.shaclGrid` with a built-in `grid` fallback. DISL's `grid` has no rule for which card goes where, and this layout's order is the discovery order of section 4, so the plugin is what carries it. What it does (`Shacl/ShaclLayout.cs:11-43`):

- Pure and deterministic, no physics, no randomness.
- Cards in projection order fill three columns left to right: card `i` sits at `x = (i mod 3) × 320`.
- Each grid row is as tall as its tallest card, measured as `64 + (targets + rows) × 22`, and the next grid row starts 48 below it.
- Positions are top-left, keyed by element id; anonymous cards are placed too, and only ever by this computation (`:5-10`).
- The session lays out the whole unbudgeted projection, so positions stay stable while panning, then overlays the registration's authored `layout:` block, when a registration exists (`Shacl/ShaclSession.cs:167-190`).
- The canvas offers the manual mode only (`ShaclCanvas.tsx:142`); there is no Arrange.

The layout's measure is not the client's: the layout assumes a 64 header and a 320 pitch, while the client draws cards 260 wide with a 30 header, so a card is `56 + n × 22` tall and the columns leave 60 of gutter. The comment "matching the layout's column pitch" on `CARD_WIDTH` (`ShaclCanvas.tsx:21`) is not literally true. The `.dis` records both numbers: the card size in the notation, the pitch in the plugin options.

## 6. Interaction

DISL can declare tools, menus, operations with dialogs, enabled expressions and refusal messages on gestures. It cannot carry a reason for an unavailable menu entry, a dialog's button label, a menu label computed from the model, or the rule that one drop acts on the card beneath it. Those are here.

**Toolbox** (`Shacl/ShaclToolboxProvider.cs:21-35`), origin-scoped by construction:

| Id | Label | Icon | Description | Runs |
| --- | --- | --- | --- | --- |
| `shacl.toolbox.node-shape` | Node shape | `mdi-check-decagram-outline` | "A shape that constrains focus nodes. Drop it where it should sit; you will be asked for its name." | `shacl.add-node-shape` |
| `shacl.toolbox.property-row` | Property row | `mdi-table-row-plus-after` | "A constraint on the values reached over one path. Drop it on a shape; you will be asked for the path." | `shacl.add-property-row` |

A drop inside a card's rectangle runs the action against that card's id; anywhere else against a placement id (`ShaclCanvas.tsx:209-222`, `:235-239`). "The drop and the menu run the same action" (`Shacl/ShaclToolboxProvider.cs:5-8`). DISL's operation tools cannot say that a drop's target depends on what lies under it.

**The eight action ids** (`Shacl/ShaclActions.cs:29-51`): `shacl.add-node-shape`, `shacl.add-target-class`, `shacl.add-target-node`, `shacl.add-property-row`, `shacl.deactivate`, `shacl.reactivate`, `shacl.remove-shape`, and the prefix `shacl.remove-target|` followed by `{predicateIri}|{termIri}`. The cases answer only when the target's origin is empty or `w3c/shacl` (`:265-267`).

**Discovery** (`Shacl/ShaclActions.cs:54-123`):

- On a placement: one entry, "Add node shape here…" (`mdi-check-decagram-outline`).
- On a shape, when the edit gate applies (below), each entry available or unavailable with the gate's reason: "Add target class…" (`mdi-crosshairs`), "Add target node…" (`mdi-crosshairs-gps`), "Add property row…" (`mdi-table-row-plus-after`); on a drawn card "Reactivate shape" (`mdi-play-circle-outline`) when deactivated, else "Deactivate shape" (`mdi-pause-circle-outline`); one "Remove target: {kind word} {term}" (`mdi-crosshairs-off`) per chip with a predicate and an IRI term, kind words `class`, `node`, `subjects of`, `objects of` (literal, blank and implicit targets get none); and "Remove shape (with {count} statements)" when the count is above one, else "Remove shape" (`mdi-delete-outline`, shortcut Delete). The count is the shape's own subject triples plus every triple of a blank subtree only it reaches (`Shacl/ShaclWriter.cs:199-210`); a three-triple shape with a two-triple blank row counts 5 (`ShaclActions.Tests.cs:103-111`).
- Only class and node targets can be added; all four kinds can be removed.

The `.dis` keeps one "Remove target" operation with a choice of removable chips, and a fixed "Remove shape" label with the count in `removalCount`, because a DISL menu has neither one entry per list item nor a computed label.

**The edit gate** (`Shacl/ShaclEditGate.cs:38-75`), which decides "without consulting the writer": a `blank:` id that is a shape applies but is unavailable with the blank-node sentence; a `res:` id that is not an IRI shape does not apply ("the data-graph reading's business"), so a data graph never grows a menu of constraint verbs; a truncated file applies but is unavailable with the truncation sentence; otherwise every entry is available. The `.dis` approximates the reasons with `enabled` expressions plus the gesture constraints.

**Dialogs** (`Shacl/ShaclActions.cs:152-162`, `:272-273`), each with an empty initial value and the button "Add":

| Action | Title | Placeholder |
| --- | --- | --- |
| `shacl.add-node-shape` | Add node shape | Shape IRI or prefixed name |
| `shacl.add-target-class` | Add target class | Class IRI or prefixed name |
| `shacl.add-target-node` | Add target node | Node IRI or prefixed name |
| `shacl.add-property-row` | Add property row | Path IRI or prefixed name |

Deactivate, Reactivate, Remove shape and Remove target run at once, with no dialog and no confirmation (`:164-210`).

**Dialog input** is resolved by the family's `RdfTermInput.Resolve` (`RdfTermInput.cs:14-53`), which the `.dis` names as the plugin functions `rdfResolveTerm` and `rdfTermRefusal`. It accepts a bracketed `<iri>`, anything containing `://`, or `prefix:local` with a declared prefix, and otherwise refuses with one of:

- "A term needs a name: a full IRI, or a prefixed name like ex:thing."
- "An IRI cannot contain spaces."
- "The brackets are empty."
- "'{value}' is neither a full IRI nor a prefixed name. Write ex:{value}, or a full IRI."
- "The prefix '{prefix}:' is not declared in this file. Declare it first, write the full IRI, or bracket a scheme IRI as <{value}>."

A prefix is never invented; the family's "Declare prefix…" entry is offered on a placement beneath the SHACL one (`RdfContextActionProvider.cs:146-167`). A commit that finds no shape IRI answers "'{actionId}' does not apply to this selection." (`RdfContextActionProvider.cs:376`).

**Writes.** Every command runs through the family's `RdfEdits.Run`: load, refuse an unparseable file with "This file does not parse, so nothing can be edited until it is fixed.", snapshot, run one writer operation (a refusal leaves the document untouched), save; the inverse restores the whole document's bytes and redo re-runs the command (`Commands/RdfEdits.cs:5-43`). Commands are keyed by shape IRI, never by element id. The `.dis` names each write as an `net.etalii.adp.w3c.turtle` plugin action; `appendPropertyShape` and `removeShapeWithSubtrees` are SHACL's own operations in `Shacl/ShaclWriter.cs`, the rest are the family writer's.

| Command | What it writes | Where |
| --- | --- | --- |
| `CreateShaclNodeShapeCommand` | `{iri} a sh:NodeShape .` appended at the end of the file after a blank line when the IRI has no statement yet; otherwise the last statement's `.` becomes `;` and `a sh:NodeShape .` follows on a continuation line. An existing IRI is not refused. | `Shacl/ShaclWriter.cs:135-142`, `RdfWriter.cs:47-67` |
| `AddShaclTargetCommand` | One `sh:targetClass` or `sh:targetNode` triple with an IRI object as a continuation of the shape's statement; duplicates are not checked. | `Shacl/ShaclWriter.cs:88-104` |
| `RemoveShaclTargetCommand` | Removes exactly the first (shape, predicate, IRI term) triple. | `Shacl/ShaclWriter.cs:111-132` |
| `AddShaclPropertyRowCommand` | Turns the `.` of the shape's last IRI-subject statement into `;` and inserts one line `sh:property [ sh:path P ; sh:datatype D ; sh:minCount n ; sh:maxCount m ; sh:name "N" ] .`, omitting absent parts, indented like the statement's second line or four spaces in from a single-line statement, with the file's own prefixes. The menu supplies only the path. | `Shacl/ShaclWriter.cs:30-85`, `:392-417` |
| `SetShaclDeactivatedCommand` | Off: adds `sh:deactivated "true"^^xsd:boolean` unless any `sh:deactivated` triple exists. On: removes the first `sh:deactivated` triple whatever its value. | `Shacl/ShaclWriter.cs:147-168`, `:421-422` |
| `SetShaclLiteralCommand` | For `sh:name` or `sh:description`: rewrites the first literal in place, dropping any language tag or datatype, or adds one. | `Shacl/ShaclWriter.cs:175-192` |
| `RemoveShaclShapeCommand` | First removes, bottom-most first and reparsing between removals, every triple of a blank subtree reachable only from the shape that is stated outside the shape's own statements; then the shape's own subject triples bottom-up, which carries every inline `[ … ]` with them. Triples naming the shape as their object stay. | `Shacl/ShaclWriter.cs:231-389` |

The property row's blank node is permitted under the blank-node boundary because it "is created, never addressed" (`Shacl/ShaclWriter.cs:24-29`). Existing blank-node content is read everywhere and mutated nowhere; it is only created by the property-row write or swept with its exclusive owning shape (R3.1 to R3.4). The removal matrix is pinned in `ShaclRemoveShape.Tests.cs:35-314`.

**Rename** (F2) is the family's `rdf.rename-resource`, "Rename a resource, rewriting every reference with it", refusing a collision with "{iri} already names something in this document; renaming onto it would silently merge two resources." (`RdfContextActionProvider.cs:25-26`, `:338-343`; R5.6).

**Refusals** from the SHACL writer and gate (`Shacl/ShaclRefusals.cs`), verbatim:

- Blank-node boundary (`:16-17`): "This constraint is written as a blank node, which has no identity that survives a reparse, so an edit keyed to it could not be undone reliably. Open the file as text to change it, or give the shape an IRI of its own." The gate, the grid, the writer and the session all answer with it, and `ShaclDoubleRefusal.Tests.cs:85` pins that both layers say the identical sentence.
- No such shape (`:20-21`): "No IRI-named shape of that name is in this file, so there is nothing to edit."
- Path required (`:24-25`): "A property shape needs exactly one sh:path, so a path is required before the row can be written."
- No such target (`:28-29`): "That target is not declared in this file, so there is nothing to remove."
- Implicit target (`:32-33`): "This shape targets its own instances because the file states it to be a class, not through a target triple, so there is no declaration to remove. Remove the class typing instead, as text."
- Truncated view (`RdfSelection.cs:21-22`): "The diagram shows only the first part of this file under the drawn-element budget, so edits through it are withheld - an edit through a partial view could touch what the view does not show. Edit the file as text instead."

**The property grid** (`Shacl/ShaclProperties.cs`) answers only for a drawn card in the default-budget projection, in the groups Identity, Shape, Targets and Constraints (`:36-39`, `:56-66`). An anonymous card is frozen with the blank-node sentence, a truncated file with the truncation sentence (`:70-71`). Rows, in order:

| Id | Label | Value | Editable, or the read-only reason |
| --- | --- | --- | --- |
| `shacl.iri` | IRI | the full IRI, or `(anonymous)` | "Rename through the context menu, so every reference follows the name." (blank: the blank-node sentence) |
| `shacl.display` | Name | the display | "Rename through the context menu, so every reference follows the name." |
| `shacl.name` | sh:name | the first `sh:name` | editable, single line, unless frozen |
| `shacl.description` | sh:description | the first `sh:description` | editable, single line, unless frozen |
| `shacl.message` (when any) | sh:message | every `sh:message` joined with ` / `, IRI cards only | "This is edited as triples in the graph reading." |
| `shacl.deactivated` | Deactivated | `true` or `false` | "Switch the shape off and on through the context menu." |
| `shacl.closed` (when closed) | Closed | `true` | "This is edited as triples in the graph reading." |
| `shacl.severity` (when not default) | Severity | for example `sh:Warning` | "This is edited as triples in the graph reading." |
| `shacl.target:{n}` | Target class, Target node, Target subjects of, Target objects of, Target (implicit) | the term, plus " (not described in this file)" when absent | "Targets are declared and withdrawn through the context menu." (implicit: the implicit-target sentence) |
| `shacl.row:{n}` | `{path}`, or `SPARQL` or `constraint` without a path, plus ` - {name}` when named | `{summary} {cardinality}` | "Property rows are added through the context menu, and edited as triples." (blank row: the blank-node sentence) |
| `shacl.sparql:{n}` | sh:select | the query text, multi-line, IRI cards only | "Shown as written, never parsed beyond extraction and never executed - this reading draws constraints, it does not run them." |

Only `shacl.name` and `shacl.description` write, through `SetShaclLiteralCommand`, and any string is accepted (`:116-143`). The `.dis` gives the attributes these labels and groups and puts each read-only reason in the attribute's `doc.description`; DISL has no field for a per-row read-only reason, and its inspector shows targets and rows as two tables rather than one grid row each.

**Selection.** Chips and rows are not selectable; they are reached through the card's grid and menu. Delete on a `shacl-edge:` connection finds no action (`RdfSelection.cs:73-95`), which the `.dis` states as `deletable: false`.

## 7. Notation the `.dis` approximates

The standalone canvas is a declarative definition for the central canvas library (`ShaclCanvas.tsx:41-144`, `shacl.css`, `src/client/src/canvas/canvas.css`). Where DISL's notation is close but not exact:

- **The card** is the library's `box` with `rx: 6px` (`shacl.css:6-8`), fill `--color-surface`, stroke `--color-border` at 1.5 (`canvas.css:166-170`), 260 wide (`ShaclCanvas.tsx:22`) and `56 + (targets + rows) × 22` tall (`shaclModel.ts:212-214`). The backend's positions are top-left and the canvas centres them (`ShaclCanvas.tsx:170-171`).
- **The title** is `{display} — {name}` or `{display}`, 20 below the top, semibold, truncated with an ellipsis (`ShaclCanvas.tsx:53-61`, `shacl.css:22-24`). **The badges** `deactivated`, `closed` and the severity display, joined with ` · `, sit right-aligned 8 in on the title line in the muted hint style (`ShaclCanvas.tsx:62-70`, `:182-184`).
- **Targets are sentences, not pills**: 11 px italic at opacity 0.85, left-aligned 8 in, one per 22 starting 46 below the top (`ShaclCanvas.tsx:71-79`, `shacl.css:26-32`). Present and absent terms look the same: "absence is the normal case, not a warning". The `.dis` draws them as a compartment.
- **The three-column row.** Each row is one label declaration with two `columns`: the path (or `SPARQL constraint`, in italic) on the left at 11 px, the summary left-aligned 110 in at 10 px muted and truncated at the card's edge, and the cardinality right-aligned 8 in at 10 px muted (`ShaclCanvas.tsx:80-111`, `shacl.css:34-47`). The code explains why columns rather than three lists: three collections over one list "would let them drift apart the moment one carried a condition - pairing row 2's cardinality with row 3's path, silently". DISL's compartment has one `itemText` per item, so the `.dis` joins the three parts into one line; a runtime following it loses the alignment of the summary and cardinality columns.
- **The duplicate React key.** On 2026-09-27 Peter reported a console warning from the SHACL canvas, "Encountered two children with the same key, `3-0`". The diagnosis was that "It was a real bug in the shared canvas library. The SHACL canvas itself was fine.": the constraint row is one label declaration with two columns, and the library keyed every drawn line by declaration index and line index only, so row 0 of declaration 3 drew three `<text>` elements under the key `3-0`. PR #79, "columns get their own label keys" (merged 2026-09-27), made each laid-out line record which column drew it and added `labelKey()` (`src/client/src/canvas/library/definition/labels.ts:51`, used at `DiagramCanvas.tsx:3043`), with a test in `labels.test.ts`. A runtime that renders multi-column rows has to give each column its own identity.
- **An anonymous card** is dashed `4 3`, "An anonymous shape draws like any other and simply cannot be moved; the dash says so without hiding it."; **a deactivated card** is at opacity 0.55, "dimmed rather than removed" (`shacl.css:10-20`).
- **Carried but not drawn**: a row's severity, blank flag and name, which reach only the grid. No icons are drawn inside cards.
- **Edges** are straight 1.5 px muted lines with a filled arrowhead at the target and the label 6 above the midpoint (`ShaclCanvas.tsx:115-131`, `canvas.css:534-568`). Every kind draws the same line: the `shacl-edge-{kind}` classes exist and no stylesheet rule uses them. The source end declares no anchors, so there is no connect gesture: "a reference is stated in the file ... so it is drawn rather than drawn-on" (`:33-40`). The `.dis` adds a `connect` constraint with a sentence of its own, because DISL has no way to say a relation has no gesture at all; standalone never shows that sentence.
- **The truncation banner** is an HTML status line above the surface, not a watermark: "Showing {shown} of {total} shapes. Edits are withheld while the view is partial." (`ShaclCanvas.tsx:270-274`). The canvas's accessible name is "SHACL shapes".
- **Colours** are standalone's theme variables (`src/client/src/index.css`): surface `#ffffff`/`#1e293b`, border `#e2e8f0`/`#334155`, text `#0f172a`/`#f1f5f9`, muted `#64748b`/`#94a3b8`; the `.dis` copies them as the family's shared theme tokens.
- **Canvas actions** are declared as `rename` (F2) and `delete` (the delete gesture, on elements and connections) (`ShaclCanvas.tsx:138-141`); double-click dispatches the library's `activate` gesture, which this canvas does not declare, so it does nothing (`doubleClick: "none"` in the `.dis`).

## 8. Validation

The `.dis` states the findings as constraints. What they cannot carry:

| Rule id (standalone) | Severity | Message | Where |
| --- | --- | --- | --- |
| `shacl.path-cardinality` | error | "{shape} is used as a property shape but states no sh:path; SHACL requires exactly one, and the row is drawn with an empty path." | the shape's first triple (`Shacl/ShaclValidator.cs:84-93`) |
| `shacl.path-cardinality` | error | "{shape} states {n} sh:path values; SHACL requires exactly one, so which values it constrains is undefined." | the first `sh:path` triple (`:88-92`) |
| `shacl.impossible-counts` | warning | "{shape} requires at least {min} values and at most {max}, so nothing can ever conform to it." | `:95-104` |
| `shacl.datatype-and-class` | warning | "{shape} constrains its values by both sh:datatype and sh:class, and no value is both a literal and an instance of a class, so nothing can conform." | `:106-115` |
| `shacl.reference-leaves-file` | info | "{display} is referenced as a shape but not described in this file - it is defined elsewhere, or missing. Nothing outside the file is read to find out." | each `sh:node` or `sh:property` triple with such an IRI object (`:122-141`) |
| `shacl.unknown-term` | warning | "sh:{local} is not a term SHACL defines, so no processor will act on it - check the spelling." | each predicate or IRI object in the `sh:` namespace outside `ShaclVocabulary.AllTerms` (`:145-162`) |

- `{shape}` is the IRI's display or "An anonymous shape" (`:180-181`); every finding carries the 1-based line of the triple it cites (`:177-178`). DISL problems attach to elements, and several of these findings concern blank property shapes and triples that are not elements, so the `.dis` hangs the per-shape rules on the card (once for its own shape, once over its rows, which reports a property shape shared by two cards twice) and the per-triple rules on the diagram, naming only the first offender.
- The family validator runs first; an unparseable file yields its single `rdf.unparseable` finding and nothing else (`:59-64`). The family's `rdf.duplicate-prefix`, `rdf.relative-iri`, `rdf.language-tag` and `rdf.unknown-datatype` pass through (`RdfValidator.cs:21-33`).
- `shacl.path-cardinality` only checks shapes used as the object of `sh:property`; a typed `sh:PropertyShape` with no path that nothing references is silent (`:80-84`).
- Deliberately not findings: a node shape without a path; a target naming a term absent from the file (R4.3); anything about data conformance. The leaving-reference rule is info rather than warning because, without the network, "this tool cannot tell 'missing' from 'described in another file'" (`:23-28`).
- `AllTerms` is the union of the target predicates, the constraint parameters and the core, SPARQL and report terms (`Shacl/ShaclVocabulary.cs:100-130`); the recommendation's own `shacl-shacl.ttl` validates clean against it.

## 9. Out of scope, by decision

- **Execution.** This is a shapes view, not a validator run against data: no data graph is loaded, no shape is run, no conformance is reported, and no menu entry anywhere offers to validate, run or pick a data file (R4.1, R4.2; `Shacl/ShaclValidator.cs:4-8`, `:17-20`; `ShaclCanvas.tsx:151-155`). The natural expectation of a SHACL tool is that it runs, which is why the requirements call this statement load-bearing. The manual check in `tests.md` (2026-09-06) notes that the shell's project-wide "Validate all" entry appears on the canvas background of every diagram, including this one, and is not this tool offering to validate data. A future specification may add explicit execution (choose a data file, run, report) without contradicting this one (R4.4).
- **The registration header** (section 3).
- **Fabricated elements for targets** and findings for absent targets (R1.3, R4.3).
- **Editing existing blank-node content** (R3.3, R5.7): refused with the blank-node sentence; open the file as text.
- **Form-style rendering** and free-floating constraint blank nodes (requirements research, "Rejected").
- **A connect gesture** and reparenting (section 7, section 3).
- **Inline label editing**: labels are derived prefixed names and rename is a dialog, family-wide (`client/readme.md`).

## 10. Where the implementation and its specification differ

Recorded so a port to another IDE knows which one to follow. The `.dis` follows the specification where the two differ, and says so here.

- **The context menu on an IRI-named shape card shows none of the SHACL entries.** The family's single `RdfContextActionProvider` consults the SHACL cases on the SKOS-pair, edge, placement, gesture and fallback branches (`RdfContextActionProvider.cs:125`, `:136`, `:165`, `:187`, `:193`), but its `res:` branch returns first, at `RdfContextActionProvider.cs:104-119`, with only the family's "Rename…" and "Remove" or "Remove (with {n} statements)". An IRI shape card's id is an ordinary `res:` id, so on the live canvas it offers only those two. The manual pass in `tests.md` (2026-09-06) confirms it: "A shape card's menu offers *Rename…* and *Remove (with 3 statements)* and nothing else". Consequences: Add target class, Add target node, Add property row, Deactivate, Reactivate and Remove target are unreachable from the menu (the Property row toolbox drop, which executes directly, still works); Delete on a card runs the family `rdf.remove-resource`, which removes every triple touching the IRI including those naming it as an object, instead of `shacl.remove-shape` with its exclusive-subtree sweep; and the statement count shown is the family's, not SHACL's. The SHACL unit tests call `ShaclActions.Discover` directly and cannot see this (`ShaclActions.Tests.cs:64-73`). The property provider avoids the same trap on purpose, putting SHACL first because "a shape card's element id is an ordinary `res:` id that the fallback would also claim" (`RdfContextPropertyProvider.cs:74-80`). The `.dis` specifies the SHACL menu of R6.2.
- **Two budgets.** The banner and the drawn cards use the SHACL card count over 1000 (`Shacl/ShaclSession.cs:209`), while edits are withheld by `RdfSelection.IsTruncated`, the family projection's budget of 1000 RDF nodes (`RdfSelection.cs:103-104`, `RdfProjection.cs:24`), used by the gate and the family provider (`RdfContextActionProvider.cs:97`). A shapes file can therefore refuse edits without a banner. R8.1 wanted the budget measured on this projection, cards counted. The `.dis` keeps the two as `truncated` and `editsWithheld`.
- **Add target** was specified as one dialog asking for kind and term (R5.3); there are two fixed entries, class and node, and no way to add a subjects-of or objects-of target (`Shacl/ShaclActions.cs:29-33`, `:84-88`).
- **Add property row** was specified with a required path and optional datatype, minimum and maximum counts (R5.4). The dialog asks only for the path (`Shacl/ShaclActions.cs:158-159`, `:260`), although the command and writer accept all of them (`Shacl/Commands/AddShaclPropertyRowCommand.cs:17-24`).
- **Remove shape** was to state its count "before anything runs" (R5.6); the count lives in the menu label, with no confirmation, and that label is unreachable on IRI cards (first item).
- **Row badges.** R1.8 says "the card or row SHALL wear the badge"; a row's severity is projected and sent (`Shacl/ShaclProjection.cs:248-251`, `api/shacl.proto:89-90`) but not drawn.
- **Client drag gating.** The design had the `blank` payload flag gate dragging client-side; the canvas does not, and the backend refusal is the only gate (`ShaclCanvas.tsx:228-234`).
- **Node-shape-level constraints** are drawn nowhere (section 4). The requirements do not ask for them explicitly; a port should decide.
- **Summary omissions** listed in section 4 never appear on a row.
- **Deactivate** writes the typed `"true"^^xsd:boolean` rather than the bare `true` of R5.5, and **Reactivate** removes the first `sh:deactivated` triple whatever its value, so it also removes a `sh:deactivated false`.
- **Layout and card dimensions** disagree (section 5).
- **Edge kinds** are labelled but drawn alike (section 7).
- **Provider classes.** The design named `ShaclContextActionProvider` and `ShaclContextPropertyProvider`; the code delegates to static `ShaclActions` and `ShaclProperties` inside the family's single providers, a correction the design itself records (2026-09-04).
- **The starter document** inserts the base name into `sh:name "…"` without escaping quotes (`Shacl/ShaclDocumentFactory.cs:34`).
- **Manual checks** in `tests.md` (2026-09-05 and 2026-09-06): absent-target chips passed; the blank-row one-sentence refusal is partly verified; the SPARQL row cannot be exercised because no vendored file contains `sh:sparql`, and the negative half, that nothing is run, passed.

## 11. Examples

Real, found-online shapes on the main path, per the vendoring rule (R9; `src/diagrams/rdf/examples/shacl/`, with seeded, not byte-compared, copies under `src/examples/diagrams/shacl/`):

- `w3c-shacl/spec-examples.ttl`: the SHACL recommendation's §2.1 and §4.6.1 examples, verbatim blocks under a five-line prefix header, which is the declared modification; W3C Software and Document License, vendored as `LICENSE.md`, retrieved 2026-09-04. It exercises blank property shapes, an unreferenced IRI property shape drawn as its own card, the sequence path `ex:knows/ex:email`, `sh:or` with blank operands on a node shape and inside a property shape, and two target kinds (`ShaclExamples.Tests.cs:82-100`).
- `w3c-shacl/shacl-shacl.ttl`: the recommendation's Appendix C, the shapes for shapes graphs, byte-for-byte unmodified, same licence: 405 lines, ten node shapes, sixty-odd property shapes and nearly every SHACL Core term, including subjects-of and objects-of targets.
- `fair-data-point/navigation-shapes.ttl`: FAIRDataTeam/FAIRDataPoint's `defaultNavigationShacl.ttl`, MIT © 2017 FAIR Data Team, vendored `LICENSE.md`, retrieved 2026-09-04, renamed only. Five node shapes targeting an external vocabulary (every chip "not described in this file"), blank rows whose `sh:node` draws path-labelled edges with an empty summary and `[0..*]`, and an `sh:node` cycle between catalogue and dataset (`ShaclExamples.Tests.cs:103-126`). The showcase copy's registration stores five positions.
- Rejected: the SEMICeu DCAT-AP shapes (ISA Open Metadata Licence v1.1, not on the permissive list) and schema.org-derived shapes (share-alike) (R9.2).
- Not demonstrated by any vendored file: `sh:sparql`, a deactivated shape, a non-default severity and an implicit class target; unit tests pin them (`ShaclProjection.Tests.cs:56-69`, `ShaclShapeDiscovery.Tests.cs:99`, `ShaclTargetChips.Tests.cs:145`).

Every example registration must resolve and validate with no finding above info (R9.5).

## 12. What DISL 0.1 could not say, in short

For whoever takes DISL to 0.2:

1. **A foreign model with a reading's projection.** The model file belongs to other tools and to three sibling readings; which triples make an element (declaration-or-use discovery, the card rule, the claimant rule for `sh:class`) can only live in a plugin.
2. **Elements derived from data**, including an edge whose source is the card that holds the row that states it, and list items (targets, rows) that are not elements.
3. **Multi-column list items**: one row with aligned left, middle and right parts, and the identity each part needs when rendered.
4. **One menu entry per list item** and menu labels computed from the model ("Remove target: class ex:Person", "Remove shape (with 5 statements)").
5. **Unavailable-with-reason** menu entries and a read-only reason on every property row.
6. **A drop whose target is whatever element lies under it**, else a placement.
7. **Dialog details**: a placeholder per parameter is expressible through a form, a button label ("Add") is not.
8. **Findings on triples and lines** rather than on elements, one per offending triple, and a parse failure that replaces every other finding.
9. **Refusals that depend on the host**: no registration, no history.
10. **Ids from IRIs and parse ordinals**, and ids that must never be stored (`blank:`).
11. **Two budgets** with different measures, and a hard cut in a declared order as opposed to a soft warning.
12. **A layout whose order is the model's discovery order**, which a built-in grid cannot be told.
13. **Viewport delivery**, a runtime concern this tool's behaviour depends on.
