# Dependency graph round-trip corpus

These documents exist so that the byte-identical claim has something real to fail against. Every
one of them was **written before this module's parser and writer ran against them**, so the
implementation had to meet the documents rather than the documents being trimmed to match an
implementation. A fixture written to match the code it tests proves only that the code agrees
with itself.

## Provenance, stated honestly

`.dgr` is ADP's own format, so unlike the C4 and Azure Pipelines corpora there is **no external
corpus to draw from** — nobody else writes these files yet. That is a real weakness and worth
naming rather than glossing: a hand-written fixture embodies its author's assumptions about the
format, and those assumptions are exactly what a corpus is supposed to be independent of.

This corpus carries a second, narrower caveat: this module is a fork of the timeline's, and its
fixtures are the timeline's rewritten with the dates taken out. So they inherit that corpus's
assumptions as well as this author's. What they do **not** inherit is any date: no fixture here
carries a begin, an end, a duration or an instant, and a `.dgr` that did would be a defect.

Two things mitigate the rest. Each file is built around a property that is **about text rather
than about graphs** — line endings, comments, indentation, unknown keys, quoting — so the
interesting cases come from how files behave in general, not from anybody's idea of what a
dependency graph should contain.

When real `.dgr` files exist, written by someone other than this module's author, they belong
here and these should be reduced to the edge cases they encode.

## What each file guards

| File | Exists to prove |
| --- | --- |
| `simple.dgr` | The ordinary case: two nodes and one directed dependency between them. If this fails, nothing else matters. |
| `shared-rows.dgr` | A row is a placement grid, not a lane. Several nodes share row 0, the indices are neither contiguous nor ascending, and row 12 exists with nothing between. |
| `coordinates.dgr` | `x` is a plain number and its written form is the author's: whole, fractional, negative and zero all round-trip unchanged. A whole `320` returning as `320.0` renders identically on a canvas, so this failure is silent unless the bytes catch it. This is the file that replaces the timeline's `mixed-precision.tml`, whose subject went with the dates. |
| `relations.dgr` | Two dependencies between the same pair are legal — the label carries the meaning — and a dependency need not carry a label at all. |
| `comments.dgr` | Comments in every position a YAML document allows: leading, trailing, between keys, between nodes, and on the end of a line. Plus blank lines. None of it is the module's to reformat. |
| `lf-line-endings.dgr` / `crlf-line-endings.dgr` | The same document under both conventions. These are a **pair**: if line-ending normalisation is ever in play, they collapse into one file and stop testing anything — which is what the `.gitattributes` entry below prevents. |
| `no-trailing-newline.dgr` | The last line has no terminator. Appending must not add one, and removing the last line must move its ending to the new last line. Two sibling modules shipped both bugs before finding them. |
| `indentation.dgr` | Valid YAML in an indentation no generator would choose — four spaces at the sequence level, dashes indented under their key. A writer that "tidies" this has broken the round-trip discipline even though the model is unchanged. |
| `scalars.dgr` | Quoting is the author's choice: double-quoted, single-quoted with an embedded apostrophe, plain with trailing whitespace, escaped quotes, and non-ASCII. Requoting is a rewrite. |
| `unmodelled-keys.dgr` | Keys this module does not model, at the document, element and relation level, including a nested sequence. Every one survives verbatim. |

## Line endings are load-bearing here

`*.dgr -text` is registered in the repository's root `.gitattributes`, beside `*.mm`, `*.dsl` and
`*.tml`. Without it, git's `text=auto eol=crlf` policy would rewrite these files on checkout, the
LF/CRLF pair would become the same file, and `no-trailing-newline.dgr` would quietly gain one —
at which point these tests would be measuring git rather than the writer.

That entry is a prerequisite for this corpus, not a detail. If a round-trip test starts failing
in a way that makes no sense, check that the attribute is still there before suspecting the code.

## The documents of the parity corpus, one per finding

The parity transcript also records the inline documents of `Parity/DependencyGraphCorpus.cs`. Those
added by dependency-graph-disl-fbl task 1 hold one finding each that the specification's requirements
mark Read, and one for the change Q2 names; none of their bytes is about line endings or a final
newline, so none is a file here. Each is recorded as today's code answers it, crash and broken body
included.

| Document | Finding | What it holds |
| --- | --- | --- |
| `inline/a4-id-held-twice` | A4 | Two holders of `aaa`, a menu and a row asked by that id. |
| `inline/b1-scalars-as-written` | B1 | `id: 007` and `id: 7`, labels `1.0`, `true` and `~`, an end `0x1F`. |
| `inline/b2-numbers-read-otherwise` | B2 | X written `0x10`, `.inf`, `Infinity` and `NaN`, beside the three of `inline/unreadable-values`. |
| `inline/b6-empty-list-with-comment`, `inline/b6-empty-list-spaced` | B6 | `elements: []` with a comment after it, and `elements: [ ]`; the first node added to each. |
| `inline/b7-value-styles` | B7 | A label in the author's quotes with a trailing comment, an X with an anchor and an alias to it, a block-scalar label, and labels written `123` and `true`. |
| `inline/b9-entries-indented-apart` | B9 | Entries with dash gaps of one and three, and a list indented by nothing. |
| `inline/b10-comments-above` | B10 | A comment directly above an entry and above a `label` line, for a node and a dependency. |
| `inline/b11-holders-and-an-anonymous-node` | B11 | Two holders of one id, each with a dependency, and a node with no id. |
| `inline/b14-wide-dash-gap-and-nested-key` | B14 | `-   label: A`, and a dependency with a nested key of the same name before its own. |
| `inline/b16-merge-key-and-flow-entry` | B16 | A `row` given through a merge key, and a node and a dependency written as flow mappings. |
| `inline/b17-x-written-small-and-large` | B17 | X written as `0.00001` and `1e21`. |
| `inline/q2-repeated-key` | Q2 | A node that says `label:` twice. |
