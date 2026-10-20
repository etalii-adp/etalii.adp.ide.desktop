# SPARQL query: what DISL cannot express

This file accompanies [sparql-query.dis](sparql-query.dis), the DISL specification of the SPARQL query diagram (`w3c/sparql`, SPARQL 1.1 query files, `.rq`). The `.dis` says what DISL can say: the metamodel, the notation, the empty toolbox, the two findings over a parsed query, and where layout and persistence hand over to plugins. This file holds everything else: how the query text is read, which rules produce the drawn elements, the layout algorithm, the one interaction, the refusals, the known gaps, and the places where DISL 0.1 fell short. Each point names the code, specification or conversation that shows it.

Sources, as read on 2026-09-30:

- The standalone implementation on `develop` of etalii.adp.ide.standalone (commit `b2a2692`): `src/diagrams/sparql/` (backend, client, `api/sparql.proto`, examples, tests), `docs/tools.md` line 143, and `tests.md`.
- The approved spec-workflow specification `sparql-diagram` (requirements, design and tasks approved 2026-09-03), removed from the tree by commit `53f68611` and read from its parent. Requirement numbers below (R1.1 and so on) are that specification's.
- Later standalone specifications that touched this module: backend-centralization (R2.2, R2.4, R2.7, task 7, PR #51 and the #78 diff work), centralized-selection R5, inline-rename-adoption, and the archived declarative-diagram-modules `sufficiency.md`.
- The Notion "Tools" database row "SPARQL query" (last edited 2026-09-28).
- This project's conversations from 2026-09-26 to 2026-09-30. None holds a decision about SPARQL itself; the rulings that apply are general ones: the `.dis` extension (Peter, 2026-09-30) and one DISL file per standalone tool in `definitions/diagrams/`.

## 1. What the tool is for

A SPARQL query diagram shows what a query *asks*: the basic graph pattern drawn as the graph it matches, the variables as the joins they are, and the form and modifiers as the frame around them. It is a deliberate sibling of the RDF diagram family (RDF, OWL, SKOS, SHACL), not a member: a `.rq` file is a question, not a serialization of a graph, so it shares no parser, store or model with `w3c/rdf` (`Diagram.cs` remarks; requirements introduction).

Catalogue data (Notion "Tools" row, docs/tools.md): kind Diagram, origin `w3c/sparql`, no previous origin, family "Database queries & results", extension `.rq`, rarity "Adoption from standard", theory SPARQL 1.1, example "sparql.org query forms", state ✅ Implemented in Standalone and not yet in IntelliJ, VS Code or Eclipse. Focus areas: Knowledge and semantics, Clarity in textual data, Software delivery. One-line purpose: "Read a SPARQL query as the graph pattern it matches, with variables as joins." Why specialized: "A basic graph pattern is a shape; drawing it shows at once how variables join and which filters narrow the result." The site's screenshot note adds: "The query's pattern is drawn as the graph it matches, with the OPTIONAL group as a region, so which variables join and which parts are optional is visible before the query runs." Standalone's icon is `mdi-help-network-outline`.

The research behind the drawing rules (requirements, "How SPARQL queries are visualized"):

- **Adopted:** one variable, one node (QueryVOWL, Gruff, RDF Explorer), "the single most important drawing rule"; variables visually distinct from concrete terms (QueryVOWL, simplified); structure for what is a graph and annotation for what is not; text-first editing (YASGUI/YASQE, and Neo4j Browser leaving the query as text).
- **Rejected:** visual query construction (the NITELIGHT and QueryVOWL ambition, which decades of tools have not made displace text); filter expressions as expression trees; subquery internals drawn inline; force-directed layout (non-deterministic, rejected family-wide).

## 2. The query file: read, never written

DISL's persistence layer describes a file a runtime writes. Here the model file is a foreign format that is only ever read, which DISL can only name as `format: "plugin:net.etalii.adp.w3c.sparqlQuery"`. What that reader does:

**Three tiers of the SPARQL 1.1 Query grammar** (`SparqlParser.cs` header; design "Backend: the document and its parser"). The parser is hand-written recursive descent, with keywords matched case-insensitively and variables written `?name` or `$name`; `#` starts a comment.

- *Parsed structurally:* the prologue (`BASE`, `PREFIX`); the four forms with their clause shapes (`SELECT` with `DISTINCT`/`REDUCED`, `*` and `(expr AS ?name)` aliases; `CONSTRUCT` with its template, including the short form `CONSTRUCT WHERE { ... }` whose template is the pattern itself; `ASK`; `DESCRIBE` with its targets, where a bare `DESCRIBE <iri>` needs no `WHERE`); `FROM` and `FROM NAMED`; the group-pattern tree (braced groups, `OPTIONAL`, `UNION` branches, `MINUS`, `GRAPH ?g`/`GRAPH <iri>`, `SERVICE` with optional `SILENT`, subqueries); triple patterns with predicate and object lists (a trailing `;` is accepted), blank-node property lists and collections, which expand to their cons-cell triples with one anonymous variable per cell (`rdf:first`, `rdf:rest`, `rdf:nil`); `a` as `rdf:type`; bare numbers and booleans typed `xsd:integer`, `xsd:decimal`, `xsd:double`, `xsd:boolean` (`SparqlVocabulary.cs`); the solution modifiers `GROUP BY`, `HAVING`, `ORDER BY`, `LIMIT`, `OFFSET`; and `VALUES`, inline or trailing the query. A trailing `VALUES` block is attached to the root where clause.
- *Recognized but kept as written:* every expression (`FILTER` bodies, `BIND` right-hand sides, `HAVING` and `ORDER BY` expressions, aggregates) and every property path. Brackets and strings are matched so the exact source text can be sliced and the variables inside found; no expression tree is built. A property path that turns out to be one plain IRI is that IRI.
- *Refused by name:* a SPARQL Update document. When the first token after the prologue is `INSERT`, `DELETE`, `LOAD`, `CLEAR`, `CREATE`, `DROP`, `COPY`, `MOVE`, `ADD` or `WITH`, the file opens unavailable with: "This is a SPARQL Update document (VERB ...), and SPARQL Update is out of scope: an update is a write instruction, not a question, and this diagram draws questions. Update documents conventionally use the .ru extension." (R1.2).

**A file that does not parse** opens unavailable naming file, line and reason, and yields exactly one finding (R1.3, R7.1). Other parse messages include "The file holds no query: it ends after the prologue.", "Expected a query form (SELECT, CONSTRUCT, ASK or DESCRIBE), found '…'.", "The query is over, but '…' follows it.", "The group that starts here is never closed with '}'.", "GRAPH names its graph by variable or IRI." and "SERVICE names its endpoint by variable or IRI.". A prefix used but not declared is a parse error.

**No writer, by construction** (R1.4). The module registers no command, no save path and no toolbox provider; `ServiceCollection.AddSparql.cs` says why. Three test layers pin it: a reflection surface test (`NoWriterSurface.Tests.cs`: no command type, no `Save*`/`Write*`/`Flush*` member, no stream or writer parameter, no HTTP client referenced), a behavioural sweep that exercises every surface and compares the `.rq` bytes (`NoWriterSweep.Tests.cs`), and a registration test that the action provider offers nothing. The byte-identical round trip therefore holds without a writer to get it right, and the `.rq` fixtures that are byte-compared (`crlf-line-endings.rq`, `lf-line-endings.rq`, `no-trailing-newline.rq`) are marked `-text` in standalone's `.gitattributes`.

**Reading lifecycle** (`SparqlDocumentStore.cs`, `SparqlDocumentReloader.cs`; backend-centralization R2.2, R2.4, R2.5, R2.7):

- The text editor is where a query changes, so a change on disk is the normal way the diagram updates; open diagrams follow without a refresh (R1.5).
- One store per process, so two connections on one query share its document.
- A reload that cannot read keeps the last good query; only the watcher's delete clears it. `BodyDeleted` is overridden for this, and removing the override silently keeps a deleted query on the canvas.
- A missing file opens as "NAME does not exist. Queries are authored in a text editor; this diagram draws an existing .rq file."; an unreadable one as "NAME could not be read: REASON".

**New files.** The design said the module would register no document factory. Core refuses to start a host whose type declares an extension without one, so `SparqlDocumentFactory.cs` exists and supplies the text of a *new* file only, in CRLF with a trailing newline, and validating clean:

```sparql
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?subject ?label
WHERE {
  ?subject rdfs:label ?label .
}
LIMIT 100
```

## 3. The registration and where positions live

The `.dis` declares `files.mode: "split"` with the model in `{name}.rq` and the view in `{name}.adp`, and `view.store: ["bounds"]`. What that stands for is standalone's registration file, which DISL does not describe:

- An `.adp` registration beside the query holds the type line `w3c/sparql`, a `body:` header naming the `.rq` (resolved against the `.adp`'s own folder, R2.3) and a `layout:` block of `<id>: <x> <y>` lines (for example `var:proteomeData: 516.659 411.066`, tests.md). Only positions are stored, never sizes; a region's stored point is its frame's top-left.
- A `.rq` routes to this type on sight by its extension: no other type claims `.rq` and this type claims no other (R2.1). Add on an existing `.rq` creates only the `.adp` (R2.2).
- The registration header facility exists but is deliberately unused: the query carries its own prologue, nothing executes so there is no endpoint or dataset to choose, and a file holds one query (requirements, "What this spec consumes").
- A bare `.rq` opened without a registration draws normally but refuses repositioning: "This query was opened without a registration, so there is nowhere to store a position. Register the file to arrange it." (R5.3).

## 4. From query to drawn elements

DISL's metamodel describes elements a user creates. Here every element is derived from the text by a pure function, `SparqlProjection.Project` (`SparqlProjection.cs`), with these rules.

**Identity.**

- One node per variable name across the whole query, subqueries excepted (R4.1). Its id is `var:{name}`.
- One node per distinct concrete term: an IRI by its full IRI (`iri:{full}`), a literal by lexical form plus language plus short datatype (`lit:{percent-encoded lexical}[@lang][^{short datatype}]`). Merging concrete terms asserts "same term", never a join.
- One node per anonymous variable, by its expansion ordinal (`anon:{n}`); a labelled blank node `_:b` is one node wherever its label recurs.
- A subquery is `sub:{scope path}.{ordinal}`; a region `region:{scope path}`; an annotation `note:{scope path}/{kind}.{ordinal}`; a pattern edge `edge:{from}|{predicate as written}|{to}|{ordinal}`, the ordinal counting repeats of the same triple; a subquery join `edge:{sub id}|{name}|var:{name}|0`. The header band is `header:query` and the banner `truncation`.
- Scope paths come from source order: the root is `where`, the template `template`, and each child scope appends `/{kind}.{ordinal}` with the ordinal counted among same-kind siblings, for example `where/union.0/branch.1`. These ids are stable across reparses of an unchanged file, which is what makes stored positions survive. The DISL `natural` id strategy with type prefixes is the nearest DISL says; it cannot say that ids come from term forms and scope paths rather than from key attributes, nor that `anon:` ids are never stored.

**Which nodes exist.** Every named variable gets a node, including one used only in predicate position, only in a `FILTER`/`BIND`/`VALUES`, or only as a name a subquery projects: those would otherwise vanish although they are part of the join surface. A `DESCRIBE`'s targets are nodes too (a bare one-line `DESCRIBE <iri>` otherwise drew nothing, found by the vendored UniProt corpus); `DESCRIBE *` contributes none.

**Placement: every node lives at the shallowest scope that references it, and region edges reach out to it** (design; `SparqlProjection.PlaceAt`). The node's scope is the longest common ancestor of every scope that mentions it. A node landing on a `UNION` itself is lifted to the union's parent, because a name shared by branches binds where the branches join (R4.5). A variable's mentioning scopes include constraint and subquery mentions; a concrete term's only its subject and object positions. An edge belongs to the scope that states it and may cross region borders to reach its endpoints; that crossing is the join being shown, not a rendering defect. DISL containment cannot say this: in DISL a child has one parent chosen when it is created, and an edge has no owning scope.

**Edges.** Each triple pattern is a directed edge from subject to object labelled with the predicate or property path exactly as written (R3.1). A variable in predicate position is a node but no edge reaches it; it appears only in the edge label. A subquery gets one labelled edge per projected name to that variable's node (R3.6). The client draws edges straight with an arrow at the object end and the label just above the midpoint; a property path is dashed.

**Annotations** (R3.3). `FILTER`, `BIND` and `VALUES` keep their text exactly as written:

- a `BIND` anchors to the variable it defines, and that variable's defining expression is the `BIND` text;
- a `VALUES` block anchors to the first variable it feeds;
- a `FILTER` anchors to the region it sits in, or floats above the patterns when it is in the root where clause, because it constrains the whole query.

**The frame.** The header band shows the form line (`SELECT`, `CONSTRUCT`, `ASK` or `DESCRIBE` plus its targets, then `DISTINCT` or `REDUCED`) followed by the dataset clauses and the solution modifiers as written rows (R3.4). It is one element per diagram and always present.

**Join count.** A variable's join count is the number of triple-pattern positions that mention it, template included (`SparqlParser.IndexScope`). For a variable that only appears as subject or object this is its degree on the canvas; a variable in predicate position counts but has no edge, and subquery-join edges reach a variable without counting. The `.dis` keeps `joinCount` as a read attribute for that reason instead of deriving it from edges.

**Determinism.** Same model, same elements, same order: variables ordered by first mentioning scope then name, then describe targets, then concrete and anonymous nodes on first mention, then edges in source order, subqueries, and annotations in tree order.

**Sanity bound** (R7.5; `SparqlProjection.SanityBound`). Hand-written queries hold tens of patterns, so the family's drawn-element budget is not used; this module's own bound is 500 drawn elements (regions, nodes, edges and annotations). Beyond it the projection keeps the first 500 in the order regions, nodes, edges, annotations, drops any edge whose endpoint was cut, and a banner says "Showing N of M elements — this query is larger than the diagram draws". DISL's `language.limits.maxElements` only asks a runtime to warn, so the cut and the banner are not in the `.dis`.

## 5. Layout

DISL names a layout; it does not define one. The `.dis` names `plugin:net.etalii.adp.w3c.sparqlScopeGrid` with a `grid` fallback. What the plugin does (`SparqlLayout.cs`; R5.5):

- Pure and deterministic, recursive bottom-up over the scope tree, no physics.
- A node is 170 by 64. Items are placed in rows of three columns with 60 horizontally and 48 vertically between them; each row is as tall as its tallest item.
- A scope's items are its own nodes first, then its child regions, in projection order. A child region is measured first and then treated as one block: its size is its own grid's extent plus 36 padding on every side and a 30 label band on top, and never less than one node wide or a node plus the band high. An empty region's grid counts as one node wide and half a node high.
- The root starts at (40, 90), below the header band. The `CONSTRUCT` template region sits on the root row after the where clause's own items.
- A region's bounds always contain its contents; this is a tested invariant (`SparqlLayout.Tests.cs`).
- A stored region position moves the frame and everything *computed* inside it by the same offset, so containment survives the move. A stored node position is absolute and wins over the computed place, even inside a moved region. Anonymous variables always take their computed place.

The DISL `trigger: "always"` with `respect: "all"` is the nearest reading: layout runs on every render and authored positions survive, but DISL has no way to say that a container's stored position shifts its computed contents while authored contents stay put.

## 6. Interaction: one gesture, and refusals that say why

DISL can mark attributes read-only, switch off deletion, copying and connecting, and give an empty toolbox. It cannot carry the refusal sentences or the rule that one gesture writes somewhere other than the model.

- **Moving is the only mutating gesture** (R6.3). Dragging a node or region frame dispatches core's `SetRegistrationLayoutCommand`, which writes the `.adp` `layout:` block as one undoable step with a byte-restoring inverse; the `.rq` is untouched (R5.1). Without a project history the answer is "This diagram is read-only.".
- **Refusals** (`SparqlSession.Refusal`), each shown where the user is looking:
  - anonymous variable: "That is an anonymous variable - written as a blank node, it has no name to key a stored position by, so it takes its computed place." (R5.4);
  - edge: "An edge is drawn between its endpoints; move one of those instead.";
  - annotation: "An annotation stays with what it constrains; move that instead.";
  - header band: "The header band states the query's form and modifiers, and stays at the top of the diagram.";
  - truncation banner: "That banner reports the truncation and is not something this diagram can move.";
  - dropping an element into another group: "A query's structure comes from its text, so nothing here can be moved into a different group. Dragging changes where an element sits on the canvas.".
- **No toolbox, no context actions, no rename** (R6.1). The action provider is registered precisely to offer nothing and answers anything that arrives anyway with "A query diagram offers no edits: it reads the .rq file, which is edited in a text editor.". No relation declares a source anchor, so no connect gesture exists, and there is no drop target. Inline rename is exempt rather than pending: a variable's name is query text with scoping rules, not a label (`client/readme.md`; inline-rename-adoption).
- **Property rows** (R6.2; `SparqlContextPropertyProvider.cs`), every one read-only with the same reason, "This diagram reads the query; edit the .rq file in a text editor and the diagram follows.", which the property-grid contract requires:
  - variable: Variable (or Anonymous variable), Projected (Yes/No), Joins, Defined by;
  - IRI: Prefixed name, IRI, Type; literal: Literal, As written, Type;
  - subquery: Projection, and Subquery with its whole text;
  - region: Group (the kind in capitals) and Constraint (its label);
  - edge: Predicate or Property path;
  - annotation: FILTER, BIND or VALUES with its text;
  - header: Form, and Modifiers joined with " · ".
  Groups are Identity, Structure and Query. The `.dis` gives attributes these labels and groups; a write that arrives anyway is refused with the reason.
- **Selection** resolves an element id against the file named by the enclosing level and checks the id is really in it; nothing nests inside these elements for selection (`SparqlContextSourceResolver.cs`).
- **Viewport delivery** (view-delta-adoption R1.2, R1.3; `SparqlElementMapper.Visible`). A connection receives only what its viewport admits: every region whose frame touches it, every node whose 170 by 64 box touches it, then one hop along edges so every edge has both ends, then edges whose ends both travel. Elements without a position (the header band, the truncation banner, annotations) always travel. Changes are sent as the shared `DiagramDiff`, removals first.

## 7. Notation the `.dis` approximates

The standalone canvas is drawn through the central canvas library (`SparqlCanvas.tsx`, `sparql.css`). Where DISL's notation is close but not exact:

- **The header band is not on the canvas.** It is an HTML band above the drawing surface: form in semibold (600) in the primary colour, modifier rows muted, wrapping with a 12 px gap. DISL has no band outside the canvas, so the `.dis` keeps its content as diagram attributes (`headerForm` derives the form line). tests.md (2026-09-05) records that the band overlays canvas content.
- **The truncation banner** is an amber box centred at the top of the surface; DISL has nothing like it.
- **Node corners.** Every node is the library's `box` without a corner radius, so IRIs and literals are both square rectangles and differ only in content, not in outline. The literal's `rx: 0` rule in `sparql.css` changes nothing visible. Variables and anonymous variables are dashed `5 3`; anonymous ones also at opacity 0.7; subqueries have a 2 px outline.
- **Projection mark.** A "→" at 8 by 16 from the node's top-left in the primary colour (`limegreen` in both themes), shown when the variable is projected (R4.2; centralized-selection R5 names it as the primary colour at rest).
- **Second label.** A 10 px muted line 10 above the bottom edge, 8 in from the left: a literal's `@lang` or short datatype, a variable's defining expression, or "subquery".
- **Annotation placement** in canvas units: on a variable node at the node's position minus 6 vertically; on a region at its frame's top-left plus (12, 18); a root-scope filter at (0, −10). Annotations are text only, 11 px muted, selectable but not draggable, drawn last so nothing covers them.
- **Regions** are frames with no fill and the border colour, drawn first so they sit behind their contents; `OPTIONAL` and `MINUS` dashed `4 4`, `GRAPH` and `SERVICE` dashed `2 3`, the rest solid; the label sits above the frame, 11 px muted. A union branch and a plain braced group have an empty label. The `CONSTRUCT` template's label is "CONSTRUCT".
- **Edges** are 1.5 px muted lines; the label is 6 above the midpoint, 11 px; a property path is dashed `6 2`.
- **Colours** are standalone's theme variables (`src/client/src/index.css`): surface `#ffffff`/`#1e293b`, border `#e2e8f0`/`#334155`, text `#0f172a`/`#f1f5f9`, muted `#64748b`/`#94a3b8`, warning `#b45309`/`#fbbf24`, primary `limegreen`; node outlines default to `#8892a6`. The `.dis` copies them as theme tokens.
- **CSS hooks.** Classes such as `sparql-node-{kind}`, `sparql-node-projected`, `sparql-region-{kind}` and `sparql-edge-path` are composed from payload values; standalone's "no stylesheet rule without an emitter" guard mounts the canvas on the shipped examples to check them (docs/guards.md, line 57).

## 8. Validation

The `.dis` states the two findings over a parsed query as constraints. What they cannot carry:

| Rule id (standalone) | Severity | Message | Where |
| --- | --- | --- | --- |
| `sparql.unparseable` | error | "This is not a SPARQL query that can be read: REASON" | the parse error's line |
| `sparql.unbound-projection` | warning | "?NAME is selected but never appears in the where clause, so its column will always be unbound." | line 1 |
| `sparql.unused-prefix` | info | "The prefix 'P:' is declared but never used." | the declaration's line |

- An unparseable file yields that one finding and no others, because rules over an empty model would bury the reason (R7.1). DISL constraints run over a model, so a parse failure has no place there.
- DISL problems attach to elements; these findings attach to *lines of the query text*, which the model does not keep. The `.dis`'s `unusedPrefix` rule reports every unused prefix in one diagram-level message, where standalone reports one finding per prefix at its own line.
- "Used" is decided on the source text, not the model: `P:` occurs anywhere except on a line whose text before it starts with `PREFIX` (case-insensitive). Occurrences in comments or strings therefore count as uses. The `.dis` carries this as a `used` field the reader fills in.
- The design also promised an info finding when a query is truncated; the validator does not emit one.
- The validator never executes the query and never contacts an endpoint; the module references no HTTP client at all.

## 9. Out of scope, by decision

- **Results.** Executing a query is out of scope entirely: no endpoint configuration, no HTTP, no result grid. Running a query needs the network the family refuses, and a query tool that sometimes executes is a different product (R7.4; NFR Security "No network, unconditionally").
- **`SERVICE`** clauses are drawn as regions and never contacted.
- **SPARQL Update** is refused by name (section 2).
- **Editing the query.** Visual query construction is rejected, not deferred; any future editing is its own specification with its own splice story (R6.1).
- **Subquery internals** are never drawn inline (R3.6).

## 10. Where the implementation and its specification differ

Recorded so a port to another IDE knows which one to follow.

- **`VALUES`** was specified as "a small table annotation on the variables it feeds" (R3.3); it is drawn as its text as written, anchored to the first variable only.
- **Region ids** were designed as `region:group.0.optional.1`; the code uses scope paths, `region:where/optional.0`. Annotation ids likewise use `note:{path}/{kind}.{ordinal}` instead of the designed `note:{kind}.{path}.{ordinal}`.
- **The document factory** exists although the design said it would not (section 2).
- **The truncation info finding** is not emitted (section 8).
- **Self-loops.** A pattern like `?x :knows ?x` gives an edge whose ends are the same node, but the client's relation declares `allowSelf: false`. The `.dis` allows self-loops, as SPARQL does.
- **Toolbox panel.** `SparqlCanvas` does not register an empty toolbox, so the Toolbox panel shows "Open a diagram…" instead of "This diagram type offers no toolbox elements" (tests.md, confirmed 2026-09-05, open).
- **Default positions.** Edges, annotations and the header band carry a default position of (0, 0). docs/creating-a-diagram-module.md names this module among four that culled such elements by position at first; the mapper now keeps them structurally (section 6).
- **Manual checks.** tests.md holds four SPARQL checks; three passed on 2026-09-05 (shared variable drawn once with "Joins: 2" and a dashed `OPTIONAL`, the projection mark, reposition leaving the `.rq` untouched). The re-run after the canvas library migration (frame behind contents, path edges still dashed, refusals verbatim, no anchors, drop target or rename) has not been executed.

## 11. Examples

Real, found-online queries on the main path, per the vendoring rule (R8; `src/diagrams/sparql/examples/`):

- `w3c-sparql/`: seven queries from the SPARQL 1.1 Query recommendation, verbatim (simple, optional, union, filter, property path, construct, aggregate with `GROUP BY`/`HAVING`), under the W3C Software and Document License.
- `uniprot/`: two from the SIB Swiss Institute of Bioinformatics' UniProt corpus, CC BY 4.0 with attribution carried: a proteome query with `DISTINCT`, comments and two `BIND`s, and a bare `DESCRIBE`.
- `adp/ask.rq`: written by ADP and labelled as such, because neither verified source had an `ASK`.
- The Wikidata example-queries page was checked and rejected: CC BY-SA, and share-alike is refused.

Each folder carries the upstream licence as `LICENSE.md` and a provenance readme. Every example registration must resolve and validate with no finding above info (R8.5). Standalone's screenshot `sparql.png` is taken from `optional.adp`.

## 12. What DISL 0.1 could not say, in short

For whoever takes DISL to 0.2:

1. **A read-only foreign model.** `persistence` assumes the runtime writes the model file. There is no way to declare "this file is read by a plugin, never written, and only the view file is written", other than a plugin format plus `readOnly` on every attribute.
2. **Elements derived from text.** Every element is computed from the query; nothing is created by a user. DISL has `derived` attributes and derived relations, but no derived *nodes* and no containment computed from the model (shallowest-scope placement).
3. **Edges owned by a scope other than their endpoints'.** A pattern edge belongs to one region and may cross its border to reach nodes in an outer one.
4. **Ids from term forms and scope paths,** and ids that must never be stored (`anon:`).
5. **Findings on text lines** rather than on elements, and a parse failure as a finding.
6. **A frame outside the canvas** (the header band) and a truncation banner.
7. **Refusal messages** per gesture and per element kind, and a read-only reason on every property row.
8. **Layout interplay:** a stored container position that moves its computed contents while authored contents stay absolute.
9. **A hard element bound** that truncates in a declared order, as opposed to a soft warning.
10. **Viewport delivery** (what a connection receives), which is a runtime concern rather than a specification one, but one this tool's behaviour depends on.
