# OWL-Time — the W3C's temporal ontology

**What it demonstrates.** The ontology reading on a real, published W3C ontology, and the one
example that exercises the constructs the specification's Requirement 3 is about: 54
`owl:Restriction` structures (universal quantification and cardinalities alike), a union, two
enumerations, disjointness, and deprecated classes still referenced by live axioms — beside 14
named individuals (the days of the week, the months, the temporal units) so the TBox and the
ABox are drawn together on one canvas.

As drawn: 114 elements, 144 axiom edges, 55 expression nodes, 17 datatype nodes, 2 dimmed
deprecated classes and 1 dimmed external term. It opens with no validation findings.

**Provenance.**

- Source: "Time Ontology in OWL", W3C Recommendation 19 October 2017,
  `https://www.w3.org/TR/owl-time/`; the namespace document itself, `https://www.w3.org/2006/time`,
  fetched with `Accept: text/turtle`.
- Retrieved: 2026-09-04.
- License: CC BY 4.0, stated by the ontology in band as
  `dct:license <https://creativecommons.org/licenses/by/4.0/>`. The owl-diagram vendoring rule
  accepts CC BY only with its attribution carried, so the attribution travels in `LICENSE.md`.
- Exact local changes: **none**. `owl-time.ttl` is the retrieved file byte for byte — the
  ontology is published in Turtle, so nothing had to be converted.
