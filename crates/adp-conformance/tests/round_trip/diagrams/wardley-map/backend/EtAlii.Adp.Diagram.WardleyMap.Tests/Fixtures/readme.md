# Wardley map test fixtures

`.owm` documents the Wardley map module's parser, writer and identity sidecar are tested
against (wardley-map task 2).

`.gitattributes` marks `*.owm` as `-text`, so these bytes survive checkout on every platform.
Requirement 3.1 says a document ADP did not change comes back byte-identical, and the tests
check that by comparing what the writer produced against the file on disk — with
`core.autocrlf` on, git would otherwise rewrite the line endings and the test would be
measuring git rather than the writer. **Replace a fixture only by committing what an authoring
tool actually wrote, or by certifying the replacement (below).**

## Certification

Every file here was parsed by the **real OnlineWardleyMaps parser** before being committed —
the one vendored by [`cli-owm`](https://github.com/monkeypants/cli-owm) **0.0.2** (MIT,
Copyright 2019 Damon Skelhorn). This is the counterpart of the C4 corpus being certified by
the Structurizr CLI: it is what separates "real DSL" from "ADP's idea of the DSL".

```
npm install --no-save cli-owm@0.0.2
node certify.mjs *.owm
```

`certify.mjs` prints what the parser understood and every `ParseError` it raised, and exits
non-zero if any file errors — so it can gate CI later. `CERTIFY_DUMP=1` additionally prints
each parsed component.

**No third-party map content is committed here.** The Wardley mapping method is Simon
Wardley's, CC BY-SA 4.0, and its share-alike terms are not something a test corpus should
quietly impose on this repository. Every file below was written for this repository and then
certified against the real parser, which is the same route the C4 corpus took.

**Caveat worth keeping.** `cli-owm` 0.0.2 vendors a parser snapshot, and the upstream DSL may
have moved since. A construct this corpus records as invalid may be valid in a newer
OnlineWardleyMaps; re-certify against a newer vendored parser before concluding otherwise.

## What certification found

Two things, recorded here because they contradict what the approved requirements assume and
because a later reader will otherwise repeat the mistake.

1. **`market` and `ecosystem` are decorators, not statements.** `market Foo [0.6, 0.8]` is a
   parse error. The real syntax is `component Foo [0.6, 0.8] (market)`, and the parser puts it
   in the same `decorators` object as `build`, `buy` and `outsource`:

   ```json
   {"name":"BetaMarket","type":"component",
    "decorators":{"ecosystem":false,"market":true,"buy":false,"build":false,"outsource":false}}
   ```

   Requirement 5.1 lists `market` and `ecosystem` alongside `component`, `anchor` and `submap`
   as statement kinds, and Requirement 6.3 lists only three decorators. The DSL's real split is
   **three statement kinds** (`component`, `anchor`, `submap`) and **five decorators**
   (`market`, `ecosystem`, `build`, `buy`, `outsource`). `inertia` is neither — it is a
   separate boolean on the component.

2. **`[a, b]` is `visibility, maturity`,** which confirms Requirement 5.2 rather than
   contradicting it: `component Alpha [0.80, 0.40]` parses to `visibility: 0.8, maturity: 0.4`.
   Worth having pinned by evidence, since it is the module's most error-prone single line.

Also noted, since each shapes a test: the parser's `line` numbers are **0-based** (ADP's
`DiagramProblemLineLocation.Number` is 1-based); an **unknown decorator** such as
`(someDecoratorFromLater)` is tolerated silently; and `y_axis`, `style` and `size` all parse
without error.

## Provenance

Two kinds of file live here, and the difference matters.

### Hand-made edge cases — deliberate, and correct to hand-write

These exist to pin one specific property each. Task 2 asks for them explicitly; they are not
approximations of anything, so writing them by hand is the right way to get them.

| File | What it guards |
|---|---|
| `minimal.owm` | The smallest document that must round-trip: a title, an anchor, a component, a link. |
| `comments-everywhere.owm` | `//` comments before the title, between elements, trailing an element, trailing a link, between links, and at end of file. Requirement 3.3 — none may be dropped or moved. |
| `crlf-line-endings.owm` | Identical content to `lf-line-endings.owm`, written CRLF throughout. |
| `lf-line-endings.owm` | The same content written LF throughout. The pair proves the writer preserves whichever style it was given rather than normalising. |
| `no-trailing-newline.owm` | A final line with no newline at all — the case a naive line-splitter silently "fixes". |
| `blank-line-runs.owm` | Runs of three and four blank lines, and two trailing blank lines. Requirement 3.2 — unaffected lines are not compacted. |
| `unmodelled-statements.owm` | `y_axis`, `style`, `size`, an unknown decorator, and one genuinely unknown keyword. Requirement 3.3 — parsed or not, all of it survives verbatim. **This file is expected to report one `ParseError`** (the unknown keyword); that is its purpose, not a defect. |
| `pipelines-both-forms.owm` | The nested form, the legacy two-coordinate form, and an empty pipeline. Requirement 5.4 — each is written back in the form it was read. |
| `annotations-and-labels.owm` | One annotation pinned at **two** coordinates beside one pinned at one, an `annotations` position block, `label [dx, dy]` offsets, and a note. Requirements 6.7, 6.8, 5.5. |
| `submaps-and-urls.owm` | Two submaps with `url(...)` references and their `url` definitions. Requirement 6.6. |

### Realistic documents — hand-written, then certified

The small files above cannot catch a parser written against a plausible-but-wrong mental model
of the DSL, because each contains too little at once. These two exist for that.

| File | What it guards |
|---|---|
| `tea-shop.owm` | The value-chain shape every Wardley map has: two anchors, a seven-component chain, label offsets, and eight links, in one document. A parser tested one construct at a time comes apart here. |
| `strategy-vocabulary.owm` | The strategy vocabulary end to end in one map: `(build)`, `(buy)`, `(outsource)`, `(market)`, `(ecosystem)`, `inertia`, two `evolve` forms (plain and the `->` rename), a flow link `+>`, a link with `; context` text, all three attitude areas, an `accelerator`, a `deaccelerator`, a `note` and a `size`. This is the file that found the `market`/`ecosystem` discrepancy above. |
