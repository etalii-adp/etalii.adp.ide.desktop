# DED — Designer Definition Language

**Specification, version 0.2 (Draft)**

|   |   |
|---|---|
| Date | 2026-10-14 |
| Kind | designer |
| Role | definition language |
| File extension | `.ded` (the JSON form; a designer type's bindings may claim other extensions, `.md` among them) |
| Paired language | DESL, the Designer Specification Language (`.des`) |
| Definition schema | `ded.schema.json` (JSON Schema, draft 2020-12), `$defs/Definition`, `$id` `https://etalii.net/adp/ded/schema/0.1/ded.schema.json` (the address 0.1 gave it) |
| Version key | `"ded": "0.2"` |
| Builds on | [FBL](../fbl/FBL-specification.md) 0.3 for storage |
| Licence | [Apache License 2.0](https://github.com/etalii-adp/etalii.adp/blob/develop/LICENSE) (`Apache-2.0`) |

---

## Status of this document

This is a draft, version 0.2. It holds what the designer types defined so far need of a stored document and nothing more (constitution principle V): the envelope every definition starts with, how a host recognises and reads one, and how a new one is written. DED 0.1 (2026-10-09, for the Knowledge designer, spec 013) states the envelope in YAML, JSON and XML. DED 0.2 (2026-10-14, for the Hierarchical Requirement Designer, spec 028) states it in Markdown (section 2), names `.md` among the shared extensions a binding claims with the envelope as its marker (section 4), and changes nothing else: every valid 0.1 definition is a valid 0.2 definition with the same meaning, and it **MAY** go on saying `0.1`. Any construct may change before 1.0 (constitution principle IV). Sections and paragraphs marked *(informative)* explain intent; everything else is *normative*.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY** and **OPTIONAL** are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in bold capitals.

---

## Table of contents

1. [Introduction](#1-introduction)
2. [The envelope](#2-the-envelope)
3. [The content](#3-the-content)
4. [Reading a definition](#4-reading-a-definition)
5. [Creating and saving a definition](#5-creating-and-saving-a-definition)
6. [The schema](#6-the-schema)
7. [Conformance](#7-conformance)
8. [Examples](#8-examples)

---

## 1. Introduction

### 1.1 What DED is

A **designer** is the kind of ADP tool that lays its content out as a form-based visual layout in which nothing is connected ([terminology](../../docs/terminology.md)). A tool engineer specifies a designer type in [DESL](../desl/DESL-specification.md). DED, the Designer Definition Language, specifies how one document a user created of a designer type is stored: a **DED definition**. A DED definition is a file that says which version of DED it is written in and which designer type it was made with, and holds that designer type's model beside it.

### 1.2 Relation to DESL and FBL *(informative)*

DED says what the documents of all designer types hold; DESL says what one designer type's documents hold; FBL says how a body is read and written. A designer type's DESL specification names FBL bindings (DESL section 5), and those bindings read the definition's body into the model of the type's metamodel and write each edit back as splices, so a user's file keeps its bytes wherever nothing changed. DED adds the one thing that all designer types share: the envelope of section 2, by which any host can tell a designer's document from any other file and knows the designer type to open it with, without a registration.

---

## 2. The envelope

Every DED definition **MUST** begin with its envelope: the version of DED and the designer type's origin (DESL section 3.2). How the envelope is written depends on the body's format.

**YAML and JSON.** The envelope is two root keys, in this order, before every other root key:

| Key        | Value | Meaning |
|------------|-------|---------|
| `ded`      | string `"0.1"` or `"0.2"` | The version of DED the definition is written in. In YAML it is quoted, so that it reads as a string. |
| `designer` | string `<vendor>/<type>` | The origin of the designer type, matching `^[a-z0-9-]+/[a-z0-9-]+$` (`etalii/knowledge`). |

```yaml
ded: "0.1"
designer: etalii/knowledge
name: Cities
```

**XML.** The root element is `ded`, with the envelope as its first two attributes, `version` and `designer`, with the values above. The designer type's own attributes of its document follow them on the same element, and its content is the root element's content.

```xml
<ded version="0.1" designer="etalii/knowledge" name="Cities">
```

**Markdown** *(DED 0.2)*. The envelope is line 1 of the file, an HTML comment that holds both values in this order:

```markdown
<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
```

The line is `<!--`, `ded:` and the version, `designer:` and the origin, and `-->`. A host **MUST** write one space between these parts and nothing else on the line, and **MUST** read the line with any run of spaces and tabs between them and after the last, which is the expression `^<!--[ \t]+ded:[ \t]+(?<ded>[^ \t]+)[ \t]+designer:[ \t]+(?<designer>[^ \t]+)[ \t]+-->[ \t]*$` (FBL section 2.5). The version is written without quotes. Only a byte-order mark **MAY** stand before the comment. A Markdown viewer does not show the line, and a person or an agent that reads the file as text does not take it for content *(informative)*.

**Other formats.** A designer type whose bindings read another format **MUST** state in its DESL specification where the envelope is written, as the header of its binding (FBL section 5.6), and **MUST** write both values.

The envelope belongs to DED, not to the designer type: its two values are host attributes of the document's root (DESL section 5.2), and a designer type **MUST NOT** declare an attribute of its document that a binding stores under `ded` or `designer`. Comments **MAY** come between or after the envelope's keys in YAML, as the format allows.

---

## 3. The content

Everything in a DED definition besides its envelope is the designer type's content: the model of its metamodel, as its DESL specification and the bindings it names define it. DED says nothing about the content's shape.

A key, element or line that the designer type's bindings do not read is kept byte for byte through every edit around it (FBL section 4), and a value the bindings cannot read is reported as the `std.unreadableEntry` finding (DESL section 6.5); neither stops the definition from opening.

---

## 4. Reading a definition

A host reads a DED definition in these steps.

1. **Recognise.** A file is a DED definition of a designer type when its envelope names that type's origin. Each binding a designer type names for a shared extension (`.yaml`, `.json`, `.xml`, `.md`) **MUST** use the envelope as its marker (FBL section 12.2): `{rootKey: "designer", value: "<origin>"}` for YAML and JSON, a `pattern` matching the root element's start tag with that `designer` for XML, and a `pattern` with `lines: 1` matching line 1 with that `designer` for Markdown, so that an ordinary Markdown file is never taken for a definition. A registration (FBL section 8) **MAY** still name the designer type for a file that lacks the envelope; the binding's header then decides (FBL section 5.6): a header that is not `required` reads the file and reports the difference, a `required` one, as the Knowledge designer's are, makes the file unreadable.
2. **Check the version.** A `ded` of `0.1` or `0.2` is read as this document says. A host **MUST** open a definition with a later version of DED read-only, with the reason, unless it implements that version.
3. **Find the designer type.** A host **MUST** open a definition whose `designer` names no designer type it has installed read-only, if at all, with the reason naming the origin. It **MUST NOT** read such a definition through another designer type's bindings.
4. **Read the content** through the bindings of the designer type's DESL specification (DESL section 10.1), which check the envelope as their header and report what they find.

When a registration and the envelope name different designer types, the envelope decides and the host reports the difference as a finding.

---

## 5. Creating and saving a definition

When a user creates a document of a designer type, the host **MUST** write the envelope first, with `ded` the version of DED it writes and `designer` the designer type's origin, and then the designer type's content, as the template of the binding chosen for the file (FBL section 13) gives it. A Markdown definition **MUST** be written with `0.2` or a later version, since DED 0.1 does not state its envelope.

The envelope is written when the definition is created and changed by nobody afterwards: no operation of a designer type (DESL section 8) can set it, and saving a definition (DESL section 10.4) leaves its bytes as they are.

The JSON form of a definition **MAY** have the extension `.ded`; a designer type whose JSON binding claims `.ded` reads it as it reads its `.json` files.

---

## 6. The schema

`ded.schema.json` checks the envelope of a definition in its YAML or JSON form: `$defs/Definition` requires `ded` with the value `"0.1"` or `"0.2"` and `designer` matching the origin pattern, and allows every other key. The schema keeps the `$id` it had in 0.1, so a file schema that refines it needs no change. A designer type that publishes a schema of its documents' files **SHOULD** refine `$defs/Definition` with an `allOf`, as `definitions/designers/knowledge.schema.json` does. The XML form is checked after reading it into the YAML and JSON form, its root element's `version` becoming `ded`. The Markdown form is checked the same way, the two values of line 1 becoming `ded` and `designer`.

A definition the schema refuses may still open: the host reads what it can and reports the rest as findings (section 4).

---

## 7. Conformance

| Class | Requirement |
|-------|-------------|
| **Definition** | Begins with the envelope of section 2, with a `ded` this document defines and the origin of a designer type. |
| **Host** | Recognises, checks and reads a definition as section 4 says, writes a new one as section 5 says, and never changes an envelope. It is the DESL host of DESL section 11. |
| **Validator** | Checks a definition's envelope against `ded.schema.json`, and its content against the file schema of its designer type when there is one. |

---

## 8. Examples

[`library.ded`](library.ded) is a table of the Knowledge designer in its JSON form. Its envelope names DED 0.1 and the designer type `etalii/knowledge`; everything after it is the table the Knowledge designer's JSON binding reads ([knowledge.md](../../definitions/designers/knowledge.md)). The Knowledge designer's examples in all three forms are in [`definitions/designers/examples/`](../../definitions/designers/examples/).

```json
{
  "ded": "0.1",
  "designer": "etalii/knowledge",
  "name": "Reading list",
  "activeView": "v1",
  "properties": [
    { "id": "p1", "name": "Title", "type": "text", "title": true },
    ...
```

[`web-shop.md`](web-shop.md) is a requirement document of the Hierarchical Requirement Designer, in Markdown. Its envelope is line 1 and names DED 0.2 and the designer type `etalii/hierarchical-requirement`; every line after it is that designer type's content, about which DED says nothing (section 3).

```markdown
<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0003 REQ-0004 -->
# Web shop
...
```
