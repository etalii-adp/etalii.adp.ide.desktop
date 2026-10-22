# STW Thesaurus for Economics

**What it demonstrates.** A real published thesaurus, in the tradition this reading was designed
for — a bilingual, notated, deeply polyhierarchical vocabulary maintained by a national library
rather than a toy taxonomy invented to flatter the diagram.

- `geographic-names.ttl` is subthesaurus **G**, 435 concepts, whole and under the 1,000-element
  drawn-element budget — so it opens complete, and the hierarchy, the notation badges and the
  `skos:related` cross-links are all visible at once. 272 of its concepts have more than one
  `skos:broader`, which is the polyhierarchy Requirement 1.6 draws once rather than repeating.
- `business-economics.ttl` is subthesaurus **B**, breadth-first from its root to 1,150 concepts —
  deliberately past the budget, so the truncated first-N view and its showing-N-of-M banner have
  a real vocabulary to be honest about (Requirement 8). Breadth-first because that is the order
  the budget itself cuts in: the extract is the top of a real hierarchy, not an arbitrary slice.

Both registrations carry a `language:` header — `en` on one, `de` on the other — so the same
vocabulary can be opened in either language and the label chooser's preference order is visible
from the explorer rather than only in tests.

**Provenance.**

- Source: `https://zbw.eu/stw/version/latest/download/stw.ttl.zip`, the ZBW's own Turtle
  distribution of the whole thesaurus (7,845 concepts, 3.8 MB unpacked).
- Retrieved: 2026-09-04.
- License: **Creative Commons Attribution 4.0 International (CC BY 4.0)**, verified at
  acquisition from the dataset's own `cc:license <https://creativecommons.org/licenses/by/4.0/>`
  triple rather than from the download page. That check mattered: the page carries a
  `rel="license"` link to ODbL and a `by-nc-sa` badge as well, either of which would have
  disqualified the source under the share-alike rule, and the data's own statement is what
  settles it. Full text in `LICENSE.md`, verbatim as served by creativecommons.org.
- **Attribution**, as CC BY requires: *STW Thesaurus for Economics*, ZBW – Leibniz Information
  Centre for Economics, `https://zbw.eu/stw/`, licensed under CC BY 4.0.
- Exact local changes: both files are **extracts**, not copies. From the full distribution:
  one subthesaurus was selected by following `skos:narrower` from its top concept; the scheme
  block was kept with its `skos:hasTopConcept` narrowed to that one root; and every
  `skos:narrower`/`broader`/`related`/`*Match` object list was filtered to concepts the extract
  actually carries, so no assertion points at a concept that is not here. No triple was
  otherwise altered, no label or notation was touched, and nothing was added. The extractor is
  described here rather than shipped, because it is a one-off against one distribution's layout.

**What this pair does not demonstrate**, said plainly so the next person does not go looking:
STW states no `skos:Collection` at all, and its `skos:prefLabel`s are completely bilingual —
every concept has both German and English. So neither the collection groups of Requirement 1.5
nor the incomplete-translation case behind the language chip of Requirement 3.3 has real data
here. Both are covered by fixtures in `EtAlii.Adp.Diagram.Rdf.Tests`; neither is covered by a
vendored vocabulary. Sources that would have carried them were considered and rejected at
acquisition: the UNESCO Thesaurus is CC BY-SA (share-alike), AGROVOC labels through SKOS-XL
which this reading deliberately does not resolve, and EuroVoc's distribution sits behind a
portal handler rather than a fetchable file.

**Both extracts are single-rooted, which matters most for layout.** Each carries exactly one
`skos:hasTopConcept` and one `skos:topConceptOf`: the extractor followed `skos:narrower` from a
single top concept and narrowed the scheme block to that root. So neither file demonstrates a
forest, several coordinate top concepts, or disconnected components — the shapes a balancing or
centring rule has to survive. A layout requirement evidenced on this corpus is evidenced on
single-rooted documents and rests on fixtures for the rest. The real STW carries several
subthesauri under one scheme, so a genuinely multi-rooted extract is available from the same
source if one is ever needed; it was not taken because the budget and polyhierarchy cases this
pair exists for read more clearly with a single root.
