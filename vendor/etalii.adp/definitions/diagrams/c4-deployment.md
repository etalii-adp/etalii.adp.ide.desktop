# C4 deployment diagram

The DISL specification of this diagram type is [c4-deployment.dis](c4-deployment.dis), written at DISL 0.5. This page holds what DISL 0.5 and FBL 0.7 cannot express about the tool, each gap with the construct it concerns, the document that shows it and what would resolve it. Everything either language can say is said in the `.dis` and is not repeated here.

The tool is the C4 deployment diagram of the standalone IDE (origin `c4/deployment`), one of six C4 tool types that share one model of a Structurizr DSL workspace. The other five are [c4-context.dis](c4-context.dis), [c4-container.dis](c4-container.dis), [c4-component.dis](c4-component.dis), [c4-system-landscape.dis](c4-system-landscape.dis), [c4-dynamic.dis](c4-dynamic.dis); the six definitions differ only in their viewpoint, toolbox, labels and origin, so their companions share their gaps. Every sentence, label, code and id in the `.dis` is the standalone tool's, as its parity transcript records them: `Parity/c4.transcript.json` under [`src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests) in etalii.adp.ide.standalone. The documents named below are that test project's fixtures; `findings/` is its `Fixtures/findings/` folder. The conversion is specified by [`specs/068-standalone-c4-disl-fbl/`](../../specs/068-standalone-c4-disl-fbl/), first written with spec-workflow in that repository, whose findings (A1 to A16, B1 to B24) and questions (Q1 to Q9) this page cites by number.

## What the diagram is for

A system's containers mapped onto the infrastructure they run on. A deployment diagram shows deployment nodes, which nest, the container and software system instances running on them, and supporting infrastructure nodes, in one deployment environment. A deployment view shows deployment nodes, infrastructure nodes and instances. A relationship between two containers it does not show is drawn between every two of their instances, once per pair; a hosting node is drawn as a boundary around what it hosts.

The definition raises `language.limits.celCost` to 10 000 000 from DISL's default of 1 000 000 (ruled in chat on 2026-10-19), so that a view with many lifted relationships keeps its lines: merging lifted lines into one line per pair costs more than linearly, 400 relationships taking about 0.9 million steps and 800 about 3.4 million, so the default would drop every lifted line of a view with about 400 or more.

## The gaps

Sixteen, each a candidate change for this repository: the twelve the conversion's requirements list (C1 to C12, in their order), three found while writing the definitions and one found while binding the format. None is worked around in the `.dis` by stating something the tool does not do. FBL 0.11 (etalii.adp's feature `066-fbl-conversion-gaps`) answers gaps 1 to 8, and FBL 0.13, of the same feature, gap 16, each below with the section that answers it and the fixture under `specifications/fbl/fixtures/` that shows it: the format is bound in the `grammar` family by [`structurizr.fbl`](../../specifications/fbl/structurizr.fbl), whose grammar is [`structurizr.lark`](../../specifications/fbl/structurizr.lark), written from the conversion's appendix draft in FBL 0.11's constructs. DISL 0.7 (etalii.adp's feature `067-desktop-fbl-bindings`) answers gap 13. Gap 12 stays unsupported on purpose; gaps 9 to 11, 14 and 15 are DISL's and stay open.

| # | Language | Construct | Finding | Status |
| --- | --- | --- | --- | --- |
| 1 | FBL | The statements of the selected view | B13 | Answered by FBL 0.11 (`structurizr-view-statements`, `structurizr-view-without-key`) |
| 2 | FBL | A comment after a brace or a statement | B3, B4 | Answered by FBL 0.11 (`structurizr-comments`, `structurizr-trailing-comments`, `structurizr-rename`) |
| 3 | FBL | A comment that spans lines | B5 | Answered by FBL 0.11 (`structurizr-block-comments`, `structurizr-recover-edges`) |
| 4 | FBL | An escape inside a quoted group | B7 | Answered by FBL 0.11 (`structurizr-escapes`); the Structurizr CLI reads escapes otherwise (below) |
| 5 | FBL | Positional optional arguments | B18 | Answered by FBL 0.11 (`structurizr-absent-arguments`) |
| 6 | FBL | A body whose braces do not balance | B22 | Answered by FBL 0.11 (`structurizr-recover-nested`, `structurizr-recover-edges`) |
| 7 | FBL | The legacy layout and the view header | B14, B16 | Answered by FBL 0.11 (`structurizr-legacy-layout`, `structurizr-view-without-key`) |
| 8 | FBL | Adding a block, and a value gathered from two places | B24, B9 | Answered by FBL 0.11 (`structurizr-add-view`, `structurizr-gathered-tags`) |
| 9 | DISL | A layout's direction and spacing from a diagram attribute | A12 | Open; DISL's, since the binding now reads a view's `autoLayout` into the diagram's attributes (gap 1), but a layout's direction and spacing are literals of the specification's layout, which only DISL can make bindable |
| 10 | DISL | Findings ordered by element | A11 | Open; DISL's, since the order of findings comes from how a specification is evaluated, not from how a body is read |
| 11 | DISL | The identifier of a new element made from its name | A9 | Open; DISL's, since a new element's identifier is made by the specification's id strategy (DISL 11.5.1), which no binding changes |
| 12 | Both | An id that depends on a directive of the body | B12 | Unsupported on purpose, as today |
| 13 | DISL | What a view exposes to a rule about the model | found while writing | Answered by DISL 0.7 (`findings/finding-order.dsl`, `findings/view-without-key.dsl`) |
| 14 | DISL | The text an unset end was written with | found while writing | Open; DISL's, since the binding reads an end as it is written, and no CEL function of DISL returns that text for an end that names nothing |
| 15 | DISL | The order of a view's members and of its legend entries | found while writing | Open; DISL's, since a view's members and its legend are ordered by DISL's evaluation of the model (3.5, 6.13), not by the reading |
| 16 | FBL | Several items written in one argument | found while binding the format | Answered by FBL 0.13 (`structurizr-split-tags`, `grammar-split-slot`) |

### 1. FBL 5.1 and 9.3: the statements of the selected view (B13)

- **Construct.** FBL names a view by its block (4.7) and leaves its content to the reading's viewpoint (9.3). Nothing reads the statements of the view a registration selects (`include`, `exclude`, `title`, `autoLayout` and its separations, the scope, a deployment view's environment, a dynamic view's interactions) into the diagram's attributes; a rule `within` a view block reads every view's.
- **Shown by.** `findings/view-statements.dsl`.
- **Answered.** FBL 0.11: a `block` rule names a view kind's views, their key and the origins whose readings show them (5.1), and the registration's view header selects one of the reading's views by its key, matched ignoring case, a view written without a key having the key its block rule gives (9.3); a rule with `selectedView` reads a statement of the selected view only (4.9, *Views*). The view's `include`, `exclude`, `title` and `autoLayout` statements, its scope, a deployment view's environment and a dynamic view's interactions are so read: the statements, the scope and the environment as the diagram's attributes, through the existing `typeMap` entry `ViewInclude` (`as: "diagram"`), and the interactions as the dynamic view's own, with no `typeMap` changed. `structurizr-view-statements` reads the view `titled` of `findings/view-statements.dsl`, and `structurizr-view-without-key` the component view of `findings/view-without-key.dsl` written without a key.
- **Meanwhile.** The module reads the body itself and builds the model this definition describes from its reading (`C4Disl` in the conversion's design, standing in for gaps 1, 2 and 6). The `typeMap` entry `ViewInclude` is the shape the binding will give it.

### 2. FBL 4.9: a comment after a brace or a statement (B3, B4)

- **Construct.** FBL 4.7 opens a block only when a statement's last non-whitespace character is the brace, so `model { // …` opens none and the braces no longer balance. FBL's `comment` is a whole line (4.6), so every rule's expression would have to admit a comment tail. Structurizr accepts both and the tool reads them.
- **Shown by.** `comments-everywhere.dsl` (one `unreadable` entry under the example binding of FBL 0.7), `findings/trailing-comments.dsl`.
- **Answered.** FBL 0.11, 4.9 (*The tree*): in the `grammar` family a brace is a token of its own and a comment is an ignored token, so `model { // …` opens the model's block, and a comment is owned by rule: one that starts its line leads the entry below it, one after a token trails the innermost entry holding that token, and every splice keeps a comment it does not remove (rules 1, 2 and 5). A `#` or `//` inside quotes is part of the string. `structurizr-comments` reads `comments-everywhere.dsl` and removes an element with its leading comments, `structurizr-trailing-comments` sets values beside trailing comments, and `structurizr-rename` renames an element and every relationship naming it with their comments kept.
- **Meanwhile.** As gap 1.

### 3. FBL 4.9: a comment that spans lines (B5)

- **Construct.** The tool skips `/* … */`, within a line and across lines. FBL has no comment that spans lines: the lines inside one are statements, a declaration inside one becomes an element, and a brace inside one counts toward the balance.
- **Shown by.** `findings/block-comments.dsl`.
- **Answered.** FBL 0.11, 4.9 (*The tree*, rule 3): a block comment is one ignored token, so nothing inside it is a statement, an element or a brace, and its owner's line span runs to the end of its last line; one that is never closed is trivia to the end of the body, reported as `fbl.unclosed-comment` under `recover.unclosed` (*Reading in part*). `structurizr-block-comments` reads `findings/block-comments.dsl` and removes the element a block comment leads, and `structurizr-recover-edges` shows a comment never closed.
- **Meanwhile.** As gap 1.

### 4. FBL 5.2 and 6.3: an escape inside a quoted group (B7)

- **Construct.** The tool reads `\"` inside a quoted argument as a quote and writes a quote back escaped. FBL has escaping rules for XML, YAML and JSON, and none for a quoted group of the `lines` or `blocks` family.
- **Shown by.** `findings/escaped-quote.dsl`; the transcript's renames to `Parity "name"`.
- **Answered.** FBL 0.11: a `{node}` slot's `escape`, `{"with": "\\", "quote": "\""}`, reads `\"` as a quote and `\\` as one backslash and leaves a backslash before any other character as it is (5.2, 4.9 *Escapes*), and a set writes every quote and backslash of the value escaped; a value that would not read back, such as one holding a line break, is refused and nothing is written (6.3). `structurizr-escapes` reads `findings/escaped-quote.dsl` and writes `Parity "name"`.
- **The Structurizr CLI reads escapes otherwise.** Its tokenizer (`Tokenizer.java` of `structurizr/java`) keeps the backslash: `\"` is read as the two characters `\"`, a quote after a backslash never ends the argument, and `\\` is read as two backslashes. The module reads `\"` as a quote, as FBL 0.11 does, and keeps `\\` as two characters, which FBL 0.11 reads as one; no document of the corpus holds `\\`, and no CLI run of `findings/escaped-quote.dsl` is recorded at `ad1a4680a`. Whether `escape` changes for the CLI's reading is the maintainer's to settle; the construct is unchanged until then.
- **Meanwhile.** The module's commands write every edit.

### 5. FBL 4.9: positional optional arguments (B18)

- **Construct.** Where an argument is absent the tool appends it and writes `""` for each absent argument before it, so a technology set on `container "Web"` gives `container "Web" "" "…"`. FBL's `insert-key` writes the one segment, which puts the technology in the description's place, and what `re-emit-line` does to a statement that opens a block is not stated.
- **Shown by.** `findings/dangling-both-ends.dsl`: the transcript sets a technology on `ghost -> phantom`, which has no label, and the line becomes `ghost -> phantom "" "Parity protocol"`. `findings/trailing-comments.dsl` shows the module defect B19 beside it: the technology set on `u -> s "Uses" // no technology yet` is appended after the comment, inside it.
- **Answered.** FBL 0.11, 4.9 (*Writing*, an absent part): an attribute's `create` with `after` and `pad` writes an absent argument right after the last present argument before it, so before a block and a trailing comment, with an empty value for each absent argument between them, in one `insert-key`; a set or a written part on a statement that opens a block changes neither the block's opening nor its bytes. That also repairs B19, since the technology is written before the comment. `structurizr-absent-arguments` shows `container "Web" // c` becoming `container "Web" "" "Java" // c`, and a part written on `softwareSystem "S" {`.
- **Meanwhile.** As gap 4.

### 6. FBL 4.9 and 7.4: a body whose braces do not balance (B22)

- **Construct.** The tool opens a document whose braces do not balance with what can be read; FBL 4.7 makes such a body unreadable, empty and read-only. A document being typed in the text editor is unbalanced between two keystrokes.
- **Shown by.** `findings/unbalanced.dsl`.
- **Answered.** FBL 0.11, 4.9 (*Reading in part*, rules 4 to 9): `recover.nested` names a block's opening and closing tokens, so that a statement that does not parse is skipped to the next one in its block, a block never closed is read to the end of the body and reported as `fbl.unclosed-block` at its line, a closing brace that closes nothing is skipped alone as `fbl.unmatched-close`, and an add into a block that is not closed is refused (7.4). `structurizr-recover-nested` reads `findings/unbalanced.dsl`, and `structurizr-recover-edges` the edges of a body.
- **Meanwhile.** As gap 1.

### 7. FBL 8.7 and 9.3: the legacy layout and the view header (B14, B16)

- **Construct.** The tool's positions live in `<base>.layout.json`, keyed by view and element, the element keys compared without case, each number as .NET writes a double. FBL 8.7 reads and writes such a file while it exists but states neither its number form, nor how its element keys compare, nor whether stale entries are pruned; and FBL 9.3 does not say what a registration whose `view:` names no view opens.
- **Shown by.** `findings/sidecar-keys.dsl` with its `.layout.json`, `findings/first-drag-undo.dsl`, `findings/view-without-key.dsl`.
- **Answered.** FBL 0.11: a legacy layout's view and element keys are matched ignoring the case of ASCII letters, a second key for one id is reported as `fbl.duplicate-key` and neither applied nor written, a number is written in its shortest form, and an entry of the view being written whose key names no element is stale, reported and removed in the same edit (8.7); a view header that names none of the reading's views opens the reading with no view's content, and a placement is refused (9.3). `structurizr-legacy-layout` shows the matching, the duplicate, the number form and the pruning on `findings/sidecar-keys.dsl` and its `.layout.json`, and `structurizr-view-without-key` a header naming no view on `findings/view-without-key.dsl`.
- **Meanwhile.** The module's `C4LayoutSidecar` stores positions as today, and a move on a view that names nothing is refused with "This diagram has no view to arrange."

### 8. FBL 5.1 and 5.2: adding a block, and a value gathered from two places (B24, B9)

- **Construct.** A block rule has no `insert`, so a view block cannot be added; and a `tags "A" "B"` line inside an element's block adds to the tags of its declaration, while FBL binds a value to one place (3.3).
- **Shown by.** `findings/tags-in-block.dsl` for the tags; for the view block, the transcript's `add view` steps, which every document of the corpus runs (`minimal.dsl` among them), each adding a view block and a registration and undoing both.
- **Answered.** FBL 0.11: a `block` rule's `insert` adds a view block at the end of the views from its `emit` and its undo removes it (5.1, 4.9 *Views*), the registration that opens the new view staying the host's; and a read-only attribute's `gather` reads its items from further nodes of the same entry, the declaration's tags and every `tags` statement of its block in document order (5.2). `structurizr-add-view` adds a container view, and `structurizr-gathered-tags` reads the tags of `findings/tags-in-block.dsl`'s shape and refuses a set of them.
- **Meanwhile.** `AddC4ViewCommand` stays code until the body is written by splices. Tags are read-only in the tool, so nothing writes them.

### 9. DISL 10: a layout's direction and spacing from a diagram attribute (A12)

- **Construct.** A view's `autoLayout` sets the layout's direction and its rank and node separation (top to bottom, 120 and 60 otherwise), and a view that declares one ignores every stored position. `LayoutConfig.direction` and `spacing` are literals, and nothing says that view data is ignored while a diagram attribute is set.
- **Shown by.** `findings/autolayout-ignores-positions.dsl` with its `.layout.json`.
- **Would resolve it.** `direction` and `spacing` as bindable values, and a layout setting that makes stored view data ignored while an expression holds.
- **Meanwhile.** The definition's `c4Ranks` states the defaults; the module's layout reads the view's statements.

### 10. DISL 8.6: findings ordered by element (A11)

- **Construct.** The tool reports the includes, then for each element its description, technology, connection and placement findings, then each relationship's, then each view's, then the coverage findings. DISL orders by rule (8.6), and `constraints.order` by code and then by line; neither states "element by element".
- **Shown by.** `findings/finding-order.dsl`.
- **Would resolve it.** An order key per finding, such as the position of its target in reading order, before the rule's position.
- **Meanwhile.** The module orders the definition's findings itself (`C4FindingOrder` in the conversion's design, Q4), finding a view's findings by their subject, the view's key.

### 11. DISL 11.5.1: the identifier of a new element made from its name (A9)

- **Construct.** A new element's identifier is the letters and digits of its name, the first one lowered, `_` before a leading digit and `element` when nothing is left, then `web`, `web2`, `web3` against a clash. `natural` composes the key attributes as written, so the name `Web App` gives `Web App`, which the pattern refuses; DISL 11.5.1 says its `natural` example gives `web`, which it does not.
- **Shown by.** The transcript's adds (`minimal.dsl`: `Web App` gives `webApp` and then `webApp2`, `42 Things` gives `_42Things`, `!!!` gives `element`), and its new documents (`42 things` gives `_42_things`).
- **Would resolve it.** A transform of the key attributes before the pattern is applied, or `natural` defined to drop what the pattern refuses.
- **Meanwhile.** The module makes the identifier (`C4Placement.IdentifierFor`) and hands it to the operation, and refuses one already in use with "This model already has something called '<identifier>'."

### 12. Both: an id that depends on a directive of the body (B12)

- **Construct.** `!identifiers hierarchical` makes a dotted reference such as `a.api` name an element by its path. Neither language states an id that depends on a directive of the body.
- **Shown by.** `findings/hierarchical-identifiers.dsl`.
- **Would resolve it.** An id rule chosen by a value the reader takes from the body.
- **Meanwhile.** Kept unsupported, as today: the directive is carried and not honoured, a dotted reference names nothing and is reported as dangling, and two elements of one name under different parents resolve to the first.

### 13. DISL 12.2: what a view exposes to a rule about the model

- **Construct.** A rule about every view read `diagram.views`, whose `View` had `id`, `viewpoint`, `name` and `members` only. The tool's view findings need more of each view: `c4.unknown-view-scope` names the scope the view is written with and is located at the view's line, and `c4.empty-view` is located at the view's line. A view's own statements are diagram attributes of the reading that shows it, and no rule about the model could read them for another view.
- **Shown by.** `findings/finding-order.dsl` (`c4.unknown-view-scope` at line 20, `c4.empty-view` at line 26), `findings/view-without-key.dsl`, the new `c4/container` and `c4/dynamic` documents.
- **Answered.** DISL 0.7 (8.2, 12.2): `View` has `scope`, the scope as written, also when it names nothing, and `location()`, where its declaration stands. `unknownViewScope` and `emptyView` state both findings as rules of the definition, at the view's line.
- **Meanwhile.** The module still reports both from its reading, the empty view at the line of the view its subject names, until the body is read through the binding.

### 14. DISL 11.2: the text an unset end was written with

- **Construct.** A relation whose mapped end names nothing is created with that end unset (11.2), and the identifier the body writes there is then nowhere in the model. The tool's sentence names it ("The relationship on line 11 names 'missing', which the model does not declare."), and the ids of a relationship and of an interaction are their ends as written. The same holds for a reference attribute: the scope a view names when it names nothing, which `View.scope` gives a rule (gap 13).
- **Shown by.** `findings/dangling-both-ends.dsl`, `findings/nested-relationships.dsl`, `findings/hierarchical-identifiers.dsl`.
- **Would resolve it.** The written text of an unset end or reference, readable in CEL, such as `self.written('source')`.
- **Meanwhile.** The plugin function `writtenEnd(relation, end)`, which the module supplies from its reading, as the timeline does (`timeline.md`, gap 4).

### 15. DISL 3.5 and 6.13: the order of a view's members and of its legend entries

- **Construct.** The tool draws, lays out and keys a view's members in the order of the statements that select them: what the view's `include` statements name first, in their order, then what `include *` adds, in model order, then, on a dynamic view, what its interactions name, in their order. A view's members are in model order (3.5), and `legend.computed.order` is evaluated on the first element of each group in model order, so neither a member nor an entry can take the position of the statement that selects it.
- **Shown by.** `findings/view-statements.dsl` (the view `named`, whose `include` names a container before a person) and the dynamic view `BatchRelease` of `crowded-component-view.dsl`.
- **Would resolve it.** An order of a viewpoint's members by an expression, such as the position of the first statement that selects each; and a legend `order` that is the least of an expression over the group's elements.
- **Meanwhile.** The module orders the members it reads from the model by the view's statements, and keys the legend by them, each entry painted from its first member (`C4Definitions.Drawing` in the conversion, task 10).

### 16. FBL 5.2: several items written in one argument

- **Construct.** The tool splits a tags argument at its commas, so `tags "Extra, More"` gives the two tags `Extra` and `More`, as Structurizr does. A `{node}` slot of a `many` attribute reads each node it reaches as one item (5.2), and a quoted argument is one token of the grammar, so the binding reads `Extra, More` as one tag.
- **Shown by.** `findings/tags-in-block.dsl` (`tags "Extra, More"`), and the argument `"Last, Least"` of `structurizr-gathered-tags`, which FBL 0.11 read as one item.
- **Answered.** FBL 0.13: a `{node}` slot of a `many` attribute takes `split`, and its value is cut at every occurrence of that text into items, each without the white space around it, an empty piece being no item (4.9, 5.2); a set writes the difference into the argument by splices, a changed item replaced alone, a new one written after the last with the slot's `separator`, a removed one taken with one separator, and every other byte of the argument kept (6.3). `structurizr.fbl` splits every tags argument at its commas, the declaration's and those of its block's `tags` statements, so `structurizr-split-tags` reads `findings/tags-in-block.dsl` with `Extra` and `More` as two tags and `Existing System` as one, and `structurizr-gathered-tags` reads `"Last, Least"` as `Last` and `Least`; `grammar-split-slot` shows the writing, on a binding whose tags are writable.
- **Meanwhile.** Tags are read-only in the tool, so nothing writes them, and the module's parser splits them (`C4Parser`, read by `C4Disl`) until the body is read through the binding.

## Stand-ins

Code that stands in for a gap while it stands, named so that nobody reads it as the design:

| Stand-in | For | Where |
| --- | --- | --- |
| `C4Disl` | Gaps 1, 2 and 6 (Q5), and gap 3 for a body with a block comment: the module builds this definition's model from its own reading of the body | The module, until the body is read through the binding |
| `C4FindingOrder` | Gap 10: today's order of findings (Q4) | The module, ordering what `C4Definitions.Problems` evaluates |
| `c4.unknown-view-scope` and the empty view's line, from the reading | Gap 13: what a view exposes to a rule about the model. It can go with DISL 0.7, once the body is read through the binding | The module (`C4Definitions.Problems`) |
| The statement order of `C4Definitions.Drawing` | Gap 15: a view's members and legend entries in the order of its statements | The module |
| `C4Placement.IdentifierFor` | Gap 11: a new element's identifier | The module, handed to the operation |
| `AddC4ViewCommand` and its inverse `RemoveC4ViewCommand` | Gap 8: adding a view block, and the registration that opens it. It can go with etalii.adp's spec 066, FR-012 (a block rule's `insert`), once the body is written by splices; the registration stays the host's | The module |
| The tags read by `C4Disl` | Gaps 8 and 16: the tags of a declaration and of its block's `tags` statements. It can go with spec 066, FR-013 (`gather`) and FR-038 (`split`, FBL 0.13), once the body is read through the binding | The module |
| `C4LayoutSidecar` | Gap 7: positions in the legacy sidecar | The module |
| `net.etalii.adp.c4.writtenEnds` | Gap 14: `writtenEnd`, supplied from the module's reading | A plugin of the definition, with a fallback |

## What stays code

Every class of the standalone module's backend project, [`src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4), beside the gap or the host concern that keeps it, as the conversion's first stage leaves it: the toolbox, the menus, the rows, the findings and what a view draws are derived from these six definitions, and the body is still read by the module's parser and written by its commands. A class marked **waits** is kept for the second stage, which reads and writes the body through the binding once the FBL gaps it needs are answered, and names the task that removes or changes it. The hand-written providers, the rule set and the old drawing code are no longer here: they are the oracle in the test project's `Parity/`.

| Class | Kept by |
| --- | --- |
| `C4Definitions` | Host: the six definitions loaded once as a `BundledDefinitionSet`, found by origin or by view kind, and the toolbox, menus, rows, findings and drawing derived from the one a view is drawn by, mapped to the `x-c4` wire ids. Gap 13 (the reading's `c4.unknown-view-scope` and the empty view's line) and gap 15 (members and legend entries in statement order) are stand-ins inside it. |
| `C4Drawing`, `C4DrawnLine`, `C4LegendItem` | Host: what one view draws, as the derivation hands it to the mapper and the layout. |
| `C4Disl` | Gaps 1, 2 and 6 (Q5), and gap 3: the definition's model built from the parser's reading. **Waits**: deleted when the body is read through the binding (task 14). |
| `C4FindingOrder` | Gap 10 (Q4): today's order of findings. |
| `C4Rules` | Host: the tool's finding codes as constants, which `C4FindingOrder` orders by and `C4StructurizrMirror` maps. The codes themselves are the definitions'. |
| `C4Validator` | Host: the definitions' findings as the host's problems, one validator per origin, as a call to `C4Definitions.Problems`. Whether it stays as that call or the host registers `C4Definitions` itself is settled when the conversion closes (task 17). |
| `C4ToolboxProvider`, `C4ContextActionProvider`, `C4ContextPropertyProvider` | Host: handing out the derived toolbox, menus and rows, and building the module's command for a chosen entry, an edited row or a toolbox drop on the empty canvas. They hold no palette, menu or row of their own. |
| `C4ContextSourceResolver` | Host: resolving a selection to an element or a relationship of the document. |
| `C4ElementMapper` | Host: the `.proto` payloads, the viewport filter, and the synthetic ids `c4:view` and `boundary:<id>` (A16). |
| `C4Layout`, `C4LayoutEngine`, `C4Boundary`, `C4Metrics` | Host: the `c4Ranks` layout, the boundaries and the moving of outsiders out of them, and the estimated box measurement (A16). A view's `autoLayout` reaching the layout is gap 9. |
| `C4Theme` | Host, where the notation does not replace it: the style of each card in its payload and the swatch of each legend entry, painted from the theme on the backend because a theme token is not resolved there. The canvas draws a person as the built-in `person` and a cylinder as a custom shape over its `styled-box`, coloured by these styles. |
| `C4Session`, `C4SessionFactory`, `C4DocumentStore`, `IC4DocumentStore` | Host: the shared document lifecycle, its reload, and one session per open reading, so that every reading of one `.dsl` shares one document and one history. |
| `ServiceCollectionAddC4Extension`, `Diagram` | Host: the seven registrations and their services; the refusing factory of `c4/code`, which keeps its registration and has no definition; and the empty document of each of the six types (`EmptyDocument`). **Waits**: the empty document comes from the binding's template when edits are written by splices (task 15). |
| `C4StructurizrMirror` | Host: the table of which rule mirrors one of Structurizr's, which the reconciliation and interop tests read. |
| `C4Parser`, `C4Workspace`, `C4ParseState`, `C4ParseScope`, `C4Tokens` | Gaps 1 to 4 and 6: the reading the definition's model is built from. **Waits**: out of the product code when the body is read through the binding (task 14). |
| `C4Element`, `C4ElementKind`, `C4Relationship`, `C4View`, `C4ViewKind`, `C4Interaction`, `C4AutoLayout`, `C4ElementStyle` | The parser's reading, and the view kinds the six types bind. **Waits**: with the parser (task 14); `C4ViewKind` stays as long as the types and the view command name a kind. |
| `C4Line`, `C4Lines` | Gaps 4 and 5: the lines the commands edit. **Waits**: out of the product code where nothing but `AddC4ViewCommand` needs them (task 15). |
| `SetElementNameCommand`, `SetElementDescriptionCommand`, `SetElementTechnologyCommand`, `SetRelationshipDescriptionCommand`, `SetRelationshipTechnologyCommand`, `C4EditHandler` and their handlers | Host: the command records the history stack and the wire carry. Gaps 4 and 5: their line surgery. **Waits**: written by splices (task 15), which repairs B19. |
| `AddC4ElementCommand`, `RemoveC4ElementCommand` and their handlers, `C4Placement` | Host: the command records. Gaps 4 and 5 and finding B20: where a new element goes and its text, and the placement refusals. **Waits**: `C4Placement` leaves the product code (task 15); `C4Placement.IdentifierFor` is gap 11 until then. |
| `AddC4ViewCommand`, `RemoveC4ViewCommand` and their handlers, `C4ViewBlock` | Gap 8: a view block and its registration added, and both undone. **Waits**: the view block is written by the binding's `insert` (FBL 0.11) when edits are written by splices (task 15); the registration it opens stays the host's. |
| `MoveC4ElementCommand`, `RestoreC4ElementPositionCommand` and their handlers, `C4LayoutSidecar`, `C4SidecarPosition` | Host: a move as one undo step. Gap 7: positions in the legacy sidecar. **Waits**: positions as FBL 8.7 says (task 16), which repairs B17; `C4LayoutSidecar` then leaves the product code. |
| `structurizr.fbl` | The module's binding, in the state FBL 0.2 can express, embedded and loaded by a test, on no reading path. **Waits**: replaced by etalii.adp's `structurizr.fbl` at FBL 0.11, in the `grammar` family with `structurizr.lark` (task 13), then read (task 14). |

Beside the backend, the client's `C4Canvas.tsx` compiles the canvas from the definition of the view's kind, and `c4Bindings.ts` states what the canvas library cannot read from it, each entry with its reason.

## Outside the definition, and not a gap

Behaviour the definition does not state because it is the host's, not because DISL or FBL lacks a construct:

- **The canvas.** How big a card is, the `c4Ranks` arrangement, the boundary drawn around a container or component view's scope and around a hosting deployment node, the moving of outsiders out of a boundary, the viewport filter and the `.proto` payloads (finding A16). The ids `c4:view` and `boundary:<id>` are the mapper's.
- **An element declared without an identifier** is named `<kind>_<name>_<n>` by the host for the session, n counting such elements in document order, and nothing is stored under that name (finding A8, Q7).
- **The host's own refusals**, which come from the wire rather than from an operation: "'<action id>' does not apply to this selection." for an action it does not answer, which is also what a commit of an answer to one of the four deployment adds receives; "'<property id>' is not a property of what is selected." for a write to a read-only row, such as Tags, Kind, Identifier, From or To; "'<id>' is not in this model any more." for a move of an id the model no longer has; "The element it would go inside is no longer in this model." when the parent of an add is gone by the time it is written; and the writer's "This document has no model block to add an element to." and "That element's block is not closed, so nothing can be added to it."
- **Prompts.** A rename or relabel prompt may be drawn in place of the element's name or the line's label. The prompt's validation refuses a blank name as it is typed; a commit of an empty rename is written.
- **Styles.** The `styles` block's headers are recorded and its properties never read, so a document's styles change nothing; the definition states the default theme (finding A15).
- **`c4/code`** keeps its registration and its refusing factory, and has no definition.

## Sources

- The standalone module: [`src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4), in particular `C4RuleSet.cs`, `C4ElementMapper.cs`, `C4ContextActionProvider.cs`, `C4ContextPropertyProvider.cs`, `C4ToolboxProvider.cs`, `C4Theme.cs` and `Commands/C4Placement.cs`.
- Its parity transcript and fixtures: [`src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests).
- The conversion's requirements and design: [`specs/068-standalone-c4-disl-fbl/`](../../specs/068-standalone-c4-disl-fbl/).
- [DISL](../../specifications/disl/DISL-specification.md), sections 3.5, 4.11.3, 6.13, 8.2, 8.6, 8.7, 10, 11.2, 11.5 and 12; [FBL](../../specifications/fbl/FBL-specification.md), sections 3, 4.6, 4.7, 4.9, 5.1, 5.2, 5.4, 6.3, 7.4, 7.5, 8.7 and 9.3.
- [The C4 model](https://c4model.com) and the [Structurizr DSL](https://docs.structurizr.com/dsl).
