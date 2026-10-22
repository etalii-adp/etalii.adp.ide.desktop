# SPARQL query examples

Real, found-online queries on the main path — not toy strawmen — per sparql-diagram
Requirement 8 and the vendoring discipline the RDF family established: every source's license
was individually verified before anything was copied, each folder's `readme.md` records
provenance (source URL, retrieval date, verified license, exact local changes), and unmodified
content is unmodified.

- **`w3c-sparql/`** — seven queries from the SPARQL 1.1 Query Language recommendation itself,
  verbatim. Between them they exercise the constructs the drawing rules are about: a plain
  basic graph pattern, `OPTIONAL`, `UNION`, `FILTER`, a property path, `CONSTRUCT` with its
  template, and an aggregate with `GROUP BY`/`HAVING`.
- **`uniprot/`** — two queries from the SIB Swiss Institute of Bioinformatics' curated UniProt
  corpus: a real proteome query using `DISTINCT`, comments and two `BIND` expressions, and a
  bare `DESCRIBE`, which is the whole query.
- **`adp/`** — one query written here rather than found, and labelled as such: an `ASK`, so the
  fourth query form is covered. Its folder readme says plainly that it is ADP's own, because
  claiming a provenance a file does not have would be worse than having none.

**A note on what "verified" meant here.** Two candidate sources were checked and one was
rejected: the Wikidata example-queries wiki page is CC BY-SA, and share-alike is refused by the
vendoring rule, so nothing from it is here. That rejection is recorded in the requirements too,
so the verification stays visible rather than becoming folklore.
