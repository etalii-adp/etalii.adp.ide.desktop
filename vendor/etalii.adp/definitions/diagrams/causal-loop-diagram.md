# Causal loop diagram: what DISL does not express

This document accompanies [causal-loop-diagram.dis](causal-loop-diagram.dis), the DISL 0.3 specification of the causal loop diagram (`systems/causal-loop-diagram`, extension `.cld`), and its FBL binding, [`specifications/fbl/causal-loop-diagram.fbl#cld`](../../specifications/fbl/causal-loop-diagram.fbl). The specification holds everything DISL 0.3 can state: the metamodel, the notation with its curved links, the toolbox with its drops, the menus with their counts and reasons, the forms with their parsing, the seven findings with the tool's own codes, sentences and order, the operations and their confirmations, and the body read and written through the binding. What DISL 0.3 and FBL 0.2 cannot state is listed here in [Language gaps](#language-gaps), each with its evidence and what would resolve it. The wire ids, which DISL does not carry by ruling, are declared under `x-cld` in the specification.

The tool it describes is the standalone module `src/diagrams/causal-loop-diagram/` in [etalii-adp/etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/causal-loop-diagram). Every sentence, label and code in the specification is taken from that module's parity transcript, `backend/EtAlii.Adp.Diagram.CausalLoopDiagram.Tests/Parity/causal-loop.transcript.json`, recorded over the fixtures beside it (`backend/EtAlii.Adp.Diagram.CausalLoopDiagram.Tests/Fixtures/*.cld`) and the two examples. The conversion that brings the module onto this specification is the standalone repository's spec-workflow specification `causal-loop-disl-fbl`; finding numbers below (A3, B1 and so on) are those of its requirements document. Paths are relative to the module unless they say otherwise.

## Contents

1. [How the specification maps the tool](#how-the-specification-maps-the-tool)
2. [What the definition now states](#what-the-definition-now-states)
3. [Language gaps](#language-gaps)
4. [Definition defects repaired](#definition-defects-repaired)
5. [Other differences a runtime will meet](#other-differences-a-runtime-will-meet)
6. [Pending with the owner](#pending-with-the-owner)
7. [What stays code](#what-stays-code)

## How the specification maps the tool

| Tool concept | In causal-loop-diagram.dis | Notes |
| --- | --- | --- |
| Variable | node type `Variable` | Key attribute `name` (the tool's identifier; `id` is reserved in DISL), `label` its caption, derived `display`. Element id `variable:<name>`. |
| Causal link | relation `CausalLink` | Keyed by `causeName` and `effectName`, which the binding reads from the link's two ends; element id `link:<from>\|<to>`. The free text is `note`, shown as "Note". |
| Loop (a claim about a cycle) | node type `Loop` | Key attribute `identifier`; `title` is shown as "Name"; `members` lists the variables in loop order. Element id `loop:<identifier>`. It is a node because DISL has no construct for a claim about a path. |
| Link polarity | enum `Polarity` | `positive`, `negative`, `unstated`. |
| Computed loop polarity | enum `LoopPolarity`, derived `Loop.computed` | Reinforcing, balancing, undecidable, from the count of negative links around the cycle. |
| Cycle enumeration | `causalCycles` of the plugin `net.etalii.adp.systems.cldCycles`, with `diagram.cycles('CausalLink', max)` as its fallback, and `diagram.cyclesTruncated` | DISL 12.4 and 13.1.1. The plugin is the tool's order; see [Other differences: the order of cycles](#other-differences-a-runtime-will-meet). |
| The body | `persistence.format: "fbl"`, binding `cld` | The standalone module's own copy of the binding; `typeMap` maps its entry types onto the metamodel. |
| Positions | `persistence.view.store: ["bounds"]` | Stored in the `.adp` registration's `layout:` block under the element ids, never in the body. |
| Ring layout | layout `ring`, `circular` with `adp.ring.*` options | See [gap 5](#5-disl-the-seating-order-and-radius-of-circular). |
| Arrange diagram | operation `arrange`, layout `selfOrganizing` from the plugin `net.etalii.adp.systems.cld` | The layout stays code; the plugin provides it alone and is not required to read the document. |

## What the definition now states

The previous companion, written against DISL 0.1, listed twelve aspects DISL could not express. DISL 0.2 and 0.3 express each, and the specification now states it with the construct named:

| Aspect | Construct |
| --- | --- |
| Enumerating the cycles the links form, bounded at 1000 | `diagram.cycles`, `diagram.cyclesTruncated` (the function `feedbackCycles()` and the rule `cycleBoundReached`) |
| One finding per cycle no loop claims | the rule `unlabelledLoop`, `forEach` over the unclaimed cycles, `enforcement: report` |
| The order of the findings | `constraints.order`: unreadable lines, dangling links and the cycle bound first; the rest by line; findings without a line last |
| The tool's own finding codes | `code` on every rule and on `std.unreadableEntry` and `std.references`, the latter computed per finding |
| A reason on an entry that does not apply | `unavailable` with a `reason` on Arrange diagram, Same direction (+) and Opposite direction (−) |
| Labels that change with state | CEL Message labels on Add delay / Remove delay and Curve the other way / Curve back |
| A count in a menu label | `'Remove variable (with ' + n + ' references)'`, with `n` from `removalCount` |
| A toolbox entry dropped onto a variable | `drop.targets` on the Link and Loop tools, `elsewhere: refuse` with "Drop a link or a loop onto a variable." |
| The empty-canvas sentence | `canvas.empty`: "This causal loop diagram states no variables yet." |
| The mirrored marker of a balancing loop | NodeIcon `flip: horizontal` under a condition on `computed` |
| Fields that read `+`, `s`, `-`, `o` and weights | FormItem `parse` with `accepts`, `write` and `refusal` on polarity and weight |
| Arrange's refusal when nothing is drawn | `layout` action `refusals.nothingDrawn` |
| One attribute, two refusals: the inline rename refuses an empty name, a quote and a control character with "A name needs at least one character, and cannot contain a quote or a line break.", while the Label row refuses a quote with `std.facets`' sentence and takes an empty label | the variable label's `parse` with `accepts` and `refusal` (DISL 0.3, 6.12), which belong to the label's editor alone |

The rest of the rewrite is the same move: a link is `routing: "curved"` with `curvature: 0.2` (finding A3; DISL 0.2 made `arc` a circular arc), with a `-0.2` variant for a flipped link; the body is read through the binding with a `typeMap` instead of a plugin (A4, B3); the element ids carry a prefix per type (B10); the wire ids are in `x-cld` (A5); and `std.duplicateId`, `std.unique` and `std.endpoints` are off, with the tool's gesture rules declared in their place, so that the findings and refusals are the tool's list and no more (A7, Q4).

## Language gaps

Each gap is a candidate change to DISL or FBL. None is worked around in the specification by stating something the tool does not do; where the specification and the tool differ because of one, the tool's behaviour is the one described here.

| # | Gap | Construct | Evidence | Would resolve it | Kept in the standalone module by |
| --- | --- | --- | --- | --- | --- |
| 1 | A reason for a statement that almost matched (B2) | FBL `lines` rule, `unmatched` | `unreadable-lines.cld`; `CausalLoopUnreadable` and the transcript's findings | A `fallback` on a rule: a looser expression with a message | `CausalLoopUnreadable` |
| 2 | A tolerant word grammar (B1, B12) | FBL `word` slots, `emit` | `lenient-spellings.cld`, `two-notes.cld`; `CausalLoopNoteWriting.Tests` | A word grammar for `lines`, or a last-match `word` slot and a lookahead | `CausalLoopNote` |
| 3 | Placing new entries of several rules as one group (B6) | FBL `insert.place` | `spellings-kept.cld`, `trailing-comments.cld`; the transcript's edit hunks | An `insert` that places one change's entries as a group | Nothing: the binding's placement is the tool's now (Q1) |
| 4 | A wavy underline (A8) | DISL text `decoration` | `client/causal-loop.css`; the browser pass of the standalone `tests.md` | A `style` on a text decoration | `client/cldBindings.ts` (`labelUnderline`, and the class that marks the loop), drawn by the standalone canvas library (`LabelDeclaration.underline`) |
| 5 | The seating order and radius of `circular` (A9) | DISL standard layout `circular` (D.2, open question 9) | `CausalLoopLayout.Tests`; the transcript's payloads | An order and a radius rule for `circular` | `CausalLoopLayout` |
| 6 | A committed value that did not change (A10) | DISL transactions (forms, operations) | Every document's edit script in the transcript (`"unchanged": true` on setting a loop's own cycle and on Delayed when delayed) | A sentence that a transaction which changes no value is not recorded | The shared history, which records no unchanged edit |
| 7 | Whether a size sent on the wire is a layout-time measurement (A11) | DISL `autoSize: "fitWidth"`, `textMetric` | `CausalLoopMetrics.Tests`; the transcript's payloads | A statement of where a fitted size is measured and whether a host may send it | `CausalLoopMetrics`, `CausalLoopElementMapper` |

### 1. FBL: a reason for a statement that almost matched

Finding B2. A rule says what it matches, not what a line that starts like it and fails should be told. The binding's last rule, of entry type `Unreadable`, takes every statement no other rule read, and the specification maps it to `unreadable`, so DISL reports it once on its line under `causal-loop.unreadable-line` with the reason `std.unreadableEntry` gives. The tool says why, in four sentences of its own, recorded over `unreadable-lines.cld`:

- line 9, `bogus statement`: "'bogus' is not a statement this reading knows."
- line 10, `link a b`: "A link reads 'link <from> -> <to>', optionally followed by a polarity, 'delayed', 'flipped', a weight and a label."
- line 11, `link a -> b weight=heavy`: "'weight=heavy' does not read as a weight."
- line 12, `variable ""`: "A variable needs an id of at least one character."

The lines are the same under the binding; the sentences are not. **Would resolve it:** a `fallback` on a `lines` or `blocks` rule, a looser expression with a message, tried when no rule matched.

### 2. FBL: a tolerant word grammar

Findings B1 and B12. The binding reads every spelling of `lenient-spellings.cld` that a regular expression can state: an unquoted label, a quoted identifier, leading whitespace, an unquoted loop name, and `S` and `O` as polarity. Three things the tool does are beyond it:

- **A bare word as a note.** The tool reads `link a -> indented + bareNote` with the note `bareNote`; the binding's `note` slot takes a quoted word only, so that link has no note.
- **The last candidate wins.** The tool reads `link a -> b + "first note" "second note"` with the note "second note" and `link b -> a - firstBare "second quoted"` with "second quoted" (`two-notes.cld`); a `word` slot takes the first word that matches, so the binding reads "first note" for the first.
- **Quotes are optional when the value has no space.** The tool reads `variable a Label` and `loop R1 Unquoted a indented`; the binding reads them too, but its `emit` templates quote a label and a loop name, so a new statement is written with quotes where the file's own spelling had none.

**Would resolve it:** a word grammar for `lines` that can say "any other word is the note, the last one winning" and "quote a value only when it needs it", short of regular expressions that restate the tool's tokenizer. Two smaller additions would do for the note: a lookahead or not-followed-by in FBL's regular expressions, which FBL forbids today, so a bare word can be told from a flag; and a `word` slot that takes the last matching word rather than the first.

Until then the standalone tool keeps its reading by a stand-in, `CausalLoopNote`: once the binding has read a link statement, the module reads its note again as its own parser did, the bare word and the last of two notes included, and uses that in place of the binding's `note`, and writes an edited note over the word it read it from. Only the note is read so; which lines are links, and every other attribute of one, are the binding's. **Open with the owner:** keep the stand-in (the default, what the tool does now), drop bare-word and last-of-two notes so that the binding's reading is the tool's, or extend FBL first and remove the stand-in then.

### 3. FBL: placing several new entries of different rules as one group

Finding B6. Drawing a link that closes cycles writes the link and its loop claims together, after the last link; the binding places each new entry after the last of its own rule, so the claims go after the last loop, and a first statement of a kind goes after the last entry rather than at the end of the file. The difference shows when a file ends in comments or blank lines, as `spellings-kept.cld` and `trailing-comments.cld` do. **Would resolve it:** an `insert` that places entries of several rules made by one change as one group. Needed only if the owner rules that the tool's present placement is kept (Q1).

### 4. DISL: a wavy underline

Finding A8. A loop whose stated letter disagrees with its arrows is underlined with a **wavy** line (drawn under the caption by the standalone canvas library, which `labelUnderline` in `client/cldBindings.ts` asks for). DISL's text decoration has `underline` and no wavy form; the specification states the underline and documents the difference on the condition. **Would resolve it:** a `style` on a text decoration (`solid`, `wavy`, `dotted`, `dashed`), as CSS has.

A host meets a second limit under this one. The standalone canvas draws a label as SVG text, and Chromium ignores `text-decoration-style` on SVG text, so the stylesheet's wavy line is drawn straight there; the standalone `tests.md` browser pass recorded it, and it was so before the conversion too. A host that draws labels as SVG therefore needs a decoration of its own that draws a wavy line under a label, whatever DISL states. **Ruled by the owner in chat, 2026-10-14:** the standalone canvas library adds that decoration. It draws a wavy line under a label as a shape of its own (`LabelDeclaration.underline`, `canvas/library/definition/wavyUnderline.ts`), in the label's colour and as wide as the text the browser measures, and the causal loop diagram asks for it under a disagreeing loop's caption through `labelUnderline` in `client/cldBindings.ts`; `client/causal-loop.css` no longer states the underline.

### 5. DISL: the seating order and radius of `circular`

Finding A9; DISL D.2, open question 9. The ring a document opens on seats the variables in a depth-first walk of the links in declaration order, starting at the top and running clockwise, on a radius that grows with the widest variable and the count so that no two overlap. DISL names `circular` and defines neither; the specification carries both as `adp.ring.*` options, which no runtime is required to read. **Would resolve it:** an order and a radius rule for `circular` in DISL's standard layouts, or a way for a definition to state them.

### 6. DISL: a committed value that did not change

Finding A10. Committing a value equal to the current one succeeds and changes nothing: setting a loop's variables to the cycle it already runs through, and setting Delayed on a link that is already delayed, each answer success with no change written and nothing on the undo stack. DISL makes every committed field one transaction and does not say that an unchanged value lands none. **Would resolve it:** a sentence in DISL that a transaction which changes no value is not recorded.

### 7. DISL: whether a size sent on the wire is a layout-time measurement

Finding A11. A variable's width is measured on the backend from its label, and the positions the layouts compute depend on it. DISL 0.3 limits `textMetric` to layout-time measurement, and does not say whether a node's `autoSize: "fitWidth"`, sent on the wire, is such a measurement. **Would resolve it:** a statement in DISL of where a fitted size is measured, and whether a host may send it.

## Definition defects repaired

A definition defect is the specification or the binding saying something the tool does not do. Each of these was found by the standalone conversion and repaired here, in the specification or the binding:

| Finding | Was | Is |
| --- | --- | --- |
| A1 | Written against DISL 0.1, which no 0.3 runtime reads | DISL 0.3 |
| A2 | Twelve workarounds for what DISL 0.2 and 0.3 express | The constructs of [What the definition now states](#what-the-definition-now-states) |
| A3 | `routing: "arc"`, which DISL 0.2 made a circular arc | `routing: "curved"` with `curvature: 0.2`, and `-0.2` for a flipped link |
| A4 | `persistence` named a plugin that reads and writes the body, with keys DISL 11.2 forbids beside a binding | `format: "fbl"` with the binding and a `typeMap` |
| B1 | The binding read a narrower grammar than the tool | Its expressions admit an unquoted label, a quoted identifier, leading whitespace, an unquoted loop name and `S` and `O` (what is beyond an expression is gap 2) |
| B3 | A link whose end names no variable was dropped | Read as an entry with reference attributes that the `typeMap` maps to the relation's ends, so it is kept, not drawn, and reported |
| B4 | Removing a variable left the loops through it | `remove.cascade` names `link` and `loop` |
| B7 | A loop had no id | `loop:<identifier>`, from the binding's `id` and the specification's prefix |
| B9 | The binding's template was two variables and a link | The tool's *Births beget births* starter |
| B10 | Stored positions named no element | `persistence.ids` with a `prefix` per type, so the ids are the keys a registration stores |
| A loop's members refused | The `members` item's `validate` message read the chosen value as a list of variables, so a text no option was labelled with, which a host passes on as typed, failed to word the refusal | The message takes such a text as it was typed: "'nobody → nowhere' is not a cycle this loop can run through: …" |
| The arrowhead | `targetMarker: "arrow"`, DISL's open V | `targetMarker: "arrowFilled"`, the filled triangle the tool has always drawn (DISL 6.11); the standalone client stated the filled triangle in code until this was repaired |

## Other differences a runtime will meet

These are not language gaps. Each is a place the transcript shows the tool doing something a runtime of this specification might do otherwise, recorded so that a host does not have to find it again.

- **The order of cycles.** DISL 12.4 orders cycles by first member in model order and then by member order. The tool's finder is Johnson's algorithm, which reports them as a depth-first walk of the links in declaration order finds them, and the standalone conversion compared the two over its corpus of fifteen documents (`CausalLoopCyclesAgainstTheLibraryTests`): the cycles are the same on every one, and the order differs on two. Over `weight-forms.cld` the tool reports a → b → c → d, a → b → c, a → b → d, a → b, a → c → d, a → c → d → b, a → c, b → c → d, b → d, where DISL 12.4 gives a → b, a → b → c, a → b → c → d, a → b → d, a → c, a → c → d, a → c → d → b, b → c → d, b → d; over `lenient-spellings.cld` the tool reports a → b, a → indented, where DISL 12.4 gives a → indented, a → b, because `indented` is declared before `b` and the link to `b` before the link to `indented`. The cycles are the same; their order decides the order of the unlabelled-loop findings and which of several shortest cycles Claim a loop picks. So the tool's finder stays, as the plugin function `causalCycles` of `net.etalii.adp.systems.cldCycles`, which `feedbackCycles` calls, with `diagram.cycles` as its fallback: a host without the plugin reports the same findings in DISL's order. **Candidate for DISL:** a way to ask `diagram.cycles` for discovery order, or a definition of its order that a depth-first finder gives.
- **Numbers.** A weight is shown and written in .NET's shortest round-trip form: `1E+21`, `1E-07`, `1E-06`, `1.2345678901234568E+29` (`weight-forms.cld`). The binding's `number: "shortest"` is ECMAScript's (`1e+21`, `1e-7`). The two agree on ordinary values (finding B11).
- **A reason sent with an entry that applies.** The menus carry a reason on entries that are enabled as well: Arrange diagram is sent "on (There is nothing to arrange until this diagram has two variables.)" with two variables present, and Opposite direction (−) "on (This link already states −.)" on a positive link. The specification states each reason with its `unavailable` condition only.
- **Reasons no definition states.** Arrange diagram on a document that cannot be read answers "This causal loop diagram could not be read, so there is nothing to arrange."; on one that is not open, "This diagram is not open on this connection, so there is nothing to arrange."; the self-organizing layout has its own budget and overlap sentences. An edit of a link's end answers "'causal-loop.link-from' cannot be edited on this selection.", an action of another tool "That action is not this diagram's.", and the empty target gets no menu. These are the host's and the plugin's, not the definition's.
- **Moving a loop or a link.** "Only a variable can be moved; a link is drawn between its ends and a loop label among its members." The specification states it as each type's `refusals.move`.
- **Adding a variable at a point.** Add variable on the canvas writes the statement and its position as one undo step (`AddVariableAtCommand`). A DISL operation's `create` and a registration write are two changes.
- **The link's names are read, not written.** `causeName` and `effectName` are read-only `value` slots of the binding that the specification keys a link by. A new link's are given by the operation's `connect` attributes; a runtime **MUST NOT** plan those as writes to the body, since the `from` and `to` reference attributes the link is written from already carry them.
- **The declared order.** `constraints.order` lists three codes. The other codes share one position after them, sorted by line, and the findings of `unlabelledLoop`, which have no line, come last. That is one reading of DISL's rule for unlisted codes and for findings without a line; the transcript records the order the tool gives, which a runtime checks itself against.
- **The turn of an undecidable loop's marker.** The specification mirrors the loop arrow for a balancing loop only, so an undecidable loop's arrow turns clockwise, as a reinforcing one's does. The standalone canvas drew it anticlockwise before it was compiled from this specification. **Ruled by the owner in chat, 2026-10-14:** keep the specification's drawing, clockwise. A third value of `flip` under a condition on `undecidable` would have been the alternative, which DISL can state, so it was a choice and not a gap.
- **Tooltips.** The specification states a tooltip for a variable, a loop and a link in CEL. The standalone canvas has no CEL and reads a CEL text from the payload its backend computes; no payload carries a tooltip, and a variable's payload carries neither its name nor its label separately, from which one could be built. So the standalone tool draws no tooltip, as it drew none before. A host concern, not a gap: the payload would have to carry the text.
- **A loop that is not a cycle.** `std.references` reports a loop member that names no variable under `causal-loop.loop-is-not-a-cycle`; no fixture of the corpus has such a loop, so that case is not in the transcript.

## Pending with the owner

- **A quote in a label (B13).** Struck by the owner's ruling in chat, 2026-10-13: every path that sets a label already refuses a quote (`label-with-quote.cld`), so the finding does not occur. The specification's `pattern` on `label`, `title` and `note` forbids it as the tool does.
- **Two variables of one name (A7).** `duplicate-variable.cld` declares `a` twice. The specification switches `std.duplicateId` off, so no finding names the second declaration, and the derived findings are none where the hand-written ones threw. The requirement was reworded by the owner's ruling in chat, 2026-10-13: the converted tool **SHALL** at least not throw. It no longer does: the standalone module's ring layout (`CausalLoopLayout`) and its element mapper, which keyed the variables by name and threw on this document, now take the first variable of a name, as everything else reads it; the payloads and every edit of that document answer, and undoing them all brings the original text back.

## What stays code

Every class left in the standalone module, `src/diagrams/causal-loop-diagram/`, beside the gap or the host concern that keeps it. A class with neither was removed: the module's `LoopPolarity`, whose arithmetic the specification states as `Loop.computed` and `Loop.disagrees`, went to the test project as the oracle the derived values are held to. The parser, the writer, the validator's rules and the providers' menus and rows went before it, to the same place.

| Class | Kept by |
| --- | --- |
| `CausalLoopCycles`, `CycleFinder`, `CycleFinderResult` | The order of cycles: `CausalLoopCycles` is the plugin function `causalCycles`, `CycleFinder` the algorithm it runs. |
| `CausalLoopUnreadable` | Gap 1. |
| `CausalLoopNote` | Gap 2. |
| `CausalLoopLayout` | Gap 5: the ring a document opens on. |
| `SelfOrganizingLayout`, `SelfOrganizingResult`, `Commands/ArrangeCausalLoopCommand` | Host: the layout the specification names as the plugin `net.etalii.adp.systems.cld`, and Arrange diagram writing the registration's `layout:` block. |
| `Commands/AddVariableAtCommand` | Host: a body change and a position as one undo step, which no single DISL operation is. |
| `CausalLoopMetrics`, `CausalLoopBox` | Gap 7, and host: the widths measured on the backend and the boxes the layouts place. |
| `CausalLoopElementMapper`, `LoopPolarityResult` | Host: the `.proto` payloads and the viewport filter (A12); a loop's polarity on the wire is the specification's `computed`. |
| `CausalLoopModel`, `CausalLoopVariable`, `CausalLoopLink`, `CausalLoopLoop`, `CausalLoopPolarity` | Host: the binding's reading as the layouts, the mapper and the commands read it, made by `CausalLoopBody.Read`. |
| `CausalLoopBody` | Host: the shared document lifecycle's body, opened through the embedded binding, with the specification's model built from it. |
| `CausalLoopDefinition` | Host: the bundled specification loaded with its plugin through the shared `DislDiagramDefinition`, which maps the derivations onto the host's types and the `x-cld` wire ids. |
| `CausalLoopSession`, `CausalLoopDocumentStore`, `ICausalLoopDocumentStore` | Host: the shared document lifecycle, the reload and the registration's positions. |
| `CausalLoopSelection`, `CausalLoopContextSourceResolver` | Host: what a wire id selects, and resolving a selection to an element of the document. |
| `CausalLoopContextActionProvider`, `CausalLoopContextPropertyProvider`, and the shared `DislToolboxProvider` | Host: handing out the derived toolbox, menus and rows, and dispatching a chosen entry or an edited row as the module's command on the history stack. What they used to compute themselves - the cycle Claim a loop starts from, the next loop identifier and variable name, the choices of a loop's members, the dialogs and every validation sentence - they now take from the specification: its operations through the DISL runtime, its forms' `initial`, `options`, `validate` and `parse` through the library's form commit, and the inline rename's sentence through the label's `parse`. |
| `CausalLoopValidator` | Host: the specification's findings as the host's problems. |
| `Commands/*Command` and their handlers, `Commands/CausalLoopEditHandler`, `Commands/CausalLoopEdits` | Host: the command records the history stack and the wire carry, each writing its change through the binding; the standalone parity tests hold each to the specification's operation. |
| `Diagram`, `ServiceCollection.AddCausalLoop` | Host: the tool's registration and its services. |
| `client/CausalLoopCanvas.tsx`, `client/cldBindings.ts` | Host: the canvas compiled from this specification, with what the library cannot read from it (CEL paths, class names, the weight ladder's classes, and gap 4's wavy underline). |
| `client/causalLoopModel.ts`, `client/useCausalLoopStream.ts`, `client/register.ts` | Host: decoding the payloads, the stream and the module's registration. |
| `client/causal-loop.css` | Host: the theme's colours. |
