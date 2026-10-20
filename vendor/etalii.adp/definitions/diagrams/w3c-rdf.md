# RDF graph: what DISL cannot express

This file accompanies [w3c-rdf.dis](w3c-rdf.dis), the DISL specification of the RDF graph diagram (`w3c/rdf`, RDF 1.1 files stored as Turtle `.ttl` or N-Triples `.nt`). The `.dis` says what DISL can say: the metamodel of resource cards, blank-node cards and statements, the notation, the toolbox and context menus, the file-level findings, the gesture refusals, and where the Turtle engine and the type-band layout hand over to plugins. This file holds everything else: how the file is read and written, which rules turn triples into drawn elements, the layout algorithm, the dialogs and their validation, the refusals verbatim, the known gaps and the places where DISL 0.1 fell short. Each point names the code, specification or conversation that shows it.

`w3c/rdf` is the anchor reading of the RDF family. Its engine (tokenizer, parser, model, writer, store, reloader, commands, registration-header helper, selection vocabulary and context providers) is shared by three sibling readings over the same bytes, each specified in this folder: [w3c-owl.dis](w3c-owl.dis) (`w3c/owl`), [w3c-skos.dis](w3c-skos.dis) (`w3c/skos`) and [w3c-shacl.dis](w3c-shacl.dis) (`w3c/shacl`). Where this file describes the engine, the siblings' companion files refer to it rather than restating it.

Sources, as read on 2026-09-30:

- The standalone implementation on `develop` of etalii.adp.ide.standalone (commit `b2a2692`): `src/diagrams/rdf/` (backend, client, `api/rdf.proto`, examples, tests), core's `LineDocument.cs`, `RegistrationLayout.cs`, `DiagramDefinition.cs` and `GestureIds.cs`, the canvas library under `src/client/src/canvas/`, `docs/tools.md` line 137, `docs/creating-a-diagram-module.md`, `docs/screenshots/readme.md`, `.gitattributes` lines 120 to 124, and `tests.md`. Paths below written `backend/…` are under `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/`, `tests/…` under `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf.Tests/` and `client/…` under `src/diagrams/rdf/client/`.
- The spec-workflow specification `rdf-diagram` (requirements, design and tasks), removed from the tree by commit `53f68611` and read from its parent. Requirement numbers below (R1.1 and so on) are that specification's; its list of "shared machinery" items (item 1 to item 10) is cited the same way.
- The Notion "Tools" database row "RDF graph".
- This project's conversations from 2026-09-26 to 2026-09-30. None holds a user decision about RDF itself; what applies is the backend-centralization work that moved the RDF store onto the shared lifecycle (PR #35, #77, #78), the screenshot capture (PR #105), and the general rulings: the `.dis` extension (Peter, 2026-09-30), one DISL file per standalone tool in `definitions/diagrams/`, and definitions interpreted by a core plugin per IDE with extra code where a definition cannot say enough (Peter, 2026-09-26).

## 1. What the tool is for

An RDF graph shows what an RDF file states: its resources, their relationships and their values (the standalone description, `backend/Diagram.cs:23`). RDF is a graph written down as text; drawing it as one makes missing links, mistyped IRIs and unexpected shapes visible.

Catalogue data (Notion "Tools" row, `docs/tools.md:137`): kind Diagram, origin `w3c/rdf`, display name "RDF graph", family "Ontologies & semantic web", rarity "Adoption from standard", theory RDF 1.1 Concepts and Turtle, state ✅ Implemented in standalone. Focus areas: Knowledge and semantics, Clarity in textual data, Software delivery. One-line purpose: "Show the triples of an RDF document as a graph of resources and labelled links." Why specialized: "RDF is a graph written down as text; drawing it as one makes missing links, mistyped IRIs and unexpected shapes visible." The Notion page body is blank. The catalogue note in `docs/tools.md` reads "RDF data graph (resources as cards, triples as edges, literals as property rows) over the `.ttl`/`.nt` files a triplestore imports and exports". Standalone's icon is `mdi-graph-outline` (`backend/Diagram.cs:24`); the screenshot `docs/screenshots/rdf.png` is taken from `w3c-turtle/example-1.adp`.

The subject is files on disk, the serializations every triplestore imports and exports, not a live connection to a running store (requirements, introduction).

The research behind the drawing rules (requirements, "How RDF graphs are visualized"):

- **Adopted:** literals as property rows inside their subject's card, not as nodes (Ontodia's card convention, shared with UML object diagrams); prefixed names wherever a human reads, full IRIs one selection away in the property grid; type-driven grouping in a deterministic layout, because "what populations does this dataset hold" is the first question a reader asks of instance data; a bounded view as the scale answer (a drawn-element budget with an honest banner); directed labelled edges with arrowheads.
- **Rejected:** whole-graph force simulation (non-deterministic, unreadable and memory-unbounded at scale, the surveyed tools' documented failure mode); 3D and VR presentations; circular and hierarchical-edge-bundling overviews, which have no stable element identity to select, position or edit. VOWL's literal-nodes convention is left to the OWL reading.

## 2. The file and the shared engine

DISL's persistence layer describes a file a runtime writes in its own format. Here the model file is a foreign format other tools own, which DISL can only name as `format: "plugin:net.etalii.adp.w3c.turtle"`. What that engine does:

**The parser** (`backend/RdfTokenizer.cs`, `backend/RdfParser.cs`) is hand-written recursive descent, "because the splice discipline needs spans … and no available library reports positions at that grain" (`backend/RdfParser.cs:13-18`). It reads:

- directives `@prefix p: <iri> .`, `@base <iri> .` and the SPARQL-style `PREFIX` and `BASE` (case-insensitive, no dot) (`backend/RdfParser.cs:49-125`); prefix IRIs resolve against the base in force, a new base against the previous one, `<>` means the base itself, and a relative IRI without a base stays relative rather than being guessed (`backend/_Model/IriTerm.cs:10`);
- subjects: an IRI, a prefixed name, a labelled blank `_:x`, a `[ … ]` property list (also `[ p o ] .` as a whole statement) and a collection `( … )`; verbs `a`, IRIs and prefixed names; objects: IRIs, prefixed names, labelled blanks, strings, numbers, booleans, `[ … ]` and `( … )` (`backend/RdfParser.cs:132-268`);
- predicate lists with `;` (repeated and trailing `;` allowed before `.` or `]`) and object lists with `,` (`backend/RdfParser.cs:184-193`);
- all four string quotings (`"…"`, `'…'`, `"""…"""`, `'''…'''`), the escapes `\t \b \n \r \f \uXXXX \UXXXXXXXX` (any other `\c` reads as `c`), `@lang` and `^^datatype` (`backend/RdfParser.cs:270-300, 527-565`); bare `42`, `1.5`, `1e3`, `true` and `false` typed `xsd:integer`, `xsd:decimal`, `xsd:double` and `xsd:boolean` (`backend/RdfParser.cs:246-260`); a language literal carries `rdf:langString` as its datatype (`backend/RdfParser.cs:282`);
- prefixed-name local parts with `\` escapes decoded and `%XX` kept as written; IRIs with only `\u`/`\U` escapes decoded, never spanning a line; `#` comments wherever trivia is allowed. The tokenizer is "not a validator" and is looser than Turtle's character tables (`backend/RdfTokenizer.cs:9-12`).

N-Triples has no reader of its own: it is parsed by the same Turtle parser as a subset (`backend/RdfParser.cs:9-11`). RDF-star (`<< >>`), TriG graphs, N-Quads, literals or numbers as subjects and `@version` are not supported and fail as parse errors, not by name.

**Parse errors** are carried in the store entry rather than thrown, with a 1-based line (`backend/RdfDocumentStore.cs:89-105`). Their sentences include "Expected a subject, found '…'.", "Expected a predicate, found '…'.", "Expected an object, found '…'.", "'^^' expects a datatype IRI.", "The prefix 'p:' is not declared.", "A lone '^' is not a Turtle token; the datatype marker is '^^'.", "A blank node label starts with '_:'.", "An IRI ran to the end of its line without its closing '>'.", "A string ran to the end of its line without its closing quote.", "A long string ran to the end of the file without its closing quotes.", "A lone '@' is not a Turtle token." and "'word' is not a Turtle keyword, and a prefixed name needs its ':'." (`backend/RdfParser.cs:83-458`, `backend/RdfTokenizer.cs:111-328`). The template for an unmet expectation is "Expected {expectation}, found '{token or end of file}'.".

**The model** (`backend/_Model/`): terms are a closed set of `IriTerm` (identity is the full IRI, `AsWritten` keeps the spelling), `BlankTerm` (label and a zero-based ordinal of first appearance within one parse) and `LiteralTerm` (lexical form, datatype, language, spelling). Every triple keeps its line span, its statement, the offsets of its own tokens and its object, and the offset of the statement's terminator; a triple "owns" its predicate and object in a first pair or a `;` continuation, and only its object in a `,` continuation (`backend/_Model/SourceSpan.cs:9-14`, `backend/_Model/RdfTriple.cs:25-41`). Prefix declarations are all kept, re-declarations included, and the last one wins (`backend/_Model/RdfModel.cs:11-35`).

**The store lifecycle** (`backend/RdfDocumentStore.cs`, `backend/IRdfDocumentStore.cs`, `backend/RdfDocumentReloader.cs`), moved onto core's shared `WritableDocumentLifecycle` in backend-centralization task 6 (PR #35) and onto the shared save result in PR #77:

- one store per process (`TryAddSingleton`, `backend/ServiceCollection.AddRdf.cs:43-46`), so every reading and every connection on one file shares one parsed document, and an edit through any reading is visible in all (R2.4);
- a missing file opens as an empty document, so a new diagram opens and its first save creates it (`backend/IRdfDocumentStore.cs:26-31`);
- a reload that cannot read keeps the last good document; only the watcher's delete clears it; a save ignores its own echo; the folder is created on first save (`backend/RdfDocumentStore.cs:9-32`);
- `Save` takes the edited entry as a parameter, the fix for a lost-edit defect where a reload between edit and save discarded the edit but reported success (`backend/IRdfDocumentStore.cs:38-46`, `tests/RdfDocumentStore.LostEdit.Tests.cs`);
- the reloader overrides `BodyDeleted` explicitly, with the warning "DROPPING THE OVERRIDE LOOKS HARMLESS AND IS NOT" (`backend/RdfDocumentReloader.cs:11-19`); a reload also covers a change to the `.adp`, which is how a stored reposition reaches other connections;
- a file that does not parse is never written: "{file} does not parse, so it was not written. {error}" (`backend/RdfDocumentStore.cs:46-52`), and every command refuses first with "This file does not parse, so nothing can be edited until it is fixed." (`backend/Commands/RdfEdits.cs:26-30`, R1.5).

**Line endings and the byte-identical round trip** (core `LineDocument.cs`; R1.3, R1.4, R1.8): each line keeps its own terminator; an untouched document's text is byte-identical to what was read; there is no phantom trailing line; an inserted line takes the document's dominant ending (LF when LF lines outnumber CRLF lines, otherwise CRLF, so ties and empty documents get CRLF); a replaced range's last line inherits the replaced terminator; and appending to an unterminated last line moves the missing terminator so append and undo are byte-exact. Byte-compared fixtures are `-text` in standalone's `.gitattributes` (`*.ttl -text`, `*.nt -text`, with the reasoning at lines 120 to 124). The parser corpus is `tests/Fixtures/`: `constructs.ttl` (every construct R1.1 names, CRLF), `crlf-line-endings.ttl`, `lf-line-endings.ttl`, `tied-line-endings.ttl`, `no-trailing-newline.ttl`, `simple.nt` and `broken.ttl`.

**The splice discipline** (item 3; `backend/RdfWriter.cs`). Every edit is a named writer operation that returns either nothing or a refusal sentence; a refused document is untouched "to the byte" (`backend/RdfWriter.cs:8-9`). No operation ever invents a prefix declaration (`backend/RdfWriter.cs:18-20`, R5.2).

- **Add a triple** (`backend/RdfWriter.cs:33-88`). The predicate is written `a` for `rdf:type`, otherwise compressed to a prefixed name or bracketed `<iri>`; a literal as `"escaped"@lang`, `"escaped"^^datatype` (omitted for `xsd:string`) or `"escaped"`, escaping `"`, `\`, LF, CR and TAB. If the subject has no statement yet, `{subject} {predicate} {object} .` is appended at the end of the file after an empty line. Otherwise the subject's last statement's `.` becomes `;` in place and the new pair goes on the next line, indented like the statement's second line, or the subject line plus four spaces. A blank-node object is refused.
- **Remove a triple** (`backend/RdfWriter.cs:95-178`). The only triple of a statement takes the statement's whole line range; an object-list member takes its object plus one adjacent `,`; any other pair takes predicate and object plus one adjacent `;`. Lines left holding only whitespace are deleted. A triple no longer in the file answers "That triple is not part of the document as it stands - the file may have changed since it was read.".
- **Remove a resource** (`backend/RdfWriter.cs:184-225`): every triple naming the IRI as subject or object is removed bottom-up, reparsing after each removal. "Nothing in the document names {iri}, so there is nothing to remove." when there are none.
- **Rename a term** (`backend/RdfWriter.cs:231-269, 488-555`): every token naming the old IRI (bracketed IRIs resolved against the base in force, prefixed names under the prefixes in force at that point, datatypes) is rewritten right to left as the compressed new IRI; the keyword `a` and prefix-directive namespaces are left alone. Refusals: "The new name is the same as the old one, so there is nothing to rename.", "{newIri} already names something in this document, so renaming onto it would silently merge two resources. Pick an unused IRI." (the new IRI in any subject, predicate, object or datatype position) and "Nothing in the document names {oldIri}, so there is nothing to rename.".
- **Replace a literal** (`backend/RdfWriter.cs:280-314`): exactly the object's characters are replaced by a short double-quoted string plus `@lang` or `^^datatype` (language wins). The original quoting style (`'…'`, `"""…"""`) is not preserved. "That value is not a literal, so it cannot be rewritten as one. Rename or reconnect the resource instead." for a non-literal.
- **Add a prefix** (`backend/RdfWriter.cs:317-337`): `@prefix {p}: <{iri}> .` on the line after the last prefix declaration, else after the base, else at the top, always in `@prefix` form even in a file that uses `PREFIX`. Refusals: "The prefix '{p}:' already expands to {iri}, so there is nothing to add." and "The prefix '{p}:' is already declared as {existing}. Redeclaring it would change what every use of it means.".

N-Triples flows through the same operations "degenerately" (`backend/RdfWriter.cs:20-22`): terms are written as full `<iri>` because an `.nt` declares no prefixes. But adding a triple to a subject that already has a statement turns its `.` into `;`, which is Turtle syntax: the file still parses here but is no longer valid N-Triples. Nothing in the code or the specification guards this.

**Commands and undo** (`backend/Commands/`, `backend/Commands/RdfEdits.cs:19-43`): `AddRdfTripleCommand`, `RemoveRdfTripleCommand`, `RemoveRdfResourceCommand`, `RenameRdfTermCommand`, `ReplaceRdfObjectLiteralCommand` and `AddRdfPrefixCommand` each run one writer operation through the project's history and save. The inverse is deliberately a whole-document snapshot, `RestoreDocumentCommand<IRdfDocumentStore>`, which writes the old text back and reloads; redo re-runs the original command. `RemoveRdfTripleCommand` answers a missing triple with "That triple is not in the file as it stands, so there is nothing to remove." and `ReplaceRdfObjectLiteralCommand` a missing value with "That value is not in the file as it stands, so there is nothing to rewrite.".

**New files** (`backend/RdfDocumentFactory.cs:21-49`) are CRLF with a trailing newline and validate clean:

```turtle
@prefix ex: <http://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

ex:{local} a ex:Resource ;
    rdfs:label "{baseName}" .
```

`{local}` is the base name with every character other than an ASCII letter, digit, `-` or `_` replaced by `_`, trimmed of `_`, or `untitled` when nothing is left. The base name goes into the label unescaped, so a `"` in a file name would break the literal.

**The serialization boundary** (item 2, R1.6) was specified as RDF/XML, JSON-LD, TriG and N-Quads opening unavailable, each refused by name with its reason. The code has no named refusal: only `.ttl` and `.nt` route to the family at all ("everything else is refused upstream at the definition, not here", `backend/IRdfDocumentStore.cs:13-14`), and a `.ttl` that holds RDF/XML fails with a generic tokenizer message.

## 3. The registration and where positions live

The `.dis` declares `files.mode: "split"` with the model in `{name}.ttl` and the view in `{name}.adp`, `view.store: ["bounds"]`, and an import entry for `.nt`. What that stands for is standalone's registration and routing, which DISL does not describe:

- **Routing** (item 7, R2.2). A bare `.ttl` or `.nt` with no `.adp` routes to `w3c/rdf` on sight (`backend/Diagram.cs:11-15`; `tests/Diagram.Tests.cs`). `.nt` is carried by core's `AlternateExtension` field (`backend/Diagram.cs:26`): the alternate routes and is claimed like the primary but derives no registration sibling, so a registered `.nt` names itself with an explicit `body:` header, which Add writes anyway (`docs/creating-a-diagram-module.md:29`). The old design left `.nt` routing open; the new core field resolved it. `SharedExtension` is not set and `SuggestsBody` is null, "the extension alone decides, which is every anchor type's behaviour", which is why the `.dis` has `claimsBareFiles: true` and an empty `suggestWhenBodyContains`.
- **The siblings never claim a bare body.** `w3c/owl`, `w3c/skos` and `w3c/shacl` set `SharedExtension: true` over the same extensions and are suggested by Add only when the text contains `owl:Ontology`, `skos:ConceptScheme`, or `sh:NodeShape`/`sh:property` (or the full IRIs), a deliberately textual test, "not whether the file is valid" (`backend/Diagram.cs:34-99`). tests.md (2026-09-06) confirms a bare ontology opens as the RDF graph and Add offers both readings.
- **The registration file** is plain text: the type line `w3c/rdf`, a `body:` header resolved against the `.adp`'s own folder (R2.5), and an optional `layout:` block of two-space-indented `<element-id>: <x> <y>` lines, for example `res:http://example.org/#spiderman: 292.254 -19.97` (`src/examples/diagrams/rdf/w3c-turtle/example-1.adp`). Core writes the block sorted by id, numbers as `0.###` in invariant culture, new lines as CRLF, everything above the block byte for byte, and removes the block when it becomes empty (`RegistrationLayout.cs:184-200`).
- **Ids in the block** are `res:{full IRI}`, "safe in the block's grammar because IRIs cannot contain spaces" (`backend/RdfSession.cs:14-17`). Only `res:` ids are ever stored.
- **The registration header facility** (item 10; `backend/RdfRegistrationHeaders.cs:16-76`) is defined here for the family and not used by this reading. A reading header is a `key: value` line after `body:`/`view:` and before `layout:`, because "core's pairing scan stops at the first line it does not recognize, so a header above `body:` would silently sever the pairing"; the helper skips the type line, scans at most eight further lines, stops at `layout:`, matches keys case-insensitively and answers null on IO errors. Header names are single lowercase words claimed in the anchor specification before use; `language:` is claimed by `w3c/skos`.
- **Moves write only the registration** (R4.1). A drag dispatches core's `SetRegistrationLayoutCommand(registrationPath, elementId, x, y)` through the project history, one undo that returns the `.adp` byte for byte; the RDF body never changes (`backend/RdfSession.cs:114-151`; test `ARepositionLandsInTheRegistration_IsOneUndoAway_AndTheBodyNeverChanges`; tests.md 2026-09-03 on the Wikidata example, LF endings preserved). The stored point is the card's top-left in module units; the client converts to and from the card's centre (`client/RdfCanvas.tsx:191-192, 226-234`).
- **Stale positions are never pruned.** R4.2 required stale ids to be "dropped on the next layout write"; core has `RegistrationLayout.Prune`, but nothing in the RDF module calls it, so a stale entry stays in the `.adp` and simply overrides nothing. A rename therefore leaves the old `res:` position behind and the renamed card falls back to its computed place.

## 4. From triples to drawn elements

DISL's metamodel describes elements a user creates. Here every element is derived from the triples by a pure function, `RdfProjection.Project(model, budget)` (`backend/RdfProjection.cs`), and sent as three element kinds on the wire (`backend/RdfElementMapper.cs`, `api/rdf.proto`): `w3c/rdf+resource` (`RdfResourcePayload`: iri, display, `type_badges`, literal rows, blank), `w3c/rdf+edge` (`RdfEdgePayload`: from, to, predicate display, predicate IRI) and `w3c/rdf+truncation` (`RdfTruncationPayload`: shown, total). The proto field is `type_badges` rather than `types` because protobuf's nested `Types` class claims that name in C# (`api/rdf.proto:22-24`).

**Nodes.** Triples are walked in document order (`backend/RdfProjection.cs:60-118`):

- every IRI in subject or object position is one card, id `res:{full IRI}`, created on first appearance; a predicate is never a card;
- every blank node that is not a collection cell is one card, id `blank:{ordinal}`, titled `_:{label}` or `_:b{ordinal}` for an anonymous `[ ]` (`backend/RdfProjection.cs:147-158`); ordinals restart with every parse, which is the root of the blank-node identity boundary;
- literals are never cards (R3.2).

**Rows.** A triple whose object is a literal appends a row to its subject's card: the predicate's display form, the decoded lexical value and an annotation (`backend/RdfProjection.cs:85-90, 219-224`). The annotation is `@{lang}` for a language literal; for a datatype other than `xsd:string` it is the datatype compressed to a prefixed name, else the bracketed full IRI (not the local-name fallback titles use); plain and explicit `xsd:string` literals have none. Rows keep document order and duplicates.

**Type badges.** An `rdf:type` triple with an IRI object is not an edge: it appends the type's display form to the subject's badges, in document order with duplicates (`backend/RdfProjection.cs:75-80`, R3.3). The type IRI does not become a card from that triple, so a class appears as a card only if another triple names it as subject or object. An `rdf:type` with a blank object is an ordinary edge; with a literal object, an ordinary row.

**Edges.** A triple whose object is an IRI or a blank node is an edge from subject to object, labelled with the predicate's display form (`backend/RdfProjection.cs:107-116, 167-174`). Its id is `edge:{fromId}|{predicateIri}|{toId}`, with `|{n}` appended for the second and later repeats of an identical triple; `|` is safe as a separator "because an IRI cannot carry it unencoded" (`backend/RdfSelection.cs:13-17`). Self-loops are projected.

**Collections** (R3.5). The parser expands `( a b c )` into cons cells, one fresh anonymous blank per member with `rdf:first` and `rdf:rest`, ending in `rdf:nil`; an empty `()` is the IRI `rdf:nil` (`backend/RdfParser.cs:356-396`). The projection skips `rdf:first`/`rdf:rest` triples on cons cells as plumbing and draws a triple whose object is a collection as one edge per member labelled `{predicate} [{n}]` (1-based) with the original predicate IRI; literal members become rows with the same label (`backend/RdfProjection.cs:32-53, 62-105, 176-198`). Cons cells are never cards. Three shapes are silently not drawn as a consequence: a triple whose subject is a collection (`( a b ) ex:p ex:o .`), a collection nested inside another, and the cells themselves. An explicit `rdf:nil` object, or an empty `()`, draws as a card for `rdf:nil`.

**Display forms** (`backend/RdfProjection.cs:201-217`, `backend/RdfWriter.cs:343-371`), the "fallback chain" of R3.1, which the `.dis` hands to the plugin function `rdfDisplayName`:

1. a prefixed name under the longest declared namespace that is a proper prefix of the IRI, if the remainder is a clean local name (not empty, not starting or ending with `.`, only letters, digits, `_`, `-` and `.`); ties on namespace length go to the later declaration, and every declaration counts, re-declared ones included;
2. else the text after the last `#` or `/`, if not empty;
3. else the whole IRI, unbracketed.

Titles, badges and edge labels use this chain; row annotations and dialog pre-fills use the compressed form with its bracketed fallback (`rdfCompress` in the `.dis`).

**Containment.** There is none: no groups, no nesting, no ports beyond the two side anchors. A reparent is refused with "A resource has no parent to move it under; dragging changes where it sits on the canvas." (`backend/RdfSession.cs:103-112`).

**The drawn-element budget** (item 5, R8; `backend/RdfProjection.cs:23-24, 120-129`). The budget is 1000 drawn cards. The pure function keeps the first 1000 cards in document order and only edges whose ends both survive. The session does something different (`backend/RdfSession.cs:19-30, 166-227`): it projects and lays out the *whole* document, filters by the viewport the client reports, and applies the budget only to the visible set, "a floor against a pathological view". The banner's counts stay the document's: when the document projects to more than 1000 cards the payload says `shown = 1000` and `total` = the document's card count, whatever is on screen, and panning reaches cards beyond the first thousand (test `PanningReachesResourcesTheBudgetDiscarded`). Read-only is decided from the document alone, `RdfProjection.Project(model).Truncated` (`backend/RdfSelection.cs:102-104`). DISL's `language.limits.maxElements` only asks a runtime to warn, so the cut, the banner and the edit gate are carried in the `.dis` as the `truncated`, `shown` and `total` diagram attributes, the watermark and the `truncated*` gesture constraints.

**Viewport delivery** (`backend/RdfViewport.cs:25-42`, `backend/RdfSession.cs:81-101, 217-220`). A card is sent when its reserved 280 by 160 layout cell overlaps the reported viewport, or when it has no position; an edge only when both its ends are sent (test `NoEdgeIsEverDeliveredWithOneEndMissing`). The first delivery is one add of everything visible; later view changes are diffed with the shared `DiagramDiff`, removals before additions (PR #78; test `AViewportChange_EmitsRemoveBeforeAdd`). Because layout is computed over the whole document, panning never moves a card (test `PanningDoesNotMoveTheNodesItBringsIntoView`). The client folds deltas by id, checks the payload type before decoding because modules share one delta stream, and clears the banner when the `truncation` element is removed (`client/rdfModel.ts:182-267`). The viewport is not persisted.

## 5. Layout

DISL names a layout; it does not define one. The `.dis` names `plugin:net.etalii.adp.w3c.rdfTypeBands` with the built-in `lanes` as fallback, keyed by the derived `band` attribute. What the plugin does (`backend/RdfLayout.cs`; R3.3, R4.4):

- Pure and deterministic, no physics and no randomness: the same file always opens the same way (`backend/RdfLayout.cs:5-10`).
- Cards are grouped in bands by their **first** type badge's display text; untyped cards share the band keyed "" (`backend/RdfLayout.cs:36`). Blank-node cards are banded the same way.
- Typed bands come first, ordered by the badge text with ordinal comparison, and the untyped band last (`backend/RdfLayout.cs:37-38`). The old design said "ordered by type IRI"; the code orders by display form.
- Within a band, cards keep projection order (document order of first appearance) and fill a grid of four columns, left to right then top to bottom: `x = column × 280`, `y = bandTop + row × 160` (`backend/RdfLayout.cs:18-26, 48`).
- The next band starts `rows used × 160 + 80` below the previous band's top; the first card of the first band is at (0, 0) and bands stack downward (`backend/RdfLayout.cs:25, 57`).
- Positions are card top-lefts. The client draws a 220-wide card inside the 280-wide cell, leaving 60 units between columns. Card height follows content while the cell is a fixed 160, so a card with many rows (the Wikidata example has hundreds) overlaps the band below.
- Stored positions overlay the computed ones element by element through core's `RegistrationLayout.Apply`; a stored id that is not drawn is ignored (`backend/RdfSession.cs:178-184`). The projection and layout are cached until the document changes (`backend/RdfSession.cs:50-56`).
- The client declares `layout: { modes: ["manual"] }` and `dragging: "enabled"` (`client/RdfCanvas.tsx:168-169`): the backend lays out and the canvas never runs a layout of its own.

The DISL `trigger: "onLoadIfMissing"` with `respect: "all"` is the nearest reading; DISL cannot say that the computed layout is recomputed on every change for every card without a stored position while nothing computed is ever written.

## 6. Interaction

DISL can declare tools, context menus, operations with parameters, a create form and gesture constraints. It cannot carry which entries the backend discovers and when, the dialog wording beyond a label, the validation of typed terms, or refusal sentences that depend on the selection. Standalone's interaction runs through the family's context providers (`backend/RdfContextActionProvider.cs`, `backend/RdfContextPropertyProvider.cs`, `backend/RdfContextSourceResolver.cs`), registered once for all four readings (`backend/ServiceCollection.AddRdf.cs:61-65`).

**Toolbox** (`backend/RdfToolboxProvider.cs:15-29`, registered for `w3c/rdf` only):

| Id | Label | Icon | Tooltip | Runs |
| --- | --- | --- | --- | --- |
| `rdf.toolbox.resource` | Resource | `mdi-card-plus-outline` | "An IRI-named resource. Drop it where it should sit; you will be asked for its name." | `rdf.add-resource` |
| `rdf.toolbox.prefix` | Prefix | `mdi-at` | "A namespace declaration, so terms can be written short." | `rdf.add-prefix` |

A drop runs the item's action with a placement id `new:{x},{y}` (core `GestureIds`, `client/RdfCanvas.tsx:240`). The drop position is **not** stored: the backend comment says "The authored drop position is the client's follow-up layout write" (`backend/RdfContextActionProvider.cs:430-431`), but `RdfCanvas` makes no such write and the canvas library has none, so a new resource takes its computed band position. R6.1 named only a resource entry; the Prefix entry is an addition.

**Context menus** (`backend/RdfContextActionProvider.cs:89-193`). Discovery returns nothing when the target is not this family's (origin vendor `w3c`, or a `.ttl`/`.nt` extension when the origin is unknown, `backend/RdfSelection.cs:51-54`), when the file does not parse, or when the document is truncated. By selection:

| Selection | Entries (label, icon, shortcut) |
| --- | --- |
| IRI card | "Rename…" `mdi-pencil-outline` F2; "Remove" `mdi-delete-outline` Delete, which reads "Remove (with {n} statements)" when the resource appears in more than one statement |
| Edge between two IRI cards, for a triple still in the file | "Remove statement" `mdi-vector-polyline-remove` Delete |
| Placement `new:x,y` (a toolbox drop) | "Add resource here…" `mdi-card-plus-outline`; "Declare prefix…" `mdi-at` |
| Relation gesture `rel:{from}->{to}` | "Relate…" `mdi-ray-start-arrow` |
| Blank-node card, the banner, anything else | nothing from this reading |

Group order is shortcut order: core's `ContextActionResolver` flattens a provider's groups and takes the first match (`backend/RdfSelection.cs:42-44`). The client declares the two shortcuts as actions: `rename` bound to F2 on elements and `delete` bound to the library's delete gesture on elements and connections (`client/RdfCanvas.tsx:164-167`). The `.dis` cannot hide menu entries per placement id or gesture id, so its `rename` and `delete` entries are the ones on cards and statements, and the placement entries are the toolbox operations.

**Dialogs and what they write** (`backend/RdfContextActionProvider.cs:197-446`):

| Action | Dialog (title, icon, placeholder, confirm) | Writes |
| --- | --- | --- |
| Rename | "Rename resource", `mdi-pencil-outline`, "New IRI or prefixed name", pre-filled with the compressed IRI, "Rename" | `RenameRdfTermCommand` |
| Remove resource | no dialog when the resource is in one statement; otherwise the confirmation "Remove resource", `mdi-delete-outline`, "Removing this resource also removes the {n} statements it appears in.", "Remove", marked as danger | `RemoveRdfResourceCommand` |
| Remove statement | none | `RemoveRdfTripleCommand` |
| Relate | "Relate", `mdi-ray-start-arrow`, "Predicate (IRI or prefixed name)", "Relate" | `AddRdfTripleCommand(from, predicate, to)` |
| Add resource | "Add resource", `mdi-card-plus-outline`, "IRI or prefixed name", "Add" | `AddRdfTripleCommand(iri, rdf:type, rdfs:Resource)`, "typed as the most general thing there is" |
| Declare prefix | "Declare prefix", `mdi-at`, "Declaration, e.g. ex: http://example.org/", "Declare" | `AddRdfPrefixCommand` |

The `.dis` confirmation for removing a resource says the same sentence without the count, because DISL's `deletion.confirm` is plain text, and it cannot skip the confirmation for a one-statement resource. The Relate dialog is the `.dis`'s `relateDialog` create form plus the `resolvePredicate` hook.

**Typed-term validation** (`backend/RdfTermInput.cs:14-53`), for rename, relate and add resource, carried in the `.dis` as the plugin functions `rdfResolveTerm` and `rdfTermRefusal`:

- empty: "A term needs a name: a full IRI, or a prefixed name like ex:thing.";
- a space anywhere: "An IRI cannot contain spaces.";
- `<…>`: the inner text is the IRI, which is how a scheme-only IRI such as `urn:…` is written; empty brackets: "The brackets are empty.";
- text containing `://`: taken verbatim as a full IRI;
- no colon: "'{x}' is neither a full IRI nor a prefixed name. Write ex:{x}, or a full IRI." (so the keyword `a` is refused; write `rdf:type` under a declared `rdf:` or the full IRI, and the new statement then draws as a badge);
- an undeclared prefix: "The prefix '{p}:' is not declared in this file. Declare it first, write the full IRI, or bracket a scheme IRI as <{x}>.";
- otherwise the prefix's expansion plus the local part, with no decoding or checking of the local part.

Rename also refuses a collision in the dialog: "{resolved} already names something in this document; renaming onto it would silently merge two resources." (subject and object positions only; the writer re-checks predicates and datatypes). A prefix declaration must read `prefix: iri` with a non-empty prefix, no spaces and an absolute IRI (brackets tolerated), otherwise "Write the declaration as 'prefix: iri', e.g. ex: http://example.org/."; the default empty prefix cannot be declared this way (`backend/RdfContextActionProvider.cs:347-351, 448-462`). Add resource does not check whether the IRI exists: on an existing resource it appends `a rdfs:Resource` to its statement. On commit, a truncated document answers with the truncation sentence and an action that does not fit the selection with "'{actionId}' does not apply to this selection." (`backend/RdfContextActionProvider.cs:357-381`).

**Relation gesture refusals** (`backend/RdfContextActionProvider.cs:280-295`): a blank end, "A blank node's identity does not survive a reparse, so relations to it cannot land in the file. Name it with an IRI first."; a placement end, "Drop the relation on a resource; a statement needs both of its ends.". On the client a relation can start only from a resource card's side anchors, can end on a resource or blank card, and cannot end where it started (`allowSelf: false`, `client/RdfCanvas.tsx:152-156`); that last refusal is silent in standalone, and the sentence on the `.dis`'s `gestureNotToSelf` constraint is the specification's own.

**Move refusals** (`backend/RdfSession.cs:118-151`): "This diagram is read-only." without a project history; "This diagram was opened without a registration, so there is nowhere to store a position. Register the file to arrange it." for a bare file (R4.3); "That element is not something this diagram can move." for an edge or the banner; "That is a blank node, whose identity does not survive a reparse, so a stored position could not be trusted. Name it with an IRI to arrange it." (R4.5). Moves stay allowed on a truncated document, because they write only the registration.

**The truncation gate** (`backend/RdfSelection.cs:20-22`): "The diagram shows only the first part of this file under the drawn-element budget, so edits through it are withheld - an edit through a partial view could touch what the view does not show. Edit the file as text instead." It answers actions, commits and property writes, and is the read-only reason on the Label and Comment rows. tests.md (2026-09-03) confirms on the Nobel laureates file that right-clicking a card under truncation opens no menu at all.

**The blank-node identity boundary** (item 4): the writer's refusal is "That element is rooted in a blank node, which has no identity that survives a reparse, so this edit cannot land in the file. Name the node with an IRI to edit it." (`backend/RdfWriter.cs:26-27`). It applies to every triple with a blank subject or object, so a resource that is related to any blank node cannot be removed from the canvas at all.

**Property rows** (`backend/RdfContextPropertyProvider.cs:82-142`), groups Identity then Documentation:

| Selection | Row | Value | Editable | Read-only reason |
| --- | --- | --- | --- | --- |
| IRI card | IRI (`rdf.iri`) | full IRI | no | "Rename through the context menu, so every reference follows the name." |
| IRI card | Types (`rdf.types`) | badges joined with ", " | no | "Types are stated by triples; add or remove them as statements." |
| IRI card | Label (`rdf.label`) | first `rdfs:label` literal, lexical form only | yes, unless truncated | the truncation sentence |
| IRI card | Comment (`rdf.comment`) | first `rdfs:comment` literal | yes, unless truncated | the truncation sentence |
| Edge | Predicate (`rdf.predicate`) | display form | no | "A statement's predicate is the statement; remove and restate to change it." |
| Blank card | Node (`rdf.iri`) | `_:label` or `_:bN` | no | "A blank node's identity does not survive a reparse, so nothing about it can be edited from the diagram." |

Setting Label or Comment rewrites the first matching literal and keeps its language and datatype, or adds a plain `rdfs:label`/`rdfs:comment` triple when none exists; an empty value writes `""`, there is no removal path, and only the first of several labels (for example multilingual ones) is reachable. An unknown property answers "'{propertyId}' cannot be edited on this selection." (`backend/RdfContextPropertyProvider.cs:148-211`). A resource with OWL axiom triples also shows read-only Axioms rows (Subclass of, Equivalent to, Disjoint with, Domain, Range, Inverse of, Characteristic) with the reason "Axioms are stated by triples; draw or remove them as edges and statements." (`backend/RdfContextPropertyProvider.cs:219-263`).

**Selection** (`backend/RdfContextSourceResolver.cs:42-91`, `backend/RdfSelection.cs:56-140`): an element must be selected within its diagram ("A diagram element must be selected within its diagram."), the routed file must be a `w3c` body ("The selected file is not an RDF document."), the id must still describe something ("That element is no longer in this file.") and the client's path must match ("The path does not match the element."). Descriptions are the display form for a card, `_:label`/`_:bN` for a blank, `{from} → {to}` for an IRI-to-IRI edge and "Truncated view" for the banner. Edges with a blank end and collection member edges describe as nothing, and a blank node that only occurs as an object is not found either, so these are drawn but cannot be selected; the `.dis` says so with the Statement's `selectable` expression. Selections are re-described on every store change and cleared when the element is gone.

**No inline rename.** The drawn labels are derived: "changing it renames that term across the whole document, with collision and prefix rules whose refusals need more room than a textbox has" (`client/readme.md`). Every label in the `.dis` is `editable: false` and double-click does nothing.

**What the sibling readings add under this reading.** The context seam does not carry which registration selected an element, so some sibling behaviour is data-driven and appears when a `.ttl` opened as `w3c/rdf` asserts the matching types:

- OWL: when the file contains `?x a owl:Ontology`, a placement also offers "Add class here…" `mdi-shape-circle-plus`, "Add object property…" `mdi-ray-start-arrow`, "Add datatype property…" `mdi-form-textbox` and "Add individual…" `mdi-account-outline` (each a dialog with placeholder "IRI or prefixed name" writing one `rdf:type` triple), and a relation between two OWL classes offers "Subclass of" `mdi-file-tree` above "Relate…" (`backend/RdfContextActionProvider.cs:145-189`, `backend/Owl/OwlSelection.cs:124-126`). The OWL Axioms property rows above are the same case. See `w3c-owl.md`.
- SKOS: `SkosActions` and `SkosProperties` carry no origin check, so a resource the file types as a SKOS concept, scheme or collection gets the scheme reading's context entries and its whole property grid (`backend/RdfContextPropertyProvider.cs:65-72`). See `w3c-skos.md`.
- SHACL: its actions and property rows answer only for the `w3c/shacl` origin (`backend/Shacl/ShaclActions.cs:267`, `backend/Shacl/ShaclProperties.cs:217`), so they do not appear here.

This contradicts the remark in `backend/RdfSelection.cs:38-45` that "a vocabulary opened as a plain data graph stops offering scheme verbs"; it is recorded as the behaviour of the current code, not as a design rule, and the `.dis` does not declare the sibling entries.

## 7. Notation the `.dis` approximates

The standalone canvas is a declarative definition for the central canvas library (`client/RdfCanvas.tsx`, `client/rdf.css`, `src/client/src/canvas/canvas.css`). Where DISL's notation is close but not exact:

- **Cards.** The library's `box` with no corner radius, 220 wide, height `30 + (16 when badged) + 16 × rows + 10` (`client/RdfCanvas.tsx:17-31`). Fill is the surface colour and the outline `var(--color-border)` at 1.5 (`canvas.css:166-170`); because the theme defines `--color-border`, the outline is `#e2e8f0` light and `#334155` dark, not the `#8892a6` fallback. The `.dis` keeps the family's token block, which includes that fallback as `color.node.outline`, and strokes cards with `color.border`.
- **Title.** The display form, 12 px, centred, 20 below the top edge, truncated to the card (`client/RdfCanvas.tsx:61-67`, `canvas.css:202-208`).
- **Badge line.** The type badges joined with " · ", 11 px in the primary colour (`limegreen` in both themes), left-aligned 8 in, 40 below the top, shown only when there are badges (`client/RdfCanvas.tsx:68-76`, `client/rdf.css:7-11`).
- **Rows.** One line per literal row, "{predicate}: {value} {annotation}", 11 px muted, left-aligned 8 in, 16 apart, starting at `30 + (16 when badged) + 12` (`client/RdfCanvas.tsx:77-88, 203`, `client/rdf.css:13-18`), truncated to the card width by the library's label machinery using the shared metric of characters × font size × 0.55 (the user's ruling of 2026-09-27 that every client width estimate uses the backend's metric). DISL compartments cannot state the exact line pitch or that rows are not wrapped.
- **Anchors.** A resource card has two anchors at the midpoints of its left and right sides, named `w` and `e` (`client/RdfCanvas.tsx:90-96`); the `.dis` models them as fixed anchor points and ports. A blank card has anchor mode `edge`: edges attach to its outline and no relation starts from it (`client/RdfCanvas.tsx:142`).
- **Blank cards.** Dashed `5 3` at 75 % opacity, "visibly provisional - the identity boundary's look" (`client/rdf.css:20-24`).
- **Edges.** Straight, 1.5 px in the muted colour, solid, with the library's filled triangle arrowhead (`M 0 0 L 10 5 L 0 10 z`, 6 by 6, filled with the line colour), the label 6 above the midpoint at 11 px muted, and a transparent 14 px hit twin (`client/RdfCanvas.tsx:146-158`, `canvas.css:534-568`, `src/client/src/canvas/library/DiagramCanvas.tsx:2103-2105`). tests.md (2026-09-03) records that edges were unselectable by mouse until the hit twin was added.
- **Selection.** The library's shared highlight: the outline in `--color-selected` (`#7c3aed` light, `#a78bfa` dark) at the declared width plus one, at least 3 (`src/client/src/canvas/library/highlight.ts`). The `.dis` leaves selection to the runtime.
- **The truncation banner** is not on the canvas: it is an HTML paragraph over the canvas host, centred 8 px from the top, 4 by 12 padding, a 1 px border and text in the warning colour (`#b45309` light, `#fbbf24` dark) on the surface colour, 12 px, reading "Showing {shown} of {total} resources — edits are withheld on this truncated view" (`client/RdfCanvas.tsx:273-277`, `client/rdf.css:26-39`). The `.dis` approximates it with a canvas watermark, which DISL draws under everything instead.
- **Host and accessibility.** The canvas host is `role="application"` with the label "RDF graph" (`client/RdfCanvas.tsx:263-272`). There is no level-of-detail switching, no icon on cards and no colour by type: the bands carry the grouping.
- **Colours** are standalone's theme variables (`src/client/src/index.css`): surface `#ffffff`/`#1e293b`, border `#e2e8f0`/`#334155`, text `#0f172a`/`#f1f5f9`, muted `#64748b`/`#94a3b8`, warning `#b45309`/`#fbbf24`, primary `limegreen`. The `.dis` copies them as theme tokens, the same block every family member uses.

## 8. Validation

`RdfValidator` (`backend/RdfValidator.cs`) is registered for `w3c/rdf`, and the OWL reading's validator delegates the file-level rules to it. It never fetches and never infers (`backend/RdfValidator.cs:8-10`, item 8). It validates the text in the request, not the store.

| Rule id (standalone) | Severity | Message | Where |
| --- | --- | --- | --- |
| `rdf.unparseable` | error | "This is not RDF that can be read: {parser message}" | the parser's line |
| `rdf.duplicate-prefix` | warning | "The prefix '{p}:' is declared more than once; from here on it means {iri}, which changes what every earlier-styled use would say." | each re-declaration's line |
| `rdf.relative-iri` | warning | "This document uses relative IRIs but declares no base, so their meaning depends on where the file happens to sit. Declare an @base." | the first triple with a relative IRI, once |
| `rdf.language-tag` | warning | "'@{tag}' is not a well-formed language tag (letters, then optional '-' groups of letters and digits)." | the triple's first line |
| `rdf.unknown-datatype` | warning | "xsd:{local} is not a datatype XSD defines; tools that check will refuse the value." | the triple's first line |

- An unparseable file yields that one finding and no other (`backend/RdfValidator.cs:58-77`, R7.1). DISL constraints run over a model, so a parse failure has no place there; the `.dis` names it in `constraints.doc`.
- DISL problems attach to elements; these findings attach to lines of the text, which the model does not keep. The `.dis` rules report on the diagram or on the card whose rows carry the literal; for a collection member literal that is the collection's owner.
- Every re-declaration of a prefix is warned, even one that repeats the same IRI (`backend/RdfValidator.cs:85-97`). The `.dis` reports the first one in one diagram-level message.
- The relative-IRI check looks at subject, predicate and object IRIs only; datatype IRIs and prefix namespaces are not examined, and nothing is reported when the file declares any base (`backend/RdfValidator.cs:99-112`).
- The language-tag rule is `^[A-Za-z]+(-[A-Za-z0-9]+)*$` (`backend/RdfValidator.cs:147`), but the tokenizer reads only letters and `-` after `@` (`backend/RdfTokenizer.cs:247`), so a tag with a digit ends the token early and becomes a parse error instead.
- The XSD list is string, boolean, decimal, integer, double, float, date, time, dateTime, dateTimeStamp, duration, gYear, gMonth, gDay, gYearMonth, gMonthDay, hexBinary, base64Binary, anyURI, language, normalizedString, token, long, int, short, byte, nonNegativeInteger, positiveInteger, negativeInteger, nonPositiveInteger, unsignedLong, unsignedInt, unsignedShort and unsignedByte (`backend/RdfValidator.cs:35-42`); `dayTimeDuration`, `yearMonthDuration`, `NMTOKEN`, `Name` and `NCName` would therefore be flagged. Datatypes outside the XSD namespace are not judged (test `AnUnknownXsdDatatype_IsWarned_ButForeignDatatypesAreNot`).
- Reading-specific rules (subclass cycles, scheme membership) belong to the sibling readings (R7.5).

## 9. Out of scope, by decision

- **A live triplestore.** The subject is files on disk; no endpoint, no connection (requirements, introduction).
- **The network and inference** (item 8): no IRI dereferencing, no `owl:imports` fetching, no remote JSON-LD contexts, no RDFS or OWL entailment, no SHACL execution. Every diagram shows what the file says, nothing a reasoner or the web could add.
- **Other serializations:** RDF/XML (a different splice discipline entirely), JSON-LD (remote contexts collide with the no-network rule, and byte-stable round trips are structurally unavailable), TriG and N-Quads (the named-graph dimension) (R1.6).
- **Layouts:** force-directed physics, 3D and VR, and circular or edge-bundling overviews are rejected with reasons (section 1).
- **Inline rename** on the canvas (section 6).
- **Vocabulary filtering:** nothing is hidden by vocabulary in this reading; RDFS, OWL and SKOS triples draw like any other.

## 10. Where the implementation and its specification differ

Recorded so a port to another IDE knows which one to follow.

1. **Named refusal of other serializations** (R1.6) is not implemented; a foreign serialization in a `.ttl` fails with a generic parse message (section 2).
2. **Collections** were to render "as an ordered group" (R3.5); they render as ordered, index-labelled member edges and rows, and a collection as subject or nested in another is silently not drawn (section 4).
3. **Stale layout ids** were to be "dropped on the next layout write" (R4.2); nothing prunes them (section 3).
4. **The toolbox** was to offer "a resource node" (R6.1); it also offers Prefix. The "add-triple" menu entry exists only as the Relate gesture; nothing adds a literal triple from a card's menu.
5. **The property grid** was to show the prefixed name and every literal row with its datatype or language (R6.2); it shows IRI, Types, Label and Comment (plus OWL axioms).
6. **The edge grid** was to show the predicate's IRI and prefixed name with reconnect as the stated path (R6.3); it shows the display form only, and the path is remove and restate.
7. **Unavailable actions** were to be "discovered unavailable with its reason … never silently absent" (R6.4); under truncation or a broken file they are absent at discovery, and the sentence appears only if one is executed anyway, or as a property row's read-only reason.
8. **Duplicate prefixes** were to be warned when "declared twice with different IRIs" (R7.2); any re-declaration is warned.
9. **Truncation** was to add "a validation info naming the truncation" (R8.2); the validator has no such rule. The session also serves a viewport-filtered view of the whole document rather than a first-N view, so the budget is a floor on the visible set and the banner counts stay document-level (section 4).
10. **Edge ids** were designed as `edge:{s}|{p}|{o-hash}`; the code uses `edge:{fromId}|{predicateIri}|{toId}` with `|{n}` for repeats.
11. **Bands** were designed "ordered by type IRI"; the code orders by the badge's display text.
12. **The N-Triples reader** was planned as a separate trivial reader; the Turtle parser reads N-Triples.
13. **`.nt` routing** was left open in the design; core's `AlternateExtension` resolved it.
14. **A new resource** was to carry "one `rdf:type` triple" (R5.7); it is typed `rdfs:Resource`, while the new-document template types its seed resource `ex:Resource`.
15. **Examples with blank nodes and collections** were required (R9.3); none of the `w3c/rdf` example bodies contains `_:`, `[` or a collection, so they are exercised only by the unit fixture `constructs.ttl`.
16. **The toolbox drop position** is described by a backend comment as a client follow-up layout write that does not exist (section 6).
17. **The context-source resolver** accepts any routed `w3c/*` body as "an RDF document" (`backend/RdfContextSourceResolver.cs:47-52`), so a `w3c/sparql` selection would pass the vendor check and the store would try to parse the `.rq` as Turtle.
18. **The scheme-verb remark** in `backend/RdfSelection.cs:38-45` says a vocabulary opened as a plain data graph stops offering scheme verbs; the SKOS entries and grid still appear (section 6).

Behaviours no specification covers, recorded because a port would otherwise rediscover them: adding a triple to an existing subject in an `.nt` file writes Turtle `;` syntax; editing a literal normalises its quoting to a short `"…"`; a rename leaves the old stored position behind; a resource related to any blank node cannot be removed; tall cards overlap the band below; only the first `rdfs:label` and `rdfs:comment` are shown and editable, and their language is kept on edit but not displayed.

## 11. Examples

Real, found-online RDF on the main path, per the example vendoring rule (item 9, R9; `src/diagrams/rdf/examples/`), each folder with its licence as `LICENSE.md` and a provenance readme:

- `w3c-turtle/example-1.ttl`: the Turtle recommendation's Example 1, verbatim, under the W3C Document License (retrieved 2026-09-03). It shows `@base` with relative IRIs (`<#green-goblin>`), prefixes, `a`, `;`, `,`, a comment and a language-tagged literal (`"Человек-паук"@ru`): two `foaf:Person` cards with mutual `rel:enemyOf` edges. It is the screenshot's source.
- `wikidata/marie-curie.ttl` and `wikidata/marie-curie.nt`: one Wikidata entity (CC0, the in-band `cc:license` triple kept), a single subject with hundreds of literal rows and many outgoing edges, typed `xsd:dateTime` and language literals, in both serializations. The `.ttl` keeps LF endings; the `.nt` holds the truthy statements only and is registered as `marie-curie-nt.adp`, which carries a `layout:` block with three `res:` entries.
- `nobel/nobel-1903.ttl`: categories, prizes and laureates as linked resources with motivations as language-tagged rows, in typed bands such as `nobel:Category`; derived from the Nobel Prize API (CC0) by the committed `generate.mjs`.
- `nobel/laureates.ttl`: 1,657 resources (1,018 laureates, 633 prizes, 6 categories), deliberately over the budget to show the banner and withheld edits (test `TheLaureatesFile_ReallyExceedsTheBudget`; tests.md 2026-09-03: "Showing 1000 of 1657 resources — edits are withheld on this truncated view", bands categories then prizes then laureates).

The same folders hold the siblings' examples (`owl-time/` and `prov-o/` for `w3c/owl`, `stw/` for `w3c/skos`, `shacl/` for `w3c/shacl`). Copies under `src/examples/diagrams/rdf/` are no longer byte-synced; the showcase `example-1.adp` and `nobel-1903.adp` carry authored `layout:` blocks. Tests over the examples: every document parses and validates clean, every one is registered, and every folder carries its provenance and licence (`tests/RdfExamples.Tests.cs`); core's `ExampleRegistrationTests` opens every registration in both trees. Share-alike sources (DBpedia, schema.org) were refused (R9.1).

## 12. What DISL 0.1 could not say, in short

For whoever takes DISL to 0.2:

1. **A foreign model file written by splices.** `persistence` can name a plugin format, not that the file belongs to other tools, must round-trip byte for byte, and changes only by named minimal splices with whole-document undo snapshots.
2. **Elements projected from another model.** Cards, rows, badges and edges are all computed from triples; DISL has derived attributes but no derived nodes, no rule that an `rdf:type` triple becomes a badge instead of an edge, and no folding of collection plumbing into ordered members.
3. **Identity that does not survive a reparse.** Blank nodes need a type-level statement that they draw but can never be positioned, related, edited or selected by id; the `.dis` spells it out as a row of gesture constraints with `rule: "false"`.
4. **Ids from IRIs and triples,** including a repeat counter for identical triples, and ids that must never be stored.
5. **A shared engine across several specifications.** Four specifications read one file through one store, and an edit through one is visible in all; DISL has no way to say that plugins and stores are shared between languages, nor that one reading's menus leak into another's.
6. **Findings on text lines** rather than on elements, and a parse failure as the only finding.
7. **A budget that truncates** with a banner outside the canvas and gates every edit, as opposed to a soft warning.
8. **Dialogs with placeholders, pre-filled values and validated input,** confirmations whose text carries a count and that are skipped below a threshold, and refusals per gesture, per element kind and per selection.
9. **Menu entries for transient targets** (a drop position, a drawn relation before it exists) and entries discovered from what the file asserts rather than from its type.
10. **Viewport delivery** and layout computed over the whole document while only the visible part is sent, which is a runtime concern but one this tool's scale answer depends on.
