# Wardley map

[wardley-map.dis](wardley-map.dis) specifies ADP's **Wardley map** (`wardley/map`) in DISL 0.3. The specification holds everything DISL 0.3 can state: the metamodel, the unit-square coordinates with the four evolution stages as ranges of the evolution axis and the axes' end labels, the notation, the toolbox, the context menus with their pickers, the forms with their checks and read-only reasons, the constraints with their codes, sentences and order, the operations with their refusals, the deletion confirmation, the ids, and the persistence through the plugin that reads and writes the `.owm` text. This page holds what DISL 0.3 and FBL 0.2 cannot state, and what a runtime will meet that neither says, so that a host implementing the diagram type from the specification ends up with the tool that exists today.

The reference implementation is the standalone module [`src/diagrams/wardley-map/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/wardley-map) in etalii.adp.ide.standalone, converted to this specification by that repository's specification `wardley-map-disl-fbl`. Every sentence, label and code in the specification and below is the one its parity transcript records (`backend/EtAlii.Adp.Diagram.WardleyMap.Tests/Parity/wardley-map.transcript.json`), and every fixture named is under `backend/EtAlii.Adp.Diagram.WardleyMap.Tests/Fixtures/`. Paths are relative to `src/diagrams/wardley-map/` in the standalone repository.

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

The specification names the required plugin `net.etalii.adp.wardley.owm` as its persistence format, because FBL 0.2 cannot read this format as the tool does (gaps 1 to 6). The plugin's `doc` lists its own refusals.

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

Each gap is a candidate change to DISL or FBL, found by the standalone conversion. None is worked around in the specification by stating something the tool does not do. Gaps 1 to 10 were found by reading the module against the languages (section C of the conversion's requirements); gaps 11 and 12 by writing the specification against the transcript.

| # | Gap | Construct | Fixture | Would resolve it |
| --- | --- | --- | --- | --- |
| 1 | A block whose brace is on the line below its statement | FBL 4.7 `blocks` (a block opens only on a statement whose last character is `{`); `lines` has no containment | `pipelines-both-forms.owm` | A rule option that lets a statement own the block the next statement opens, or a `blocks` option naming the brace style |
| 2 | Braces that do not balance, and a brace that opens nothing | FBL 4.7 (an unbalanced body is unreadable; any statement ending in `{` opens a block) | `unclosed-pipeline.owm`, `brace-ending-statements.owm` | Only a statement matched by a rule with `opens`, or by a block rule, opens a block; an unclosed block ends at the end of the body with a finding |
| 3 | A comment that trails a statement | FBL 4.6 `comment` (whole lines only); `re-emit-line` rewrites the comment with the statement | `comments-everywhere.owm`, `trailing-comments.owm` | A binding-level `trailingComment` expression, cut before rules match and kept outside every re-emitted span |
| 4 | A slot found by search inside a group | FBL 2.5 expression subset (no lookaround), `word` slots of one word, one order per `line` expression | `lenient-trailer.owm` | A slot that finds a segment of several words anywhere in a group, in any order |
| 5 | One set-valued attribute stored as the presence of words | FBL 3.3 (one attribute, one place) | `decorators.owm` | A slot whose value is the set of words present, unknown words kept |
| 6 | A list of structured values inside one statement | FBL `lines`/`blocks` slots (one string per group) | `annotations-and-labels.owm` | A slot that yields a list of groups, each splice-able |
| 7 | The Wardley map's identity file is an array of `{id, kind, key}` | FBL 8.7, which describes `{base}.identities.json` as an object mapping a key to an id | `examples/example 2/annotations-and-labels.identities.json`, `examples/example 2/strategy-vocabulary.identities.json` | FBL 8.7 describing the array the tool writes, or a way to read and write it |
| 8 | The key space of `identities:`, an id of its own for each holder of one key, what a change of the key's attribute does to an entry, and whether identity writes are undone | FBL 8.6 (one block of `<key>: <id>`, keys not per rule, a second holder gets `std.duplicateId` and an id that is not stored; nothing on rekeying or history) | `same-key-twice.owm`, `duplicate-names.owm` | Keys per rule or kind, an ordinal for a later holder, a rekey when the attribute a key is computed from changes, and a sentence on whether an identity write is part of the edit's history |
| 9 | `fs.exists` against the project root, and the Wardley example | DISL 12.4 `fs.exists` (the subject's folder), its example on `Submap` | `submap-addresses.owm` | A base-folder argument or a `project` root for `fs.exists`, and the example rewritten over `Url` addresses |
| 10 | A number written by a fixed invariant pattern | DISL 12.4 `formatNumber` (ICU in `env.locale`) | `out-of-range.owm` | A `formatNumber` that takes an invariant locale, or a separate invariant formatter |
| 11 | A picker's confirm label, and a picker of one | DISL 9 `connect` with `target: "pick"` (`pickerTitle`, `emptyText`); a choice dialog with no confirm label | The transcript's choice requests for Link to…, Flow to… and Remove link… | A `pickerConfirm` label, and a `single: "run"` that runs without asking when there is one candidate |
| 12 | Findings within one code ordered by kind, not line | DISL 8.6 ordering, `constraints.order` (8.1) | `out-of-range.owm` | An ordering option that keeps a built-in's findings in the order of the collections it walks |

### 1 to 4. FBL: what the `.owm` grammar needs

The four together are why the body is still read by a plugin. Gap 1 alone blocks every map with a pipeline, because the one nested construct of the format is written with its brace below. Gaps 2 to 4 are what the parser reads leniently today, which the conversion keeps reading (its question Q2): a pipeline that is never closed keeps its children and the map stays editable, only adding to that pipeline being refused ("'X' has a pipeline that is missing its closing brace."); a `//` cuts any line except after a `:`; and `label [dx, dy]`, `( market )`, `inertia` and `url(name)` are found anywhere after the coordinates.

### 5 and 6. FBL: decorators and annotation positions

The specification models the five decorators as the booleans `market`, `ecosystem`, `build`, `buy` and `outsource`, with the row `wardley.decorators` computed over them and the five toggles as one `forEach` menu entry, and an annotation's occurrences as `positions`, the text between its outer brackets, from which the derived node type `AnnotationPin` and the per-occurrence `wardley.coordinate-out-of-range` (the rule `annotationOutOfRange`) read the numbers with the plugin function `positions`. Both shapes are what a binding could bind, once these gaps are answered; until then the plugin reads them.

### 7 and 8. FBL: the identity file

Read through FBL 0.2, every existing map would get new ids and keep a stale file beside it (gap 7), and two holders of one key would get one id between them or an id that is not stored (gap 8), both against the rulings under [Identities](#identities).

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

`std.duplicateId` is off, because every holder of a repeated key has an id of its own. `std.unreadableEntry` is kept on, as `wardley.line-passed-over`, a warning worded by the reader ("This line is not a statement of a Wardley map, so it was passed over.", "This component statement could not be read, so it was passed over.", "This evolve statement could not be read, so it was passed over.", "This line inside a pipeline is not a component of it, so it was passed over."), because the tool reports those lines today and this keeps the reading as it is.

## Other differences a runtime will meet

- **Decorator order.** The row `wardley.decorators` lists the decorators in the order market, ecosystem, build, buy, outsource, whatever order the statement writes them in; the toggles write a new decorator after the last one.
- **Sorting names.** The pickers sort their candidates by `upper(name)`; the tool sorts ordinally ignoring case. The two agree on ASCII names; CEL's `upper()` folds case by Unicode rules, so names that differ in a character outside ASCII may sort differently.
- **Annotation positions.** The plugin function `positions` reads the occurrences as the module's parser does; a host without the plugin uses its CEL fallback, which splits the text on its brackets and commas and converts each number with `double()`, so one number that will not convert fails the whole evaluation where the parser passes over the statement.
- **The evolution axis is no longer an element.** The tool sent the stages to its canvas as a synthetic element `wardley/map+evolution-axis`; the specification's `axis.ranges` replace it.
- **New elements land at 0.5, 0.5**, and a new pipeline component at maturity 0.5, because the drop point does not reach the add.

## What remains in the module, and why

At this step the module still reads and writes the body, holds the ids, and hands out its own toolbox, menus, rows and findings; the conversion's later tasks derive those from this specification. What stays when they are done, and the gap or host concern that keeps it, is recorded here by the conversion's last task.

## Sources

- Module code, fixtures and parity transcript: `src/diagrams/wardley-map/` in etalii.adp.ide.standalone.
- The conversion's findings, A1 to A10 and B1 to B15, its language gaps (section C) and its open questions: `.spec-workflow/specs/wardley-map-disl-fbl/requirements.md` in etalii.adp.ide.standalone.
- The certification of the fixtures against the Online Wardley Maps parser: `backend/EtAlii.Adp.Diagram.WardleyMap.Tests/Fixtures/readme.md`.
- Catalogue row: `docs/tools.md` in etalii.adp.ide.standalone, row for `wardley/map`.
