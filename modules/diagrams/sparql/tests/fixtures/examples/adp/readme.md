# One query written here, not found

**This folder is ADP's own, and that is the whole point of it being separate.** Requirement 8.3
asks the corpus to cover each of SPARQL's four query forms. `SELECT`, `CONSTRUCT` and `DESCRIBE`
are covered by the vendored folders beside this one; no `ASK` query turned up in either verified
source during the acquisition pass, and writing one while filing it under someone else's
provenance would have been worse than having no example at all.

So `ask.rq` was written here. It is not vendored, carries no third-party license, and needs no
attribution.

**What it demonstrates.** The `ASK` form — a query whose answer is a yes or a no, so the header
band is the only place a result could ever be described — over a pattern that still has a real
shape: a predicate list, two variables joined through `?friend`, and a `FILTER` annotating the
group. It is the smallest query that shows the header band mattering.
