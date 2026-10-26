---
id: atd-specification
type: submodule-design
title: ADP Tool Language (ATD) — normative specification
status: draft
parent: atd
---
# ATD, the ADP Tool Language

**Specification, version 0.1 (Working Draft)**

|                          |                                                                                              |
|--------------------------|----------------------------------------------------------------------------------------------|
| Version                  | 0.1                                                                                          |
| Status                   | Working Draft                                                                                |
| Date                     | 2026-10-25                                                                                   |
| Grammar                  | [`grammar/`](grammar/): `atd.lark` and the modules it imports, in ATD grammar notation (section 4) |
| Tree schema              | `atd.schema.json` (JSON Schema, draft 2020-12), `$defs/Tree`; also `$defs/Findings`, `$defs/Trace`, `$defs/ConformanceFixture`, `$defs/Drawing`, `$defs/Capability`, `$defs/ParseTables` (Appendix A) |
| Serves                   | every kind of tool: diagrams (with [DISL](../disl/DISL-specification.md)), designers (with [DESL](../desl/DESL-specification.md)), editors (in place of [EDSL](../edsl/EDSL-specification.md)), and their bindings (with [FBL](../fbl/FBL-specification.md)) |
| Expression language      | CEL — Common Expression Language (https://cel.dev)                                           |
| File extension           | `.atd`, the ADP Tool Description                                                             |
| Version key              | `atd <major>.<minor>`, the first construct of every file                                     |
| Licence                  | [Apache License 2.0](https://github.com/etalii-adp/etalii.adp/blob/develop/LICENSE) (`Apache-2.0`) |

---

## Status of this document

This is a working draft, version 0.1, the first of ATD, written for phase 1 of the move of every tool type to one language (spec 035). In phase 1 the tool descriptions of the tool types in scope are written in ATD beside the DISL, DESL and FBL files that every host keeps running, and ATD's toolchain checks, edits, validates and debugs them; no host loads a tool from a tool description (section 1.2). Any construct may change before 1.0 (constitution principle IV); a description is always read with the grammar of the version it declares (section 18).

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY** and **OPTIONAL** are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in bold capitals. Sections and paragraphs marked *(informative)* explain intent and give guidance; everything else is *normative*. The words of [docs/terminology.md](../../docs/terminology.md) are used as defined there: tool, tool type, diagram, designer, editor, tool engineer, specification, definition, binding, body, splice, reading, finding, host, runtime, ADP tool language, tool description, script, capability, conformance fixture and trace.

---

## Table of contents

1. [Introduction](#1-introduction) *(informative)*
2. [Conformance](#2-conformance)
3. [Lexical structure](#3-lexical-structure)
4. [ATD grammar notation](#4-atd-grammar-notation)
5. [Files and descriptions](#5-files-and-descriptions)
6. [Values](#6-values)
7. [The core](#7-the-core)
8. [The canvas surface](#8-the-canvas-surface)
9. [The form surface](#9-the-form-surface)
10. [The text surface](#10-the-text-surface)
11. [Bindings](#11-bindings)
12. [The script](#12-the-script)
13. [Capabilities](#13-capabilities)
14. [Static semantics](#14-static-semantics)
15. [Findings](#15-findings)
16. [Canonical layout](#16-canonical-layout)
17. [The host's concerns](#17-the-hosts-concerns)
18. [Versioning](#18-versioning)
19. [Complete examples](#19-complete-examples) *(informative)*

[Changes](#changes) · [Appendix A — The schema's `$defs`](#appendix-a--the-schemas-defs) · [Appendix B — Keywords](#appendix-b--keywords) · [Appendix C — DISL, DESL and FBL in ATD](#appendix-c--disl-desl-and-fbl-in-atd)

---

## 1. Introduction

*(informative)*

### 1.1 What ATD is

ATD is the one language in which a tool type of any kind is described completely: its identity, its model, its surface (a canvas for a diagram, a form-based surface for a designer, text for an editor), its behaviour, the bindings to the files it reads and writes, and the logic that hosts write per tool type today, in a small script that is part of the language. A tool type is described in one `.atd` file, or in a root file with the parts it imports: a **tool description**.

Today a tool engineer specifies a tool type in up to three JSON files, each in a language of its own: a specification in DISL or DESL, a binding in FBL, and a companion `.md` that lists in prose what neither language can say, which each host then writes as code. ATD states each concept once, in a core shared by every kind (section 7); puts the logic beside the declarations it serves (section 12); names what is too heavy or numeric for a script as a **capability** with a contract (section 13); and is defined by one grammar that is LALR(1), so that it can be checked as one unit, given one set of editing tools, and written by people and agents under a grammar that admits only valid text.

### 1.2 The phases of the move

- **End state.** The `.atd` text alone is normative for every tool type; the JSON forms of DISL, DESL, EDSL and FBL are retired as authored forms, and every host runs tools from `.atd`.
- **Phase 1, this version.** ATD is specified; the tool types in scope (29 diagram, 2 designer and 5 editor types) are described in `.atd` under `definitions/`, each with a trace (section 2.5) and conformance fixtures (section 2.3); the toolchain to write, check, edit, validate and debug descriptions exists. The `.dis`, `.des` and `.fbl` files stay the ones every host runs, unchanged. A host that finds a tool type both as `.atd` and as JSON files runs it from the JSON files and ignores the `.atd` when it loads tools.
- **Later phases.** Conversion of the JSON files, hosts running tools from `.atd`, and the retirement of the JSON languages as authored forms are features of their own.

### 1.3 Relation to the other specifications

DISL, DID, DESL, DED, EDSL, EDD and FBL remain specifications of this repository. ATD carries every construct of DISL 0.6, DESL 0.2 and FBL 0.8 (Appendix C maps each of their properties to the ATD construct that carries it), and the constructs an editor needs, which EDSL, a placeholder, does not yet have (section 10). ATD reuses their schemas: a setting of an ATD construct is checked against the `$defs` entry of DISL, DESL or FBL for the same thing (section 14.6), and a binding in ATD is an FBL binding written as text. Tools keep storing their documents in DID, DED or a body a binding reads; a tool description replaces none of them.

---

## 2. Conformance

### 2.1 Conformance classes

- A **description** conforms to ATD 0.1 when every file of it parses with the grammar of section 4 under the version it declares (section 5.4) and the checker of section 14 reports no finding of severity `error` for it.
- A **checker** conforms when, for every description, it reports exactly the findings of sections 14 and 15, by code, severity, file, line and column. The refused corpus beside this document (`refused/`, section 19.3) is the contract: a conforming checker gives each refused file exactly the findings its `.findings.json` states, and each example beside this document no finding.
- A **language server** conforms when it reports, for the same text, the same diagnostics as a conforming checker (FR-054), and offers the capabilities FR-051 of spec 035 lists (colouring, completion, definition, references, rename, diagnostics, hover, formatting and an outline) in the parts of a file that parse when another part does not.
- A **runtime** conforms when it runs a description that checks without errors, and, for every conformance fixture of that description, produces the readings, findings, splices, drawings and bytes the fixture states (section 2.3), with no code of its own for any tool type, apart from the capabilities of section 13.

### 2.2 Key words

The key words of the status section apply to every normative section. A requirement on "a reader" applies to a checker, a language server and a runtime alike.

### 2.3 Conformance fixtures

A **conformance fixture** is one case of one tool type: input files, the gestures made on them, and what the reference host produced for each. It extends FBL's round-trip fixture (FBL §15.3) with gestures, readings, findings, splices, drawings and the provenance of each part. A case is a folder `definitions/<kind>s/conformance/<type>/<case>/` holding `conformance.json` (`atd.schema.json#/$defs/ConformanceFixture`) and its files.

- `tool` is the tool type's origin; `description` the path of the root `.atd`, relative to the case's folder.
- `inputs` names the input files by role (`body`, `registration`, and for a folder body each file); they are kept byte for byte, and Git never converts them (`.gitattributes`: `-text`).
- `read` is what reading the inputs yields: the elements with their type, parent and the attributes compared; the relations with their ends; the findings (each with its rule or code and its line); and the parts the reader could not read.
- `drawing` is the scene after reading (section 2.4), inline or as `{"$ref": "<file>"}` beside the case.
- `steps` are the gestures, in order. A step has exactly one of `action` (with `args`, after `select`), `property`, `move`, `create`, `connect`, `delete`, `outside`, `undo` and `redo`, and states what it produces: `splices` (FBL `$defs/Splice`, UTF-8 byte offsets into the body before the step), the body's bytes after it (`expect` inline, or `expectFile`), `findings`, `drawing`, or `refused` with the reason the gesture was refused with. `ids` lists the ids the reference host generated in the step, in order; a runtime **MUST** be given them, in that order, in place of ids of its own, so that bytes can be equal.
- An `outside` step writes a file of the case as another program or an agent would, replacing the first occurrence of `from` in `file` by `to`, and tells the host the file changed, as its watcher does; the history starts afresh with it, so no step before it can be undone, and a refused step leaves the bytes as they were. A designer's gesture on its page is an `action` naming the operation, with the entry it is made on in `select` and the operation's parameters in `args`.
- An `undo` **MUST** give back the bytes before the step it undoes exactly, and a `redo` the bytes after it.
- `provenance` names, for each part (`bytes`, `reading`, `findings`, `drawing`), the host and repository that produced it, `commit`, the reference commit whose behaviour it records, `recordedWith`, for a recorded part the commit of the recorder, and `source`, a standalone host's golden or `atd record`. The reference host differs by part: the standalone host for bytes, as the desktop host's parity goldens record them, and the desktop host at its reference commit for readings, findings and drawings (spec 035, Q5). A tool type the desktop host did not have at its reference commit (the Hierarchical Requirement Designer, added by spec 036) takes its readings and findings from the desktop host at the commit that records them, which `commit` and `recordedWith` then both name. A designer's page has no recording painter, so a designer's case states no drawing.
- A fixture changes only by recording it again, never by hand. A runtime passes a fixture when every part of every step equals what the fixture states, bytes byte for byte; a difference is reported with its step, its part, the expected and the actual value (for bytes, the first line that differs) and the construct of the description that produced the value (section 15.1, `ATD9001`).

`fixtures/mindmap/basic/` beside this document is a worked example, written by hand for the example `mindmap.atd`: a read, one action and its undo.

### 2.4 Drawings

A **drawing** is the scene of one view after a reading or a step, in the vocabulary of the desktop host's scene (`adp-scene`), never pixels (`atd.schema.json#/$defs/Drawing`):

- `elements`: each drawn element with its `id`, `type`, `shape`, `bounds` (`x`, `y`, `w`, `h`), `labels` (text, the font token, position), `style` (paints resolved to colours, with the token names they came from), `children` and `state`;
- `edges`: each drawn edge with its `id`, `type`, `source`, `target`, `route` (points), `markers` and `labels`;
- `chrome`: bands, notices, legends, headers and rulers;
- `order`: the ids of everything drawn, in drawing order.

Positions and sizes are in the canvas's domain units, rounded to 0.01. Two drawings are **equal** when their elements and their edges are equal as sets keyed by `id`, their chrome is equal, and their `order` lists are equal. `fixtures/mindmap/basic/drawing.read.json` is a worked example.

### 2.5 Traces

A **trace** is the file `<name>.trace.json` beside a tool description (`atd.schema.json#/$defs/Trace`). It shows that the description is complete (FR-021 to FR-023):

- `companion` names the type's companion file and the sections of it whose items the description must state. An **item** is each `###` heading and each top-level list item under a named section, numbered from 1 in each section; its id is `<section-slug>/<n>`, the slug being the section's heading in lowercase with every run of other characters than letters and digits made one `-`.
- `items` lists every item exactly once, with its `text` (the item's heading or the start of its line), and either `statedBy`, the part of the description that states it, or `hostConcern`, one of section 17's concerns, with its `reason`.
- `hostCode` lists every behaviour read in a host's code at its reference commit: `host`, `repository`, `commit`, `path`, `lines` and a one-sentence `behaviour`, and either `statedBy` (the reference host's behaviour, which the description states), or `divergesFrom` (another host's behaviour that differs from the reference host's and is not stated; it names the `statedBy` of the reference behaviour the description states instead), or `hostConcern` with its `reason`.

A `statedBy` names a construct as `<keyword> <name>`, with the names of its enclosing constructs where one name is not enough (`fn siblingName`, `operation addSibling`, `binding freeplane element node`, `notation node Node label text`, `capability layout.twoSidedTree`, `algorithm mindmap`). Every `statedBy` and `divergesFrom` **MUST** resolve in the description (`atd check --trace`), and every item of the named sections **MUST** be traced exactly once (`validate-examples.py`). `mindmap.trace.json` with its companion `mindmap.md` is a worked example.

### 2.6 Capability contracts

A capability (section 13) is specified in `capabilities/<name>/` beside this document:

- `contract.json` (`atd.schema.json#/$defs/Capability`): its `name` (`<extension point>.<algorithm or format>`), its `version` (semver), its `extensionPoint`, its `input` and `output` (references to `$defs` of `atd.schema.json`), its `args` (each with its type, values, default and documentation), its `laws` (`deterministic`, `pinnedStay`, `stable`, `readingNeverFails`, `minimalSplices`, `pure`), its `exceptions` to the determinism of section 12.5 with their reasons, its `fallback` and its `status` (`provisional`, `standard`, `deprecated`);
- `fixtures/<case>.json` (`$defs/CapabilityFixture`): an `input`, the `args` and the expected `output`.

A host **MUST** pass every fixture of a capability before it offers it. `capabilities/layout.twoSidedTree/contract.json` is a worked example; its fixtures are recorded from the code that lays the mind map out today.

---

## 3. Lexical structure

### 3.1 Text and positions

An `.atd` file is UTF-8. A byte-order mark at its start is not part of any token and is kept; offsets count it. Lines end with LF or CRLF; a lone CR is whitespace. A position is reported as a 1-based line and a 1-based column counted in Unicode code points, as DISL §8.6 and FBL §2.6 require, and, where a range is given in bytes (`$defs/Span`, a fix's edits), as UTF-8 byte offsets into the file including the byte-order mark.

### 3.2 Comments and documentation

- `//` starts a comment that runs to the end of the line.
- `/*` starts a comment that runs to the first `*/`; comments do not nest.
- A comment that starts with `///` is a **documentation comment**. The documentation comments on the lines directly above a construct, with nothing but whitespace between them and it, are that construct's documentation: their text after `///` and one space, joined with line ends. A construct's `doc` member adds to it (section 7.3).

Comments and whitespace separate tokens and are otherwise ignored by the grammar; the formatter keeps each with the construct section 16.2 assigns it.

### 3.3 Terminal classes

Every token is of one class, which the language servers' semantic tokens and the generated highlighting grammars (`highlighting/`) use (FR-052):

| Class | Terminals | Examples |
|---|---|---|
| keyword | every string terminal that spells a name (Appendix B) | `element`, `label`, `create` |
| name | `NAME` | `Topic`, `siblingName` |
| key | `KEY` | `stroke:`, `color.surface:` |
| assignment | `ASSIGN` | `count =` |
| string | `STRING` | `"Mind map"`, `r"^\s*#"`, `"""…"""` |
| bytes | `BYTES` | `b"…"` |
| number | `INT`, `UINT`, `DOUBLE`, `VERSION` | `24`, `3u`, `1.5`, `0.1` |
| length | `LENGTH` | `12px`, `50%`, `1.5em` |
| attribute reference | `ATTR_REF` | `@text`, `@address.city` |
| token reference | `TOKEN_REF` | `$color.surface` |
| parameter reference | `PARAM_REF` | `#radius` |
| origin | `ORIGIN` | `freeplane/mindmap` |
| capability range | `CAPABILITY_RANGE` | `@^1` |
| word | `DASHED_WORD` | `y-down`, `after-last`, `unreadable-requirement` |
| operator and punctuation | every other string terminal | `&&`, `->`, `{`, `:` |
| comment | `COMMENT`, `BLOCK_COMMENT` | `// …`, `/* … */` |

### 3.4 Tokens

The terminals are defined in the grammar's modules (section 4.3); this section states what each means.

- **NAME** `[A-Za-z_][A-Za-z0-9_]*`: DISL's SimpleId, so that every name is a CEL identifier. A dotted name (`layout.twoSidedTree`) is names joined by `.` tokens.
- **KEY**: a name, or names joined by `.`, each of which may hold `-`, followed by spaces or tabs and `:`, as one token: `stroke:`, `x-acme:`, `color.surface:`. A setting's key is the token without its colon and the spaces before it. Taking the key and its colon as one token is what lets a setting stand among a construct's members, whatever the key spells (section 6.4).
- **ASSIGN**: a name followed by spaces or tabs and `=`, as one token: the start of an assignment to a local variable (section 12.2).
- **STRING**: CEL's string literal, in all its forms: quoted with `"` or `'`, with CEL's escapes; tripled (`"""…"""`, `'''…'''`) across lines; or raw, prefixed with `r` or `R`, with no escapes. A string's value is CEL's.
- **BYTES**: CEL's bytes literal, in CEL expressions only.
- **INT**, **UINT**, **DOUBLE**: CEL's numeric literals. A value position also admits a number with a leading `-` (section 6.1).
- **LENGTH**: a number in any of CEL's forms followed, with no space, by a unit of DISL §2.6: `px`, `mm`, `in`, `em` or `%`.
- **ATTR_REF** `@path`, **TOKEN_REF** `$name`, **PARAM_REF** `#name`: references to an attribute, a theme token and a parameter (section 6.1).
- **ORIGIN** `<vendor>/<type>`: a tool type's origin, offered only where an origin is expected (section 5.1).
- **CAPABILITY_RANGE**: `@` followed by a semver range (`@^1`, `@~1.2`, `@>=1.0`), directly after a capability's name (section 13.2).
- **DASHED_WORD**: a bare word with hyphens (`y-down`, `remove-when-empty`): a literal (section 6.1), and, where a binding declares or names a binding, a rule, a block, a section or a file rule, a name (section 11.3).
- **VERSION** `<major>.<minor>`: the language version in the header (section 5.1).

### 3.5 The regular expression subset

The terminals of the grammar, and of every grammar a binding carries (section 11.9), are written in FBL §2.5's common subset of regular expressions, without what a token has no use for and what engines read differently:

- **Admitted**: literals, character classes `[…]` and `[^…]`, the escapes `\d \D \s \S \w \W \t \n \r \\` and escaped punctuation, groups `(…)` and non-capturing groups `(?:…)`, alternation `|`, and the greedy quantifiers `* + ? {n} {n,} {n,m}`.
- **Not admitted**: anchors `^` and `$`; named groups `(?<name>…)`; lazy and possessive quantifiers; backreferences, lookaround, atomic groups, inline flags and Unicode property classes; and `.` outside a character class, since engines differ on which line ends `.` matches (write the class: `[^\n]`).
- `\d`, `\w` and `\s` and their negations mean their ASCII classes: `[0-9]`, `[A-Za-z0-9_]` and `[ \t\n\r\f\v]`.
- An expression matches the text the leftmost-first semantics of RE2 gives at the position, which the other engines agree with for this subset.

`atd-grammar.py --lint` refuses every terminal outside the subset.

### 3.6 The lexing rule and soft keywords

The lexer is **contextual**. At each position, the candidates are the terminals the parser's current state can shift, with the ignored ones (whitespace and comments); among the candidates that match:

1. the longest match wins;
2. of matches of the same length, the terminal of the higher priority (no terminal of ATD 0.1 states one, so all are of priority 0);
3. of those, a string terminal wins over a pattern terminal.

**Keywords are soft** (FR-008). A keyword is a string terminal that spells a name. By rule 3 it wins over `NAME` only where the state can shift it; elsewhere the same word is a `NAME`. Where the grammar expects a name, it expects the rule `name`, which admits `NAME` and every keyword (Appendix B), so a keyword can always stand as a name. A setting's key and an assignment's target are tokens of their own (`KEY`, `ASSIGN`), so any word, keywords included, can be a key or a local variable. The names of a binding and of its rules, blocks, sections and file rules are FBL's (FBL §2.2), which admit `-` and `.`: where a binding declares or names one, the grammar expects the rule `rule_name`, a `name` or a `DASHED_WORD`, or several joined by `.` (`unreadable-requirement`, `by-id.v2`). `atd-grammar.py --probe` puts every keyword in the place of every name of the examples and checks that the parser reads it as that name.

**CEL's reserved words** (`as`, `break`, `const`, `continue`, `else`, `false`, `for`, `function`, `if`, `import`, `in`, `let`, `loop`, `namespace`, `null`, `package`, `return`, `true`, `var`, `void`, `while`) stay reserved inside CEL, as CEL requires: the rule `word`, which CEL's identifiers use, lists every keyword but those, and the checker refuses one of them used as an identifier in an expression (`ATD4003`). Outside CEL they are names like any other keyword; `true`, `false` and `null` are not keywords of ATD at all, but bare words, which section 6.6 reads as JSON's literals.

A later version that adds a keyword does not change how an existing file is read: a file is read with the grammar of the version it declares (section 18).

---

## 4. ATD grammar notation

ATD's grammar is written in **ATD grammar notation**, a subset of the grammar language of Lark (https://github.com/lark-parser/lark), stated here so that it can be implemented without Lark. The same notation is used by a binding that carries a grammar (section 11.9). `grammar/notation.lark` is the notation written as a grammar; every module of ATD's grammar reads under it.

### 4.1 The subset

| Admitted | Not admitted |
|---|---|
| a rule `name: expansion \| expansion`, and a terminal `NAME: "x" \| /re/` | templates `_rule{x}` |
| `?rule` (inlined when it has one child), `_rule` and `_TERMINAL` (filtered from the tree) | rule priorities `rule.2` |
| `x?`, `x*`, `x+`, `[x]`, `( … )` | `%declare`, `%override`, `%extend` |
| an alias `-> name` | `%import` of anything outside the language's modules (in a binding, outside the binding's own files beside it) |
| a terminal priority `TERMINAL.2` | the flags of strings and regular expressions (`"x"i`, `/x/i`) |
| `%ignore TERMINAL` | terminals outside the subset of section 3.5 |
| `%import .module (names)` | keep-all-tokens rules `!rule`, ranges `"a".."z"`, repetitions `x ~ 3` |

Rule names are lowercase, terminal names uppercase. Comments are `//` to the end of the line. A definition starts with its name and colon and runs to the next definition; an alternative may continue on the next line after `|`.

### 4.2 Meaning

- A rule's expansions are its alternatives; an expansion is a sequence of items; an item is a rule, a terminal, an anonymous string (a terminal whose pattern is that string), an anonymous regular expression, or a group.
- `x?` and `[x]` are `x` or nothing; `x*` is any number of `x`; `x+` one or more; `( … )` groups alternatives.
- A terminal is a string, a regular expression, or a sequence and choice of other terminals; it matches as section 3.6 says.
- `%ignore T` makes `T` a candidate in every state and drops what it matches.

### 4.3 Modules and imports

The grammar of ATD 0.1 is the root module `grammar/atd.lark` and the modules it imports: `core`, `cel`, `script`, `canvas`, `form`, `text`, `binding` and `notation`. Every module's symbols share one grammar:

- In the root, `%import .m (a, b, …)` makes the symbols `a`, `b` of module `m` the root's own.
- A module uses symbols of other modules by name, without an import of its own; the root's import list for that module names each such symbol, which then resolves to the root's symbol of that name. Every terminal a module defines is in the root's import list for it.
- Every other symbol of module `m` is private to it, and is named `m__symbol` (`_m__symbol` for a symbol that starts with `_`) in the grammar the modules compose.

These are the rules of Lark's `%import`, so that Lark builds the grammar from `atd.lark` directly (contracts/grammar.md, rule 1).

### 4.4 From notation to rules

An implementation builds its tables from the grammar after this expansion, which is Lark's, so that two implementations build the same automaton:

1. A group of alternatives inside an expansion is distributed: `a (b | c) d` becomes the two expansions `a b d` and `a c d`.
2. `x?` and `[x]` become two expansions, one with `x` and one without.
3. `x+` becomes a new rule `H: x | H x`, and `x` is replaced by `H`.
4. `x*` becomes the same rule `H`, and two expansions, one with `H` and one without.
5. Two repetitions of the same item share one rule `H`, wherever they stand.
6. Duplicate expansions of a rule are kept once.
7. The start symbol is `description`. The automaton is augmented as an LR automaton is: the state the start state reaches on `description` accepts at `$END`; the augmenting rule is not one of the grammar's rules.

### 4.5 The parse tree

A conforming parser need not build Lark's tree; ATD's own tree is the abstract tree of `$defs/Tree`, which an implementation lowers from the parse (section 5.5). For a grammar a binding carries, the tree is shaped as Lark shapes it, since a binding's selectors name its nodes (section 11.9): a node per rule, named by the rule or its alias; `?rule` replaced by its child when it has one; `_rule` replaced by its children; `_TERMINAL` and anonymous strings dropped; and `[x]` that matched nothing kept as an empty child.

### 4.6 The flattened grammar

`grammar/atd.flat.lark` is the whole grammar in one file, generated by `atd-grammar.py --flatten`: the modules' definitions with the names of section 4.3, no `%import`, the root's `%ignore`s, and the rule `start: description` that model services look for. It builds the same tables as the modules, and LLGuidance's Lark front end, the context-free engine behind grammar-constrained output in the major model services, accepts it (FR-090). It is the file to hand an agent (section 19.4).

### 4.7 Parse tables

`grammar/atd.tables.json` (`$defs/ParseTables`) is the LALR(1) automaton of the grammar, normalized so that two implementations can be compared byte for byte: terminals by name, an anonymous string terminal named by its quoted string; the rules after section 4.4, sorted and numbered, the rule `H` of a repetition named `(<alternatives>)+`, its expansions that do not start with `H`, sorted and joined with ` | `; and the states numbered breadth first from the start state, visiting the shifted symbols in sorted order, each with its actions on terminals (`s<state>`, `r<rule>`, `accept`) and its gotos. `atd-grammar.py --tables` writes it from Lark; `atd tables` writes it from another implementation. The grammar builds with no shift/reduce and no reduce/reduce conflict, and no two terminals a state accepts match the same text (FR-002, SC-001).

---

## 5. Files and descriptions

### 5.1 The header and the tool construct

Every `.atd` file **MUST** start with the header `atd <major>.<minor>`, the version of ATD it is written in. It is the one construct every version of the grammar keeps (section 18).

A **root** file follows its header with the tool construct, which names the kind, the origin and the version of the tool type and holds its members:

```text
atd 0.1
diagram freeplane/mindmap version "1.4.0" {
  label "Mind map"
  …
}
```

- The kind is `diagram`, `designer` or `editor`, and decides which constructs the description may hold (section 14.4).
- The origin is the tool type's `<vendor>/<type>` (DISL's and DESL's `language.origin`). A description of a tool type that has a `.dis` or `.des` **MUST** have its origin, and its `label` **MUST** be its display name (FR-025).
- `version` is the tool type's semantic version (DISL's `language.version`).
- The members are the constructs of sections 7 to 13, in any order; their order is kept (section 16).

### 5.2 Parts

A description **MAY** be split over several files (FR-012). One is its root; each other is a **part**, which follows its header with `part of <origin>` and holds members as the tool construct does, without braces:

```text
atd 0.1
part of freeplane/mindmap

notation { … }
layout { … }
```

A part's members are members of the description. A part **MUST** name the origin of the root that imports it (`ATD2007`), and **MUST NOT** be imported by two roots (`ATD2008`).

### 5.3 Imports

- `import "<path>"` in a root or a part takes the part at `<path>` into the description. The path is relative to the importing file and **MUST** stay within the description's folder (`ATD2009` otherwise, as for a file that is not there).
- Imports **MAY** form any graph; a cycle among them is reported once, at the import that closes it as the files are visited from the root in the order of their imports (`ATD2003`), and the description is still checked.
- `import "<path>" as <name>` takes another tool description as a library, as DISL §3.3's `imports` do: its element types, functions and other declarations are visible prefixed with `<name>.`. Its settings are DISL's `Import` (`version`, `include`, `integrity`).

### 5.4 Versions

A reader **MUST** read the header before anything else and select the grammar of the version it declares. A version it does not know is refused with a finding that names both versions (`ATD0001`), never read as the nearest one it knows. A description is checked with the grammar and the rules of the version it declares, so a keyword a later version adds never reaches an earlier file.

### 5.5 The abstract tree

A reader lowers what it parses into the **abstract tree** of `atd.schema.json#/$defs/Tree`, which `atd tree` prints: per file, its version and its tool or part, each construct a node with its `kind`, its `span` (section 3.1), its `name`, its `doc` and its fields, each value as section 6.6 says and each expression with CEL's abstract syntax. The abstract tree is not an interchange form (FR-030): nothing reads it back as a description, and the `.atd` text is normative.

---

## 6. Values

### 6.1 The value rule

Every value position follows one rule (FR-006):

| Written | Is |
|---|---|
| a bare word (`center`, `remove-when-empty`), a number (`24`, `-8`, `1.5`), a length (`12px`, `50%`) or a string (`"Mind map"`) | a **literal** |
| `( <CEL> )` | a **CEL expression** |
| `@path` | the value of an **attribute** of the current element; two-way where the position is editable, as DISL's `{attribute}` |
| `$name` | a **theme token** (DISL §6.2) |
| `#name` | a **parameter** of the enclosing shape, marker or template |
| `{ key: value … }` | a **settings map** (section 6.4) |
| `[ value, … ]` | a **list** |

The marks `@`, `$` and `#` are unused by CEL, and parentheses are CEL's own grouping, so no CEL text is ambiguous with them.

### 6.2 Positions that take only an expression

A position that only ever takes an expression takes CEL **without** parentheses: a constraint's `when` and `rule`, an operation's `enabled`, a hook's `when`, a variant's `when`, a binding rule's `when`, a script's conditions and right-hand sides, and the arguments of actions. Such an expression ends where the next construct starts; it is never directly followed by `{` that is not its own, which is why CEL's message construction is not part of ATD (section 6.3).

### 6.3 CEL in ATD

Expressions are CEL (FR-007), and the grammar contains CEL's expression syntax (`grammar/cel.lark`), so that an expression is parsed, coloured and type-checked as part of the description:

- the conditional `?:`, `||`, `&&`, the relations `< <= >= > == != in`, `+ -`, `* / %`, `!` and unary `-`, member selection, indexing, calls, lists and maps, with CEL's precedence;
- every literal: `int`, `uint`, `double`, `string` and `bytes` in all their forms, `true`, `false`, `null`;
- CEL's optional syntax: `a.?b`, `a[?i]`, `[?x]` and `{?k: v}`;
- the macros (`has`, `all`, `exists`, `exists_one`, `map`, `filter`, `cel.bind`) as calls.

CEL's **message construction** (`Name{field: value}`) is not part of ATD: no ADP CEL environment declares a message type (DISL §12.2, DESL §9.2), and leaving it out lets a bare expression be followed by a block (`if cond { … }`). The abstract tree holds an expression with CEL's abstract syntax (`$defs/CelNode`): an operator as a call of its CEL function name (`_+_`, `_&&_`, `!_`, `-_`, `_?_:_`, `_[_]`, `@in`), a macro as a call marked `macro`.

### 6.4 Settings

The open vocabulary of DISL, DESL and FBL (strokes, fonts, layout options, persistence details, extension `x-` keys and the many properties ATD gives no keyword of its own) is written as **settings**: `key: value` pairs, among a construct's members or in a settings map `{ … }`, whose values follow the value rule, so that they nest (FR-011).

```text
stroke: { color: $color.border, width: 1.5 }
padding: [6, 10]
size: { width: (math.round(textWidth(self.text, 14.0) + 20.0)), resizable: false }
```

- A key is a `KEY` token (section 3.4), or, in a settings map, a string followed by `:` for a key that is not a name.
- A comma between two settings is optional.
- The grammar knows only the form. Which keys a construct admits, and what their values may be, is checked against the JSON Schema of the construct (section 14.6), so that a stroke has one schema, DISL's, whether it is written in JSON or in ATD.
- An `x-` key is admitted everywhere and not checked.

### 6.5 DISL's four embeddings

DISL embeds CEL four ways (DISL §2.5); in ATD each is the value rule:

| DISL | ATD |
|---|---|
| (a) *Expression*: a string that is CEL, or `{"cel": …}` | in a position of section 6.2, bare CEL; in a setting, `( CEL )` |
| (b) *Bindable*: a literal, `{"cel"}`, `{"attribute"}`, `{"token"}` or `{"param"}` | a literal, `( CEL )`, `@path`, `$name` or `#name` |
| (c) *GeomExpr*: a number, a percentage string or a string of CEL | a number, a length (`50%`), or `( CEL )` |
| (d) the *action expression*, in which a literal string is written `"'Untitled'"` | statements (section 12), whose arguments are CEL: `"Untitled"` |

None of the four survives as a form of its own.

### 6.6 The JSON value of a value

A value has a JSON value, which is what its setting is checked against and what the runtime hands DISL's, DESL's and FBL's engines (section 14.6):

- a bare word is a JSON string, except `true`, `false` and `null`, which are JSON's literals; a number is a JSON number; a length and a string are JSON strings;
- `( e )` is, in a property the schema types as an *Expression*, the string `e`; in any other property, `{"cel": "e"}`;
- `@p`, `$n` and `#n` are `{"attribute": "p"}`, `{"token": "n"}` and `{"param": "n"}`;
- a settings map is a JSON object with its keys in written order, and a list a JSON array.

A literal in a property the schema types as an *Expression* is refused (`ATD5003`): an expression is written as one.

---

## 7. The core

The core holds what every kind shares, stated once (FR-004): identity and origin, localized text and documentation, element types and attributes, data types and enums, ids, containment, constraints and findings, operations, hooks, actions, reasons and messages, limits and settings (`grammar/core.lark`). Each construct carries the DISL or DESL construct this section names.

### 7.1 Members of a tool

A tool construct, and a part, holds any of: `label`, `doc`, settings, `import` (section 5.3), `requires capability` (section 13), `model`, `constraints` and `constraint`, `operation`, `hook`, `reason`, `message`, `behavior`, `persistence`, `plugin`, `fn` (section 12), the constructs of its kind's surface (sections 8 to 10) and `binding`s (section 11). The tool construct's settings are DISL's `Language` and DESL's `Language` (`id`, `defaultLocale`, `locales`, `icon`, `authors`, `homepage`, `license`, `limits`, `fileExtension`, `profiles`).

### 7.2 Labels and localized text

`label <value>` is a construct's display name (DISL §2.4): a string, or a settings map from BCP 47 tags to strings (`{ en: "State", de: "Zustand" }`), or, in a position DISL types as a Message, `( CEL )` returning a string or a map of strings. A construct without a label has the one DISL derives from its name.

### 7.3 Documentation

A construct's documentation is its `///` comments (section 3.2) and its `doc <value>` member: a string, the summary, or a settings map of DISL's `Doc` (`summary`, `description`, `rationale`, `examples`, `seeAlso`, `tags`, `audience`, `since`, `deprecated`, `image`, `helpUrl`). The language servers show it on hover with the construct's entry of Appendix B.

### 7.4 The model

`model { … }` holds the tool type's metamodel (DISL §4, DESL §4): `element`, `relation`, `enum`, `datatype` and `document`, and settings of DISL's `Metamodel`.

### 7.5 Element types, relation types and ports

- `element <Name> [extends <Types>] { … }` is an element type: DISL's node type, DESL's element type. Its members are `attribute`, `contains`, `port`, `label`, `doc` and settings of DISL's `NodeType` (`abstract`, `icon`, `labelAttribute`, `multiplicity`, `bounds`, `tags`, `viewOnly`, `derived`).
- `relation <Name> [extends <Types>] [from <Types> to <Types>] { … }` is a relation type (DISL §4.9), admitted in diagrams only. Its members are those of an element type; its settings are DISL's `RelationType` (`directed`, `allowSelfLoops`, `allowParallel`, `acyclic`, `derived`, `connectsRelations`), and `source` and `target` settings give the rest of each end (DISL's `RelationEnd`).
- `port <name> { … }` is a port type of an element type (DISL §4.10).
- Inheritance is C3, as DISL §4.7 says, for element types and relation types alike.

### 7.6 Attributes

`attribute <name>: <Type> [required] [many] [readonly] [default <value>] { … }` is an attribute (DISL §4.3, DESL §4.3). The type is one of DISL's or DESL's primitive types, an enum, a data type, an element type (a reference), or a CEL parameterised type (`list<Concept>`). Its settings are DISL's `Attribute` (`label`, `min`, `max`, `pattern`, `markup`, `group`, `order`, `unit`, `readOnlyReasons` and the others) and DESL's (`mentions`, `timezone`).

### 7.7 Enums and data types

- `enum <Name> ["<label>"] { <value> … }` is an enumeration (DISL §4.5). Each value is a name, or a string where it is not one, with an optional block of DISL's `EnumValue` (`label`, `color`, `icon`, `value`, `deprecated`); the enum's other settings (`extensible`, `ordered`) stand among its values.
- `datatype <Name> [: <Base>] { … }` is a data type (DISL §4.4).

### 7.8 Containment and the document

- `contains <Types> [as <slot>] [ordered] { … }` says which element types an element type contains (DISL §4.8, DESL §4.6): without `as`, in its children; with `as`, in the named slot. Its settings are DISL's `Containment` (`min`, `max`, `perType`, `acrossViews`).
- `document { … }` holds the attributes of the document itself: DISL's `metamodel.diagram`, DESL's `metamodel.document`.

### 7.9 Constraints and findings

- `constraint <id> [for <Types>] { … }` is a constraint (DISL §8.2, DESL §6.2): `when <CEL>`, `rule <CEL>`, `severity <name>`, `message <value>`, `fix <name> ["<label>"] { … }` quick fixes (DISL §8.5) whose statements are actions, `label`, `doc`, and settings of DISL's `Constraint` (`kind`, `code`, `attribute`, `target`, `enforcement`, `timing`, `group`, `subject`, `location`, …).
- `constraints { … }` holds constraints, `builtin <std.name> { … }` settings of the built-in constraints (DISL §8.7, DESL §6.5), and settings of DISL's `Constraints` (`blockSaveOn`, `defaults`, `groups`, `order`).
- A finding of a running tool is DISL's (DISL §8.6); a finding of ATD's checker is section 15's.

### 7.10 Operations

`operation <id> ["<label>"] [for <Types>] { … }` is an operation (DISL §9.3, DESL §8.1). Its members are `enabled <CEL>`, `unavailable <value>` (a list of reasons), `param <name>: <Type> …`, `confirm { … }` (DISL's `Confirmation`), `label`, `doc`, settings of DISL's `Operation` (`icon`, `shortcut`, `paramsForm`, `simulate`), and statements (section 12), which are its actions, run in the operation's transaction. `for document` makes it an operation of the document.

### 7.11 Hooks

`hook <id> on <event> [for <Types>] { … }` is a hook (DISL §9.2): `when <CEL>`, settings of DISL's `Hook` (`phase`, `order`, `attribute`), and statements. Hooks are admitted in diagrams only, as DESL has none.

### 7.12 Reasons and messages

- `reason <id> "<text>"`, or `reason <id> { when <CEL>  message <value> }`, declares a named reason (DISL §2.3, §9.1's `reasons`). A list of reasons, wherever DISL has one (`unavailable`, `readOnlyReasons`, refusals), is a list of values: a message, a settings map `{ when: (…), message: … }`, or `{ reason: <id> }`.
- `message <std.name> <value>` sets a standard message (DISL §9.1's `messages`).
- `behavior { … }` holds the rest of DISL's `Behavior` as settings: `deletion`, `clipboard`, `retype`, `undo`, `editGate`, `placements`, `maxHookDepth`.

### 7.13 Persistence and ids

`persistence { … }` holds DISL's and DESL's `Persistence` as settings, ids and id strategies among them (`ids: { strategy: …, prefix: …, types: { … } }`, DISL §11.5, DESL §5.3, the `counter` strategy with its `mark` among them), the part of the model a registration keeps (`settings`, DESL §5.5), and the type map, view data and migrations. The type map (`typeMap`) names the binding types that are not types of the model, as DESL §5.2 does: a binding rule may name a key of it as its type (section 14.7). A binding is a `binding` of the description (section 11), never a reference to a file.

### 7.14 Plugins

`plugin <name> { … }` declares a DISL plugin (DISL §13.1). In ATD, an algorithm the platform provides is a capability (section 13); a plugin declaration carries a DISL specification's plugin to its description, until the capability that replaces it is in the catalogue.

### 7.15 Limits

The tool construct's setting `limits` holds DISL's `language.limits` and the script's budget (section 12.6): `limits: { steps: 50000 }` sets a lower budget than the default.

---

## 8. The canvas surface

The surface of a diagram (`grammar/canvas.lark`): its coordinates, notation, toolbox, forms, layout and viewpoints, each carrying the DISL construct named here (FR-003).

### 8.1 Coordinates and snapping

`coordinates { … }` (DISL §5.2) holds `system <name> { … }` (DISL's `CoordinateSystem`, §5.7), `axis <name> { … }` (DISL's `Axis`, §5.3 to §5.6), `snapping <name> { … }` (a snapping profile, DISL's `Snapping`, §5.9 and §5.10), `default <system>`, and settings of DISL's `Coordinates`.

### 8.2 Placement

A node's placement (DISL §5.8) is its setting `placement: { … }` (DISL's `Placement`), whose coordinates are values: a literal, `@attribute` for a two-way binding to an attribute, or `( CEL )`. A node's snapping is its setting `snapping`.

### 8.3 Notation, themes, styles and definitions

`notation { … }` (DISL §6) holds:

- `theme { … }`: DISL's `Theme` (`tokens`, `modes`, `defaultMode`, `followSystem`, `contrast`), its tokens keyed by dotted names (`color.surface: "#FFFFFF"`);
- `style <name> [extends <names>] { … }`: a named style, DISL's `Style`;
- `shape <name> { … }`, `marker <name> { … }`, `icon <name> { … }`: custom shapes, markers and icons (DISL's `ShapeDef`, `MarkerDef`, `IconDef`);
- `canvas { … }` and `defaults { … }`: DISL's `Canvas` and the notation's defaults;
- `node`, `edge` and `port` notations (sections 8.4, 8.5);
- settings of DISL's `Notation` (`textMetric`, `fonts`).

### 8.4 Node notation

`node <Types> { … }` is the notation of element types (DISL §6.9):

- `shape <value> [{ … }]`: the shape, a built-in or defined one, with its parameters (DISL's `ShapeInstance`);
- `label <value> [as <id>] [{ … }]`: a label, its text a value, its id after `as`, and DISL's `Label` as settings (`editable`, `position`, `wrap`, `placeholder`, …);
- `variant <name> [when <CEL>] { … }`: a variant (DISL's `NodeVariant`), with the members of a node notation;
- `state <name> { … }`: the style of a state (`hover`, `selected`, `dropTarget`, …, DISL's `States`);
- `compartment <id> { … }`, `badge <id> { … }`, `port <Types> { … }`: DISL's `Compartment`, `Badge` and `PortNotation`;
- settings of DISL's `NodeNotation` (`style`, `size`, `anchors`, `container`, `icon`, `deletable`, `refusals`, `doubleClick`, `lod`, `tooltip`, …), and of DISL's `Style` (`fill`, `stroke`, `font`, `padding`, …): a setting that is a property of `Style` and not of `NodeNotation` is a property of the node's inline style, its `style` when it names no style, or the style it adds to the one it names.

### 8.5 Edge notation

`edge <Types> { … }` is the notation of relation types (DISL §6.10): `line { … }` (DISL's `LineSpec`), `marker source|target|mid <value> [{ … }]` (DISL §6.11), `label`, `variant`, `state`, and settings of DISL's `EdgeNotation` (`selectable`, `deletable`, `reconnectable`, `layer`, `anchoring`, `stub`, …) and, as for a node, of its inline `Style`.

### 8.6 Toolbox and context tools

`toolbox { … }` (DISL §7.1 to §7.4) holds `group <id> ["<label>"] { … }` (nested groups and tools, DISL's `ToolGroup`), `tool <id> ["<label>"] { … }` (DISL's `Tool`), `context [for <Types>] [when <CEL>] { … }` (a context tool set, DISL's `ContextToolSet`, whose `tool [<name>] ["<label>"] { … }` entries are DISL's `ContextTool`), `template <id> ["<label>"] { … }` (DISL's `Template`), and settings of DISL's `Toolbox` (`generic`, `position`, `searchable`, `starters`). A context tool has no id in DISL, so its name is optional and its block is not.

### 8.7 Forms

`form <id> ["<label>"] [for <Types>] { … }` is a form (DISL §7.5): `field <attribute> ["<label>"] { … }`, `section [<id>] ["<label>"] { … }` holding fields, sections and items, and `item [<id>] ["<label>"] { … }` for every other kind of form item, each with DISL's `FormItem` as settings; and settings of DISL's `Form`. Forms are admitted in diagrams and designers (section 14.4). An embedded form is a node's setting `form` (DISL §7.6).

### 8.8 Layout

`layout { … }` (DISL §10) holds `default <name>`, `algorithm <name> [capability <name>] { … }`, and settings of DISL's `Layout` (`animate`, `incremental`, `respect`, `trigger`, `containers`). An algorithm is either one of DISL's (its setting `algorithm: tidyTree`, `layered`, `force`, `rowPacked`, `rows`, with DISL's `LayoutConfig`), or a capability of the `layout` extension point, named after `capability` and given its arguments as settings (section 13.3).

### 8.9 Viewpoints

`viewpoint <id> ["<label>"] { … }` is a viewpoint (DISL §3.5, DISL's `Viewpoint`).

---

## 9. The form surface

The surface of a designer (`grammar/form.lark`): which element types and attributes play which part of the layout (DESL §7). A host's component for a kind of surface serves all designer types of that kind.

### 9.1 The table

`surface table { … }` (DESL §7.2 to §7.4) holds:

- `rows <Type> { … }`: the rows' element type;
- `columns <Type> { … }`: the columns' element type, with the attributes that play `name`, `valueType`, `title`, `parent` and `options` as settings (DESL's `Columns`);
- `cells <Type> { … }`: the cells' element type, with `column` and `items` (DESL's `Cells`);
- `views <Type> { … }`: the views' element type, with `name`, `active` and `settings` (DESL's `Views`);
- `valueType` (section 9.2), and settings.

### 9.2 Value types and conversions

`valueType <key> ["<label>"] { … }` is a value type (DESL §7.5): settings of DESL's `ValueType` (`key`, `many`, `editor`, `comparisons`, `sort`), and `convert <type> <conversion>` for each conversion to another value type (DESL §7.6: `written-form`, `parse`, `match-option`, `wrap`, `unwrap`, or a settings map).

### 9.3 The hierarchy

`surface hierarchy { … }` (DESL §7.7) holds `levels <Type> { … }`, `entries <Type> { … }`, `notes <Type> { … }`, `filters <Type> { … }`, `widths <Type> { … }` and `labels <attribute> { … }`, each with the attributes that play its parts as settings (DESL's `HierarchyMembers`, `Notes`, `Filters`, `Widths` and `Labels`), and settings of DESL's `Hierarchy` (`collapsed`, `ids`). `widths` names the element type that keeps the widths of the columns of levels, which the user changes by dragging (DESL §7.7.1): its settings `level` and `width` name the attributes of a column's depth and of its width in pixels.

---

## 10. The text surface

The surface of an editor (`grammar/text.lark`). EDSL is a placeholder, so this section is written from what the editors the hosts implement do: the five editors of the desktop host at its reference commit `fe5feec`, which port the standalone host's.

### 10.1 What the editors do

| Behaviour | Editor | Read in (`etalii.adp.ide.desktop` at `fe5feec`) |
|---|---|---|
| A buffer with one cursor and a selection; undo grouped word by word; movements by character, word, line and document; typed text, paste (CRLF made LF), copy, cut, Tab as four spaces | all | `crates/adp-text/src/lib.rs` 1-355, `crates/adp-shell/src/editor.rs` 253-320 |
| A line-number gutter, the current line marked, no soft wrap | all | `crates/adp-shell/src/editor.rs` 65-251 |
| Findings drawn as squiggles by severity, and listed in the problems panel at line and column (1-based, columns in characters) | all | `crates/adp-shell/src/editor.rs` 33-47, 196-213; `crates/adp-shell/src/problems.rs` 14-58 |
| Colouring by token classes, from the syntax of the file's extension (syntect's Sublime syntaxes), none above 20,000 lines | all | `crates/adp-shell/src/highlight.rs` 16-54 |
| Files opened: UTF-8 or ASCII, refused above 5 MB or when binary or not UTF-8; each line keeps its line ending, new lines take the dominant one; the byte-order mark is kept | all | `crates/adp-documents/src/text.rs` 1-124 |
| Which editor opens a file: by its extension (`md`, `markdown`; `json`; `xml`, `xsd`; `yaml`, `yml`; anything else plain text) | all | `crates/adp-shell/src/documents.rs` 21-85 |
| An outline of the ATX headings by level, outside fenced code | Markdown | `modules/editors/markdown/src/lib.rs` 16-48 |
| A preview beside the text: CommonMark with tables, strikethrough and task lists | Markdown | `modules/editors/markdown/src/lib.rs` 85-225 |
| Findings of a tolerant parser (fourteen messages, a duplicate key a warning) | JSON | `modules/editors/json/src/parse.rs` 93-396 |
| Validation against the JSON Schema the root's `$schema` names, read from the project only | JSON | `modules/editors/json/src/lib.rs` 1-273 |
| Completion from that schema (property names not yet present, `enum` and `const` values) | JSON | `modules/editors/json/src/lib.rs` 275-335 |
| Well-formedness, the declared encoding, and validation against the XSD or DTD the document names | XML | `modules/editors/xml/src/lib.rs` 1-306 |
| Parser findings per document of a stream, tabs as indentation, duplicate keys, aliases, and validation against the schema a `yaml-language-server` comment names | YAML | `modules/editors/yaml/src/lib.rs` 1-521 |

No editor folds, completes in the shell, offers quick fixes or opens links at the reference commit; the constructs below state those an editor type may declare, so that a description can state what an editor of its type does where hosts do it.

### 10.2 Constructs

`surface text { … }` holds:

- `files "<glob>", …`: the files the editor opens;
- `tokens <syntax> { … }`: the token classes come from the named syntax (`markdown`, `json`, …); `token <class> { … }` declares a class of its own, with its `pattern` (section 3.5) and `scope`;
- `fold <Types> { … }`: the element types of the text whose ranges fold;
- `outline <Types> [by <attribute>] { … }`: the element types the outline shows, nested by an attribute (a heading's `level`);
- `complete from <source> { … }`: a completion source, a capability (`capability <name>`) or a source the host names (`"schema"`);
- `check with <source> { … }`: a check whose findings the editor shows, a capability or a named source (`"schema"`, `"xsd"`);
- `preview <source> { … }`: an embedded preview, a capability of the `render` extension point;
- `fix <name> ["<label>"] { … }`: a quick fix on a range, its statements actions on the text's model;
- settings.

The text's model is the description's model: its element types are what the editor's binding reads from the text (headings, keys, elements). The origins of the five editor types are those of the desktop host's catalogue: `editor/plain`, `editor/markdown`, `editor/json`, `editor/xml`, `editor/yaml`.

---

## 11. Bindings

A binding (`grammar/binding.lark`) is an FBL 0.9 binding written as text (FBL §3 to §13): every property of FBL's `Binding` has its ATD form in Appendix C. Its meaning, reading and writing are FBL's, unchanged.

### 11.1 The binding

`binding <name> [family <family> | family capability <name>] { … }`:

- `family <family>` is the family of a declared file body (FBL §4): `yaml`, `json`, `xml`, `lines`, `blocks`, `bullets` or `grammar` (section 11.9);
- `family capability <name>` reads and writes the body through a capability of the `reader` extension point, under FBL §11's persistence plugin contract (FBL's `reader` as a `PluginReader`); its arguments are the binding's setting `args`;
- a binding without `family` has a folder body (FBL §10), described by `body { kind: folder … }`;
- `label` is FBL's `title`.

A description **MAY** have several bindings, one per format family, as DESL's `persistence.bindings` lists them.

### 11.2 Claims, body, text and header

`claims { … }` (FBL's `Claims`), or `claims files "<glob>", …` for its `names`; `body { … }` (FBL's `Body`), `text { … }` (FBL's `TextDefaults`), `header { … }` (FBL's `Header`), and settings of FBL's `Binding` (`comment`, `unmatched`, `readOnly`).

### 11.3 Rules, slots and attribute bindings

`element <Type> [as <name>] [at "<selector>" | line "<regex>"] { … }` is an element rule (FBL §5.1), `relation <Type> [as <name>] […] { … }` a relation rule (FBL §5.4). The name after `as` is FBL's rule `name`, written as FBL writes it, hyphens and dots included (`as unreadable-requirement`, section 3.6); without it the rule's name is its type's. The same holds for every name a binding declares or names: the binding's own, a block's, a section's, and those `parent`, `within`, `files` and `references` name. Members:

- `id from <slot> [{ … }]` or `id { … }` (FBL §5.3);
- `attribute <name> [from <slot>] [{ … }]`: an attribute binding (FBL §5.2), its options as settings (`empty`, `absent`, `default`, `number`, `time`, `style`, `map`, `override`, `content`, `create`, `readOnly`, `read`, `quoted`, `fallback`), and `references <rules> by <name>` for FBL's `reference` (`by id` for a reference by stored id, FBL §5.7);
- `parent <rules> [slot <name>]` (FBL §5.5), `within <names>`, `when <CEL>`, `files <names>`;
- `insert { … }` and `remove { … }` (section 11.5);
- in a relation rule, `source from <slot>` and `target from <slot>`;
- settings of FBL's `ElementRule` (`caseInsensitive`, `opens`, `undo`, `readOnly`).

A **slot** is `<kind> [<argument>]`: `key "<key>"`, `attribute "<name>"`, `text`, `group "<name>"`, `continuation`, `parent "<attribute>"`, `capture "<name>"` or `value (<CEL>)`, with the slot's other properties (`child`, `word`, `flag`) in the block that follows it.

### 11.4 Blocks and sections

`block <name> [line "<regex>"] { … }` is a block rule of the `blocks` family (FBL §4.7), `section <name> "<heading>" { … }` a section of the `bullets` family (FBL §4.8).

### 11.5 Inserting and removing

`insert { … }` and `remove { … }` are FBL's `Insert` and `Remove` (FBL §6.2), as settings: `place`, `container`, `emit`, `keys`, `create`, `skeleton`, `constant`, `cascade`.

### 11.6 Registration and template

`registration { … }` is FBL's `RegistrationSettings` (FBL §8: `blocks`, `createOnFirstPlacement`, `headers`, `resource`, `legacyLayout`, `legacyIdentities`), with the blocks `layout`, `identities` and those a binding declares. `template <value>` is the bytes of a new body (FBL §13): a string, or a settings map of FBL's `Template` (`byOrigin`).

### 11.7 A capability as the family

A binding whose family is a capability has no rules: what it reads is what the capability's reader returns (FBL §11.2), and every edit is a splice the capability plans. The capability is required with `requires capability` (section 13.2).

### 11.8 Guarantees

FBL's guarantees hold for every binding in ATD (FBL §1.4, §6, §7): reading never fails on content, unmapped text is kept byte for byte, every edit is a minimal splice with an exact undo.

### 11.9 The grammar family

`family grammar` is FBL 0.9's `grammar` family (FBL §4.9): the binding carries an LALR(1) grammar in ATD grammar notation (section 4) and maps the nodes of the tree it reads to elements and relations.

- `grammar { … }` carries the grammar inline: everything between the braces is the grammar's text, read under `grammar/notation.lark` (the rule `grammar_text`), and is FBL's `body.grammar` as a string. `grammar from "<file>.lark"` names a file beside the description that holds it, FBL's `body.grammar` as `{file}`. A binding of the family has exactly one of the two, and a binding of another family has neither.
- The grammar is in the subset of section 4.1, its terminals in the subset of section 3.5; its `%import`s name the binding's own files beside the description, never the modules of ATD's grammar. It is built as section 4.4 says, from the start rule that the setting `start` of `body` names (FBL's `body.start`, default `start`), with the lexing rule of section 3.6.
- `body { … }` takes the family's settings: `start`, and `recover: { unit: <rule>, sync: [<terminals>] }`, which reads a body that does not parse in part (FBL §4.9).
- A rule's `at "<selector>"` selects nodes of the tree by their names (`at "/triples_statement"`), and a slot `node "<selector>"` reads the text of the node or token its relative selector reaches, with the setting `match` when only part of the text is the value (`attribute name from node "PNAME_NS" { match: r"(?<value>[^:]*):" }`); FBL §4.9 and §5.2 define both.
- The checker builds the grammar with the desktop host's table builder, on `etalii-lark` (research R9): a grammar that is not LALR(1), or in which two terminals a state accepts match the same text, is refused with `ATD8001`, placed at the head of the definition of the rule whose reduction is in conflict (of two such rules, the one defined first), and naming the kind of conflict, the terminal and the rules. A grammar the checker cannot read as notation is `ATD1001` at the place it stops, as for any syntax error; a file named by `grammar from` that is not there is `ATD2009`.
- `atd-grammar.py --tables` writes the normalized tables (section 4.7) of every grammar a binding example carries, inline or from a file, to `grammar/tables/<example>.<binding>.tables.json`, and those of every grammar an FBL example names to `grammar/tables/<fbl example>.<binding>.tables.json`, so that the desktop host's builder is cross-checked against Lark's for binding grammars too.

FBL §4.9 says what the family reads, how it writes, and how it reads a body in part; its guarantees are FBL's (section 11.8). In phase 1 no host but the desktop host is required to read the family (FR-081): the family's round-trip fixtures, `specifications/fbl/fixtures/w3c-turtle-grammar*/`, carry the guarantee to every host that implements it later.

The example `binding-grammar.atd` carries Turtle's grammar inline.

---

## 12. The script

The script (`grammar/script.lark`) is the part of ATD in which a tool type's logic is written: the small algorithms the companions list and the hosts write in code (sibling naming, cycle enumeration, relationship elevation). It is not an embedded general-purpose language (FR-019): its expressions are CEL, and it adds only local variables, conditionals, loops with a stated limit, local lists and maps, functions with bounded recursion, and the actions of DISL and DESL as statements (FR-013).

### 12.1 Functions

```text
fn siblingName(parent: Node, base: string) -> string recursion 8 {
  …
}
```

A function has a name, unique among the description's functions; typed parameters, whose types are CEL types or the description's element types; an optional result type; an optional `recursion <n>`; and a block of statements.

- A function is callable from every expression position (FR-017): from CEL, by its name, in operations, hooks, constraints, derived attributes, labels and binding rules.
- A function with a result type **MUST** return a value of it or `fail` on every path (`ATD7003`).
- `recursion <n>` bounds how deep the function recurses, at most 64 (`ATD7004`); absent, the bound is 64.
- DISL's CEL functions (`functions`, DISL §3.4) are functions whose block is one `return`.

### 12.2 Statements

| Statement | Meaning |
|---|---|
| `let <name> [: <Type>] = <CEL>` | an immutable local |
| `var <name> [: <Type>] = <CEL>` | a mutable local |
| `<name> = <CEL>` | assigns a mutable local (the name and `=` are one token, section 3.4) |
| `if <CEL> { … } [else if … \| else { … }]` | a conditional |
| `for <name> [, <name>] in <CEL> limit <n> { … }` | a loop over a list, a map (its keys, and with two names its keys and values) or a range |
| `while <CEL> limit <n> { … }` | a loop while a condition holds |
| `return <CEL>`, and `return` last in a block | returns from the function, or ends an operation's or hook's statements |
| `break`, `continue` | leave or continue the nearest loop |
| `fail <CEL>` | refuses the gesture with the message, a string |
| an action (section 12.3) | a change to the model, in the gesture's transaction |

A loop **MUST** state its `limit`; one without does not parse. A loop that reaches its limit stops the call with a finding of its own (`ATD7101`), not merely the loop: a limit is the most iterations the tool engineer expects, and reaching it is an error, distinct from a budget overrun.

### 12.3 Actions

The actions of DISL §9.4 and DESL §8.2 are statements, each allowed only where a model may change (section 12.7); their arguments are CEL:

| Action | DISL |
|---|---|
| `set <path> = <CEL>` (`set self.text = …`, or `set text = …` for `self`) | `set` |
| `unset <path>` | `unset` |
| `create <Type> [{ <attr>: <CEL> … }] [in <CEL>] [after <CEL> \| before <CEL>] [slot <name>] [at <CEL>] [size <CEL>] [place <name>] [as <name>]` | `create` |
| `connect <Type> from <CEL> to <CEL> [{ … }] [as <name>]` | `connect` |
| `delete <CEL>` | `delete` |
| `move <CEL> [to <CEL>, <CEL>] [into <CEL>] [slot <name>] [after <CEL> \| before <CEL>]` | `move` (DISL), and DESL §8.2's `move` |
| `resize <CEL> to <CEL>, <CEL>` | `resize` |
| `retype <CEL> to <Type>` | `retype` |
| `reparent <CEL> to <CEL> [slot <name>] [after <CEL> \| before <CEL>]` | `reparent` |
| `reorder <CEL> by <CEL> \| after <CEL> \| before <CEL>` | `reorder` |
| `view <CEL> { <key>: <CEL> … }` | `view` |
| `open <CEL>` | `open` |
| `openForm <form> for <CEL>` | `openForm` |
| `editLabel <CEL> [{ label: <id> }]` | `editLabel` |
| `select <CEL>`, `reveal <CEL>` | `select`, `reveal` |
| `highlight <CEL> [{ durationMs: <n> }]` | `highlight` |
| `notify <CEL> [severity <name>]` | `notify` |
| `layout <name> [on <CEL>] [{ … }]` | `layout` |
| `call <operation>(<args>)` | `call` |
| `call plugin <name>(<args>)` | `plugin` |
| `fail <CEL>` | `abort` |
| `let`, `if`, `for` | `let`, `if` with `then` and `else`, `forEach` with `as` and `do` |

DISL's `when` on an action is an `if` around it. A name an action binds with `as` is a local of the statements after it.

`move` is DISL's in a diagram and DESL's in a designer (DESL §8.2). `into` names the new parent, `null` for the document's own elements; `slot` its slot; `after` or `before`, at most one, a sibling there, the element going last without either. The element keeps its id, its attributes and everything it contains; a move into the element itself or into something it contains, or into a parent or slot that does not allow its type, rolls the transaction back; a move whose parent and slot are the element's own is a `reorder`. `to <x>, <y>` sets a node's position on the canvas and is admitted in diagrams only (`ATD3002`).

### 12.4 The transaction

A script changes a model only through its actions, inside the transaction of the gesture that called it (FR-016): each action adds its commands to the transaction exactly as DISL's operation actions do (DISL §14.4), every expression sees the effects of every action before it, and the transaction is one step of the history with an exact undo. A call that ends other than by returning (a `fail`, a CEL error, a budget or recursion overrun, a loop at its limit, or the debugger's stop) discards the whole transaction, also when it runs in a hook inside another operation's transaction, and refuses the gesture with a finding that names the function.

### 12.5 Determinism

A script is deterministic (FR-014):

- its environment is CEL's, with DISL §12.4's library and §12.5's rules: no function reads a clock, randomness, a file, the network, the environment or the host; the checker refuses a call of any function that is neither in that library nor a function of the description (`ATD7001`);
- a `for` over a map visits its keys in ascending order of CEL's ordering (strings by code point);
- arithmetic is CEL's: `int` and `uint` are 64-bit with CEL's overflow errors, `double` is IEEE 754 binary64, and `string(double)` formats as CEL's specification says;
- a local list or map grows only by its statements; a value is at most 1,000,000 elements or code points long, beyond which the call stops as a budget overrun.

### 12.6 The budget

Every call runs under a **step budget** (FR-015): one step per statement executed, per loop iteration, and per CEL call evaluated. The budget is per gesture, the transaction's: 100,000 steps unless the description's `limits.steps` sets a lower number. Recursion is at most 64 frames, or the lower `recursion` of a function. Exceeding the budget (`ATD7102`) or the recursion (`ATD7103`) stops the call and discards the transaction. Where there is no gesture to refuse (a label, a constraint, a derived attribute, a binding rule read when a document is opened), the position falls back as DISL §2.5's table says for an expression that fails: a label takes its default, a constraint reports that it could not be evaluated, a derived attribute or element is absent with a finding, and a binding rule's entry is unreadable (FBL §7.4).

### 12.7 Purity

A function whose block, or a function it calls, contains an action is **impure**. An expression position that only reads (a label, a constraint's `when` and `rule`, a derived attribute, a variant's `when`, a binding rule's `when`, a setting's `( CEL )`) **MUST NOT** call an impure function, through any chain of calls (`ATD7002`); an operation, a hook and a fix may.

### 12.8 Host attributes

`hostAttribute(<element>, <name>)` is the value the binding read into the host attribute `<name>` of the element (`persistence`'s `typeMap`, `hostAttributes`, DISL §11.2), as the binding read it, and `null` when it read none. DISL keeps host attributes out of CEL; a description reads them through this one function, by name, so that a rule can judge what the file wrote where the model holds nothing: a relation's end that names no element is unset in the model (DISL §4.9), while the binding can read the id it was written with a second time, into a host attribute (`dependency-graph.atd`'s `writtenEnd`).

- The function only reads, so it may be called from every expression position (section 12.7). A host attribute is document data, so the call is deterministic (section 12.5).
- A `<name>` written as a string **MUST** be a host attribute of an entry of `persistence`'s `typeMap` (`ATD2001`).
- A first argument that is not an element of the document, or a second that is not a string, stops the call as a CEL error.

---

## 13. Capabilities

### 13.1 What a capability is

A **capability** is a named algorithm the platform provides and a description calls, for work the script is not meant for: heavy or numeric algorithms, and readers of formats FBL's families cannot declare (FR-018). A capability is **generic**: its name, contract, inputs and outputs are in the terms of its algorithm or the format it reads, never in a tool type's vocabulary (a two-sided tree layout, not a mind map layout; a reader named for its format), even when one tool type is its only user (spec 035, Q7). Code reached only through a capability is not tool-specific code.

### 13.2 Requiring a capability

`requires capability <name>@<range>` names a capability the description uses, with a semver range of its versions (`^1`, `~1.2`, `>=1.0`). A capability the description uses and does not require, and one it requires that is not in the catalogue (section 13.6), are findings (`ATD2010`). A range is resolved as npm's semver ranges are: the highest version the host provides that satisfies it.

### 13.3 Extension points

A capability is called only where its extension point allows:

| Point | Called from | Input | Output |
|---|---|---|---|
| `layout` | `algorithm <name> capability <name> { … }` | nodes with sizes and pins, edges (`$defs/LayoutInput`) | positions (`$defs/LayoutResult`) |
| `routing` | an edge's `line { routing: … }` | edge ends and obstacles | route points |
| `reader` | `binding … family capability <name>` | a body (`$defs/ReaderInput`) | a reading and splices (`$defs/ReaderResult`) |
| `analysis` | a derived element's or relation's `from`, a constraint | a model (`$defs/AnalysisInput`) | derived elements, relations, findings (`$defs/AnalysisResult`) |
| `arrange` | an operation, as `call plugin <name>(…)` | the selection and the model | actions |
| `render` | a label, a preview | a value (`$defs/RenderInput`) | text runs or a drawing (`$defs/RenderResult`) |

### 13.4 A capability the host lacks

A description that requires a capability the running host does not provide, or whose range it cannot meet, still opens: the gestures that need the capability are refused with a reason, and one finding names the capability (`ATD9010`).

### 13.5 Determinism of capabilities

A capability is held to the determinism of section 12.5. A capability that must read beyond its input states each exception in its contract's `exceptions`, with its reason, and takes what it reads as input from the working copy, so that a conformance case lists those files among its inputs (`analysis.msbuildImports` reads the `.props` and `.targets` files a project imports).

### 13.6 The catalogue

The catalogue is the capabilities with a contract in `capabilities/` beside this document. It grows as the tool descriptions in scope are written, each capability entering with its contract and recorded fixtures (section 2.6):

| Capability | Point | Status |
|---|---|---|
| `layout.twoSidedTree` | layout | provisional |
| `reader.turtle` | reader | provisional |
| `render.markdown` | render | provisional |

---

## 14. Static semantics

What the grammar cannot state and a checker **MUST** check (FR-010).

### 14.1 A description

A description is its root and the parts its imports reach (section 5). Every file is parsed; a file that does not parse is reported (`ATD1001`). A description whose files do not all parse is checked no further than its syntax: what the parser skipped would otherwise be reported again as names that resolve to nothing. Its names are still collected, so that a language server's hover, definition and completion answer in the parts that parse (FR-053).

### 14.2 Namespaces and names

Each of the following is a namespace, as in DISL §2.2: types (element and relation types share one), enums and data types (sharing one with types), coordinate systems, axes, snapping profiles, styles, shapes, markers, icons, forms, tools, templates, constraints, operations, hooks, reasons, functions, layouts and their algorithms, viewpoints, bindings, and each binding's rules. Two declarations of one name in a namespace are a finding (`ATD2006`); names differing only in case are one name. A reference resolves in its namespace across every file of the description; a name a library import brings is prefixed with its alias. A reference that resolves to nothing is a finding at the reference (`ATD2001` an attribute, `ATD2002` a type, `ATD2004` a binding rule, `ATD2005` a function, `ATD2010` a capability, `ATD2011` any other name), with the closest names (section 15.5). An element type, attribute, parameter, function or local whose name is a CEL reserved word cannot be used in CEL and is a finding at its declaration (`ATD4003`).

### 14.3 Imports and parts

Section 5.2 and 5.3: a part names its root's origin (`ATD2007`), is imported by one root (`ATD2008`), and its file exists in the description's folder (`ATD2009`); a cycle is reported once (`ATD2003`).

### 14.4 Kinds

A description admits only the constructs its kind allows (FR-005). This table is normative, and a checker reads it from one place:

| Construct family | Diagram | Designer | Editor |
|---|---|---|---|
| Element types, attributes, enums, data types, ids, containment | yes | yes | yes |
| Relation types, ports, derived relations | yes | no (`ATD3001` at the construct) | no (`ATD3001`) |
| `coordinates`, `notation`, `toolbox`, `layout`, `viewpoint`, and a `move`'s `to <x>, <y>` | yes | no (`ATD3002`) | no (`ATD3002`) |
| `form` | yes | yes | no (`ATD3002`) |
| `surface table`, `surface hierarchy` | no (`ATD3002`) | yes | no (`ATD3002`) |
| `surface text` | no (`ATD3002`) | no (`ATD3002`) | yes |
| Constraints, operations, actions, reasons, messages, labels, documentation, origin, settings | yes | yes | yes |
| Hooks | yes | no (`ATD3001`) | yes |
| Bindings of every family, registration | yes | yes | yes |
| Functions, capabilities | yes | yes | yes |

A designer **MUST** have one form surface and an editor one text surface (`ATD3003`).

### 14.5 Expressions

Every expression is type-checked in the CEL environment of its position (DISL §2.5, §12.3; DESL §9.3; FBL §2.4): the variables in scope, the functions of DISL's library and the description's own, and the result type the position expects. The environments:

| Position | Variables | Result |
|---|---|---|
| a constraint's `when` and `rule`, of `for <Types>` | `self`, `diagram` or `document`, and DISL §12.3's constraint context | `bool` |
| an operation's `enabled`, a context tool's `visible` and `enabled` | `self`, `diagram` or `document`, `selection` | `bool` |
| an operation's or hook's statements | `self`, `diagram` or `document`, `selection`, `p` (the parameters), `event` and `old` in a hook, the locals | (statements) |
| a variant's `when`, a notation's `( CEL )` | `self`, `diagram`, `view` | `bool`, or the property's type |
| a binding rule's `when`, a slot's `value (…)` | `entry`, `text`, `style`, `file` (FBL §2.4) | `bool`, the attribute's type |
| a function's statements | its parameters and locals | its result type |
| a setting's `( CEL )` | the context of the property, as DISL, DESL or FBL names it | the property's type |

An expression whose type does not fit the position is a finding (`ATD4001`); an unknown variable (`ATD4004`) and a CEL reserved word used as an identifier (`ATD4003`) are findings too.

### 14.6 Settings

Each construct's settings are checked against the `$defs` entry of DISL, DESL or FBL that Appendix C names for the construct (FR-011): the setting's JSON value (section 6.6) is put where the construct's JSON form would have it, and validated. A key the schema does not admit (`ATD5001`), a value it does not admit (`ATD5002`), and a literal where the schema wants an expression (`ATD5003`) are findings at the setting. An `x-` key is admitted and not checked.

### 14.7 Model, surface and binding as one unit

The model, the surface and the bindings are checked together, so that one cause gives one finding, where it is:

- a binding rule's type **MUST** be a type of the model or a key of `persistence`'s `typeMap` (DISL §11.2, DESL §5.2), as DESL §10.1 says (`ATD2002` otherwise). A key mapped `as: document` reads and writes the attributes of `document { … }`; one mapped to a type, that type's; each through the entry's `attributes`, which rename the binding's attributes to the model's. A key mapped `as: header` or `as: unreadable` keeps its entries out of the model, and its rules bind no attribute; an `unreadable` entry is reported by `std.unreadableEntry`;
- every attribute a binding rule reads or writes **MUST** be one of those its type gives it (`ATD6001` at the binding, once, not again where the attribute is used);
- a notation, form, surface or outline names the model's types and attributes (`ATD2002`, `ATD2001`);
- a rule a binding names (`parent`, `within`, `cascade`, `references`) is one of its rules (`ATD2004`), as FBL §14.1 requires.
- a hierarchy surface fits its types, as DESL §10.1 requires of one (`ATD6002`, at the member of `surface hierarchy` that does not fit): the levels' type contains itself and the entries' type; each `labels` attribute is an attribute of every type of its `on`; the `collapsed` attribute is one of the levels' type and of the entries' type; the `widths` type's `level` is a `string` attribute and its `width` an `int` attribute. A type or an attribute a member names that does not exist is `ATD2002` or `ATD2001`, not `ATD6002`;
- a `counter` id rule's `mark` (DESL §5.3.5) is a `string` attribute of `document { … }` (`ATD6003`, at the setting `mark`);
- each key of `persistence`'s `settings` (DESL §5.5) is a block that every binding of the description declares in its `registration`'s `blocks` (`ATD6004`, at the key); the types and attributes a setting names are the model's (`ATD2002`, `ATD2001`).

### 14.8 The script

Section 12's rules: a function call resolves (`ATD2005`) and is in the library or the description (`ATD7001`); purity through the call graph (`ATD7002`); every path returns or fails (`ATD7003`); `recursion` at most 64 (`ATD7004`); a loop's `limit` a non-negative integer; an action only where a model may change.

---

## 15. Findings

### 15.1 Codes

A finding has a code `ATD<nnnn>`, by range:

| Code | Finding |
|---|---|
| `ATD0001` | The description declares a version of ATD the reader does not know; the message names both. |
| `ATD0002` | The file does not start with `atd <major>.<minor>`. |
| `ATD1001` | A syntax error: what was found, and what was expected. |
| `ATD1002` | Text no token matches. |
| `ATD2001` | No such attribute. |
| `ATD2002` | No such element or relation type. |
| `ATD2003` | Imports that form a cycle (once). |
| `ATD2004` | No such binding rule. |
| `ATD2005` | No such function. |
| `ATD2006` | A name declared twice in one namespace. |
| `ATD2007` | A part of another tool type's origin. |
| `ATD2008` | A part two roots import. |
| `ATD2009` | An imported file that is not there or outside the description's folder. |
| `ATD2010` | A capability not in the catalogue, or used without `requires capability`. |
| `ATD2011` | No such name of another namespace (a reason, an operation, a style, …). |
| `ATD3001` | A construct of the model the description's kind does not admit. |
| `ATD3002` | A surface construct of another kind. |
| `ATD3003` | A designer or editor without its one surface, or with two. |
| `ATD4001` | An expression whose type does not fit its position. |
| `ATD4003` | A CEL reserved word as an identifier. |
| `ATD4004` | An unknown variable in an expression. |
| `ATD5001` | A setting the construct's schema does not have. |
| `ATD5002` | A setting's value the schema does not admit. |
| `ATD5003` | A literal where the schema wants an expression. |
| `ATD6001` | A binding that reads or writes an attribute the model does not declare. |
| `ATD6002` | A hierarchy surface that does not fit its types (section 14.7). |
| `ATD6003` | A `counter` id rule whose mark is not a `string` attribute of the document. |
| `ATD6004` | A setting of `persistence` that no binding's registration keeps as a block, or that one binding does not. |
| `ATD7001` | A call of a function outside CEL's library and the description (a clock, randomness, a file, …). |
| `ATD7002` | An impure function called from a position that only reads. |
| `ATD7003` | A path of a function that neither returns nor fails. |
| `ATD7004` | A recursion bound above 64. |
| `ATD7101` | At run time: a loop reached its limit. |
| `ATD7102` | At run time: the step budget is exceeded. |
| `ATD7103` | At run time: the recursion bound is exceeded. |
| `ATD8001` | A grammar in a binding that is not LALR(1), naming the conflict (FBL 0.9). |
| `ATD9001` | Validation: a difference from a conformance fixture. |
| `ATD9010` | A capability the running host does not provide. |

### 15.2 Severities

`error` (the description is not accepted), `warning` and `information`. Every finding of section 15.1 is an error unless its construct says otherwise.

### 15.3 Positions and messages

A finding names its file, the 1-based line and column (in code points) of its start, and of its end; `related` places it concerns; and its message, a sentence in English in ADP's words (docs/terminology.md), naming the construct as the tool engineer wrote it. A syntax error names what was found and what was expected. Localized text a description carries is never in a finding's message.

### 15.4 Machine-readable findings

`atd check --json` writes `$defs/Findings`, the findings in a stable order: by file, then line and column, then code (FR-091). A finding carries its `fixes`, each a title and text edits in byte offsets, and its `suggestions`. The language servers' diagnostics carry the same code, and the fixes as code actions.

### 15.5 Closest names

The closest names a finding offers for a name that resolves to nothing are the names of the same namespace in scope within an edit distance of 2 or sharing a prefix of 3, at most five, nearest first.

---

## 16. Canonical layout

### 16.1 The layout

The formatter writes a description in one canonical layout (FR-032):

- two spaces of indentation per level of braces;
- one member per line; a block of settings (an attribute's, a style's, a shape's and the like) on one line when it holds no comment and fits in 120 columns, otherwise one setting per line between `{` and `}`; a block of members always one member per line;
- one space around `:` of a setting after it only, around binary operators, after `,`, and before `{`; none inside parentheses and brackets;
- members in the order written: the formatter never reorders;
- one blank line kept where the text has one or more between members; none at the start or end of a block;
- the file ends with one line end, LF unless the file's first line ends with CRLF.

### 16.2 Comments

Every comment stays with its construct:

- a comment on a line of its own belongs to the next construct, and is written above it at its indentation;
- a comment at the end of a line belongs to the construct on that line, and stays at its end;
- a comment before a closing brace, or at the end of the file, belongs to the enclosing block, and stays last in it.

### 16.3 Idempotence

Formatting a formatted description changes nothing. A file that does not parse is not formatted: the formatter changes nothing and reports why.

### 16.4 Machine edits

An edit a machine makes to a description (a refresh, an agent, a rename) **MUST** be possible as splices that leave every byte outside them as it was (FR-033): a rename is one splice per use, a fix a set of splices. The formatter's own output is the one edit that may rewrite a whole file.

---

## 17. The host's concerns

What a description leaves to the host is only (FR-022): **rendering** (pixels, fonts and their rasterizing, images), **gestures** (the pointer and keys a host maps to the intents a description names), **text measurement** (beyond the text metric a description declares), **file dialogs**, **opening links** (beyond `open`'s rules), and the host's **undo stack** (beyond each transaction's one step). An item of a companion that is one of these is traced as `hostConcern` with its reason (section 2.5); a description states nothing else about them.

---

## 18. Versioning

- The header `atd <major>.<minor>` is the one construct every version keeps; a reader selects the grammar by it (section 5.4).
- While ATD is 0.x, a minor version **MAY** change any construct. Each released minor's grammar is kept under `grammar/<version>/`, and the draft's in `grammar/`.
- Every description in this repository is checked under its declared version, and, before a new version is released, under the new one (FR-102): a description that fails under the new version is changed, or the version is.
- From 1.0 on, a change that makes a valid description invalid or changes its meaning is a major version.

---

## 19. Complete examples

*(informative)*

### 19.1 The examples

Beside this document, each a complete description that checks without findings:

| File | Shows |
|---|---|
| `mindmap.atd`, `mindmap.notation.atd` | a diagram split over a root and a part, with script functions, constraints, operations, a capability and an `xml` binding |
| `reading-list.atd` | a designer with a table surface and a `yaml` binding |
| `notes.atd` | an editor with a text surface and a `bullets` binding |
| `binding-json.atd`, `binding-lines.atd`, `binding-blocks.atd` | a binding of the `json`, `lines` and `blocks` families |
| `binding-capability.atd` | a binding whose family is a capability |

And the worked examples of section 2: `fixtures/mindmap/basic/`, `mindmap.trace.json` with `mindmap.md`, and `capabilities/*/contract.json`.

The tool descriptions under `definitions/` are examples too, and `atd-grammar.py --probe` reads them with those beside this document: `definitions/designers/hierarchical-requirement.atd`, with its part `hierarchical-requirement.markdown.atd`, is a designer with a hierarchy surface, the widths of its columns, `move` with a place, binding types the type map maps as the document and as unreadable, and rule names with hyphens.

### 19.2 The walk

`walk/walk.json` (`$defs/Walk`) is the walk of 30 edits and requests over the examples that every language server meets alike (SC-005): completion in a binding rule's attribute position, a rename across a description's files, typing on after a syntax error, formatting twice, and a description with a part.

### 19.3 The refused corpus

`refused/` holds descriptions a checker refuses, each with the findings it must give (`refused/README.md`).

### 19.4 For agents

An agent writing a description is given `grammar/atd.flat.lark`, with which a model service constrains its output, two examples, and the findings of `atd check --json`, which it reads as data.

---

## Changes

- **0.1** (2026-10-23, spec 035): the first version.
- **0.1, amendment of 2026-10-25** (spec 035, batch 3, for the Hierarchical Requirement Designer): `widths <Type> { … }` in `surface hierarchy`, carrying DESL 0.2's `widths` (§9.3, C.2); `move` takes `slot`, and `after` or `before`, so that it carries DESL §8.2's `move` (§12.3, C.2); a binding's, rule's, block's, section's and file rule's name may hold hyphens and dots, as FBL's names do (§3.6, §11.3); a binding rule's type may be a key of `persistence`'s `typeMap` (§7.13, §14.7); and DESL 0.2's checks of a hierarchy, a counter's mark and a registration's settings are `ATD6002`, `ATD6003` and `ATD6004` (§14.7, §15.1). ATD 0.1 had no released grammar, so the amendment changes it in place; every example and tool description in this repository that checked before still checks, with the same meaning. With it, a conformance case of a designer: a step `outside`, a file written outside the host (§2.3, `$defs/ConformanceStep`), and the reference for the readings and findings of a tool type the desktop host did not have at its reference commit (§2.3).
- **0.1, amendment of 2026-10-25, FBL 0.9** (spec 035, story 7): the `grammar` family of FBL 0.9 (§11.9, FBL §4.9): `family grammar` with `grammar { … }`, read under `grammar/notation.lark`, which the root module now imports, or `grammar from "<file>.lark"`; the keyword `grammar` (Appendix B); the settings `start` and `recover` of `body` and the slot `node` with its setting `match` (C.3); where `ATD8001`, for a grammar that is not LALR(1), stands (§11.9, §15.1); and the tables of every binding example's grammar under `grammar/tables/` (§11.9). Every description that checked before still checks, with the same meaning.


---

## Appendix A — The schema's `$defs`

`atd.schema.json` has one `$defs` entry per structure; each line gives its name and what it is.

| `$defs` | |
|---|---|
| `Span` | Where a node stands in its file: UTF-8 byte offsets of its first byte and of the byte after it, and the 1-based line and column of its start and end, columns in code points (ATD §3.1, §15). |
| `Name` | A name as written, with where it stands: a NAME or a keyword in a name position (ATD §3.6). |
| `QualifiedName` | A dotted name, `layout.twoSidedTree`, `std.missingId`. |
| `TypeRef` | A type: a qualified name with its type arguments (`list<Concept>`). |
| `Value` | A value (ATD §6): exactly one of a literal, a CEL expression with its abstract syntax, a reference to an attribute, a theme token or a parameter, a settings map, or a list. |
| `Setting` | One `key: value` pair (ATD §6.4); `quoted` when the key was written as a string. |
| `CelNode` | CEL's abstract syntax (ATD §6.3), with spans: an operator is a call of its CEL function name (`_+_`, `_&&_`, `!_`, `-_`, `_?_:_`, `_[_]`, `@in`, `_[?_]`, `_?._`), a macro is a call marked `macro`, a method call has a `target`. |
| `Tree` | The abstract tree of one `.atd` file, as `atd tree` prints it (ATD §4.5, FR-030). It is not an interchange form: nothing reads it back as a description. |
| `Tool` | The tool construct of a root file (ATD §5.1). |
| `Part` | A part of a description (ATD §5.2). |
| `Member` | A member of a tool or a part: a construct of the core (ATD §7), the script (§12), a surface (§8 to §10) or a binding (§11). |
| `Label` | `label value`: a display name or a label's text. |
| `DocMember` | `doc value`. |
| `SettingMember` | A setting among the members of a construct. |
| `Import` | `import "file"` (a part), or `import "file" as name` (a library) (ATD §5.3). |
| `Requires` | `requires capability name@range` (ATD §13). |
| `Model` | `model { … }` (ATD §7.4). |
| `Element` | `element Name extends … { … }`: an element type (DISL's node type, DESL's element type). |
| `Relation` | `relation Name from … to … { … }` (DISL §4.9). |
| `Attribute` | `attribute name: Type flags { … }` (DISL §4.3, DESL §4.3). |
| `Contains` | `contains Types as slot ordered` (DISL §4.8, DESL §4.6). |
| `Port` | `port name { … }` (DISL §4.10). |
| `Enum` | `enum Name "Label" { values }` (DISL §4.5). |
| `DataType` | `datatype Name: Base { … }` (DISL §4.4). |
| `DocumentDecl` | `document { … }`: the attributes of the document itself. |
| `Constraints` | `constraints { … }` (DISL §8, DESL §6). |
| `Constraint` | `constraint id for Types { … }`. |
| `Builtin` | `builtin std.name { … }`: settings of a built-in constraint (DISL §8.7). |
| `Fix` | `fix name "Label" { … }`: a quick fix (DISL §8.5), or a fix on a range of an editor's text (ATD §10). |
| `Operation` | `operation id "Label" for Types { … }` (DISL §9.3, DESL §8.1). |
| `Param` | `param name: Type`. |
| `Hook` | `hook id on event for Types { … }` (DISL §9.2). |
| `Reason` | `reason id "text"` or `reason id { when … message … }` (DISL §2.3, §9.1). |
| `MessageDecl` | `message id value`: a standard message (DISL §9.1). |
| `Behavior` | `behavior { … }`. |
| `Persistence` | `persistence { … }`. |
| `Plugin` | `plugin name { … }` (DISL §13). |
| `Function` | `fn name(params) -> Type recursion n { … }` (ATD §12.1). |
| `Expr` | A CEL expression in a position that takes only an expression, written bare (ATD §6.2). |
| `Statement` | A statement of the script (ATD §12.2) or an action (§12.3). Its fields are those its form writes: `name`, `type`, `value`, `target`, `path`, `condition`, `then`, `else`, `names`, `iterable`, `limit`, `body`, `assignments`, `parent`, `after`, `before`, `slot`, `at`, `size`, `place`, `as`, `source`, `to`, `by`, `x`, `y`, `width`, `height`, `form`, `severity`, `on`, `operation`, `args`, `settings`. |
| `Coordinates` | `coordinates { system … axis … snapping … default … }` (DISL §5). Its members are the constructs ATD's section names, each with `kind` the construct's keyword (`system`, `node`, `edge`, `variant`, `group`, `tool`, `algorithm`, `rows`, `valueType`, `outline`, …), `name` or `names`, its own fields and its `settings`. |
| `Notation` | `notation { … }` (DISL §6). Its members are the constructs ATD's section names, each with `kind` the construct's keyword (`system`, `node`, `edge`, `variant`, `group`, `tool`, `algorithm`, `rows`, `valueType`, `outline`, …), `name` or `names`, its own fields and its `settings`. |
| `Toolbox` | `toolbox { … }` (DISL §7.1 to §7.4). Its members are the constructs ATD's section names, each with `kind` the construct's keyword (`system`, `node`, `edge`, `variant`, `group`, `tool`, `algorithm`, `rows`, `valueType`, `outline`, …), `name` or `names`, its own fields and its `settings`. |
| `Layout` | `layout { default … algorithm … }` (DISL §10). Its members are the constructs ATD's section names, each with `kind` the construct's keyword (`system`, `node`, `edge`, `variant`, `group`, `tool`, `algorithm`, `rows`, `valueType`, `outline`, …), `name` or `names`, its own fields and its `settings`. |
| `TableSurface` | `surface table { … }` (DESL §7.2 to §7.6). Its members are the constructs ATD's section names, each with `kind` the construct's keyword (`system`, `node`, `edge`, `variant`, `group`, `tool`, `algorithm`, `rows`, `valueType`, `outline`, …), `name` or `names`, its own fields and its `settings`. |
| `HierarchySurface` | `surface hierarchy { … }` (DESL §7.7). Its members are the constructs ATD's section names, each with `kind` the construct's keyword (`system`, `node`, `edge`, `variant`, `group`, `tool`, `algorithm`, `rows`, `valueType`, `outline`, …), `name` or `names`, its own fields and its `settings`. |
| `TextSurface` | `surface text { … }` (ATD §10). Its members are the constructs ATD's section names, each with `kind` the construct's keyword (`system`, `node`, `edge`, `variant`, `group`, `tool`, `algorithm`, `rows`, `valueType`, `outline`, …), `name` or `names`, its own fields and its `settings`. |
| `Form` | `form id "Label" for Types { … }` (DISL §7.5). |
| `Viewpoint` | `viewpoint id "Label" { … }` (DISL §3.5). |
| `Binding` | `binding name family … { … }` (ATD §11). |
| `BindingMember` | A member of a binding: `claims`, `body`, `text`, `header`, `registration`, `template`, `block`, `section`, an element or relation rule, or a `setting`, `doc` or `label` member. |
| `Rule` | An element or relation rule (FBL §5.1, §5.4). |
| `Finding` | A finding of the checker, a language server or validation (ATD §15, FR-091). |
| `Findings` | The findings of one run, in a stable order: by file, then position, then code (ATD §15.4). A refused file of the corpus states those it must give in this form; `message` is then not compared. |
| `ParseTables` | The LALR(1) automaton of a grammar in ATD grammar notation, normalized so that two implementations can be compared byte for byte (ATD §4.7): terminals by name (an anonymous string terminal is named by its quoted string), the rules of the grammar after the expansion of §4.4 sorted and numbered (a helper of a repetition named `(<symbols>)+`), and the states numbered breadth first from the start state over the symbols in sorted order, each with its actions on terminals (`s<state>`, `r<rule>`, `accept`) and its gotos on nonterminals. |
| `Trace` | The trace of a tool description (ATD §2.5, data-model.md §10): every item of its companion's gap sections and every behaviour of a host's code, each with the part of the description that states it, the host's concern it is, or the reference behaviour it diverges from. |
| `Provenance` | Who produced one part of a fixture (ATD §2.3). |
| `ConformanceStep` | One gesture of a case and what it must produce (ATD §2.3). Exactly one of `action`, `property`, `move`, `create`, `connect`, `delete`, `outside`, `undo` and `redo`; `select` before it. |
| `ConformanceFixture` | One conformance case of one tool type (ATD §2.3, data-model.md §8). |
| `Drawing` | The scene of one view in `adp-scene`'s vocabulary (ATD §2.4, data-model.md §9). Two drawings are equal when their elements and edges are equal as sets keyed by `id` and their `order` lists are equal. |
| `Capability` | The contract of a capability (ATD §2.6, §13, data-model.md §7). |
| `CapabilityFixture` | One fixture of a capability: an input, the arguments and the expected output (ATD §2.6). |
| `LayoutInput` | The input of a `layout` capability: the nodes with their sizes and pins, the edges and the containment, in the canvas's domain units. |
| `LayoutResult` | The output of a `layout` capability: a position for every node it places, in the canvas's domain units, rounded to 0.01, and optionally route points. |
| `ReaderInput` | The input of a `reader` capability: the body's bytes as text, and the edit to plan when one is asked (FBL §11.2). |
| `ReaderResult` | The output of a `reader` capability: what FBL §11.2's read returns, or the splices of a planned edit. |
| `RenderInput` | The input of a `render` capability: a value to render. |
| `RenderResult` | The output of a `render` capability: text runs or a drawing. |
| `AnalysisInput` | The input of an `analysis` or `arrange` capability: a model or part of one. |
| `AnalysisResult` | The output of an `analysis` capability: derived elements or relations and findings; of an `arrange` capability: actions. |
| `Walk` | The 30-edit walk of SC-005 (ATD §19): steps over the examples, each a request at a position or an edit, with its expected result. |
| `SettingsSchemas` | The `$defs` of DISL, DESL and FBL that the settings of ATD's constructs are checked against (ATD §14.6), by reference; each construct's entry is listed in ATD's Appendix A. |

## Appendix B — Keywords

Every keyword of ATD 0.1, by the module that defines it, with the documentation the language servers show on hover. Every keyword is soft: it is also a name wherever a name is expected (§3.6). `true`, `false` and `null` are CEL's literals, not keywords.

### B.1 `atd.lark`

| Keyword | Documentation |
|---|---|
| `atd` | The header `atd <major>.<minor>`, the first construct of every file: the version of ATD the file is written in (§5.1, §18). |
| `designer` | The kind of a tool type that is a form-based visual layout: `designer <origin> version "<semver>" { … }` (§5.1). |
| `diagram` | The kind of a tool type that draws elements and the relations between them: `diagram <origin> version "<semver>" { … }` (§5.1). |
| `editor` | The kind of a tool type in which typing text is the core interaction: `editor <origin> version "<semver>" { … }` (§5.1). |
| `of` | `part of <origin>`: the description a part belongs to (§5.2). |
| `part` | `part of <origin>`: starts a part of a description (§5.2). |
| `version` | `version "<semver>"`: the tool type's version, in the tool construct (§5.1). |

### B.2 `core.lark`

| Keyword | Documentation |
|---|---|
| `as` | Names what precedes it: a slot in `contains`, a label's id, a binding rule's name, an import's alias, the element an action creates. A CEL reserved word: not an identifier inside CEL. |
| `attribute` | In an element type: `attribute <name>: <Type> [required] [many] [readonly] [default <value>] { … }`, an attribute (§7.6, DISL §4.3). In a binding rule: `attribute <name> from <slot> { … }`, an attribute binding (§11.3, FBL §5.2). |
| `behavior` | `behavior { … }`: DISL's behaviour settings beside its operations and hooks: deletion, clipboard, retype, undo, the edit gate (§7.12). |
| `builtin` | In `constraints`: `builtin <std.name> { … }`, the settings of a built-in constraint (DISL §8.7). |
| `capability` | `requires capability <name>@<range>`, `algorithm … capability <name>`, `family capability <name>`, `preview capability <name>`: a named, generic algorithm the platform provides (§13). |
| `confirm` | In an operation: `confirm { … }`, the confirmation it asks for (DISL §9.5). |
| `constraint` | `constraint <id> [for <Types>] { when … rule … message … }`: a constraint, whose findings the running tool reports (§7.9, DISL §8.2). |
| `constraints` | `constraints { … }`: constraints, the settings of the built-in ones, and the constraints' common settings (§7.9). |
| `contains` | In an element type: `contains <Types> [as <slot>] [ordered] { … }`, which element types it contains (§7.8, DISL §4.8). |
| `datatype` | In `model`: `datatype <Name> [: <Base>] { … }`, a data type (§7.7, DISL §4.4). |
| `default` | In an attribute: `default <value>`, its default. In `coordinates` and `layout`: `default <name>`, the default system or algorithm. |
| `doc` | `doc <value>`: documentation of the construct it stands in, a string or DISL's `Doc` (§7.3); `///` comments add to it. |
| `document` | In `model`: `document { … }`, the attributes of the document itself (§7.8). |
| `element` | In `model`: `element <Name> [extends <Types>] { … }`, an element type (§7.5). In a binding: `element <Type> [as <name>] [at|line …] { … }`, an element rule (§11.3). |
| `enabled` | In an operation: `enabled <CEL>`, when it can be run (DISL §9.3). |
| `enum` | In `model`: `enum <Name> ["<label>"] { <values> }`, an enumeration (§7.7, DISL §4.5). |
| `extends` | `extends <Types>`: the supertypes of an element or relation type, or the styles a style extends; inheritance is C3 (DISL §4.7). |
| `fix` | `fix <name> ["<label>"] { … }`: a quick fix of a constraint (DISL §8.5), or a fix on a range of an editor's text (§10.2). |
| `for` | Names the types a construct applies to (`constraint … for`, `operation … for`); `for <name> in <CEL> limit <n> { … }` is a loop (§12.2). A CEL reserved word. |
| `from` | In a relation type: its source types. In `connect`: the source. In a binding: `from <slot>`, the slot a value is read from (§11.3). |
| `hook` | `hook <id> on <event> [for <Types>] { … }`: statements run when an event happens (§7.11, DISL §9.2). |
| `import` | `import "<path>"` takes a part into the description; `import "<path>" as <name>` another description as a library (§5.3). A CEL reserved word. |
| `label` | `label <value>`: a display name or, in a notation, a label: `label <value> [as <id>] { … }` (§7.2, §8.4). |
| `many` | In an attribute: the flag `many`, a list of values (DISL §4.3). |
| `message` | In a constraint or reason: `message <value>`, its sentence. At the top: `message <std.name> <value>`, a standard message (§7.12). |
| `model` | `model { … }`: the tool type's metamodel: element and relation types, enums, data types and the document's attributes (§7.4). |
| `on` | In a hook: `on <event>`. In `layout`: `on <CEL>`, the scope it lays out. |
| `operation` | `operation <id> ["<label>"] [for <Types>] { … }`: a command, its statements run in one transaction (§7.10, DISL §9.3). |
| `ordered` | In `contains`: the flag `ordered`, the children's order is meaningful (DISL §4.8). |
| `param` | In an operation: `param <name>: <Type> …`, a parameter its form asks for (DISL §9.3). |
| `persistence` | `persistence { … }`: DISL's and DESL's persistence settings, ids among them (§7.13). |
| `plugin` | `plugin <name> { … }`: a DISL plugin declaration (§7.14); `call plugin <name>(…)` calls one. |
| `port` | In an element type: `port <name> { … }`, a port type. In a node notation: `port <Types> { … }`, its notation (DISL §4.10, §6.9). |
| `readonly` | In an attribute: the flag `readonly`, never written by an edit (DISL §4.3). |
| `reason` | `reason <id> "<text>"` or `reason <id> { when … message … }`: a named reason why something is refused or unavailable (§7.12). |
| `relation` | In `model`: `relation <Name> [from <Types> to <Types>] { … }`, a relation type (§7.5). In a binding: a relation rule (§11.3). |
| `required` | In an attribute: the flag `required` (DISL §4.3). |
| `requires` | `requires capability <name>@<range>`: a capability the description uses (§13.2). |
| `rule` | In a constraint: `rule <CEL>`, what must hold (DISL §8.2). |
| `severity` | In a constraint: `severity <name>` (`error`, `warning`, `info`, `hint`). In `notify`: the message's severity. |
| `to` | In a relation type: its target types. In `connect`, `move`, `resize`, `retype` and `reparent`: where to. |
| `unavailable` | In an operation: `unavailable <reasons>`, why it cannot be run (DISL §9.3). |
| `when` | A condition: of a constraint, an operation's context, a hook, a variant, a context tool set, a binding rule. |

### B.3 `script.lark`

| Keyword | Documentation |
|---|---|
| `after` | In `create`, `move`, `reparent` and `reorder`: the sibling the element is placed directly after (DISL §9.4, positional create; DESL §8.2). |
| `at` | In a binding rule: `at "<selector>"`, where the entries are (FBL §4.2). In `create`: the point the element is placed at. |
| `before` | In `create`, `move`, `reparent` and `reorder`: the sibling the element is placed directly before. |
| `break` | Leaves the nearest loop (§12.2). A CEL reserved word. |
| `by` | In `reorder`: the number of places to move by. In `outline`: the attribute that nests the outline. In `references`: the attribute that names the referenced entry, or `id`. |
| `call` | `call <operation>(<args>)` runs another operation; `call plugin <name>(<args>)` delegates to a plugin action (§12.3). |
| `connect` | `connect <Type> from <CEL> to <CEL> { … } [as <name>]`: creates a relation (§12.3, DISL §9.4). |
| `continue` | Continues the nearest loop with its next iteration (§12.2). A CEL reserved word. |
| `create` | `create <Type> { <attr>: <CEL> … } [in <CEL>] [after|before <CEL>] [as <name>]`: creates an element (§12.3, DISL §9.4). |
| `delete` | `delete <CEL>`: deletes an element or a list of them (§12.3). |
| `editLabel` | `editLabel <CEL> [{ label: <id> }]`: starts editing a label in place (§12.3). |
| `else` | The branch of `if` taken when its condition is false (§12.2). A CEL reserved word. |
| `fail` | `fail <CEL>`: refuses the gesture with the message and discards the transaction (§12.2, DISL's `abort`). |
| `fn` | `fn <name>(<params>) [-> <Type>] [recursion <n>] { … }`: a script function, callable from CEL (§12.1). |
| `highlight` | `highlight <CEL> [{ durationMs: <n> }]`: applies the `highlighted` state (§12.3). |
| `if` | `if <CEL> { … } [else …]`: a conditional (§12.2). A CEL reserved word. |
| `into` | In `move`: the new parent, `null` for the document's own elements. |
| `layout` | `layout { … }`: a diagram's layouts (§8.8). As a statement, `layout <name> [on <CEL>]`: runs a layout (§12.3). |
| `let` | `let <name> [: <Type>] = <CEL>`: an immutable local (§12.2). A CEL reserved word. |
| `limit` | In a loop: `limit <n>`, the most iterations it may take; reaching it stops the call (§12.2). |
| `move` | `move <CEL> [to <x>, <y>] [into <parent>] [slot <name>] [after <CEL> \| before <CEL>]`: changes an element's placement, its parent and its place among its siblings (§12.3, DESL §8.2). |
| `notify` | `notify <CEL> [severity <name>]`: shows a message (§12.3). |
| `open` | `open <CEL>`: opens a link, as DISL §9.4 says (§12.3). |
| `openForm` | `openForm <form> for <CEL>`: opens a form for an element (§12.3). |
| `place` | In `create`: the parent placement that chooses its parent and index (DISL §9.1). |
| `recursion` | In a function: `recursion <n>`, how deep it may recurse, at most 64 (§12.6). |
| `reorder` | `reorder <CEL> by|after|before <CEL>`: changes an element's place among its siblings (§12.3). |
| `reparent` | `reparent <CEL> to <CEL> …`: moves an element to another parent (§12.3). |
| `resize` | `resize <CEL> to <width>, <height>` (§12.3). |
| `return` | `return <CEL>` returns from a function; `return` last in a block ends the statements (§12.2). A CEL reserved word. |
| `retype` | `retype <CEL> to <Type>`: changes an element's type (§12.3). |
| `reveal` | `reveal <CEL>`: scrolls the canvas to elements (§12.3). |
| `select` | `select <CEL>`: changes the selection (§12.3). |
| `set` | `set <path> = <CEL>`: assigns an attribute (§12.3). |
| `size` | In `create`: the new element's size. |
| `slot` | In `create`, `move`, `reparent` and a binding's `parent`: the containment slot. |
| `unset` | `unset <path>`: removes an attribute's stored value (§12.3). |
| `var` | `var <name> [: <Type>] = <CEL>`: a mutable local (§12.2). A CEL reserved word. |
| `view` | `view <CEL> { <key>: <CEL> … }`: changes view data (§12.3, DISL §9.4). |
| `while` | `while <CEL> limit <n> { … }`: a loop (§12.2). A CEL reserved word. |

### B.4 `canvas.lark`

| Keyword | Documentation |
|---|---|
| `algorithm` | In `layout`: `algorithm <name> [capability <name>] { … }`, a layout algorithm: one of DISL's, named by its setting `algorithm`, or a capability of the `layout` extension point (§8.8). |
| `axis` | In `coordinates`: `axis <name> { … }`, an axis (DISL §5.3). |
| `badge` | In a node notation: `badge <id> { … }`, a badge (DISL §6.9). |
| `canvas` | In `notation`: `canvas { … }`, the canvas's background, header, legend, filters and notices (DISL §6.13). |
| `compartment` | In a node notation: `compartment <id> { … }`, a compartment (DISL §6.9). |
| `context` | In `toolbox`: `context [for <Types>] [when <CEL>] { … }`, a context tool set (DISL §7.3). |
| `coordinates` | `coordinates { … }`: the coordinate systems, axes and snapping profiles of a diagram (§8.1, DISL §5). |
| `defaults` | In `notation`: `defaults { … }`, the notation's defaults. |
| `edge` | In `notation`: `edge <Types> { … }`, the notation of relation types (§8.5, DISL §6.10). |
| `field` | In a form: `field <attribute> ["<label>"] { … }`, a form field for an attribute (DISL §7.5). |
| `form` | A form: `form <id> ["<label>"] [for <Types>] { … }` (§8.7, DISL §7.5). |
| `group` | In `toolbox`: `group <id> ["<label>"] { … }`, a group of tools (DISL §7.1). |
| `icon` | In `notation`: `icon <name> { … }`, an icon definition (DISL §6.14). |
| `item` | In a form: `item [<id>] ["<label>"] { … }`, a form item of any kind (DISL §7.5). |
| `line` | In an edge notation: `line { … }`, the line's stroke and routing (DISL §6.10). In a binding rule or block: `line "<regex>"`, the statement (FBL §4.6). |
| `marker` | In `notation`: `marker <name> { … }`, a marker definition. In an edge notation: `marker source|target|mid <value>` (DISL §6.11). |
| `mid` | In an edge notation: `marker mid <value>`, a marker along the edge. |
| `node` | In `notation`: `node <Types> { … }`, the notation of element types (§8.4, DISL §6.9). |
| `notation` | `notation { … }`: how a diagram is drawn: theme, styles, shapes, markers, node and edge notations (§8.3, DISL §6). |
| `section` | In a form: `section [<id>] ["<label>"] { … }`. In a binding of the `bullets` family: `section <name> "<heading>"` (FBL §4.8). |
| `shape` | In `notation`: `shape <name> { … }`, a custom shape. In a node notation: `shape <value> { … }`, the node's shape (DISL §6.7, §6.8). |
| `snapping` | In `coordinates`: `snapping <name> { … }`, a snapping profile (DISL §5.9). |
| `source` | In an edge notation: `marker source …`. In a relation rule: `source from <slot>`. |
| `state` | In a node or edge notation: `state <name> { … }`, the style of a state (DISL §6.6). |
| `style` | In `notation`: `style <name> [extends <names>] { … }`, a named style (DISL §6.6). |
| `system` | In `coordinates`: `system <name> { … }`, a coordinate system (DISL §5.7). |
| `target` | In an edge notation: `marker target …`. In a relation rule: `target from <slot>`. |
| `template` | In `toolbox`: `template <id> { … }`, a template (DISL §7.4). In a binding: `template <value>`, the bytes of a new body (FBL §13). |
| `theme` | In `notation`: `theme { … }`, the theme's tokens and modes (DISL §6.2). |
| `tool` | In `toolbox`: `tool <id> ["<label>"] { … }`, a tool; in `context`, a context tool (DISL §7.2, §7.3). |
| `toolbox` | `toolbox { … }`: a diagram's tools, groups, context tools and templates (§8.6, DISL §7). |
| `variant` | In a node or edge notation: `variant <name> [when <CEL>] { … }`, a variant (DISL §6.9). |
| `viewpoint` | `viewpoint <id> ["<label>"] { … }`: a viewpoint (§8.9, DISL §3.5). |

### B.5 `form.lark`

| Keyword | Documentation |
|---|---|
| `cells` | In `surface table`: `cells <Type> { … }`, the element type of the cells, with the attribute naming the column (DESL §7.3). |
| `columns` | In `surface table`: `columns <Type> { … }`, the element type of the columns, with the attributes that play their parts (DESL §7.3). |
| `convert` | In a value type: `convert <type> <conversion>`, what a value becomes when its column's type changes (DESL §7.6). |
| `entries` | In `surface hierarchy`: `entries <Type> { … }`, the entries' element type (DESL §7.7). |
| `filters` | In `surface hierarchy`: `filters <Type> { … }`, the stored filters' element type (DESL §7.7). |
| `hierarchy` | `surface hierarchy { … }`: a designer's surface of boxes that contain boxes (§9.3, DESL §7.7). |
| `labels` | In `surface hierarchy`: `labels <attribute> { … }`, an attribute whose values name members of a list (DESL §7.7). |
| `levels` | In `surface hierarchy`: `levels <Type> { … }`, the levels' element type (DESL §7.7). |
| `notes` | In `surface hierarchy`: `notes <Type> { … }`, the notes' element type (DESL §7.7). |
| `rows` | In `surface table`: `rows <Type> { … }`, the rows' element type (DESL §7.2). |
| `surface` | `surface table|hierarchy|text { … }`: a designer's or editor's surface (§9, §10). |
| `table` | `surface table { … }`: a designer's surface of rows and columns (§9.1, DESL §7.2). |
| `valueType` | In `surface table`: `valueType <key> ["<label>"] { … }`, a value type (DESL §7.5). |
| `views` | In `surface table`: `views <Type> { … }`, the views' element type (DESL §7.4). |
| `widths` | In `surface hierarchy`: `widths <Type> { … }`, the element type that keeps the widths of the columns of levels, with the attributes of a column's depth and its width (DESL §7.7.1). |

### B.6 `text.lark`

| Keyword | Documentation |
|---|---|
| `check` | In `surface text`: `check with <source> { … }`, a check whose findings the editor shows (§10.2). |
| `complete` | In `surface text`: `complete from <source> { … }`, a completion source (§10.2). |
| `files` | In `surface text`: the files an editor opens. In `claims`: the files a binding takes. In a binding rule: the file rules it reads (FBL §10). |
| `fold` | In `surface text`: `fold <Types> { … }`, the element types whose ranges fold (§10.2). |
| `outline` | In `surface text`: `outline <Types> [by <attribute>] { … }`, what the outline shows (§10.2). |
| `preview` | In `surface text`: `preview <source> { … }`, an embedded preview (§10.2). |
| `text` | `surface text { … }`: an editor's surface (§10). In a binding: `text { … }`, the conventions of new text (FBL §6.3). As a slot: the text of an element (FBL §5.2). |
| `token` | In `surface text`: `token <class> { … }`, a token class of the editor's own (§10.2). |
| `tokens` | In `surface text`: `tokens <syntax> { … }`, the syntax the token classes come from (§10.2). |
| `with` | In `surface text`: `check with <source>`. |

### B.7 `binding.lark`

| Keyword | Documentation |
|---|---|
| `binding` | `binding <name> [family <family>] { … }`: how the tool type's model is read from and written to a format, an FBL binding written in ATD (§11). |
| `block` | In a binding of the `blocks` family: `block <name> line "<regex>" { … }`, a statement that opens a block and produces nothing (FBL §4.7). |
| `body` | In a binding: `body { … }`, the body's kind, family and folder settings (FBL §3.2). |
| `claims` | In a binding: `claims { … }` or `claims files "<glob>", …`, which files the binding takes (FBL §12). |
| `family` | In a binding: `family <family>` (`yaml`, `json`, `xml`, `lines`, `blocks`, `bullets`, `grammar`) or `family capability <name>` (§11.1). |
| `grammar` | In a binding of the `grammar` family: `grammar { … }`, the LALR(1) grammar that reads the body, in ATD grammar notation, or `grammar from "<file>.lark"`, the same in a file beside the description (§11.9, FBL §4.9). |
| `header` | In a binding: `header { … }`, the version or identity mark the body starts with (FBL §5.6). |
| `id` | In a binding rule: `id from <slot>` or `id { … }`, where an element's id is stored (FBL §5.3). |
| `insert` | In a binding rule: `insert { … }`, how a new entry is written (FBL §6.2). |
| `parent` | In a binding rule: `parent <rules> [slot <name>]`, the rules whose entries contain this one's (FBL §5.5). |
| `references` | In an attribute binding: `references <rules> by <attribute|id>`, the value names an entry of those rules (FBL §5.7). |
| `registration` | In a binding: `registration { … }`, what the binding adds to the `.adp` registration (FBL §8). |
| `remove` | In a binding rule: `remove { … }`, how an entry is removed (FBL §6.2). |
| `within` | In a binding rule: `within <names>`, the blocks or sections it matches in (FBL §4.7, §4.8). |

### B.8 `cel.lark`

| Keyword | Documentation |
|---|---|
| `in` | In `create`: the parent. In `for`: what the loop iterates. In CEL: membership. A CEL reserved word. |

---

## Appendix C — DISL, DESL and FBL in ATD

Every property of DISL 0.6, DESL 0.2 and FBL 0.8, by the JSON pointer of its schema's `$defs`, with the ATD construct that carries it (FR-003); story 2 scenario 1 of spec 035 checks a tool description against it. A property no construct of its own carries is a setting of the construct that carries its object, checked against that object's `$defs` (§6.4, §14.6); a property of an object that is only ever a value (a stroke, a font, a paint) is a key of the settings map that value is written as.

### C.1 DISL 0.6

1520 properties.

| JSON pointer | ATD | Section |
|---|---|---|
| `disl.schema.json#/$defs/Reason/properties/when` | `when` of `reason`, or the key `when` of a reason value | §7.12 |
| `disl.schema.json#/$defs/Reason/properties/message` | `message` of `reason`, or the key `message` of a reason value | §7.12 |
| `disl.schema.json#/$defs/Reason/properties/id` | the name of `reason` | §7.12 |
| `disl.schema.json#/$defs/Reason/properties/doc` | the setting `doc` of `reason`, or a value of `unavailable` and other reasons | §7.12 |
| `disl.schema.json#/$defs/Reason/properties/reason` | the name of a `reason` a reason value names | §7.12 |
| `disl.schema.json#/$defs/NamedReason/properties/when` | `when` of `reason` | §7.12 |
| `disl.schema.json#/$defs/NamedReason/properties/message` | `message` of `reason` | §7.12 |
| `disl.schema.json#/$defs/NamedReason/properties/doc` | the setting `doc` of `reason` | §7.12 |
| `disl.schema.json#/$defs/Doc/properties/summary` | the setting `summary` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Doc/properties/description` | the setting `description` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Doc/properties/rationale` | the setting `rationale` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Doc/properties/examples` | the setting `examples` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Doc/properties/seeAlso` | the setting `seeAlso` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Doc/properties/tags` | the setting `tags` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Doc/properties/audience` | the setting `audience` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Doc/properties/since` | the setting `since` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Doc/properties/deprecated` | the setting `deprecated` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Doc/properties/image` | the setting `image` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Doc/properties/helpUrl` | the setting `helpUrl` of `doc` and `///` | §7.3 |
| `disl.schema.json#/$defs/Expression/properties/cel` | the key `cel` in a value of Expression (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Expression/properties/resultType` | the key `resultType` in a value of Expression (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Expression/properties/doc` | the key `doc` in a value of Expression (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/CelValue/properties/cel` | the key `cel` in a value of CelValue (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/CelValue/properties/resultType` | the key `resultType` in a value of CelValue (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/CelValue/properties/doc` | the key `doc` in a value of CelValue (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AttrBinding/properties/attribute` | the key `attribute` in a value of AttrBinding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AttrBinding/properties/offset` | the key `offset` in a value of AttrBinding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AttrBinding/properties/doc` | the key `doc` in a value of AttrBinding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TokenRef/properties/token` | the key `token` in a value of TokenRef (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ParamRef/properties/param` | the key `param` in a value of ParamRef (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Point/properties/x` | the key `x` in a value of Point (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Point/properties/y` | the key `y` in a value of Point (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Box/properties/x` | the key `x` in a value of Box (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Box/properties/y` | the key `y` in a value of Box (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Box/properties/w` | the key `w` in a value of Box (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Box/properties/h` | the key `h` in a value of Box (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MinMax/properties/min` | the key `min` in a value of MinMax (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MinMax/properties/max` | the key `max` in a value of MinMax (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MinMax/properties/doc` | the key `doc` in a value of MinMax (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PluginUse/properties/plugin` | the key `plugin` in a value of PluginUse (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PluginUse/properties/args` | the key `args` in a value of PluginUse (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PluginUse/properties/doc` | the key `doc` in a value of PluginUse (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PluginCall/properties/name` | the key `name` in a value of PluginCall (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PluginCall/properties/args` | the key `args` in a value of PluginCall (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PluginCall/properties/doc` | the key `doc` in a value of PluginCall (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Language/properties/id` | the setting `id` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/version` | `version "<semver>"` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/origin` | the origin of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/label` | `label` | §5.1 |
| `disl.schema.json#/$defs/Language/properties/defaultLocale` | the setting `defaultLocale` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/locales` | the setting `locales` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/authors` | the setting `authors` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/license` | the setting `license` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/homepage` | the setting `homepage` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/icon` | the setting `icon` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/fileExtension` | the setting `fileExtension` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/limits` | the setting `limits` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/requires` | `requires capability` for a capability; otherwise the setting `requires` | §5.1 |
| `disl.schema.json#/$defs/Language/properties/profiles` | the setting `profiles` of the tool construct | §5.1 |
| `disl.schema.json#/$defs/Language/properties/doc` | `doc` and `///` | §5.1 |
| `disl.schema.json#/$defs/BudgetWithhold/properties/edits` | the key `edits` in a value of BudgetWithhold (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/BudgetWithhold/properties/reason` | the key `reason` in a value of BudgetWithhold (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/BudgetWithhold/properties/doc` | the key `doc` in a value of BudgetWithhold (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Budget/properties/measure` | the key `measure` in a value of Budget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Budget/properties/max` | the key `max` in a value of Budget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Budget/properties/truncate` | the key `truncate` in a value of Budget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Budget/properties/order` | the key `order` in a value of Budget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Budget/properties/unit` | the key `unit` in a value of Budget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Budget/properties/notice` | the key `notice` in a value of Budget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Budget/properties/withhold` | the key `withhold` in a value of Budget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Budget/properties/finding` | the key `finding` in a value of Budget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Budget/properties/doc` | the key `doc` in a value of Budget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Import/properties/from` | the path of `import` | §5.3 |
| `disl.schema.json#/$defs/Import/properties/as` | `as` of `import` | §5.3 |
| `disl.schema.json#/$defs/Import/properties/version` | the setting `version` of `import … as` | §5.3 |
| `disl.schema.json#/$defs/Import/properties/integrity` | the setting `integrity` of `import … as` | §5.3 |
| `disl.schema.json#/$defs/Import/properties/include` | the setting `include` of `import … as` | §5.3 |
| `disl.schema.json#/$defs/Import/properties/doc` | the setting `doc` of `import … as` | §5.3 |
| `disl.schema.json#/$defs/Function/properties/params` | the parameters of `fn` | §12.1 |
| `disl.schema.json#/$defs/Function/properties/returns` | `-> Type` | §12.1 |
| `disl.schema.json#/$defs/Function/properties/cel` | the body of `fn`: `return <expression>` | §12.1 |
| `disl.schema.json#/$defs/Function/properties/uses` | the setting `uses` of `fn` | §12.1 |
| `disl.schema.json#/$defs/Function/properties/recursion` | `recursion n` | §12.1 |
| `disl.schema.json#/$defs/Function/properties/doc` | the setting `doc` of `fn` | §12.1 |
| `disl.schema.json#/$defs/Viewpoint/properties/label` | the setting `label` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/icon` | the setting `icon` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/coordinateSystem` | the setting `coordinateSystem` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/include` | the setting `include` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/exclude` | the setting `exclude` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/members` | the setting `members` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/toolbox` | the setting `toolbox` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/layout` | the setting `layout` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/notation` | the setting `notation` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/canvas` | the setting `canvas` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/variantOf` | the setting `variantOf` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/toggle` | the setting `toggle` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/default` | the setting `default` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Viewpoint/properties/doc` | the setting `doc` of `viewpoint` | §8.9 |
| `disl.schema.json#/$defs/Plugin/properties/version` | the setting `version` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Plugin/properties/provides` | the setting `provides` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Plugin/properties/required` | the setting `required` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Plugin/properties/fallback` | the setting `fallback` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Plugin/properties/args` | the setting `args` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Plugin/properties/source` | the setting `source` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Plugin/properties/celFunctions` | the setting `celFunctions` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Plugin/properties/plans` | the setting `plans` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Plugin/properties/add` | the setting `add` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Plugin/properties/move` | the setting `move` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Plugin/properties/doc` | the setting `doc` of `plugin` | §7.14 |
| `disl.schema.json#/$defs/Specification/properties/$schema` | none: `atd.schema.json` describes the tree, and a description has no `$schema` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/disl` | the header `atd <version>` (§5.1) | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/dedl` | the header `atd <version>` (§5.1) | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/language` | the tool construct (§5.1) | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/imports` | `import … as` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/functions` | `fn` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/metamodel` | `model { … }` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/coordinates` | `coordinates { … }` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/notation` | `notation { … }` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/toolbox` | `toolbox { … }` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/forms` | `form` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/constraints` | `constraints { … }` and `constraint` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/behavior` | `behavior { … }`, `operation`, `hook`, `reason`, `message` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/layout` | `layout { … }` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/persistence` | `persistence { … }` and `binding` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/viewpoints` | `viewpoint` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/plugins` | `plugin` | §5.1 |
| `disl.schema.json#/$defs/Specification/properties/doc` | `doc` and `///` | §5.1 |
| `disl.schema.json#/$defs/Attribute/properties/type` | `attribute name: Type` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/label` | the setting `label` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/required` | the flag `required` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/default` | `default value` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/many` | the flag `many` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/minItems` | the setting `minItems` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/maxItems` | the setting `maxItems` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/uniqueItems` | the setting `uniqueItems` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/readOnly` | the flag `readonly` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/fixed` | the setting `fixed` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/derived` | the setting `derived` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/transient` | the setting `transient` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/unique` | the setting `unique` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/key` | the setting `key` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/secret` | the setting `secret` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/facets` | the setting `facets` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/group` | the setting `group` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/order` | the setting `order` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/min` | the setting `min` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/max` | the setting `max` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/exclusiveMin` | the setting `exclusiveMin` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/exclusiveMax` | the setting `exclusiveMax` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/step` | the setting `step` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/precision` | the setting `precision` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/unit` | the setting `unit` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/minLength` | the setting `minLength` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/maxLength` | the setting `maxLength` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/pattern` | the setting `pattern` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/format` | the setting `format` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/markup` | the setting `markup` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/timezone` | the setting `timezone` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/writtenPrecision` | the setting `writtenPrecision` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/newPrecision` | the setting `newPrecision` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/calendar` | the setting `calendar` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/palette` | the setting `palette` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/alpha` | the setting `alpha` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/schemes` | the setting `schemes` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/paths` | the setting `paths` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/context` | the setting `context` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/resultType` | the setting `resultType` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/schema` | the setting `schema` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/mediaTypes` | the setting `mediaTypes` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/maxBytes` | the setting `maxBytes` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/samePrecisionAs` | the setting `samePrecisionAs` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/readOnlyReasons` | the setting `readOnlyReasons` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/absentText` | the setting `absentText` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/emptyText` | the setting `emptyText` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/outOfRange` | the setting `outOfRange` of `attribute` | §7.6 |
| `disl.schema.json#/$defs/Attribute/properties/doc` | `doc` and `///` | §7.6 |
| `disl.schema.json#/$defs/OutOfRange/properties/drag` | the key `drag` in a value of OutOfRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/OutOfRange/properties/typed` | the key `typed` in a value of OutOfRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/OutOfRange/properties/message` | the key `message` in a value of OutOfRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/OutOfRange/properties/doc` | the key `doc` in a value of OutOfRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NeighbourBounds/properties/attributes` | the key `attributes` in a value of NeighbourBounds (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NeighbourBounds/properties/between` | the key `between` in a value of NeighbourBounds (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NeighbourBounds/properties/gap` | the key `gap` in a value of NeighbourBounds (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NeighbourBounds/properties/inner` | the key `inner` in a value of NeighbourBounds (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NeighbourBounds/properties/absent` | the key `absent` in a value of NeighbourBounds (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NeighbourBounds/properties/on` | the key `on` in a value of NeighbourBounds (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NeighbourBounds/properties/pinned` | the key `pinned` in a value of NeighbourBounds (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NeighbourBounds/properties/doc` | the key `doc` in a value of NeighbourBounds (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DataType/properties/base` | the setting `base` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/fields` | the setting `fields` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/display` | the setting `display` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/label` | the setting `label` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/min` | the setting `min` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/max` | the setting `max` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/exclusiveMin` | the setting `exclusiveMin` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/exclusiveMax` | the setting `exclusiveMax` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/step` | the setting `step` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/precision` | the setting `precision` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/unit` | the setting `unit` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/minLength` | the setting `minLength` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/maxLength` | the setting `maxLength` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/pattern` | the setting `pattern` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/format` | the setting `format` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/markup` | the setting `markup` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/timezone` | the setting `timezone` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/calendar` | the setting `calendar` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/palette` | the setting `palette` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/alpha` | the setting `alpha` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/schemes` | the setting `schemes` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/context` | the setting `context` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/resultType` | the setting `resultType` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/schema` | the setting `schema` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/mediaTypes` | the setting `mediaTypes` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/maxBytes` | the setting `maxBytes` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/DataType/properties/doc` | the setting `doc` of `datatype` | §7.7 |
| `disl.schema.json#/$defs/EnumValue/properties/value` | the setting `value` of an `enum` value | §7.7 |
| `disl.schema.json#/$defs/EnumValue/properties/label` | the setting `label` of an `enum` value | §7.7 |
| `disl.schema.json#/$defs/EnumValue/properties/color` | the setting `color` of an `enum` value | §7.7 |
| `disl.schema.json#/$defs/EnumValue/properties/icon` | the setting `icon` of an `enum` value | §7.7 |
| `disl.schema.json#/$defs/EnumValue/properties/deprecated` | the setting `deprecated` of an `enum` value | §7.7 |
| `disl.schema.json#/$defs/EnumValue/properties/doc` | the setting `doc` of an `enum` value | §7.7 |
| `disl.schema.json#/$defs/Enum/properties/values` | the enum's values | §7.7 |
| `disl.schema.json#/$defs/Enum/properties/ordered` | the setting `ordered` of `enum` | §7.7 |
| `disl.schema.json#/$defs/Enum/properties/extensible` | the setting `extensible` of `enum` | §7.7 |
| `disl.schema.json#/$defs/Enum/properties/label` | the string after the enum's name | §7.7 |
| `disl.schema.json#/$defs/Enum/properties/doc` | the setting `doc` of `enum` | §7.7 |
| `disl.schema.json#/$defs/Containment/properties/allowed` | `contains Types` | §7.8 |
| `disl.schema.json#/$defs/Containment/properties/min` | the setting `min` of `contains` | §7.8 |
| `disl.schema.json#/$defs/Containment/properties/max` | the setting `max` of `contains` | §7.8 |
| `disl.schema.json#/$defs/Containment/properties/perType` | the setting `perType` of `contains` | §7.8 |
| `disl.schema.json#/$defs/Containment/properties/ordered` | the flag `ordered` of `contains` | §7.8 |
| `disl.schema.json#/$defs/Containment/properties/acrossViews` | the setting `acrossViews` of `contains` | §7.8 |
| `disl.schema.json#/$defs/Containment/properties/slots` | `contains Types as slot` | §7.8 |
| `disl.schema.json#/$defs/Containment/properties/doc` | the setting `doc` of `contains` | §7.8 |
| `disl.schema.json#/$defs/PortType/properties/label` | the setting `label` of `port` (model) | §7.5 |
| `disl.schema.json#/$defs/PortType/properties/direction` | the setting `direction` of `port` (model) | §7.5 |
| `disl.schema.json#/$defs/PortType/properties/multiplicity` | the setting `multiplicity` of `port` (model) | §7.5 |
| `disl.schema.json#/$defs/PortType/properties/dynamic` | the setting `dynamic` of `port` (model) | §7.5 |
| `disl.schema.json#/$defs/PortType/properties/maxConnections` | the setting `maxConnections` of `port` (model) | §7.5 |
| `disl.schema.json#/$defs/PortType/properties/accepts` | the setting `accepts` of `port` (model) | §7.5 |
| `disl.schema.json#/$defs/PortType/properties/attributes` | the setting `attributes` of `port` (model) | §7.5 |
| `disl.schema.json#/$defs/PortType/properties/doc` | the setting `doc` of `port` (model) | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/label` | `label` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/icon` | the setting `icon` of `element` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/abstract` | the setting `abstract` of `element` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/extends` | `extends` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/attributes` | `attribute` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/bounds` | the setting `bounds` of `element` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/ports` | `port` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/children` | `contains` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/multiplicity` | the setting `multiplicity` of `element` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/derived` | the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/viewOnly` | the setting `viewOnly` of `element` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/labelAttribute` | the setting `labelAttribute` of `element` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/tags` | the setting `tags` of `element` | §7.5 |
| `disl.schema.json#/$defs/NodeType/properties/doc` | `doc` and `///` | §7.5 |
| `disl.schema.json#/$defs/DerivedNode/properties/from` | the setting `from` of the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/DerivedNode/properties/key` | the setting `key` of the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/DerivedNode/properties/id` | the setting `id` of the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/DerivedNode/properties/attributes` | the setting `attributes` of the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/DerivedNode/properties/parent` | the setting `parent` of the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/DerivedNode/properties/slot` | the setting `slot` of the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/DerivedNode/properties/sources` | the setting `sources` of the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/DerivedNode/properties/reason` | the setting `reason` of the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/DerivedNode/properties/edits` | the setting `edits` of the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/DerivedNode/properties/doc` | the setting `doc` of the setting `derived` of `element` | §7.5 |
| `disl.schema.json#/$defs/RelationEnd/properties/types` | the setting `types` of the settings `source` and `target` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationEnd/properties/exclude` | the setting `exclude` of the settings `source` and `target` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationEnd/properties/ports` | the setting `ports` of the settings `source` and `target` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationEnd/properties/min` | the setting `min` of the settings `source` and `target` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationEnd/properties/max` | the setting `max` of the settings `source` and `target` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationEnd/properties/role` | the setting `role` of the settings `source` and `target` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationEnd/properties/optional` | the setting `optional` of the settings `source` and `target` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationEnd/properties/doc` | the setting `doc` of the settings `source` and `target` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/label` | `label` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/icon` | the setting `icon` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/abstract` | the setting `abstract` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/extends` | `extends` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/source` | `from` Types, with the setting `source` for the rest of the end | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/target` | `to` Types, with the setting `target` for the rest of the end | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/directed` | the setting `directed` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/allowSelfLoops` | the setting `allowSelfLoops` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/allowParallel` | the setting `allowParallel` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/acyclic` | the setting `acyclic` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/attributes` | `attribute` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/bounds` | the setting `bounds` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/connectsRelations` | the setting `connectsRelations` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/derived` | the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/viewOnly` | the setting `viewOnly` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/tags` | the setting `tags` of `relation` | §7.5 |
| `disl.schema.json#/$defs/RelationType/properties/doc` | `doc` and `///` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/from` | the setting `from` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/key` | the setting `key` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/id` | the setting `id` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/attributes` | the setting `attributes` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/source` | the setting `source` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/target` | the setting `target` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/owner` | the setting `owner` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/sources` | the setting `sources` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/reason` | the setting `reason` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/edits` | the setting `edits` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/DerivedRelation/properties/doc` | the setting `doc` of the setting `derived` of `relation` | §7.5 |
| `disl.schema.json#/$defs/Metamodel/properties/diagram` | `document { … }` | §7.4 |
| `disl.schema.json#/$defs/Metamodel/properties/dataTypes` | `datatype` | §7.4 |
| `disl.schema.json#/$defs/Metamodel/properties/enums` | `enum` | §7.4 |
| `disl.schema.json#/$defs/Metamodel/properties/types` | `element` | §7.4 |
| `disl.schema.json#/$defs/Metamodel/properties/relations` | `relation` | §7.4 |
| `disl.schema.json#/$defs/Metamodel/properties/doc` | `doc` and `///` | §7.4 |
| `disl.schema.json#/$defs/Calendar/properties/id` | the key `id` in a value of Calendar (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Calendar/properties/workingDays` | the key `workingDays` in a value of Calendar (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Calendar/properties/workingHours` | the key `workingHours` in a value of Calendar (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Calendar/properties/holidays` | the key `holidays` in a value of Calendar (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Calendar/properties/exceptions` | the key `exceptions` in a value of Calendar (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Calendar/properties/firstDayOfWeek` | the key `firstDayOfWeek` in a value of Calendar (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Calendar/properties/doc` | the key `doc` in a value of Calendar (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Category/properties/id` | the key `id` in a value of Category (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Category/properties/label` | the key `label` in a value of Category (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Category/properties/color` | the key `color` in a value of Category (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Category/properties/size` | the key `size` in a value of Category (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Category/properties/doc` | the key `doc` in a value of Category (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/visible` | the key `visible` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/position` | the key `position` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/size` | the key `size` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/levels` | the key `levels` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/ticks` | the key `ticks` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/boundaryFormats` | the key `boundaryFormats` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/attach` | the key `attach` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/minUnit` | the key `minUnit` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/format` | the key `format` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/nonWorkingStyle` | the key `nonWorkingStyle` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Ruler/properties/doc` | the key `doc` in a value of Ruler (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RulerTicks/properties/minSpacingPx` | the key `minSpacingPx` in a value of RulerTicks (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RulerTicks/properties/steps` | the key `steps` in a value of RulerTicks (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RulerTicks/properties/doc` | the key `doc` in a value of RulerTicks (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ZoomLevel/properties/label` | the key `label` in a value of ZoomLevel (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ZoomLevel/properties/scale` | the key `scale` in a value of ZoomLevel (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ZoomLevel/properties/ruler` | the key `ruler` in a value of ZoomLevel (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ZoomLevel/properties/doc` | the key `doc` in a value of ZoomLevel (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Axis/properties/kind` | the setting `kind` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/label` | the setting `label` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/min` | the setting `min` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/max` | the setting `max` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/reversed` | the setting `reversed` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/origin` | the setting `origin` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/outOfRange` | the setting `outOfRange` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/ranges` | the setting `ranges` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/endLabels` | the setting `endLabels` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/ruler` | the setting `ruler` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/expand` | the setting `expand` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/unit` | the setting `unit` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/scale` | the setting `scale` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/precision` | the setting `precision` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/base` | the setting `base` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/format` | the setting `format` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/valueType` | the setting `valueType` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/timezone` | the setting `timezone` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/calendar` | the setting `calendar` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/collapse` | the setting `collapse` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/zoomLevels` | the setting `zoomLevels` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/today` | the setting `today` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/categories` | the setting `categories` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/bandSize` | the setting `bandSize` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/minBandSize` | the setting `minBandSize` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/maxBandSize` | the setting `maxBandSize` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/bandSizes` | the setting `bandSizes` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/padding` | the setting `padding` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/gap` | the setting `gap` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/reorderable` | the setting `reorderable` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/allowUnassigned` | the setting `allowUnassigned` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/header` | the setting `header` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/start` | the setting `start` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/direction` | the setting `direction` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/wrap` | the setting `wrap` of `axis` | §8.1 |
| `disl.schema.json#/$defs/Axis/properties/doc` | the setting `doc` of `axis` | §8.1 |
| `disl.schema.json#/$defs/AxisRange/properties/id` | the key `id` in a value of AxisRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AxisRange/properties/label` | the key `label` in a value of AxisRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AxisRange/properties/from` | the key `from` in a value of AxisRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AxisRange/properties/to` | the key `to` in a value of AxisRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AxisRange/properties/fill` | the key `fill` in a value of AxisRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AxisRange/properties/style` | the key `style` in a value of AxisRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AxisRange/properties/separator` | the key `separator` in a value of AxisRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AxisRange/properties/labelPosition` | the key `labelPosition` in a value of AxisRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AxisRange/properties/doc` | the key `doc` in a value of AxisRange (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GridDisplay/properties/visible` | the key `visible` in a value of GridDisplay (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GridDisplay/properties/style` | the key `style` in a value of GridDisplay (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GridDisplay/properties/spacing` | the key `spacing` in a value of GridDisplay (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GridDisplay/properties/majorEvery` | the key `majorEvery` in a value of GridDisplay (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GridDisplay/properties/minorColor` | the key `minorColor` in a value of GridDisplay (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GridDisplay/properties/majorColor` | the key `majorColor` in a value of GridDisplay (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GridDisplay/properties/minZoomForMinor` | the key `minZoomForMinor` in a value of GridDisplay (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GridDisplay/properties/bands` | the key `bands` in a value of GridDisplay (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GridDisplay/properties/nonWorking` | the key `nonWorking` in a value of GridDisplay (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GridDisplay/properties/doc` | the key `doc` in a value of GridDisplay (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Guides/properties/smart` | the key `smart` in a value of Guides (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Guides/properties/distribution` | the key `distribution` in a value of Guides (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Guides/properties/user` | the key `user` in a value of Guides (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Guides/properties/style` | the key `style` in a value of Guides (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Guides/properties/doc` | the key `doc` in a value of Guides (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/kind` | the setting `kind` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/label` | the setting `label` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/x` | the setting `x` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/y` | the setting `y` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/angle` | the setting `angle` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/radius` | the setting `radius` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/center` | the setting `center` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/orientation` | the setting `orientation` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/bounds` | the setting `bounds` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/infinite` | the setting `infinite` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/grid` | the setting `grid` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/snapping` | the setting `snapping` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/guides` | the setting `guides` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/nested` | the setting `nested` of `system` | §8.1 |
| `disl.schema.json#/$defs/CoordinateSystem/properties/doc` | the setting `doc` of `system` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/grid` | the setting `grid` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/values` | the setting `values` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/calendar` | the setting `calendar` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/ticks` | the setting `ticks` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/bands` | the setting `bands` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/divisions` | the setting `divisions` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/ratio` | the setting `ratio` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/cel` | the setting `cel` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/byZoom` | the setting `byZoom` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/plugin` | the setting `plugin` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/args` | the setting `args` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/min` | the setting `min` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/max` | the setting `max` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/direction` | the setting `direction` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/ties` | the setting `ties` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/mode` | the setting `mode` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/SnapRuleObject/properties/doc` | the setting `doc` of a value of `snapping` | §8.1 |
| `disl.schema.json#/$defs/AxisRules/properties/x` | the key `x` in a value of AxisRules (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AxisRules/properties/y` | the key `y` in a value of AxisRules (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AxisRules/properties/both` | the key `both` in a value of AxisRules (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Snapping/properties/enabled` | the setting `enabled` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/userToggle` | the setting `userToggle` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/mode` | the setting `mode` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/x` | the setting `x` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/y` | the setting `y` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/both` | the setting `both` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/angle` | the setting `angle` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/radius` | the setting `radius` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/size` | the setting `size` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/reference` | the setting `reference` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/rotation` | the setting `rotation` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/bendpoints` | the setting `bendpoints` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/labels` | the setting `labels` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/targets` | the setting `targets` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/toleranceScreenPx` | the setting `toleranceScreenPx` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/bypassModifier` | the setting `bypassModifier` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/feedback` | the setting `feedback` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/applyToProgrammatic` | the setting `applyToProgrammatic` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/byGesture` | the setting `byGesture` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Snapping/properties/doc` | the setting `doc` of `snapping` | §8.1 |
| `disl.schema.json#/$defs/Coordinates/properties/axes` | `axis` | §8.1 |
| `disl.schema.json#/$defs/Coordinates/properties/systems` | `system` | §8.1 |
| `disl.schema.json#/$defs/Coordinates/properties/snapProfiles` | `snapping` | §8.1 |
| `disl.schema.json#/$defs/Coordinates/properties/default` | `default` | §8.1 |
| `disl.schema.json#/$defs/Coordinates/properties/doc` | the setting `doc` of `coordinates` | §8.1 |
| `disl.schema.json#/$defs/PlacementSource/properties/cel` | the key `cel` in a value of PlacementSource (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PlacementSource/properties/write` | the key `write` in a value of PlacementSource (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PlacementSource/properties/doc` | the key `doc` in a value of PlacementSource (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PlacementSource/properties/layout` | the key `layout` in a value of PlacementSource (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Placement/properties/system` | the setting `system` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/x` | the setting `x` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/y` | the setting `y` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/x2` | the setting `x2` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/y2` | the setting `y2` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/width` | the setting `width` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/height` | the setting `height` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/angle` | the setting `angle` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/radius` | the setting `radius` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/anchor` | the setting `anchor` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/movable` | the setting `movable` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/resizable` | the setting `resizable` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/stack` | the setting `stack` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/clampToParent` | the setting `clampToParent` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Placement/properties/doc` | the setting `doc` of the setting `placement` of `node` | §8.2 |
| `disl.schema.json#/$defs/Gradient/properties/type` | the key `type` in a value of Gradient (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Gradient/properties/angle` | the key `angle` in a value of Gradient (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Gradient/properties/center` | the key `center` in a value of Gradient (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Gradient/properties/radius` | the key `radius` in a value of Gradient (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Gradient/properties/stops` | the key `stops` in a value of Gradient (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Pattern/properties/type` | the key `type` in a value of Pattern (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Pattern/properties/pattern` | the key `pattern` in a value of Pattern (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Pattern/properties/color` | the key `color` in a value of Pattern (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Pattern/properties/background` | the key `background` in a value of Pattern (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Pattern/properties/spacing` | the key `spacing` in a value of Pattern (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Pattern/properties/angle` | the key `angle` in a value of Pattern (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Pattern/properties/width` | the key `width` in a value of Pattern (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ImagePaint/properties/type` | the key `type` in a value of ImagePaint (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ImagePaint/properties/src` | the key `src` in a value of ImagePaint (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ImagePaint/properties/fit` | the key `fit` in a value of ImagePaint (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ImagePaint/properties/opacity` | the key `opacity` in a value of ImagePaint (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/type` | the key `type` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/amplitude` | the key `amplitude` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/wavelength` | the key `wavelength` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/radius` | the key `radius` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/spacing` | the key `spacing` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/length` | the key `length` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/side` | the key `side` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/marker` | the key `marker` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/distance` | the key `distance` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/name` | the key `name` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/args` | the key `args` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineEffect/properties/doc` | the key `doc` in a value of LineEffect (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/color` | the key `color` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/width` | the key `width` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/minWidthScreenPx` | the key `minWidthScreenPx` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/opacity` | the key `opacity` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/dash` | the key `dash` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/dashOffset` | the key `dashOffset` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/dashScale` | the key `dashScale` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/cap` | the key `cap` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/join` | the key `join` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/miterLimit` | the key `miterLimit` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/align` | the key `align` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/double` | the key `double` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/casing` | the key `casing` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/effect` | the key `effect` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/flow` | the key `flow` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/sketch` | the key `sketch` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stroke/properties/doc` | the key `doc` in a value of Stroke (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/family` | the key `family` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/size` | the key `size` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/minSizeScreenPx` | the key `minSizeScreenPx` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/weight` | the key `weight` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/style` | the key `style` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/color` | the key `color` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/lineHeight` | the key `lineHeight` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/letterSpacing` | the key `letterSpacing` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/decoration` | the key `decoration` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/transform` | the key `transform` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/variant` | the key `variant` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Font/properties/halo` | the key `halo` in a value of Font (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Style/properties/extends` | `extends` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/fill` | the setting `fill` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/fillOpacity` | the setting `fillOpacity` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/stroke` | the setting `stroke` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/opacity` | the setting `opacity` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/cornerRadius` | the setting `cornerRadius` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/shadow` | the setting `shadow` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/font` | the setting `font` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/textAlign` | the setting `textAlign` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/verticalAlign` | the setting `verticalAlign` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/padding` | the setting `padding` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/cursor` | the setting `cursor` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/filter` | the setting `filter` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/blend` | the setting `blend` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/visible` | the setting `visible` of `style` | §8.3 |
| `disl.schema.json#/$defs/Style/properties/doc` | the setting `doc` of `style` | §8.3 |
| `disl.schema.json#/$defs/States/properties/hover` | the setting `hover` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/selected` | the setting `selected` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/focused` | the setting `focused` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/dragging` | the setting `dragging` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/highlighted` | the setting `highlighted` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/dropTarget` | the setting `dropTarget` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/dropReject` | the setting `dropReject` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/invalid` | the setting `invalid` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/warning` | the setting `warning` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/disabled` | the setting `disabled` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/editing` | the setting `editing` of `state` | §8.4 |
| `disl.schema.json#/$defs/States/properties/filteredOut` | the setting `filteredOut` of `state` | §8.4 |
| `disl.schema.json#/$defs/Theme/properties/tokens` | the setting `tokens` of `theme` | §8.3 |
| `disl.schema.json#/$defs/Theme/properties/modes` | the setting `modes` of `theme` | §8.3 |
| `disl.schema.json#/$defs/Theme/properties/defaultMode` | the setting `defaultMode` of `theme` | §8.3 |
| `disl.schema.json#/$defs/Theme/properties/followSystem` | the setting `followSystem` of `theme` | §8.3 |
| `disl.schema.json#/$defs/Theme/properties/extends` | the setting `extends` of `theme` | §8.3 |
| `disl.schema.json#/$defs/Theme/properties/contrast` | the setting `contrast` of `theme` | §8.3 |
| `disl.schema.json#/$defs/Theme/properties/doc` | the setting `doc` of `theme` | §8.3 |
| `disl.schema.json#/$defs/ContrastRequirement/properties/foreground` | the key `foreground` in a value of ContrastRequirement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContrastRequirement/properties/background` | the key `background` in a value of ContrastRequirement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContrastRequirement/properties/minRatio` | the key `minRatio` in a value of ContrastRequirement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContrastRequirement/properties/doc` | the key `doc` in a value of ContrastRequirement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IconDef/properties/svg` | the setting `svg` of `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/IconDef/properties/path` | the setting `path` of `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/IconDef/properties/stroke` | the setting `stroke` of `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/IconDef/properties/src` | the setting `src` of `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/IconDef/properties/size` | the setting `size` of `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/IconDef/properties/font` | the setting `font` of `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/IconDef/properties/glyph` | the setting `glyph` of `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/IconDef/properties/colored` | the setting `colored` of `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/IconDef/properties/label` | the setting `label` of `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/IconDef/properties/doc` | the setting `doc` of `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/IconRef/properties/icon` | the key `icon` in a value of IconRef (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IconRef/properties/flip` | the key `flip` in a value of IconRef (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeParam/properties/type` | the key `type` in a value of ShapeParam (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeParam/properties/default` | the key `default` in a value of ShapeParam (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeParam/properties/min` | the key `min` in a value of ShapeParam (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeParam/properties/max` | the key `max` in a value of ShapeParam (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeParam/properties/values` | the key `values` in a value of ShapeParam (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeParam/properties/unit` | the key `unit` in a value of ShapeParam (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeParam/properties/label` | the key `label` in a value of ShapeParam (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeParam/properties/persist` | the key `persist` in a value of ShapeParam (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeParam/properties/doc` | the key `doc` in a value of ShapeParam (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/op` | the key `op` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/x` | the key `x` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/y` | the key `y` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/cx` | the key `cx` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/cy` | the key `cy` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/c1x` | the key `c1x` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/c1y` | the key `c1y` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/c2x` | the key `c2x` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/c2y` | the key `c2y` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/rx` | the key `rx` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/ry` | the key `ry` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/rotation` | the key `rotation` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/w` | the key `w` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/h` | the key `h` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/r` | the key `r` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/largeArc` | the key `largeArc` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/sweep` | the key `sweep` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathSegment/properties/when` | the key `when` in a value of PathSegment (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathDef/properties/viewBox` | the key `viewBox` in a value of PathDef (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathDef/properties/d` | the key `d` in a value of PathDef (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathDef/properties/segments` | the key `segments` in a value of PathDef (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PathDef/properties/fillRule` | the key `fillRule` in a value of PathDef (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapePart/properties/id` | the key `id` in a value of ShapePart (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapePart/properties/shape` | the key `shape` in a value of ShapePart (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapePart/properties/box` | the key `box` in a value of ShapePart (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapePart/properties/style` | the key `style` in a value of ShapePart (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapePart/properties/rotate` | the key `rotate` in a value of ShapePart (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapePart/properties/when` | the key `when` in a value of ShapePart (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapePart/properties/outline` | the key `outline` in a value of ShapePart (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapePart/properties/hit` | the key `hit` in a value of ShapePart (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapePart/properties/tooltip` | the key `tooltip` in a value of ShapePart (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapePart/properties/doc` | the key `doc` in a value of ShapePart (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/param` | the key `param` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/x` | the key `x` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/y` | the key `y` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/axis` | the key `axis` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/value` | the key `value` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/yParam` | the key `yParam` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/yValue` | the key `yValue` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/snap` | the key `snap` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/cursor` | the key `cursor` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/style` | the key `style` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/label` | the key `label` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/visible` | the key `visible` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/write` | the key `write` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/refusals` | the key `refusals` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Handle/properties/doc` | the key `doc` in a value of Handle (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ScaleMode/properties/nineSlice` | the key `nineSlice` in a value of ScaleMode (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeDef/properties/label` | the setting `label` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/params` | the setting `params` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/path` | the setting `path` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/parts` | the setting `parts` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/svg` | the setting `svg` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/plugin` | the setting `plugin` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/outline` | the setting `outline` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/textArea` | the setting `textArea` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/handles` | the setting `handles` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/defaultSize` | the setting `defaultSize` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/aspectRatio` | the setting `aspectRatio` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/scaling` | the setting `scaling` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeDef/properties/doc` | the setting `doc` of `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/ShapeInstance/properties/type` | the key `type` in a value of ShapeInstance (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeInstance/properties/params` | the key `params` in a value of ShapeInstance (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ShapeInstance/properties/doc` | the key `doc` in a value of ShapeInstance (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Position/properties/anchor` | the key `anchor` in a value of Position (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Position/properties/offset` | the key `offset` in a value of Position (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Position/properties/align` | the key `align` in a value of Position (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Label/properties/id` | `as name` of `label` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/text` | the value after `label` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/format` | the setting `format` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/placeholder` | the setting `placeholder` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/editable` | the setting `editable` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/parse` | the setting `parse` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/editText` | the setting `editText` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/position` | the setting `position` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/style` | the setting `style` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/background` | the setting `background` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/maxWidth` | the setting `maxWidth` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/wrap` | the setting `wrap` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/maxLines` | the setting `maxLines` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/overflow` | the setting `overflow` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/rotation` | the setting `rotation` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/icon` | the setting `icon` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/visible` | the setting `visible` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/draggable` | the setting `draggable` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/link` | the setting `link` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/tooltip` | the setting `tooltip` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/layer` | the setting `layer` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/at` | the setting `at` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/offset` | the setting `offset` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/side` | the setting `side` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/distance` | the setting `distance` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/orientation` | the setting `orientation` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/avoidMarkers` | the setting `avoidMarkers` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/keepOnSegment` | the setting `keepOnSegment` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Label/properties/doc` | the setting `doc` of `label` of `node` or `edge` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/id` | the setting `id` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/title` | the setting `title` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/items` | the setting `items` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/itemText` | the setting `itemText` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/itemIcon` | the setting `itemIcon` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/itemStyle` | the setting `itemStyle` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/itemConditions` | the setting `itemConditions` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/itemLink` | the setting `itemLink` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/itemOrder` | the setting `itemOrder` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/groupBy` | the setting `groupBy` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/layout` | the setting `layout` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/separator` | the setting `separator` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/size` | the setting `size` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/overflow` | the setting `overflow` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/maxItems` | the setting `maxItems` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/collapsible` | the setting `collapsible` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/collapsed` | the setting `collapsed` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/emptyText` | the setting `emptyText` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/editable` | the setting `editable` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/visible` | the setting `visible` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/padding` | the setting `padding` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/style` | the setting `style` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/Compartment/properties/doc` | the setting `doc` of `compartment` | §8.4 |
| `disl.schema.json#/$defs/GroupBy/properties/attribute` | the key `attribute` in a value of GroupBy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GroupBy/properties/collapsed` | the key `collapsed` in a value of GroupBy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GroupBy/properties/count` | the key `count` in a value of GroupBy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GroupBy/properties/empty` | the key `empty` in a value of GroupBy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GroupBy/properties/title` | the key `title` in a value of GroupBy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GroupBy/properties/doc` | the key `doc` in a value of GroupBy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NodeIcon/properties/icon` | the key `icon` in a value of NodeIcon (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NodeIcon/properties/position` | the key `position` in a value of NodeIcon (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NodeIcon/properties/size` | the key `size` in a value of NodeIcon (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NodeIcon/properties/color` | the key `color` in a value of NodeIcon (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NodeIcon/properties/flip` | the key `flip` in a value of NodeIcon (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NodeIcon/properties/visible` | the key `visible` in a value of NodeIcon (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NodeIcon/properties/doc` | the key `doc` in a value of NodeIcon (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Badge/properties/id` | the setting `id` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/pack` | the setting `pack` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/position` | the setting `position` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/offset` | the setting `offset` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/shape` | the setting `shape` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/text` | the setting `text` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/icon` | the setting `icon` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/flip` | the setting `flip` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/style` | the setting `style` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/size` | the setting `size` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/visible` | the setting `visible` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/tooltip` | the setting `tooltip` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/onClick` | the setting `onClick` of `badge` | §8.4 |
| `disl.schema.json#/$defs/Badge/properties/doc` | the setting `doc` of `badge` | §8.4 |
| `disl.schema.json#/$defs/BadgeLayout/properties/start` | the key `start` in a value of BadgeLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/BadgeLayout/properties/offset` | the key `offset` in a value of BadgeLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/BadgeLayout/properties/direction` | the key `direction` in a value of BadgeLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/BadgeLayout/properties/spacing` | the key `spacing` in a value of BadgeLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/BadgeLayout/properties/doc` | the key `doc` in a value of BadgeLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/PortNotation/properties/shape` | the setting `shape` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/PortNotation/properties/size` | the setting `size` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/PortNotation/properties/style` | the setting `style` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/PortNotation/properties/states` | the setting `states` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/PortNotation/properties/side` | the setting `side` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/PortNotation/properties/position` | the setting `position` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/PortNotation/properties/inset` | the setting `inset` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/PortNotation/properties/movable` | the setting `movable` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/PortNotation/properties/label` | the setting `label` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/PortNotation/properties/order` | the setting `order` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/PortNotation/properties/doc` | the setting `doc` of `port` (notation) | §8.4 |
| `disl.schema.json#/$defs/AnchorSpec/properties/mode` | the key `mode` in a value of AnchorSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AnchorSpec/properties/gap` | the key `gap` in a value of AnchorSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AnchorSpec/properties/spread` | the key `spread` in a value of AnchorSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AnchorSpec/properties/sides` | the key `sides` in a value of AnchorSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AnchorSpec/properties/points` | the key `points` in a value of AnchorSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AnchorSpec/properties/drawnFrom` | the key `drawnFrom` in a value of AnchorSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/AnchorSpec/properties/doc` | the key `doc` in a value of AnchorSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/layout` | the key `layout` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/nesting` | the key `nesting` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/direction` | the key `direction` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/gap` | the key `gap` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/padding` | the key `padding` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/columns` | the key `columns` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/header` | the key `header` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/contentArea` | the key `contentArea` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/clip` | the key `clip` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/autoGrow` | the key `autoGrow` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/collapsible` | the key `collapsible` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/dropZones` | the key `dropZones` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/highlightOnDrop` | the key `highlightOnDrop` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ContainerSpec/properties/doc` | the key `doc` in a value of ContainerSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SizeSpec/properties/default` | the key `default` in a value of SizeSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SizeSpec/properties/min` | the key `min` in a value of SizeSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SizeSpec/properties/max` | the key `max` in a value of SizeSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SizeSpec/properties/fixed` | the key `fixed` in a value of SizeSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SizeSpec/properties/resizable` | the key `resizable` in a value of SizeSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SizeSpec/properties/aspectRatio` | the key `aspectRatio` in a value of SizeSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SizeSpec/properties/autoSize` | the key `autoSize` in a value of SizeSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SizeSpec/properties/width` | the key `width` in a value of SizeSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SizeSpec/properties/height` | the key `height` in a value of SizeSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SizeSpec/properties/doc` | the key `doc` in a value of SizeSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmbeddedForm/properties/form` | the key `form` in a value of EmbeddedForm (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmbeddedForm/properties/region` | the key `region` in a value of EmbeddedForm (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmbeddedForm/properties/interaction` | the key `interaction` in a value of EmbeddedForm (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmbeddedForm/properties/minZoom` | the key `minZoom` in a value of EmbeddedForm (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmbeddedForm/properties/fallback` | the key `fallback` in a value of EmbeddedForm (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmbeddedForm/properties/autoSize` | the key `autoSize` in a value of EmbeddedForm (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmbeddedForm/properties/style` | the key `style` in a value of EmbeddedForm (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmbeddedForm/properties/doc` | the key `doc` in a value of EmbeddedForm (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LodRule/properties/maxZoom` | the key `maxZoom` in a value of LodRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LodRule/properties/hide` | the key `hide` in a value of LodRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LodRule/properties/labels` | the key `labels` in a value of LodRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LodRule/properties/shape` | the key `shape` in a value of LodRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LodRule/properties/render` | the key `render` in a value of LodRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LodRule/properties/doc` | the key `doc` in a value of LodRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/shape` | `shape` | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/style` | the setting `style` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/partStyles` | the setting `partStyles` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/size` | the setting `size` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/placement` | the setting `placement` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/snapping` | the setting `snapping` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/rotation` | the setting `rotation` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/labels` | `label` | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/compartments` | `compartment` | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/form` | the setting `form` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/icon` | the setting `icon` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/badges` | `badge` | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/badgeLayout` | the setting `badgeLayout` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/ports` | `port` | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/anchors` | the setting `anchors` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/container` | the setting `container` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/variants` | `variant` | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/lod` | the setting `lod` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/states` | `state` | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/conditions` | the setting `conditions` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/tooltip` | the setting `tooltip` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/layer` | the setting `layer` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/selectable` | the setting `selectable` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/deletable` | the setting `deletable` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/copyable` | the setting `copyable` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/connectable` | the setting `connectable` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/hitPaddingScreenPx` | the setting `hitPaddingScreenPx` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/doubleClick` | the setting `doubleClick` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/drillDown` | the setting `drillDown` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/accessibility` | the setting `accessibility` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/refusals` | the setting `refusals` of `node` (notation) | §8.4 |
| `disl.schema.json#/$defs/NodeNotation/properties/doc` | `doc` and `///` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/when` | `when` of `variant` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/shape` | `shape` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/style` | the setting `style` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/partStyles` | the setting `partStyles` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/size` | the setting `size` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/snapping` | the setting `snapping` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/rotation` | the setting `rotation` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/labels` | `label` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/compartments` | `compartment` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/form` | the setting `form` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/icon` | the setting `icon` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/badges` | `badge` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/badgeLayout` | the setting `badgeLayout` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/ports` | `port` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/anchors` | the setting `anchors` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/container` | the setting `container` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/lod` | the setting `lod` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/states` | `state` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/conditions` | the setting `conditions` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/tooltip` | the setting `tooltip` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/layer` | the setting `layer` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/selectable` | the setting `selectable` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/deletable` | the setting `deletable` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/copyable` | the setting `copyable` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/connectable` | the setting `connectable` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/hitPaddingScreenPx` | the setting `hitPaddingScreenPx` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/doubleClick` | the setting `doubleClick` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/drillDown` | the setting `drillDown` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/accessibility` | the setting `accessibility` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/refusals` | the setting `refusals` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/NodeVariant/properties/doc` | the setting `doc` of `variant` of `node` | §8.4 |
| `disl.schema.json#/$defs/MarkerDef/properties/label` | the setting `label` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerDef/properties/path` | the setting `path` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerDef/properties/parts` | the setting `parts` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerDef/properties/ref` | the setting `ref` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerDef/properties/size` | the setting `size` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerDef/properties/params` | the setting `params` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerDef/properties/orient` | the setting `orient` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerDef/properties/fill` | the setting `fill` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerDef/properties/stroke` | the setting `stroke` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerDef/properties/shorten` | the setting `shorten` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerDef/properties/doc` | the setting `doc` of `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/MarkerInstance/properties/type` | the key `type` in a value of MarkerInstance (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MarkerInstance/properties/params` | the key `params` in a value of MarkerInstance (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MarkerInstance/properties/markers` | the key `markers` in a value of MarkerInstance (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MarkerInstance/properties/spacing` | the key `spacing` in a value of MarkerInstance (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MidMarker/properties/marker` | the key `marker` in a value of MidMarker (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MidMarker/properties/at` | the key `at` in a value of MidMarker (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MidMarker/properties/repeat` | the key `repeat` in a value of MidMarker (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MidMarker/properties/orient` | the key `orient` in a value of MidMarker (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MidMarker/properties/visible` | the key `visible` in a value of MidMarker (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MidMarker/properties/doc` | the key `doc` in a value of MidMarker (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LineSpec/properties/stroke` | the setting `stroke` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/routing` | the setting `routing` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/cornerRadius` | the setting `cornerRadius` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/curvature` | the setting `curvature` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/avoidNodes` | the setting `avoidNodes` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/avoidPadding` | the setting `avoidPadding` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/bendpoints` | the setting `bendpoints` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/segments` | the setting `segments` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/minSegmentLength` | the setting `minSegmentLength` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/startDirection` | the setting `startDirection` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/endDirection` | the setting `endDirection` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/bezier` | the setting `bezier` of `line` | §8.5 |
| `disl.schema.json#/$defs/LineSpec/properties/doc` | the setting `doc` of `line` | §8.5 |
| `disl.schema.json#/$defs/BezierSpec/properties/reach` | the key `reach` in a value of BezierSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/BezierSpec/properties/backward` | the key `backward` in a value of BezierSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/BezierSpec/properties/maxReach` | the key `maxReach` in a value of BezierSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/BezierSpec/properties/doc` | the key `doc` in a value of BezierSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EndAnchor/properties/mode` | the key `mode` in a value of EndAnchor (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EndAnchor/properties/gap` | the key `gap` in a value of EndAnchor (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EndAnchor/properties/sides` | the key `sides` in a value of EndAnchor (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EndAnchor/properties/points` | the key `points` in a value of EndAnchor (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EndAnchor/properties/part` | the key `part` in a value of EndAnchor (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EndAnchor/properties/side` | the key `side` in a value of EndAnchor (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EndAnchor/properties/at` | the key `at` in a value of EndAnchor (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EndAnchor/properties/movable` | the key `movable` in a value of EndAnchor (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EndAnchor/properties/default` | the key `default` in a value of EndAnchor (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EdgeNotation/properties/line` | `line` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/style` | the setting `style` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/sourceMarker` | `marker source` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/targetMarker` | `marker target` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/midMarkers` | `marker mid` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/stub` | the setting `stub` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/labels` | `label` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/anchoring` | the setting `anchoring` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/selfLoop` | the setting `selfLoop` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/parallel` | the setting `parallel` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/jumps` | the setting `jumps` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/snapping` | the setting `snapping` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/hitWidthScreenPx` | the setting `hitWidthScreenPx` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/layer` | the setting `layer` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/states` | `state` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/conditions` | the setting `conditions` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/variants` | `variant` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/tooltip` | the setting `tooltip` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/selectable` | the setting `selectable` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/deletable` | the setting `deletable` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/reconnectable` | the setting `reconnectable` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/accessibility` | the setting `accessibility` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/refusals` | the setting `refusals` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/connect` | the setting `connect` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeNotation/properties/doc` | the setting `doc` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/when` | `when` of `variant` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/line` | `line` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/style` | the setting `style` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/sourceMarker` | `marker source` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/targetMarker` | `marker target` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/midMarkers` | `marker mid` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/stub` | the setting `stub` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/labels` | `label` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/anchoring` | the setting `anchoring` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/selfLoop` | the setting `selfLoop` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/parallel` | the setting `parallel` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/jumps` | the setting `jumps` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/snapping` | the setting `snapping` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/hitWidthScreenPx` | the setting `hitWidthScreenPx` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/layer` | the setting `layer` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/states` | `state` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/conditions` | the setting `conditions` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/tooltip` | the setting `tooltip` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/selectable` | the setting `selectable` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/deletable` | the setting `deletable` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/reconnectable` | the setting `reconnectable` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/accessibility` | the setting `accessibility` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/refusals` | the setting `refusals` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/EdgeVariant/properties/doc` | the setting `doc` of `variant` of `edge` | §8.5 |
| `disl.schema.json#/$defs/Stub/properties/length` | the key `length` in a value of Stub (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stub/properties/side` | the key `side` in a value of Stub (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stub/properties/style` | the key `style` in a value of Stub (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stub/properties/label` | the key `label` in a value of Stub (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Stub/properties/doc` | the key `doc` in a value of Stub (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GestureRefusals/properties/move` | the key `move` in a value of GestureRefusals (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GestureRefusals/properties/resize` | the key `resize` in a value of GestureRefusals (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GestureRefusals/properties/reparent` | the key `reparent` in a value of GestureRefusals (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GestureRefusals/properties/delete` | the key `delete` in a value of GestureRefusals (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GestureRefusals/properties/connect` | the key `connect` in a value of GestureRefusals (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GestureRefusals/properties/reconnect` | the key `reconnect` in a value of GestureRefusals (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GestureRefusals/properties/copy` | the key `copy` in a value of GestureRefusals (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GestureRefusals/properties/editLabel` | the key `editLabel` in a value of GestureRefusals (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GestureRefusals/properties/doc` | the key `doc` in a value of GestureRefusals (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ConnectGesture/properties/from` | the key `from` in a value of ConnectGesture (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ConnectGesture/properties/tool` | the key `tool` in a value of ConnectGesture (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ConnectGesture/properties/pointer` | the key `pointer` in a value of ConnectGesture (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ConnectGesture/properties/doc` | the key `doc` in a value of ConnectGesture (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GesturePointer/properties/button` | the key `button` in a value of GesturePointer (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GesturePointer/properties/start` | the key `start` in a value of GesturePointer (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GesturePointer/properties/modifiers` | the key `modifiers` in a value of GesturePointer (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/GesturePointer/properties/doc` | the key `doc` in a value of GesturePointer (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Canvas/properties/background` | the setting `background` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/page` | the setting `page` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/bounds` | the setting `bounds` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/zoom` | the setting `zoom` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/watermark` | the setting `watermark` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/legend` | the setting `legend` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/minimap` | the setting `minimap` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/selection` | the setting `selection` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/title` | the setting `title` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/header` | the setting `header` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/notices` | the setting `notices` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/filters` | the setting `filters` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/empty` | the setting `empty` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/fit` | the setting `fit` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/margin` | the setting `margin` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/Canvas/properties/doc` | the setting `doc` of `canvas` | §8.3 |
| `disl.schema.json#/$defs/ChromeBand/properties/rows` | the key `rows` in a value of ChromeBand (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ChromeBand/properties/style` | the key `style` in a value of ChromeBand (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ChromeBand/properties/visible` | the key `visible` in a value of ChromeBand (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ChromeBand/properties/doc` | the key `doc` in a value of ChromeBand (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Notice/properties/id` | the key `id` in a value of Notice (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Notice/properties/text` | the key `text` in a value of Notice (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Notice/properties/severity` | the key `severity` in a value of Notice (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Notice/properties/position` | the key `position` in a value of Notice (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Notice/properties/visible` | the key `visible` in a value of Notice (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Notice/properties/actions` | the key `actions` in a value of Notice (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Notice/properties/dismissible` | the key `dismissible` in a value of Notice (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Notice/properties/style` | the key `style` in a value of Notice (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Notice/properties/doc` | the key `doc` in a value of Notice (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/label` | the key `label` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/control` | the key `control` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/appliesTo` | the key `appliesTo` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/options` | the key `options` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/default` | the key `default` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/match` | the key `match` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/keep` | the key `keep` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/effect` | the key `effect` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/position` | the key `position` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/persist` | the key `persist` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Filter/properties/doc` | the key `doc` in a value of Filter (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmptyMessage/properties/text` | the key `text` in a value of EmptyMessage (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmptyMessage/properties/when` | the key `when` in a value of EmptyMessage (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmptyMessage/properties/style` | the key `style` in a value of EmptyMessage (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/EmptyMessage/properties/doc` | the key `doc` in a value of EmptyMessage (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LegendComputed/properties/key` | the key `key` in a value of LegendComputed (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LegendComputed/properties/label` | the key `label` in a value of LegendComputed (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LegendComputed/properties/swatch` | the key `swatch` in a value of LegendComputed (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LegendComputed/properties/order` | the key `order` in a value of LegendComputed (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/LegendComputed/properties/doc` | the key `doc` in a value of LegendComputed (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Notation/properties/theme` | `theme` | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/fonts` | the setting `fonts` of `notation` | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/textMetric` | the setting `textMetric` of `notation` | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/styles` | `style` | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/shapes` | `shape` (notation) | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/markers` | `marker` (notation) | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/icons` | `icon` (notation) | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/nodes` | `node` | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/edges` | `edge` | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/ports` | `port` (notation) | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/canvas` | `canvas` | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/defaults` | `defaults` | §8.3 |
| `disl.schema.json#/$defs/Notation/properties/doc` | the setting `doc` of `notation` | §8.3 |
| `disl.schema.json#/$defs/TextMetric/properties/kind` | the key `kind` in a value of TextMetric (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TextMetric/properties/advance` | the key `advance` in a value of TextMetric (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TextMetric/properties/count` | the key `count` in a value of TextMetric (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TextMetric/properties/lineHeight` | the key `lineHeight` in a value of TextMetric (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Tool/properties/id` | the name of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/kind` | the setting `kind` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/label` | the string after the tool's name | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/icon` | the setting `icon` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/creates` | the setting `creates` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/initial` | the setting `initial` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/size` | the setting `size` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/variantOf` | the setting `variantOf` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/mode` | the setting `mode` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/sticky` | the setting `sticky` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/shortcut` | the setting `shortcut` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/enabled` | the setting `enabled` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/visible` | the setting `visible` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/unavailable` | the setting `unavailable` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/preview` | the setting `preview` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/after` | the setting `after` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/template` | the setting `template` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/operation` | the setting `operation` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/plugin` | the setting `plugin` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/drop` | the setting `drop` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/createSource` | the setting `createSource` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/createTarget` | the setting `createTarget` of `tool` | §8.6 |
| `disl.schema.json#/$defs/Tool/properties/doc` | the setting `doc` of `tool` | §8.6 |
| `disl.schema.json#/$defs/CreateEnd/properties/type` | the key `type` in a value of CreateEnd (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/CreateEnd/properties/initial` | the key `initial` in a value of CreateEnd (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/CreateEnd/properties/size` | the key `size` in a value of CreateEnd (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/CreateEnd/properties/doc` | the key `doc` in a value of CreateEnd (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropSpec/properties/targets` | the key `targets` in a value of DropSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropSpec/properties/elsewhere` | the key `elsewhere` in a value of DropSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropSpec/properties/refusal` | the key `refusal` in a value of DropSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropSpec/properties/place` | the key `place` in a value of DropSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropSpec/properties/doc` | the key `doc` in a value of DropSpec (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ParentPlacement/properties/candidates` | the key `candidates` in a value of ParentPlacement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ParentPlacement/properties/as` | the key `as` in a value of ParentPlacement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ParentPlacement/properties/rank` | the key `rank` in a value of ParentPlacement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ParentPlacement/properties/index` | the key `index` in a value of ParentPlacement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ParentPlacement/properties/whenEmpty` | the key `whenEmpty` in a value of ParentPlacement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ParentPlacement/properties/refusal` | the key `refusal` in a value of ParentPlacement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ParentPlacement/properties/doc` | the key `doc` in a value of ParentPlacement (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/on` | the key `on` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/kind` | the key `kind` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/creates` | the key `creates` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/via` | the key `via` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/direction` | the key `direction` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/slot` | the key `slot` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/operation` | the key `operation` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/plugin` | the key `plugin` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/label` | the key `label` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/icon` | the key `icon` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/shortcut` | the key `shortcut` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/enabled` | the key `enabled` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/visible` | the key `visible` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/unavailable` | the key `unavailable` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/position` | the key `position` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/target` | the key `target` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/candidates` | the key `candidates` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/candidateLabel` | the key `candidateLabel` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/pickerTitle` | the key `pickerTitle` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/emptyText` | the key `emptyText` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/forEach` | the key `forEach` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/as` | the key `as` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/args` | the key `args` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/submenu` | the key `submenu` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DropTarget/properties/doc` | the key `doc` in a value of DropTarget (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ToolGroup/properties/id` | the name of `group` | §8.6 |
| `disl.schema.json#/$defs/ToolGroup/properties/label` | the string after the group's name | §8.6 |
| `disl.schema.json#/$defs/ToolGroup/properties/icon` | the setting `icon` of `group` | §8.6 |
| `disl.schema.json#/$defs/ToolGroup/properties/collapsed` | the setting `collapsed` of `group` | §8.6 |
| `disl.schema.json#/$defs/ToolGroup/properties/visible` | the setting `visible` of `group` | §8.6 |
| `disl.schema.json#/$defs/ToolGroup/properties/viewpoints` | the setting `viewpoints` of `group` | §8.6 |
| `disl.schema.json#/$defs/ToolGroup/properties/tools` | `tool` | §8.6 |
| `disl.schema.json#/$defs/ToolGroup/properties/groups` | `group` | §8.6 |
| `disl.schema.json#/$defs/ToolGroup/properties/doc` | the setting `doc` of `group` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/kind` | the setting `kind` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/creates` | the setting `creates` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/via` | the setting `via` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/direction` | the setting `direction` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/slot` | the setting `slot` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/operation` | the setting `operation` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/plugin` | the setting `plugin` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/label` | the setting `label` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/icon` | the setting `icon` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/shortcut` | the setting `shortcut` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/enabled` | the setting `enabled` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/visible` | the setting `visible` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/unavailable` | the setting `unavailable` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/position` | the setting `position` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/target` | the setting `target` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/candidates` | the setting `candidates` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/candidateLabel` | the setting `candidateLabel` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/pickerTitle` | the setting `pickerTitle` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/emptyText` | the setting `emptyText` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/forEach` | the setting `forEach` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/as` | the setting `as` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/args` | the setting `args` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/submenu` | the setting `submenu` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/group` | the setting `group` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextTool/properties/doc` | the setting `doc` of `tool` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextToolSet/properties/for` | `for` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextToolSet/properties/when` | `when` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextToolSet/properties/placement` | the setting `placement` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextToolSet/properties/tools` | `tool` | §8.6 |
| `disl.schema.json#/$defs/ContextToolSet/properties/standardEntries` | the setting `standardEntries` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextToolSet/properties/runSingle` | the setting `runSingle` of `context` | §8.6 |
| `disl.schema.json#/$defs/ContextToolSet/properties/doc` | the setting `doc` of `context` | §8.6 |
| `disl.schema.json#/$defs/FragmentNode/properties/ref` | the key `ref` in a value of FragmentNode (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentNode/properties/type` | the key `type` in a value of FragmentNode (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentNode/properties/attributes` | the key `attributes` in a value of FragmentNode (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentNode/properties/parent` | the key `parent` in a value of FragmentNode (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentNode/properties/slot` | the key `slot` in a value of FragmentNode (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentNode/properties/at` | the key `at` in a value of FragmentNode (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentNode/properties/size` | the key `size` in a value of FragmentNode (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentRelation/properties/ref` | the key `ref` in a value of FragmentRelation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentRelation/properties/type` | the key `type` in a value of FragmentRelation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentRelation/properties/source` | the key `source` in a value of FragmentRelation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentRelation/properties/target` | the key `target` in a value of FragmentRelation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FragmentRelation/properties/attributes` | the key `attributes` in a value of FragmentRelation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Template/properties/label` | the string after the template's name | §8.6 |
| `disl.schema.json#/$defs/Template/properties/icon` | the setting `icon` of `template` (toolbox) | §8.6 |
| `disl.schema.json#/$defs/Template/properties/params` | the setting `params` of `template` (toolbox) | §8.6 |
| `disl.schema.json#/$defs/Template/properties/fragment` | the setting `fragment` of `template` (toolbox) | §8.6 |
| `disl.schema.json#/$defs/Template/properties/preview` | the setting `preview` of `template` (toolbox) | §8.6 |
| `disl.schema.json#/$defs/Template/properties/doc` | the setting `doc` of `template` (toolbox) | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/groups` | `group` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/tools` | `tool` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/layout` | the setting `layout` of `toolbox` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/position` | the setting `position` of `toolbox` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/searchable` | the setting `searchable` of `toolbox` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/showRecent` | the setting `showRecent` of `toolbox` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/generic` | the setting `generic` of `toolbox` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/contextTools` | `context` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/contextMenus` | `context` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/templates` | `template` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/starters` | the setting `starters` of `toolbox` | §8.6 |
| `disl.schema.json#/$defs/Toolbox/properties/doc` | the setting `doc` of `toolbox` | §8.6 |
| `disl.schema.json#/$defs/FieldValidation/properties/rule` | the key `rule` in a value of FieldValidation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FieldValidation/properties/message` | the key `message` in a value of FieldValidation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FieldValidation/properties/severity` | the key `severity` in a value of FieldValidation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FieldValidation/properties/timing` | the key `timing` in a value of FieldValidation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FieldParse/properties/accepts` | the key `accepts` in a value of FieldParse (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FieldParse/properties/write` | the key `write` in a value of FieldParse (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FieldParse/properties/refusal` | the key `refusal` in a value of FieldParse (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FieldParse/properties/doc` | the key `doc` in a value of FieldParse (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/FormItem/properties/kind` | the setting `kind` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/id` | the name of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/attribute` | the name of `field` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/widget` | the setting `widget` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/label` | the string after the item's name | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/placeholder` | the setting `placeholder` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/visible` | the setting `visible` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/enabled` | the setting `enabled` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/required` | the setting `required` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/validate` | the setting `validate` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/options` | the setting `options` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/widgetOptions` | the setting `widgetOptions` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/colSpan` | the setting `colSpan` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/width` | the setting `width` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/prefix` | the setting `prefix` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/suffix` | the setting `suffix` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/unit` | the setting `unit` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/format` | the setting `format` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/rows` | the setting `rows` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/display` | the setting `display` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/parse` | the setting `parse` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/optionLabel` | the setting `optionLabel` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/refusals` | the setting `refusals` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/readOnly` | the setting `readOnly` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/onChange` | the setting `onChange` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/title` | the setting `title` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/collapsible` | the setting `collapsible` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/collapsed` | the setting `collapsed` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/items` | the members of `section` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/tabs` | the setting `tabs` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/text` | the setting `text` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/value` | the setting `value` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/icon` | the setting `icon` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/operation` | the setting `operation` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/actions` | the setting `actions` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/confirm` | the setting `confirm` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/readOnlyReasons` | the setting `readOnlyReasons` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/absentText` | the setting `absentText` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/emptyText` | the setting `emptyText` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/showAbsent` | the setting `showAbsent` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/initial` | the setting `initial` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/unavailable` | the setting `unavailable` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/plugin` | the setting `plugin` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/FormItem/properties/doc` | the setting `doc` of `field`, `section` or `item` | §8.7 |
| `disl.schema.json#/$defs/Form/properties/for` | `for` of `form` | §8.7 |
| `disl.schema.json#/$defs/Form/properties/usage` | the setting `usage` of `form` | §8.7 |
| `disl.schema.json#/$defs/Form/properties/layout` | the setting `layout` of `form` | §8.7 |
| `disl.schema.json#/$defs/Form/properties/items` | `field`, `section` and `item` | §8.7 |
| `disl.schema.json#/$defs/Form/properties/commit` | the setting `commit` of `form` | §8.7 |
| `disl.schema.json#/$defs/Form/properties/label` | the string after the form's name | §8.7 |
| `disl.schema.json#/$defs/Form/properties/icon` | the setting `icon` of `form` | §8.7 |
| `disl.schema.json#/$defs/Form/properties/submitLabel` | the setting `submitLabel` of `form` | §8.7 |
| `disl.schema.json#/$defs/Form/properties/cancelLabel` | the setting `cancelLabel` of `form` | §8.7 |
| `disl.schema.json#/$defs/Form/properties/danger` | the setting `danger` of `form` | §8.7 |
| `disl.schema.json#/$defs/Form/properties/doc` | the setting `doc` of `form` | §8.7 |
| `disl.schema.json#/$defs/QuickFix/properties/label` | the string after the fix's name | §7.9 |
| `disl.schema.json#/$defs/QuickFix/properties/when` | `when` of `fix` | §7.9 |
| `disl.schema.json#/$defs/QuickFix/properties/actions` | the statements of `fix` | §7.9 |
| `disl.schema.json#/$defs/QuickFix/properties/operation` | the setting `operation` of `fix` | §7.9 |
| `disl.schema.json#/$defs/QuickFix/properties/preferred` | the setting `preferred` of `fix` | §7.9 |
| `disl.schema.json#/$defs/QuickFix/properties/applyToAll` | the setting `applyToAll` of `fix` | §7.9 |
| `disl.schema.json#/$defs/QuickFix/properties/doc` | the setting `doc` of `fix` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/id` | the name of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/code` | the setting `code` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/label` | `label` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/kind` | the setting `kind` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/scope` | `for` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/over` | the setting `over` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/forEach` | the setting `forEach` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/when` | `when` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/rule` | `rule` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/severity` | `severity` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/message` | `message` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/target` | the setting `target` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/attribute` | the setting `attribute` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/location` | the setting `location` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/subject` | the setting `subject` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/timing` | the setting `timing` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/enforcement` | the setting `enforcement` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/fixes` | `fix` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/suppressible` | the setting `suppressible` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/enabled` | the setting `enabled` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/group` | the setting `group` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/tags` | the setting `tags` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/cost` | the setting `cost` of `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraint/properties/doc` | `doc` and `///` | §7.9 |
| `disl.schema.json#/$defs/Constraints/properties/rules` | `constraint` | §7.9 |
| `disl.schema.json#/$defs/Constraints/properties/groups` | the setting `groups` of `constraints` | §7.9 |
| `disl.schema.json#/$defs/Constraints/properties/defaults` | the setting `defaults` of `constraints` | §7.9 |
| `disl.schema.json#/$defs/Constraints/properties/blockSaveOn` | the setting `blockSaveOn` of `constraints` | §7.9 |
| `disl.schema.json#/$defs/Constraints/properties/builtIn` | `builtin` | §7.9 |
| `disl.schema.json#/$defs/Constraints/properties/order` | the setting `order` of `constraints` | §7.9 |
| `disl.schema.json#/$defs/Constraints/properties/doc` | the setting `doc` of `constraints` | §7.9 |
| `disl.schema.json#/$defs/BuiltInSetting/properties/severity` | the setting `severity` of `builtin` | §7.9 |
| `disl.schema.json#/$defs/BuiltInSetting/properties/enabled` | the setting `enabled` of `builtin` | §7.9 |
| `disl.schema.json#/$defs/BuiltInSetting/properties/code` | the setting `code` of `builtin` | §7.9 |
| `disl.schema.json#/$defs/BuiltInSetting/properties/message` | the setting `message` of `builtin` | §7.9 |
| `disl.schema.json#/$defs/BuiltInSetting/properties/refusal` | the setting `refusal` of `builtin` | §7.9 |
| `disl.schema.json#/$defs/BuiltInSetting/properties/oncePerGroup` | the setting `oncePerGroup` of `builtin` | §7.9 |
| `disl.schema.json#/$defs/BuiltInSetting/properties/doc` | the setting `doc` of `builtin` | §7.9 |
| `disl.schema.json#/$defs/SourceLocation/properties/file` | the key `file` in a value of SourceLocation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SourceLocation/properties/line` | the key `line` in a value of SourceLocation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SourceLocation/properties/column` | the key `column` in a value of SourceLocation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/SourceLocation/properties/length` | the key `length` in a value of SourceLocation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/constraintId` | the key `constraintId` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/code` | the key `code` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/severity` | the key `severity` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/message` | the key `message` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/elementId` | the key `elementId` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/elementIds` | the key `elementIds` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/attribute` | the key `attribute` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/location` | the key `location` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/subject` | the key `subject` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/view` | the key `view` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Finding/properties/pointer` | the key `pointer` in a value of Finding (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ValidatorOutput/properties/$schema` | the key `$schema` in a value of ValidatorOutput (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ValidatorOutput/properties/findings` | the key `findings` in a value of ValidatorOutput (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Action/properties/set` | `set path = expression` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/unset` | `unset path` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/target` | the target of the action's statement | §12.3 |
| `disl.schema.json#/$defs/Action/properties/create` | `create Type { … } in … after … as name` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/connect` | `connect Type from … to … { … } as name` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/as` | `as name` of `create` and `connect`, the loop variable of `for` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/delete` | `delete` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/move` | `move … to x, y into parent` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/resize` | `resize … to width, height` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/retype` | `retype … to Type` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/reparent` | `reparent … to parent` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/reorder` | `reorder … by|after|before …` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/let` | `let name = expression` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/if` | `if expression { … }` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/then` | the block of `if` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/else` | `else { … }` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/forEach` | `for name in expression limit n { … }` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/do` | the block of `for` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/view` | `view target { … }` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/open` | `open` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/select` | `select` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/reveal` | `reveal` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/highlight` | `highlight … { durationMs: … }` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/durationMs` | the setting `durationMs` of `highlight` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/editLabel` | `editLabel … { label: … }` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/openForm` | `openForm form for target` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/notify` | `notify … severity …` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/layout` | `layout name on scope` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/abort` | `fail message` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/call` | `call operation(args)` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/args` | the arguments of `call` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/plugin` | `call plugin name(args)` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/when` | `if expression { action }` | §12.3 |
| `disl.schema.json#/$defs/Action/properties/doc` | a `//` comment | §12.3 |
| `disl.schema.json#/$defs/Hook/properties/id` | the name of `hook` | §7.11 |
| `disl.schema.json#/$defs/Hook/properties/on` | `on` of `hook` | §7.11 |
| `disl.schema.json#/$defs/Hook/properties/for` | `for` of `hook` | §7.11 |
| `disl.schema.json#/$defs/Hook/properties/attribute` | the setting `attribute` of `hook` | §7.11 |
| `disl.schema.json#/$defs/Hook/properties/when` | `when` of `hook` | §7.11 |
| `disl.schema.json#/$defs/Hook/properties/actions` | the statements of `hook` | §7.11 |
| `disl.schema.json#/$defs/Hook/properties/phase` | the setting `phase` of `hook` | §7.11 |
| `disl.schema.json#/$defs/Hook/properties/order` | the setting `order` of `hook` | §7.11 |
| `disl.schema.json#/$defs/Hook/properties/doc` | the setting `doc` of `hook` | §7.11 |
| `disl.schema.json#/$defs/Operation/properties/label` | the string after the operation's name, or `label` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/icon` | the setting `icon` of `operation` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/shortcut` | the setting `shortcut` of `operation` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/for` | `for` of `operation` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/params` | `param` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/paramsForm` | the setting `paramsForm` of `operation` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/enabled` | `enabled` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/unavailable` | `unavailable` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/confirm` | `confirm` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/actions` | the statements of `operation` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/plugin` | the setting `plugin` of `operation` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/simulate` | the setting `simulate` of `operation` | §7.10 |
| `disl.schema.json#/$defs/Operation/properties/doc` | `doc` and `///` | §7.10 |
| `disl.schema.json#/$defs/Simulation/properties/for` | the key `for` in a value of Simulation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Simulation/properties/state` | the key `state` in a value of Simulation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Simulation/properties/initial` | the key `initial` in a value of Simulation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Simulation/properties/next` | the key `next` in a value of Simulation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Simulation/properties/until` | the key `until` in a value of Simulation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Simulation/properties/stepMs` | the key `stepMs` in a value of Simulation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Simulation/properties/maxSteps` | the key `maxSteps` in a value of Simulation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Simulation/properties/notice` | the key `notice` in a value of Simulation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Simulation/properties/doc` | the key `doc` in a value of Simulation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DeletionPolicy/properties/children` | the key `children` in a value of DeletionPolicy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DeletionPolicy/properties/relations` | the key `relations` in a value of DeletionPolicy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DeletionPolicy/properties/references` | the key `references` in a value of DeletionPolicy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DeletionPolicy/properties/confirm` | the key `confirm` in a value of DeletionPolicy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/DeletionPolicy/properties/doc` | the key `doc` in a value of DeletionPolicy (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Confirmation/properties/message` | the key `message` in a value of Confirmation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Confirmation/properties/title` | the key `title` in a value of Confirmation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Confirmation/properties/confirmLabel` | the key `confirmLabel` in a value of Confirmation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Confirmation/properties/cancelLabel` | the key `cancelLabel` in a value of Confirmation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Confirmation/properties/danger` | the key `danger` in a value of Confirmation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Confirmation/properties/when` | the key `when` in a value of Confirmation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Confirmation/properties/count` | the key `count` in a value of Confirmation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Confirmation/properties/threshold` | the key `threshold` in a value of Confirmation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Confirmation/properties/doc` | the key `doc` in a value of Confirmation (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Behavior/properties/placements` | the setting `placements` of `behavior`, with `operation`, `hook`, `reason` and `message` beside it | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/hooks` | `hook` | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/operations` | `operation` | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/deletion` | the setting `deletion` of `behavior`, with `operation`, `hook`, `reason` and `message` beside it | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/clipboard` | the setting `clipboard` of `behavior`, with `operation`, `hook`, `reason` and `message` beside it | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/retype` | the setting `retype` of `behavior`, with `operation`, `hook`, `reason` and `message` beside it | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/undo` | the setting `undo` of `behavior`, with `operation`, `hook`, `reason` and `message` beside it | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/maxHookDepth` | the setting `maxHookDepth` of `behavior`, with `operation`, `hook`, `reason` and `message` beside it | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/reasons` | `reason` | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/editGate` | the setting `editGate` of `behavior`, with `operation`, `hook`, `reason` and `message` beside it | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/messages` | `message` | §7.10 |
| `disl.schema.json#/$defs/Behavior/properties/doc` | the setting `doc` of `behavior`, with `operation`, `hook`, `reason` and `message` beside it | §7.10 |
| `disl.schema.json#/$defs/LayoutConfig/properties/algorithm` | the setting `algorithm` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/direction` | the setting `direction` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/spacing` | the setting `spacing` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/edgeRouting` | the setting `edgeRouting` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/overrideRouting` | the setting `overrideRouting` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/options` | the setting `options` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/scope` | the setting `scope` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/includeViewOnly` | the setting `includeViewOnly` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/rowPacked` | the setting `rowPacked` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/rows` | the setting `rows` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/tidyTree` | the setting `tidyTree` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/force` | the setting `force` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/LayoutConfig/properties/doc` | the setting `doc` of `algorithm` | §8.8 |
| `disl.schema.json#/$defs/RowPacked/properties/keepRows` | the key `keepRows` in a value of RowPacked (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowPacked/properties/order` | the key `order` in a value of RowPacked (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowPacked/properties/gap` | the key `gap` in a value of RowPacked (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowPacked/properties/followConnections` | the key `followConnections` in a value of RowPacked (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowPacked/properties/targetAfter` | the key `targetAfter` in a value of RowPacked (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowPacked/properties/rowsCovered` | the key `rowsCovered` in a value of RowPacked (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowPacked/properties/doc` | the key `doc` in a value of RowPacked (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowsLayout/properties/writes` | the key `writes` in a value of RowsLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowsLayout/properties/extent` | the key `extent` in a value of RowsLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowsLayout/properties/clearance` | the key `clearance` in a value of RowsLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowsLayout/properties/affinity` | the key `affinity` in a value of RowsLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowsLayout/properties/ties` | the key `ties` in a value of RowsLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowsLayout/properties/swapPasses` | the key `swapPasses` in a value of RowsLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/RowsLayout/properties/doc` | the key `doc` in a value of RowsLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TidyTree/properties/order` | the key `order` in a value of TidyTree (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TidyTree/properties/parentAlign` | the key `parentAlign` in a value of TidyTree (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TidyTree/properties/origin` | the key `origin` in a value of TidyTree (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TidyTree/properties/rows` | the key `rows` in a value of TidyTree (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TidyTree/properties/drag` | the key `drag` in a value of TidyTree (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TidyTree/properties/doc` | the key `doc` in a value of TidyTree (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ForceLayout/properties/tiers` | the key `tiers` in a value of ForceLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ForceLayout/properties/gap` | the key `gap` in a value of ForceLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ForceLayout/properties/doc` | the key `doc` in a value of ForceLayout (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Layout/properties/algorithms` | `algorithm` | §8.8 |
| `disl.schema.json#/$defs/Layout/properties/default` | `default` of `layout` | §8.8 |
| `disl.schema.json#/$defs/Layout/properties/containers` | the setting `containers` of `layout` | §8.8 |
| `disl.schema.json#/$defs/Layout/properties/trigger` | the setting `trigger` of `layout` | §8.8 |
| `disl.schema.json#/$defs/Layout/properties/respect` | the setting `respect` of `layout` | §8.8 |
| `disl.schema.json#/$defs/Layout/properties/incremental` | the setting `incremental` of `layout` | §8.8 |
| `disl.schema.json#/$defs/Layout/properties/animate` | the setting `animate` of `layout` | §8.8 |
| `disl.schema.json#/$defs/Layout/properties/doc` | the setting `doc` of `layout` | §8.8 |
| `disl.schema.json#/$defs/MigrationStep/properties/renameType` | the key `renameType` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/renameAttribute` | the key `renameAttribute` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/deleteType` | the key `deleteType` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/deleteAttribute` | the key `deleteAttribute` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/setAttribute` | the key `setAttribute` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/convertAttribute` | the key `convertAttribute` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/transform` | the key `transform` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/retype` | the key `retype` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/convertView` | the key `convertView` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/renamePort` | the key `renamePort` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/renameSlot` | the key `renameSlot` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/plugin` | the key `plugin` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/MigrationStep/properties/doc` | the key `doc` in a value of MigrationStep (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Migration/properties/from` | the key `from` in a value of Migration (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Migration/properties/to` | the key `to` in a value of Migration (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Migration/properties/steps` | the key `steps` in a value of Migration (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Migration/properties/doc` | the key `doc` in a value of Migration (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IdStrategy/properties/strategy` | the setting `strategy` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/encoding` | the setting `encoding` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/prefix` | the setting `prefix` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/expression` | the setting `expression` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/stable` | the setting `stable` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/ephemeral` | the setting `ephemeral` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/reason` | the setting `reason` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/pattern` | the setting `pattern` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/suffix` | the setting `suffix` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/types` | the setting `types` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/missing` | the setting `missing` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/compare` | the setting `compare` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdStrategy/properties/doc` | the setting `doc` of the setting `ids` of `persistence` | §7.13 |
| `disl.schema.json#/$defs/IdRule/properties/strategy` | the key `strategy` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IdRule/properties/encoding` | the key `encoding` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IdRule/properties/prefix` | the key `prefix` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IdRule/properties/expression` | the key `expression` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IdRule/properties/stable` | the key `stable` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IdRule/properties/ephemeral` | the key `ephemeral` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IdRule/properties/reason` | the key `reason` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IdRule/properties/pattern` | the key `pattern` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IdRule/properties/suffix` | the key `suffix` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/IdRule/properties/doc` | the key `doc` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/Persistence/properties/format` | the setting `format` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/binding` | the setting `binding` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/typeMap` | the setting `typeMap` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/encoding` | the setting `encoding` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/indent` | the setting `indent` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/newline` | the setting `newline` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/finalNewline` | the setting `finalNewline` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/compression` | the setting `compression` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/mediaType` | the setting `mediaType` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/files` | the setting `files` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/ids` | the setting `ids` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/references` | the setting `references` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/ordering` | the setting `ordering` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/omitDefaults` | the setting `omitDefaults` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/precision` | the setting `precision` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/timestamps` | the setting `timestamps` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/canonical` | the setting `canonical` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/view` | the setting `view` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/metadata` | the setting `metadata` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/definition` | the setting `definition` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/migrations` | the setting `migrations` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/collaboration` | the setting `collaboration` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/export` | the setting `export` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/import` | the setting `import` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/Persistence/properties/doc` | the setting `doc` of `persistence`, with the description's `binding`s | §7.13 |
| `disl.schema.json#/$defs/TypeMapEntry/properties/as` | the key `as` in a value of TypeMapEntry (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TypeMapEntry/properties/attributes` | the key `attributes` in a value of TypeMapEntry (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TypeMapEntry/properties/hostAttributes` | the key `hostAttributes` in a value of TypeMapEntry (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/TypeMapEntry/properties/doc` | the key `doc` in a value of TypeMapEntry (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ViewBind/properties/type` | the key `type` in a value of ViewBind (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ViewBind/properties/element` | the key `element` in a value of ViewBind (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ViewBind/properties/x` | the key `x` in a value of ViewBind (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ViewBind/properties/y` | the key `y` in a value of ViewBind (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ViewBind/properties/group` | the key `group` in a value of ViewBind (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ViewBind/properties/collapsed` | the key `collapsed` in a value of ViewBind (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ViewBind/properties/values` | the key `values` in a value of ViewBind (a settings map, §6.4) | §6.4 |
| `disl.schema.json#/$defs/ViewBind/properties/doc` | the key `doc` in a value of ViewBind (a settings map, §6.4) | §6.4 |

### C.2 DESL 0.2

223 properties.

| JSON pointer | ATD | Section |
|---|---|---|
| `desl.schema.json#/$defs/Specification/properties/$schema` | none: `atd.schema.json` describes the tree, and a description has no `$schema` | §5.1 |
| `desl.schema.json#/$defs/Specification/properties/desl` | the header `atd <version>` (§5.1) | §5.1 |
| `desl.schema.json#/$defs/Specification/properties/language` | the tool construct (§5.1) | §5.1 |
| `desl.schema.json#/$defs/Specification/properties/metamodel` | `model { … }` | §5.1 |
| `desl.schema.json#/$defs/Specification/properties/persistence` | `persistence { … }` and `binding` | §5.1 |
| `desl.schema.json#/$defs/Specification/properties/constraints` | `constraints { … }` and `constraint` | §5.1 |
| `desl.schema.json#/$defs/Specification/properties/surface` | `surface table` or `surface hierarchy` | §5.1 |
| `desl.schema.json#/$defs/Specification/properties/operations` | `operation` | §5.1 |
| `desl.schema.json#/$defs/Specification/properties/functions` | `fn` | §5.1 |
| `desl.schema.json#/$defs/Specification/properties/doc` | `doc` and `///` | §5.1 |
| `desl.schema.json#/$defs/Language/properties/id` | the setting `id` of the tool construct | §5.1 |
| `desl.schema.json#/$defs/Language/properties/version` | `version "<semver>"` of the tool construct | §5.1 |
| `desl.schema.json#/$defs/Language/properties/origin` | the origin of the tool construct | §5.1 |
| `desl.schema.json#/$defs/Language/properties/label` | `label` | §5.1 |
| `desl.schema.json#/$defs/Language/properties/doc` | `doc` and `///` | §5.1 |
| `desl.schema.json#/$defs/Language/properties/defaultLocale` | the setting `defaultLocale` of the tool construct | §5.1 |
| `desl.schema.json#/$defs/Language/properties/locales` | the setting `locales` of the tool construct | §5.1 |
| `desl.schema.json#/$defs/Language/properties/authors` | the setting `authors` of the tool construct | §5.1 |
| `desl.schema.json#/$defs/Language/properties/license` | the setting `license` of the tool construct | §5.1 |
| `desl.schema.json#/$defs/Language/properties/homepage` | the setting `homepage` of the tool construct | §5.1 |
| `desl.schema.json#/$defs/Language/properties/icon` | the setting `icon` of the tool construct | §5.1 |
| `desl.schema.json#/$defs/Language/properties/limits` | the setting `limits` of the tool construct | §5.1 |
| `desl.schema.json#/$defs/Function/properties/params` | the parameters of `fn` | §12.1 |
| `desl.schema.json#/$defs/Function/properties/returns` | `-> Type` | §12.1 |
| `desl.schema.json#/$defs/Function/properties/cel` | the body of `fn`: `return <expression>` | §12.1 |
| `desl.schema.json#/$defs/Function/properties/uses` | the setting `uses` of `fn` | §12.1 |
| `desl.schema.json#/$defs/Function/properties/doc` | the setting `doc` of `fn` | §12.1 |
| `desl.schema.json#/$defs/Metamodel/properties/document` | `document { … }` | §7.4 |
| `desl.schema.json#/$defs/Metamodel/properties/enums` | `enum` | §7.4 |
| `desl.schema.json#/$defs/Metamodel/properties/types` | `element` | §7.4 |
| `desl.schema.json#/$defs/Metamodel/properties/doc` | `doc` and `///` | §7.4 |
| `desl.schema.json#/$defs/ElementType/properties/label` | `label` | §7.5 |
| `desl.schema.json#/$defs/ElementType/properties/icon` | the setting `icon` of `element` | §7.5 |
| `desl.schema.json#/$defs/ElementType/properties/attributes` | `attribute` | §7.5 |
| `desl.schema.json#/$defs/ElementType/properties/children` | `contains` | §7.5 |
| `desl.schema.json#/$defs/ElementType/properties/labelAttribute` | the setting `labelAttribute` of `element` | §7.5 |
| `desl.schema.json#/$defs/ElementType/properties/doc` | `doc` and `///` | §7.5 |
| `desl.schema.json#/$defs/Containment/properties/allowed` | `contains Types` | §7.8 |
| `desl.schema.json#/$defs/Containment/properties/ordered` | the flag `ordered` of `contains` | §7.8 |
| `desl.schema.json#/$defs/Containment/properties/slots` | `contains Types as slot` | §7.8 |
| `desl.schema.json#/$defs/Containment/properties/doc` | the setting `doc` of `contains` | §7.8 |
| `desl.schema.json#/$defs/Attribute/properties/type` | `attribute name: Type` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/label` | the setting `label` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/doc` | `doc` and `///` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/required` | the flag `required` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/default` | `default value` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/many` | the flag `many` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/readOnly` | the flag `readonly` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/min` | the setting `min` of `attribute` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/max` | the setting `max` of `attribute` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/minLength` | the setting `minLength` of `attribute` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/maxLength` | the setting `maxLength` of `attribute` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/pattern` | the setting `pattern` of `attribute` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/timezone` | the setting `timezone` of `attribute` | §7.6 |
| `desl.schema.json#/$defs/Attribute/properties/mentions` | the setting `mentions` of `attribute` | §7.6 |
| `desl.schema.json#/$defs/Mentions/properties/pattern` | the setting `pattern` of the setting `mentions` of `attribute` | §7.6 |
| `desl.schema.json#/$defs/Mentions/properties/types` | the setting `types` of the setting `mentions` of `attribute` | §7.6 |
| `desl.schema.json#/$defs/Mentions/properties/doc` | the setting `doc` of the setting `mentions` of `attribute` | §7.6 |
| `desl.schema.json#/$defs/Enum/properties/values` | the enum's values | §7.7 |
| `desl.schema.json#/$defs/Enum/properties/ordered` | the setting `ordered` of `enum` | §7.7 |
| `desl.schema.json#/$defs/Enum/properties/label` | the string after the enum's name | §7.7 |
| `desl.schema.json#/$defs/Enum/properties/doc` | the setting `doc` of `enum` | §7.7 |
| `desl.schema.json#/$defs/EnumValue/properties/value` | the setting `value` of an `enum` value | §7.7 |
| `desl.schema.json#/$defs/EnumValue/properties/label` | the setting `label` of an `enum` value | §7.7 |
| `desl.schema.json#/$defs/EnumValue/properties/icon` | the setting `icon` of an `enum` value | §7.7 |
| `desl.schema.json#/$defs/EnumValue/properties/doc` | the setting `doc` of an `enum` value | §7.7 |
| `desl.schema.json#/$defs/Persistence/properties/format` | the setting `format` of `persistence`, with the description's `binding`s | §7.13 |
| `desl.schema.json#/$defs/Persistence/properties/bindings` | the setting `bindings` of `persistence`, with the description's `binding`s | §7.13 |
| `desl.schema.json#/$defs/Persistence/properties/ids` | the setting `ids` of `persistence`, with the description's `binding`s | §7.13 |
| `desl.schema.json#/$defs/Persistence/properties/typeMap` | the setting `typeMap` of `persistence`, with the description's `binding`s | §7.13 |
| `desl.schema.json#/$defs/Persistence/properties/settings` | the setting `settings` of `persistence`, with the description's `binding`s | §7.13 |
| `desl.schema.json#/$defs/Persistence/properties/doc` | the setting `doc` of `persistence`, with the description's `binding`s | §7.13 |
| `desl.schema.json#/$defs/Setting/properties/type` | the key `type` in a value of Setting (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Setting/properties/key` | the key `key` in a value of Setting (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Setting/properties/value` | the key `value` in a value of Setting (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Setting/properties/initial` | the key `initial` in a value of Setting (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Setting/properties/doc` | the key `doc` in a value of Setting (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Setting/properties/attribute` | the key `attribute` in a value of Setting (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Setting/properties/of` | the key `of` in a value of Setting (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/TypeMapEntry/properties/as` | the key `as` in a value of TypeMapEntry (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/TypeMapEntry/properties/attributes` | the key `attributes` in a value of TypeMapEntry (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/TypeMapEntry/properties/hostAttributes` | the key `hostAttributes` in a value of TypeMapEntry (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/TypeMapEntry/properties/doc` | the key `doc` in a value of TypeMapEntry (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/IdStrategy/properties/strategy` | the setting `strategy` of the setting `ids` of `persistence` | §7.13 |
| `desl.schema.json#/$defs/IdStrategy/properties/encoding` | the setting `encoding` of the setting `ids` of `persistence` | §7.13 |
| `desl.schema.json#/$defs/IdStrategy/properties/stable` | the setting `stable` of the setting `ids` of `persistence` | §7.13 |
| `desl.schema.json#/$defs/IdStrategy/properties/missing` | the setting `missing` of the setting `ids` of `persistence` | §7.13 |
| `desl.schema.json#/$defs/IdStrategy/properties/types` | the setting `types` of the setting `ids` of `persistence` | §7.13 |
| `desl.schema.json#/$defs/IdStrategy/properties/doc` | the setting `doc` of the setting `ids` of `persistence` | §7.13 |
| `desl.schema.json#/$defs/IdRule/properties/strategy` | the key `strategy` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/IdRule/properties/expression` | the key `expression` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/IdRule/properties/prefix` | the key `prefix` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/IdRule/properties/digits` | the key `digits` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/IdRule/properties/mark` | the key `mark` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/IdRule/properties/placeholder` | the key `placeholder` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/IdRule/properties/ephemeral` | the key `ephemeral` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/IdRule/properties/reason` | the key `reason` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/IdRule/properties/doc` | the key `doc` in a value of IdRule (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Constraints/properties/rules` | `constraint` | §7.9 |
| `desl.schema.json#/$defs/Constraints/properties/defaults` | the setting `defaults` of `constraints` | §7.9 |
| `desl.schema.json#/$defs/Constraints/properties/blockSaveOn` | the setting `blockSaveOn` of `constraints` | §7.9 |
| `desl.schema.json#/$defs/Constraints/properties/doc` | the setting `doc` of `constraints` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/id` | the name of `constraint` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/code` | the setting `code` of `constraint` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/label` | `label` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/scope` | `for` of `constraint` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/when` | `when` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/rule` | `rule` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/severity` | `severity` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/message` | `message` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/target` | the setting `target` of `constraint` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/enforcement` | the setting `enforcement` of `constraint` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/enabled` | the setting `enabled` of `constraint` | §7.9 |
| `desl.schema.json#/$defs/Constraint/properties/doc` | `doc` and `///` | §7.9 |
| `desl.schema.json#/$defs/Operation/properties/label` | the string after the operation's name, or `label` | §7.10 |
| `desl.schema.json#/$defs/Operation/properties/for` | `for` of `operation` | §7.10 |
| `desl.schema.json#/$defs/Operation/properties/params` | `param` | §7.10 |
| `desl.schema.json#/$defs/Operation/properties/unavailable` | `unavailable` | §7.10 |
| `desl.schema.json#/$defs/Operation/properties/confirm` | `confirm` | §7.10 |
| `desl.schema.json#/$defs/Operation/properties/actions` | the statements of `operation` | §7.10 |
| `desl.schema.json#/$defs/Operation/properties/doc` | `doc` and `///` | §7.10 |
| `desl.schema.json#/$defs/Confirmation/properties/message` | the setting `message` of `confirm` | §7.10 |
| `desl.schema.json#/$defs/Confirmation/properties/when` | the setting `when` of `confirm` | §7.10 |
| `desl.schema.json#/$defs/Confirmation/properties/count` | the setting `count` of `confirm` | §7.10 |
| `desl.schema.json#/$defs/Confirmation/properties/threshold` | the setting `threshold` of `confirm` | §7.10 |
| `desl.schema.json#/$defs/Confirmation/properties/doc` | the setting `doc` of `confirm` | §7.10 |
| `desl.schema.json#/$defs/Action/properties/set` | `set path = expression` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/unset` | `unset path` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/target` | the target of the action's statement | §12.3 |
| `desl.schema.json#/$defs/Action/properties/create` | `create Type { … } in … after … as name` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/as` | `as name` of `create` and `connect`, the loop variable of `for` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/delete` | `delete` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/reorder` | `reorder … by|after|before …` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/move` | `move … into parent slot name after|before …` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/let` | `let name = expression` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/if` | `if expression { … }` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/then` | the block of `if` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/else` | `else { … }` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/forEach` | `for name in expression limit n { … }` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/do` | the block of `for` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/when` | `if expression { action }` | §12.3 |
| `desl.schema.json#/$defs/Action/properties/doc` | a `//` comment | §12.3 |
| `desl.schema.json#/$defs/Surface/properties/kind` | `table` or `hierarchy` after `surface` | §9 |
| `desl.schema.json#/$defs/Table/properties/kind` | `surface table` | §9.1 |
| `desl.schema.json#/$defs/Table/properties/columns` | `columns Type { … }` | §9.1 |
| `desl.schema.json#/$defs/Table/properties/rows` | `rows Type { … }` | §9.1 |
| `desl.schema.json#/$defs/Table/properties/cells` | `cells Type { … }` | §9.1 |
| `desl.schema.json#/$defs/Table/properties/views` | `views Type { … }` | §9.1 |
| `desl.schema.json#/$defs/Table/properties/valueTypes` | `valueType` | §9.1 |
| `desl.schema.json#/$defs/Table/properties/doc` | the setting `doc` of `surface table` | §9.1 |
| `desl.schema.json#/$defs/Hierarchy/properties/kind` | `surface hierarchy` | §9.3 |
| `desl.schema.json#/$defs/Hierarchy/properties/levels` | `levels Type { … }` | §9.3 |
| `desl.schema.json#/$defs/Hierarchy/properties/entries` | `entries Type { … }` | §9.3 |
| `desl.schema.json#/$defs/Hierarchy/properties/labels` | `labels name { … }` | §9.3 |
| `desl.schema.json#/$defs/Hierarchy/properties/notes` | `notes Type { … }` | §9.3 |
| `desl.schema.json#/$defs/Hierarchy/properties/collapsed` | the setting `collapsed` of `surface hierarchy` | §9.3 |
| `desl.schema.json#/$defs/Hierarchy/properties/filters` | `filters Type { … }` | §9.3 |
| `desl.schema.json#/$defs/Hierarchy/properties/widths` | `widths Type { … }` | §9.3 |
| `desl.schema.json#/$defs/Hierarchy/properties/ids` | the setting `ids` of `surface hierarchy` | §9.3 |
| `desl.schema.json#/$defs/Hierarchy/properties/doc` | the setting `doc` of `surface hierarchy` | §9.3 |
| `desl.schema.json#/$defs/HierarchyMembers/properties/type` | the key `type` in a value of HierarchyMembers (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/HierarchyMembers/properties/title` | the key `title` in a value of HierarchyMembers (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/HierarchyMembers/properties/description` | the key `description` in a value of HierarchyMembers (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/HierarchyMembers/properties/doc` | the key `doc` in a value of HierarchyMembers (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Labels/properties/on` | the key `on` in a value of Labels (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Labels/properties/options` | the key `options` in a value of Labels (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Labels/properties/prefix` | the key `prefix` in a value of Labels (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Labels/properties/doc` | the key `doc` in a value of Labels (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Notes/properties/type` | the key `type` in a value of Notes (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Notes/properties/about` | the key `about` in a value of Notes (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Notes/properties/kind` | the key `kind` in a value of Notes (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Notes/properties/text` | the key `text` in a value of Notes (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Notes/properties/detail` | the key `detail` in a value of Notes (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Notes/properties/resolved` | the key `resolved` in a value of Notes (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Notes/properties/doc` | the key `doc` in a value of Notes (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Filters/properties/type` | the key `type` in a value of Filters (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Filters/properties/name` | the key `name` in a value of Filters (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Filters/properties/query` | the key `query` in a value of Filters (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Filters/properties/doc` | the key `doc` in a value of Filters (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/Widths/properties/type` | the type after `widths` | §9.3 |
| `desl.schema.json#/$defs/Widths/properties/level` | the setting `level` of `widths` | §9.3 |
| `desl.schema.json#/$defs/Widths/properties/width` | the setting `width` of `widths` | §9.3 |
| `desl.schema.json#/$defs/Widths/properties/doc` | the setting `doc` of `widths` | §9.3 |
| `desl.schema.json#/$defs/Columns/properties/type` | the type after `columns` | §9.1 |
| `desl.schema.json#/$defs/Columns/properties/name` | the setting `name` of `columns` | §9.1 |
| `desl.schema.json#/$defs/Columns/properties/valueType` | the setting `valueType` of `columns` | §9.1 |
| `desl.schema.json#/$defs/Columns/properties/title` | the setting `title` of `columns` | §9.1 |
| `desl.schema.json#/$defs/Columns/properties/parent` | the setting `parent` of `columns` | §9.1 |
| `desl.schema.json#/$defs/Columns/properties/options` | the setting `options` of `columns` | §9.1 |
| `desl.schema.json#/$defs/Columns/properties/doc` | the setting `doc` of `columns` | §9.1 |
| `desl.schema.json#/$defs/Cells/properties/type` | the type after `cells` | §9.1 |
| `desl.schema.json#/$defs/Cells/properties/column` | the setting `column` of `cells` | §9.1 |
| `desl.schema.json#/$defs/Cells/properties/items` | the setting `items` of `cells` | §9.1 |
| `desl.schema.json#/$defs/Cells/properties/doc` | the setting `doc` of `cells` | §9.1 |
| `desl.schema.json#/$defs/Views/properties/type` | the type after `views` | §9.1 |
| `desl.schema.json#/$defs/Views/properties/name` | the setting `name` of `views` | §9.1 |
| `desl.schema.json#/$defs/Views/properties/active` | the setting `active` of `views` | §9.1 |
| `desl.schema.json#/$defs/Views/properties/settings` | the setting `settings` of `views` | §9.1 |
| `desl.schema.json#/$defs/Views/properties/doc` | the setting `doc` of `views` | §9.1 |
| `desl.schema.json#/$defs/ValueType/properties/label` | the string after the value type's name | §9.2 |
| `desl.schema.json#/$defs/ValueType/properties/key` | the setting `key` of `valueType` | §9.2 |
| `desl.schema.json#/$defs/ValueType/properties/many` | the setting `many` of `valueType` | §9.2 |
| `desl.schema.json#/$defs/ValueType/properties/editor` | the setting `editor` of `valueType` | §9.2 |
| `desl.schema.json#/$defs/ValueType/properties/comparisons` | the setting `comparisons` of `valueType` | §9.2 |
| `desl.schema.json#/$defs/ValueType/properties/sort` | the setting `sort` of `valueType` | §9.2 |
| `desl.schema.json#/$defs/ValueType/properties/converts` | `convert type conversion` | §9.2 |
| `desl.schema.json#/$defs/ValueType/properties/doc` | the setting `doc` of `valueType` | §9.2 |
| `desl.schema.json#/$defs/Reason/properties/when` | the setting `when` of `reason` | §7.12 |
| `desl.schema.json#/$defs/Reason/properties/message` | the setting `message` of `reason` | §7.12 |
| `desl.schema.json#/$defs/Reason/properties/doc` | the setting `doc` of `reason` | §7.12 |
| `desl.schema.json#/$defs/Doc/properties/summary` | the setting `summary` of `doc` and `///` | §7.3 |
| `desl.schema.json#/$defs/Doc/properties/description` | the setting `description` of `doc` and `///` | §7.3 |
| `desl.schema.json#/$defs/Doc/properties/rationale` | the setting `rationale` of `doc` and `///` | §7.3 |
| `desl.schema.json#/$defs/Doc/properties/examples` | the setting `examples` of `doc` and `///` | §7.3 |
| `desl.schema.json#/$defs/Doc/properties/seeAlso` | the setting `seeAlso` of `doc` and `///` | §7.3 |
| `desl.schema.json#/$defs/Doc/properties/tags` | the setting `tags` of `doc` and `///` | §7.3 |
| `desl.schema.json#/$defs/Doc/properties/audience` | the setting `audience` of `doc` and `///` | §7.3 |
| `desl.schema.json#/$defs/Doc/properties/since` | the setting `since` of `doc` and `///` | §7.3 |
| `desl.schema.json#/$defs/Doc/properties/deprecated` | the setting `deprecated` of `doc` and `///` | §7.3 |
| `desl.schema.json#/$defs/Doc/properties/helpUrl` | the setting `helpUrl` of `doc` and `///` | §7.3 |
| `desl.schema.json#/$defs/CelValue/properties/cel` | the key `cel` in a value of CelValue (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/CelValue/properties/resultType` | the key `resultType` in a value of CelValue (a settings map, §6.4) | §6.4 |
| `desl.schema.json#/$defs/CelValue/properties/doc` | the key `doc` in a value of CelValue (a settings map, §6.4) | §6.4 |

### C.3 FBL 0.9

218 properties.

| JSON pointer | ATD | Section |
|---|---|---|
| `fbl.schema.json#/$defs/Document/properties/$schema` | none | §11.1 |
| `fbl.schema.json#/$defs/Document/properties/fbl` | the header `atd <version>` (§5.1); a binding is FBL 0.9's in ATD 0.1 | §11.1 |
| `fbl.schema.json#/$defs/Document/properties/doc` | `doc` and `///` | §11.1 |
| `fbl.schema.json#/$defs/Document/properties/bindings` | `binding` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/title` | `label` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/doc` | `doc` and `///` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/claims` | `claims { … }` or `claims files …` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/body` | `body`, and `family` after the binding's name | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/reader` | `family <family>` (`declared`) or `family capability name` (a persistence plugin) | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/readOnly` | the setting `readOnly` of `binding` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/text` | `text { … }` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/header` | `header` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/comment` | the setting `comment` of `binding` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/unmatched` | the setting `unmatched` of `binding` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/blocks` | `block` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/sections` | `section` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/elements` | `element` rules | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/relations` | `relation` rules | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/registration` | `registration` | §11.1 |
| `fbl.schema.json#/$defs/Binding/properties/template` | `template` | §11.1 |
| `fbl.schema.json#/$defs/Claims/properties/extensions` | the setting `extensions` of `claims` | §11.2 |
| `fbl.schema.json#/$defs/Claims/properties/names` | the setting `names` of `claims` | §11.2 |
| `fbl.schema.json#/$defs/Claims/properties/shared` | the setting `shared` of `claims` | §11.2 |
| `fbl.schema.json#/$defs/Claims/properties/registrationOnly` | the setting `registrationOnly` of `claims` | §11.2 |
| `fbl.schema.json#/$defs/Claims/properties/marker` | the setting `marker` of `claims` | §11.2 |
| `fbl.schema.json#/$defs/Claims/properties/suggest` | the setting `suggest` of `claims` | §11.2 |
| `fbl.schema.json#/$defs/Claims/properties/origins` | the setting `origins` of `claims` | §11.2 |
| `fbl.schema.json#/$defs/Claims/properties/readings` | the setting `readings` of `claims` | §11.2 |
| `fbl.schema.json#/$defs/Marker/properties/rootKey` | the key `rootKey` in a value of Marker (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Marker/properties/value` | the key `value` in a value of Marker (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Marker/properties/firstLine` | the key `firstLine` in a value of Marker (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Marker/properties/pattern` | the key `pattern` in a value of Marker (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Marker/properties/lines` | the key `lines` in a value of Marker (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Body/properties/kind` | the setting `kind` of `body` | §11.2 |
| `fbl.schema.json#/$defs/Body/properties/family` | `family` after the binding's name | §11.2 |
| `fbl.schema.json#/$defs/Body/properties/alsoRead` | the setting `alsoRead` of `body` | §11.2 |
| `fbl.schema.json#/$defs/Body/properties/grammar` | `grammar { … }` or `grammar from "<file>.lark"` in a binding of `family grammar` | §11.9 |
| `fbl.schema.json#/$defs/Body/properties/start` | the setting `start` of `body` | §11.9 |
| `fbl.schema.json#/$defs/Body/properties/recover` | the setting `recover` of `body` | §11.9 |
| `fbl.schema.json#/$defs/GrammarSource/anyOf/1/properties/file` | the file of `grammar from "<file>.lark"` | §11.9 |
| `fbl.schema.json#/$defs/Recover/properties/unit` | the key `unit` in the value of `recover` (a settings map, §6.4) | §11.9 |
| `fbl.schema.json#/$defs/Recover/properties/sync` | the key `sync` in the value of `recover` (a settings map, §6.4) | §11.9 |
| `fbl.schema.json#/$defs/Body/properties/recognise` | the setting `recognise` of `body` | §11.2 |
| `fbl.schema.json#/$defs/Body/properties/files` | the setting `files` of `body` | §11.2 |
| `fbl.schema.json#/$defs/Body/properties/ignore` | the setting `ignore` of `body` | §11.2 |
| `fbl.schema.json#/$defs/Body/properties/settle` | the setting `settle` of `body` | §11.2 |
| `fbl.schema.json#/$defs/FileRule/properties/name` | the key `name` in a value of FileRule (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/FileRule/properties/glob` | the key `glob` in a value of FileRule (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/FileRule/properties/family` | the key `family` in a value of FileRule (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/FileRule/properties/readOnly` | the key `readOnly` in a value of FileRule (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/PluginReader/properties/plugin` | `family capability name` | §11.7 |
| `fbl.schema.json#/$defs/PluginReader/properties/version` | the setting `version` of `family capability` | §11.7 |
| `fbl.schema.json#/$defs/PluginReader/properties/args` | the setting `args` of `family capability` | §11.7 |
| `fbl.schema.json#/$defs/TextDefaults/properties/newline` | the setting `newline` of `text` (binding) | §11.2 |
| `fbl.schema.json#/$defs/TextDefaults/properties/indent` | the setting `indent` of `text` (binding) | §11.2 |
| `fbl.schema.json#/$defs/TextDefaults/properties/sequenceIndent` | the setting `sequenceIndent` of `text` (binding) | §11.2 |
| `fbl.schema.json#/$defs/TextDefaults/properties/finalNewline` | the setting `finalNewline` of `text` (binding) | §11.2 |
| `fbl.schema.json#/$defs/TextDefaults/properties/quote` | the setting `quote` of `text` (binding) | §11.2 |
| `fbl.schema.json#/$defs/Header/properties/key` | the setting `key` of `header` | §11.2 |
| `fbl.schema.json#/$defs/Header/properties/line` | the setting `line` of `header` | §11.2 |
| `fbl.schema.json#/$defs/Header/properties/value` | the setting `value` of `header` | §11.2 |
| `fbl.schema.json#/$defs/Header/properties/required` | the setting `required` of `header` | §11.2 |
| `fbl.schema.json#/$defs/BlockRule/properties/name` | the name of `block` | §11.4 |
| `fbl.schema.json#/$defs/BlockRule/properties/doc` | the setting `doc` of `block` | §11.4 |
| `fbl.schema.json#/$defs/BlockRule/properties/line` | `line "regex"` of `block` | §11.4 |
| `fbl.schema.json#/$defs/BlockRule/properties/within` | the setting `within` of `block` | §11.4 |
| `fbl.schema.json#/$defs/BlockRule/properties/caseInsensitive` | the setting `caseInsensitive` of `block` | §11.4 |
| `fbl.schema.json#/$defs/BlockRule/properties/view` | the setting `view` of `block` | §11.4 |
| `fbl.schema.json#/$defs/SectionRule/properties/name` | the name of `section` | §11.4 |
| `fbl.schema.json#/$defs/SectionRule/properties/doc` | the setting `doc` of `section` (binding) | §11.4 |
| `fbl.schema.json#/$defs/SectionRule/properties/heading` | the string after the section's name | §11.4 |
| `fbl.schema.json#/$defs/SectionRule/properties/caseInsensitive` | the setting `caseInsensitive` of `section` (binding) | §11.4 |
| `fbl.schema.json#/$defs/ElementRule/properties/name` | `as name` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/doc` | `doc` and `///` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/type` | the type after `element` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/at` | `at "selector"` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/line` | `line "regex"` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/within` | `within names` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/opens` | the setting `opens` of `element` (binding rule) | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/caseInsensitive` | the setting `caseInsensitive` of `element` (binding rule) | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/files` | `files names` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/when` | `when expression` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/id` | `id from slot` or `id { … }` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/parent` | `parent rules slot name` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/attributes` | `attribute name from slot { … }` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/insert` | `insert { … }` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/remove` | `remove { … }` | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/undo` | the setting `undo` of `element` (binding rule) | §11.3 |
| `fbl.schema.json#/$defs/ElementRule/properties/readOnly` | the setting `readOnly` of `element` (binding rule) | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/name` | `as name` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/doc` | `doc` and `///` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/type` | the type after `relation` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/at` | `at "selector"` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/line` | `line "regex"` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/within` | `within names` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/opens` | the setting `opens` of `relation` (binding rule) | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/caseInsensitive` | the setting `caseInsensitive` of `relation` (binding rule) | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/files` | `files names` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/when` | `when expression` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/id` | `id from slot` or `id { … }` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/parent` | `parent rules slot name` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/attributes` | `attribute name from slot { … }` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/insert` | `insert { … }` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/remove` | `remove { … }` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/undo` | the setting `undo` of `relation` (binding rule) | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/readOnly` | the setting `readOnly` of `relation` (binding rule) | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/source` | `source from slot` | §11.3 |
| `fbl.schema.json#/$defs/RelationRule/properties/target` | `target from slot` | §11.3 |
| `fbl.schema.json#/$defs/IdBinding/properties/from` | `id from slot` | §11.3 |
| `fbl.schema.json#/$defs/IdBinding/properties/sidecar` | `id { sidecar: { … } }` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/key` | `from key …` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/attribute` | `from attribute …` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/text` | `from text …` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/continuation` | `from continuation …` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/node` | `from node …` | §11.3, §11.9 |
| `fbl.schema.json#/$defs/Slot/properties/child` | the setting `child` of the slot after `from` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/group` | `from group …` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/parent` | `from parent …` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/capture` | `from capture …` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/value` | `from value …` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/word` | the setting `word` of the slot after `from` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/flag` | the setting `flag` of the slot after `from` | §11.3 |
| `fbl.schema.json#/$defs/Slot/properties/match` | the setting `match` of the slot after `from` | §11.3, §11.9 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/key` | `attribute name from key …` | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/attribute` | `attribute name from attribute …` | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/text` | `attribute name from text …` | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/continuation` | `attribute name from continuation …` | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/node` | `attribute name from node …` | §11.3, §11.9 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/child` | the setting `child` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/group` | `attribute name from group …` | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/parent` | `attribute name from parent …` | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/capture` | `attribute name from capture …` | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/value` | `attribute name from value …` | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/word` | the setting `word` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/flag` | the setting `flag` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/match` | the setting `match` of `attribute` of a rule | §11.3, §11.9 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/empty` | the setting `empty` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/default` | the setting `default` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/read` | the setting `read` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/quoted` | the setting `quoted` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/fallback` | the setting `fallback` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/number` | the setting `number` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/time` | the setting `time` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/style` | the setting `style` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/reference` | `references rules by name` in the attribute's block | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/map` | the setting `map` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/absent` | the setting `absent` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/override` | the setting `override` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/content` | the setting `content` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/create` | the setting `create` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/AttributeBinding/properties/readOnly` | the setting `readOnly` of `attribute` of a rule | §11.3 |
| `fbl.schema.json#/$defs/Insert/properties/place` | the setting `place` of `insert` | §11.5 |
| `fbl.schema.json#/$defs/Insert/properties/container` | the setting `container` of `insert` | §11.5 |
| `fbl.schema.json#/$defs/Insert/properties/create` | the setting `create` of `insert` | §11.5 |
| `fbl.schema.json#/$defs/Insert/properties/keys` | the setting `keys` of `insert` | §11.5 |
| `fbl.schema.json#/$defs/Insert/properties/emit` | the setting `emit` of `insert` | §11.5 |
| `fbl.schema.json#/$defs/Insert/properties/constant` | the setting `constant` of `insert` | §11.5 |
| `fbl.schema.json#/$defs/Insert/properties/skeleton` | the setting `skeleton` of `insert` | §11.5 |
| `fbl.schema.json#/$defs/Insert/properties/when` | the setting `when` of `insert` | §11.5 |
| `fbl.schema.json#/$defs/Remove/properties/cascade` | the setting `cascade` of `remove` | §11.5 |
| `fbl.schema.json#/$defs/Remove/properties/container` | the setting `container` of `remove` | §11.5 |
| `fbl.schema.json#/$defs/RegistrationSettings/properties/headers` | the setting `headers` of `registration` | §11.6 |
| `fbl.schema.json#/$defs/RegistrationSettings/properties/blocks` | the setting `blocks` of `registration` | §11.6 |
| `fbl.schema.json#/$defs/RegistrationSettings/properties/createOnFirstPlacement` | the setting `createOnFirstPlacement` of `registration` | §11.6 |
| `fbl.schema.json#/$defs/RegistrationSettings/properties/resource` | the setting `resource` of `registration` | §11.6 |
| `fbl.schema.json#/$defs/RegistrationSettings/properties/legacyLayout` | the setting `legacyLayout` of `registration` | §11.6 |
| `fbl.schema.json#/$defs/RegistrationSettings/properties/legacyIdentities` | the setting `legacyIdentities` of `registration` | §11.6 |
| `fbl.schema.json#/$defs/Template/properties/text` | the setting `text` of `template` (binding) | §11.6 |
| `fbl.schema.json#/$defs/Template/properties/byOrigin` | the setting `byOrigin` of `template` (binding) | §11.6 |
| `fbl.schema.json#/$defs/Template/properties/doc` | the setting `doc` of `template` (binding) | §11.6 |
| `fbl.schema.json#/$defs/Registration/properties/origin` | the key `origin` in a value of Registration (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Registration/properties/body` | the key `body` in a value of Registration (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Registration/properties/view` | the key `view` in a value of Registration (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Registration/properties/resource` | the key `resource` in a value of Registration (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Registration/properties/headers` | the key `headers` in a value of Registration (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Registration/properties/layout` | the key `layout` in a value of Registration (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Registration/properties/identities` | the key `identities` in a value of Registration (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Registration/properties/blocks` | the key `blocks` in a value of Registration (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Splice/properties/operation` | the key `operation` in a value of Splice (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Splice/properties/file` | the key `file` in a value of Splice (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Splice/properties/start` | the key `start` in a value of Splice (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Splice/properties/end` | the key `end` in a value of Splice (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Splice/properties/text` | the key `text` in a value of Splice (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Fixture/properties/$schema` | the key `$schema` in a value of Fixture (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Fixture/properties/doc` | the key `doc` in a value of Fixture (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Fixture/properties/binding` | the key `binding` in a value of Fixture (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Fixture/properties/input` | the key `input` in a value of Fixture (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Fixture/properties/read` | the key `read` in a value of Fixture (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Fixture/properties/steps` | the key `steps` in a value of Fixture (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Edit/properties/add` | the key `add` in a value of Edit (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Edit/properties/set` | the key `set` in a value of Edit (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Edit/properties/remove` | the key `remove` in a value of Edit (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Edit/properties/move` | the key `move` in a value of Edit (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Edit/properties/place` | the key `place` in a value of Edit (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Edit/properties/setting` | the key `setting` in a value of Edit (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Edit/properties/save` | the key `save` in a value of Edit (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/ModelChange/properties/add` | the key `add` in a value of ModelChange (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/ModelChange/properties/set` | the key `set` in a value of ModelChange (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/ModelChange/properties/remove` | the key `remove` in a value of ModelChange (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/ModelChange/properties/move` | the key `move` in a value of ModelChange (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/ModelChange/properties/retype` | the key `retype` in a value of ModelChange (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/ChangePosition/properties/index` | the key `index` in a value of ChangePosition (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/ChangePosition/properties/after` | the key `after` in a value of ChangePosition (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/ChangePosition/properties/before` | the key `before` in a value of ChangePosition (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Expression/properties/cel` | the key `cel` in a value of Expression (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Expression/properties/resultType` | the key `resultType` in a value of Expression (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Expression/properties/doc` | the key `doc` in a value of Expression (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/summary` | the key `summary` in a value of Doc (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/description` | the key `description` in a value of Doc (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/rationale` | the key `rationale` in a value of Doc (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/examples` | the key `examples` in a value of Doc (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/seeAlso` | the key `seeAlso` in a value of Doc (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/tags` | the key `tags` in a value of Doc (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/audience` | the key `audience` in a value of Doc (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/since` | the key `since` in a value of Doc (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/deprecated` | the key `deprecated` in a value of Doc (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/image` | the key `image` in a value of Doc (a settings map, §6.4) | §6.4 |
| `fbl.schema.json#/$defs/Doc/properties/helpUrl` | the key `helpUrl` in a value of Doc (a settings map, §6.4) | §6.4 |
