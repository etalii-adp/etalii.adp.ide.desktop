---
id: fbl-specification
type: submodule-design
title: Format Binding Language (FBL) — normative specification
status: draft
parent: fbl
---
# FBL — Format Binding Language

**Specification, version 0.17 (Draft)**

|                          |                                                                                              |
|--------------------------|----------------------------------------------------------------------------------------------|
| Date                     | 2026-11-01                                                                                   |
| Document schema          | `fbl.schema.json` (JSON Schema, draft 2020-12), `$defs/Document`                             |
| Also in the schema       | `$defs/Registration` (the parsed `.adp` registration), `$defs/Fixture` (round-trip fixtures) |
| Serves                   | every kind of tool: [DISL](../disl/DISL-specification.md) through `persistence.binding`, [DESL](../desl/DESL-specification.md) through `persistence.bindings` |
| Expression language      | CEL — Common Expression Language (https://cel.dev)                                           |
| Media type (provisional) | `application/vnd.fbl.document+json`                                                          |
| File extension           | `.fbl`                                                                                       |
| Licence                  | [Apache License 2.0](https://github.com/etalii-adp/etalii.adp/blob/develop/LICENSE) (`Apache-2.0`) |

---

## Status of this document

This is a draft, version 0.17. It is complete enough to implement a conforming validator of FBL documents and a host that reads and writes bodies through declared bindings, but individual constructs may still change before version 1.0. FBL 0.2 (2026-10-05, approved by the product owner as part of the DISL 0.3 additions) gives the model changes a persistence plugin plans their fields: an add with a position, a move and a retype (section 11.2), which declared bindings follow too (section 6.4); and it lets a DISL specification map a binding's own type and attribute names onto its metamodel (DISL §11.2, `persistence.typeMap`; section 5.1). Every valid 0.1 document is a valid 0.2 document with the same meaning, except that a plugin that does not declare `move` is sent a reorder or re-parent as a remove and an add. FBL 0.3 (2026-10-09, for the Knowledge designer, spec 013) changes no behaviour: it names the tool type's specification wherever 0.2 said DISL, since a DESL specification uses bindings too (DESL section 5), and pairs each construct it relies on with its DISL and its DESL section (section 1.3); it lets a specification name one binding per format family, the body choosing among them by the files each claims (sections 1.2, 8.1); it spells a reference to a stored id as `by: "id"` (sections 5.2, 5.7); it lets a fixture's edit be a `move` or an `add` with a `position`, as section 11.2 already defines them (section 15.3); and it says how a container written on one line is created and removed around its entry (section 6.2). Every valid 0.2 document is a valid 0.3 document with the same meaning. FBL 0.4 (2026-10-09, for the Agent activity diagram, spec 014, by the ruling of 2026-10-09) lets a binding keep its entries and values in a nested mapping that the body may not have yet: an insert whose container is placed `at: "end"` creates every missing level of its selector (section 6.2), an attribute reached through `child` is written into a mapping it creates the same way in yaml and json (section 5.2), and `remove-empty-levels` removes the levels an entry's removal leaves empty (section 6.2). Every valid 0.3 document is a valid 0.4 document with the same meaning. FBL 0.5 (2026-10-10, spec 015, for the tools whose YAML files people write by hand) says what a slot reads from a scalar: it defines a scalar's **written text** (section 4.1.5), lets an attribute binding read it with `read` and makes it what a text attribute, an id and a reference read (sections 5.2, 5.3), presents it to CEL as `text` and `style` beside `entry` (section 2.4), lets a binding keep an entry readable when one value does not convert, with `fallback`, and read a quoted number, with `quoted` (sections 5.2, 7.4), and lets a fixture state the attribute values a reading yields and an add name the `slot` it fills (section 15.3). Every valid 0.4 document is a valid 0.5 document; section 5.2 lists the two places where 0.5 gives a meaning that 0.4 left open. FBL 0.6 (2026-10-12, spec 026, by the maintainer's ruling of 2026-10-12 on row B-09 of spec 025's behaviour-change register) makes an attribute binding's `empty` rule decide a new entry's key as it decides a set (sections 5.2, 6.2): 0.5 said that a key a new entry has no value for is left out, and left open whether an empty value counts as no value on an insert; 0.6 says it does not, so an attribute an add gives its empty value is written with `keep`, left out with `remove` and refuses the add with `refuse` (section 6.4), with a set's defaults, while an attribute the add does not give is still left out. Every valid 0.5 document is a valid 0.6 document. FBL 0.7 (2026-10-14, spec 028, for the Hierarchical Requirement Designer) adds a sixth family, `bullets`, which reads the list items under named headings of a Markdown file as a tree (section 4.8): a binding's `sections` name the headings, a rule's `within` the sections its items stand in, the slot `{continuation: true}` reads the lines indented under an item (section 5.2), and a move to another depth re-indents the entry it moves (sections 5.5, 6.3), and an insert's `emit` can write a literal bracket, as a Markdown task list needs (section 6.2), a fixture's `read` may state the parent of an element (section 15.3), an insert may place a line right after the header, and a case-insensitive rule's `map` ignores case (sections 5.2, 6.2). It lets a binding declare further blocks of the registration, `registration.blocks`, in the grammar `layout:` and `identities:` already have, each with the order its entries keep, whether they are keyed by ids and whether its writes are steps of the history (sections 8.1, 8.4), so a tool type can keep beside the body what the body's readers do not need. A word slot of a `many` attribute holds every word of its group that matches, which lets a line carry a list of tags (sections 4.6, 6.3). It also makes a host compare the body with the bytes an edit was planned against before applying it, as 0.6 required before an undo or a redo (section 7.2). Every valid 0.6 document is a valid 0.7 document with the same meaning. FBL 0.8 (2026-10-22, spec 033, for the functional decomposition graph, by the maintainer's ruling of 2026-10-22) lets an insert of the `yaml` and `json` families write a **constant**: a key and a scalar value every new entry of the rule is written with, that no attribute supplies, such as the value of the key whose value chose the rule (section 6.2). A constant only writes: `when` alone decides which rule an entry is read by, no edit but the add writes it, and a binding whose rule also writes the constant's key is refused when it is loaded (section 14.1). Every valid 0.7 document is a valid 0.8 document with the same meaning. FBL 0.9 (2026-10-25, spec 035, story 7, for the ADP tool language) adds a seventh family, `grammar`, for a format no other family reads (section 4.9): the binding carries an LALR(1) grammar in ATD grammar notation, inline or in a file beside the document, which must build without a conflict or the binding is refused when it is loaded (section 14.1); its rules select the nodes of the tree the grammar reads by their names, a slot `{node}` reads the text of a node, with `match` when only part of it is the value (section 5.2), a node's splice is its byte range, text no rule maps is kept, new entries are written from `emit` and removed with their lines (sections 6.2, 6.3), and with `recover` a body that does not parse is read in part, the text that does not being kept and reported at its line (sections 4.9, 7.4). No host is required to support the family in 0.9; one that does passes its round-trip fixtures byte for byte (section 15.1). Every valid 0.8 document is a valid 0.9 document with the same meaning. FBL 0.10 (2026-10-29, spec 066, for the Databricks conversion) lets one binding serve readings whose tool types have different metamodels: a rule's `origins` names the readings it is read for, so that an entry becomes at most one element per reading (section 5.1); a registration's `resource` takes a capture per origin with `byOrigin`, or none, and a header naming a resource the body does not hold opens the reading empty with `fbl.missing-resource` (sections 7.4, 8.2); and one open body keeps one model per origin it is read for, every reading of it editable and sharing its history (sections 9.1, 9.2). It states how the `yaml` family writes into a flow sequence: an empty one is opened into a block sequence, a one-line one takes a new item inside it and gives one up with one separator, and one written over several lines, like a flow mapping, takes no insert (sections 4.3, 6.2). It lets a binding give the sentence of each refusal it causes, with `refusals` (sections 5.2, 6.2, 6.4); writes an attribute reached through `child` into the mapping an add gives the new entry (section 6.2); reads a `secondary` slot when the attribute's own is absent and removes it when the attribute is written (section 5.2); and offers the template placeholder `{key:lower}` (section 13). A fixture can state that its input is a template, with `template`, and the bytes of the files beside its input that a step writes, with `expectFiles`, so that a fixture of a registration states the edits of the body it opens (section 15.3). Every valid 0.9 document is a valid 0.10 document with the same meaning. FBL 0.11 (2026-10-29, spec 066, for the C4 conversions) extends the `grammar` family so that the Structurizr DSL is bound in it (`structurizr.fbl`, with `structurizr.lark`). A comment is owned by the statement it stands on or before and kept by every splice, moved and removed with it (section 4.9); `recover` reads inside blocks too, with `nested` naming a block's opening and closing tokens and `unclosed` a token that may run to the end of the body, so that a statement that does not parse is skipped to the next one in its block, and an unclosed block, an unmatched closing token and an unclosed comment are each read as far as they go and reported (sections 4.9, 7.4); a slot's `escape` reads and writes a quoted value's escapes (sections 5.2, 6.3); an attribute's absent and positional parts are written with `create`'s `after` and `pad`, and a `many` attribute's items are read from several nodes, `gather` reading a read-only value from two places (sections 5.2, 6.2); a rule's `opens` writes the block a first child needs around it (section 4.9); a `block` rule gives the views of a `grammar` binding, with `at`, `view`, `origins` and `insert`, so that a view block can be added and a `selectedView` rule reads a statement of the selected view only, the registration's view header choosing among the reading's views by their origin (sections 5.1, 9.3); and the legacy layout matches its keys ignoring case, writes the shortest number and prunes what is stale, reporting a key written twice as `fbl.duplicate-key` (sections 7.4, 8.7). A host that reads or writes Structurizr DSL through its declared binding supports the family (section 15.1). Every valid 0.10 document is a valid 0.11 document with the same meaning. FBL 0.12 (2026-10-29, spec 066, for the Wardley map conversion) extends the `grammar` family so that the Wardley map's `.owm` is bound in it (`wardley-map.fbl`, with `wardley-map.lark`), so that both formats section 15.1 requires a host of the family to read have a declared binding. A block whose opening brace stands on the line after its statement is read as the block of that statement (section 4.9); the segments that follow a statement's fixed parts are read in any order, each by its node, a single-valued one set alone and a `many` one written as a set of its items (section 4.9); a `{node}` slot's `items` reads a list of structured values, each item set, added and removed with its `separator` (sections 5.2, 6.3); an id kept in the sidecar has a `kind`, the key space it shares with the rules of that kind, and an `ordinal`, which gives each later holder of one natural key an entry and an id of its own; an entry moves with its key when the values it is computed from change, in the same edit; and an id first stored for an element is written with the next write of the body and is no step of the history (sections 5.3, 8.6). `registration.legacyIdentities` is read as the JSON array of `{id, kind, key}` the Wardley map's tool writes, its entries of a kind no rule declares kept and reported (section 8.7). Every valid 0.11 document is a valid 0.12 document with the same meaning, except that FBL 0.9 described the legacy identities file as a JSON object from natural key to id: no binding, example or fixture used that form and no tool wrote it, and 0.12 does not read it. FBL 0.13 (2026-10-30, spec 066, for gap 16 of the C4 conversions, by the maintainer's ruling of 2026-10-30) lets a `{node}` slot of a `many` attribute read several items from one node: with `split`, the slot's value is cut at every occurrence of that text, each piece without the white space around it is an item with a span of its own, and an empty piece is no item, so that Structurizr's `tags "Extra, More"` reads the two tags `Extra` and `More` (sections 4.9, 5.2). A set writes the difference into the node: a changed item by a `replace-value` of its own span, a new one after the last item with the slot's `separator`, a removed one with one separator, and a node left with no item as a removed node of the slot, so that every byte of an item the set keeps, and between two items it keeps, stays (section 6.3). `structurizr.fbl` splits the tags at their commas. Every valid 0.12 document is a valid 0.13 document with the same meaning. FBL 0.14 (2026-10-30, spec 066, for the Databricks conversion's adds of a resource) lets an insert of the `yaml` and `json` families add a **member** to a mapping: a rule whose `at` ends in a `{capture}` segment, such as `/resources/jobs/{job}`, writes a new entry under the key the add gives the attribute bound to that capture, at the place its `place` gives among the container's members, its value the entry's keys, constants and `skeleton` one step deeper, and refuses an add whose key is missing, empty or already in the mapping, with the sentences `refusals.key` and `refusals.duplicate` give (section 6.2). `databricks.fbl` adds a bundle's jobs and pipelines so. Every valid 0.13 document is a valid 0.14 document with the same meaning; no earlier document declared such an insert, which 0.13 left undefined. FBL 0.15 (2026-10-31, spec 067, for Turtle) lets the `grammar` family read a triple: a slot's selector reaches up and across the tree with `..` and `+`, an attribute binding's `or` names further slots, the first present being read, a slot with `expand` reads a term through the body's `prefixes` and writes it back through them, one with `ordinal` reads a node's number in one of the body's `ordinals`, stable across readings of the same bytes, one with `name` reads a node's name, a slot's `escape` takes the other sequences of a format's strings with `sequences`, `unicode`, `writes` and `requote`, and `body.order` can read the elements in the order their entries end (sections 4.9, 5.2, 6.3). `w3c-turtle-grammar.fbl` binds Turtle's triples so. Every valid 0.14 document is a valid 0.15 document with the same meaning; none could give these properties, and `..` and `+` were names no grammar can give a node. FBL 0.17 (2026-11-01, by Peter's choice of 2026-11-01 on how a host treats a binding with constructs newer than it reads) lets a host that supports the `grammar` family at an earlier version than a construct a binding uses open, read and edit that binding's documents rather than only read them, provided it reports each construct it does not read as a warning naming it and writes no splice that touches the text such a construct governs, which stays byte for byte; a host that cannot honour that opens them read-only as before, and the C4 and Wardley hosts' requirement of 0.11 is unchanged (section 15.1). It adds no construct: every valid 0.16 document is a valid 0.17 document with the same meaning. Sections and paragraphs marked *(informative)* explain intent and give guidance; everything else is *normative*.

FBL relies on constructs that the specification language of the tool type's kind defines: id strategies and ephemeral ids, findings and their source locations, derived elements, the editing transaction and the tool type's origin. Where this document names one of them, the specification language is the definition; FBL defines none of its own. Where this document says **the tool type's specification**, it means the DISL specification of a diagram or the DESL specification of a designer whose persistence names the binding. Section 1.3 lists, for each construct, the section of DISL and of DESL that defines it; a citation of DISL in this document is read, for a designer, as a citation of the DESL section that table pairs it with.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY** and **OPTIONAL** are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in bold capitals.

---

## Table of contents

1. [Introduction](#1-introduction)
2. [Foundations](#2-foundations)
3. [The binding](#3-the-binding)
4. [Format families and their lossless readings](#4-format-families-and-their-lossless-readings)
5. [Rules](#5-rules)
6. [Writing](#6-writing)
7. [History and reading problems](#7-history-and-reading-problems)
8. [The registration](#8-the-registration)
9. [Several readings of one body](#9-several-readings-of-one-body)
10. [Folder subjects](#10-folder-subjects)
11. [Persistence plugins](#11-persistence-plugins)
12. [Routing](#12-routing)
13. [Templates](#13-templates)
14. [Processing model](#14-processing-model)
15. [Conformance](#15-conformance)
16. [Security and robustness](#16-security-and-robustness)
17. [Today's diagram definitions](#17-todays-diagram-definitions-informative) *(informative)*
- [Appendix A — JSON Schema](#appendix-a--json-schema)

---

## 1. Introduction

### 1.1 What FBL is

Many ADP tools do not own the file their model lives in. A C4 diagram's model is a Structurizr workspace, a mind map's is a Freeplane `.mm` file, a Databricks job's is a bundle resource file, and even ADP's own formats (the timeline's `.tml`, the causal loop diagram's `.cld`) are meant to be read and edited by hand beside ADP. DISL's persistence layer (DISL §11) describes something else: a DID definition that a writer regenerates canonically. Before FBL, every such tool declared a required persistence plugin, which is code, and code has to be written once for each of the four ADP hosts.

FBL, the Format Binding Language, declares instead how such a file maps to a tool type's model in **both directions**:

- which files a tool claims, and how a host routes a file to it;
- how the file is read into elements, relations and attribute values, tolerating whatever it cannot read;
- how every change to the model is written back as **named, minimal splices**, so that everything the change does not concern stays byte for byte as it was, and so that every edit has an exact undo;
- where the view data a user places (positions) is kept: in an `.adp` **registration** beside the file, never in the file;
- how several tool types read one file at once, and how a folder is read and watched as one subject;
- and, for the formats that are too intricate to declare, the **contract** a persistence plugin implements, so that plugins behave alike in every host.

### 1.2 How FBL is used *(informative)*

A tool engineer writes an FBL document, `*.fbl`, holding one or more named bindings, and points a tool type's specification at one of them, or, for a tool type whose documents may be written in several formats, at one binding per format family:

```json
{ "persistence": { "format": "fbl", "binding": "timeline.fbl#timeline" } }
{ "persistence": { "format": "fbl", "bindings": ["knowledge.fbl#yaml", "knowledge.fbl#json", "knowledge.fbl#xml"] } }
```

A host that opens a file first routes it to a binding (section 12), reads its registration if there is one (section 8), opens the body through the binding for the body's family (sections 4 and 5) and builds the model the tool type's specification describes. Every transaction the user commits is planned as splices (section 6), applied to the body's bytes and kept in the body's history for undo (section 7). Positions the user drags are written to the registration. Other tools can keep editing the body, and the host picks their changes up (section 7.3).

The same binding serves every host. The round-trip fixtures in `fixtures/` (section 15.3) are the shared test: a host that passes them reads and writes bodies the way every other host does.

### 1.3 Relation to the specification languages

FBL serves every kind of tool. What it reads a body into, and the constructs it relies on, are defined by the specification language of the tool type's kind: DISL for a diagram, DESL for a designer. Each language defines them on its own; neither takes them from the other.

| Concern | For a diagram | For a designer |
|---|---|---|
| Metamodel, constraints, behaviour and presentation of a tool type | DISL | DESL |
| Element types, attributes, containment, references ("node type" and "relation type" in this document) | DISL §4 | DESL section 4 (element types; a designer has no relation types) |
| A binding's type and attribute names mapped onto the metamodel (`persistence.typeMap`) | DISL §11.2 | DESL section 5.2 |
| Id strategies, derived ids, ephemeral (unstable) ids | DISL §11.5 (`persistence.ids`) | DESL section 5.3 (`persistence.ids`) |
| Findings, their severities, codes and source locations; the built-in findings `std.unparseable`, `std.unreadableEntry`, `std.missingId`, `std.duplicateId` | DISL §8.6, §8.7 | DESL sections 6.3, 6.5 |
| `std.pluginMissing`, derived elements, computed containment | DISL §8.7, §4.11 | none: DESL 0.1 has no plugins and no derived elements |
| Reading order and `positionIn` | DISL §8.6, §12.4 | DESL sections 10.2, 9.4 |
| The editing transaction | DISL §14.4 | DESL section 10.3 |
| A tool type's origin, `<vendor>/<type>` | DISL §3.2 (`language.origin`) | DESL section 3.2 (`language.origin`) |
| Localized text and documentation objects | DISL §2.3, §2.4 | DESL sections 2.3, 2.4 |
| CEL, its cost limits and its determinism | DISL §2.5, §12 | DESL sections 2.5, 9 |
| Extension properties | DISL §2.8 | DESL section 2.6 |
| A document stored as ADP's own definition file | DID | DED |
| A model stored in a file or folder another tool owns | **FBL** | **FBL** |
| The `.adp` registration, view data of such a model, routing | **FBL** | **FBL** |

DISL's persistence layer names FBL with `format: "fbl"` and `binding` (DISL §11.2); DESL's with `format: "fbl"` and `bindings` (DESL section 5). With that format, DISL's writer settings (`files`, `indent`, `newline`, `ordering`, `precision` and the others DISL lists) do not apply: FBL writes by splices and keeps the file's own conventions.

### 1.4 Design principles

1. **The file is the model.** A body is the only store of its model. A host **MUST NOT** keep a second copy of the model anywhere (a cache is not a copy if it is rebuilt from the body).
2. **Touch only what changed.** A save without an edit writes the bytes that were read. An edit changes only the bytes of its splices.
3. **Every write can be undone exactly**, or is refused before it happens.
4. **Reading never fails** on content. What cannot be read becomes a finding; what is not bound is kept.
5. **One answer in every host.** Everything that decides bytes (spans, splices, formatting of new text) is specified here, so two hosts write the same bytes for the same edit.
6. **Declare what can be declared; contract the rest.** A format FBL cannot declare is read by a plugin, but everything around the plugin (routing, registration, history, drift) is still FBL's.

### 1.5 Non-goals

- FBL is not a general grammar formalism. It covers the format families of section 4, which are the ones today's tools need (constitution principle V). *(FBL 0.9)* The `grammar` family carries a grammar, but only to read one format's syntax as a tree that rules select from; what a node means is still the binding's rules, and an LALR(1) grammar in ATD's notation is all it admits.
- FBL does not compute the model beyond reading it: which elements a view shows, derived elements and layouts are the specification language's.
- FBL does not merge concurrent edits. A body is edited by one host at a time; changes from anywhere else arrive as external changes (section 7.3).

---

## 2. Foundations

### 2.1 The document

An FBL document is a JSON document (RFC 8259) encoded in UTF-8, validated by `fbl.schema.json#/$defs/Document`:

| Property   | Type                 | Description                                                    |
|------------|----------------------|----------------------------------------------------------------|
| `$schema`  | URI                  | Optional; the schema's `$id` with `#/$defs/Document`.          |
| `fbl`      | `"0.1"`, `"0.2"`, `"0.3"`, `"0.4"`, `"0.5"`, `"0.6"`, `"0.7"`, `"0.8"`, `"0.9"`, `"0.10"`, `"0.11"`, `"0.12"`, `"0.13"`, `"0.14"`, `"0.15"`, `"0.16"`, `"0.17"` | Required. The FBL version the document is written in.    |
| `doc`      | Doc                  | Documentation, as in DISL §2.4.                                |
| `bindings` | map Name → Binding   | Required, at least one. Each binding is described in section 3. |

A document **MUST NOT** contain duplicate keys. Properties whose names start with `x-` are extension properties and are ignored by hosts that do not know them, as in DISL §2.8.

### 2.2 Names

A binding name and a rule name is a **Name**: a letter or underscore followed by letters, digits, underscores, dots or hyphens (`^[A-Za-z_][A-Za-z0-9_.-]*$`). Rule names are unique within their binding. Attribute names and type references are DISL's (`SimpleId`, `TypeRef`).

### 2.3 Binding references

A binding is referenced as `<document>#<name>`: a URI reference to an FBL document, resolved against the referring document's own location, followed by `#` and the binding's name. `#<name>` alone refers to a binding in the same document. A DISL specification refers to a binding the same way from `persistence.binding` (DISL §11.2), or embeds a Binding object there; a DESL specification lists one binding per format family in `persistence.bindings` (DESL section 5).

### 2.4 CEL

Wherever FBL needs a condition or a computed value it uses CEL, as DISL does (DISL §2.5, §12), with the cost limits DISL sets. The variables available depend on where an expression appears:

| Variable       | Type                     | Available in                                   | Meaning |
|----------------|--------------------------|------------------------------------------------|---------|
| `entry`        | map                      | rule `when`, slot `value`, sidecar `key`       | The entry the rule matched, as CEL values (section 4.1.4). |
| `text`         | map                      | same, yaml and json *(FBL 0.5)*                | The same entry with every scalar as its written text (section 4.1.5): `text.label` is `"yes"` where `entry.label` is `true`. |
| `style`        | map                      | same, yaml and json *(FBL 0.5)*                | The same entry with every scalar as the name of the style it is written in: `plain`, `single`, `double`, `literal` or `folded`; json strings are `double`, other json scalars `plain`. |
| `parent`       | map or null              | same                                           | The nearest enclosing entry matched by any rule of the binding, or null. |
| `path`         | map string → string      | tree and xml rules                             | The values of the selector's `{capture}` segments (section 4.2). |
| `groups`       | map string → string      | lines, blocks and bullets rules                | The named groups of the rule's `line` that took part in the match; a group that did not is absent, so `has(groups.x)` tests it. |
| `line`         | int                      | every rule                                     | The 1-based line on which the entry starts. |
| `registration` | map string → string      | every rule                                     | The headers of the registration the body was opened through (section 8), empty without one. |
| `attributes`   | map                      | `insert.when`                                  | The attribute values of the element being added. |

A CEL expression **MUST** compile against these variables; a validator reports one that does not.

### 2.5 Regular expressions

`lines`, `blocks` and `bullets` rules, sections, markers, headers and comments use regular expressions in the **common subset** of RE2, .NET, Java and ECMAScript syntax, which all four hosts' languages and CEL's `matches` read alike:

- literals, `.`, character classes `[…]` and `[^…]`, the escapes `\d \D \s \S \w \W \t \n \r \\` and escaped punctuation;
- anchors `^` and `$` (a line never contains its line ending, so both refer to the line);
- groups `(…)`, non-capturing groups `(?:…)` and named groups `(?<name>…)`, names being SimpleIds;
- alternation `|` and the quantifiers `* + ? {n} {n,} {n,m}` and their lazy forms.

Backreferences, lookaround, atomic groups, possessive quantifiers, inline flags and Unicode property classes are not in the subset, and a validator **MUST** reject an expression that uses them. Case-insensitive matching of ASCII letters is requested with a rule's `caseInsensitive`. Matching is against the whole line unless the expression says otherwise with anchors, except that *(FBL 0.7)* a `bullets` rule with `within` is matched against the item's text and a section against the heading's text (section 4.8); a match that could succeed in several ways takes the one the leftmost-first semantics of RE2 gives, which the other engines agree with for this subset.

### 2.6 Text

A body file is UTF-8, with or without a byte-order mark. A byte-order mark, if present, is kept and belongs to no node; offsets in this document and in fixtures are UTF-8 byte offsets into the file including it. A file that is not valid UTF-8 is an unreadable body (section 7.5). A **line** is a maximal run of bytes without CR or LF; its **line ending** is the CRLF, LF or lone CR that follows it, if any. Line and column numbers in findings are 1-based; columns count Unicode code points, as DISL §8.6 requires.

### 2.7 Versioning

`fbl` names the version of FBL a document is written in: `"0.1"`, `"0.2"`, `"0.3"`, `"0.4"`, `"0.5"`, `"0.6"`, `"0.7"`, `"0.8"`, `"0.9"`, `"0.10"`, `"0.11"`, `"0.12"`, `"0.13"`, `"0.14"`, `"0.15"`, `"0.16"` or `"0.17"`. Each is a superset of the one before, and a host **MUST** read a document of an earlier minor version with its meaning. While 0.17 is a draft, its schema is published at the 0.1 address (`https://etalii.net/adp/fbl/schema/0.1/fbl.schema.json`), which accepts every 0.1 document unchanged. A host **MUST** refuse a document whose major version it does not support, and **SHOULD** read a document of a newer minor version, ignoring what it does not know, with a warning. Before 1.0 any construct may change (constitution principle IV).

---

## 3. The binding

### 3.1 Overview

A **binding** declares how one format maps to a language's model. Every binding has the same parts:

```json
{
  "title": "Timeline Markup Language",
  "claims": { "extensions": [".tml"], "origins": ["generic/timeline"] },
  "body": { "kind": "file", "family": "yaml" },
  "reader": "declared",
  "text": { "newline": "crlf", "indent": 2, "finalNewline": true },
  "header": { "key": "timeline", "value": 1 },
  "elements": [ … ],
  "relations": [ … ],
  "registration": { … },
  "template": { "text": "timeline: 1\r\nelements: []\r\n" }
}
```

| Property       | Type                            | Description |
|----------------|---------------------------------|-------------|
| `title`, `doc` | LocalizedText, Doc              | As in DISL. |
| `claims`       | Claims                          | Required. Which files the binding takes (section 12). |
| `body`         | Body                            | Required. A file of one family, or a folder (sections 4 and 10). *(FBL 0.9)* A body of the `grammar` family carries its grammar (section 4.9). |
| `reader`       | `"declared"` or PluginReader    | Required. `declared`: the rules of this binding read and write the body. A PluginReader: a persistence plugin does (section 11). |
| `readOnly`     | bool or LocalizedText           | The body is never written; a text is the reason a host shows. |
| `text`         | TextDefaults                    | Conventions for new text where the body gives no evidence (section 6.3). |
| `header`       | Header                          | A version or identity mark the body starts with (section 5.6). |
| `comment`      | Regex                           | `lines` and `blocks`: a line that matches it is a comment (section 4.6). |
| `unmatched`    | `"keep"` (default), `"report"`  | `lines`, `blocks` and `bullets`: whether a statement no rule matches is also reported (section 7.4). |
| `blocks`       | BlockRule[]                     | `blocks`: statements that open blocks and produce nothing (section 4.7). |
| `sections`     | SectionRule[]                   | *(FBL 0.7)* `bullets`: the headings whose list items the rules read (section 4.8). |
| `elements`     | ElementRule[]                   | Rules producing elements (section 5). |
| `relations`    | RelationRule[]                  | Rules producing relations (section 5.4). |
| `registration` | RegistrationSettings            | What the binding adds to the registration (section 8). |
| `template`     | Template                        | The bytes of a new body (section 13). |

A declared binding **MUST** have at least one element or relation rule. A binding with a plugin reader has no rules; its model is what the plugin reads.

### 3.2 The body

| Property    | Type                                   | Description |
|-------------|----------------------------------------|-------------|
| `kind`      | `"file"`, `"folder"`                   | Required. |
| `family`    | `"yaml"`, `"json"`, `"xml"`, `"lines"`, `"blocks"`, *(FBL 0.7)* `"bullets"`, *(FBL 0.9)* `"grammar"` | Required for a declared file body: the family whose lossless reading the rules address (section 4). |
| `grammar`   | string or `{file}`                     | *(FBL 0.9)* Required for, and only for, the `grammar` family: the grammar that reads the body, in ATD grammar notation, as its text or as the name of a file beside the FBL document (section 4.9). |
| `start`     | Name                                   | *(FBL 0.9)* `grammar`: the grammar's start rule. Default `start`. |
| `recover`   | `{unit, sync}`                         | *(FBL 0.9)* `grammar`: read a body that does not parse in part, unit by unit, skipping to a synchronising token (section 4.9). Absent: a body that does not parse is unreadable (section 7.5). |
| `prefixes`  | `{declarations, bases, absolute, prefixed, local, terminals}` | *(FBL 0.15)* `grammar`: the directives a slot with `expand` reads a term through, and how a term is written (section 4.9). |
| `ordinals`  | map name → `{at, same}`                | *(FBL 0.15)* `grammar`: numberings of nodes, which a slot with `ordinal` reads (section 4.9). |
| `order`     | `"document"`, `"closed"`               | *(FBL 0.15)* `grammar`: `closed` reads the body's elements in the order their entries end (section 4.9). Default `document`. |
| `alsoRead`  | family[]                               | Other families accepted for the same body. The host tries `family` first, then each of these in order, and uses the first under which the body is readable. A document read under another family keeps that family for writing. |
| `recognise`, `files`, `ignore`, `settle` | | Folder bodies only (section 10). |

### 3.3 The one-place rule

A value that can be written back **MUST** be bound to exactly one place in the body: a key, an XML attribute, the text of an element, a named group of a line, or a word of a group (section 5.2). A value bound only through CEL (`value`) is read-only. This is what makes write-back defined: FBL never inverts an expression.

### 3.4 Read-only bindings and rules

A binding with `readOnly` is never written: the host offers no gesture that changes the model, and says why when the user tries. A rule or an attribute binding with `readOnly` is read and never written; a gesture that would change it is refused with the rule's reason, or a host-chosen one when `readOnly` is `true`. Elements that DISL derives (DISL §4.11) are never written either.

---

## 4. Format families and their lossless readings

### 4.1 The common model

For each family, this section defines a **lossless reading**: a tree of nodes over the body's bytes in which every byte belongs to exactly one node or to the **trivia** (whitespace, comments, line endings) that one node owns. Rules (section 5) select nodes; splices (section 6) replace the spans of nodes. A host **MAY** parse a body however it likes, but the spans it uses **MUST** be the ones defined here.

#### 4.1.1 Entries and spans

An **entry** is a node that a rule can match: a mapping member or sequence item (yaml, json), an element (xml), a statement (lines, blocks, bullets), a list item (bullets), *(FBL 0.9)* a node of the parse tree (grammar). Every entry has:

- its **own span**: the bytes of the entry itself, from its first to its last significant byte;
- its **line span**, when the entry starts its line and ends its line (nothing but trivia before it on its first line and after it on its last): from the first byte of its first line to the end of the line ending of its last line, extended upwards over its **leading comments**, the comment lines directly above it at the same indentation with no blank line in between. A trailing comment on the entry's last line is part of its line span.

A **value** is a node that a slot binds: a scalar, a flow collection, an attribute value, the text of an element, a named group, *(FBL 0.9)* a node or token of the parse tree, or the part of its text that `match` takes (grammar). Its span excludes quotes and delimiters for the purpose of reading, and includes them for the purpose of `replace-value` (section 6.1).

Blank lines belong to no entry: they are trivia of the enclosing container and are never removed or added by a splice unless this document says so.

#### 4.1.2 Unbound content

Everything no rule binds is **unbound**: unknown keys, attributes, elements and statements, comments, directives. Unbound content is read past, kept byte for byte, and moves only when a splice removes an entry that contains it (an unknown key inside a removed entry goes with the entry).

#### 4.1.3 Document order

Entries have **document order**, the order of their first bytes. Reading order is document order (DISL relies on it for `positionIn`); for a folder subject, files are read in the ordinal order of their relative paths, and document order within each.

#### 4.1.4 Entries as CEL values

An entry is presented to CEL (section 2.4) as follows. A yaml or json mapping is a map from key to value; a sequence is a list; a scalar is a string, int, double, bool or null by the YAML 1.2 core schema (JSON by its own types). An xml element is a map from attribute name to string, plus the key `text` holding its text content. A lines, blocks or bullets statement, and a bullets item, is presented as its `groups`. *(FBL 0.9)* A node of a `grammar` body is a map with `type`, its name, `text`, its text, and `children`, the list of its children as such maps, an optional part that matched nothing being null; a token is such a map whose `children` is empty.

A key that is present with no value is present in the map with the value null, so `has(entry.end)` is true for `end:` and false where the key is absent.

#### 4.1.5 Written text *(FBL 0.5)*

Every scalar of a yaml or json body has, beside the typed value of section 4.1.4, a **written text**: what its author wrote.

| Scalar | Written text |
|--------|--------------|
| yaml plain scalar | Its characters, without leading and trailing spaces and without a comment; a plain scalar over several lines is folded as YAML folds it. |
| yaml quoted scalar (single or double) | Its content, without the quotes, with its escapes resolved. |
| yaml block scalar (`\|`, `>`) | Its content as YAML defines it for its indicators. |
| yaml key with no value (`end:`) | The empty text. |
| json string | Its content, with its escapes resolved. |
| json number, `true`, `false`, `null` | Its literal characters. |

A tag does not change the written text. A scalar reached through an alias or a merge key has the written text of the scalar it stands for. The members of a flow collection have a written text each, by the same rules. The `xml`, `lines`, `blocks`, `bullets` and *(FBL 0.9)* `grammar` families read text and nothing else, so there the value a slot reads is already what was written.

So `yes`, `"yes"` and `'yes'` have the written text `yes`; `007` has `007`; `1.0` has `1.0`; `0x10` has `0x10`. The typed value of each is what the YAML 1.2 core schema gives: a bool, a string, a string, the int 7, the double 1.0, the int 16.

### 4.2 Selectors

Tree rules (yaml, json), xml rules and *(FBL 0.9)* grammar rules select entries with a **selector**: segments separated by `/`. Section 4.9 says what a segment names in the `grammar` family.

| Segment        | Matches |
|----------------|---------|
| a key or name  | the member with that key (yaml, json) or the child element with that name (xml) |
| `*`            | any member of a mapping or any item of a sequence (yaml, json), any child element (xml) |
| `**`           | any number of levels, including none |
| `{name}`       | any member of a mapping, whose key is bound to `path.name` |
| `name[@A='v']` | xml: a child element with that name whose attribute `A` has the value `v` |

A selector that starts with `/` is absolute, from the document's root (the root mapping or sequence; the document element's parent in xml, so `/map` is the root element `map`). A selector without it is relative to the entry the rule is written for (in `insert.container`, `child`), or to the root elsewhere. `/` alone selects the root. A key containing `/`, `*`, `{` or `}` cannot be selected in FBL 0.1.

### 4.3 The `yaml` family

The body is a YAML 1.2 stream. Only the first document is bound; any further document is unbound. The lossless reading follows the YAML 1.2 syntax, with these definitions:

- A **block mapping entry** is a key and its value; its own span runs from the first byte of the key to the last byte of the value (for a block collection, its last entry's last byte). A **block sequence entry** is `-` and its value; its own span runs from the `-`.
- The **indentation** of an entry is the column of its first byte: the key, or the `-`. For a mapping that is a sequence item (`- id: a`), the mapping's entries are indented at the column of their keys (`id` at 4 in `  - id: a`).
- A **scalar value's** span is the scalar as written: with its quotes for quoted scalars; for a plain scalar, without trailing spaces or a comment; for a block scalar (`|`, `>` with indicators), from the indicator to the end of its last content line, without that line's ending.
- A **flow collection** (`[…]`, `{…}`) is one value. Its members are readable (CEL sees them), but its only writable span is the whole collection, written by `replace-value` in the style the attribute binding asks for (`flow` writes a one-line flow sequence). *(FBL 0.10)* A flow sequence is **one-line** when its `[` and its `]` stand on one line. The items of a flow sequence are entries for the rules' selectors (section 4.2), each with the span of a json item (section 4.4): its own span runs from its first to its last byte, and its separator is the `,` between it and the next item. An insert and a removal of such an entry are planned as section 6.2 says for flow sequences: an empty flow sequence is opened into a block sequence, a one-line one takes a new item inside it, and a flow sequence written over several lines, like a flow mapping, takes no insert.
- **Anchors, aliases, tags and merge keys** are read (an alias reads as the anchored value; a tag is ignored for typing when it is a core-schema tag and kept otherwise; `<<` merges as YAML 1.1 defines it, for reading only). A slot reached through an alias or a merge is read-only.
- A **YAML syntax error** makes the body unreadable (section 7.5). A document whose structure is well-formed YAML but not what a rule expects (a sequence item that is not a mapping, a scalar where a mapping is needed) yields unreadable entries (section 7.4), never an unreadable body.

### 4.4 The `json` family

The body is a JSON text (RFC 8259). Comments are not allowed: a body with a comment is unreadable. The nodes are objects, arrays, members, items and values, with these spans:

- A **member's** own span runs from the opening quote of its name to the last byte of its value; an **item's** from the first to the last byte of its value.
- A member or item **starts its line** when only whitespace precedes it on its line. The **separator** of an entry is the comma between it and the next entry of the same container.
- A string value's span includes its quotes.

A duplicate member name is read as a finding on the second and later occurrences (`std.duplicateId` is DISL's for ids; a duplicate key is `fbl.duplicate-key`), and only the first is bound.

### 4.5 The `xml` family

The body is an XML 1.0 document. The lossless reading keeps the prolog, comments, processing instructions and a document type declaration as unbound content; entity declarations are not processed, and a reference to an entity other than the five predefined ones and character references makes the containing element an unreadable entry. Nodes are elements, attributes and text:

- An **element's** own span runs from `<` of its start tag to `>` of its end tag, or of its tag when it is **self-closed** (`<x/>`).
- An **attribute's** value span is the text between its quotes; its own span runs from its name to its closing quote, and **attribute order is significant** (kept, and used for new attributes, section 6.3).
- An element's **text** is its character data with references decoded; a `text` slot's span is the character data between the start tag and the first child element or the end tag.
- Carriage returns are part of the content, not normalised: a reader that uses an XML parser **MUST** make it keep CR, since XML 1.0 normalises line endings and the body's bytes are the model.
- Escaping when writing: in text, `&`, `<` and `>` are escaped as `&amp;`, `&lt;` and `&gt;`; in attribute values also `"` as `&quot;`, LF as `&#xa;` and CR as `&#xd;`. Other characters are written literally in UTF-8.
- A `content: "html-paragraphs"` text slot reads the element's HTML body as plain text, one line per `<p>` element, whitespace inside a paragraph collapsed to single spaces; writing it emits one `<p>line</p>` per line with the escaping above.

### 4.6 The `lines` family

The body is a sequence of lines. Each line is exactly one of:

- **blank**: only whitespace;
- a **comment**: it matches the binding's `comment`; it is owned by the statement directly below it, if no blank line intervenes (a leading comment, section 4.1.1);
- a **statement**: any other line. A statement matched by a rule's `line` is an entry of that rule; the first rule in the binding's order whose `line` matches (and whose `when` holds) wins. A statement no rule matches is unbound (and reported when `unmatched` is `report`).

A statement's own span is the line without its ending; its line span adds its leading comments and its line ending. Its **values** are its named groups. A group used with `word` is split into **words**: runs of non-whitespace, where a run starting with `"` extends to the next `"` and includes both quotes. A `word` slot's value is the first word of the group that matches the slot's `word` expression; if that expression has a named group `value`, the value is that group, otherwise the whole word (without quotes when the word is quoted). A `flag` word slot's value is true when such a word is present. *(FBL 0.7)* A word slot of an attribute with `many` holds every word of the group that matches its `word` expression, in order, each an item: the word's group `value`, or the whole word.

### 4.7 The `blocks` family

The body is lines as in the `lines` family, with nesting: a statement whose last non-whitespace character, outside double-quoted strings, is `{` **opens a block**, which extends to the line whose only non-whitespace content is the matching `}`. A statement's entry then spans from its line to the end of the closing line. Blocks nest; braces inside double-quoted strings do not count. A body whose braces do not balance is unreadable.

`blocks` (BlockRule) name the statements that open a block and produce no element (`model {`, `views {`), so that rules can state where they apply:

| Property          | Type     | Description |
|-------------------|----------|-------------|
| `name`            | Name     | Required. |
| `line`            | Regex    | Required. The statement. |
| `within`          | Name[]   | The block rules or element rules whose block the statement must be directly inside; `^` is the top level. Absent: anywhere. |
| `caseInsensitive` | bool     | Match ignoring the case of ASCII letters. |
| `view`            | group name | The block defines a **view** of the body, named by this group's value. A registration's `view` header selects it (section 9.3). *(FBL 0.11)* In the `grammar` family `view` is a slot, and a block rule has `at`, `origins` and `insert` (section 4.9). |
| `origins`         | origin[]   | *(FBL 0.11)* The readings the block is a view for (section 5.1). |

An element rule with `opens: true` may open a block (it may also appear on one line without one); `within` on element and relation rules works as for block rules. A statement is matched only by rules whose `within` admits its enclosing block.

### 4.8 The `bullets` family *(FBL 0.7)*

The body is lines, as in the `lines` family, holding list items that nest by their indentation: the part of a Markdown file that a tool reads as a tree. FBL reads the list items under the headings a binding names, and nothing else of Markdown (section 1.5).

**Columns.** A line's **indentation** is the spaces and tabs it starts with, counted in columns from 0, a tab counting four columns wherever it stands (not up to the next multiple of four).

**Line kinds.** Each line is one of the following, decided in document order. An **item** is **open** from its item line until a line closes it.

1. The lines after the one that opens a **fence**, up to and including the one that closes it, are fence lines; the opening line itself is decided by the rules below. A fence opens on a line whose text after its indentation starts with three or more backticks or three or more tildes, and closes on the next line whose text after its indentation is a run of at least as many of the same character and nothing else but whitespace, or at the end of the body. Nothing inside a fence is any other kind of line.
2. A **blank** line holds only whitespace. It closes nothing.
3. Any other line first closes every open item whose marker stands at the line's indentation or deeper.
4. An **item line** is indentation, a **marker** `-`, `*` or `+`, then whitespace or the end of the line, and is not a thematic break (below). Its item is nested in the deepest item still open, if any, and is then open itself. The item's **text** is what follows the marker and the whitespace after it, to the end of the line, trailing whitespace included; its **content column** is the column where the text begins, or the marker's column plus two when the text is empty.
5. While an item is still open, any other line is a **continuation line** of the deepest open item. A fence that opens on a continuation line belongs to that item: its lines are continuation lines of it, whatever their indentation.
6. With no item open, a line is a **heading** when it has at most three columns of indentation, then one to six `#`, then whitespace or the end of the line. Its **level** is the number of `#`, and its **text** the rest of the line without leading and trailing whitespace and without a closing run of `#` that whitespace precedes. A line underlined with `=` or `-` is not a heading in this family.
7. With no item open, a line is a **thematic break** when it holds three or more of one of `-`, `*` and `_`, and nothing else but spaces and tabs. It is never an item line, open or not.
8. With no item open, a line that opens a fence makes the fence **unbound**, with every line in it.
9. Any other line is a **statement**, as in the `lines` family.

**Sections.** A binding's `sections` name headings, as `blocks` name the statements that open a block:

| Property          | Type  | Description |
|-------------------|-------|-------------|
| `name`            | Name  | Required. Referenced by a rule's `within` and by `insert.container`. |
| `heading`         | Regex | Required. Matched against a heading's text. |
| `caseInsensitive` | bool  | Match ignoring the case of ASCII letters. |

A **section** is the lines after the first heading whose text matches `heading`, up to the next heading with as many `#` or fewer, or to the end of the body. A heading with more `#` inside it does not end it. A later heading that matches too is an ordinary heading. An item **stands in** a section when its item line is one of the section's lines.

**Entries and spans.** An item and a statement are entries. An item's **own span** runs from its marker to the end of its last continuation line or of its last nested item, whichever comes later, without that line's ending; its **line span** runs from the first byte of its item line to the end of that last line's ending. Blank lines inside an item's own span are part of it; those after it belong to no entry (section 4.1.1). A statement's spans are those of section 4.6. This family has no comments.

**Rules.** A rule with `within` matches items of the sections it names: its `line` is matched against the item's text. A rule without `within` matches statements, as in the `lines` family. Of the rules that may match an entry, the first in the binding's order whose `line` matches and whose `when` holds takes it (section 5.1). The named groups of `line` are the entry's values, split into words as section 4.6 says.

**Continuation.** An item's **continuation** is its continuation lines from the first to the last before its first nested item, the blank lines between them included; the continuation lines after a nested item are part of the item's span and are unbound. The slot `{continuation: true}` reads the continuation as text: its lines joined by LF, each with the whitespace before the item's content column taken off (a line indented less loses all of its indentation, and a tab the content column cuts into is replaced by the spaces that remain of it), a blank line read as an empty line. An item without continuation lines reads as the empty text.

**Unbound.** Headings, thematic breaks, blank lines and unbound fences are unbound. An item that stands in no section a rule's `within` names is unbound. An item that no rule matches, and an unreadable item (section 7.4), which includes an item whose rule's type the tool type's specification maps as unreadable (DESL section 5.2), are not looked into: they are unbound with everything in their span, and no rule matches an item nested in them. A statement that no rule matches is unbound, and reported only when `unmatched` is `report`.

The body is never unreadable for its family: Markdown has no malformed list (section 7.5).

### 4.9 The `grammar` family *(FBL 0.9)*

The body is text that a context-free grammar the binding carries reads: a format with a syntax of its own that no other family reads, such as Turtle, bound declaratively instead of read by a persistence plugin (section 11). The family's tree is the one the grammar gives, so its rules address the format's own constructs by their names.

**The grammar.** `body.grammar` is a grammar in **ATD grammar notation**, the subset of Lark's grammar language that ATD section 4 states (its rules are `specifications/atd/grammar/notation.lark`): either the grammar's text as a string, or `{"file": "<name>.lark"}`, a file beside the FBL document. A grammar **MAY** `%import` rules and terminals from other files beside the document (`%import .turtle_terms (IRIREF)` reads `turtle_terms.lark`) and from nowhere else. `body.start` names the start rule, `start` when it is absent. The grammar is built as ATD section 4.4 says, its terminals are written in the regular expression subset of ATD section 3.5, and its text is cut into tokens by the contextual lexing rule of ATD section 3.6, which is Lark's: at each position the candidates are the terminals the parser's state can shift and the ignored ones, a keyword is read by the pattern of its priority that spells it, and the others are tried by priority first. The grammar **MUST** build as an LALR(1) automaton with no shift/reduce and no reduce/reduce conflict, in which no two pattern terminals of one priority that a state accepts match the same text; a binding whose grammar does not is refused when it is loaded, with a problem that names the conflict (section 14.1). ATD section 4.7 normalizes the automaton, so that two implementations can show that they build the same one.

**The tree.** Reading a body gives a tree shaped as ATD section 4.5 says: a node per rule, named by the rule or by the alias of the alternative that matched; a rule written `?rule` replaced by its child when it has one, and `_rule` by its children; a token of a named terminal a leaf named by its terminal, except a `_TERMINAL`; anonymous strings left out; and an optional part that matched nothing kept as an empty child. The root is the node of the start rule.

- Every node and token has an **own span**. A token's is its bytes. A node's runs from the first byte of the first token its rule matched to the last byte of the last token its rule matched, the tokens the tree leaves out included, so that the `.` that ends a Turtle statement is in the statement's span though not in its tree. A node that matched no token has no span. The root's span is the whole body.
- A node's or token's **text** is the bytes of its own span.
- The tokens the grammar ignores (`%ignore`) are trivia. An ignored token that holds a character other than whitespace is a **comment**. *(FBL 0.11)* A comment is owned as follows, so that every host keeps the same comment with the same entry:
  1. A comment that is the first token on its line, with every comment on the lines directly above it at no blank line's distance, is a **leading comment** of the entry whose first token is the next token after them (section 4.1.1).
  2. A comment that follows a token on its line is a **trailing comment** of the innermost entry that holds that token: `model { // the model` is the comment of the `model` entry, because `{` is a token of that entry and of none of its children, and `container "Web" // c` is the container's. A trailing comment is part of that entry's line span and of no value's span.
  3. A comment that opens on one line and closes on a later one is owned by rule 1 when it starts its line and by rule 2 when it follows a token, and its owner's line span then runs to the end of the comment's last line. Nothing inside it is a token, so it holds no statement, element or brace; a statement's content before or after it on its line is read as if it were not there; removing or moving its owner takes it with the owner's line span. A comment that is never closed is trivia to the end of the body, reported under `recover.unclosed` (*Reading in part*) rather than making the body unreadable.
  4. A comment inside a value's span (`"a" /* x */ "b"` read by one slot) is part of the value's text, and a set of the value replaces it with the rest of the span; a binding that reads such a value **SHOULD** take one token with `match`.
  5. Every splice keeps a comment it does not remove: a `replace-value` changes the value's span only, a part written where it was absent is written before a trailing comment (*Writing*), and a removed entry takes its leading and trailing comments with its line span.

  A grammar states where a comment can start by its terminals alone, under the lexing rule of ATD section 3.6: where a longer token of another terminal starts before a comment's opening, such as a URL token that holds `//`, the comment terminal is never a candidate there, and the comment's characters are the token's.
- *(FBL 0.11)* **Blocks** are what the grammar reads as blocks, and nothing else: a statement whose text ends in `{` and that the grammar reads as one token (a title, a note's text) opens nothing and keeps the `{` in its text, and a `{` or `}` inside a token is never the `open` or `close` of `recover.nested`, so recovery never treats it as a block.
- *(FBL 0.12)* A grammar in which a block's opening brace may stand on the line after its statement reads both forms in one body. `wardley-map.lark` is the example: the token that opens a pipeline's block is a `{` that line endings may precede, and where the parser's state can shift it, it is of a higher priority than the line ending (ATD section 3.6), so `pipeline Kettle` followed by a line holding only `{` owns that block as `pipeline Kettle {` would. The block's `open` token, for `recover.nested` and for a finding, is that token, line endings included. An add writes a new block in the form the binding declares, by its insert's `emit` and by the rule's `opens` (*Writing*): `{"open": "\n{", "close": "}"}` for the Wardley map's brace on its own line, `{"open": " {", "close": "}"}` for Structurizr's on the statement's line.

**Entries, values and selectors.** An entry is a node a rule's `at` selects; it has its own span and, when it starts and ends its lines, its line span (section 4.1.1). A value is the text of a node or token a `{node}` slot reaches, or the group `value` of its `match` (section 5.2). A selector is section 4.2's, in which a segment names a child node by its rule or alias, or a token by its terminal:

| Segment        | Matches |
|----------------|---------|
| a name         | the child nodes of that rule or alias, and the child tokens of that terminal |
| `*`            | any child node or token |
| `**`           | any number of levels, including none |
| `name[path='v']` | a child named `name` that has, at the relative selector `path` (segments without tests), a node or token whose text is `v` |
| `.`            | in a `{node}` slot only, as the whole selector: the entry itself |
| `..`           | *(FBL 0.15)* in a `{node}` slot and in the slots of `body.prefixes` only: the node that holds this one; nothing from the root |
| `+`            | *(FBL 0.15)* in a `{node}` slot only: the next child of the node that holds this one, an empty optional part being no child; nothing from the last |

An absolute selector is matched from the root, so `/statement` selects the root's children named `statement`, and `/` alone the root; a slot's selector is relative to its entry. `{capture}` segments are not admitted in this family. An entry's children are its nodes and tokens in document order, an empty optional part being no child.

*(FBL 0.11)* **Escapes.** A `{node}` slot may take `escape`, `{"with": "\\", "quote": "\""}`, one character each, for a value written inside quotes that escapes them. Reading replaces `with` followed by `quote` by `quote` and `with` followed by `with` by `with`, and keeps `with` before any other character as it is, so `"Jo \"the user\""` reads `Jo "the user"` and `C:\temp` reads `C:\temp`; the escape applies to the value after `match`, whose group `value` stays the value's span. Writing escapes every `quote` and every `with` character of the value (section 6.3). Without `escape`, a value is read and written as it is.

*(FBL 0.13)* **Several items in one node.** A `{node}` slot of a `many` attribute may take `split`, a text of one character or more, for a format that writes several items in one token, as Structurizr writes the tags `"Extra, More"`. The slot's value, the group `value` of its `match` or else the node's text, is cut at every occurrence of `split`, from the left; each piece, without the **white space** at its start and at its end (the characters of Unicode's `White_Space` property), is an **item**, whose span is those bytes, and a piece that is then empty is no item: `" Extra ,More,, "` holds the two items `Extra` and `More`, and `""` none. With `escape`, each item's escapes are resolved after the cut; `split` **MUST NOT** hold the escape's `with` or `quote` character (section 14.1), so that cutting before the escapes are resolved gives the items cutting after would. The items of every node the slot reaches follow each other in document order, a node's items in the order they are written, and a slot of `gather` (section 5.2) may split its own nodes. A set of the attribute is written as section 6.3 says. Without `split`, a node is one item, as before 0.13.

*(FBL 0.12)* **Segments in any order.** A grammar may read a part of a statement as **segments** that stand in any order, each a node with its own span, as `wardley-map.lark` reads what follows a component's coordinates: `label [-30, 12]`, `( market )`, `url(name)` and `inertia`, and any other word. A slot bound to a segment selects it by its node, wherever it stands among the others. A set replaces that segment's value alone (`replace-value`), and every other byte of the statement stays; a segment that is absent is written by the attribute binding's `create` at the place it declares (*Writing*); and a segment set to the empty value with `empty: "remove"` is removed with one separator, the whitespace before it, and nothing else. A `many` attribute reads every segment its selector reaches, in written order, each an item, and a set writes only the difference (section 5.2): a new segment goes right after the slot's last one, or at `create.after` when there is none, and a removed one goes with the whitespace before it. So the Wardley map's decorators read and write as one set, a decorator the tool type does not know being an item as written. Where a segment stands twice for a single-valued slot, the first is read and written, and the second is kept as it is.

*(FBL 0.15)* **A value above the entry.** A format can write once what several entries share: Turtle writes a subject once for every triple of its statement. A `{node}` slot's selector reaches it with `..` and `+`: from the `object` of `ex:a ex:p ex:b .`, `../../../../subject/iri` reaches the statement's subject and `../../verb` the predicate. Such a value is the value of each entry that reaches it, and a set of it is written in one place: the splices of one edit that replace one span with one text are one splice (section 6.4); two that replace it with different texts refuse the edit.

*(FBL 0.15)* **Further slots.** An attribute binding's `or` lists further slots of the entry for a value that a format writes in several forms. The first slot present, the attribute's own and then `or`'s in order, is the value; a set writes it as that slot writes (section 6.3), a slot of `or` that is read-only refusing it. Reading a `{value}` slot in `or` always finds it present, so it stands last. `map`, `default` and the binding's other options apply to whichever slot was read.

*(FBL 0.15)* **A node's name.** A `{node}` slot with `name: true` reads the name of the node it reaches, its rule, alias or terminal, rather than its text, so that `map` can state what each form of a term is: `{"node": "*", "name": true, "map": {"iri": "iri", "blank_node": "blank"}}`. Such a slot is read-only.

*(FBL 0.15)* **Terms, read through the body's directives.** `body.prefixes` declares what a slot with `expand` reads a term through:

| Property | Description |
|----------|-------------|
| `declarations` | Required. The entries that declare a prefix: each `{at, name, iri}`, `at` a selector from the root, `name` and `iri` `{node}` slots relative to the entry. |
| `bases` | The entries that declare a base: each `{at, iri}`. |
| `absolute` | Required. A regular expression whose group `value` is the IRI of a term written whole: `<(?<value>[^>]*)>`. |
| `prefixed` | Required. A regular expression whose groups `prefix` and `local` are the parts of a term written under a prefix. |
| `local` | Required. What a local name must match, whole, for an IRI to be written under a prefix. |
| `terminals` | Required. The terminals whose tokens are terms under a prefix. |

A declaration takes force at the end of its entry, and a later declaration of a prefix replaces an earlier one from there on. A slot with `expand`, `true` or `{"keywords": {text: iri}}`, reads the text of the node it reaches, in this order: a text `keywords` holds is that IRI; a text `prefixed` matches is the IRI of the last declaration in force of that prefix followed by the local part, each `\` in it dropped with the character after it kept; any other text is written whole, and its IRI is the group `value` of `absolute`, each `\uXXXX` and `\UXXXXXXXX` read as its code point, and resolved against the last base in force by RFC 3986, section 5, when there is one and it is not absolute already, the empty IRI being the base itself. The IRI of a declaration or a base is read as such a term, at its own place. A token of `terminals` outside a declaration whose prefix no declaration in force names makes the body unreadable (section 7.5), with the reason "The prefix 'x:' is not declared."; a body so has one meaning or none. Section 6.3 writes a term.

*(FBL 0.15)* **A node's number.** `body.ordinals` names numberings of nodes, each `{at, same}`: `at` lists selectors from the root, and `same` names a terminal. A slot with `ordinal`, the name of a numbering, reads the number of the node it reaches as decimal text, and is absent when that numbering does not hold the node. The nodes the selectors reach are numbered from 0 in document order, by the first byte of each, a node before the nodes it holds, of two with one span too; a token of the terminal `same` takes the number of the first token of that terminal with its text. So the numbers are a function of the body's bytes: reading the same bytes twice gives each node the same number, an edit before a node may change it, and nothing writes one. A slot with `ordinal` is read-only, and a host **MUST NOT** store such a number: DISL's `ephemeral` ids say the same of an element's id built from one (DISL §11.5.3).

*(FBL 0.15)* **The sequences of a string.** A slot's `escape` may take more than `with` and `quote`, for a format whose strings have more escapes: `sequences` maps a character after `with` to the character it stands for, `unicode` maps a letter after `with` to the number of hexadecimal digits after it that give a code point, `writes` lists the characters a value writes by their sequence, beside `quote` and `with`, and `requote` is the quote a set writes the whole node in. Reading replaces each as it stands; `with` before any other character, or before a letter of `unicode` without that many digits or with no code point, is kept as it is, as before 0.15. A grammar that states its strings' escapes in its terminals never gives it such a value. A `match` with one group `value` across several quoting forms reads each: `(?:"\""|"|')(?<value>(?s:.*?))(?:"\""|"|')`.

*(FBL 0.15)* **The order of a reading.** With `body.order` `closed`, a reading's elements are in the order their entries end, by the last byte of each, an entry held by another before it, of two with one span too, rather than in document order: so the triples of a blank node's property list come before the triple that names the blank node, as a reader that states a triple when its construct closes gives them. Every rule of the binding reads so; the order is section 15.3's reading order, and what `after-last` follows stays document order.

*(FBL 0.11)* **Views.** A binding of this family may have `blocks` (section 4.7), which name nodes that produce no element:

- A BlockRule of the family has `at` and no `line`. With `view`, a `{node}` or `{value}` slot, its entries are **views** of the body, each named by the slot's value, which a registration's `view` header selects (section 9.3); a `{value}` slot gives the name of a view written without one. With `origins` (section 5.1), it is a view for readings of those origins only.
- An element or relation rule with `selectedView: true` matches only the entry of the view the reading shows and the entries inside it (section 9.3); in every other view the same entries are not read, and are unbound for that reading. A rule so reads what the view's own entry says, such as the kind of view it is, as well as its statements.
- Every view is still an entry of the body, whichever view a reading shows, so a rule about the model (DISL's `View`) sees all of them, and an element rule may select the same node.
- A BlockRule's `insert` adds one of its entries from its `emit`, at the place it states, planned and undone as an element's insert is (section 6.2); the new entry produces no element, and adding a view creates no registration (section 8.4).

**Reading in part.** Without `recover`, a body that the grammar does not read, or in which text is matched by no terminal the parser's state can shift, is unreadable (section 7.5). With `recover: {"unit": u, "sync": [...]}`, the start rule's one expansion **MUST** be `u*`, and the body is read **unit by unit**, so that a part that does not parse leaves the rest readable:

1. From the start of the body, and after each unit or skipped text, the reader passes trivia and parses one unit, with `u` as the start symbol: it reads tokens while the parser can shift them. When the text at the position after the last token gives no token the parser's state can shift, or the body ends, and the parser can accept `u` there, the unit ends after its last token.
2. When the parser can neither shift nor accept, reading **stopped**: at the first byte of the token the parser could not shift, or of the text no terminal matched. The reader then skips from the unit's first byte to the end of the first token of a **synchronising terminal** that ends after the place reading stopped, lexing from that place with every terminal of the grammar as a candidate and ATD section 3.6's rule among them, and passing a character no terminal matches; to the end of the body when no such token follows.
3. Each `sync` entry names a terminal of the grammar, or is the text of one of its anonymous string terminals (`"."`).

The units read are the root's children, in order, as they would be in a body that parses. The skipped text is unbound: it is kept byte for byte, no rule matches anything in it, and it is reported once, as `fbl.unparsed-text` at the line and column where reading stopped (section 7.4). A body that parses as a whole is read as the units it is made of, so `recover` changes nothing for it.

*(FBL 0.11)* With `recover.nested`, `{"open": o, "close": c}`, each a terminal of the grammar or the text of one of its anonymous string terminals, a unit is read in part inside its blocks too, continuing the rules above:

4. The reader keeps, for each block it is inside, the parser's state at the block's last **boundary**: right after the block's `open` token, and after each `sync` token shifted at that depth.
5. When reading stops inside a block, the text from that block's last boundary is skipped to the end of the first `sync` token that ends after the place reading stopped, lexing with every terminal as rule 2 does; when the skipped text reaches a `close` token at the block's own depth first, the skip ends before it. The parser is restored to the boundary's state, and reading continues after the skipped text. The skipped text is unbound and reported once, as `fbl.unparsed-text` at the line and column where reading stopped.
6. When the body ends inside blocks, each block still open ends at the end of the body: the reader closes it without text (writing nothing, and a `sync` token first where the grammar needs one), its node's span ends at its last token, and it is reported as `fbl.unclosed-block` at its `open` token.
7. A `close` token that the parser cannot shift and that closes no block is skipped alone, unbound, and reported as `fbl.unmatched-close`; it closes nothing.
8. With `recover.unclosed`, a map from an ignored terminal to a text, a token of that terminal that does not end with its text is reported as `fbl.unclosed-comment` at its first line; it is still trivia.
9. An add whose container is a block reported `fbl.unclosed-block` is refused with the reason "The block is not closed, so nothing can be added to it." Every other edit is planned as usual.

Without `nested`, FBL 0.9's recovery stands. A body that parses as a whole is read as without it, and a unit at the top level that does not parse is skipped as rule 2 says.

**Writing.** The family writes with `replace-value`, `insert-entry`, `remove-entry` and `rewrite-reference` (section 6.1), and, *(FBL 0.11)*, `insert-key`, `remove-key` and `open-block`. Section 6.3 gives the new text.

- *(FBL 0.11)* **An absent part.** An attribute whose node is absent from the entry is set by its binding's `create`, `{"emit": " \"{value}\"", "after": ["description", "name"], "pad": true}`, which declares the part: `emit` is its text, `{value}` the value written as section 6.3 writes it, with the slot's `escape`; `after` lists, nearest first, the attributes of the same rule whose parts precede this one. The part is written right after the last token of the first attribute of `after` that is present, so before a block, a trailing comment and anything else after it, by one `insert-key`; when none of them is present, the set is refused. With `pad: true` the part is positional: each attribute of `after` before the first present one is written too, before it, with its own `create.emit` and the empty value, nearest last, in the same splice. A part that is present, empty or not, is replaced and never padded again. Setting an attribute to its empty value with `empty: "remove"` is a `remove-key` of the part's span and the whitespace between it and the token before it. Clearing the last positional part removes that part alone and leaves the empty parts before it. `absent` and `empty: "remove"` then apply as in the other families; without `create`, an attribute whose node is absent cannot be set.
- *(FBL 0.11)* A set, or a part written where it was absent, on an entry that opens a block changes neither the block's opening nor any byte of the block.
- *(FBL 0.16)* **A statement continued.** A format can write a new entry into a statement that already holds its kind, as Turtle continues a subject's statement with `;`. An insert's `continues`, `{"at": selector, "same": {attribute: Slot}, "separator": ";", "emit": "{p} {o} ."}`, says so: an add whose values `same` finds in an entry `at` selects is written into the last such entry in document order, as section 6.3 says, and any other add is written as `place` and `emit` say.
- *(FBL 0.16)* **A part that goes with its separator.** A format can write one entry inside lists it shares with others, as Turtle writes an object in a `,` list inside a `;` list inside a statement. A remove's `parts` and `whole` say so: the entry goes as the smallest part of those lists that is its alone, with one separator, and the whole statement goes when nothing else is left in it (section 6.3). Other entries that read what the part held keep what they share, since a part is removed only while something else is left in its list.
- *(FBL 0.11)* **A block written around a first child.** An element rule's `opens` is, in this family, `{"open": " {", "close": "}"}`. An add with `place: "last-child"` into an entry that has no block writes `open` right after the entry's last token and before a trailing comment, then the child, then the closing line, `close` indented like the entry, as `open-block`, `insert-entry` and `open-block` splices; its undo removes all of it. A line feed in `open` writes a line ending and the entry's indentation, for a format whose opening brace stands on the next line.

Because the text a slot or an `emit` writes is not quoted by FBL, and escaped only by a slot's `escape` (a value is written as it is, inside what its `match` keeps), a host **MUST** read the changed entry again before it applies an edit: a set after which the entry does not read with the value set, and an add after which the body does not read with one more entry of the rule, that entry having the add's values, are refused (section 6.4). So a value that is not a token of its terminal, such as a Turtle label holding a `"` that `match` does not admit, or a value holding a line break in a one-line string that `escape` cannot write, never reaches the body.

The family's guarantees are FBL's: reading never fails on content (section 7.4), unbound text is kept byte for byte (section 4.1.2), every edit is splices with an exact undo (sections 6, 7), and two hosts plan the same splices (section 6.5). The family's round-trip fixtures are `fixtures/w3c-turtle-grammar/` and `fixtures/w3c-turtle-grammar-partial/`; *(FBL 0.15)* `fixtures/grammar-reach/`, `fixtures/grammar-expand/`, `fixtures/grammar-ordinals/`, `fixtures/grammar-string-sequences/` and `fixtures/w3c-turtle-triples/` state 0.15's constructs; *(FBL 0.16)* `fixtures/grammar-continue/`, `fixtures/grammar-remove-parts/`, `fixtures/grammar-blank-line/` and the steps of `fixtures/w3c-turtle-triples/` state 0.16's.

---

## 5. Rules

### 5.1 Element rules

An element rule turns entries into elements of one node type:

| Property          | Type                        | Description |
|-------------------|-----------------------------|-------------|
| `name`            | Name                        | Required. Referenced by `parent`, `cascade`, `reference.to` and `within`. |
| `type`            | TypeRef                     | Required. The node type produced, as the tool type's specification declares it. |
| `at`              | Selector                    | yaml, json, xml, *(FBL 0.9)* grammar: where the entries are. |
| `line`            | Regex                       | lines, blocks: the statement; bullets: the statement, or with `within` the item's text (section 4.8). Exactly one of `at` and `line`. |
| `within`          | Name[]                      | blocks: the block rules or element rules whose block encloses the statement (section 4.7); *(FBL 0.7)* bullets: the sections whose items the rule matches (section 4.8). |
| `opens`, `caseInsensitive` | | `opens`: blocks (section 4.7), *(FBL 0.11)* or in grammar `{open, close}`, the text of a block written around a first child (section 4.9); `caseInsensitive`: lines, blocks and bullets. |
| `selectedView`    | bool                        | *(FBL 0.11)* grammar: the rule matches only the entry of the view the reading shows and the entries inside it (sections 4.9, 9.3). |
| `files`           | Name[]                      | Folder subjects: the file rules whose files this rule reads (section 10). |
| `when`            | Expression                  | The entry is matched only when this holds. |
| `origins`         | origin[]                    | *(FBL 0.10)* The readings the rule is read for, as origins of the binding's `claims.origins` (section 12.1). Absent: every reading. |
| `id`              | IdBinding                   | Where the element's id is stored (section 5.3). |
| `parent`          | `{rules, slot}`             | Containment (section 5.5). |
| `attributes`      | map attribute → AttributeBinding | Section 5.2. |
| `insert`          | Insert                      | How a new element is written (section 6.2). Absent: adding is refused. |
| `remove`          | Remove                      | How an element is removed (section 6.2). Absent: removing is refused. |
| `undo`            | `"inverse"` (default), `"snapshot"` | Section 7.1. |
| `readOnly`        | bool or LocalizedText       | Section 3.4. |

*(FBL 0.2)* A rule's `type`, and the attributes its bindings name, are the metamodel's, unless the tool type's specification maps the binding's names onto its metamodel with `persistence.typeMap` (DISL §11.2): then they are the binding's own names, and the map says what each type and attribute becomes, including a type read as the diagram's attributes, as a header, as an unreadable entry or as a relation whose ends are reference attributes. FBL reads and writes the body by the binding's names either way.

**Origins** *(FBL 0.10)*. A binding that several tool types read (section 9) serves readings whose metamodels differ. A rule with `origins`, an element rule or a relation rule (section 5.4), is a rule of the readings whose origin it lists and of no other: for a reading of another origin it matches no entry, its `insert` plans no add and its `remove` no removal. Absent, the rule is a rule of every reading, as before 0.10. *(FBL 0.11)* A block rule (sections 4.7, 4.9) takes `origins` with the same meaning: with it, the block rule's entries are views for readings of those origins only (section 9.3), and for a reading of another origin they are no view. Every origin a rule lists, an element, relation or block rule, **MUST** be one of the binding's `claims.origins` (section 14.1). One entry can so be one element to one reading and another to another: `/resources/pipelines/{pipeline}` of a Databricks bundle file is a `Resource` to the bundle reading and the `Pipeline` itself to the pipeline reading, through two rules whose `origins` do not overlap, and neither reading holds an element of a type its metamodel does not declare.

When several rules match the same entry, the first in the binding's order (elements before relations) whose `when` holds takes it, *(FBL 0.10)* among the rules whose `origins` admit the reading; an entry becomes at most one element or relation **per reading**. Rules of the same node type are how a type is written in several shapes (the timeline's `Period` and `Moment` are one kind of entry, told apart by `has(entry.end)`).

### 5.2 Slots and attribute bindings

A **slot** is exactly one place a value is read from and written to:

| Slot                     | Families          | The value |
|--------------------------|-------------------|-----------|
| `{key}`                  | yaml, json        | The value of that member of the entry's mapping; with `child`, of the mapping the relative selector `child` reaches. |
| `{attribute}`            | xml               | The value of that attribute of the element, or of the element `child` selects. |
| `{text: true}`           | xml               | The text of the element, or of the element `child` selects. |
| `{group}`                | lines, blocks, bullets | The named group; with `word` (and `flag`), a word of it (section 4.6). |
| `{continuation: true}`   | bullets           | *(FBL 0.7)* The item's continuation, its lines with their indentation taken off (section 4.8). |
| `{node}`                 | grammar           | *(FBL 0.9)* The text of the first node or token, in document order, that the relative selector `node` reaches from the entry (`.` is the entry itself); with `match`, a regular expression with a named group `value` that the whole text must match, that group, whose span is then the value's span (section 4.9). *(FBL 0.11)* With `escape`, the value's escapes are resolved (section 4.9). For an attribute with `many` the slot reads every node its selector reaches, in document order, each an item, and a set writes the difference as section 6.3 says for a word slot: a `replace-value` of a changed item, an `insert-key` by the slot's `create` of a new one, a `remove-key` with one separator of a removed one; where a node stands twice for a single-valued slot, the first is read and written. *(FBL 0.12)* With `items`, a map from an item attribute to a `{node}` slot relative to each node the selector reaches, the attribute is a list of **structured values**: one item per node, in document order, each a map of its item attributes with a span of its own, so that `{"node": "positions/position", "items": {"visibility": {"node": "visibility"}, "maturity": {"node": "maturity"}}}` reads the annotation `[[0.43,0.49],[0.08,0.79]]` as `[{"visibility": 0.43, "maturity": 0.49}, {"visibility": 0.08, "maturity": 0.79}]`. `separator` is the text written between two items, `", "` when the slot gives none. Section 6.3 writes them. *(FBL 0.13)* With `split`, a text, each node the selector reaches holds the items its value is cut into at that text, so that `tags "Extra, More"` holds two (section 4.9); `separator` is then the text written between two of them, and section 6.3 writes them. *(FBL 0.15)* The selector may reach up and across with `..` and `+`; with `expand` the value is the term's IRI, with `ordinal` the node's number and with `name` the node's name (section 4.9). |
| `{parent: a}`            | all               | The value of attribute `a`'s slot in the enclosing entry the rule's `parent` names, or the nearest enclosing matched entry. Read-only unless that entry's slot is writable. |
| `{capture: c}`           | yaml, json, xml   | The key a `{c}` selector segment matched; writing it renames the key. |
| `{value: cel}`           | all               | Computed; read-only. |

An **attribute binding** is a slot bound to one attribute of the rule's type, with these options:

| Property    | Values | Meaning |
|-------------|--------|---------|
| `empty`     | `"remove"`, `"keep"`, `"refuse"` | Setting the attribute to its empty value (empty string, empty list, null) removes the slot (`remove-key`), writes the empty value (`keep`, the default for strings), or is refused. *(FBL 0.6)* The rule, with the same default, applies to an insert as well as to a set: it decides whether a new entry that an add gives this attribute's empty value has its key (section 6.2). |
| `absent`    | map family → `"insert"`, `"refuse"` | Setting a value when the slot is absent from the entry: insert it (`insert-key`, the default) or refuse, per family. |
| `default`   | value | The value read when the slot is absent. |
| `number`    | `"shortest"`, `{decimals}` | How a number is written (section 6.3). |
| `time`      | `"keep-precision"` | A written date or date-time keeps the precision of the value it replaces (section 6.3). |
| `style`     | `plain`, `double`, `single`, `literal`, `flow` | yaml: the style of a newly written value, when the value it replaces gives none (section 6.3). |
| `reference` | `{to, by}` | The value names an entry of the rules `to` by the value of their attribute `by`, or, with `by: "id"`, by the id those rules store with `id.from` (section 5.7). DISL reserves `id` as an element's built-in field, so it is never an attribute's name. |
| `map`       | map wire value → model value | Translates values both ways. *(FBL 0.7)* In a rule with `caseInsensitive`, a wire value matches a key ignoring the case of ASCII letters, and is kept as written when the model value does not change. Several wire values may map to one model value; writing uses the first key in document order of the map whose value matches, unless the value being replaced already maps to it, in which case it is kept. |
| `override`  | Slot | A second slot that wins on read when present and is removed (`remove-key`) when the attribute is written. |
| `secondary` | Slot | *(FBL 0.10)* A second slot read **only when the attribute's own slot is absent** from the entry, the reverse of `override`: the attribute is then the secondary's value. Writing the attribute writes its own slot (`replace-value`, or `insert-key` where it is absent, as `absent` allows) and removes the secondary (`remove-key`) in the same edit, so a pipeline's legacy `target` becomes `schema`; when both are present the own slot is read and the secondary is removed at the next write of the attribute. Removing the attribute's slot, by `empty: "remove"`, removes the secondary with it, so that the value does not come back from it. It names a place, unlike FBL 0.5's `fallback`, which is the value of a slot that is present and does not convert. **MUST NOT** be given beside `override` on one attribute binding. |
| `content`   | `"text"`, `"html-paragraphs"` | xml text slots (section 4.5). |
| `create`    | `{emit, place}`, *(FBL 0.4)* `{at: "end"}`, *(FBL 0.11)* `{emit, after, pad}` | xml: how a missing child element holding the value is written: `emit` with `{value}` as the placeholder, placed `first`, `last` or `{before: name}` among the element's children. Emptying the value removes the child with the line span it has (`remove-key`). *(FBL 0.4)* yaml, json, on a `{key}` slot with `child`: when a level of the mapping `child` reaches is missing, setting a value creates every missing level as an insert's `at: "end"` does (section 6.2), then inserts the key into the innermost (`insert-key`), all at one offset; into a mapping that exists, the key goes right after the last key of that mapping the rule binds before it, or as its first key when there is none. Emptying the value (`empty: "remove"`) then also removes the levels the key leaves empty, as `remove-empty-levels` does. *(FBL 0.11)* grammar: how an absent part is written and removed, `emit` after the first present attribute of `after`, padded with `pad` (section 4.9). |
| `gather`    | Slot[] | *(FBL 0.11)* grammar: further `{node}` slots of the same entry, whose values follow the first slot's in document order, for a read-only binding of a `many` attribute only (section 14.1), such as a declaration's tags followed by the `tags` statements of its block. Section 3.3 stands: a gathered attribute is never written. |
| `or`        | Slot[] | *(FBL 0.15)* grammar: further slots of the same entry; the first present, after the attribute's own, is the value and is written as that slot writes (section 4.9, *Further slots*). |
| `readOnly`  | bool or LocalizedText | Section 3.4. |
| `refusals`  | `{empty, absent}` | *(FBL 0.10)* The sentence a host shows, each a LocalizedText, when a set or an add is refused by `empty: "refuse"` (`empty`) or a set by an `absent` refusal (`absent`); section 6.4. |
| `read`      | `"text"`, `"typed"` | *(FBL 0.5)* yaml, json: what the slot reads from a scalar. `text`: its written text (section 4.1.5). `typed`: its typed value (section 4.1.4), converted to the attribute's type. Absent: `text` for an attribute whose type is a string, `typed` for every other (below). |
| `quoted`    | `"text"` (default), `"convert"` | *(FBL 0.5)* yaml, with `read: "typed"`: whether a quoted scalar converts to a number or a bool from its written text (`"12"` reads as 12) or stays a text that does not convert. |
| `fallback`  | `{value, report}` | *(FBL 0.5)* What the attribute is when its slot is present and its value does not convert to the attribute's type: `value`. The entry stays readable and the bytes of the slot are kept. `report` (default `true`): whether the finding `fbl.unconverted-value` is reported (section 7.4). |

**Reading a scalar** *(FBL 0.5)*. What a slot of a yaml or json body reads is settled by `read`, and without it by the attribute's type in the tool type's specification:

| The scalar is written | A string attribute, an id, a reference; or `read: "text"` | An int or number attribute | A bool attribute | An attribute of any other type, without `read` |
|---|---|---|---|---|
| `yes` | `"yes"` | does not convert | `true` | its typed value, `true`, converted |
| `1.0` | `"1.0"` | 1.0 (an int attribute: 1) | does not convert | 1.0, converted |
| `007` | `"007"` | 7 | does not convert | 7, converted |
| `"12"` | `"12"` | does not convert; 12 with `quoted: "convert"` | does not convert | `"12"`, converted |
| nothing (`key:`) | `""` | null: the attribute is absent, and `default` applies | the same | the same |
| the key is absent | the attribute is absent: `default` applies | the same | the same | the same |

- A slot read as text **MUST** give the written text, whatever the scalar's typed value. `map` translates the written text. An id (section 5.3) and a reference (section 5.7) are read and compared as written text, with or without `read`.
- A slot read as typed converts the typed value to the attribute's type. A conversion that loses nothing succeeds (the int 1 to a number, the double 1.0 to an int); any other does not convert. With `quoted: "convert"`, a quoted scalar is first read by the core schema from its written text.
- A value that does not convert makes its entry unreadable (section 7.4), unless the binding gives the attribute a `fallback`.
- A binding type the tool type's specification maps with `typeMap`, and a host attribute, have the type of the attribute they map to; a host attribute, which has none, is read as text.
- `read`, `quoted` and `fallback` **MUST NOT** be given on a slot of the `xml`, `lines`, `blocks` or `bullets` family, nor on a `value` or `capture` slot; `fallback` applies there in no case, since a text always reads.

These are the two places where 0.5 gives a meaning that 0.4 left open. 0.4 said what CEL sees of a scalar (section 4.1.4) and that a value which "does not convert to the attribute's type" makes an entry unreadable, and did not say what a string attribute reads from a scalar that the core schema types otherwise, nor how an id written `007` compares. Hosts read them differently; 0.5 settles both as the written text, which is what the tools that own such files do.

```json
"attributes": {
  "label": { "key": "label", "empty": "keep" },
  "begin": { "key": "begin", "read": "text", "time": "keep-precision" },
  "row":   { "key": "row", "default": 0, "quoted": "convert", "fallback": { "value": 0, "report": false } }
}
```

`label` is a string attribute and reads the written text unasked. `begin` is a date-time attribute that this tool keeps as the text it was written with, so that it can report a time it cannot read in its own words. `row: wide` reads as row 0 and the element is still drawn; `row: "3"` reads as 3.

An attribute of the node type that no binding names is not stored in the body: it takes its DISL default, or is derived as DISL says. A DISL attribute marked `fixed` (DISL §4.3) **MUST NOT** be bound. *(FBL 0.8)* A value of the file's layout that every new entry of a rule carries and that no attribute holds, such as the value of the key that tells the rule's entries from others in the same list, is written with the insert's `constant` (section 6.2), not with an attribute.

### 5.3 Ids

How ids are made, derived and whether they are ephemeral is DISL's (`persistence.ids`, DISL §11.5). FBL says only where an id is stored, with the rule's `id`:

- `{"from": Slot}`: the id is the value of the slot. It is read from it, and written to it when the element is created (with a new id DISL's strategy generates, once, when the gesture happens, so a redo re-creates the element under the same id). A rename of the id is a `replace-value` plus the reference rewrites of section 5.7. *(FBL 0.5)* The id is the slot's written text (section 4.1.5): an entry written `id: 007` has the id `007`, and a reference written `7` does not name it.
- `{"sidecar": {"key": cel}}`: the id is kept in the registration's `identities` block (section 8.6), keyed by the value of `key`, for formats whose entries have no id of their own (a Wardley map's components are named, not identified). *(FBL 0.12)* `kind` names the key space of the rule's identities and `ordinal` gives each later holder of one key an entry of its own (section 8.6).
- No `id`: the id is the one DISL derives (`strategy: "derived"`), from the element's attributes and, through `self.location()`, its place in the file.

Ids that DISL marks **ephemeral** (an id that changes when the file is edited, such as one built from a line number) are never stored: not in a registration's layout (section 8.5), not in a sidecar. A missing or duplicate id is not a reason to refuse the body: DISL's `ids.missing` and its findings `std.missingId` and `std.duplicateId` decide, and an entry is always addressable by its place. *(FBL 0.16)* An entry's place is its rule's name and its line, `term@7`, and where several entries of one rule stand on one line, the second and later add their number on the line, `term@7#2`, in reading order, so that a change names one entry: `ex:p ex:b , ex:c` holds two.

### 5.4 Relation rules

A relation rule is an element rule with two more slots, `source` and `target`, each **REQUIRED**. Their values reference the ends by id: an end whose value names no element is a **dangling reference**, reported as `fbl.dangling-reference` on the entry, and the relation is not created (DISL relations always have both ends). A relation whose ends are its enclosing entries takes them with `{parent: …}` (a Databricks dependency's target is the task whose `depends_on` it is in).

### 5.5 Containment

`parent: {rules, slot}` makes the element a child, in DISL containment slot `slot`, of the nearest enclosing entry matched by one of `rules`: the nearest ancestor node in the tree families and xml, the nearest enclosing block in `blocks`, *(FBL 0.7)* the nearest item the entry is nested in in `bullets`, which is the nearest open item above it with its marker in a smaller column (section 4.8). A rule without `parent` produces top-level elements. Moving an element to another parent is a `remove-entry` in the old parent and an `insert-entry` in the new one, in one edit, keeping the entry's own bytes. *(FBL 0.7)* In `bullets` they are the entry's own bytes re-indented: a move to another depth shifts the indentation of every line of the entry by the difference (section 6.3), and changes nothing else.

### 5.6 The header

`header` names a version or identity mark: a root `key` with a `value` (yaml, json), or a `line` expression the first statement or the root start tag must match (lines, blocks, xml), or *(FBL 0.7)* that line 1 must match, after a byte-order mark (bullets). A body without the mark, or with another value, is read, and the difference is reported as `fbl.header-mismatch` (a warning). With `required: true`, a body without the mark is unreadable (section 7.5) instead. The header is never written except by the template.

### 5.7 References and rename

A slot is a **reference** when its attribute binding has `reference`, or when it is a relation's `source` or `target` (which reference ids). A reference by id is written `by: "id"`, and every rule it names **MUST** store its id with `id.from`. When the referenced value changes (the `by` attribute of an entry of the `to` rules, or an id stored with `from`), every reference to the old value is rewritten in the same edit, one `rewrite-reference` splice each. A reference inside a `word` or a list value is rewritten in place, word by word or item by item. A rename to a value another entry of the same rules already has is refused.

A group holding several references (a causal loop's members) is split into words, each a reference.

---
## 6. Writing

### 6.1 The splice catalogue

Every write is a **splice**: the replacement of one byte range of the body (possibly empty) with new text (possibly empty). Every splice is one of the twelve operations below, and every edit consists of such splices only. Each has an exact inverse: the splice that puts the replaced bytes back.

| Operation           | Replaces | With | Families |
|---------------------|----------|------|----------|
| `replace-value`     | a value's span, quotes included | the new value, in the value's own style when it fits (section 6.3) | all |
| `insert-key`        | nothing, at the position `insert.keys` or the attribute order gives | a new key and value (yaml, json), attribute (xml), optional segment or word (lines, blocks, bullets), item's continuation (bullets), child element holding a value (xml `create`), or *(FBL 0.11)* absent part (grammar `create`) | all |
| `remove-key`        | a key or attribute and the trivia it owns; a word and the whitespace before it; an `override` slot; a `create`d child; an item's continuation; *(FBL 0.11)* a part and the whitespace before it (grammar) | nothing | all |
| `insert-entry`      | nothing, at the place `insert.place` gives | a new entry with its separators and line endings | all |
| `remove-entry`      | an entry's line span, or its own span and one separator | nothing | all |
| `ensure-container`  | nothing, at the place `insert.create` gives | a missing container (a key with an empty block value, an array, an element, a section's heading) | yaml, json, xml, bullets; the registration's blocks |
| `remove-container`  | a container left empty and its line span | nothing | yaml, json, xml; the registration's blocks |
| `rewrite-reference` | a reference's span | the new value it names | all |
| `re-emit-line`      | a statement's own span, or an item's text | the statement or text emitted from its rule's `emit` with the element's current values | lines, blocks, bullets |
| `open-block`        | nothing, before the statement's line ending; and nothing, after the block's last line; *(FBL 0.11)* grammar: nothing, after the entry's last token and before a trailing comment | ` {`; and a closing `}` line indented like the statement; *(FBL 0.11)* grammar: `opens.open` and `opens.close` | blocks, grammar |
| `self-close`        | the `>` of a start tag and the end tag with the whitespace before it; or `/>` | `/>`; or `>` and a new end tag | xml |
| `replace-separator` | *(FBL 0.16)* the last token of an entry an insert's `continues` writes into | the text `continues.separator` gives (section 6.3) | grammar |

*(FBL 0.7)* In a declared block of the registration whose order is `kept`, a move of an entry is a `remove-entry` and an `insert-entry` of the same bytes (section 8.4).

A host **MUST NOT** write a body by any other means than these splices, except when the user explicitly replaces the body with the editor's own version after an external change (section 7.3).

### 6.2 Inserting and removing

**Insert.** A rule's `insert` says where a new entry goes:

| Property    | Description |
|-------------|-------------|
| `place`     | Required. `after-last`: right after the last entry of this rule in the container (document order); if there is none, at `end`. `end`: after the container's last entry. `start`: before its first entry. `last-child`: after the parent entry's last child (xml, blocks, bullets), opening the parent first when it is self-closed (`self-close`) or one-line (`open-block`). `next-sibling`: right after the entry the gesture was made on. `end-of-document`: after the last byte of the body. *(FBL 0.7)* `after-header`: as a line of its own right after the line the binding's `header` matched (lines, blocks, bullets), as a mark line beside the header needs when it is missing. `{before: key}`: before the member `key` of the container. |
| `container` | Selector of the container (tree, xml), the name of the enclosing block rule (blocks), or *(FBL 0.7)* the name of the section (bullets); in `bullets`, the container of an add with a parent is that parent item, whose nested items are its entries. Relative selectors resolve against the parent entry. `{capture}` segments resolve to the values the registration or the parent entry bound them to. |
| `create`    | When the container does not exist: `at` is `end-of-document`, `{before: key}` (before a member of the container's parent), `{after: key}` (right after a key of the entry, as the first key after it), or `{under: selector}` (at the end of that container), or *(FBL 0.4)* `end` (after the last member of the container's parent, which for a member of the root is the end of the document); `text`, when given, is the container's own text, otherwise the key alone with an empty block value (yaml), `"key": []` (json) or an empty element (xml). One `ensure-container` splice. *(FBL 0.4)* With `end`, the container's parent may be missing too: see *Missing levels* below. *(FBL 0.7)* In `bullets` the container is a section: `at` is `end-of-document`, and `text` is its heading line, such as `## Behavior` (section 6.3). |
| `keys`      | The order of keys in a new entry (tree) or of its attributes (xml). A key the new entry has no value for is left out. *(FBL 0.6)* An attribute the add does not give has no value, and its key is left out whatever its binding's `empty` says. An attribute the add gives its empty value (section 5.2) has a value, and a host **MUST** write or leave out its key by that attribute binding's `empty`, with the defaults a set has: `keep` writes the key with the empty value, in the form section 6.3 gives it (in yaml the empty string is not plain-safe, so it is quoted, `key: ""` with the default `text.quote`; in json `"key": ""`); `remove` leaves the key out; `refuse` refuses the add (section 6.4). A binding without `empty` **MUST** give the key what a set of that attribute to its empty value gives the slot, which for a string attribute is `keep`. So one binding gives the same entry for an add as for an add followed by a set to the same value. A new entry written with `emit` follows `emit`'s own rule for optional segments instead. *(FBL 0.8)* A constant's key that `keys` names takes that place, with the constant's value. Every other constant follows the entry's other keys, in the order `constant` lists its members, and comes before `skeleton`. *(FBL 0.10)* An attribute bound with `child` (section 5.2) that the add gives a value is written in the new entry under the levels of `child`, which the entry gets as a set creates them (`create`, section 5.2): the outermost level's key at its place in `keys` when `keys` names it, else after the entry's other keys and before its constants and `skeleton`, each deeper level one indentation step deeper (section 6.3) and the attribute's own key in the innermost, all in the entry's one `insert-entry`; attributes whose `child` reach one level share it, in the order of their bindings. In yaml a level is its key with a block mapping below it, in json a member whose value is an object. So a library added to a YAML pipeline is `- notebook:` with `path:` one step deeper below it, and reads back as the library added. |
| `emit`      | The text of a new entry for families without keys (lines, blocks, xml, bullets, *(FBL 0.9)* grammar, where a line feed in it starts a new line, section 6.3), and for json items written on one line: literal text with placeholders `{attribute}` (the attribute's written form, section 6.3), `{id}`, `{source}`, `{target}`; `[ … ]` encloses an optional segment, written only when every placeholder in it is non-empty, and *(FBL 0.7)* `\[` and `\]` write a literal bracket (`\[ \] {text}` writes an open task of a Markdown task list). A `flag` attribute's placeholder writes the flag's word when true and nothing when false. *(FBL 0.7)* The placeholder of an attribute with `map` writes the wire value that section 5.2 gives the value, and the placeholder of an attribute the add does not give writes its binding's `default`, or nothing without one. |
| `constant`  | *(FBL 0.8)* yaml, json: a map from a key of the new entry to a scalar (a string, a number, a bool or null) that every new entry of the rule is written with, and that no attribute supplies, such as `{"type": "ui-element"}` for a rule chosen by `when: "entry.type == 'ui-element'"`. Its place among the keys is in `keys`, above. **MUST NOT** be given with `emit`, nor in a binding of another family (section 14.1). |
| `skeleton`  | Fixed text written with the entry after its keys, indented as its keys (a Databricks task's `notebook_task`). |
| `when`      | An expression over `attributes`; when false, adding is refused. |
| `continues` | *(FBL 0.16)* grammar: `{at, same, separator, emit}`, a statement a new entry may be written into rather than where `place` puts it (sections 4.9, 6.3). `at` selects the statements from the root; `same` maps an attribute of the rule to a `{node}` slot relative to the statement, whose value must equal what the add gives that attribute; `separator` is the text that replaces the statement's last token; `emit` is the new entry's text, as `emit` is. |
| `refusals`  | *(FBL 0.10)* `{when, container}`, each a LocalizedText: the sentence a host shows when the add is refused because `when` does not hold (`when`), or because the container is missing and the insert has no `create` (`container`); *(FBL 0.14)* and, for an insert that adds a member (below), because the add gives no key or an empty one (`key`), or a key the mapping already holds (`duplicate`); section 6.4. |

**Constants** *(FBL 0.8)*. A host **MUST** write every member of an insert's `constant` into each new entry of the rule, in the entry's one `insert-entry` splice, at the place `keys` gives it. Its value is written as section 6.3 writes a new value of its JSON type, with the binding's `text` defaults and none of an attribute binding's options: a string by the yaml scalar rules (so `"1.0"` or `"yes"` is quoted, and reads back as the same text) or as a json string, a number as `number: "shortest"` writes it, and `true`, `false` and `null` as such. A constant takes no part in reading: which rule reads an entry is decided by the rules' order and their `when` alone (section 5.1), so a binding keeps its `when` and its constants in agreement itself, and a round-trip fixture is what shows that a new entry is read by the rule that added it.

An add is planned by the first rule of its type, in the binding's order, that has an `insert` whose `when` holds, *(FBL 0.10)* among the rules whose `origins` admit the reading the add is made in (section 5.1). *(FBL 0.7)* In `bullets` an item's siblings are the items nested in the same parent, or the top-level items of the section when it has none: `after-last`, `end` and `last-child` put a new item after the last of them, `start` before the first, and section 6.3 says where that is in bytes.

An insert is one `insert-entry` splice, preceded by an `ensure-container` splice at the same offset when the container is created. Splices at the same offset are applied in the order the edit lists them. *(FBL 0.3)* A container that encloses its entries on one line (a json array or object written on one line) cannot be written before an entry that goes inside it, so it is created as two `ensure-container` splices, its opening and its closing, with the `insert-entry` between them, all three at the same offset; removing such a container with its last entry is likewise its opening (`remove-container`), the entry (`remove-entry`) and its closing (`remove-container`), as `self-close` splits an xml element around its children.

**A new member, named by its key** *(FBL 0.14)*. In the `yaml` and `json` families, a rule whose `at` ends in a `{c}` segment reads the members of a mapping, each named by its key (`/resources/jobs/{job}`). Its `insert` adds a **member** to the container, which is that mapping: the new member's key is the value the add gives the attribute the rule binds to `{capture: c}` (section 5.2), and a host **MUST** take it as the new entry's key rather than as a set of the attribute, so that a `readOnly` on that binding, which refuses a rename, does not refuse the add. The key is written as section 6.3 writes a new string value: in yaml a scalar, plain when it is plain-safe and else in `text.quote`, followed by `:` and the line ending, the member's value below it one indentation step deeper, holding the entry's keys in `keys` order, its constants and its `skeleton`, as a new mapping item's are written; in json a string, `: ` and an object holding them, written one member per line as its siblings are. The member goes where `place` puts it among the container's members: `after-last` right after the line span of the rule's last member, so that a new job follows the last job, before the container's members of any other rule; `end` after the container's last member; `start` before its first; `{before: key}` before that member. It is indented like the member it follows, else like the one it precedes, else one step deeper than the container's key (section 6.3). A container that is missing is created as `create` says, with its missing levels as below, so that `create: {"at": "end"}` on `/resources/jobs` writes `resources:` at the end of the body when it is missing and `jobs:` at the end of `resources`; a container written as its key alone (`jobs:`) takes the member as the first of a block mapping below the key, and one written in flow form (`jobs: {}`) takes no insert, as a flow mapping does not (below). An add that gives the key attribute no value or the empty text is refused, and so is one whose key the container already holds, compared as written text (section 4.1.5), since a mapping holds one key once; their sentences are the insert's `refusals.key` and `refusals.duplicate` (section 6.4). The member is one `insert-entry` splice of its key and its value, after the `ensure-container` splices of the levels it creates, all at one offset, and its undo removes exactly those bytes. Removing such an entry removes its line span, its key's line and every line of its value, with one separator in json, as for any entry; the rule's `remove.container` decides what becomes of a container it leaves empty, `remove-empty-levels` removing `jobs:` and then `resources:` when nothing is left in them. A rule with such an `insert` **MUST** bind an attribute to `{capture: c}` (section 14.1).

**Missing levels** *(FBL 0.4)*. An insert whose `create.at` is `end` may find more than its container missing: `container: "/view/placements"` in a body without `view`. Each missing level is created, outermost first, at the end of the level above it, and the new entry is inserted into the innermost; all of these splices are at one offset. Every level but the container itself is written as a mapping. In yaml a level is the key alone with an empty block value, one indentation step deeper than the level above it (section 6.3), and is one `ensure-container` splice; the levels' splices come first, outermost first, then the entry's. In json a level is `"key": {` and its closing `}`, and in xml an element with the key's name; each such level is two `ensure-container` splices, its opening and its closing, each on its own line, so that the openings come outermost first, then the entry, then the closings innermost first, as a container written on one line is created above. Only levels named by key segments are created: when a level a `*` or `{capture}` segment names is missing, there is no key to write, and the insert is refused. The same holds for an attribute's `create` (section 5.2), whose levels are those of its `child` selector. Removing the entry undoes this only with `remove-empty-levels`: a level is removed when nothing is left in it, which with `remove-when-empty` happens to the container alone.

**Flow sequences** *(FBL 0.10)*. In yaml, a container that is a flow sequence (section 4.3) is written into so:

- **An empty flow sequence** (`tasks: []`, nothing but whitespace between its brackets) takes an insert as a `replace-value` of the value and the whitespace between it and its key's `:` by nothing, followed by the `insert-entry` of the new entry as the first item of a block sequence under the key: after the line ending of the key's line, so after a comment on that line, which stays where it is, written as a new yaml sequence item under a key with no sibling (section 6.3), with the line endings of section 6.3. Both splices are one edit.
- **A one-line flow sequence** (`depends_on: [{task_key: a}]`) takes an insert as one `insert-entry` right before its `]`, of `, ` and the new item, after its last item; with `place: "start"`, right after its `[`, of the item and `, `. A mapping item is written in flow form, `{` its members `}`, each member `key: value` with `, ` between them, its keys in `insert.keys` order and each value by section 6.3's yaml scalar rules, a value that is not plain-safe or holds `,`, `[`, `]`, `{` or `}` being quoted; any other item is its value alone.
- **A removal** of an item of a flow sequence is a `remove-entry` of the item and one separator: the `,` after it and the whitespace up to the next item, or, for the last item, the `,` before it and the whitespace between that separator and the item, as in json (below). The only item is removed alone, leaving `[]`, and `remove-when-empty` then removes the container as for any container left empty.
- **A flow sequence written over several lines, and a flow mapping,** take no insert: an add into one is refused with a reason (section 6.4), and nothing is written.

Each of these has its exact inverse (section 7.1): the undo of an insert into an empty flow sequence puts back the value and the whitespace before it and removes the entry, so the body is the bytes it was.

**Remove.** A rule's `remove` says what goes with an entry:

| Property    | Description |
|-------------|-------------|
| `cascade`   | Rules whose entries go too when they reference the removed entry (relations whose `source` or `target` is it, elements whose reference names it). Entries nested inside the removed entry's span go with it without being named. |
| `parts`     | *(FBL 0.16)* grammar: the lists an entry is written in, innermost first, each `{item, separator}`: `item`, a selector relative to the entry, reaches the part of it that stands in the list (`.` for the entry itself), and `separator` is the text between two items of the list. The list is what holds that part, and its items are the children of the same name. See section 6.3. |
| `whole`     | *(FBL 0.16)* grammar: a selector relative to the entry reaching the statement that goes when none of `parts` holds anything else (section 6.3). |
| `container` | `keep` (default): a container left empty stays. `remove-when-empty`: it is removed with a `remove-container` splice. *(FBL 0.4)* `remove-empty-levels`: it is removed, and so is each level of its selector above it that is left empty with it, up to the first level that still holds something, a level a `*` or `{capture}` segment names, or the root; each level is removed by `remove-container` splices in the order *Missing levels* below creates it, around the entry's own. |

A removal removes the entry's **line span** when the entry starts and ends its lines, else its own span. In json, removing an item also removes one separator: the one after it, or, for the last item, the one before it together with the whitespace between that separator and the item. Cascaded entries are removed bottom-up (last in document order first); all splices are one edit.

### 6.3 New text

New text follows the body's own conventions where the body shows them, and the binding's `text` defaults only where it does not. A host **MUST** apply these rules exactly; they are what makes two hosts write the same bytes.

**Line endings.** A splice that writes line breaks uses the ending of the line its insertion point is on, or, when that line has none (the last line without a final newline), the body's dominant ending (the one that ends the most lines; CRLF when CRLF and LF tie; a lone CR counts as neither). A body without any line ending uses `text.newline`, default LF. An insertion after a last line that has no ending writes the line break before the new text and none after it, so the body still has no final newline.

**Indentation.** A new entry is indented like its previous sibling in the container, or, when it has none, like its next sibling, or, when it has neither, one indentation step deeper than its parent entry. The step is the difference between the parent's and a child's indentation anywhere in the body, the first such pair in document order; else `text.indent` (default 2, or a tab). A new yaml sequence item under a key follows `text.sequenceIndent` when the body shows no example: `indented` (default) puts `-` one step deeper than the key, `flush` in the key's column. Keys of a new mapping item align with the column after `- `.

**yaml scalars.** A replaced value keeps its style (plain, single, double, literal, folded) when the new value can be written in it; else, and for a new value, the style is chosen in this order: the attribute's `style`; plain if **plain-safe**; else `text.quote` (default `double`). A string is plain-safe when it is not empty; has no leading or trailing whitespace; contains no line break or control character; does not start with any of ``- ? : , [ ] { } # & * ! | > ' " % @ ` ``; contains neither `: ` nor ` #` and does not end with `:`; and would be read back as the same string under the YAML 1.2 core schema and as a string under YAML 1.1 (so not `null`, `~`, `true`, `false`, `yes`, `no`, `on`, `off`, `y`, `n` in any case, a number, or a date or date-time), unless the attribute's DISL type is the type it would read as. Double-quoted strings escape `\`, `"` and control characters (`\n`, `\t`, `\r`, `\uXXXX`); single-quoted strings double `'`. A multi-line string with style `literal` is written `|-` with its lines indented one step deeper than its key. A list with style `flow` is written `[a, b]`, its items by the same rules; an empty list is `[]`. *(FBL 0.5)* A value written for a slot read as text **MUST** read back through the same binding as the same text. The plain-safe test above still holds for it, so the text `yes` or `1.0` is written quoted, and a file another tool reads keeps its meaning there; an attribute whose `style` is `plain` is written plain whenever YAML's syntax allows it, for a format whose every reader takes the written text.

**json values.** Strings are written with the escapes RFC 8785 uses; numbers as below; `true`, `false`, `null` as such. A new member of an object or item of an array that is written one per line starts on a new line indented like its sibling; one in a container written on one line (or given by `emit`) is written after `, ` on the same line. The separator of the previous last entry is written as part of the new entry's splice.

**Numbers.** With `number: "shortest"` (the default), an integer value is written without a fraction, and any other number as the shortest decimal representation that reads back as the same IEEE 754 double, in ECMAScript's `Number.prototype.toString` form (RFC 8785 §3.2.2.3). With `{decimals: n}`, the number is rounded to `n` decimals, halves away from zero, and trailing zeros and a trailing point are dropped.

**Times.** With `time: "keep-precision"`, a date or date-time replacing a value is written with the precision of the value it replaces: a date as `YYYY-MM-DD`; a date-time as `YYYY-MM-DDTHH:MM:SS`, and with a fraction or an offset only if the replaced value had one. A new value is written in the precision the gesture gave it.

**xml.** New attributes are written in `insert.keys` order, then in the order of their binding; an attribute added to an existing element goes after its last attribute, preceded by one space. A new element goes on its own line when its previous sibling is on its own line, with that sibling's indentation. A self-closed tag is written `/>` without a space unless the element being replaced wrote one.

**lines and blocks.** A new statement is its rule's `emit` on its own line, indented (in `blocks`) one step deeper than its enclosing block's statement. A changed value is written by `replace-value` on its group's span. A value whose slot has no span in the statement (an absent optional group or word) is written by `insert-key` when the rule's `emit` places it after a present value and before nothing that must move, and otherwise by `re-emit-line`, which rewrites the statement in the canonical form its `emit` gives; unbound words of a re-emitted statement are lost, so a host **MUST** prefer `insert-key` when both are possible. *(FBL 0.7)* A word a set adds, the value of a word slot that holds none or a new item of a `many` one, is an `insert-key` of one space and the word, right after the last word of the group held by that slot or by a slot listed before it in the rule's `attributes` on the same group, else at the start of the group. A removed word or item is a `remove-key` of the word and the whitespace before it, and a changed one a `replace-value` of the word, or of its group `value`. Items keep their order, and a new one is written after the slot's last. A set of such a slot writes the difference between the old list and the new one: the items of their longest common subsequence stay, an item removed and one added at the same place are one change, and the rest are removals and additions.

**bullets** *(FBL 0.7)*. The family's new text, with the line endings and the last line without an ending as this section says above:

- **A new item** is one line, its indentation, its marker, one space and its rule's `emit`, written as for `lines`. It takes the marker and the indentation of its previous sibling, else of its next sibling, else the marker of its parent and the parent's content column, else `-` at column 0. When the add gives the attribute of the rule's `continuation` slot a value that is not empty, each line of the value follows on a line of its own, indented with spaces to the new item's content column, an empty line of the value written as an empty line.
- **Where it goes.** After the line span of its previous sibling; else at the start of the line span of its next sibling; else, as its parent's first nested item, at the end of the line ending of the parent's last line before its first nested item, which is its item line or the last line of its continuation; else, as the first item of a section that holds none, at the end of the line ending of the section's last line that is not blank, which is the heading when the section holds nothing else, with a line ending written first, so that a blank line stands before the item.
- **A missing section** is created by `insert.create` with `at: "end-of-document"` and the heading as `text`: one `ensure-container` at the end of the body that writes a line ending, the heading and a line ending, so that a blank line stands before the heading; when the body's last line is blank already, or the body is empty, it writes the heading and a line ending only. The item that follows it at the same offset is the first item of a section that holds none. In a body whose last line has no line ending, each of the two splices also writes the line break that line lacks, before its text, and none after it: the `ensure-container` writes two line endings and the heading, the item two line endings and its line, and the body still ends without a line ending.
- **A changed value** of a group or a word is written as for `lines`, in the item's text. `re-emit-line` rewrites the item's text, never its indentation or its marker.
- **The continuation.** Its value span runs from the end of the item line, before its line ending, to the end of the last line of the continuation, before that line's ending. Setting it is a `replace-value` of that span with the new value, every line of it written after a line ending and indented with spaces to the item's content column, an empty line written empty; on an item without continuation it is an `insert-key` at the end of the item line; setting the empty text is a `remove-key` of the span.
- **A move to another depth.** The entry's line span is removed (`remove-entry`) and inserted where a new item would go at its new place (`insert-entry`), every line of it that is not blank shifted by the difference between the column a new item's marker would have there and the column its marker had. A line is shifted right by writing that many spaces before it, and left by taking that many columns of whitespace off its start, a tab counting four columns; a tab the shift cuts into is replaced by the spaces that remain of it. The marker and every other byte stay as they were. Like every offset of an edit, the place of the insertion is taken in the body before the edit: a move to just after the entry's old parent is inserted at the start of the removed span.

**grammar** *(FBL 0.9)*. The family's new text, with the line endings and the last line without an ending as this section says above:

- **A changed value** is a `replace-value` of the value's span (the group `value` of the slot's `match`, or the node's own span) with the attribute's new value as text, unescaped and unquoted, unless *(FBL 0.11)* the slot has `escape`, in which case every `quote` and `with` character of the value is written after `with` (section 4.9); the family's read-again check (section 4.9, *Writing*) still applies, and refuses a value the escape cannot write, such as one holding a line break. A number is written as section 6.3's *Numbers* says, `true` and `false` as such.
- **A new entry** is its rule's `emit`, with its placeholders replaced as for `lines`, where each line feed in `emit` starts a new line: it is written as the line ending this section chooses, followed by the new entry's indentation. An entry's **indentation** is the whitespace before it on its first line.
- **Where it goes.** `after-last` follows the last entry of the rule in document order, `end` the last child of the container, `next-sibling` the entry the gesture was made on; `start` precedes the container's first child. The container is the node `insert.container` selects, the root when it is absent. A new entry that follows an entry goes after that entry's line span, on a line of its own, with its indentation, and ends with a line ending; when the entry followed does not end its line, the new entry is written right after its own span, after a line ending and the indentation of the line the followed entry ends on, and ends with nothing. A new entry that precedes a child goes at the start of that child's line span, with its indentation, followed by a line ending. A new entry with nothing to follow or precede, and `end-of-document`, goes after the last byte of the body: on a line of its own after a body whose last line is empty or which is empty, else after a line ending, with no indentation, the body keeping or lacking its final line ending. *(FBL 0.16)* A line feed that opens an `emit` at `end-of-document` in the `grammar` family is a blank line before the entry, left out after a body that is empty or whose last line is blank, so that `\n{s} {p} {o} .` never writes two blank lines in a row.
- **A removed entry** is a `remove-entry` of its line span, or of its own span when it does not start and end its lines.
- *(FBL 0.16)* **A new entry in a statement** of an insert's `continues` (section 6.2). The statement is the last entry in document order that `at` selects and in which each slot of `same` reads the value the add gives its attribute. Its last token is a `replace-separator` with `separator`. The new entry is an `insert-entry` at the end of the line that token stands on, so after a comment on that line: a line ending first when that line has none, else the line ending last, as *Line endings* says, and the entry's `emit` at its **indentation**, the whitespace that starts the statement's second line when the statement stands on several, else the whitespace that starts its first line followed by `text.indent` spaces. So `ex:b rdfs:label "B" ; ex:knows ex:c .  # kept` given `ex:likes ex:a` becomes `ex:b rdfs:label "B" ; ex:knows ex:c ;  # kept` and the line `    ex:likes ex:a .` after it. The family's read-again check (section 4.9) applies; both splices are one edit, undone together.
- *(FBL 0.16)* **A removed entry of a rule with `parts`** is one `remove-entry`. The first of `parts`, in order, whose list holds another item than the one `item` reaches names the **part**: that item's own span, and `separator` the part's separator. It goes as follows.
  - **With its separator.** When the token before the part is the separator, with only white space between them, the span runs from that separator's first byte. Else, when the separator follows the part on its line, with only spaces and tabs between them, the span runs through it and the spaces and tabs after it. Else, as where a comment stands between them, the part goes without one.
  - **Its line.** When something other than white space stands before the span on its first line, and white space stands both right before the span and right after it on its last line, that white space before it goes too, so that `ex:knows ex:b , ex:c ;` without `ex:c` becomes `ex:knows ex:b ;`. When nothing but white space would be left of the lines the span touches, those whole lines go with their line endings.
  - **The whole statement.** When no list of `parts` holds another item, the entry `whole` reaches goes, every line it stands on with the comment that follows it there and no comment above it, or its own span when something else stands on those lines. A line left last in a body that ends without a line ending gives up its own ending in the removal, so the body still ends without one.
  - Every byte of the entries that stay, their comments included, is kept, and a removal after which the body does not read as it did, as where a part left alone would no longer parse, is refused (section 4.9).
- A reference is rewritten by `rewrite-reference` of its value's span (section 5.7).
- *(FBL 0.12)* **An item** of a slot with `items` (section 5.2). A changed value of one item is a `replace-value` of that value's span, and nothing else of the list changes. A new item is an `insert-key`, right after the last item, of `separator` and the slot's `create.emit` with each placeholder an item attribute's value: `,[{visibility},{maturity}]` after `[0.08,0.79]`. A removed item is a `remove-key` of the item's span and one separator: the one before it, or, for the first item, the one after it. A set of the attribute writes the difference between the old list and the new one as a `many` slot does (section 5.2), so an item that is not changed keeps its bytes. In an insert's `emit`, the placeholder of an `items` attribute is every item written by `create.emit`, joined by `separator`.
- *(FBL 0.15)* **A term** of a slot with `expand` is a `replace-value` of the node's own span, written as follows: a text of the slot's `keywords` that stands for the IRI; else `prefix:local` under the declaration with the longest IRI that the new IRI starts with and is longer than, a later declaration winning a tie, whether or not it is in force at that place, when what remains of the IRI matches `local` whole; else the IRI between `<` and `>`. In an insert's `emit` a term's placeholder is written so. So renaming a term writes it in the file's own prefixes, and a writer never invents a declaration. The family's read-again check (section 4.9) applies, and refuses a term that would read otherwise there.
- *(FBL 0.15)* **A string** of a slot with `escape` and `requote`: a set is a `replace-value` of the whole node the slot reaches, with the value, escaped as `escape` says, between two of `requote`, so that `'A'` or `"\""A"\""` set to `B` becomes `"B"`, and what follows the node, such as a language tag, stays.
- *(FBL 0.13)* **An item** of a slot with `split` (section 4.9). A set of the attribute writes the difference between the old list and the new one as a `many` slot does (section 5.2): the items of their longest common subsequence stay, an item removed and one added at the same place are one change, and the rest are removals and additions. A changed item is a `replace-value` of the item's span with the new value, escaped as the slot's `escape` says, and no other byte of its node changes. A new item is an `insert-key`, right after the last item the slot reaches, of `separator` and the value, so that `"Extra, More"` given `Most` becomes `"Extra, More, Most"`; where no node the slot reaches holds an item, the new items, joined by `separator`, are a `replace-value` of the value's span of the last node it reaches (`""` becomes `"Extra, More"`), and where it reaches no node, they are the `{value}` of the one node the slot's `create` writes. A run of removed items that follows an item the set keeps in the same node is a `remove-key` from the end of that item to the end of the run's last item, so that the separator and the white space before each removed item go with it: `"One ,Two,  Three"` without `Two` becomes `"One,  Three"`. A run at the start of a node, before an item the set keeps, is a `remove-key` from the start of the run's first item to the start of that kept item: `"Extra, More"` without `Extra` becomes `"More"`. A node whose every item the set removes is removed as a node of a `many` slot without `split` is, by a `remove-key` of the node and the whitespace before it. Every byte of an item the set keeps, and between two items it keeps, stays. In an insert's `emit`, the placeholder of a `split` attribute is its items joined by `separator`.

### 6.4 Edits

A user gesture, form commit, operation or quick fix is one DISL transaction (DISL §14.4). When the transaction commits, its model changes are **planned** as splices: each model change of section 11.2 (*(FBL 0.2)* an `add`, `set`, `remove`, `move` or `retype`, with the fields given there) becomes the splices this section gives, in the order the transaction made them, then adjusted so that all offsets refer to the body before the edit. *(FBL 0.16)* Each change is planned against the body as the changes before it leave it, and a run of `remove`s is planned bottom-up, the last entry in document order first, as a cascade is (section 6.2), so that each takes its own separator and the entries still to go keep their places. *(FBL 0.15)* Two splices of one edit that replace one span with one text are one splice, and two that replace it with different texts refuse the edit (section 4.9). The result is one **edit**: its splices applied together, recorded together, undone together. If any change cannot be planned (no `insert`, no `remove`, a read-only slot, `empty: "refuse"` on a set or, *(FBL 0.6)*, on an add that gives the attribute its empty value (section 6.2), an `absent` refusal, a duplicate on rename, a plugin's refusal), the whole transaction is rejected with that reason and rolled back, and nothing is written. *(FBL 0.10)* A binding gives the sentence of a refusal it causes, as `readOnly` gives one (section 3.4): an attribute binding's `refusals.empty` for `empty: "refuse"` on a set or on an add, its `refusals.absent` for an `absent` refusal, an insert's `refusals.when` for a `when` that does not hold, and its `refusals.container` for a container that is missing when the insert has no `create` (section 6.2). A host **MUST** show that sentence as the reason of the refusal, in the user's language as a LocalizedText gives it; without one, the wording is the host's.

*(FBL 0.2)* A declared binding plans the changes of section 11.2 as a plugin that declares `add.position` and `move` does. An `add` with a `position` is one `insert-entry` directly before the entry of the sibling at that index (or after the last sibling's entry when the index equals their number, and, with no sibling, where section 6.3 puts a new entry that has none), in place of the rule's `insert.place`; a rule whose `insert` cannot place it there refuses the add. A position `{after: id}` naming an entry of another type at the same level is one `insert-entry` on a line of its own right after that entry's line span, so that a statement can stand beside a header it is no sibling of. A `move` is the `remove-entry` and `insert-entry` of section 5.5, the inserted text being the entry's own bytes, re-indented in `bullets` (section 6.3), at its `position`. A `retype` is planned as a `remove` and an `add` of the element under its new type's rule, and the host warns as section 11.2 says; *(FBL 0.8)* the add writes the new rule's constants (section 6.2). No other change writes a constant: a `set`, a `move`, a rename and the reference rewrites of section 5.7 leave a constant's key and value as they are, and a `remove` removes them with the entry. *(FBL 0.7)* In `bullets`, when the new type's rule has an `emit`, a retype is one `re-emit-line` of the item's text from that `emit` with the retype's values: the item's continuation and nested items stay where they are, and the host does not warn.

### 6.5 Determinism

For the same body, binding and edit, a conforming host **MUST** plan the same splices and write the same bytes. Offsets are UTF-8 byte offsets; splices of one edit do not overlap; splices at the same offset apply in the order listed.

### 6.6 Saving

A host writes a body when the user saves, or after every edit if the host saves automatically; either way what is written is the body's bytes after its edits. A body **MUST** be written atomically (a temporary file in the same folder, then moved into place), keeping the file's permissions. A body that is unreadable (section 7.5) or read-only is never written.

---

## 7. History and reading problems

### 7.1 Undo and redo

Each open body has one **history** of edits, shared by every reading of it (section 9). An edit records its splices, the bytes each replaced, and the digest of the whole body after it. Undo applies the inverse splices of the most recent edit; redo applies the edit's splices again. A new edit clears the redo stack.

*(FBL 0.7)* A write to a declared block with `history: false` (section 8.1) is applied at once and is not a step of the history: undo and redo pass it by, and it leaves the redo stack as it is. It changes only the lines of its block, so an undo or a redo of an edit made before it is applied with its registration offsets moved by what such writes added or removed before them, and the comparison of section 7.2 leaves those blocks' lines out.

A rule or edit with `undo: "snapshot"` records the whole body before the edit instead; its undo replaces the whole body with that snapshot, as one splice of the whole range under the operation of the edit's first splice. Snapshots are for edits whose inverse would be many splices, such as a removal cascading through a large file; the result is the same bytes either way.

### 7.2 Drift

Before applying an undo or a redo, the host **MUST** compare the body on disk (or in the host's buffer, when another editor in the same host holds it) with the bytes the history expects: for an undo, the body after the edit; for a redo, the body before it. When they differ, the body has **drifted**: the host **MUST** refuse the undo or redo with a reason ("The file has changed since this edit, so it cannot be undone."), **MUST NOT** write, and **SHOULD** offer to reload.

*(FBL 0.7)* Before applying an edit, the host **MUST** compare the body on disk, or in the host's buffer as above, with the bytes the edit was planned against. When they differ, the host **MUST** refuse the edit with a reason ("The file has changed since this edit was made, so it was not written."), **MUST NOT** write, and **MUST** read the body again. Section 7.3 then applies: the history is cleared as for any external change.

### 7.3 External changes

A host watches every open body. When a body changes from outside and the host has no unsaved edits for it, the host reads it again, applies the difference to every open reading (elements matched by id keep their view data), and clears the body's history. When the host has unsaved edits, it tells the user and offers two choices: reload (the unsaved edits are lost) or keep the editor's version, which writes the whole of the editor's bytes as one edit that can itself be undone. A reload that finds the body unreadable keeps the last readable model on screen, read-only, with the finding; a deleted body closes its readings.

### 7.4 Tolerant reading

Reading **MUST NOT** fail on the content of a body. Every entry a rule matches but cannot read (a missing required slot, a value that does not convert to the attribute's type and has no `fallback` (section 5.2), a sequence item that is not a mapping) is an **unreadable entry**: it is reported as DISL's `std.unreadableEntry` at its source location, it produces no element (or an element with the readable attributes, when the binding's language says so through DISL's tolerant loading), and its bytes are kept untouched. Other entries are read as usual.

FBL's own finding codes:

| Code                        | Severity | When |
|-----------------------------|----------|------|
| `fbl.unbound-statement`     | warning  | A `lines`, `blocks` or *(FBL 0.7)* `bullets` statement no rule matches, when the binding's `unmatched` is `report`. |
| `fbl.dangling-reference`    | warning  | A reference or relation end names nothing (section 5.4). |
| `fbl.header-mismatch`       | warning  | The header is missing or has another value (section 5.6). |
| `fbl.duplicate-key`         | warning  | A json object or yaml mapping repeats a key; the first is bound. *(FBL 0.11)* Also two keys of one view of a legacy layout that match one id (section 8.7). |
| `fbl.unconverted-value`     | warning  | *(FBL 0.5)* A value does not convert to its attribute's type and the attribute is read as its `fallback` (section 5.2), unless the fallback's `report` is `false`. Its location is the value's span, and its detail carries `attribute` and the written `text`. |
| `fbl.missing-body`          | error    | A registration's body does not exist (section 8.2). |
| `fbl.missing-resource`      | warning  | *(FBL 0.10)* A registration's `resource` header names a resource the body does not hold (section 8.2). Its location is the header's line in the registration. |
| `fbl.stale-view-data`       | info     | A registration stores view data for an id the body no longer has (section 8.5). |
| `fbl.unknown-header`        | info     | A registration has a header neither FBL nor the binding declares (section 8.1). |
| `fbl.unknown-block`         | info     | *(FBL 0.7)* A registration has a block neither FBL nor the binding declares (section 8.1). It is kept as written. |
| `fbl.unclosed-block`        | warning  | *(FBL 0.11)* A block of a `grammar` body read with `recover.nested` is still open at the end of the body (section 4.9). Its location is the line and column of its `open` token. |
| `fbl.unmatched-close`       | warning  | *(FBL 0.11)* A `close` token closes no block; it is unbound (section 4.9). Its location is the token. |
| `fbl.unclosed-comment`      | warning  | *(FBL 0.11)* A token of a terminal named in `recover.unclosed` does not end with its text; it runs to the end of the body (section 4.9). Its location is its first line. |
| `fbl.unparsed-text`         | warning  | *(FBL 0.9)* A `grammar` body with `recover` holds text that does not parse as a unit: from the unit's first byte to the end of the synchronising token after the place reading stopped, or to the end of the body (section 4.9). The text is kept and binds nothing. Its location is the line and column where reading stopped, and its length that of the skipped text from there. |

Every finding carries a DISL source location: `file` relative to the subject with `/` separators (for a file body, the body's own name), and the line, column and length of the entry's own span. A finding about something that is not drawn uses DISL's `subject`.

### 7.5 The unreadable body

A body is **unreadable** when it is not valid UTF-8, when it is not well-formed in its family (a YAML or JSON syntax error, XML that is not well-formed, unbalanced braces), when a `required` header is missing, or when a plugin reports it unreadable. *(FBL 0.7)* A `bullets` body is never unreadable for its family (section 4.8): only invalid UTF-8 and a missing `required` header make it unreadable. *(FBL 0.9)* A `grammar` body is not well-formed when its grammar does not read it, or text in it is matched by no terminal the parser's state can shift; with `recover` it is never unreadable for its family, and is read in part (section 4.9). An unreadable body opens as an **empty, read-only** model with one finding, DISL's `std.unparseable`, located at the problem, which replaces all other findings. It is never written. For a file body the body is the **primary file**, and `std.unparseable` stops rule evaluation (DISL §8.6); a folder subject has no primary file (section 10.3).

---

## 8. The registration

### 8.1 The line form

A **registration** is a small text file, `*.adp`, that names what a document is and keeps the view data a user placed. It is UTF-8 lines:

```text
databricks/job
body: resources/nightly_ingest.yml
resource: nightly_ingest
layout:
  task:ingest: 0 0
  task:quality_gate: 260 0
```

1. **Line 1** is the **origin** of the tool type (`language.origin`, DISL §3.2), after a byte-order mark if there is one. A host finds the tool type's specification with that origin, and through its persistence the binding: the one a DISL specification's `persistence.binding` names, or, of the bindings a DESL specification's `persistence.bindings` names, the one whose `claims` take the body (section 12.1). A body two of them claim is refused with a reason, and a validator reports a specification whose bindings claim one extension twice.
2. **Headers** follow, one per line, `key: value` (the first `: ` separates them; blank lines are skipped). FBL defines `body`, `view` and `resource`; a binding declares others in `registration.headers` (the SKOS reading's `language`). A header neither defines is kept and reported as `fbl.unknown-header`. The header region ends at the first line that is not a header.
3. **Blocks** follow the headers: `layout:` (section 8.3), `identities:` (section 8.6) and *(FBL 0.7)* the blocks the binding declares, each a line holding only the block's name and colon, followed by its entries, each indented by two spaces (any whitespace is read), `<key>: <value>`, where the key is everything before the **last** `: ` of the line, so keys may contain `: `. A block's name is a Name (section 2.2).

*(FBL 0.7)* A binding declares further blocks in `registration.blocks`, as it declares further headers in `registration.headers`, each by its name, with these properties:

| Property  | Values | Meaning |
|-----------|--------|---------|
| `order`   | `"ordinal"` (default), `"kept"` | `ordinal`: entries are written in the ordinal order of their keys, as `layout:` is. `kept`: an entry stands where the edit put it (section 8.4). |
| `keys`    | `"names"` (default), `"ids"` | `ids`: an entry's key is an element's id, and the block is pruned and keeps no ephemeral id as section 8.5 says of layout. |
| `history` | `true` (default), `false` | `false`: writes to the block are not steps of the history (section 7.1). |
| `doc`     | Doc | What the block holds. |

A value of a declared block is one line of text, not empty, and holds no `: `, since the key is everything before the last one. What a value means is the tool type's specification's; FBL reads and writes it as text. A block neither FBL nor the binding declares is kept as it is written, and reported as `fbl.unknown-block`.

Anything after the blocks that is neither a block nor an entry is kept as unbound content. `$defs/Registration` in `fbl.schema.json` describes the parsed form; the examples in `registrations/` validate against it.

### 8.2 Finding the body

`body` is a path relative to the registration's folder, `/`-separated. Without it, the body is the sibling file with the registration's base name and one of the binding's extensions (the first that exists, in the order of `claims.extensions`), or, for a folder binding, the folder the registration is in. A body that does not exist opens the document empty with `fbl.missing-body`, and nothing is written until the user points the registration at a file. A host **MUST NOT** follow a `body` outside the workspace without the user's consent (section 16).

`view` names which view of the body the document shows (section 9.3). `resource` selects one resource of a body that holds several: its value binds the selector capture the binding's `registration.resource` names for the reading (a Databricks bundle file's job key), so that every selector segment of that capture matches only that key: `capture`, the same capture for every reading, or *(FBL 0.10)* the capture `byOrigin` gives the reading's origin, so that one binding selects a job for the job reading and a pipeline for the pipeline reading. Without the header, the first such entry in document order is used, *(FBL 0.10)* for each origin by its own capture. A reading whose origin `byOrigin` maps to `null`, or does not list, selects no resource and sees every one, as without `resource`; a `resource` header in its registration is kept and selects nothing. *(FBL 0.10)* A header naming a resource the body does not hold opens the reading empty, with the finding `fbl.missing-resource` (section 7.4): the body is not unreadable, and its other readings are read as before.

### 8.3 The layout block

`layout:` holds the positions the user placed, and only those: one entry per placed element, `<id>: <x> <y>`, the top-left corner of the element in the reading's canvas units. An element without an entry is placed by layout. Entries are:

- written in the ordinal order of their ids (byte order of the UTF-8 encoding);
- with numbers in `{decimals: 3}` form (section 6.3), so `120.5` and `640`;
- with the registration's own line ending by the rules of section 6.3.

Placing an element for the first time inserts its entry (`insert-entry`), creating `layout:` after the headers first when it is missing (`ensure-container`); moving it replaces its numbers (`replace-value`); undoing the first placement removes the entry, and the `layout:` line with the last entry (`remove-container`). Other view data (sizes, waypoints, label offsets) is not stored in FBL 0.1. *(FBL 0.7)* What else a tool type keeps beside the body, in a block of its own, a binding declares in `registration.blocks` (section 8.1).

### 8.4 Writing the registration

A registration is written with the splice rules of section 6, so a user's hand edits, comments below the blocks and unknown headers survive. View changes write only the registration; model changes write only the body, except where one edit concerns both (a rename of a stored id moves its layout entry in the same edit, with splices in both files, undone together). A host creates a registration beside a body the first time the user places an element, when the binding's `registration.createOnFirstPlacement` is true, and refuses the placement with a reason otherwise; a registration created this way is `<origin>` then `body: <name>` when the body's base name differs from the registration's, with the body's dominant line ending. *(FBL 0.7)* `createOnFirstPlacement` also creates it the first time the user changes a setting that a declared block keeps. Creating the file is not part of the edit: it is written with that text first, outside the history, and the edit is planned against it, so an undo leaves the file in place.

*(FBL 0.7)* **Declared blocks** are written by the same splices. A change to a setting that a declared block keeps writes the registration, as part of the edit of the transaction that makes it, so a change that concerns the body too is one edit with splices in both files, undone together.

- **The order of blocks.** `layout:`, `identities:`, then the declared blocks in the order the binding declares them. A missing block is created (`ensure-container`) right after the last line of the nearest block before it in that order that exists, or after the headers when none does, and its line goes with its last entry (`remove-container`). An undeclared block stays where it is.
- **Entries.** An entry is written `<key>: <value>`, indented by two spaces, with the registration's line ending. A new entry (`insert-entry`) goes where the ordinal order of the keys puts it in an `ordinal` block; in a `kept` block it goes at the position the change gives, else after the last entry. The place is found among the entries that remain once stale entries and, for a move, the moved entry are taken out. A changed value is a `replace-value` of the value, a renamed key a `replace-value` of the key, and a removed entry a `remove-entry` of its line. A move in a `kept` block is a `remove-entry` and an `insert-entry` of the entry's own bytes.
- **Refusals.** An edit that would write an empty value, or a value that holds `: ` or a line break, is refused (section 6.4), and so is a key that is empty, holds a line break, or is the key of another entry of the block.

### 8.5 Stale and ephemeral ids

A layout entry whose id is not in the model is **stale**: it is not applied to any other element, it is reported as `fbl.stale-view-data`, and it is removed at the next write of the registration, as part of that edit. A host **MUST NOT** store a position for an element whose id DISL marks ephemeral; dragging such an element moves it for the current session only, and the host tells the user why the position will not be kept. A layout entry found for an ephemeral id is ignored and reported by DISL's `std.ephemeralViewData` (DISL §11.5.3). *(FBL 0.7)* All of this holds for a declared block with `keys: "ids"` as for `layout:`: an entry whose key is no id of the model is stale, reported as `fbl.stale-view-data` and removed at the next write of the registration, and an ephemeral id is never stored as a key. The removal of a stale entry from a block with `history: false` is no step of the history either: the undo of the edit it was written with does not put the entry back.

### 8.6 Identities

`identities:` holds ids for bindings whose rules use `id.sidecar` (section 5.3): one entry per element, `<natural key>: <id>`, where the natural key is the value of the rule's `sidecar.key` expression and the id is made by DISL's strategy the first time the element is read without one. *(FBL 0.12)* When the rule declares `sidecar.kind`, its entries' keys are `<kind>:<natural key>`, so that the elements of two rules with one natural key, a component and a note of one text, keep entries of their own; rules that declare one kind, a Wardley map's components, anchors and submaps, share its key space. Entries are written, ordered and pruned like layout entries.

Two elements of one kind (or, without `kind`, of the binding) with the same natural key: without `ordinal`, the second is reported as DISL's `std.duplicateId` and gets an id that is not stored. *(FBL 0.12)* With `ordinal`, a text holding `{n}` once, the holders are counted per kind in document order, and each later holder's natural key is followed by `ordinal` with `{n}` replaced by its ordinal in decimal, 2 for the second: the Wardley map's `"\u001f{n}\t"` keys the second `Tea` as `Tea`, U+001F, `2` and a tab, the form its tool writes. Each holder so has an entry and an id of its own, read again for it when the body is opened again; a later holder's entry is first stored with the next write of the body.

*(FBL 0.12)* When an attribute that a natural key is computed from changes, by a set or by a reference rewritten in the same edit (section 5.7), the element's entry moves to its new key in the same edit, by a `replace-value` of its key, and an undo moves it back. An id first stored for an element, one read without an entry, is written with the next write of the body and is no step of the history, as a `history: false` block's writes are not (section 7.1); an entry whose key no longer names an element is pruned by the write that finds it so, as section 8.5 prunes a layout entry.

### 8.7 Legacy sidecars

Two sidecar files that hosts wrote before FBL are read, so existing documents keep their view data:

- `registration.legacyLayout`, such as `{base}.layout.json` for the C4 types: a JSON object keyed by view key, then by element id, each `{"x": …, "y": …}`. Positions of the registration's view are read from it when the registration has no `layout:` block, and written back to it, by json splices (section 6), as long as it exists. *(FBL 0.11)* View keys and element keys are matched with a view's key and an element's id ignoring the case of ASCII letters. When two keys of one view match one id, the first in the file applies; the others are neither applied nor written, and each is reported as `fbl.duplicate-key`. A number is written as section 6.3 writes it with `number: "shortest"`. A move is a `replace-value` of `x` and of `y`; a first placement an `insert-entry` of the element's member, laid out as the member before it in its view is (on one line, or one key per line), and an `ensure-container` of the view's object, at the end of the root object, when the file has none for the view; undoing it removes them. An entry of the view being written whose key names no element of the body is **stale**: it is reported as `fbl.stale-view-data` and removed at that write, in the same edit, as section 8.5 removes a layout entry; entries of the other views of the file are not touched.
- `registration.legacyIdentities`, such as `{base}.identities.json` for the Wardley map: *(FBL 0.12)* a JSON array of entries `{"id": …, "kind": …, "key": …}`, in which `kind` is a rule's `sidecar.kind` and `key` its natural key as section 8.6 makes it, `ordinal` included. An entry is matched to the element of that kind with that key. The file is read when the registration has no `identities:` block, and written back to it while it exists, by json splices (section 6): a new entry is an `insert-entry` after the array's last item, laid out as that item is; a moved key a `replace-value` of `key`; a pruned entry a `remove-entry` with one separator. An entry whose `kind` no rule of the binding declares is kept as written, never pruned, and reported as `fbl.stale-view-data`. FBL 0.9 described the file as a JSON object from natural key to id; no tool wrote that form, and it is not read.

A host **MUST NOT** create either file for a new document.

---

## 9. Several readings of one body

### 9.1 One open body

A **reading** is one tool type's view of a body. A host keeps one **open body** per pair of canonical body path (absolute, with the platform's case rules) and binding: its bytes, its lossless reading, its findings, its history, and *(FBL 0.10)* one model for each origin it is read for, which the binding's rules give for that origin (section 5.1). Every reading of the body is a view of that open body: an edit made in any of them is planned once against the body's bytes, applied once, recorded in the one history, and shown in every other open reading, of its own origin and of every other, without a reload. Undo in any reading undoes the most recent edit of the body, whichever reading made it, and redo likewise. *(FBL 0.10)* A registration's resource selection (section 8.2) is a view of its origin's model, so two registrations of one origin that select different resources show and edit their own while sharing the body's history.

### 9.2 One binding per body

Every reading of a body **MUST** use the same binding: the six C4 types name `structurizr.fbl#workspace`, the four W3C types `w3c-turtle.fbl#turtle`. A tool type whose specification names a different binding for a body that is already open **MUST** be opened read-only, with a reason, until the other readings close. *(FBL 0.10)* Readings of different origins through one binding are all editable: the bundle, job and pipeline readings of a Databricks bundle file, through `databricks.fbl#configuration`, each change the one open body. A validator **SHOULD** warn when two specifications claim the same extension with different bindings and no marker tells them apart.

### 9.3 Per-reading view data

Each reading keeps its own view data in its own registration (one `.adp` per reading), so positions of a container view and of a context view of the same workspace never mix, and none is written into the body. The registration's `view` header selects which view of the body the reading shows, for bindings whose `blocks` define views (sections 4.7 and, *(FBL 0.11)*, 4.9), matched ignoring the case of ASCII letters; without it, the first view in document order of the block rules whose `origins` include the reading's origin, or of every view block when none has `origins`. Which elements a view draws is the reading's DISL viewpoint's, not FBL's. What FBL reads of the selected view is what the rules with `selectedView` read inside it (section 4.9). *(FBL 0.11)* A registration whose `view` names no view of the body opens a reading that shows no view's content; no finding is reported for it; a change of view data (a placement) is refused with the reason "This view is not in the document, so its positions cannot be stored." and nothing is written. A `view` header over a binding that defines no views is kept, and the reading shows the body, as before.

### 9.4 Offering readings

When a host is asked which readings it offers for a body, it lists every installed tool type whose binding claims the body (section 12), and, for a binding with `claims.readings`, each reading in the order of `claims.origins`, those whose `suggest` matches first.

---

## 10. Folder subjects

### 10.1 Recognition

A body with `kind: "folder"` is a **folder subject**: the model is read from the files of a folder. `recognise` says which folders qualify: every glob in `all` matches at least one entry of the folder, at least one glob in `any` does (when `any` is given), and no glob in `none` does. Globs are relative to the folder, `/`-separated: `*` matches within one path segment, `**` any number of segments, `?` one character, `[…]` a character class. Matching is case-sensitive on case-sensitive file systems and case-insensitive on others.

### 10.2 Files

`files` lists **file rules**: a `name`, a `glob`, and the `family` the file is read in. A declared folder binding's rules name the file rules they read with `files`, and see each file's entries as if the file were a body of its own (selectors are absolute within the file). A folder with a plugin reader hands the selected files to the plugin (section 11). `ignore` globs are never read. A symbolic link or other reparse point **MUST NOT** be followed.

### 10.3 Reading, watching and diffing

A folder subject is read file by file, in the ordinal order of the files' relative paths. A file that cannot be read is reported as DISL's `std.unparseable` located at that file, and the rest of the folder is read: a folder subject has no primary file, so no single file stops rule evaluation. A folder that no longer satisfies `recognise`, or no longer exists, is an unreadable body.

While a folder subject is open, the host watches every file it read and the folders that `files` globs could add files to. After a change, the host waits until no further change has arrived for `settle` milliseconds (default 400), then reads the subject again once, however many files changed, and applies the difference to every open reading: elements whose ids survived keep their view data, new ones are placed by layout, removed ones disappear. Hosts honour `settle` as closely as their platform allows. A host **MAY** stop watching a folder too large to watch reasonably, and **MUST** then say so and read again on request.

### 10.4 Writing

A folder subject is read-only unless its binding declares writes for a file rule's files (rules with `insert` or writable attributes reading that file rule). Then each file is a body of its own for writing: splices, history, drift and atomic saves apply per file, and one edit may splice several files, undone together. The registration of a folder subject sits in the folder (or beside it, naming it with `body`).

---

## 11. Persistence plugins

### 11.1 Where a plugin sits

Some formats are too intricate to declare: projecting Turtle triples onto cards and rows, evaluating MSBuild with its imports and wildcards, parsing SPARQL. For them a binding's `reader` is a **persistence plugin**:

```json
{ "reader": { "plugin": "net.etalii.adp.w3c.turtle", "version": "^0.1.0" } }
```

The plugin is declared in the DISL specification's `plugins` with `provides: ["persistenceFormat"]` (DISL §13.1); `args` are validated against its declaration. Everything else in the binding is declared and done by the host exactly as for a declared reader: claims and routing, the registration, several readings, folder recognition and watching, templates, history, drift and saving. A format FBL can declare **SHOULD** be bound rather than read by a plugin; *(FBL 0.9)* a format with a context-free syntax can be declared with the `grammar` family (section 4.9).

### 11.2 Operations

A plugin implements these operations, stated as data exchanged so that each host binds them to its own plugin mechanism:

| Operation  | Receives | Delivers |
|------------|----------|----------|
| `read`     | The body's bytes (a file body), or for a folder subject the relative path and bytes of every file the file rules select; the binding's `args`. | The elements and relations: type, id, parent and slot, attribute values; for each, the source span of the entry and of every writable value (file, byte range); the findings, with source locations; and whether the body is unreadable as a whole. |
| `plan`     | The current bytes, the last `read` result, and one model change (below): add, set, remove, move or retype an element or relation. | The splices that realise it, each `{operation, file, start, end, text}` with an operation from section 6.1; or a refusal with the sentence the host shows. |
| `template` | The new body's name and the template placeholders (section 13). | The bytes of a new body. Only asked when the binding has no `template.text`. |
| `watch`    | The last `read` result (folder subjects, optional). | The paths the reading depends on beyond the file rules; the host watches them too. |

**Model changes** *(FBL 0.2)*. The change `plan` receives is one of these, each an object with the change's name as its one key (`$defs/ModelChange`):

| Change   | Fields |
|----------|--------|
| `add`    | `type`, `id` (optional), `attributes`, `parent` (optional), `slot` (optional), `position` (optional) |
| `set`    | `id`, `attributes` |
| `remove` | `id` |
| `move`   | `id`, `parent` (`null` for the top level), `slot` (optional), `position` |
| `retype` | `id`, `type`, `attributes` (the values after the type's `attributeMapping`, DISL §9.5) |

where `position` (`$defs/ChangePosition`) is `{ "index": int }`, `{ "after": id }` or `{ "before": id }`. A plugin declares which changes it plans in its DISL declaration (DISL §13.1): `"plans": ["add", "set", "remove", "move", "retype"]`, with `"add": { "position": true }` when it honours a position, and `"move": { "position": true }` likewise. Without `plans`, a plugin plans `add`, `set` and `remove`.

- A host **MUST** send an add with `position` when the change came from an ordered placement (DISL `create` with `after` or `before`, a parent placement, a drop), and a plugin that declares `add.position` **MUST** insert at that position or refuse.
- A host **MUST** send a reorder or a re-parent as one `move`, never as a remove and an add, so the plugin can splice the element's bytes unchanged; a plugin that does not declare `move` gets the remove and the add instead.
- A host **MUST** send a DISL retype as one `retype`; a plugin that does not declare `retype` **MUST** be sent a remove and an add, and the host **MUST** warn that identity and unbound bytes may not survive.
- An index **MUST** be counted among the parent's children in the slot after the element was removed from its old place, so the same index means the same thing for a move within one parent and across parents.

```json
{ "move":   { "id": "1.2", "parent": "1", "position": { "index": 2 } } }
{ "retype": { "id": "1.3", "type": "Check", "attributes": {} } }
{ "add":    { "type": "Check", "attributes": { "label": "New question" }, "parent": "1", "position": { "index": 1 } } }
```

An agent behavior model's Markdown plugin declares all five changes and positions. Move later on `1.2`, the second of three children of `1`, sends the move above; changing `1.3` from Do to Check sends the retype; a drop under `1` between its first and second child sends the add. Each becomes line splices that keep every other byte.

### 11.3 What the host does

The host, not the plugin, applies splices to bytes and saves them atomically; keeps the history and undoes with inverse splices or snapshots; checks drift; watches the body and calls `read` again after external changes; stores view data in the registration; routes files; and shares one open body between readings. A plugin-backed body therefore behaves exactly as a declared one: byte-preserving writes, exact undo, drift refusal, several readings, folder subjects.

### 11.4 What the plugin must do

- `read` **MUST NOT** fail on content: problems are findings, and an unreadable body is reported as such (section 7).
- `read` and `plan` **MUST** be deterministic: the same bytes and change give the same result, in every host's implementation of the plugin.
- Splices from `plan` **MUST** change only the bytes the change concerns, **MUST** name their operation truthfully, and **MUST** follow the new-text rules of section 6.3 where they apply to the format.
- A plugin **MUST NOT** write files, keep its own undo history, or store view data. A read-only binding's plugin is never asked to `plan`.
- A plugin **MAY** report findings about anything in the body, with DISL's source location and `subject`; the host shows them like any other finding.

### 11.5 The formats expected to stay plugins *(informative)*

| Format | Tool types | Why a plugin |
|---|---|---|
| Turtle and N-Triples | W3C RDF, OWL, SHACL, SKOS | Terms span tokens, prefixes compress IRIs, and one IRI yields several elements; the projection is code. *(FBL 0.9)* Turtle's syntax is read by the `grammar` family in `w3c-turtle-grammar.fbl`, and spec 035 binds the W3C RDF type's Turtle with it. |
| SPARQL 1.1 | SPARQL query | A recursive grammar projected onto scopes; read-only. |
| `.sln`, `.slnx`, MSBuild projects | .NET dependency graph | Wildcards, imports and central package management are evaluated; read-only. |
| Ansible project folders | Ansible structure | YAML and INI inventories, task directives and role resolution; read-only. |
| Helm chart folders | Helm chart | Templates are scanned as lines, not YAML, and includes resolved; read-only. |
| Azure Pipelines template expansion | Azure DevOps pipeline | Following `template:` references and unwrapping `${{ }}` expressions is evaluation; the rest of the file could be declared. |

---
## 12. Routing

### 12.1 Claims

A binding's `claims` say which files it takes:

| Property           | Type                   | Description |
|--------------------|------------------------|-------------|
| `extensions`       | string[]               | Lowercase, with the dot (`.tml`, `.ttl`). Matched ignoring case. |
| `names`            | glob[]                 | File names claimed whatever their extension (`databricks.yml`, `Chart.yaml`). |
| `shared`           | bool                   | The extension belongs to many formats (`.yml`, `.json`). A shared binding **MUST** have a `marker` or be `registrationOnly`. |
| `marker`           | Marker                 | The opt-in mark a file needs before this binding claims it (section 12.2). |
| `registrationOnly` | bool                   | The binding never claims a bare file; the file opens only through a registration. |
| `suggest`          | `{contains}`           | Substrings of a body's first 64 KiB that make a host propose this binding (section 12.3). |
| `origins`          | string[]               | The origins of the tool types that share this binding; each is the `language.origin` of a specification whose persistence names it. |
| `readings`         | map origin → `{bare, suggest}` | For a shared binding: which reading a bare file opens as (`bare`, at most one) and what suggests each reading. |

### 12.2 Markers

A marker is one of: `{rootKey, value?}`, a root key a yaml or json body must have (with that value, if given); `{firstLine}`, a prefix the first line must start with, after a byte-order mark; `{pattern, lines?}`, an expression one of the first `lines` lines (default 20) must match, line 1 after a byte-order mark. A host evaluates a marker on the body's bytes without reading it through the binding.

### 12.3 Routing a file

When the user opens a file that is not a registration, the host routes it:

1. The **candidates** are the bindings of installed tool types whose `names` match the file name, or whose `extensions` include its extension; a `registrationOnly` binding is not a candidate, and a `shared` one is a candidate only if its marker matches.
2. With one candidate, the file opens as it: as its `bare` reading when the binding has `readings`, or as its only tool type.
3. With several, the host offers them all and **MUST NOT** choose silently; with none, the file is not a diagram.

When the user opens a registration, line 1 decides the tool type (section 8.1). When the user asks to add a diagram for a file (a host's "Add" or "Open as"), the host **MAY** propose every binding whose `suggest` matches the file's content, first those whose reading's `suggest` matches; accepting one creates a registration. A host **MAY** also offer to add a binding's marker to a file that lacks it; it adds the marker only when the user agrees, as a splice (an `insert-key` of the root key after the last root key, or an inserted first line), which is one edit that can be undone.

### 12.4 Replacing `x-adp` routing keys *(informative)*

The routing keys today's definitions keep in `x-adp` map to claims: `claimsBareFiles` is `readings.<origin>.bare`; `sharedExtension` and `x-adp-shared-extension` are `shared`; `suggestWhenBodyContains` is `suggest.contains`; `documentExtensions` is `extensions`; `subject: folder` is `body.kind: "folder"`; `registrationHeaders` is `registration.headers`; `x-adp-origin` is DISL's `language.origin`.

---

## 13. Templates

A binding's `template` is the exact text of a new body. When the user creates a new document of a tool type:

1. The host chooses the text: `template.byOrigin[<origin>]` when present for the tool type, else `template.text`; with a plugin reader and no template, the plugin's `template` operation.
2. It replaces the placeholders, and nothing else: `{name}` (the new file's name), `{base}` (its base name), `{key}` (the base name with every character outside ASCII letters, digits and `_` replaced by `_`, leading and trailing `_` removed, `_` prefixed when it then starts with a digit, and `untitled` when nothing is left), *(FBL 0.10)* `{key:lower}` (`{key}` with every ASCII letter in lower case, so `My_Job.yml` gives `my_job`), and `{newid:<rule>}` (a new id made by DISL's strategy for that rule's type). `{` and `}` not forming a placeholder are literal.
3. It writes the file, and a registration when the binding is `registrationOnly` or the document needs one; it **MUST NOT** overwrite an existing file.

A template **MUST** read through its binding with no finding of severity warning or above; a validator that can read bindings checks it. The user's first edits are splices into the template's bytes like any others.

---

## 14. Processing model

### 14.1 Loading an FBL document

1. **Parse** the JSON; reject duplicate keys.
2. **Check the version**: `fbl` major version supported.
3. **Validate** against `fbl.schema.json#/$defs/Document`.
4. **Resolve names**: rule names unique within each binding; every rule named in `parent`, `cascade`, `reference.to`, `within` and `files` exists; every reference's `by` is an attribute every rule in `to` binds, or `id` when every rule in `to` stores its id with `id.from`; every `type` names a type of the specification that uses the binding (checked when the tool type's specification is loaded with it, DISL §14.1).
5. **Compile** every regular expression in the common subset (section 2.5) and every CEL expression in its context (section 2.4).
6. **Check** the rules: every writable attribute has exactly one slot (section 3.3); no `fixed` DISL attribute is bound; a `shared` claim has a marker or is `registrationOnly`; at most one reading is `bare`; a declared binding has a family and rules; a plugin reader's plugin is declared by the specification; *(FBL 0.8)* an insert's `constant` stands only in a binding whose body's family is `yaml` or `json`, and never beside `emit`, and none of its keys is the key of a `{key}` slot without `child` that the same rule writes: a writable attribute binding (one that is not `readOnly`, whose rule is not `readOnly`, and that is not a `value` or `capture` slot), `id.from`, or a relation rule's `source` or `target`. A read-only attribute binding **MAY** read a constant's key. *(FBL 0.10)* Every origin a rule's `origins` lists, and every origin `registration.resource.byOrigin` names, is one of the binding's `claims.origins`; and no attribute binding has both `secondary` and `override`. *(FBL 0.9)* A body of the `grammar` family has `grammar` and no other family has it; the grammar, its file and the files it imports are read from beside the document, are in ATD grammar notation (ATD section 4.1) with terminals in its regular expression subset (ATD section 3.5), and build from `start` as an LALR(1) automaton with no shift/reduce or reduce/reduce conflict and no two terminals a state accepts matching the same text, and a binding whose grammar does not is refused with a problem that names the conflict: its kind, the terminal and the rules; `recover.unit` is a rule of the grammar and the start rule's one expansion is `unit*`, and every entry of `recover.sync` names a terminal of the grammar or is the text of one of its anonymous string terminals; every segment of a rule's `at` and of a `{node}` slot names a rule, an alias or a terminal of the grammar, or is `*`, `**` or, in a slot, `.`; a rule of the family has `at` and no `line`; and no slot of the family is `{key}`, `{attribute}`, `{text}`, `{group}`, `{continuation}` or `{capture}`, nor a `{node}` slot of another family. *(FBL 0.11)* Every terminal `recover.nested` and the keys of `recover.unclosed` name is a terminal of the grammar or the text of one of its anonymous string terminals, and each terminal of `unclosed` an ignored one; `escape`, `create` with `after`, `gather`, `opens` as an object, `selectedView` and a block rule's `at` and `insert` stand only in a binding of the `grammar` family, where a block rule has `at` and no `line`; every name in `create.after` is another attribute of the same rule; `gather` stands only on a read-only attribute binding; and `selectedView` only in a binding with a block rule that has `view`. *(FBL 0.12)* `items` and `separator` stand only on a `{node}` slot of a binding of the `grammar` family, and `items` only on an attribute that is `many` in the tool type's specification, the attribute a `typeMap` maps it to included (checked when the tool type's specification is loaded with the binding, as step 4's types are); `sidecar.ordinal` holds `{n}` once. *(FBL 0.14)* A rule of the `yaml` or `json` family whose `at` ends in a `{c}` segment and that has an `insert` binds an attribute to `{capture: c}`, and an insert's `refusals.key` and `refusals.duplicate` stand only on such a rule. *(FBL 0.13)* `split` stands only on a `{node}` slot of a binding of the `grammar` family, never beside `items`, and only on an attribute that is `many` in the tool type's specification, checked as `items` is; and it holds neither character of the slot's `escape`.

*(FBL 0.16)* A validator also checks that an insert's `continues` and a remove's `parts` and `whole` stand only in a binding of the `grammar` family, that every key of `continues.same` is an attribute the rule binds, and that `whole` stands only beside `parts`.

*(FBL 0.15)* A validator also checks that a slot's `ordinal` names one of the body's `ordinals`, that a binding with a slot with `expand` has `body.prefixes`, and that `expand`, `ordinal` and `name` stand on a `{node}` slot and at most one of them on any one slot.

A validator reports every problem with the JSON Pointer of its location, a severity and a message, as DISL §14.1 does.

### 14.2 Opening a document

1. **Route** the file (section 12.3), or read the registration (section 8.1) and find the body (section 8.2).
2. **Open the body**: reuse the open body for this path and binding if there is one (section 9.1); otherwise read it through the binding's reader (sections 4, 5, 11), giving the model, the source spans and the findings.
3. **Build** the reading: DISL loads the model as it loads a definition (DISL §14.2), derives what it derives, evaluates constraints.
4. **Apply view data** from the registration's layout (section 8.3), reporting stale entries.

### 14.3 Editing

Each committed DISL transaction is planned as one edit (section 6.4), applied to the open body, recorded in its history, and shown in every reading. A view change (a placement) is an edit of the registration.

### 14.4 Saving and closing

Saving writes each changed file atomically (section 6.6). Closing the last reading of an open body releases it; its history goes with it.

---

## 15. Conformance

### 15.1 Conformance classes

| Class | Requirements |
|---|---|
| **FBL document** | Validates against `$defs/Document` and passes the checks of section 14.1 without errors. |
| **Validator** | Implements section 14.1, *(FBL 0.9)* building every grammar a binding carries, and reports problems with JSON Pointers; checks registrations against `$defs/Registration` and fixtures against `$defs/Fixture` and section 15.3. |
| **Host — declared** | Implements the lossless readings of the families it supports (section 4), rules (section 5), the splice catalogue and new-text rules (section 6), history and drift (section 7), the registration (section 8), several readings (section 9), routing and templates (sections 12, 13), and passes every fixture of the families it supports. It states which families it supports. |
| **Host — full** | Host — declared for the six families of FBL 0.8, plus folder subjects (section 10) and the plugin contract (section 11). *(FBL 0.9)* No host is required to support the `grammar` family; a host that does says so and passes its fixtures byte for byte, every host producing the same splices (section 6.5). *(FBL 0.11)* Except that a host which reads or writes Structurizr DSL workspaces or `.owm` maps through their declared bindings (the six C4 tool types and the Wardley map) **MUST** support the `grammar` family with every construct of this version, say so, and pass its fixtures byte for byte with the same splices as every other host. |
| **Persistence plugin** | Meets section 11.4 for its format. |

A host that does not support a binding's family or plugin **MUST** open its documents read-only, if it can read them at all, and say why; this is DISL's rule for a missing required plugin (DISL §13.1). *(FBL 0.17)* A host that supports the `grammar` family at an earlier version than a construct of the family that a binding uses, as a host at 0.9 does a binding that uses a construct of 0.11 or later, **MAY** open, read and edit that binding's documents, reading the binding as section 2.7 says. It then **MUST** report each construct it does not read as a warning that names the construct, and **MUST NOT** write a splice that touches text such a construct governs: the text of the entries and nodes that the rule, slot or property carrying the construct reads, and the place where a write the construct shapes would go. That text stays byte for byte, and an edit the host could plan only by touching it is refused, with the construct named as the reason. A host that cannot honour that, as where it cannot tell which text such a construct governs, **MUST** treat the binding as one whose family it does not support, and so open its documents read-only as the paragraph above says, which is what *(FBL 0.11)* FBL 0.11 to 0.16 required of every such host. This leaves unchanged what the class **Host — full** requires of a host that reads or writes Structurizr DSL workspaces or `.owm` maps: it **MUST** support the `grammar` family with every construct of this version.

### 15.2 Graceful degradation

A host that cannot watch files reads bodies on open and on request only, and says so. A host that cannot keep the registration (a read-only workspace) keeps positions for the session only, and says so. Nothing else in this document degrades: byte preservation, exact undo and drift refusal are required of every host.

### 15.3 Round-trip fixtures

`fixtures/<name>/fixture.json`, validated by `$defs/Fixture`, is a conformance test for hosts:

| Property  | Description |
|-----------|-------------|
| `binding` | The binding under test, relative to the fixture. |
| `input`   | The input body, a file beside the fixture, compared byte for byte (the repository keeps these files from line-ending conversion). |
| `template` | *(FBL 0.10)* `{name, origin}`: the input is the new document the binding's template gives for a file of that name and the reading of that origin (section 13), its placeholders replaced. The validator writes the template so and compares it with the input byte for byte; `read` and `steps` then apply to it as to any input, as the reading of that origin. A template holding `{newid:<rule>}` cannot be written so, and is not stated this way. |
| `read`    | What reading the input yields: *(FBL 0.7)* `document`, the attributes of the document as `attributes` below, `elements` (`id`, `type`, *(FBL 0.7)* optionally `parent`, the id of the element it is contained in or null at the top level, and *(FBL 0.5)* optionally `attributes`, the values read for the attributes it names, a JSON string for a text and a JSON number, bool or null otherwise), `findings` (`rule`, `line`), `unreadable`. |
| `steps`   | In order: an `edit` (add, set, remove, *(FBL 0.3)* move, place, *(FBL 0.7)* setting, or save without change; an add or a move carries a `position` as section 11.2 gives it, and *(FBL 0.5)* an add the `slot` it fills, as section 11.2's add does, which a host needs when one type fills several slots of one parent), an `undo` or a `redo`; the `splices` it must produce, as `{operation, start, end, text}` with UTF-8 byte offsets into the document before the step; the document after it (`expect`, or `expectFile`); *(FBL 0.10)* `expectFiles`, the bytes after it of each file beside the input that its splices name in their `file`, by that name, relative to the input's folder (the body a registration names, or a sidecar), a splice without `file` being the input's; `refused` with the reason when the edit must be refused and write nothing. |

*(FBL 0.7)* `elements` lists every element the body's entries yield, in reading order, and, when the input is a registration, the elements the tool type's specification makes of its blocks as well (DESL sections 5.5, 7.7.2). An attribute with `many` is stated as a JSON array of its items in order, and an enumeration's value by its key. A step's `edit` is one change, or a list of changes that one transaction makes, planned in order as one edit (section 6.4); a `set` or `remove` may carry `occurrence`, the place in reading order of the holder of a duplicated id it concerns (2 for the second). Its `splices` are listed in the order section 6.5 applies them: by offset, and at one offset in the order the edit made them. A `setting` on a list block that is absent is planned as the tool type's specification gives it, which may write the whole list (DESL section 5.5).

A host passes a fixture when reading the input gives what `read` lists (for an element with `attributes`, each attribute named there has that value and that JSON type; attributes not named are not compared) and every step produces exactly its splices and its document. For a refused step it passes when it refuses the edit and writes nothing: the text of `refused` is informative, because FBL leaves a refusal's wording to the host except where a binding gives the reason itself (`readOnly`, section 3.4, and *(FBL 0.10)* `refusals`, section 6.4), and then it is that reason. The repository's validator checks every fixture's consistency: each step's splices turn the document before it into its `expect`, and back by their inverses; an undo returns the document the undone edit started from; a save and a refused edit change nothing. Undo steps list the inverse splices under the operation of the splice each inverts. *(FBL 0.10)* A file a step's splices name in `file` is replayed as the input is, from its bytes beside the input, with its undo and redo checked the same way; a fixture whose input is a registration so states the edits of the body it opens.

*(FBL 0.7)* A `setting` edit changes an entry of a declared block of the registration, which is then the fixture's input: it names the `block` and the entry's `key`, and gives its new `value`, its new `position` in a `kept` block (`{index}`, `{after: key}` or `{before: key}`), a new key as `rename`, or `remove: true`; a `setting` with a `position` and no `value` moves the entry. When the input is a registration, the fixture's `read` is that of the document it opens: the elements of the body it names, and the findings, a registration finding's `line` being a line of the registration. A fixture for a registration that `createOnFirstPlacement` creates starts from the text section 8.4 creates. A step whose setting writes a block outside the history is no step of the history, so an undo after it undoes the edit before it.

Together the fixtures exercise every operation of section 6.1. *(FBL 0.11)* They do so for the `grammar` family's writes too: `replace-value`, `insert-key`, `remove-key`, `insert-entry`, `remove-entry`, `rewrite-reference` and `open-block` each appear in a fixture of a `grammar` binding, and *(FBL 0.16)* `replace-separator`, which only this family writes.

---

## 16. Security and robustness

- **No code execution.** An FBL document contains no code but CEL and regular expressions. Persistence plugins are code and are installed through the explicit trust decision DISL §16 requires.
- **Regular expressions.** Hosts whose regular expression engine backtracks (.NET, Java, ECMAScript) **MUST** bound the time a match may take, or use a linear-time engine; an expression that exceeds the bound is a finding on the statement, not a hang.
- **Paths.** A registration's `body` and a folder subject's files are resolved within the workspace; a host **MUST NOT** read or write outside it, or follow symbolic links, without the user's consent.
- **Sizes.** Hosts **MUST** enforce limits on the size of a body they read and the number of entries they build, as DISL §16 requires for stored diagrams, and report a body over the limit as unreadable rather than read part of it silently.
- **Integrity of other tools' files.** A host never writes a body it could not read, never writes bytes outside an edit's splices, writes atomically, and refuses an undo on drift. These are what make it safe to point ADP at files other tools own.
- **Templates** are text; placeholders are replaced literally and never evaluated.
- **Grammars** *(FBL 0.9)* are data: an LALR(1) automaton reads a body in time linear in its tokens, and a host **MUST** bound the size of a grammar and of its automaton it builds, refusing the binding beyond them. A grammar's files are read only from beside the FBL document.

---

## 17. Today's diagram definitions *(informative)*

The 25 diagram definitions in `definitions/diagrams/` were written before FBL; 24 of them name a persistence plugin as their format and the timeline names DISL's `yaml`. Under FBL:

| Outcome | Definitions |
|---|---|
| A declared binding (15 plus the timeline) | timeline (`timeline.fbl`), dependency graph, functional decomposition graph, hype cycle graph, Databricks bundle, job and pipeline (*(FBL 0.10)* `databricks.fbl`, the one binding the three readings of a bundle file share; `databricks-job.fbl` and `databricks-pipeline.fbl` are the per-reading bindings of 0.1), causal loop diagram (`causal-loop-diagram.fbl`), Wardley map (*(FBL 0.12)* `wardley-map.fbl` *(grammar)*, the `.owm` read by the `grammar` family with `wardley-map.lark`), mind map (`mindmap.fbl`), the six C4 types (`structurizr.fbl` *(grammar)*, *(FBL 0.11)* the Structurizr DSL read by the `grammar` family with `structurizr.lark`) |
| A declared binding, with a plugin for part of its reading | Azure DevOps pipeline (template expansion) |
| A plugin reader under the contract (8) | the four W3C types (`w3c-turtle.fbl`), SPARQL query, .NET dependency graph (`dotnet-dependency-graph.fbl`), Ansible structure (`ansible-structure.fbl`), Helm chart (`helm-chart.fbl`) |

The timeline's specification would then say:

```json
{
  "persistence": {
    "format": "fbl",
    "binding": "https://etalii.net/adp/fbl/examples/0.1/timeline.fbl#timeline",
    "ids": { "strategy": "uuid-v4", "encoding": "base36" },
    "view": { "store": [] }
  }
}
```

Rewriting the definitions to use FBL is follow-up work, as is implementing FBL in the hosts. The per-definition record is in the feature that introduced FBL, `specs/005-format-binding/inventory.md`.

---

## Appendix A — JSON Schema

The normative schema is [`fbl.schema.json`](fbl.schema.json) beside this document (JSON Schema draft 2020-12, `$id` `https://etalii.net/adp/fbl/schema/0.1/fbl.schema.json`). Its `$defs` are: `Document`, `Binding`, `Claims`, `Marker`, `Body`, `Family`, `FileRule`, `PluginReader`, `TextDefaults`, `Header`, `BlockRule`, *(FBL 0.7)* `SectionRule`, `ElementRule`, `RelationRule`, `IdBinding`, `Slot`, *(FBL 0.9)* `GrammarSource` and `Recover`, `AttributeBinding`, `Insert`, `Remove`, `RegistrationSettings`, `Template`, `Registration`, `Splice`, `Fixture`, `Edit`, the model changes `ModelChange` and `ChangePosition` (FBL 0.2, section 11.2), and the helpers `Name`, `BindingRef`, `Regex`, `Selector`. While FBL is a draft, the schema keeps its 0.1 `$id` and accepts every version from `"fbl": "0.1"` to `"0.12"`. It references DISL's `Doc`, `LocalizedText`, `Expression` and `TypeRef`.

The examples beside this document are `timeline.fbl`, `databricks-job.fbl`, `databricks-pipeline.fbl`, `mindmap.fbl`, `causal-loop-diagram.fbl`, `structurizr.fbl`, `w3c-turtle.fbl`, `helm-chart.fbl`, `ansible-structure.fbl`, `dotnet-dependency-graph.fbl`, a .NET solution read by a plugin that opens the files it leads to itself, *(FBL 0.5)* `dependency-graph.fbl`, the dependency graph's hand-written YAML read with every scalar as written, *(FBL 0.5)* `sankey.fbl`, the Sankey diagram's YAML read the same way, with a computed attribute that holds the written text of every member of an entry, *(FBL 0.5)* `supply-chain.fbl`, the supply chain diagram's YAML read the same way, whose one node rule reads the seven stages and their six older spellings through `map` and whose `group` key is a reference to a group's stored id rather than a nesting, *(FBL 0.7)* `behavior-tree.fbl`, Agent Behavior Modelling's Markdown read by the `bullets` family, and *(FBL 0.9)* `w3c-turtle-grammar.fbl`, Turtle read by the `grammar` family with the grammar `w3c-turtle.lark` beside it, which `.github/scripts/atd-grammar.py --check` builds in strict mode as it builds every grammar an example names, *(FBL 0.10)* `databricks.fbl`, one Databricks bundle file read as the bundle, a job and a pipeline through one binding whose rules name the readings they serve, *(FBL 0.11)* `structurizr.fbl` with the grammar `structurizr.lark` beside it, the Structurizr DSL read by the `grammar` family for the six C4 types, and *(FBL 0.12)* `wardley-map.fbl` with the grammar `wardley-map.lark` beside it, the Wardley map's `.owm` read by the `grammar` family, its ids kept by kind and natural key in the legacy identities file the tool writes; registrations are in `registrations/` and round-trip fixtures in `fixtures/`, and *(FBL 0.8)* documents a host **MUST** refuse when it loads them, each valid against the schema, are in `refused/`. `python .github/scripts/validate-examples.py` validates them all, and checks that each document in `refused/` is refused.
