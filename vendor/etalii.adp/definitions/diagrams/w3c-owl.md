# OWL ontology: what DISL cannot express

This file accompanies [w3c-owl.dis](w3c-owl.dis), the OWL ontology reading (`w3c/owl`) at DISL 0.3. It lists only what that definition cannot say, each item beside the language gap or host concern that keeps it outside. The engine, the registration and the seven language gaps are the RDF family's and are written down once in [w3c-rdf.md](w3c-rdf.md); every one of them applies here too.

Sources, as read on 2026-10-22: the standalone module `src/diagrams/rdf/` on `develop` (`Owl/` and the family's providers), the parity transcript `owl.transcript.json` that `rdf-disl-fbl` froze from today's code (task 1), and that specification's requirements and design.

## 1. What the definition now carries

- **Classification by role, punning included** (4.11.2): class, `owl:Thing`, datatype, individual and ontology header are derived node types over the same triples; an IRI that is both a class and an individual is two elements, `res:` and `ind:`.
- **The anchors** a property without a stated domain or range attaches to, `thing:{property}|domain`, `thing:{property}|range` and `dt:{property}`, are derived from the property's own triples.
- **Class expressions**: every blank object of `rdfs:subClassOf`, `owl:equivalentClass` or `owl:disjointWith` with a named subject, and everything nested in it, is a derived restriction or operator on today's structural id; its Manchester-style text is the recursive `owlText`, capped at two levels on the canvas and uncapped in the grid (3.4), standing in for `ExpressionRenderer`.
- **Edges**: subclass, equivalence, disjointness (pairwise over `owl:AllDisjointClasses`), object and datatype properties from each domain to each range, assertions between individuals, and expression references, each a derived relation with today's id and repeat counter.
- **The five `owl.*` findings** are constraints: `owl.subclass-cycle` over `diagram.cycles` (D5), the other four over the triples. The five `rdf.*` findings are the plugin's.
- **Ids that are never stored**: an expression's and an anchor's, each with today's sentence as the reason a move is refused.

## 2. What stays outside, and why

| What | Kept by | Why |
| --- | --- | --- |
| The column layout by asserted subclass depth, and the expressions and anchors placed beside the class that uses them | The layout plugin `net.etalii.adp.w3c.owlDepthColumns` | DISL names layouts for a plugin (finding A11). |
| The `.proto` payloads, among them `OwlNodePayload`'s `kind`, `deprecated` and `external` | The module | A runtime concern (finding A11). |
| The budget's cut by whole units: a named node together with the expressions grouped with it, all kinds in one order of first appearance | Recorded here | A DISL budget counts derived elements in their computation order, type by type (4.11.5); the definition's budget therefore cuts the same number of elements and may keep a different set on a file of more than 1000. The step that switches this reading measures it. |
| The repeat counter of an edge id, shared by every kind of edge | Recorded here | The definition counts repeats within one relation type. Two edges of different kinds share an id only when one IRI is drawn in two roles between the same two nodes. |
| One `owl.subclass-cycle` finding per strongly connected set of classes | Recorded here | `diagram.cycles` yields one cycle per simple cycle; a set holding two cycles is one finding today and two here. |
| An axiom row's wire id, `owl.axiom.{index}` and `owl.characteristic.{index}`, and the reason on a row that renders an expression | `FormDerivation`, through `x-owl` | A form item has one id and one list of reasons; today each axiom row has its own numbered id, and a row rendering an expression reads "That is an anonymous class expression, whose identity does not survive an edit to the file, so nothing about it can be edited from the diagram. Edit the expression as text." instead of the axioms' sentence. |
| The edit gate's count | The `truncated` reason | Today every reading withholds edits while the RDF graph reading is cut, not while this one is; the reason reads that count (`familyTruncated`). |

## 3. Entries that leave this reading (Q4)

A resource the file types as a SKOS concept, scheme or collection gets the scheme reading's entries and its whole property grid today, and a placement in a file holding a concept scheme offers "Add concept here…"; this definition states neither. The transcript records them, and the step that switches this reading lists them in the transcript's diff.

## 4. Language gaps

All seven of [w3c-rdf.md, section 5](w3c-rdf.md#5-language-gaps) apply. The four add entries and "Subclass of" each write one triple, so gaps 1 and 2 touch this reading through the family's rename, prefix and removal only.
