# Functional decomposition graph: what DISL does not express

This document accompanies [functional-decomposition-graph.dis](functional-decomposition-graph.dis), the DISL 0.3 specification of the functional decomposition graph diagram. The specification holds everything DISL 0.3 can state: the metamodel with its fixed and clamped sizes, the superellipse, parallelogram, trapezoid and diode shapes, the toolbox of dropped element tools, the menus with their groups, the placement and connect menus, the forms with their row ids and the width's `parse`, the right-button body drag that connects, the constraints with the tool's own codes, sentences and order, the conditional removal confirmation, base-36 ids, and the FBL binding the document is read and written through. What DISL 0.3 cannot state, the wire ids, is declared in the specification under `x-fdg` and described here in [The wire ids](#the-wire-ids). The language gaps of DISL and of FBL are in [Language gaps](#language-gaps), and the code the standalone module keeps beside the specification, each class with its reason, is in [What stays code in the standalone module](#what-stays-code-in-the-standalone-module).

The tool it describes is the standalone module `src/diagrams/functional-decomposition-graph/` in [etalii-adp/etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/functional-decomposition-graph), origin `etalii/functional-decomposition-graph`, documents with the extension `.fdg`. Every label, sentence and code in the specification is copied from the module's parity transcript, `backend/EtAlii.Adp.Diagram.FunctionalDecompositionGraph.Tests/Parity/functional-decomposition-graph.transcript.json`, read at standalone commit `6d1e0a24`. The work that rewrote it is the standalone specification `.spec-workflow/specs/functional-decomposition-graph-disl-fbl/`: task 2 rewrote it, and task 12 brought this document up to date with the converted module, read at standalone commit `454fc509`. Unless a path says otherwise, it is relative to the module: `backend/` is `backend/EtAlii.Adp.Diagram.FunctionalDecompositionGraph/`, and `client/` is the module's client folder.

## Contents

1. [What the diagram is for](#what-the-diagram-is-for)
2. [The types](#the-types)
3. [The document](#the-document)
4. [Interaction](#interaction)
5. [Rules, findings and refusals](#rules-findings-and-refusals)
6. [Notation](#notation)
7. [The wire ids](#the-wire-ids)
8. [Language gaps](#language-gaps)
9. [What stays code in the standalone module](#what-stays-code-in-the-standalone-module)
10. [The example](#the-example)
11. [Rulings from the owner](#rulings-from-the-owner)
12. [Sources](#sources)

## What the diagram is for

A functional decomposition graph breaks a product down from what the user sees to what the system does: UI elements contain UI elements, actions and data; actions own data and functions; data owns data and functions; functions own functions. An action can also *show* a UI element, which is how navigation is drawn without making it ownership.

- The notation is ADP's own rather than a standards body's, which is why the origin is `etalii/` (standalone `docs/tools.md`, the row for `etalii/functional-decomposition-graph`).
- The Notion "Tools" database row: kind Diagram, family "Business process & workflow notation", focus areas "Software delivery" and "Humans and agents together", rarity "Unique to ADP", state Prototype.
- State: ⚗️ Prototype, recorded under `x-adp.state`, which is catalogue data rather than DISL (see [Rulings from the owner](#rulings-from-the-owner)).

## The types

| DISL name | Document type | Wire id of its tool | Kind |
| --- | --- | --- | --- |
| `UiElement` | `ui-element` | `fdg.toolbox.ui-element` | element |
| `Action` | `action` | `fdg.toolbox.action` | element |
| `DataElement` | `data-element` | `fdg.toolbox.data-element` | element |
| `Function` | `function` | `fdg.toolbox.function` | element |
| `Comment` | `comment` | `fdg.toolbox.comment` | element |
| `UiChild` | `ui-child` | `fdg.connect.ui-child` | relation |
| `OwnsAction` | `owns-action` | `fdg.connect.owns-action` | relation |
| `OwnsData` | `owns-data` | `fdg.connect.owns-data` | relation |
| `OwnsFunction` | `owns-function` | `fdg.connect.owns-function` | relation |
| `Shows` | `shows` | `fdg.connect.shows` | relation |

- The abstract types `FdgElement`, `NamedElement`, `FdgRelation` and `Ownership` exist only in the specification, to share attributes, menus and forms and to state acyclicity once. The module has no such types.
- Each concrete type carries its document spelling as the fixed attribute `documentType`, and the function `documentTypeOf` maps a DISL name to it inside sentences.
- **`UnknownElement` and `UnknownConnection`** are two concrete types for an entry whose `type:` the notation does not have. They have no tool, no notation and no `create`, and the binding's catch-all rules are mapped to them rather than `unreadable`, so they stay in the model: their ids are counted by `std.duplicateId`, a connection to one is reported by the type check as `fdg.forbidden-link`, and `uniqueName` counts an unknown element's name. `UnknownElement` extends `NamedElement` because the transcript shows the tool giving such an entry the named menu ("Rename…", "Remove") and the rows Name, Description and Width; an unknown connection gets the connection menu and rows. The two types stand in for FBL gap 2 of the standalone specification (a rule chosen by a value cannot write that value), which would otherwise need ten typed rules.

## The document

The document is read and written through the FBL binding named in `persistence.binding`, `backend/functional-decomposition-graph.fbl#fdg`, an FBL 0.8 binding the module owns and embeds. Every entry is an element of the binding, one rule per element type and per relation chosen by its `type:` with `when`, and each rule writes that value into a new entry as its second key with its insert's `constant` (FBL 0.8). A connection is an element with reference attributes, so one whose end names no element is kept and reported. Every value is read as the text it was written with (FBL 0.5), and every edit is a model change the FBL library plans as splices: no module code parses or writes a line of YAML. The hand-written `FdgParser` and `FdgWriter` the tool had before are kept only in the module's test project, as the oracle the conversion was checked against.

- **Shape.** A line-oriented YAML subset: a header line `functional-decomposition-graph: 1`, then an `elements:` list and a `connections:` list. It is ADP's own schema, not DID. The header is mapped to the element type `Graph` (`typeMap`, `as: "header"`).
- **Element keys:** `id`, `type`, `name`, `description`, `text`, `x`, `y`, `width`, `height`. **Connection keys:** `id`, `type`, `from`, `to`, `name`, `description`; `typeMap` maps `from` and `to` to `source` and `target`.
- **Position is the top-left corner** (`x`, `y`). The node notations place the node by its top-left (`placement`), so no conversion is stated.
- **Height is stored only for a Comment.** The four named types share one height of 48, stated as a `fixed` attribute on `NamedElement` with the row reason "Only a Comment has its own height; the others share one.". A fixed attribute is not written.
- **A Comment has `text`, never `name`.** A `name` written on a Comment is kept as the read-only attribute `name` (`typeMap` maps the binding's `strayName` to it), reported as `fdg.unreadable-entry`, and counted by `uniqueName`, as the transcript's `names-counted` case shows ("New UI element 3").
- **Ids.** New entries get a ShortGuid, a version 4 GUID written as 25 characters of base 36 (`ids.encoding: "base36"`). Hand-written ids such as `task-detail` are kept as written. Ids are unique across elements and connections together.
- **Round trip.** An unchanged document comes back byte-identical, and every edit is a splice that leaves the rest of the file, its line endings and its comments untouched. A changed value replaces only its own span, in its own style, so a trailing comment and the author's quotes survive; a new key goes where the binding's `keys` order puts it; a new entry is indented like its previous sibling. Undo restores the bytes through the host's shared restore command. Because the bytes are the test subject, standalone's `.gitattributes` marks `*.fdg -text`.
- **Removing an element keeps a comment line directly above it**, as the tool always has, where FBL's line span would take it too ([gap 5](#gap-5-a-removal-that-leaves-the-comment-above)).
- **A first entry opens an empty flow list**, `elements: []` of the template and `elements: [] # none yet` alike ([gap 4](#gap-4-opening-an-empty-flow-collection)).
- **A missing `connections:` section is not created**: a connect is refused with "The document has no `connections:` section to add to.", the planner's refusal worded as the tool's ([gap 6](#gap-6-the-wording-of-a-body-that-cannot-be-read-and-of-a-planners-refusal)).
- **A body that is not YAML opens with an empty model and every edit refused** (finding B8 of the standalone specification, ruled in chat 2026-10-21). The one finding is `std.unparseable` with the code `fdg.unreadable-entry`, a warning: "The document could not be read as YAML: " and the parser's own message. An edit is refused with "<file> could not be read, so it cannot be edited.", or "…so nothing can be edited until it can: <finding>". The refusals are host sentences, worded by the host rather than by the specification or the binding (FBL gap 6 of the standalone specification).
- **Registration.** A diagram is added to an ADP project by a `.adp` file whose origin line is `etalii/functional-decomposition-graph` and whose body is `body: <name>.fdg` (`examples/field-service/field-service.adp`).

## Interaction

- **Adding** is a drop from the toolbox (`mode: "drop"`), with the new element's centre where it is dropped: a named element is 160 by 48, a Comment 240 by 96 with the text "New comment". There is no "Links" group. A click on a tool answers "An element is added by dropping it where it belongs.".
- **Placement menu.** The canvas menu has five entries, "Add ui element here", "Add data element here", "Add action here", "Add function here" and "Add comment here", in the order the tool has always listed its element types, which is not the palette's.
- **New names are unique**: "New UI element", "New action", "New data element", "New function", then the same with " 2", " 3" and so on, counting every `name` in the graph (`uniqueName`, built from `positionIn` because DISL has no `lists.range`, finding A14).
- **Connecting** is a right-button drag from one element's body to another's (`connect.pointer`, `button: "secondary"`, `start: "body"`). The connect menu offers five entries labelled "ui child", "owns action", "owns data", "owns function" and "shows". Which relation one drag creates is [gap 7](#gap-7-which-relation-a-pointer-gesture-creates).
- **Rename** is F2 (`fdg.rename`); on a Comment the entry is "Edit text…", and on a connection "Rename…" (`fdg.rename-connection`).
- **Remove** is the Delete key: "Remove" for an element (`fdg.remove`), "Remove connection" for a connection (`fdg.disconnect`). Removing an element asks for confirmation only when it has connections, naming how many: "Removing this element also removes the N connection(s) to or from it." (`Confirmation` with `count` and `threshold`; a self-loop is counted once).
- **Menus** all sit in one group and show no standard entries; they are the same when the diagram is read-only.
- **Rows.** A named element has Name and Description in "Identity" and Width in "Size"; a Comment has Text, Description, Width and Height; a connection has Name (row id `connectionName`) and Description.
- **Width and Height show four decimals** ("140.1235"), as the file holds them (finding A6, ruled in chat 2026-10-21). A typed value that is not a positive number is refused with "'<value>' is not a size this graph can draw.".
- **Sizes below the minimum** (width 80, a Comment's height 48) are clamped, not refused, whether dragged or typed (`outOfRange` with `clamp` both ways). A stored size below the minimum is kept, drawn and reported.

## Rules, findings and refusals

- **Codes** (`constraints.order`): `fdg.duplicate-id`, `fdg.unreadable-entry`, `fdg.dangling-reference`, `fdg.self-link`, `fdg.forbidden-link`, `fdg.second-parent`, `fdg.second-shows`, `fdg.ownership-cycle`. `fdg.unreadable-entry` is a warning, because the lines are kept and the document still draws; every other code is an error.
- **Connect refusals** run in a fixed order: type, then cardinality, then cycle, each naming its check: "Refused by the type check: …", "Refused by the cardinality check: `X` already has its `owns-data` parent, and may have 1.", "Refused by the cycle check: `T` already owns `S`, so this link would make an ownership loop.". A duplicate id answers "That id is already used in this graph.".
- **The four ownership relations are acyclic together; Shows is not.** `acyclic` on the abstract `Ownership` covers the union of its subtypes (DISL 0.2, *Changes from 0.1*). An action may show the UI element that owns it.

## Notation

- **UI element**: the built-in `superellipse` with `exponent` 4. **Data**: `parallelogram`, `skew` 0.2, `direction` right. **Action**: `trapezoid`, `inset` 0.15, `direction` down, which differ from the built-in defaults and so are stated. **Function**: a custom diode shape whose corner radius is `min(h/2, w)`. **Comment**: a note.
- **Colours** are light and dark theme tokens with a `contrast` requirement of 4.5 against the text.
- **Connections** are cubic béziers with a filled triangle at the target and the name at the midpoint. Placement is free and manual; there is no snapping and no automatic layout.

## The wire ids

The standalone client (`client/fdgIds.ts`) and backend share wire ids that DISL has no place for. `x-fdg` maps them from the specification's names, as `x-ghg` does for the hype cycle:

- `tools`: each tool's id and the action its drop runs.
- `actions`: keyed by the entry's operation, else its kind; a key `<Type>/<kind>` overrides that for entries offered on `<Type>` or its subtypes (`FdgRelation/editLabel` is `fdg.rename-connection`, `FdgRelation/delete` is `fdg.disconnect`), and a `<name>` in an id is replaced by the entry's argument of that name, mapped through `types` (`fdg.connect.<relation>`).
- `properties`: keyed by the form item's id, else its attribute (`connectionName` is `fdg.connection-name`).
- `types`: DISL name to document type. An element's wire id is the id written in the document, with no prefix.

## Language gaps

These are the items of section C of the standalone specification's requirements, gap 11 that its design adds, gap 12 that the rewrite of this specification adds, and gaps 13 and 14 that writing through the binding showed. Each is recorded with the construct concerned, its evidence and what would resolve it, and each is a candidate change for the DISL or the FBL specification. Where the tool's behaviour is kept, it is kept by a stand-in the standalone module names, listed in [What stays code in the standalone module](#what-stays-code-in-the-standalone-module), and never by a silent workaround. No gap was closed by weakening a check, by skipping a fixture or by stating here something the tool does not do. Fixtures are in the module's `backend/EtAlii.Adp.Diagram.FunctionalDecompositionGraph.Tests/Fixtures/`, tests in that project, and the parity transcript holds what each fixture answers.

| Gap | Language | Construct | State | Evidence | Stand-in |
| --- | --- | --- | --- | --- | --- |
| [1](#gap-1-a-slot-that-reads-a-yaml-scalar-as-written) | FBL | A slot that reads a YAML scalar as written | Answered by FBL 0.5 | `scalars-as-written.fdg`; `FdgBody.Tests`, `AValue_ReadsAsWritten` | None |
| [2](#gap-2-a-rule-chosen-by-a-value) | FBL | A rule chosen by a value, writing that value | Writing answered by FBL 0.8; choosing remains | `unknown-types.fdg`, `malformed-entries.fdg`; the binding's fifteen rules | `UnknownElement` and `UnknownConnection` |
| [3](#gap-3-the-indentation-indicator-of-a-block-scalar) | FBL | The indentation indicator of a literal block scalar | Open | `comment-text-forms.fdg`; `FdgWrittenAsTyped.Tests`, `ABlockText_BeginningWithASpace_IsGap3_AndCannotBeRead` | None: the case is reported |
| [4](#gap-4-opening-an-empty-flow-collection) | FBL | Opening an empty flow collection | Open in the text; the runtime does it | `empty-list-comment.fdg`; `FdgFirstEntry.Tests` | None |
| [5](#gap-5-a-removal-that-leaves-the-comment-above) | FBL | A removal that leaves the comment above | Open | `comment-above-element.fdg`; `FdgDocument.Tests`, `RemovingTheFirstElement_KeepsTheCommentAboveIt` | `FdgCommentKeeper` |
| [6](#gap-6-the-wording-of-a-body-that-cannot-be-read-and-of-a-planners-refusal) | FBL | The wording of an unreadable body and of a planner's refusal | Open | `not-yaml.fdg`, `key-twice.fdg`, `no-connections-section.fdg`, `unread-keys.fdg`, `header-missing.fdg`, `header-version.fdg`; `FdgDocument.Tests`, `AConnectWithNoConnectionsSection_IsRefusedInTheToolsSentence` | `FdgReaderSentences` |
| [7](#gap-7-which-relation-a-pointer-gesture-creates) | DISL | Which relation a pointer gesture creates | Open | client `fdgConnect.test.tsx`; the transcript's connect cases | The library's `relationOnto` |
| [8](#gap-8-an-order-of-findings-that-is-not-by-code-and-then-by-line) | DISL | An order of findings not by code and line | Open | `finding-order.fdg`, `link-order.fdg`, `rule-second-parent.fdg` | `FdgFindingOrder` |
| [9](#gap-9-a-size-clamped-on-commit) | DISL | A size clamped on commit | Answered by DISL 0.2's `outOfRange` | `FdgDocument.Tests`, `AResizeBelowTheMinimum_IsClamped` | `FdgSizeClamp`, for the runtime |
| [10](#gap-10-a-number-written-in-quotes) | FBL | A number read from any scalar | Open | `number-forms.fdg` | `FdgReading`, `FdgReaderSentences` |
| [11](#gap-11-the-id-an-unresolved-end-was-written-with) | DISL | The id an unresolved end was written with | Open | `cardinality-unknown-end.fdg`, `link-chain.fdg` | `FdgLinkChain` |
| [12](#gap-12-the-detail-and-place-of-a-cycle-and-of-a-cardinality-finding) | DISL | The detail and place of a cycle and of a cardinality finding | Open | `ownership-loop-rotation.fdg`, `rule-second-parent.fdg` | None: the standalone runtime supplies them |
| [13](#gap-13-the-indentation-of-a-new-entry) | FBL | The indentation of a new entry | Settled by FBL's rule; the text leaves a part unsaid | `mixed-indentation.fdg` | None |
| [14](#gap-14-the-sign-of-a-number-that-rounds-to-zero) | FBL | The sign of a number that rounds to zero | Recorded | `number-rounding.fdg` | None |

### Gap 1: a slot that reads a YAML scalar as written

A name written `1.0`, `true` or `null` is that text to the tool (finding B1). FBL 0.2 typed a plain scalar by the YAML core schema, which is how the hype cycle came to read `1.0` back as `1`. **Answered by FBL 0.5** (sections 4.1.5 and 5.2): a string attribute, an id, a reference and a host attribute read the written text unasked, and the binding reads every value of a `.fdg` that way. Evidence: `scalars-as-written.fdg`, and `AValue_ReadsAsWritten`, which failed with seven cases before the attribute types were given to the reader.

### Gap 2: a rule chosen by a value

An entry's type is a value (`type: ui-element`), not the list it sits in (finding B3). FBL fixes a rule's `type`, so the binding has one rule per element type and per relation, told apart by `when`, a catch-all rule for each list behind them, and a not-a-mapping rule for each list: fifteen rules where the format has two lists. **Writing the value is answered by FBL 0.8**: an insert's `constant` writes `type:` as the second key of a new entry (etalii.adp spec 033, #420). **Choosing the rule is not**: an entry whose type the notation does not have is kept by the catch-all rules as the concrete types `UnknownElement` and `UnknownConnection`, which have no tool, no notation and no `create`, so that their ids are counted and their links checked. Evidence: `unknown-types.fdg` and `malformed-entries.fdg`, whose findings and refusals the transcript holds. Would resolve it: a rule whose `type` is read from a key through a map, which would make fifteen rules five and the two unknown types the map's default.

### Gap 3: the indentation indicator of a block scalar

A Comment's text written as a literal block whose first line begins with a space needs an indentation indicator (`|2-`), because YAML takes the indentation of a block from its first line (finding B13). FBL 6.3 says a multi-line string with style `literal` is written `|-` with its lines indented one step deeper than its key, and does not mention the indicator, so the library writes `|-`, the space reads as indentation, and the body no longer reads. That breaks FBL 0.5's own rule that a value written for a slot read as text **MUST** read back as the same text. Every other case of finding B13 is repaired: a one-line text stays plain, a quoted one stays quoted, and a text whose first line begins with a space keeps the space when it is written quoted. Evidence: `comment-text-forms.fdg`, and `ABlockText_BeginningWithASpace_IsGap3_AndCannotBeRead`, which holds what the library writes today and so changes when the language does. Would resolve it: an indentation indicator in 6.3 whenever a literal's first line begins with a space, and a statement of whether a one-line value that becomes several lines changes style. It is recorded and not worked around.

### Gap 4: opening an empty flow collection

The first element of a new document goes into `elements: []`, and of a document whose list reads `elements: [] # none yet` into that (finding B14). FBL 4.3 makes a flow collection writable only as a whole, by `replace-value`, and its splice catalogue has no operation that opens one into a block sequence; the standalone FBL library opens it all the same, keeping the trailing comment. The tool before the conversion opened `[]` and `[ ]` only, and put an item under a key that still held `[]` otherwise. Evidence: `empty-list-comment.fdg`, and `FdgFirstEntry.Tests`, whose trailing-comment cases failed against the tool before the conversion and whose template cases failed with the library's opening switched off. Would resolve it: an operation in the splice catalogue that opens an empty flow collection into a block one, keeping what follows it on its line.

### Gap 5: a removal that leaves the comment above

FBL removes an entry's line span, which reaches up over its leading comments, the comment lines directly above it at the same indentation (finding B15). In the example, `# The screens.` sits directly above the first element. Under Q2 of the standalone requirements the comment stays, as it always has. The stand-in `FdgCommentKeeper` plans each removal with the library and starts each removal splice past the comment lines its span begins with; nothing else of the plan changes. Evidence: `comment-above-element.fdg`, and `RemovingTheFirstElement_KeepsTheCommentAboveIt`, seen to fail with the keeper switched off. Would resolve it: a `remove` option that leaves the leading comments.

### Gap 6: the wording of a body that cannot be read and of a planner's refusal

A binding cannot word what its reader reports, and `typeMap` keeps host attributes out of CEL, so the tool's own sentences come from module code (findings B2, B6, B7, B8, B16): a key the reader does not read, a `name` on a Comment, an entry that is not a mapping, a number that does not read, the three header sentences, a body that is not YAML or repeats a key, and the planner's refusal of a connect with no `connections:` section, which reads "The document has no `connections:` section to add to." rather than the planner's "The file has no connections to add to.". The stand-in `FdgReaderSentences` words each from what the binding and the planner report and hands it to the evaluator on its line. Evidence: `not-yaml.fdg`, `key-twice.fdg`, `no-connections-section.fdg`, `unread-keys.fdg`, `header-missing.fdg` and `header-version.fdg`, and `AConnectWithNoConnectionsSection_IsRefusedInTheToolsSentence`, seen to fail with the mapping broken. Would resolve it: a binding's own sentences for its reader's findings and its planner's refusals, keyed by their reasons.

### Gap 7: which relation a pointer gesture creates

Five relation types claim the same right-button body drag. The tool creates the one relation whose ends admit both elements, and there is exactly one for every allowed pair because each relation is identified by its target type; the connect menu offers all five. DISL 0.3's `connect.pointer` says how a gesture starts, but not how a runtime chooses among relation types that claim the same gesture (finding A9). The standalone canvas library chooses by the pair of ends (`relationOnto`), for every module. Evidence: the client's `fdgConnect.test.tsx` and the transcript's connect cases. Would resolve it: a rule in 7.x or 9.x that a gesture claimed by several relation types creates the one whose endpoint constraints admit the pair, and offers a choice otherwise.

### Gap 8: an order of findings that is not by code and then by line

`constraints.order` orders findings by code, then by line. The tool's order differs in three places (finding A13, worded for all three by Amendment 1 of the standalone design):

1. **Within `fdg.unreadable-entry`**, the findings come in reading order: elements, then connections, then the header.
2. **Across the link codes**, one finding per connection in document order, mixing `fdg.dangling-reference`, `fdg.self-link` and `fdg.forbidden-link`, where code-then-line would sort three connections that fail as forbidden, dangling, forbidden into another order.
3. **For cardinality**, by relation in the order the notation lists the relations (`ui-child`, `owns-action`, `owns-data`, `owns-function`, `shows`), not by line.

The owner ruled in chat on 2026-10-11 that today's order stays. It is held by the stand-in `FdgFindingOrder`. Evidence: `finding-order.fdg` and `link-order.fdg`, and the derived-findings parity test, seen to fail with the stand-in switched off. Would resolve it: an `order` per code, or a `by` of `document`, `relation` or `line` on `constraints.order`.

### Gap 9: a size clamped on commit

A width below 80, or a Comment's height below 48, is clamped to the minimum when it is committed, not refused (finding A6). **Answered by DISL 0.2's `outOfRange`** (4.3) with `drag` and `typed` both `clamp`, which this specification states. The standalone runtime does not yet implement `outOfRange.typed`, so the stand-in `FdgSizeClamp` raises a committed size to the `min` the definition declares; it stands in for the runtime, not for the language. Evidence: `AResizeBelowTheMinimum_IsClamped`, seen to fail with the clamp switched off. Would resolve it: `outOfRange.typed` in the standalone runtime's form commit.

### Gap 10: a number written in quotes

The tool reads a position or a size from any scalar, as .NET reads a number in the invariant culture: `x: "120"` is 120, `x: 1e2` is 100, and `x: 0x10`, which the YAML core schema reads as 16, is reported as "`x: 0x10` is not a number; zero was used.", with the element kept at zero and the finding on the value's line (finding B2). FBL 7.4 makes a value that does not convert an unreadable entry, which would take the element out of the model. The binding therefore reads `x`, `y`, `width` and `height` as written text, and `FdgReading` converts them by the tool's rule, with `FdgReaderSentences` reporting the ones that do not read. Evidence: `number-forms.fdg`, ten spellings of a number. Would resolve it: a number slot whose conversion and whose fallback a binding states (the scalars it accepts, and a value that does not convert kept at a default and reported rather than made unreadable).

### Gap 11: the id an unresolved end was written with

When a connection's end names no element, the tool still counts it for cardinality and names it: "`ghost` has 2 `ui-child` parents; it may have 1.". A CEL expression sees an unresolved end as absent, so it cannot read the id it was written with, and `std.multiplicity` cannot count connections whose shared end resolves to nothing. The tool also reports one link finding per connection (dangling, then self, then unknown relation, then pair) where `std.endpoints` and `std.references` would each raise one. Both are held by the stand-in `FdgLinkChain` (finding A11, under Q4 of the standalone requirements). Evidence: `cardinality-unknown-end.fdg` and `link-chain.fdg`. Would resolve it: a `sourceId` and `targetId` on a relation in the CEL context (12.x), holding the ids as written.

### Gap 12: the detail and place of a cycle and of a cardinality finding

Section 8.7 binds no `detail` keys for `std.acyclic` findings, only `violation.cycle` in refusals, and gives no location for `std.multiplicity` or `std.acyclic` findings. The tool reports:

- a loop as "Ownership loops: a -> b -> a.", the members rotated to start at the least id, one finding per elementary loop, on the line of that least-id member;
- a cardinality breach on the line of the last connection over the limit.

The specification's message reads `detail.?cycle` optionally, so it is correct once a runtime supplies it; the standalone runtime supplies both, a choice of the runtime and not of the language. Evidence: `ownership-loop-rotation.fdg`, `ownership-loops-sharing-a-member.fdg` and `rule-second-parent.fdg`. Would resolve it: `detail.cycle` for `std.acyclic` findings, and a stated location for both built-ins, in 8.7.

### Gap 13: the indentation of a new entry

The tool copied the first entry of a list, the gap after the dash included; FBL 6.3 indents a new entry like its previous sibling and aligns its keys with the column after `- ` (finding B17). They differ in a list whose entries are not indented alike: after `narrow` in `mixed-indentation.fdg` a new entry is written `  - id:`, where the tool wrote `  -   id:`. The tool now does what FBL says, a change named in the parity transcript (Q1), so nothing stands in. What the text leaves unsaid is whether the gap after a sibling's dash is copied, which FBL's rule for keys answers by implication only (as `timeline-disl-fbl` found, its finding B7). Evidence: `mixed-indentation.fdg`. Would resolve it: a sentence in 6.3 saying the marker and the gap after it are copied from the sibling, or are not.

### Gap 14: the sign of a number that rounds to zero

With `{decimals: 4}`, a value that rounds to zero from below (`-0.00004`) is written `0`; the tool wrote `-0` (finding B19). Both read as zero. FBL 6.3 says a number is rounded to `n` decimals, halves away from zero, with trailing zeros dropped, and says nothing of the sign of a zero, so the library's `0` is recorded and not repaired. The other half of B19, rounding, was a defect of the library (it rounded the binary double, so `200.00005` became `200`) and is repaired there. Evidence: `number-rounding.fdg`. Would resolve it: a sentence in 6.3 that a number which rounds to zero is written `0`.

## What stays code in the standalone module

When the conversion was complete, these were every class of the module's product project (`backend/`), each beside the gap or the host concern that keeps it. A class with neither would have been removed; none was left without one.

| Class | Stays because |
| --- | --- |
| `Diagram`, `ServiceCollection.AddFunctionalDecompositionGraph` | The type's identity in the catalogue, and its wiring into the host (host concern) |
| `FdgDefinition` | The bundled specification loaded once, and what is derived from it: palette, menus, rows, confirmation, findings, connect refusals, and the transaction a command applies (the conversion itself) |
| `FdgBody` | One body opened through the binding, its DISL model built by the shared builder, and its changes planned by the library (the host's side of FBL) |
| `FdgReading` | The tool's rules over what the binding reads: a number converted from its written text ([gap 10](#gap-10-a-number-written-in-quotes)), the first holder of an id written twice keeping it (finding B5), an end resolved to an element by the id it was written with ([gap 11](#gap-11-the-id-an-unresolved-end-was-written-with)), and a mapping that repeats a key read as a body that could not be read (finding B9, Q4) |
| `FdgReaderSentences` | The reader's and the planner's sentences ([gap 6](#gap-6-the-wording-of-a-body-that-cannot-be-read-and-of-a-planners-refusal)) |
| `FdgLinkChain` | One link finding per connection, and the cardinality of an unresolved end (Q4, finding A11; [gap 11](#gap-11-the-id-an-unresolved-end-was-written-with)) |
| `FdgFindingOrder` | Today's order of findings in three places ([gap 8](#gap-8-an-order-of-findings-that-is-not-by-code-and-then-by-line)) |
| `FdgSizeClamp` | `outOfRange.typed`, not yet in the standalone runtime ([gap 9](#gap-9-a-size-clamped-on-commit)) |
| `FdgCommentKeeper` | A removal that keeps the comment above (Q2, [gap 5](#gap-5-a-removal-that-leaves-the-comment-above)) |
| `FdgValidator` | The seam the findings reach the Errors and Warnings panel by; it maps the derived findings and holds no rule (host concern) |
| `FdgContextActionProvider`, `FdgContextPropertyProvider` | The host's menu and property-grid seams; each hands out what `FdgDefinition` derives and computes nothing (host concern) |
| `FdgContextSourceResolver` | The host's selection seam (host concern) |
| `FdgDocumentStore`, `IFdgDocumentStore` | The shared document lifecycle, told how a body reads (host concern) |
| `FdgSession` | One connection's view of one document (host concern) |
| `FdgElementMapper` | The payloads and the viewport filter, which are the host's wire (finding A16) |
| The nine command records and their handlers, and `FdgEditHandler` | History and the wire, which keep each record and signature; the refusal of an element, a connection or a type that is not there; and `ConnectFdgElementsCommandHandler`'s two refusals that are not checks ("This notation has no `<relation>` relation." and "That link names an element that is no longer in this graph.") |

Gone from the module's product code: `FdgParser`, `FdgWriter` and the `_Model/` records (the hand-written reader and writer, kept in the test project's `Parity/` as the oracle), `FdgRuleSet`, `FdgRelations`, `FdgOwnership` and `FdgConnectVerdict` (the hand-written rules, likewise), `FdgToolboxProvider` (the shared `DislToolboxProvider` hands out the palette), `FdgDisl` (the adapter that built the DISL model from the parser's reading until the binding read the body), and the module's `YamlDotNet` reference and its use of `LineSplice`.

## The example

`examples/field-service/field-service.fdg` in the standalone module is a field-service app decomposed from its screens down to functions, with two Comments and two Shows links. Those links form a navigation round trip (Task list, Open task, Task detail, Back to list, Task list), a loop that is legal because it closes only through Shows. Its readme (`examples/field-service/readme.md`, "What it does not demonstrate") lists what it leaves out.

The example in the DISL specification, [`specifications/disl/functional-decomposition.dis`](../../specifications/disl/functional-decomposition.dis), is a DISL 0.2 excerpt that shows 0.2 constructs; where it differs from the tool, its own `doc.description` says so.

The standalone client readme (`client/readme.md`, "What is not settled yet") once said that only one of Shows and the ownership relations could be reached from an Action. That was resolved when the relation came to be inferred from the pair, and the readme now says so.

## Rulings from the owner

- The tool stays a Prototype (Peter, 2026-09-26: "no, keep it as is as it is still a prototype").
- A body that is not YAML opens empty with every edit refused, the Width and Height rows show four decimals, and finding B12 is struck because it does not occur (chat, 2026-10-21).
- The order of findings stays as today, held by `FdgFindingOrder` (chat, 2026-10-11; [gap 8](#gap-8-an-order-of-findings-that-is-not-by-code-and-then-by-line)).
- FBL 0.5 answers finding B1, and FBL spec 033's `constant` answers the writing half of finding B3, so reading and writing through the binding waited on nothing; every elementary ownership loop is reported, as DISL does, which adds a fourth loop to `ownership-loops-sharing-a-member.fdg` (chat, 2026-10-22).
- The standalone requirements were approved written against the defaults of their open questions: an edit writes what FBL writes (Q1), a comment above a removed element stays (Q2), a body that is not YAML opens empty and read-only (Q3), findings and refusals are held to today's (Q4), and the module defects A15, B13 and B14 are repaired, each with a guard seen to fail first (Q6).
- DISL gaps are proposed as additions to the DISL specification rather than kept as plugins (2026-10-05), which is why the earlier plugin persistence is gone and only the wire ids remain under an `x-` key.

## Sources

- Module code, tests and the parity transcript: `src/diagrams/functional-decomposition-graph/` in etalii.adp.ide.standalone.
- Specifications: `.spec-workflow/specs/functional-decomposition-graph/` and `.spec-workflow/specs/functional-decomposition-graph-disl-fbl/` in etalii.adp.ide.standalone.
- Catalogue row: `docs/tools.md` in etalii.adp.ide.standalone.
- Notion: the "Tools" database row for Functional decomposition graph.
