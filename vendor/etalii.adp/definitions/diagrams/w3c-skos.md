# SKOS concept scheme: what DISL cannot express

This file accompanies [w3c-skos.dis](w3c-skos.dis), the SKOS concept scheme reading (`w3c/skos`) at DISL 0.3. It lists only what that definition cannot say, each item beside the language gap or host concern that keeps it outside. The engine, the registration and the seven language gaps are the RDF family's and are written down once in [w3c-rdf.md](w3c-rdf.md); every one of them applies here too.

Sources, as read on 2026-10-22: the standalone module `src/diagrams/rdf/` on `develop` (`Skos/` and the family's providers), the parity transcript `skos.transcript.json` that `rdf-disl-fbl` froze from today's code (task 1), and that specification's requirements and design, including its amendment ruled 2026-10-11.

## 1. What the definition now carries

- **Schemes, concepts and collections** are derived node types over the type triples and the triples that use them, blank-node terms included (with an `ephemeral` id and today's sentence).
- **One hierarchy edge per pair**, merged from `skos:broader` and `skos:narrower` in either direction, every stating triple among its sources, so Disconnect removes them all as one edit; one related edge per unordered pair; one mapping edge per triple between two concepts (4.11.3, keyed relations).
- **The order the tool meets the vocabulary in**, and so the order its budget cuts: schemes in id order, each with its top concepts and broaderless members and their descendants breadth first, then unfiled concepts, then collections (`skosOrder`, recursive with a depth cap).
- **The label chooser**: the display language, then English, then the untagged label, then the smallest remaining tag, preferred labels before alternate ones, the IRI's local name last.
- **Seven of the eight `skos.*` findings** are constraints, the cycle finding over `diagram.knots` (D5). The five `rdf.*` findings are the plugin's.

## 2. What stays outside, and why

| What | Kept by | Why |
| --- | --- | --- |
| The display language, from the registration's `language:` header | The module, as the diagram attribute `language` | No CEL context reads a registration and the plugin is not told its headers (gap 5). The module reads the header as today (Requirement 7.3). |
| `skos.misplaced-header`: a `language:` line above `body:` or `view:`, which severs the pairing and is ignored | The module | It is about the registration, which behaves as today in this series (design, *Amendments ruled*, option A). `skos-language-above.adp`, `skos-language-nospace.adp` and `skos-language-uppercase.adp` show the forms it reads. |
| The layered layout, each scheme a band | The layout plugin | DISL names layouts for a plugin (finding A11). |
| The `.proto` payloads, among them the chosen label's tag and kind and a scheme's member count | The module | A runtime concern (finding A11). |
| The order of `skos.duplicate-preflabel` and `skos.missing-preflabel` findings | Recorded here | Today both are judged concept by concept, so a concept's findings stand together; DISL reports rule by rule (8.6). The set of findings is the same. |
| One `skos.hierarchy-cycle` per broken cycle | Recorded here | The layout breaks a knot by excluding its lowest edge and looks again, so one knot holding two cycles is two findings today; `diagram.knots` gives one per knot. |
| A label, documentation or mapping row's wire id, keyed by kind and language or by predicate and target | `FormDerivation`, through `x-skos` | A form item has one id; today each label row is `skos.label:{kind}:{language}` and each out-of-file mapping `skos.mapping:{predicate}:{target}`. |
| Add concept writing its type, its preferred label and its filing as one edit | The definition's operation, three `create` actions | One transaction in DISL; planned one triple at a time it is gap 1. |
| Minting a new concept's IRI from its label under the namespace the file names things in | Plugin functions `rdfMintIri`, `rdfMintRefusal` | The prefix a subject is written under is the engine's (13.1.1). |
| The edit gate's count | The `truncated` reason | Today every reading withholds edits while the RDF graph reading is cut; the reason reads that count (`familyTruncated`). |

## 3. Entries that leave this reading (Q4)

A placement in a file holding `owl:Ontology` offers the four OWL add entries today, and a relation between two OWL classes "Subclass of"; this definition states neither. The transcript records them, and the step that switches this reading lists them in the transcript's diff.

## 4. Language gaps

All seven of [w3c-rdf.md, section 5](w3c-rdf.md#5-language-gaps) apply. Gap 5 is this reading's own: the label it draws depends on a registration header no CEL context can read. Gap 1 is met by Add concept and by Disconnect, each several triples in one edit.
