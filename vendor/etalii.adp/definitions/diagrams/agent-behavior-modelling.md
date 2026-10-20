# Agent Behavior Modelling

The companion to [agent-behavior-modelling.dis](agent-behavior-modelling.dis). The `.dis` specifies the diagram type in DISL 0.3; this page explains it, holds what DISL 0.3 cannot yet state, and ties each point to the code that shows it.

The diagram type is implemented in the standalone IDE as the module `src/diagrams/agent-behavior-modelling/` of [etalii-adp/etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone), origin `etalii/agent-behavior-modelling`, read here at commit `466cc56b` (branch `research/hype-cycle-fbl-qa6b70`). It is also implemented in the Visual Studio Code plug-in, [etalii-adp/etalii.adp.ide.vscode](https://github.com/etalii-adp/etalii.adp.ide.vscode), against this definition; where that host differs is recorded in its `docs/parity.md`. Paths below without a repository are relative to that module folder, and `backend/…/` stands for `backend/EtAlii.Adp.Diagram.AgentBehaviorModelling/`; `standalone:` marks a path relative to the standalone repository root. The research it rests on is [docs/research/behavior-trees-for-agents.md](../../docs/research/behavior-trees-for-agents.md) in this repository.

## What the diagram is for

A chat agent's instructions are usually prose. Agent Behavior Modelling writes them as a behavior tree, the structure game developers use for character AI, tuned to agent engineering: composites say whether steps run in order, as alternatives or together; wrappers say when a step is retried, repeated, guarded or held for the user's approval; leaves are the checks, the work, the questions to the user and the hand-overs to sub-agents.

- **The Markdown file is the instructions.** The tree is stored in the instruction file the agent reads (a system prompt, an `AGENTS.md`, a `CLAUDE.md`, a skill). The agent reads the tree as instructions; the diagram reads the same lines as a tree.
- **The `.adp` file is only the visualization.** It registers the Markdown file as a diagram and keeps the heights the author dragged rows to. Deleting it loses a layout, never a behavior.
- The notation is ADP's own, so the origin is `etalii/`, declared as `language.origin` (`backend/…/Diagram.cs:28`; `standalone:docs/tools.md`, row for `etalii/agent-behavior-modelling`).
- State: ⚗️ Prototype, recorded under `x-adp.state`. No browser pass has been recorded yet.

## Mapping from the `.dis` to the implementation

| `.dis` type | Keyword in the Markdown | Wire kind | Menu and toolbox name | Shape |
| --- | --- | --- | --- | --- |
| `Sequence` | Do in order | `sequence` | Do in order | superellipse |
| `Fallback` | Try in order | `fallback` | Try in order | superellipse |
| `Parallel` | Do together | `parallel` | Do together | superellipse |
| `Retry` | Retry up to N times | `retry` | Retry | hexagon |
| `RepeatUntil` | Repeat until | `repeat` | Repeat until | hexagon |
| `Guard` | Only while | `guard` | Only while | hexagon |
| `Approval` | Ask approval before | `approval` | Ask approval before | hexagon |
| `Check` | Check | `check` | Check | pill |
| `Do` | Do | `action` | Do | box |
| `AskUser` | Ask the user | `ask` | Ask the user | parallelogram |
| `Delegate` | Delegate | `delegate` | Delegate | diode |
| `Child` | (nesting) | `child` | — | orthogonal line with an arrow |

The kinds are declared once in `backend/…/_Model/AbmNodeKinds.cs`, and the client states the same ids once in `client/abmIds.ts`. The abstract types `BehaviorNode`, `Composite`, `Decorator` and `Leaf` exist only in the `.dis`, to share attributes and to state each family's number of children once; the implementation has the family as a field of the kind (`AbmNodeCategory`). Each concrete type carries its keyword and wire kind in its own `x-abm` key.

**Wire ids.** DISL derives a tool's or an operation's id from its name in the `.dis`, and a tool id cannot contain a dot, so today's ids are kept through the top-level `x-abm` mapping rather than by renaming anything:

- An element's wire type is `etalii/agent-behavior-modelling+` followed by its kind (`x-abm.typePrefix`, `x-abm.types`; `backend/…/AbmElementMapper.cs`). A Child line's id is `child:` and the child's place, which the derived relation's `id` states.
- Each toolbox tool's id in the `.dis` is the lower-case kind; on the wire it is `abm.toolbox.<kind>`, dropping as `abm.add.<kind>` (`x-abm.tools`; `backend/…/AbmToolboxProvider.cs:28-29`).
- The context actions are `abm.rename` (editLabel), `abm.remove` (delete), `abm.move-earlier` (moveUp), `abm.move-later` (moveDown), `abm.edit-notes` (editNotes), `abm.add.<kind>` (addChild and addHere), `abm.connect.child` (moveUnder) and `abm.arrange` (arrange and arrangeFromNode) (`x-abm.actions`; `backend/…/AbmContextActionProvider.cs:36-56`).
- The property rows are `abm.kind`, `abm.label`, `abm.attempts`, `abm.notes` and `abm.place` (`x-abm.properties`; `backend/…/AbmContextPropertyProvider.cs:26-38`).

The `.dis` uses DISL 0.2's functions for the values more than one place needs: `keyword(n)` (the keyword a node is drawn with, a Retry's count included, with the singular `time` for one attempt, `AbmNodeKinds.KeywordOf`), `kinds()` (the eleven in toolbox order), `kindChoice(t)` (the menu name, `Retry` for a Retry), `familyOf(t)`, `startingLabel(t)`, `takesAnotherChild(n)`, `canBecome(t, n)` and `shorten(s)`.

## The file format (Markdown)

The `.dis` declares `persistence.format: "fbl"` with the binding `agent-behavior-modelling.fbl#abm`. That binding file is not written yet. Its reader is an FBL §11 persistence plugin, `net.etalii.adp.etalii.abmMarkdown`, which wraps today's `AbmParser` and `AbmWriter`: FBL's declarative readers cannot find a tree inside someone else's prose, so the plugin reads it and plans each edit as line splices, and FBL keeps everything else (the registration, the undo, the byte-preserving write). Because the format is `fbl`, the `.dis` gives none of the keys DISL 0.2 §11.2 forbids beside it (`files`, `encoding`, `newline` and the rest); it keeps only `binding`, `ids` and `view`.

**The plugin declares the changes it plans.** FBL 0.1 §11.2 gave a plan operation add, set and remove changes, with an add carrying no position, a move carrying neither a new parent nor an index, and no change of type. Today's tool needs all three: a node is added at a place among its siblings, re-parented to the end of another's children or reordered, and retyped by rewriting its keyword. The `.dis` declares them with DISL 0.3's `plans` (`add`, `set`, `remove`, `move` and `retype`), `add.position` and `move.position` on the plugin, which FBL 0.2 §11.2 hands a plugin as its own changes. That a retype keeps the node's label and notes is still `x-abm-plan.retype.keeps` (item 3 below).

The format itself, from `backend/…/AbmParser.cs` and `AbmWriter.cs`:

- **Where the tree is.** The first bullet list under the first heading whose text is `Behavior` or `Behaviour`, at any heading level, case-insensitive. The section ends at the next heading of the same level or higher (`AbmParser.cs:38-64`). Fenced code blocks are skipped, so an example tree quoted in a code block is never read as the tree (`AbmParser.cs:272-298`).
- **A node** is one list item, written `- **<Keyword>:** <label>`. The markers `-`, `*` and `+` are all read; new items are written with the marker the list already uses.
- **A thematic break is not a node.** A line of three or more marks of one kind, optionally separated by spaces (`---`, `- - -`, `* * *`, `___`), is a CommonMark thematic break and ends nothing and starts nothing; it is never read as an item (the item pattern, `AbmParser.cs:313`).
- **Children** are the items indented under an item, in order. A tab counts as four columns (`AbmParser.cs:66-105`).
- **Notes** are the lines indented under an item that are not list items themselves, blank lines between them included. They are kept as written and are the node's `notes` attribute.
- **Retry's count** is part of the keyword: `Retry up to 3 times`, or `Retry up to 1 time` (`_Model/AbmNodeKinds.cs:126-131`). A new Retry, and a node changed into one, gets three (`AbmNodeKinds.cs:79`; `Commands/AbmCommands.cs:104`).
- **An item without a keyword**, or with bold text that is none of the eleven (a Retry with no number among them), is read as a Do whose label is the whole item. The reader sets its `implicit` attribute, so it is drawn dashed and reported (`abm.no-keyword`), and a hand-written list opens rather than refusing (`AbmParser.cs:184-187`).
- **Ids are places in the tree**: `1`, `1.2`, `1.2.1`. Nothing is written into the Markdown to name a node, because anything written there the agent would read too. DISL 0.2 states this exactly with `ids.strategy: "derived"`, an expression that reads the parent's id and `positionIn` (`AbmParser.cs:142-163`). A place survives a rename, so selection and a stored position survive it too; a drag that reorders siblings renames the stored positions with them, so they follow the node; a reorder by Alt+Up or Alt+Down, or a re-parent, does not, and a stored height then follows the place, which is a known limit.
- **Round trip.** The parser never throws, and every edit is a line splice: the rest of the file, its line endings and its prose are untouched, and an unchanged file is written back byte for byte. A new list item uses the file's existing line ending.
- **Adding to a file without a tree.** The first node added to a Markdown file with no Behavior heading appends a `## Behavior` section at the end of the file (`AbmWriter.cs:279-290`).
- **A new diagram** (`AbmDocumentFactory.cs`) is written with CRLF: a title, one line saying what the file is, a `## How to follow the behavior` section that tells the agent what each keyword means, and a `## Behavior` section holding `- **Do in order:** Handle the request`. The legend is what lets a model that has never heard of behavior trees follow one. In FBL this is the plugin's `template` operation.
- **Registration.** `.md` is a shared extension (`SharedExtension: true`, `Diagram.cs:34`), as `.yml` is for Azure DevOps pipelines: a repository is full of Markdown that is not a behavior model, so a `.md` file becomes one only when the user registers it through Add, which writes the `.adp`. Add suggests the type only for a file that has a Behavior heading (`SuggestsBody`).

## Layout

The `.dis` lays the tree out with DISL 0.3's standard algorithm `tidyTree` (§10.3), its options under the algorithm's `tidyTree` key.

- **Computed, top-down.** The root sits at the top; each node's children are laid out left to right in document order beneath it, each subtree packed as close as its outline allows (Reingold-Tilford), each parent centred over its first and last child, so no two subtrees overlap (`backend/…/AbmLayout.cs:99-139`: nodes 200 by 60, 28 between siblings, 56 between levels, lines 31-40). Several roots stand side by side, 56 apart; the leftmost node is at x = 0.
- **Across, the order; down, the row.** A node's x is always the computed one, because its place among its siblings is the order they run in. All children of one parent share one row: the `.adp` registration's `layout:` block keeps the top-left of every node a drag moved, keyed by its place (`1.2: 360 160`), and a row is drawn at the first height stored for any of its nodes, or hangs 56 below its parent when none is, and never closer to the parent than 16 (`AbmLayout.cs:54-83`, `MinimumGap` line 43). So everything beneath a row follows it. The `.dis` says `respect: pinned` and `persistence.view.store: ["pinned", "bounds"]`; that the pin moves a whole row is in `tidyTree.rows` and `tidyTree.drag`.
- **Viewport.** The whole tree is laid out and then culled to the viewport; a parent line is kept when either end is in view and brings both ends with it (`AbmElementMapper.cs`).

## Interaction

- **Adding** is a drop from the toolbox, one tool per kind, each with the code's label, icon and tooltip (the capitalised meaning and "Drop it below the node it belongs under.", `AbmToolboxProvider.cs:15-35`). The node goes under the nearest node above the drop point that can take another child, placed among its children by where it was dropped; into an empty tree the first node becomes the root; a drop with no such node above it is refused with "Drop the node below the node it belongs under: a Do in order, a Try in order, a Do together, or a Retry, Repeat until, Only while or Ask approval before that has no child yet." (`AbmContextActionProvider.cs:288-316`). The new node starts with its kind's starting label (`Commands/AbmCommands.cs:32-43`); the label is not opened for editing after the drop, so the tools say `after: "none"` (the canvas declares no edit-on-drop for any kind). The `.dis` uses DISL 0.2's `mode: "drop"` tools; the parent choice is DISL 0.3's `drop.place: "underNearestAbove"`, defined once in `behavior.placements`. The empty canvas's menu offers the same eleven as "Add <kind> here", placed the same way (`addHere`).
- **Adding a child** from a node's menu is "Add child: <kind>" for each kind, offered when the node can take another child; it adds the node as the last child (`AbmContextActionProvider.cs:114-117`). DISL 0.2's `forEach` over `kinds()` states the eleven entries once.
- **Re-parenting** is a right-button drag from the new parent's body to the node (`client/AbmCanvas.tsx:140`): the node moves, with everything under it, to the end of the new parent's children. The `.dis` maps the derived Child relation's `connect` edit to the operation `moveUnder`, which the menu always offers, as the module does, and whose `abort` actions are the writer's refusals ("A node cannot move beneath itself.", "\"Check\" holds no children.", "\"Only while\" holds exactly one child, and already has it.", "That node is already there.", `AbmWriter.cs:193-218`). That the gesture uses the right button, from the parent's body, is DISL 0.3's `connect.pointer` on the Child edge.
- **Dragging** a node carries everything beneath it, and its siblings follow it up and down. Dropped past a sibling's centre, it takes that sibling's place: while it moves, the siblings it passes step aside to show where it will land, and on release its lines move in the Markdown with everything under them (`Commands/ArrangeAbmNodeCommand.cs`). The order and the row's height are one undoable step. A model opened read-only, or without a registration, refuses the drag ("This behavior model is read-only.", "This behavior model was opened without a registration, so there is nowhere to keep a position.", `AbmSession.cs:97-102`).
- **Reordering** among siblings is also Move earlier (Alt+Up) and Move later (Alt+Down), roots included; the node's lines move with everything under them. The `.dis` uses DISL 0.2's `moveUp` and `moveDown` context tools, with `behavior.messages` giving today's reasons at the ends: "It is already the first of its siblings." and "It is already the last of its siblings." (`AbmContextActionProvider.cs:108-109`).
- **Rename** is F2 or a double-click, editing the label only. The keyword is the kind, changed in the property grid's Kind choice. The choice lists only the kinds that can hold the node's present children (`AbmContextPropertyProvider.cs:135-140`), and a kind typed past the list is refused ("\"Check\" holds no children, and this node has 2 children.", `AbmWriter.cs:57-76`). The `.dis` states the change of kind with `behavior.retype` and the field as DISL 0.3's form item of kind `type`, its choices from `options` and named by `optionLabel`. Attempts shows only for a Retry and refuses a value below 1 ("'0' is not a number of attempts; a Retry allows at least 1.", `AbmContextPropertyProvider.cs:115-118`); Notes edits the note lines; Place is read-only with "A node's place follows from where it sits in the tree." (line 79). All five are in one group, Node.
- **Edit notes…** opens a dialog titled Notes, prefilled, with a Save button (`AbmContextActionProvider.cs:208-209`); the `.dis` states it as the `notesDialog` form used as the `editNotes` operation's `paramsForm`.
- **Remove** (Delete) removes the node and everything under it, as one splice. When nodes go with it, it asks first, through DISL 0.2's deletion confirmation with `threshold: 1`: "Removing this node also removes the 1 node beneath it." or "Removing this node also removes the N nodes beneath it.", titled and confirmed Remove, as a danger (`AbmContextActionProvider.cs:217-233`).
- **Arrange diagram** (`Commands/ArrangeAbmCommand.cs`) is offered on empty canvas and on every node, with the icon `mdi-sitemap-outline` (`AbmContextActionProvider.cs:91-96`). The tidy tree is the arrangement: a behavior tree's layout is computed from the tree, and a drag is only an override kept in the registration, so arranging means dropping the overrides. It removes the registration's `layout:` block and never touches the Markdown; one undo puts the registration back byte for byte. It is refused with "This behavior model was opened without a registration, so it has no dragged positions to forget." when there is no registration and with "This behavior model is already arranged." when the block is empty (`ArrangeAbmCommand.cs:38,47`), and is disabled with "There is nothing to arrange until this behavior model has a node." for an empty tree. The `.dis` declares two operations carried out by the plugin `net.etalii.adp.etalii.abmArrange`: `arrange` for the diagram and `arrangeFromNode` for a node, because an operation's `for` cannot name both.
- **Undo** restores the whole document text (`RestoreDocumentCommand<IAbmDocumentStore>`), so every edit above is one undoable step; a drag's undo (`RestoreAbmArrangementCommand`) puts back the Markdown and every stored position together.

## Rules

The rule ids, in `backend/…/AbmBreach.cs`, reach the Errors and Warnings panel through `AbmValidator`, which evaluates these constraints, each finding located by its line. Each is a declared rule in the `.dis` with its `code`, except `abm.no-behavior`, which the reader plugin reports (FBL §11.4), because no element of the model exists to carry it. DISL's built-ins `std.containment`, `std.multiplicity` and `std.facets` are switched off, so every breach is reported once, in today's words.

| Code | Severity | When | Located on |
| --- | --- | --- | --- |
| `abm.no-keyword` | warning | An item has no keyword and is read as a Do. | the item |
| `abm.several-roots` | warning | The list has more than one top-level item. | the second root |
| `abm.leaf-with-children` | error | A Check, Do, Ask the user or Delegate has children. | the leaf |
| `abm.decorator-children` | error | A wrapper has no child or more than one. | the wrapper |
| `abm.empty-composite` | warning | A composite has no children, the natural state while a tree is being written. | the composite |
| `abm.no-attempts` | error | A Retry allows no attempt. | the Retry |
| `abm.no-behavior` | information | The file has no Behavior heading, or the section holds no list yet. | line 1, or the heading |

## Notation

- **Two labels.** Each node shows its keyword in small capitals 20 below its top (10px, weight 600, letter spacing 0.02em) and its label 40 below it (12px), and is named for assistive technology as "<keyword>: <label>" (`client/AbmCanvas.tsx:72-89`, `client/abm.css:28-35`). The keyword is the function `keyword(self)`; only the label is edited in place.
- **Shapes.** Composites are DISL 0.2's built-in `superellipse` with exponent 4, the canvas library's own (`standalone:src/client/src/canvas/library/shapes/outline.ts`); wrappers the built-in hexagon, a Check a pill, a Do a box, an Ask the user the built-in parallelogram. A Delegate is a diode, which DISL has no built-in for, so the `.dis` gives it as a custom path drawn as the library draws it (`AbmCanvas.tsx:35-47`).
- **Colours** are CSS variables in `standalone:src/client/src/index.css` (`--color-diagram-abm-*`, lines 30-34 and 140-144) applied by `client/abm.css`, the functional decomposition graph's fills reused per family: composites green (`#aaed92`, dark `#2e7814`), wrappers yellow (`#fcf281`, `#736a03`), Check blue (`#9edcfa`, `#086fa1`), Do grey (`#ededed`, `#696969`), Ask the user and Delegate teal (`#86e6d9`, `#187569`).
- **An implicit Do** is drawn with a dashed outline (4 3, `client/abm.css:38-40`), a style condition on its `implicit` attribute.
- **Lines** leave the middle of the parent's bottom and arrive at the middle of the child's top (DISL 0.2's `anchors.sides`, `AbmCanvas.tsx:96`), routed orthogonally with an arrowhead at the child, in the muted text colour.

## What DISL 0.3 and FBL 0.2 cannot state yet

Two things are still stated through `x-` keys:

1. **Wire ids** — the top-level `x-abm` mapping and each concrete type's own `x-abm` key, described under *Wire ids* above. They stay the host's own and are not proposed for the specification (Peter, 2026-10-06, "Spec all but wire ids").
2. **What a retype keeps** — `x-abm-plan.retype.keeps` on the `abmMarkdown` plugin: a node retyped through the plugin keeps its label and its notes, because only its keyword is rewritten.

Two behaviours that needed an `x-` key under DISL 0.3 as first drafted became standard DISL 0.3 on 2026-10-06, by Peter's ruling, and the `.dis` uses the standard keys:

1. **Menu groups** — `group` on each context-menu entry (DISL 7.3; was `x-menu.group`): consecutive entries with one name form one group, so a node's edits, its eleven Add child entries and Arrange diagram are three groups, and the canvas's adds and Arrange two (`AbmContextActionProvider.cs:75-136`).
2. **Findings order** — `constraints.order` (DISL 8.6; was `constraints.x-order`): the item without a keyword first, then more than one root, then every other finding by its node's line, a node's structure finding before its Retry finding, as the module's hand-written rules listed them.

Five behaviours that needed an `x-` key under DISL 0.2 and FBL 0.1 are standard in DISL 0.3 and FBL 0.2 (approved 2026-10-05), and the `.dis` uses those keys: drop placement under the nearest node above (`behavior.placements`, `place` on each drop and on `addHere`'s create), the Kind field (a form item of kind `type`), the right-button connect (`connect.pointer`), the tidy tree with stored rows (`tidyTree`) and the plugin's planned changes (`plans`, `add`, `move`).

Two plugins carry what DISL leaves to code: `abmMarkdown` (persistence, required) and `abmArrange` (Arrange diagram). The `abmTreeLayout` plugin that stood in for the tidy tree is gone, now that `tidyTree` is a standard algorithm. The 0.1 definition's `abmReorder` plugin is gone: DISL 0.2's `moveUp` and `moveDown` state the reorder exactly.

## Where the `.dis` and the code may still differ

- **Edge cases of text.** `shorten` counts characters as CEL does (code points); the code counts UTF-16 units, so a label with characters outside the Basic Multilingual Plane may be cut at a different place.
- **Refusing a line from a leaf.** Today the client does not offer a line starting at a leaf; the `.dis` offers `moveUnder` and refuses it with the writer's sentence.
- **The label's editing box** (27 below the top, 22 high) and the removal dialog's icon are not stated.

## Known gaps

- **No browser pass recorded**, and no screenshot in `standalone:docs/screenshots/`.
- **Delegate does not open the linked file**; the link is text in the label.
- **The binding file** `agent-behavior-modelling.fbl` is named by the `.dis` but not written yet.

## The examples

`examples/` holds four agents: `pull-request-reviewer`, `customer-support`, `bug-fixer` and `research-assistant`. Together they use all eleven kinds, and `research-assistant.adp` has a `layout:` block that lowers one row, with everything beneath it, below its computed height. They were written for ADP because no published corpus of this notation can exist; their readme (`examples/readme.md`) says what they do not demonstrate.

## Sources

- Module code and tests: `src/diagrams/agent-behavior-modelling/` in etalii.adp.ide.standalone.
- Catalogue row: `docs/tools.md` in etalii.adp.ide.standalone.
- Research: `docs/research/behavior-trees-for-agents.md` in this repository.
- Notion: the "Tools" database row for Agent Behavior Modelling.
