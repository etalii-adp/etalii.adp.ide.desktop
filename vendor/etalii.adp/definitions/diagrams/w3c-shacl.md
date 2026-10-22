# SHACL shapes: what DISL cannot express

This file accompanies [w3c-shacl.dis](w3c-shacl.dis), the SHACL shapes reading (`w3c/shacl`) at DISL 0.3. It lists only what that definition cannot say, each item beside the language gap or host concern that keeps it outside. The engine, the registration and the seven language gaps are the RDF family's and are written down once in [w3c-rdf.md](w3c-rdf.md); every one of them applies here too.

Sources, as read on 2026-10-22: the standalone module `src/diagrams/rdf/` on `develop` (`Shacl/` and the family's providers), the parity transcript `shacl.transcript.json` that `rdf-disl-fbl` froze from today's code (task 1), and that specification's requirements and design.

## 1. What the definition now carries

- **Shapes by declaration or by use**, in the order of the triple that first made each one a shape, and which of them are cards: node shapes, except blank ones living only inside a combinator or a qualified reference, and IRI property shapes no card references (4.11.2).
- **Rows and target chips** are list attributes of a card, never elements: a row's path in SHACL path syntax is the recursive `shPath` with a depth cap (3.4), its summary prints the parameters in the fixed order, and a blank operand is summarized one level deep.
- **References** between cards, at most one per source, kind and target, a property row's `sh:class` drawn only where exactly one card claims the class.
- **The five `shacl.*` findings** are constraints, the vocabulary of `shacl.unknown-term` a constant of the definition. The five `rdf.*` findings are the plugin's.

## 2. What stays outside, and why

| What | Kept by | Why |
| --- | --- | --- |
| The shape menus of the 0.1 draft: add a target class or node, add a property row from the menu, deactivate and reactivate, remove a target, remove a shape with its subtrees | Not stated | Today a named shape is first a resource, and the family's Rename and Remove answer for it before the shape's own entries are asked, so none of these entries appears in the transcript and the definition does not state them (Requirement 2.4). A blank shape card would be offered them all unavailable, with the blank-node sentence; no document of the corpus draws a blank card, so the transcript cannot show it either, and a fixture holding one is the evidence the step that switches this reading needs. The code that would carry the entries out is in `ShaclActions` and `ShaclWriter`. |
| Add property row, reached by dropping its toolbox entry on a shape | Plugin action `addPropertyShapeBlock` | It writes one `sh:property [ … ]` block on a blank node that is created and never addressed: several triples as one construct (gaps 1 and 2). |
| The order of `shacl.path-cardinality`, `shacl.impossible-counts` and `shacl.datatype-and-class` findings | Recorded here | Today the three are judged shape by shape; DISL reports rule by rule (8.6). The set of findings is the same. |
| The `sh:class` claimant counted over the drawn cards only | Recorded here | The definition counts every card; the two differ only on a file of more than 1000 cards. |
| A target, constraint or SPARQL row's wire id, `shacl.target:{index}`, `shacl.row:{index}`, `shacl.sparql:{index}`, and the reason on a row whose shape is blank or whose target is implicit | `FormDerivation`, through `x-shacl` | A form item has one id and one list of reasons; today each row has its own numbered id and its own reason. |
| The card layout | The layout plugin | DISL names layouts for a plugin (finding A11). |
| The `.proto` payloads | The module | A runtime concern (finding A11). |
| The edit gate's count | The `truncated` reason | Today edits are withheld while the RDF graph reading is cut, not while the shapes are; the reason reads that count (`familyTruncated`). |

## 3. Entries that leave this reading (Q4)

A placement offers the four OWL add entries where the file holds `owl:Ontology`, and "Add concept here…" where it holds a concept scheme, today; this definition states neither. The transcript records them, and the step that switches this reading lists them in the transcript's diff.

## 4. Language gaps

All seven of [w3c-rdf.md, section 5](w3c-rdf.md#5-language-gaps) apply. Gap 1 is met by Add property row.
