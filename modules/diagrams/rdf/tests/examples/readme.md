# RDF examples

Real, found-online RDF on the main path — not toy strawmen — per rdf-diagram Requirement 9 and
the vendoring discipline the helm-charts specification established: every source's license was
individually verified before anything was copied, each folder's `readme.md` records provenance
(source URL, retrieval date, verified license, exact local changes), and unmodified content is
unmodified.

- **`w3c-turtle/`** — the Turtle specification's own canonical example, verbatim: the smallest
  real document exercising prefixes, `a`, predicate lists, object lists and a language-tagged
  literal.
- **`wikidata/`** — Marie Curie's Wikidata statements, in both of the family's serializations:
  the Turtle statement block byte-identical from Wikidata's own export, and the same truthy
  statements as N-Triples. A dense single-subject graph: one card with many literal rows, many
  outgoing edges.
- **`nobel/`** — the Nobel Prize dataset, derived from the official CC0 API by the committed
  script beside it: `nobel-1903.ttl` is one coherent year (the year both Curies stood in
  Stockholm), and `laureates.ttl` is the whole dataset — 1,657 resources, deliberately past the
  drawn-element budget, so the truncated first-N view has a real file to be honest about
  (Requirement 8).

The ontology reading (`w3c/owl`) has two of its own, both published in Turtle by their standards
bodies, both copied byte for byte, and both opening with no validation findings:

- **`owl-time/`** — the W3C/OGC Time ontology (CC BY 4.0, attribution carried): 54 restrictions,
  a union, two enumerations, disjointness, deprecated classes still referenced, and the days,
  months and units as named individuals, so one file shows the schema and its instances at once.
- **`prov-o/`** — the W3C provenance ontology (W3C document and software terms): 39 classes with
  their hierarchy, properties with domains and ranges, unions and disjointness — a mid-size real
  standard that fits one screen.
- **`stw/`** — the STW Thesaurus for Economics (ZBW), read as `w3c/skos`: two subthesaurus
  extracts, one under the drawn-element budget and drawn whole, one deliberately past it so the
  truncated view has a real vocabulary to be honest about. Bilingual, notated and deeply
  polyhierarchical. CC BY 4.0, attribution carried in the folder's readme.

Two things the ontology set deliberately does *not* carry, recorded so the absence is not read
as an oversight. The **pizza ontology**, named as a candidate while this spec was written, is
published upstream only as RDF/XML; converting it would have to be a declared local change, and
no converter is installed in this repository's toolchain, so it was left out rather than
converted by hand into a provenance nobody could check. And there is no **over-budget ontology**
here: the permissive, Turtle-native ontologies large enough to pass the drawn-element budget are
the OBO releases, which have the same RDF/XML problem. The budget's neighborhood-whole cut is
covered by unit tests instead (`OwlProjectionTests`).
