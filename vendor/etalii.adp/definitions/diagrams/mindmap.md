# Mind map (`freeplane/mindmap`)

This is the companion to [`mindmap.dis`](mindmap.dis), the DISL 0.3 specification of the Mind map diagram type. The `.dis` holds everything DISL can say: the one node type and its attributes, the derived branches, the notation, the toolbox and the context menu, the property form, the three rules, the deletion confirmation, the per-viewer folding and the operations. This file holds everything it cannot: the Freeplane `.mm` file format the persistence plugin reads, the two-sided layout algorithm, sibling naming, links, the drag-to-re-parent gesture, and the places where the `.dis` could only approximate what the standalone implementation does.

The standalone host runs this type from the `.dis`: its toolbox, context menus, property rows and findings are derived from it, and its canvas is compiled from the notation. Where the `.dis` and the standalone code disagreed, the code won and the `.dis` was rewritten to say what the code does (Peter's ruling for the DISL switch-over, 2026-10).

Every statement here is tied to the source that shows it. Paths are in [etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone) on `develop` unless another repository is named; `src/diagrams/mindmap/` is abbreviated to `mindmap/`. "Req" refers to the standalone spec `mindmap-diagram`, which was removed from the tree and is read with `git show "ece03c36^:.spec-workflow/archive/specs/mindmap-diagram/requirements.md"` (and `design.md`, `tasks.md` beside it).

## Sources

| Source | What it contributed |
|---|---|
| `mindmap/backend/EtAlii.Adp.Diagram.Mindmap/` | The model, the `.mm` reader and writer, the layout, the commands, the context actions and properties, the validator, the view state and the session. |
| `mindmap/client/` (`MindmapCanvas.tsx`, `mindmap.css`, `mindmapModel.ts`, `readme.md`) | The notation, the declared actions and keys, the drag behaviour, the styling. |
| `mindmap/api/mindmap.proto` | The node payload on the wire. |
| `mindmap/examples/readme.md`, `mindmap/backend/EtAlii.Adp.Diagram.Mindmap.Tests/Fixtures/readme.md` | What real Freeplane files contain, and what the shipped examples do not exercise. |
| `.spec-workflow/archive/specs/mindmap-diagram/` (in history before `ece03c36`) | The requirements (Req 1-13), design and tasks; all 24 tasks are done. |
| `docs/tools.md` row `freeplane/mindmap` | Catalogue state: ⚗️ Prototype, kind Diagram. |
| Notion, "Tools" database, row "Mind map" | Purpose ("Lay out ideas as a tree of branches around one central topic, in the FreeMind/Freeplane format"), why specialized ("A mind map's value is its radial hierarchy; a dedicated tool keeps branches ordered and foldable while the .mm file stays readable by Freeplane"), focus areas "Clarity in textual data" and "Knowledge and semantics", family "Knowledge & informal modeling"; Standalone ⚗️ Prototype, IntelliJ ✅ Implemented. |
| [etalii.adp.ide.intellij](https://github.com/etalii-adp/etalii.adp.ide.intellij) `specs/001-freemind-mindmap-designer/spec.md`, module `freemind/` | How the IntelliJ host treats the same files differently (section 12). |
| Project conversation "FreeMind layout, panning, centering" (2026-09-28) | Peter's IntelliJ layout ruling: wider horizontal spread so branch curves never bend back, drag-to-pan on empty canvas, open centred on the root. |

## 1. Identity

- Origin tag `freeplane/mindmap`, display name "Mind map", description "Ideas branching from one central topic, for thinking a subject through rather than specifying it.", icon `mdi-family-tree`, body extension `.mm` (`Diagram.cs`).
- Freeplane is named as the vendor because it is the tool most associated with the `.mm` format; other mind-map tools (XMind, MindMeister, Coggle, FreeMind) would each get their own `<vendor>/mindmap` row if their formats were ever read (`docs/tools.md`, section 10).
- The DISL `language.id` is `net.etalii.adp.freeplane.mindmap`, because a DISL id cannot contain the `/` of the origin tag.

## 2. Files: registration and body

DISL's `persistence.files` describes the files of one diagram, but not the standalone host's two-file pair or its routing, so they are recorded here.

- A standalone diagram is a pair: `<name>.adp`, the registration, whose first line is the MIME type `freeplane/mindmap` (`mindmap/examples/example 1/mindmap.adp`), and `<name>.mm`, the body, a sibling with the same base name (Req 2.1, 2.2). No third file is ever written; anything ADP-specific that is ever needed goes into the `.adp` after the MIME line (Req 3.5, 12.4).
- An `.adp` routes by its MIME line (Req 2.3); a bare `.mm` without an `.adp` still opens, routed by extension, and no `.adp` is created for it (Req 2.7). An unknown MIME type is reported by name (Req 2.6); two definitions claiming one extension disable extension routing and report the ambiguity (Req 2.8).
- Renaming or moving the `.adp` moves the `.mm` with it as one undoable command (Req 2.10). Deleting the `.adp` deletes both after a confirmation that names both files, and is not undoable (Req 2.11).
- A missing or empty body opens as an empty map named after the file and is written by the first save (Req 2.5, 2.12; `MindmapDocumentStore.Parse`).
- A new diagram is created from the Add dialog under the `freeplane` vendor group, default name `mindmap`, then `mindmap-2` and so on; both files are written or neither (Req 1). Its body is one root node whose text is the base name: `<map version="freeplane 1.11.5">`, LF newlines, `<node TEXT="<name>" ID="ID_<short id>"/>` (`MindmapDocumentFactory.cs`).
- Other hosts need not use a registration file: IntelliJ opens the `.mm` alone (section 12).

## 3. The `.mm` format

`mindmap.dis` stores the map through FBL (`persistence.format: "fbl"`): its binding `mindmap.fbl#mindmap`, which lives beside the standalone module (`mindmap/backend/EtAlii.Adp.Diagram.Mindmap/mindmap.fbl`), names the persistence plugin `net.etalii.adp.freeplane.mm` as its reader, because no declared FBL rule reads nested `<node>` elements whose text may be HTML. The plugin reads every `<node>` as a `Node` with `text`, `notes`, `link` (only when `LINK` is present, empty or not), `folded` and `position`, its parent and its place, and keeps the `ID` the file writes as `storedId`. A later holder of an `ID` the file gives twice, and a node without one, is read under an ephemeral id; findings name it by what the file writes. A file the plugin cannot read is unreadable, with the reason as `std.unparseable`'s `detail.reason`. Edits are not planned by the plugin in the standalone host: they are made in place on the XML by the module's commands, which carry their own undo. The rest of this section is the plugin's contract.

### 3.1 The principle: the XML is the model

The XML a map was parsed from is the single source of truth. Reading a node reads its element; editing a node edits its element; saving writes the XML back rather than regenerating it. Everything ADP does not understand therefore survives exactly as it was (`_Model/MindmapNode.cs`, `MindmapDocument.cs`; Req 3.2, 3.3). This is why the `.dis` states no canonical form, ordering or `omitDefaults`: with `format: "fbl"` DISL forbids them, and the file's own conventions are kept.

### 3.2 Structure

- The document element is `<map>`; it holds exactly one `<node>`, the central topic. Anything else - no `<map>`, zero or several root nodes, malformed XML - is not a map and fails with a message naming the file; only that diagram fails, never the workspace (`MindmapDocument.Parse`, `MindmapFormatException`; Req 3.8).
- Non-node elements may precede the root node: a genuine Freeplane 1.12 save writes `<bookmarks/>` before it, so the root is found by name, not by position (Fixtures readme, item 4).
- Child nodes are nested `<node>` elements; their order is meaningful and kept.

### 3.3 What is read from a node

| Construct | Meaning | Source |
|---|---|---|
| `ID` | The node's identity; the element id on the wire and in selections. | `MindmapNode.Id` |
| `TEXT` | The node's text. | `MindmapNode.Text` |
| `richcontent TYPE="NODE"` | Formatted text. When present it wins over `TEXT`; its HTML `<body>` is read as plain text, one line per `<p>`, whitespace collapsed. | `MindmapNode.Text`, `PlainTextOf` |
| `richcontent TYPE="NOTE"` | Notes, read as plain text the same way. | `MindmapNode.Notes` |
| `FOLDED="true"` (case-insensitive) | Seeds the collapsed state when a viewer opens the map (section 5). | `MindmapNode.Folded` |
| `LINK` | The link, exactly as stored (section 8). | `MindmapNode.Link` |
| `POSITION` on a first-level node | The side of the central topic: `left`/`right` (Freeplane 1.11) or `top_or_left`/`bottom_or_right` (1.12). | `MindmapNode.Position`, `MindmapLayout` |

`richcontent` is read with or without `CONTENT-TYPE="xml/"`, which 1.12 stopped writing (Fixtures readme, item 5).

Everything else is preserved and neither shown nor edited: `hook`, `MapStyle`, `map_styles`, `stylenode`, `tags`, `properties`, `font`, `edge`, `icon`, `cloud`, `arrowlink`, `attribute`, `bookmarks`, comments (Fixtures readme; `mindmap/examples/readme.md`).

### 3.4 What is written

- **Text:** the `TEXT` attribute. Setting text removes any `richcontent TYPE="NODE"`, so formatted text becomes plain text on rename, without a warning (`MindmapNode.SetText`).
- **Notes:** `<richcontent TYPE="NOTE"><html><head/><body><p>line</p>…</body></html></richcontent>`, one `<p>` per line, placed before the first child node on its own line. Emptying the notes removes the element and the newline after it, and a node left with only whitespace becomes self-closing again (`MindmapNode.SetNotes`).
- **Link:** the `LINK` attribute; unlinking removes it.
- **New nodes:** `<node TEXT="…" ID="ID_<short id>"/>`, attributes in that order.
- **Never written:** `FOLDED` (fold is view state, Req 9.4), `POSITION`, and any layout position (Req 5.8).

### 3.5 Identifiers

- Every node has an `ID` once loaded. A node without one is given `ID_` followed by a ShortGuid in memory, and the id is written on the next save - never on open, and never by the read-only validator (`MindmapDocument.AssignMissingIds`, `NewId`; Req 3.4).
- Freeplane's own ids are `ID_` plus digits; a genuine Freeplane save never contains an id-less node (Fixtures readme, item 3).
- `mindmap.dis` declares `uuid-v4` with prefix `ID_` and `base36` encoding, the ShortGuid above.

### 3.6 Byte-exact round trip

A map that ADP opens and saves without changing is written back byte for byte; a changed map differs only where it changed (Req 3.3). The writer (`FreeplaneXmlWriter.cs`) is written to match Freeplane, not a generic XML writer:

- no XML declaration; `/>` with no space before it; attributes in the order they were read; whitespace text exactly as loaded;
- in text, only `&`, `<` and `>` are escaped; in attributes also `"`, and a newline as `&#xa;` and a carriage return as `&#xd;`;
- non-ASCII characters and emoji are written literally in UTF-8.

**Line endings are mixed by design.** A Windows Freeplane save ends its structural lines with CRLF while the HTML inside `richcontent` uses LF (Fixtures readme, item 1). An XML parser normalises CRLF to LF, so the reader escapes each CR before parsing and the writer emits it raw again; the whitespace after `</map>` is kept verbatim (`MindmapDocument.Parse`, `ToText`). Structural edits insert the file's own structural newline, read from the whitespace after `<map …>` (`FreeplaneNewline.cs`). In the repository `.gitattributes` marks `*.mm` as `-text`, so fixture bytes survive checkout. `mindmap.dis` therefore sets no `persistence.newline`.

Other 1.11-to-1.12 differences the writer tolerates by not rewriting anything: `icon` before `edge`, `arrowlink` inclinations with `pt` units, a larger `MapStyle`, a trailing space in the version comment, `</html></richcontent>` on one line (Fixtures readme, items 6-10).

### 3.7 Saving, reloading and failure

- Saves are atomic, via a temporary file moved into place (Req 3.7; `WritableDocumentLifecycle`). A failed write returns an error and leaves the edit in memory to be retried (backend-centralization R3.2, R3.4).
- One loaded document is shared by every viewer of a map (Req 2.4).
- An external edit of the `.mm` is reloaded and pushed to every viewer; it is not undoable (Req 11.8, the shared `DiagramDocumentReloader<IMindmapDocumentStore>`). A reload that cannot parse keeps the last good map; deleting the body clears the map to the empty state (backend-centralization R2.4, R2.5).

## 4. Layout

`mindmap.dis` names `plugin:net.etalii.adp.freeplane.mindmapLayout` with an `mrtree` fallback, sets `trigger: always` (users never place nodes) and passes the metrics as options. The algorithm is `MindmapLayout.Compute` (Req 5.1):

1. Measure every node (below). Place the central topic with its centre at the origin (0, 0).
2. If the central topic is collapsed, stop.
3. Split the first-level children into a right and a left list, in document order. `POSITION` `left` or `top_or_left` sends a child left; `right` or `bottom_or_right` sends it right; a child with no or another value alternates, starting right, counting only unpositioned children (right, left, right, …), which is the rule Freeplane applies to an unpositioned map.
4. Lay out each side as a column of subtrees vertically centred on the central topic, starting at the central topic's edge plus `gapBeside(rootWidth)`: to the right from its right edge, to the left from its left edge (nodes on the left are right-aligned to that x).
5. A column stacks its subtrees top to bottom. Each subtree's height is the larger of its own node's height and its visible children's stacked heights plus gaps; a collapsed node or a leaf counts only itself. Between two adjacent subtrees the gap is `gapBetween(widthA, widthB)`, from the two subtrees' own node widths.
6. Each node is vertically centred in its slot; its children form a column at its outer edge plus `gapBeside(nodeWidth)`, centred on the node's centre y, growing away from the centre.
7. Collapsed branches take no space and their descendants get no position.

Metrics (`_Model/MindmapMetrics.cs`), all in canvas units:

| Setting | Value | Use |
|---|---|---|
| Font | `system-ui, sans-serif`, 14 | Text width estimate |
| Line height | 1.4 | Node height |
| Horizontal padding | 10 each side | Node width |
| Vertical padding | 6 top and bottom | Node height |
| Horizontal gap | 48 | Floor of `gapBeside` |
| Vertical gap | 10 | Floor of `gapBetween` |
| Minimum width | 32 | Empty nodes stay clickable |
| Minimum gap ratio | 0.1 | Wide nodes get proportionally more air |

- Node width = max(32, textWidth + 20); node height = 14 × 1.4 + 12 = 31.6; values rounded to 2 decimals.
- textWidth = characters × 14 × 0.55, where a character is a UTF-16 code unit (shared `TextMetric`, `src/backend/EtAlii.Adp.Documents/TextMetric.cs`).
- `gapBeside(w)` = max(48, w × ratio); `gapBetween(a, b)` = max(10, max(a, b) × ratio).
- Only the ratio is configurable, as `Mindmap:MinimumGapRatio` in `appsettings.json` (`MindmapOptions.cs`).
- The layout is a pure, deterministic function of the tree and the fold set: the same map gives the same positions for every viewer and after every reconnect (Req 5.2, 5.5).
- The client draws each node at the size the backend measured and does not re-measure (Req 5.6, 5.7). Positions on the wire are box centres, which is why the `.dis` placement anchor is `center`.

IntelliJ lays out the same files with its own framework; after Peter's 2026-09-28 request it places children 50 px from their parent, caps bezier control-point reach so branches never curve back, pans on a left drag over empty canvas and opens centred on the central topic (intellij PR #16). Those are host choices, not part of this type; the standalone layout above has no panning or centring rule of its own beyond the shared canvas.

## 5. Folding

- `FOLDED="true"` in the file seeds each viewer's collapsed set when that viewer first opens the map (`MindmapConnectionView.SeededFrom`; Req 9.3).
- After that, collapsing and expanding is per connection, keyed by viewer and map (`MindmapViewState`). It is never written to the file, never lands on the undo history, is allowed in read-only mode, and does not outlive the connection (Req 9.4, 9.6; design "Deviations").
- The action is offered only on a node with children, labelled "Collapse" or "Expand", on Space (`MindmapContextActionProvider`).
- A collapsed branch is drawn as its node with the ⊕ glyph; its descendants disappear and the rest of the map re-lays out (`MindmapSession.OnFoldToggled`).
- Revealing a node inside a collapsed branch - by a selection from elsewhere - expands its collapsed ancestors (`FoldedAncestorsOf`; Req 10.5).

DISL 0.2 made this expressible, and the `.dis` now says it: `persistence.view.viewer` lists `collapsed`, `view.initial` seeds it from `self.folded` (the file's `FOLDED`, a read-only attribute), the context menu reads `self.view.collapsed` to show Expand or Collapse, and `toggleFold` is a `view` action. The plugin operation `net.etalii.adp.freeplane.mindmapFold` of the 0.1 `.dis` is gone.

## 6. Notation details DISL does not pin down

- **Node:** a rectangle with corners rounded by 6 (`centered-box`, the custom shape `topicBox` in the `.dis`, whose geometry a standalone test compares with the drawn box), fill `--color-surface`, stroke `--color-border` 1.5, text `--color-text` at 12px, `user-select: none`, pointer cursor (`mindmap.css`). Before the backend has measured a node the client draws it 120 × 32 (`MindmapCanvas.tsx`).
- **Font size mismatch:** the backend sizes boxes for 14px text while the client renders 12px, so boxes have more room than their text needs. Recorded, not resolved.
- **Corner glyphs:** `•` notes, `↗` a non-empty link, `⊕` collapsed with children, joined by single spaces with the ends trimmed (so notes and a fold without a link leave two spaces between them), anchored top right (offset y 12, inset x 4; the `.dis` function `indicators`). A CSS rule meant them muted and 10px but never applied; they render like the node text, and whether they should look as that rule meant is an open question for Peter (`mindmap.css`).
- **Branches:** a cubic bezier from the parent to the child, leaving and entering each box at the middle of its left or right side only (`anchors: { mode: "sides", sides: ["left", "right"] }`), under the boxes, 1.5 wide in `--color-border`, no arrowheads.
- **Branches are not selectable**, by decision: a branch is a node's link to its parent, so pressing it is pressing the background (client readme; centralized-selection Req 2.4).
- **Double-click** does nothing of its own (`doubleClick: "none"`); the text is edited in place through Rename.
- **Drag feedback:** the dragged node at 0.65 opacity; the candidate parent ringed in `--color-primary` (limegreen), 2.5 wide, dashed 6 3; a dashed preview branch from the candidate parent to the dragged node.
- **Theme tokens** in the `.dis` copy the standalone shell's light and dark values (`src/client/src/index.css`).

## 7. Interactions

### 7.1 Keys and actions

| Action | Key | Offered when | Notes |
|---|---|---|---|
| Add child | Insert, Tab | always | Tab is the XMind convention; it invokes the same action and reaches the backend as Insert. |
| Add sibling | Enter | not on the central topic | Inserted right after the node. |
| Rename… | F2 | always | Inline editor over the node. |
| Delete | Delete | not on the central topic | See 7.4. |
| Add notes… / Edit notes… | none | always | Label depends on whether notes exist. |
| Link to… / Change link… | none | always | See section 8. |
| Unlink | none | the node has a link | |
| Collapse / Expand | Space | the node has children | View state only. |

Sources: `MindmapContextActionProvider.cs`, `MindmapCanvas.tsx`. Actions that do not apply are not offered rather than disabled (Req 8.6). F2, Delete and Insert are also the explorer's keys; the innermost level of the current selection decides which provider receives them, never a global key table (Req 8.7). Escape cancels an inline edit and dispatches nothing (Req 7.8). In read-only mode no document-changing action is offered (Req 6.8). The `.dis` writes Space as `"Space"`; the standalone host sends and matches it as the key `" "`, and declares Tab as a second key of Add child, which a DISL entry's one `shortcut` cannot say. Expand and Collapse are two entries of the one `toggleFold` operation, each `visible` for one collapsed state, because an entry's icon is not an expression.

### 7.2 Add, then edit in place

Add child and add sibling create the node at once, under a name taken from its new siblings, and then open the inline editor on it, so the usual path replaces the name before anyone reads it; cancelling leaves the name (`AddThenEditAsync`). A new child is appended as the last child; a new sibling goes right after the node. DISL's `create` action cannot say "right after this node", so that position is the runtime's.

**Sibling naming** (`src/backend/EtAlii.Adp.Diagram/SiblingNaming.cs`), which the `.dis` stubs as the function `siblingName`:

1. Ignore empty sibling names; trim the rest.
2. If siblings share a numbered stem, continue it from one past the highest number, not the count ("Phase 1", "Phase 3" → "Phase 4"); the stem most siblings agree on wins ("Step 1" … "Step 4", "Draft 9" → "Step 5").
3. Otherwise, if at least two siblings share a last word, use "New" plus that word ("Measure pass", "Arrange pass" → "New pass"); one sibling is a coincidence, not a pattern.
4. Otherwise use "Node".
5. Make the result unique among the siblings.

### 7.3 Drag to re-parent

A node is never positioned by dragging; a drag is a re-parenting proposal (`MindmapCanvas.tsx` `onElementMoved`). Released over another node, the dragged node and its whole branch become that node's last child; released over empty canvas, nothing happens. The central topic cannot be moved, and a node cannot be dropped on itself or inside its own branch: the client does not offer such a target and the backend refuses it ("The root node cannot be moved.", "A node cannot be moved into its own branch."; `MoveNodeCommand.cs`). The move is one undo step that restores the previous parent and the previous place among its siblings. Reordering among siblings has no gesture or key (see section 11).

### 7.4 Delete

Deleting a leaf happens without a question. Deleting a branch asks "Delete branch?" with "Delete '<text>' and the N nodes under it? You can undo this.", as a danger action. Both are undoable; the undo restores the whole subtree - ids, text, notes, links and unknown content - at its old place (`RemoveNodeCommand`, `RestoreSubtreeCommand`; Req 7.4). The `.dis` says this with `deletion.confirm`: `count` is the nodes beneath, `threshold` 1, so a leaf goes without asking.

### 7.5 Toolbox

One entry, "Node" (`mdi-card-plus-outline`), "Drop on a node to add a child under it." Dropping it on a node runs Add child on that node; dropping it on empty canvas does nothing (`MindmapToolboxProvider.cs`, `onElementDropped`). DISL's tool kinds either create a node where it is dropped or run an operation on the selection, so the `.dis` declares it as the `addChild` operation and "the node under the drop is the target" is recorded here.

### 7.6 Properties

The property grid shows Text, Notes (multi-line) and Link as editable, each edit one undo step through the same commands as the actions; an emptied link is an unlink. "Collapsed" (category View, only for a node with children) and "Identifier" (category Model) are shown read-only with the explanations the `.dis` form carries. "Collapsed" reads the file's `FOLDED`, not the viewer's fold state, as the standalone grid always has; the 0.1 `.dis` showed the viewer's state and was corrected to the code. Whether the row should follow the viewer instead, which is what its explanation describes, is an open question for Peter.

### 7.7 Selection

- One node at a time: rubber-band selection and Ctrl+click are excluded for mind maps (Req 10.9; multi-select spec Req 7.2).
- A node selection is nested under its diagram file and verified against the map: the node must exist and its path, the chain of texts from the central topic, must match (`MindmapContextSourceResolver.cs`; Req 10.2-10.6). A renamed node keeps the selection; a removed one clears it.

## 8. Links

- Stored in `LINK` exactly as Freeplane defines it: relative to the map file, with forward slashes, or an absolute URL. A genuine Freeplane save confirmed both the attribute and the map-relative resolution (Fixtures readme, "Provenance"; Req 12.1).
- A file link is picked from a tree of the project's files and folders, never typed, so an unresolvable link cannot be created by mistyping (Req 12.7). The tree skips `.git`, `node_modules`, `bin`, `obj`, `.claude`, empty folders and files starting with `~adp-` (`MindmapContextActionProvider.ProjectTree`). The chosen project-relative path is converted to the map-relative form (`MindmapLinks.ToMapRelative`).
- A URL (any absolute non-file URI) is shown but never resolved to a file (`MindmapLinks.IsExternal`).
- A file link never resolves outside the project, however it got into the file (Req 12.9 in code, 12.11 in requirements; `MindmapLinks.ResolveWithinProject`).
- Requirements not met yet, see section 11: the wire carries the raw link only, and following, broken-link display and rewriting links on move are not wired.

## 9. Rules

All three rules are in the `.dis` (DISL 0.3), in the order `constraints.order` gives:

| Rule | Severity | Message | In the `.dis` |
|---|---|---|---|
| `mindmap.not-a-map` | Error | "'<name>' is not a readable mind map: <reason>" | The built-in `std.unparseable` with this code and message; a file that is not readable has no other finding. |
| `mindmap.unnamed-root` | Warning | "The map '<name>' has an unnamed central topic." | The rule `unnamedCentralTopic`. |
| `mindmap.duplicate-id` | Error | "Two nodes share the id '<id>' ('<a>' and '<b>')." | The rule `duplicateId`, on each later holder in document order; the built-in `std.duplicateId` is off because it orders a group by its first member. Two nodes with one id would answer to each other's selections. |

`<name>` is the file the Problems panel attributes the map to (its `.adp`, else the `.mm`) without `.adp`: `fileBase(diagram.file)`. Ordinary nodes without text are deliberately not judged, because Freeplane keeps them (Req 7.6). A map with several root nodes is not readable at all, so the 0.1 rule `singleCentralTopic` is gone.

## 10. Runtime behaviour specific to the standalone host

- **Wire payload** (`mindmap.proto`): each node is an element of type `freeplane/mindmap+node` at its box centre, with `text`, `notes`, `has_children`, `folded` (always false; fold is the receiver's own state), `link`, `parent_id`, `width`, `height`. Branches are not sent; the client derives them from `parent_id`, which is what the `.dis`'s derived `Branch` relation expresses. The spec's `+edge` element type was never used.
- **Viewport delivery:** only nodes that intersect the viewer's reported viewport are sent, plus exactly one hop of parents and children so branches running off-screen still have both ends; never partners of partners (`MindmapElementMapper.Visible`; Req 11.5).
- **Deltas:** an edit is an upsert of the node; a structural change re-sends the visible set after removing what went; a fold is a group (plus an upsert of every survivor, since the layout moved them) and an expand an ungroup; removals always precede additions (`MindmapSession`; backend-centralization R4.5).
- **Undo:** every document change is a command with an inverse on the project's shared history (`Commands/`); fold is not.

## 11. Known gaps in the standalone implementation

1. **Links are stored but not followed.** Only `MindmapLink.raw` is filled on the wire; `project_relative_path`, `target_entry_id` and `broken` never are, so following a link, showing a broken link and the target's entry id (Req 12.2, 12.5, 12.8, 12.12) are not met. `MindmapLinks.ResolveWithinProject` and `MindmapLinks.Rebase` are called only from tests, so links are not rewritten when the map moves (Req 12.14) or when a target is renamed through ADP (Req 12.9).
2. **Keys the requirements list but the canvas does not declare:** Ctrl+Up and Ctrl+Down to reorder among siblings, and arrow-key navigation between nodes (Req 8.1). Reordering has no gesture at all; a drag always appends as last child.
3. **No missing-link-target rule** in the validator, although the validation seam now carries the paths it needs (`MindmapValidator` remarks).
4. **Formatted text is lost on rename without a warning.** IntelliJ warns first (its FR-020).
5. **Not shown or edited, only preserved:** icons, clouds, arrow links, per-node styling (colours, fonts), attribute tables, tags.
6. **The shipped examples** contain no `FOLDED`, no notes and no `POSITION`, and a link only on the central topic (`mindmap/examples/readme.md`).
7. **Catalogue state** is still ⚗️ Prototype in `docs/tools.md` and Notion although all 24 tasks are done; task 24 moved it to ✅ and that move has not happened.

## 12. Other hosts

The IntelliJ host (`etalii.adp.ide.intellij`, spec `001-freemind-mindmap-designer`, module `freemind/`) reads and writes the same `.mm` files, with these differences a shared runtime would have to choose between:

- a single `.mm` file, no registration file;
- targets FreeMind 0.7.1-1.0.1, and maps it saves must open in FreeMind 1.0.1 (FR-012); Freeplane-only content is preserved but not shown;
- **folding is a persisted, undoable edit** (FR-023), where standalone keeps it as per-viewer view state;
- new nodes get `CREATED`/`MODIFIED` timestamps and edits update `MODIFIED` (FR-011);
- shows arrow links, icons, colours, fonts and notes, with notes readable on hover (FR-015, FR-018); deleting a node removes its arrow links (FR-021);
- warns before replacing formatted text (FR-020); supports multi-select (FR-017);
- keys: Insert and Tab add a child, Enter a sibling, F2 renames, Delete deletes, Ctrl+Up/Down reorder, Ctrl+Left/Right move, Space folds (`freemind/src/main/resources/META-INF/adp-freemind-editing.xml`);
- a 1,000-node map opens in under 2 s and an edit shows in under 0.1 s (SC-003).

## 13. What DISL still lacks for this type

The 0.1 list had seven items. DISL 0.2 and 0.3 answered four, and the `.dis` now uses them: FBL persistence for a format the type adopts (1), viewer state (2), anchors on some sides (6) and a deletion confirmation with a count and a threshold (7). What is left, in the order of how much of this file each would absorb:

1. **A tree layout with sides**, or a way to describe a two-sided `mrtree`; `radial` and `mrtree` both differ from the conventional mind-map arrangement.
2. **Containment drawn as edges**: DISL containment nests children visually inside their parent. The `.dis` gets the mind-map picture from a derived relation plus a container that delegates to the layout, which works but says it sideways.
3. **Positional `create`** ("insert after this sibling") and **drop-target semantics for tools** ("run on the node under the drop").
4. **Several shortcuts for one entry** (Insert and Tab).
