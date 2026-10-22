# Marie Curie, from Wikidata

**What it demonstrates.** A dense single-subject data graph: one resource carrying hundreds of
literal identifier rows and dozens of outgoing edges to other entities — prefixed names,
`a`, `;` predicate lists, `,` object lists, typed literals (`xsd:dateTime`), language-tagged
literals, and both of the family's serializations over the same statements.

**Provenance.**

- Source: `https://www.wikidata.org/wiki/Special:EntityData/Q7186.ttl?flavor=simple` and
  `.../Q7186.nt?flavor=simple` (Q7186 is Marie Curie).
- Retrieved: 2026-09-03.
- License: Creative Commons CC0 1.0 Universal. Wikidata publishes its structured data under
  CC0, and the export states it in-band — the `cc:license
  <http://creativecommons.org/publicdomain/zero/1.0/>` triple in `marie-curie.ttl` is
  Wikidata's own statement, retained. See `LICENSE.md`.
- Exact local changes:
  - `marie-curie.ttl` is lines 1–39 (the prefix declarations, the `schema:Dataset` block with
    the license triple, and the `wikibase:Item` declaration) plus lines 2031–2992 (the entity's
    whole truthy-statement block) of the retrieved Turtle, both ranges byte-identical. The
    ~2,000 per-language `schema:Article` sitelink blocks between them, and the trailing
    `owl:sameAs` lines after the statement block, were removed whole. Nothing inside a kept
    block was touched; the file keeps its original LF line endings.
  - `marie-curie.nt` is every line of the retrieved N-Triples matching
    `^<http://www.wikidata.org/entity/Q7186> <http://www.wikidata.org/prop/direct/` — the
    truthy statements, lines byte-identical, order preserved.
