# Mindmap test fixtures

Real `.mm` files the mindmap module's parser, serializer and layout are tested against.

## Provenance

`architecture.mm` is a **genuine Freeplane save**: the original hand-written corpus was opened
in Freeplane 1.12.15 on Windows, saved unchanged, and committed exactly as Freeplane wrote it
(mindmap-diagram task 1). `.gitattributes` marks `*.mm` as `-text` so those bytes survive
checkout on every platform. Do not hand-edit this file; replace it only with another genuine
save.

The load-and-save settled the two Requirement 12 questions the spec had asserted on an
unverified reading:

- A code-artifact link lives in the node's **`LINK` attribute** — Freeplane read our
  hand-written `LINK="../../../../backend/.../ContextServiceImpl.cs"` and wrote it back
  byte-identically, as it did the folder link and the `https` URL.
- Freeplane resolves such a link **relative to the map file** — it loaded the map with those
  relative links intact and preserved them as relative paths on save.

## What the genuine save changed — the schema knowledge the hand-written fixture lacked

Every difference between the hand-written 1.11.5 corpus and Freeplane 1.12.15's save of it,
recorded here because each one is something the parser/serializer would otherwise have been
written without knowing:

1. **Line endings are mixed, by design.** The structural XML — every element line — ends
   `CRLF` (the platform newline; this was a Windows save), while the HTML bodies inside
   `richcontent` end `LF`: Freeplane serializes those with a separate writer. An XML parser
   normalizes `\r\n` to `\n` while reading (XML 1.0 §2.11), which is why
   `MindmapDocument.Parse` escapes the `CR`s first and the writer re-emits them raw — without
   that, no Windows save could ever round-trip byte for byte. Edits synthesize the file's own
   structural newline (`FreeplaneNewline`).
2. **`POSITION` renamed its values**: `right`/`left` became `bottom_or_right`/`top_or_left`.
   The layout honours both vocabularies; ADP does not write the attribute itself.
3. **Every node carries an `ID`.** Freeplane assigned ids to the two deliberately ID-less
   nodes on save — a genuine save can never contain an ID-less node, so that tolerance (files
   no Freeplane has saved yet) is exercised by a hand-written map inside
   `MindmapDocumentTests.AssigningMissingIds` instead, and the corpus guard asserts the
   opposite: all nodes have ids.
4. **A new element can appear before the root node**: `<bookmarks/>` is written as a direct
   child of `<map>` ahead of the `<node>` tree. A parser that assumes `map`'s first element
   child is the root node breaks.
5. **`richcontent` lost `CONTENT-TYPE="xml/"`** — 1.12 writes only `TYPE`. Both forms must
   parse.
6. **Whitespace layout tightened**: `</html></richcontent>` is written as one line, and a
   `<richcontent>` opens on the same line as its owning `<node ...>` tag. Whitespace-exact
   round-tripping, not pretty-printing, is what survives this.
7. **Child element order**: Freeplane writes `icon` before `edge` (the hand-written file had
   them reversed) and `arrowlink` before `edge`. Preserved as-is; nothing may reorder.
8. **`arrowlink` inclinations gained units**: `STARTINCLINATION="86;0;"` became
   `"64.5 pt;0 pt;"`.
9. **The MapStyle hook grew**: new `properties` attributes (`show_icons`,
   `show_icon_for_attributes`, `show_tags`, `showTagCategories`), a new
   `<tags category_separator="::"/>` element, and a much larger default `map_styles` tree.
   All opaque to ADP; all must survive untouched (Requirement 3.2).
10. **The version comment gained a trailing space** before `-->`. Even the comment is not
    stable across versions — one more reason nothing regenerates what it did not change.

Corners no genuine save exercises, pinned by unit tests instead: a `CR` inside an attribute
value is written escaped (`&#xd;`, matching the `&#xa;` Freeplane uses for `LF`), and a lone
`CR` not followed by `LF` does not occur in Freeplane output and is not preserved.

## What `architecture.mm` deliberately contains

Each of these exists to break a naive parser or a naive serializer:

| | Feature | Why it is here |
|---|---|---|
| Structure | 25 nodes, 5 levels deep, both `POSITION` sides (`bottom_or_right`, `top_or_left`) | Layout and tree handling (Requirements 4, 5) |
| Identity | an `ID` on every node — what a genuine save guarantees | Ids the store and deltas rely on; ID-less tolerance lives in an inline hand-written test map |
| Links | a relative file path, a folder path, and an `https` URL | Requirement 12.1's storage form, confirmed by Freeplane itself |
| Folding | `FOLDED="true"` on a branch with children | Fold state seeded from the file (Requirement 9.3) |
| Notes | `richcontent TYPE="NOTE"` | Requirement 7.7 |
| Rich text | `richcontent TYPE="NODE"` — a node whose text is markup, not a `TEXT` attribute | A node's text is not always where you expect it |
| Empty text | a node with `TEXT=""` | Requirement 7.6: empty node text is valid |
| Unknown to ADP | `bookmarks`, `hook`, `MapStyle`, `map_styles`, `stylenode`, `tags`, `properties`, `font`, `edge`, `icon`, `cloud`, `arrowlink`, `attribute` | Requirement 3.2: preserved untouched, never regenerated |
| Escaping | `&amp;`, `&lt;`, `&gt;` in attributes; non-ASCII and an emoji written literally, as Freeplane writes them | Requirement 3.3: byte-stable serialization |
| Line endings | CRLF structure with LF `richcontent` bodies, and a CRLF final newline | Requirement 3.3 against what Freeplane actually writes |

A parser that models only "node with TEXT and children" will read this file and lose most of
it. That is the point.
