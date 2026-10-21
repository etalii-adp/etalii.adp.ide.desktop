# The Nobel Prize dataset

**What it demonstrates.** `nobel-1903.ttl` is one coherent year — categories, prizes and
laureates as linked resources, motivations as language-tagged literal rows — small enough to
read whole; 1903 is the year both Curies stood in Stockholm, tying this folder to the Wikidata
one. `laureates.ttl` is the whole dataset: 1,018 laureates, 633 prizes and 6 categories — 1,657
resources, deliberately past the 1,000-node drawn-element budget, so the truncated first-N view
and its showing-N-of-M banner have a real file to be honest about (Requirement 8).

**Provenance.**

- Source: the official Nobel Prize API, `https://api.nobelprize.org/2.1/laureates` (paged) and
  `https://api.nobelprize.org/2.1/nobelPrizes?nobelPrizeYear=1903`.
- Retrieved: 2026-09-03, by running `node generate.mjs` in this folder.
- License: Creative Commons Zero (CC0), per the Nobel Prize terms of use for
  api.nobelprize.org and data.nobelprize.org
  (`https://www.nobelprize.org/about/terms-of-use-for-api-nobelprize-org-and-data-nobelprize-org/`).
  See `LICENSE.md`.
- Exact local changes: the two `.ttl` files are **derived**, not copied — the API serves JSON,
  and the historical linked-data endpoints at `data.nobelprize.org` now serve an application
  shell rather than Turtle. `generate.mjs`, committed beside the data, is the complete
  derivation: it fetches the JSON, sorts deterministically, and writes these files; re-running
  it regenerates them against the live API. CC0 permits derivation without condition. The
  resource IRIs reuse `data.nobelprize.org`'s historical URI shapes
  (`.../resource/laureate/{id}`), so the derived data stays linkable to the original scheme.
