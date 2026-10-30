# Wardley map

[wardley-map.dis](wardley-map.dis) specifies ADP's **Wardley map** (`wardley/map`) in DISL 0.3. The specification holds everything DISL 0.3 can state: the metamodel, the unit-square coordinates with the four evolution stages as ranges of the evolution axis and the axes' end labels, the notation, the toolbox, the context menus with their pickers, the forms with their checks and read-only reasons, the constraints with their codes, sentences and order, the operations with their refusals, the deletion confirmation, the ids, and the persistence through the plugin that reads and writes the `.owm` text. This page holds what DISL 0.3 and FBL 0.2 cannot state, and what a runtime will meet that neither says, so that a host implementing the diagram type from the specification ends up with the tool that exists today.

The reference implementation is the standalone module [`src/diagrams/wardley-map/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/wardley-map) in etalii.adp.ide.standalone, converted to this specification by that repository's specification `wardley-map-disl-fbl`. Its delivery report, with every gap as a candidate change for this repository, is [wardley-map-report.md](wardley-map-report.md). Every sentence, label and code in the specification and below is the one its parity transcript records (`backend/EtAlii.Adp.Diagram.WardleyMap.Tests/Parity/wardley-map.transcript.json`), and every fixture named is under `backend/EtAlii.Adp.Diagram.WardleyMap.Tests/Fixtures/`. Paths are relative to `src/diagrams/wardley-map/` in the standalone repository.

## Contents

1. [What the diagram is for](#what-the-diagram-is-for)
2. [The document on disk](#the-document-on-disk)
3. [Identities](#identities)
4. [What DISL cannot express](#what-disl-cannot-express)
5. [Language gaps](#language-gaps)
6. [Definition defects repaired](#definition-defects-repaired)
7. [Other differences a runtime will meet](#other-differences-a-runtime-will-meet)
8. [What remains in the module, and why](#what-remains-in-the-module-and-why)
9. [Sources](#sources)

## What the diagram is for

A Wardley map places the components of a value chain by two measures: how visible each is to the user whose need anchors the chain (top to bottom), and how evolved it is, from genesis through custom built and product to commodity (left to right). Links say what depends on what; an evolution says where a component is heading; pipelines, inertia and the market, ecosystem, build, buy and outsource decorators say how it is sourced. The map is for arguing about a strategy rather than asserting one.

- **Origin.** The method is Simon Wardley's and the text format is that of Online Wardley Maps, which is why the origin is `wardley/`.
- **State.** ⚗️ Prototype, the state its row in the standalone `docs/tools.md` carries, recorded under `x-adp.state`.
- **Out of scope by decision**: editing the `.owm` as text with a live preview, as onlinewardleymaps.com and its editor extensions do.

## The document on disk

A map is an `.adp` registration whose first line is `wardley/map` and a body, `<name>.owm` (`.wm` is read too), in the Online Wardley Maps text format. A new map's body is `title <name>` and a line ending. The fixtures are certified against the real Online Wardley Maps parser vendored by `cli-owm` 0.0.2 (`Fixtures/readme.md`, `certify.mjs`).

| Statement | Model |
| --- | --- |
| `title Text`, `style Name`, `size [w, h]` | Diagram attributes; `style` and `size` are kept and not drawn. A second `title` wins (`two-titles.owm`). |
| `component Name [v, m] label [dx, dy] (decorator) inertia url(name)` | `Component`; what follows the coordinates may come in any order (`lenient-trailer.owm`) |
| `anchor Name [v, m]`, `submap Name [v, m] url(name)` | `Anchor`, `Submap` |
| `url name [address]` | `Url` |
| `A->B`, `A+>B`, optionally `; context` | `Dependency`, `Flow`; the ends are names |
| `evolve Name m`, `evolve Name->NewName m` | `Evolution`; only the first of a component is used (`evolve-twice.owm`) |
| `pipeline Parent` and a `{ component Child [m] }` block | `Pipeline` and `PipelineComponent`; the brace is on the line below (`pipelines-both-forms.owm`) |
| `pipeline Parent [a, b]` | `Pipeline` in the older two-coordinate form, its numbers kept as written |
| `note text [v, m]` | `Note` |
| `annotation N [v, m] text`, `annotation N [[v, m], [v, m]] text` | `Annotation`, its occurrences drawn as `AnnotationPin`s; `annotations [v, m]` is kept and not drawn |
| `accelerator Name [v, m]`, `deaccelerator Name [v, m]` | `Accelerator` |
| `pioneers`, `settlers`, `townplanners [v1, m1, v2, m2]` | `Attitude`; the corners may be given in any order |
| `y_axis`, `x_axis`, `evolution`, `presentation`, any other line, `// comment` | Not modelled and kept byte for byte (`unmodelled-statements.owm`, `comments-everywhere.owm`); a `//` after a `:` is not a comment, so `https://` survives |

Coordinates are written `[visibility, maturity]`; the specification places `x` from maturity and `y` from visibility. `market` and `ecosystem` are decorators, not statement kinds: `market Foo [0.6, 0.8]` is a parse error for the real parser (`Fixtures/readme.md`). A document ADP did not change is written back byte for byte, every line ending, blank-line run and missing final ending included (`crlf-line-endings.owm`, `lf-line-endings.owm`, `mixed-line-endings.owm`, `no-trailing-newline.owm`, `blank-line-runs.owm`); an edit rewrites only the lines it changes, and a new statement is appended at the end.

The specification names the required plugin `net.etalii.adp.wardley.owm` as its persistence format, because FBL 0.2 could not read this format as the tool does (gaps 1 to 6). FBL 0.12 binds it in the `grammar` family by `wardley-map.fbl` (see [Language gaps](#language-gaps)); the specification names the plugin until the conversion reads the body through that binding. The plugin's `doc` lists its own refusals.

## Identities

The body holds no ids; a statement names another element by its name. Every element is given a ShortGuid, a version 4 GUID written as 25 characters of base 36 (`persistence.ids`), kept beside the body in `<name>.identities.json`, keyed by the element's kind and a natural key: a component's name, a link's ends and kind, a pipeline's parent, a pipeline component's parent and name, a note's text, an annotation's number, an accelerator's name, an attitude's kind and corners. A missing or stale file is reconciled against the body on opening, so ids survive an edit made in another tool where the key still matches.

Two elements with one key (two components of one name, one link stated twice, two notes of one text) each get an id of their own, ruled by the user in chat on 2026-10-11 ("both get a unique id. everything is changed to cope with it."). The rulings of 2026-10-12 settle what follows from that: a later holder's entry carries its ordinal in its key and is first stored with the next save that writes the map; a statement naming a repeated name attaches to the first holder in document order; a link, an evolution or a pipeline drawn from a later holder first gives that holder a new name with a number after it; a rename renames the selected holder only; adding or renaming to a name the map already has stays refused; and after an edit made outside ADP the ids follow the order of the holders (`same-key-twice.owm`, `duplicate-names.owm`). None of this is in the specification, which DISL and FBL cannot say (gaps 7 and 8).

## What DISL cannot express

- **A submap that is not in the project** (gap 9). `wardley.submap-missing`, a warning, runs over every `url` definition's address, resolves it against the project root, and is located on the definition's line: "'Linked' points at 'linked.owm', which is not in this project." or "This map defines 'also-missing.owm', which is not in this project." (`submap-addresses.owm`). DISL's `fs.exists` resolves against the folder of the subject, and its own example in 12.4 states the rule on `Submap` with `self.url`, which in this tool is the name of a definition, not an address. The specification lists the code in `constraints.order` and states no rule for it.
- **A number written by the format's own invariant pattern** (gap 10). `wardley.coordinate-out-of-range` writes the value as `0.####` whatever the locale: `[0.5, 1.00005]` gives "The maturity of 'Far' is 1.0001, and a Wardley map's axes run from 0 to 1." (`out-of-range.owm`). DISL's `formatNumber` is ICU formatting in `env.locale`; the specification calls the plugin function `fixedNumber`, with `formatNumber(n, '0.####')` as its fallback.
- **The label of a picker's confirm button, and a picker of one** (gap 11). Link to… and Flow to… confirm with "Link" and "Flow"; Remove link… asks nothing when the element has one link and offers "Remove link" with the button "Remove" otherwise (the transcript's choice requests). `connect` with `target: "pick"` has a title and an empty text but no confirm label, and always asks.
- **Findings of one code in the order of their kinds** (gap 12). `wardley.coordinate-out-of-range` is reported collection by collection: components, pipelines, evolutions, notes, accelerators, attitudes, annotations, so `out-of-range.owm` lists lines 2, 3, 4, 17, 19, 9, 10, 11, 12, 13. DISL 8.6 orders findings of one code by location.
- **The wire ids** are under `x-wardley`, as ruled for every converted tool (Peter, 2026-10-06, "Spec all but wire ids"): the toolbox's ids, the action ids keyed by operation or kind with `<Type>/<kind>` overrides and a `<decorator>` placeholder, the property ids keyed by form item, and the element type ids.

## Language gaps

Each gap is a candidate change to DISL or FBL, found by the standalone conversion. None is worked around in the specification by stating something the tool does not do. Gaps 1 to 10 were found by reading the module against the languages (section C of the conversion's requirements); gaps 11 and 12 by writing the specification against the transcript. FBL 0.12 (etalii.adp's feature `066-fbl-conversion-gaps`) answers gaps 1 to 8, each below with the section that answers it and the fixture under `specifications/fbl/fixtures/` that shows it: the format is bound in the `grammar` family by [`wardley-map.fbl`](../../specifications/fbl/wardley-map.fbl), whose grammar is [`wardley-map.lark`](../../specifications/fbl/wardley-map.lark), written from the conversion's appendix draft in FBL 0.12's constructs. The specification still names the plugin until the conversion's task 11 reads the body through that binding. Gaps 9 to 12 are DISL's and stay open: each concerns what a specification computes, says or orders, which no binding of the body changes.

| # | Gap | Construct | Fixture | Status |
| --- | --- | --- | --- | --- |
| 1 | A block whose brace is on the line below its statement | FBL 4.7 `blocks` (a block opens only on a statement whose last character is `{`); `lines` has no containment | `pipelines-both-forms.owm` | Answered by FBL 0.12, 4.9 (`wardley-pipelines`) |
| 2 | Braces that do not balance, and a brace that opens nothing | FBL 4.7 (an unbalanced body is unreadable; any statement ending in `{` opens a block) | `unclosed-pipeline.owm`, `brace-ending-statements.owm` | Answered by FBL 0.11 and 0.12, 4.9 (`wardley-unclosed-pipeline`, `wardley-brace-ending`) |
| 3 | A comment that trails a statement | FBL 4.6 `comment` (whole lines only); `re-emit-line` rewrites the comment with the statement | `comments-everywhere.owm`, `trailing-comments.owm` | Answered by FBL 0.11, 4.9 (`wardley-comments`) |
| 4 | A slot found by search inside a group | FBL 2.5 expression subset (no lookaround), `word` slots of one word, one order per `line` expression | `lenient-trailer.owm` | Answered by FBL 0.12, 4.9 (`wardley-trailer`) |
| 5 | One set-valued attribute stored as the presence of words | FBL 3.3 (one attribute, one place) | `decorators.owm` | Answered by FBL 0.11 and 0.12, 4.9 and 5.2 (`wardley-trailer`) |
| 6 | A list of structured values inside one statement | FBL `lines`/`blocks` slots (one string per group) | `annotations-and-labels.owm` | Answered by FBL 0.12, 5.2 and 6.3 (`wardley-annotation-positions`) |
| 7 | The Wardley map's identity file is an array of `{id, kind, key}` | FBL 8.7, which describes `{base}.identities.json` as an object mapping a key to an id | `examples/example 2/annotations-and-labels.identities.json`, `examples/example 2/strategy-vocabulary.identities.json` | Answered by FBL 0.12, 8.7 (`wardley-legacy-identities`, `wardley-strategy-identities`) |
| 8 | The key space of `identities:`, an id of its own for each holder of one key, what a change of the key's attribute does to an entry, and whether identity writes are undone | FBL 8.6 (one block of `<key>: <id>`, keys not per rule, a second holder gets `std.duplicateId` and an id that is not stored; nothing on rekeying or history) | `same-key-twice.owm`, `duplicate-names.owm` | Answered by FBL 0.12, 5.3 and 8.6 (`wardley-duplicate-names`, `wardley-rename-identity`) |
| 9 | `fs.exists` against the project root, and the Wardley example | DISL 12.4 `fs.exists` (the subject's folder), its example on `Submap` | `submap-addresses.owm` | Open; DISL's: a base-folder argument or a `project` root for `fs.exists`, and the example rewritten over `Url` addresses |
| 10 | A number written by a fixed invariant pattern | DISL 12.4 `formatNumber` (ICU in `env.locale`) | `out-of-range.owm` | Open; DISL's: a `formatNumber` that takes an invariant locale, or a separate invariant formatter |
| 11 | A picker's confirm label, and a picker of one | DISL 9 `connect` with `target: "pick"` (`pickerTitle`, `emptyText`); a choice dialog with no confirm label | The transcript's choice requests for Link to…, Flow to… and Remove link… | Open; DISL's: a `pickerConfirm` label, and a `single: "run"` that runs without asking when there is one candidate |
| 12 | Findings within one code ordered by kind, not line | DISL 8.6 ordering, `constraints.order` (8.1) | `out-of-range.owm` | Open; DISL's: an ordering option that keeps a built-in's findings in the order of the collections it walks |

### 1 to 4. FBL 4.9: what the `.owm` grammar needs, answered

The four together were why the body is read by a plugin. Gap 1 alone blocks every map with a pipeline, because the one nested construct of the format is written with its brace below. Gaps 2 to 4 are what the parser reads leniently, which the conversion keeps reading (its question Q2): a pipeline that is never closed keeps its children and the map stays editable, only adding to that pipeline being refused ("'X' has a pipeline that is missing its closing brace."); a `//` cuts any line except after a `:`; and `label [dx, dy]`, `( market )`, `inertia` and `url(name)` are found anywhere after the coordinates. FBL 0.12, 4.9, answers gap 1: the token that opens a pipeline's block is a `{` that line endings may precede, so a pipeline with its brace below owns its block, and `opens` writes a new one that way (`wardley-pipelines`). Gap 2 is answered by FBL 0.11's recovery inside blocks and by 4.9's rule that only what the grammar reads as a block opens one: a `{` that ends a title, a comment, a context or a note is a byte of its token, a `{` alone where a statement may start opens nothing and is passed over, a `}` that closes nothing is reported, and an add into an unclosed pipeline is refused while every other edit is planned (`wardley-brace-ending`, `wardley-unclosed-pipeline`). Gap 3 is answered by FBL 0.11's comments, an ignored token that every splice keeps, with a url address one token that runs past the `//` of its scheme (`wardley-comments`). Gap 4 is answered by FBL 0.12's segments in any order, each a node of its own set alone, written by `create` at its declared place and removed with one separator (`wardley-trailer`).

Four spellings the module reads are not read by `wardley-map.lark`, each reported as `fbl.unparsed-text` and kept: an evolution of a component whose name ends in a number (`evolve Tea 2 0.8`, whose `2` is read as the maturity); a pipeline's `{` after a comment on a line of its own between it and its statement; a component name holding parentheses, which open a decorator; and the keyword `inertia` written in another case is read as a segment whose value `map` does not translate, since `caseInsensitive` is not a property of the `grammar` family.

### 5 and 6. FBL 5.2: decorators and annotation positions, answered

The specification models the five decorators as the booleans `market`, `ecosystem`, `build`, `buy` and `outsource`, with the row `wardley.decorators` computed over them and the five toggles as one `forEach` menu entry, and an annotation's occurrences as `positions`, the text between its outer brackets, from which the derived node type `AnnotationPin` and the per-occurrence `wardley.coordinate-out-of-range` (the rule `annotationOutOfRange`) read the numbers with the plugin function `positions`. The five booleans have no order, so the row reads the order the statement writes them in through the plugin function `writtenDecorators`, whose fallback lists them in the notation's order. FBL 0.12 answers both in the binding: a `many` attribute's `{node}` slot reads every decorator in written order, an unknown one as written, and a set writes the difference (`wardley-trailer`), and a slot's `items` reads the positions as a list of structured values, each set, added and removed alone (`wardley-annotation-positions`). The specification's shapes change with the conversion's task 11, which binds them; until then the plugin reads them.

### 7 and 8. FBL 8.6 and 8.7: the identity file, answered

Read through FBL 0.2, every existing map would get new ids and keep a stale file beside it (gap 7), and two holders of one key would get one id between them or an id that is not stored (gap 8), both against the rulings under [Identities](#identities). FBL 0.12, 8.7, reads the legacy identities file as the array the tool writes, an entry matched by its kind and key, one of a kind no rule declares kept and reported (`wardley-legacy-identities`), the accelerators' and the attitudes' keys as the tool writes them (`wardley-strategy-identities`); 5.3 and 8.6 give a sidecar id its `kind` and an `ordinal`, `"\u001f{n}\t"` as the tool writes it, so that each holder of one key has an id of its own, and move an entry with its key in the edit that changes it, the undo moving it back (`wardley-duplicate-names`, `wardley-rename-identity`).

### 9. DISL: the project root

The rule needs the address resolved where the tool resolves it. Stating it with `fs.exists` would state a rule the tool does not apply, so the specification states none.

### 10. DISL: the invariant number

The first half of this gap as section C records it, `oncePerGroup` reporting at the second or last holder where the tool reports at the first, is struck: the rule `duplicateName` is stated at the first holder by a `when` over `positionIn`, with the lines of every holder from `location()`. What remains is the number.

### 11. DISL: the picker's button and a picker of one

Found writing the menus against the transcript. The specification's `connect` entries carry their title and empty text; the confirm labels and the unlink of one link stay in the module's map of wire ids.

### 12. DISL: the order within one code

Found writing the constraints against the transcript. `constraints.order` puts the codes in the tool's order; within `wardley.coordinate-out-of-range` a runtime following DISL lists the findings by line.

## Definition defects repaired

A definition defect is the specification saying something the tool does not do, or leaving out something it does. The earlier definition was written against DISL 0.1; these were repaired by the rewrite at DISL 0.3, and the last by the task that loaded it in the standalone library:

| Finding | Was | Is |
| --- | --- | --- |
| A1 | DISL 0.1, which the standalone client refuses | DISL 0.3 |
| A2 | The text format and its round trip named only as a plugin | The required plugin `net.etalii.adp.wardley.owm` stated as what it is, with its refusals; `format: "fbl"` waits on gaps 1 to 6 |
| A2 | Ids outside the document called not expressible | `persistence.ids` `uuid-v4` in `base36`, stable; the sidecar's shape is gaps 7 and 8 |
| A2 | References by name that may dangle called not expressible | The rules `danglingSource`, `danglingTarget`, `evolveTargetMissing` and `pipelineParentMissing`, which name the end as written through `writtenEnd` |
| A2 | The evolution stages as an enum and a derived attribute | `axis.ranges`: Genesis, Custom Built, Product (+rental), Commodity (+utility) at 0.175, 0.4 and 0.7, as bands of increasing opacity with dashed separators; `stage` reads `axisRange()` |
| A2 | No end labels | `endLabels`: "Invisible" at the bottom, "Visible" at the top |
| A2 | A fixed canvas | `fit: "stretch"` over bounds of 1000 by 1000 with a margin of 90 |
| A2 | The label offset as view data | A label `offset` bound to the attribute `labelOffset`, default `[15, 4]` |
| A2 | Static menu labels and a confirmation without the name | CEL Messages: Evolve to… or Change evolution target…, Mark as or Clear for inertia and each decorator, and "Remove '{name}', and every link and evolution that names it?" |
| A2 | Link to… and Flow to… without a picker | `connect` with `target: "pick"`, the other elements' names as candidates, sorted |
| A2 | Read-only rows without their reason | `readOnlyReasons` on every read-only row, and `behavior.reasons.readOnlyFile` "This map's file is read-only." |
| A2 | `wardley.submap-missing` called not expressible | Stated as not expressible for the right reason, the project root (gap 9) |
| A2 | Clamping a drag and refusing a typed value not told apart | `outOfRange` on both axes: a drag stops at the edge, a typed value is refused with "Both of a Wardley map's axes run from 0 to 1, so a position outside that is off the map." |
| A3 | `urls` a diagram list, `decorators` one list, `AnnotationPin` a stored child | `Url` a node type; five boolean decorators; `positions` text and a derived `AnnotationPin` |
| A4 | No wire ids | The `x-wardley` block |
| A9 | An evolution a node type of its own, and a warning for a second `evolve` | The rows Evolves to and Becomes on the component, whose `parse.write` creates, sets or removes the `evolve`; no warning, because the tool gives none |
| A7 | `wardley.duplicate-name` located on the element's name, which names nothing | At the first holder, with the lines of all holders |
| Task 4 | The next annotation number taken with `.max()` on a list, which is no function of DISL's CEL, so the standalone loader refused the specification | `math.greatest` over the numbers, one of the math extension's functions DISL requires (12.1) |
| Task 6 | The add, Rename… and pipeline dialogs accepting an empty name, which the tool refuses at once | A commit `validate` on each name field: "Every element needs a name.", or "A note needs something to say." for a note and an annotation |
| Task 6 | Evolves to accepting an empty value on an element that is not evolving, which writes nothing | Refused with "'X' is not evolving." |
| Task 6 | Decorators listed in the notation's order | In the order the statement writes them, through `writtenDecorators` (gap 5) |
| Task 6 | Label offset shown as the struct itself | The item's `display`, `dx, dy` through `fixedNumber`; the data type's own `display` is not what a form row shows in the standalone runtime |
| Task 6 | A pipeline's Components on one line | The `textarea` widget, as the tool shows them |

`std.duplicateId` is off, because every holder of a repeated key has an id of its own. `std.unreadableEntry` is kept on, as `wardley.line-passed-over`, a warning worded by the reader ("This line is not a statement of a Wardley map, so it was passed over.", "This component statement could not be read, so it was passed over.", "This evolve statement could not be read, so it was passed over.", "This line inside a pipeline is not a component of it, so it was passed over."), because the tool reports those lines today and this keeps the reading as it is.

## Other differences a runtime will meet

- **Decorator order.** The row `wardley.decorators` lists the decorators in the order the statement writes them in through `writtenDecorators`; a host without the plugin lists them in the order market, ecosystem, build, buy, outsource. The toggles write a new decorator after the last one.
- **Sorting names.** The pickers sort their candidates by `upper(name)`; the tool sorts ordinally ignoring case. The two agree on ASCII names; CEL's `upper()` folds case by Unicode rules, so names that differ in a character outside ASCII may sort differently.
- **Annotation positions.** The plugin function `positions` reads the occurrences as the module's parser does; a host without the plugin uses its CEL fallback, which splits the text on its brackets and commas and converts each number with `double()`, so one number that will not convert fails the whole evaluation where the parser passes over the statement.
- **The evolution axis is no longer an element.** The tool sent the stages to its canvas as a synthetic element `wardley/map+evolution-axis`; the specification's `axis.ranges` replace it. In its place the tool sends one element `wardley/map+diagram` for the map itself, carrying the attributes `metamodel.diagram` declares (title, style, size and the annotations' position), and the canvas names the map by that title.
- **New elements land at 0.5, 0.5**, and a new pipeline component at maturity 0.5, because the drop point does not reach the add.

## What remains in the module, and why

Every type of the product project `backend/EtAlii.Adp.Diagram.WardleyMap/`, checked against `git ls-files` of that project at the end of the conversion's stage one (its task 10), beside the gap or the host concern that keeps it, and what stage two (its tasks 11 to 13, which wait on gaps 1 to 4, 7 and 8) does to it. `WardleyToolboxProvider` had neither, since core's `DislToolboxProvider` hands out a bundled definition's palette, and was removed by that task. `WardleyRuleSet`, `WardleyActions` and the providers' hand-written logic left the product code earlier in stage one; the test project keeps them as the oracle (`Parity/HandWrittenWardley.cs`).

| Type | Kept by | Stage two |
| --- | --- | --- |
| `WardleyParser` | The body read as the tool reads it: a pipeline's brace on the line below (gap 1), the lenient reading of braces, comments and a statement's trailer (gaps 2 to 4), decorators and annotation positions (gaps 5 and 6). | Leaves; the body is read through a binding (task 11), but for one named reader of what gaps 3 and 4 leave open. |
| `WardleyWriter`, `RestoreWardleyLineCommand` and its handler | Every edit written by rewriting the lines it touches, and undone by putting a line back, because the body is not read through a binding (gaps 1 to 6). | Leave; every edit is the binding's splices (task 12). |
| `WardleyDisl` | The DISL model built from the parser's reading and the store's identities (gap 1). | Leaves; the model is the binding's reading (task 11). |
| `WardleyTransactionCommands` | A transaction of the definition written as one of today's commands, standing in for FBL's splices (gap 1). | Leaves (task 12). |
| `WardleyIdentities`, `WardleyIdentityKeys`, `WardleyIdentityMoves` | The identity file as the tool writes it, an array of `{id, kind, key}` (gap 7), with an id of its own for every holder of one key and the ids carried across an edit (gap 8). | Leave once the ids are kept through the registration and the legacy identities file (task 12). |
| `WardleyDecorators` | The five decorators as the words the statement writes, in the order it writes them (gap 5). | Leaves with the parser and the writer, unless gap 5 is still open. |
| `WardleyFunctions` | The plugin `net.etalii.adp.wardley.functions`: `writtenEnd`, `writtenDecorators` (gap 5), `positions` (gap 6) and `fixedNumber` (gap 10). | Stays for every gap still open; a function whose gap is closed leaves. |
| `WardleySubmapRule` | `wardley.submap-missing`, which resolves an address against the project root (gap 9, finding A5). | Stays until gap 9 is answered. |
| `WardleyMap` and the records and enums of `_Model/` | What the parser reads into, and what the mapper, the session, the commands and the descriptions address. | What the binding's reading does not replace leaves with the parser (task 12). |
| `MoveWardleyElementCommand`, `WardleyElementCommands` (eleven records) and `WardleyElementCommandHandlers` | The host's command records, unchanged on the wire, each through the project's history; the handlers locate against current state and refuse with the tool's sentences. | The records stay; the handlers plan their edits as the binding's model changes (task 12). |
| `WardleyElementMapper`, `WardleyElementTypes` | The host's wire: the map in the element vocabulary of `api/wardley-map.proto`, the diagram element carrying the attributes of `metamodel.diagram`, and the viewport filter that keeps a frame and judges a link by its ends (Requirement 7.1). | Stay. |
| `WardleyAxis`, `WardleyEvolution` | The `.proto` payloads' coordinates on the canvas's square and each element's evolution stage, resolved (Requirement 7.1); a dragged or dropped position clamped to the map. | Stay. |
| `WardleySession` | The host's session seam: the deltas a connection is sent, and a move from the canvas's coordinates. | Stays. |
| `IWardleyDocumentStore`, `WardleyDocumentStore` | The shared document lifecycle and its reload rules: one open map per file, its identities kept beside it and remembered for an undo and a redo. | Stay; read through the binding (task 11). |
| `WardleyEditability` | Whether a map's file can be written, which withholds every edit (`behavior.reasons.readOnlyFile`). | Stays. |
| `WardleyDefinition` (with `WardleyStep`) | The bundled `.dis` loaded once, and the palette, menus, rows, findings and deletion confirmation derived from it, mapped by `x-wardley`. | Stays. |
| `WardleyElementDescriptions`, `WardleySelection`, `WardleyContextSourceResolver` | The host's selection: an element id resolved against the map, its text and path, and a placement on empty map resolved to the map itself so that the diagram's menu is offered there. | Stay. |
| `WardleyContextActionProvider`, `WardleyContextPropertyProvider`, `WardleyValidator` | Core's seams, handing out what the definition derives. | Stay. |
| `Diagram` | The host's catalog entry: the origin and the `.owm` extension. | Stays. |
| `ServiceCollectionAddWardleyMapExtension`, `ServiceCollectionAddWardleyMapCommandsExtension` | The host's registration of the types above, and of core's `DislToolboxProvider` over the definition. | Stay. |

Beside them the client keeps `client/wardleyBindings.ts`, which says what the canvas library cannot read from the definition (where the values the backend computes sit in the payloads, the class names that tie the drawing to `wardley.css`, the submap's double ring and the keys the backend is sent), and `client/wardleyModel.ts`, which decodes the payloads. The hand-written canvas definition is read by a test only, from `client/handWrittenDefinition.ts`: the oracle `client/wardleyCompiledDefinition.test.tsx` holds the compiled one to.

## Sources

- Module code, fixtures and parity transcript: `src/diagrams/wardley-map/` in etalii.adp.ide.standalone.
- The conversion's findings, A1 to A10 and B1 to B15, its language gaps (section C) and its open questions: `.spec-workflow/specs/wardley-map-disl-fbl/requirements.md` in etalii.adp.ide.standalone.
- The certification of the fixtures against the Online Wardley Maps parser: `backend/EtAlii.Adp.Diagram.WardleyMap.Tests/Fixtures/readme.md`.
- Catalogue row: `docs/tools.md` in etalii.adp.ide.standalone, row for `wardley/map`.
