# UniProt queries from the SIB curated corpus

**What they demonstrate.** Queries people actually run against a public endpoint, which is the
point of having them: `proteome-location-of-gene.rq` carries `DISTINCT`, two `BIND` expressions
and the author's own comments, so the diagram has to draw a real query's shape rather than a
tidy one; `describe-embl-cds.rq` is a bare `DESCRIBE` with no where clause at all, which is the
smallest legal query this module can be handed and a good test of a header band that has to say
something useful when the canvas is nearly empty.

**Provenance.**

- Source: `https://github.com/sib-swiss/sparql-examples`, the SIB Swiss Institute of
  Bioinformatics' curated example corpus for its public endpoints, files
  `examples/UniProt/103_uniprot_proteome_location_of_gene.ttl` and
  `examples/UniProt/15_describe_an_EMBL_cds.ttl`. The corpus's own target endpoint is
  `https://sparql.uniprot.org/sparql/`.
- Retrieved: 2026-09-03.
- License: verified at the repository's `LICENSE.md` — the MIT License, © 2024 SIB Swiss
  Institute of Bioinformatics, with the example queries additionally offered under CC BY 4.0.
  Both are on the accepted list; MIT's condition is that its copyright and permission notice
  travel with the copies, and CC BY's is attribution. Both are satisfied by `LICENSE.md` beside
  this file and by the attribution here. Verified 2026-09-03.
- Exact local changes: the query text of each is verbatim. The corpus stores each query as an
  RDF description whose `sh:select`/`spex:describe` property holds the query string; what is
  copied here is that query string alone, extracted into a `.rq` file with CRLF line endings per
  the repository's house style. The surrounding RDF metadata — the comment, keywords and target
  endpoint — is not copied, and is summarised in this readme instead. A sibling `.adp`
  registration is this repository's own addition.

The corpus's own descriptions of these two queries, for the record: "List UniProtKB proteins
with genetic replicon that they are encoded on using the Proteome data." and "Select all triples
that relate to the EMBL CDS entry AA089367.1".
