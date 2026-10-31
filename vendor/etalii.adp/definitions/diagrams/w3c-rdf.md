# RDF graph: what DISL cannot express

This file accompanies [w3c-rdf.dis](w3c-rdf.dis), the RDF graph (`w3c/rdf`) at DISL 0.3. It lists only what that definition cannot say, each item beside the language gap or host concern that keeps it outside, and the seven language gaps the conversion found, each with its construct, the fixture that shows it and what would resolve it. Everything else is in the definition.

`w3c/rdf` is the anchor reading of the RDF family: [w3c-owl.dis](w3c-owl.dis), [w3c-skos.dis](w3c-skos.dis) and [w3c-shacl.dis](w3c-shacl.dis) read the same bytes. The engine, the registration and the gaps are written down here once; the siblings' companions refer to this file.

Sources, as read on 2026-10-22: the standalone module `src/diagrams/rdf/` of etalii.adp.ide.standalone on `develop`, the four parity transcripts its specification `rdf-disl-fbl` froze from today's code (task 1, `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf.Tests/Parity/*.transcript.json`, called *the transcripts* below), that specification's requirements and design, and DISL 0.3 and FBL 0.2 in this repository. Fixtures named below are under `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf.Tests/Fixtures/` there.

## 1. What the definition now carries

What the 0.1 draft listed here as inexpressible, DISL 0.2 and 0.3 have since answered, and the definition says it directly:

- **Every element is derived from the triples** (4.11): the turtle plugin reads one stored `Triple` per triple, one `Prefix` per directive and the `Base`; cards, rows, badges and statements are derived node and relation types over them, a collection folded into its members by the recursive `collectionMembers` with a depth cap (3.4).
- **Ids** are derived rules (11.5.2): `res:` and the IRI, `blank:` and the ordinal, `edge:` with the tool's repeat counter. A blank card and a triple naming a blank node are `ephemeral` (11.5.3), with today's sentence as the reason a move is refused.
- **The budget** of 1000 cards cuts, shows its notice and withholds every edit with today's sentence (3.2.1).
- **The counted menu label** "Remove (with 3 statements)" and the confirmation above two statements (4.11.4, 9.5), the dialogs' titles, labels, pre-fills, validations and confirming buttons (7.5).
- **The five `rdf.*` findings** come from the plugin's read and are shared by every reading of the body (FBL 9.1); a body that does not parse is `std.unparseable` with the code `rdf.unparseable`, and replaces every other finding (8.6).
- **Persistence** is `fbl` through the module's binding, `w3c-turtle.fbl#turtle`, and the plugin `net.etalii.adp.w3c.turtle` declares that it reads and plans the body and which naming functions it provides.
- **The wire ids** the standalone client and backend share are the `x-rdf` block (ruled 2026-10-06: wire ids stay an `x-` block).

## 2. What stays outside, and why

| What | Kept by | Why |
| --- | --- | --- |
| The Turtle and N-Triples parser, its spans and its sentences | The plugin's `read` | A persistence plugin is where a format's reader lives (FBL 11). The parser records UTF-16 offsets and line ranges; a splice is a range of UTF-8 bytes counted with a byte-order mark, so the plugin converts (finding B5). `byte-order-mark.ttl` is refused by the parser today, yet an edit applies and undo does not restore it (finding recorded with task 1). |
| How an edit is written: the dominant line ending, the requoted literal, the `@prefix` form, whitespace trimmed around a removal | The plugin's `plan`, and `RdfWriter` until gaps 1 and 2 are answered | These are the writer's own rules (finding B6, Q1). |
| Renaming a term everywhere it is written, declaring a prefix | Plugin actions `renameTerm` and `addPrefix` | Neither is the change of one stored element (gap 2). |
| Undo as a whole-document snapshot, redo by re-running the command | The host's history | The host of FBL 7 undoes by inverse splices and refuses on drift; the bytes are the same and the refusal is new (finding B13), and moving the host's history is no part of this series. |
| The unreadable body's sentence: "This file does not parse, so nothing can be edited until it is fixed." | The module | `PluginBody` says "The file could not be read, so it is never written."; the sentence must stay the tool's (finding B14, a runtime gap in standalone). |
| Routing a bare `.ttl` or `.nt`, offering the siblings, the order they are offered in | The binding's `claims` | FBL's, already declared there. |
| The registration: a move of a bare file refused with "This diagram was opened without a registration, so there is nowhere to store a position. Register the file to arrange it.", "This diagram is read-only." without a history, a stale position never pruned, a renamed resource leaving its old position behind | The host's registration handling | Ruled 2026-10-11, option A: the registration behaves as today in this series, and these move together to the change that moves the host's registration handling to the library for every tool. |
| The type bands | The layout plugin `net.etalii.adp.w3c.rdfTypeBands`, `lanes` by `band` as its fallback | DISL names layouts for a plugin (finding A11). |
| Viewport delivery and the `.proto` payloads | The module | A runtime concern; DISL has no streaming model on purpose (finding A11). |
| Resolving a typed term against the prefixes in force | Plugin functions `rdfResolveTerm`, `rdfTermRefusal` | The prefixes in force at a point of the file are the engine's (13.1.1). |

Three behaviours differ in timing rather than in result, and are left for the step that switches the reading over to show in its transcript:

- A relation gesture with a blank end is refused before the Relate dialog opens today; the definition refuses on commit, with the same sentence.
- "Drop the relation on a resource; a statement needs both of its ends." answers a relation gesture ending on a placement; a DISL connect gesture always ends on an element, so the case does not arise.
- Under truncation today's menus are empty; the definition withholds them through the edit gate, with the truncation sentence.

## 3. Entries that leave this reading (Q4)

Each reading offers its own entries and rows (Q4, at its default). Today, a file opened as `w3c/rdf` that asserts `owl:Ontology` is offered the four OWL add entries at a placement and "Subclass of" between two classes; a resource the file types as a SKOS concept, scheme or collection gets the scheme reading's entries and its whole property grid; and a resource with OWL axiom triples carries read-only Axioms rows. This definition states none of them. The transcripts record them (for example `owl-ontology.ttl` and `skos-merged-hierarchy.ttl` in `rdf.transcript.json`), and the step that switches this reading lists them in the transcript's diff.

## 4. `rdf-graph.dis`

[rdf-graph.dis](../../specifications/disl/rdf-graph.dis) is DISL's own example, a trimmed excerpt written at 0.2 to show that version's constructs. It is kept as it is, and differs from this definition where the definition follows today's tool:

- it declares no `format` and no `binding`, and its plugin provides only `action` and `celFunctions` (finding A2);
- it derives the type bands as `Band` elements, which here are the layout plugin's;
- its budget draws IRI cards before blank nodes and adds a second budget of 5000 RDF nodes; the tool draws cards in the order the file first names them and has one budget;
- it states the duplicate-prefix, relative-IRI and language-tag findings as constraints, the last with another sentence; here all five are the plugin's;
- its removal question reads "Remove {name} and the {n} statements it appears in?"; the tool asks "Removing this resource also removes the {n} statements it appears in." under the title "Remove resource";
- it refuses a statement from a resource to itself and offers an "Add type" operation; the tool does neither;
- it lets a relation join blank nodes and moves a new resource to the clicked point; the tool refuses a blank end and leaves the position to the client;
- it has no stored `Prefix` type, which `rdf.duplicate-prefix` and Declare prefix need.

## 5. Language gaps

The conversion found seven candidates for etalii.adp. None is worked around in a definition (Requirement 10.2); each stands until the language answers it.

| # | Gap | Construct | Fixture | What would resolve it |
| --- | --- | --- | --- | --- |
| 1 | **`plan` for a whole transaction** (finding B1) | FBL 11.2: `plan` receives one model change. Removing a resource removes every triple naming it, and the removal of two pairs of one statement claims the same `;`; planned one at a time they overlap, which `PluginBody.Order` refuses, and one gesture becomes several undo steps. | `shared-statement.ttl` | `plan` receives the ordered changes of one transaction and delivers one set of splices, so a plugin writes a group as one construct. |
| 2 | **An edit that is not a change of one element** (finding B2) | FBL 11.2 and DISL 9, 13.1: renaming a term rewrites every token naming it, in every position; declaring a prefix places a directive. Neither specification says how a plugin action hands the host splices for its history. | `rename-positions.ttl` | A plugin action that delivers splices under FBL 11.4, applied and recorded by the host as one edit; or a `rename` change that names a value and every place it is written. |
| 3 | **A writable value shared by several elements** (finding B3) | FBL 3.3 binds a writable value to one place; a subject is written once for every pair of a `;` list and a predicate once for every object of a `,` list. | `shared-statement.ttl` | FBL says how a value written once for several entries is bound, or that a plugin's `read` may report one span for several. |
| 4 | **A splice for a separator or a terminator** (finding B4) | FBL 6.1 names eleven operations, none of them the replacement of a separator, and 11.4 requires a plugin to name its operations truthfully. Adding a pair turns a statement's `.` into `;`; inserting before the `.` instead moves a trailing comment down a line. | `trailing-comment.ttl` | An operation for replacing a separator or terminator, or the statement that a plugin's operation names are its own. |
| 5 | **What `read` and `plan` are told about the document** (findings B7, A8) | FBL 11.2 hands the plugin the bytes and the binding's `args`, never the body's name, so an `.nt` body cannot be told from a `.ttl` one and a new pair is written in Turtle; and no CEL context of DISL 12.3 reads a registration header, so the SKOS label language cannot reach the definition. | `simple.nt`, `skos-language-header-forms.ttl` with its three registrations | FBL tells `read` and `plan` the body's name and the registration's declared headers; DISL lets CEL read a registration header. |
| 6 | **A grouping function** (finding A10) | Every id rule and every grouping attribute filters the triples once per triple, so a reading is quadratic in the number of triples. | the corpus's largest bodies, `stw/business-economics.ttl` and `stw/geographic-names.ttl` | A DISL function grouping a list by key in one pass. A gap only if the measurement the design asks for before each switch exceeds `limits.celCost` on those bodies. |
| 7 | **Entries of one specification offered under another reading of the same body** (finding A7) | DISL has no way for one specification to contribute entries to another's menus. | `owl-ontology.ttl`, `skos-merged-hierarchy.ttl` in `rdf.transcript.json` | Arises only if the leak of section 3 is kept (Q4's second option); with each reading offering its own entries, nothing is needed. |

## 6. The description's binding (spec 067)

The description `w3c-rdf.atd` no longer reads through the plugin: its part `w3c-rdf.turtle.atd` is FBL 0.15's binding `triples` of [`w3c-turtle-grammar.fbl`](../../specifications/fbl/w3c-turtle-grammar.fbl), so a stored `Triple`, `Prefix` and `Base` are read with no persistence plugin, and the five `rdf.*` findings are rules of the description. Gaps 1, 3 and 4 above stand for writing: a new triple is a statement of its own rather than a `;` continuation of its subject's statement, and a triple is not removed, since no splice states what becomes of a statement it shares. The module, the reference for the cases, still writes both, and the steps that differ are listed by name in the desktop's tests.
