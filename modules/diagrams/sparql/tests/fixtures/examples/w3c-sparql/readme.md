# The SPARQL specification's own example queries

**What they demonstrate.** The recommendation introduces its language through worked examples,
which makes them the natural corpus for a diagram that draws that language: between them they
cover every drawing rule this module has.

| File | What it exercises |
| --- | --- |
| `simple.rq` | A basic graph pattern of two triple patterns sharing `?x` — the smallest real join |
| `optional.rq` | `OPTIONAL`, with `?x` used inside and outside it — the placement rule's own case |
| `union.rq` | `UNION`, with `?title` and `?book` shared by both branches — the union lift |
| `filter.rq` | `FILTER` with a `regex` call, drawn as an annotation showing its text as written |
| `property-path.rq` | `foaf:knows+`, drawn as an edge label rather than a parsed structure |
| `construct.rq` | `CONSTRUCT`, whose template draws as its own region beside the where clause |
| `aggregate.rq` | `GROUP BY`, `HAVING` and a `SUM` alias — solution modifiers as header content |

**Provenance.**

- Source: "SPARQL 1.1 Query Language", W3C Recommendation 21 March 2013,
  `https://www.w3.org/TR/sparql11-query/`. Sections, in the table's order: 2.2 (Multiple
  Matches), 6.1 (Optional Pattern Matching), 7 (Matching Alternatives), 3.1 (Restricting the
  Value of Strings), 9.2 (Examples), 16.2 (CONSTRUCT), 11 (Aggregates).
- Retrieved: 2026-09-03.
- License: the W3C Document License, which the specification's own footer applies via the W3C
  document-use rules (`https://www.w3.org/Consortium/Legal/copyright-documents`). It permits
  copying and distribution with attribution and without modification — exactly this folder's
  use. The sparql-diagram requirements name the W3C Software and Document License as the
  accepted W3C license; this document predates that combined license, and the older Document
  License is the stricter of the two, satisfied here by copying verbatim with this attribution.
  See `LICENSE.md`.
- Exact local changes: none to any query's text. Each query is the example verbatim, in its own
  file with a CRLF line ending per the repository's house style, and with a sibling `.adp`
  registration this repository adds so the file opens as a diagram. The specification presents
  each query beside the data it runs against; only the query is copied, because only the query
  is what this diagram draws.

Copyright © 2013 W3C® (MIT, ERCIM, Keio, Beihang).
