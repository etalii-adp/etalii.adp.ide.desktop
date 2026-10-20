# DID — Diagram Definition Language

**Specification, version 0.2 (Working Draft)**

|                           |                                                                                      |
|---------------------------|--------------------------------------------------------------------------------------|
| Date                      | 2026-09-30                                                                           |
| Definition schema         | `did.schema.json` (JSON Schema, draft 2020-12), `$defs/Definition`                   |
| Specification language    | DISL, the Diagram Specification Language, in [`../disl/`](../disl/DISL-specification.md) |
| Media types (provisional) | `application/vnd.did.definition+json`, `application/vnd.did.fragment+json`           |
| File extension            | `.did`                                                                               |
| Licence                   | [Apache License 2.0](https://github.com/etalii-adp/etalii.adp/blob/develop/LICENSE) (`Apache-2.0`) |

---

## Status of this document

This is a working draft. DID, the Diagram Definition Language, specifies how a diagram a user created is stored. It is complete enough to implement a conforming reader, writer and validator of stored diagrams together with [DISL](../disl/DISL-specification.md), but individual constructs may still change before version 1.0. Sections and paragraphs marked *(informative)* explain intent and give guidance; everything else is *normative*.

DID 0.2 adds to DID 0.1 what DISL 0.2 needs of a stored diagram: tolerant reading of ids, findings instead of refusals, ephemeral and derived elements, suppressions by subject, enum stored forms, fixed attributes, optional relation targets and time value forms. Every valid DID 0.1 definition is a valid DID 0.2 definition and means the same; section 12 lists every change. DID continues the document format of the earlier combined format (DEDL became DISL and DID); section 11 lists the old identifiers that readers still accept. The abbreviation DID here always means ADP's Diagram Definition Language, not the W3C's Decentralized Identifiers.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY** and **OPTIONAL** are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in bold capitals.

---

## Table of contents

1. [Introduction](#1-introduction)
2. [Serialization](#2-serialization)
3. [Logical structure](#3-logical-structure)
4. [Identifiers](#4-identifiers)
5. [View data and style overrides](#5-view-data-and-style-overrides)
6. [Ordering, precision and canonical form](#6-ordering-precision-and-canonical-form)
7. [Fragments](#7-fragments)
8. [Processing model](#8-processing-model)
9. [Conformance](#9-conformance)
10. [Examples](#10-examples)
11. [Deprecated aliases](#11-deprecated-aliases)
12. [Changes from 0.1](#12-changes-from-01)
- [Appendix A — JSON Schema](#appendix-a--json-schema)
- [Appendix B — Glossary](#appendix-b--glossary)

---

## 1. Introduction

### 1.1 What DID is

Each kind of ADP tool has a specification language and a definition language. For diagrams they are DISL and DID. In DISL a **tool engineer** writes a **specification** (a `.dis` file): one diagram type, with its metamodel, coordinates, notation, toolbox, constraints, behavior, layout and persistence settings. The diagrams users then create of that type are stored as **definitions** in DID (`.did` files): the elements and relations a user added, their attribute values, and the view data of each diagram.

A DID definition is always read together with the specification it names in its `language` object. The specification says which types and attributes exist, how view data relates to the model, and which persistence settings apply (DISL section 11); this document says what the stored result looks like and how it is read and written.

"Document" stays the word for the content a user works on in a host; for a diagram, its stored form is a DID definition.

### 1.2 Relation to DISL *(informative)*

- A DID definition names its DISL specification in `language` (section 3).
- The specification's persistence layer (DISL section 11) configures how the definition is written: format, file split, identifiers, view data, ordering and precision.
- A runtime loads the specification, then reads and writes definitions with it (section 8).

The schema of DID references DISL's shared primitives (`QualifiedId`, `SemVer`, `Point`) by their absolute `$id` instead of copying them, so each is defined once.

---

## 2. Serialization

A DID definition is a JSON text (RFC 8259) encoded in UTF-8 without a byte-order mark, unless the specification's persistence settings choose another format (DISL 11.2), which encodes the same logical structure. Its top-level value MUST be an object. The file extension **SHOULD** be `.did`, unless the specification declares its own in `language.fileExtension`.

A definition **SHOULD** declare the schema it conforms to, and **MUST** declare the DID version it targets in the version key `did`, as `"major.minor"`:

```json
{
  "$schema": "https://etalii.net/adp/did/schema/0.2/did.schema.json#/$defs/Definition",
  "did": "0.2",
  "language": { "id": "org.example.statemachine", "version": "1.2.0" }
}
```

> The schema is published by the ADP website at `https://etalii.net/adp/did/schema/<version>/did.schema.json`.

A writer of this version **MUST** write `"did": "0.2"`. A reader of this version **MUST** accept `"0.1"` and `"0.2"`, and reads a DID 0.1 definition as DID 0.2: every valid 0.1 definition is a valid 0.2 definition with the same meaning (section 12). A definition that names the 0.1 schema address in `$schema` is validated against the 0.2 schema.

A reader **MUST** refuse a definition with a higher major DID version than it supports and **SHOULD** warn for a higher minor version. The media type of a definition is `application/vnd.did.definition+json`, unless the specification declares another in `persistence.mediaType`.

Any object in a definition **MAY** contain properties whose names begin with `x-`. Readers **MUST** ignore extension properties they do not understand and **MUST** preserve them unchanged when rewriting a file (DISL 2.8).

---

## 3. Logical structure

Independently of format and file split, a definition has this logical structure (JSON Schema: `did.schema.json`, `$defs/Definition`):

```json
{
  "$schema": "https://etalii.net/adp/did/schema/0.2/did.schema.json#/$defs/Definition",
  "did": "0.2",
  "language": { "id": "org.example.statemachine", "version": "1.2.0" },
  "meta": {
    "createdAt": "2026-09-25T20:14:03Z",
    "modifiedAt": "2026-09-26T08:00:00Z",
    "generator": "Acme Diagrams 4.2"
  },
  "diagram": { "attributes": { "title": "Door controller" } },
  "elements": [
    { "id": "st_01J8Z3…", "type": "InitialState" },
    { "id": "st_01J8Z4…", "type": "State", "attributes": { "name": "Closed" } },
    { "id": "st_01J8Z5…", "type": "State", "attributes": { "name": "Open", "entryAction": "light.on()" } }
  ],
  "relations": [
    { "id": "tr_01J8Z6…", "type": "Transition", "source": "st_01J8Z4…", "target": "st_01J8Z5…",
      "attributes": { "trigger": "open" } }
  ],
  "views": [
    {
      "id": "v_main", "viewpoint": "main",
      "viewport": { "x": 0, "y": 0, "zoom": 1 },
      "nodes": {
        "st_01J8Z3…": { "x": 40,  "y": 60,  "w": 20,  "h": 20 },
        "st_01J8Z4…": { "x": 120, "y": 40,  "w": 120, "h": 60 },
        "st_01J8Z5…": { "x": 340, "y": 40,  "w": 120, "h": 60, "style": { "fill": "#EAF3DE" } }
      },
      "edges": {
        "tr_01J8Z6…": { "waypoints": [], "labels": { "trigger": { "dx": 0, "dy": -6 } } }
      }
    }
  ],
  "suppressions": [],
  "extensions": {}
}
```

**Element record** (`elements[]`):

| Property     | Type                       | Description                                                                                                                                       |
|--------------|----------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| `id`         | string                     | **Required**, unique within the definition.                                                                                                       |
| `type`       | string                     | **Required**, a concrete node type (qualified by import alias if imported).                                                                       |
| `attributes` | object                     | Stored attribute values (omitting unset and, if `omitDefaults`, default values). Attribute values follow DISL 4.2 representations; references are ids. |
| `parent`     | id                         | Containing element (omitted for top-level).                                                                                                       |
| `slot`       | string                     | Slot within the parent (DISL 4.8).                                                                                                                   |
| `order`      | int or string              | Position among siblings when `children.ordered` (fractional-index strings are RECOMMENDED for collaboration: `"a0"`, `"a0V"`).                    |
| `ports`      | `{id, type, attributes}`[] | Port instances. `type` is the port name (`"in"`).                                                                                                 |
| `x-*`        |                            | Extensions.                                                                                                                                       |

Nesting is expressed by `parent`, not by physical nesting, so moving an element between containers changes one line. Runtimes MAY offer a nested pretty-print as an alternative `layout: "nested"`, but the flat form is canonical.

**Attribute values.** Attribute values in `attributes` (of elements, ports, relations, view-only elements and `diagram`) follow the representations of DISL 4.2, with these rules:

- An enum value is stored in its stored form: the member's `value` when the enumeration declares one, and otherwise the member's key, as in DID 0.1 (DISL 4.5). A reader maps a stored form back to its key; an unknown stored form is kept for an `extensible` enum and reported otherwise.
- A `fixed` attribute (DISL 4.3) is a constant of the specification and **MUST NOT** be written. A reader that finds a stored value for a fixed attribute **MUST** ignore it, and **MUST** report a warning when it differs from the fixed value. The stored value is dropped on the next write.
- A `yearMonth` value is written `"±YYYY-MM"`: an ISO 8601 year with at least four digits (more when the year needs them) in astronomical numbering, so `0000` is 1 BCE, preceded by `-` for years before `0000` and by nothing otherwise, then `-` and a two-digit month. Examples: `"2026-09"`, `"0000-01"`, `"-3200-06"`, `"12026-01"`. A writer **MUST** write at least four year digits and a leading `-` for negative years, and **MUST NOT** write a leading `+`; a reader **SHOULD** accept a leading `+`.
- A `datetime` value is written as an RFC 3339 date-time, as in DID 0.1. When the attribute declares `writtenPrecision: "preserve"`, a value **MAY** also be a full date (`"YYYY-MM-DD"`, RFC 3339 `full-date`); when it declares `timezone: "floating"` or `writtenPrecision: "preserve"`, a value **MAY** also be a local date-time without offset (`"YYYY-MM-DDThh:mm[:ss[.fff]]"`). A reader **MUST** accept these forms for such an attribute, and a writer **MUST** write a value it read, or changed, in the form and precision it was read with (DISL 4.2, `writtenPrecision`); a new value takes the attribute's `newPrecision`.

**Derived elements.** Records of derived types (DISL 4.11) **MUST NOT** be written: derived elements are computed from the model on every load (section 8.1) and are never part of the stored model. A reader that finds a record of a derived type in `elements` or `relations` **MUST** treat it as unknown content (section 8.2): preserved verbatim on save, left out of the model, and reported.

**Relation record** (`relations[]`): `id`, `type`, `source`, `target` (element or port ids), `attributes`, `x-*`. A port id is written `"<elementId>#<portId>"` when port ids are only unique per element. `target` **MAY** be absent when the relation type declares its target end optional (DISL 4.9, `target.optional: true`), for a relation that has no target yet (a stub); it is written only when the relation has a target. A reader **MUST** report a relation without `target` whose type does not declare an optional target end as an unresolvable reference; `source` is always **REQUIRED**.

**View record** (`views[]`): one per diagram (a definition can hold several diagrams of the same model, one per viewpoint or several of one viewpoint).

| Property    | Type                       | Description                                                                                                                                               |
|-------------|----------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|
| `id`        | string                     | View id.                                                                                                                                                  |
| `viewpoint` | string                     | Viewpoint name.                                                                                                                                           |
| `name`      | string                     | Diagram name shown in tabs.                                                                                                                               |
| `viewport`  | `{x, y, zoom}`             | Last viewport, if `viewport` is in `view.store`.                                                                                                          |
| `nodes`     | map id → NodeView          | Placement of nodes in this view. Elements not listed are hidden in this view (unless the viewpoint auto-includes them, in which case layout places them). |
| `edges`     | map id → EdgeView          | Routing and label data.                                                                                                                                   |
| `viewOnly`  | `{id, type, attributes}`[] | View-only elements (notes, frames, images, free text) that belong to this view only.                                                                      |
| `guides`    | `{axis, value}`[]          | User guides.                                                                                                                                              |
| `settings`  | object                     | Per-view settings (grid visible, snapping on/off, theme mode).                                                                                            |

**NodeView**:

| Property          | Description                                                                                                                                                                                                    |
|-------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `x`, `y`          | Position in domain values of the placement's system (numbers, timestamps, band references). Omitted for bound or computed coordinates — **bound values live only in the model**, never duplicated in the view. |
| `w`, `h`          | Size in domain units (durations as ISO 8601 on time axes). Omitted when bound, computed or equal to the default (with `omitDefaults`).                                                                         |
| `x2`, `y2`        | When the placement uses `x2`/`y2` and they are free.                                                                                                                                                           |
| `angle`, `radius` | Polar placements.                                                                                                                                                                                              |
| `rotation`        | Degrees.                                                                                                                                                                                                       |
| `z`               | Integer z-order among siblings.                                                                                                                                                                                |
| `collapsed`       | bool.                                                                                                                                                                                                          |
| `pinned`          | bool (layout `respect: "pinned"`).                                                                                                                                                                             |
| `params`          | Shape parameter values adjusted by handles (DISL 6.8).                                                                                                                                                            |
| `labels`          | map label id → `{dx, dy}` offsets.                                                                                                                                                                             |
| `ports`           | map port id → `{side, position}`.                                                                                                                                                                              |
| `compartments`    | map id → `{collapsed}`.                                                                                                                                                                                        |
| `style`           | Style overrides, restricted to DISL's `view.styleOverrides`.                                                                                                                                                        |
| `hidden`          | bool — element placed but hidden.                                                                                                                                                                              |

**EdgeView**: `waypoints` (list of Points in domain values), `sourceAnchor`, `targetAnchor` (`[fx, fy]` fixed anchors), `controlPoints` (for bezier), `labels` (map id → `{at, dx, dy}`), `routing` (per-edge override if allowed), `style`, `z`, `hidden`.

`language` names the specification: `id` and `version` are **REQUIRED** and equal the specification's `language.id` and the `language.version` the definition was written with; `definition` (the URI of the `.dis` file) and `integrity` (its hash) are written when the specification's `persistence.definition.embed` is `"reference"` (DISL 11.8). The property keeps the name `definition` it had in the earlier format.

`meta` holds the metadata the specification lists in `persistence.metadata` (DISL 11.8). `diagram.attributes` holds the attributes of the diagram root element (DISL 4.1).

**Suppressions** of findings (DISL 8.6) are stored as `suppressions: [{constraint, element?, subject?, reason, by, at}]`:

| Property     | Description                                                                                                                                                  |
|--------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `constraint` | **Required**. The id of the constraint or built-in rule whose finding is suppressed.                                                                         |
| `element`    | The id of the element or relation the finding targets.                                                                                                       |
| `subject`    | The subject of a finding that names no element (DISL 8.6, a finding's `subject`), for a finding about something that is not drawn.                           |
| `reason`     | Why the finding is suppressed.                                                                                                                               |
| `by`, `at`   | Who suppressed it, and when (RFC 3339).                                                                                                                      |

A suppression **MUST** have exactly one of `element` and `subject`. A suppression **MUST NOT** name an ephemeral id (DISL 11.5); it **MAY** name a derived element whose id is stable. A stored suppression that names an ephemeral id **MUST** be ignored, reported as a finding, and dropped on the next write, as view data keyed by one is (section 5). Every DID 0.1 suppression, which always has `element`, is a valid DID 0.2 suppression.

---

## 4. Identifiers

Every element, relation, port and view carries an `id` that is unique within the definition and matches the specification's `persistence.ids.pattern` (default `^[A-Za-z0-9_.:#-]{1,128}$`). New ids are generated as the specification's `persistence.ids` declares (DISL 11.5). Unless the specification sets `stable: false`, an id **MUST NOT** change once assigned. References between records (`parent`, relation `source` and `target`, reference-typed attributes, the keys of `nodes` and `edges` in a view, a suppression's `element`) are ids. Nodes, relations and views share one id space; ports are in the id space of their element when port ids are written `"<elementId>#<portId>"`.

**Writers.** A writer **MUST NOT** write a record with a missing or empty id, an id that does not match the pattern, or an id that another record of the definition already has. `id` stays required in the schema, because the schema describes what writers produce. A save writes back unchanged the records the user did not edit, so a duplicate made by hand stays until the user resolves it; a writer **MUST NOT** change an id to resolve a duplicate except through a user action, such as the quick fix of `std.duplicateId`.

**Readers.** A reader **MUST** load a definition in which ids are missing or duplicated, and **MUST** report each case as a finding, `std.missingId` or `std.duplicateId` (DISL 8.7), instead of refusing the definition:

- Of the records that share an id, the first in record order (`elements`, then `relations`, then `views`, each in array order) keeps it: every lookup and reference resolves to it. The second and later are loaded and treated as ephemeral (DISL 11.5) until the duplication is resolved.
- A record without a usable id (absent, empty, or not matching the pattern) is handled as the specification's `persistence.ids.missing` declares. With `"assign"`, it receives a new id from the applicable strategy; the new id **MUST NOT** be written when the definition is opened or validated, and **MUST** be written by the next save that writes the definition. With `"ephemeral"`, it is loaded and treated as ephemeral.

**Comparison.** Ids, including references and the keys of `nodes` and `edges`, are compared as the specification's `persistence.ids.compare` declares: exactly by default, or with `"ignore-case"` under Unicode default case folding, in which case the spelling of the first occurrence in record order is kept.

**Derived ids.** An id whose strategy is `derived` (DISL 11.5) is recomputed from the model on load. When the recomputed id differs from the stored one, the record takes the recomputed id and the references to it (parents, relation ends, reference attributes, view keys and suppressions) follow; the new id is written by the next save.

---

## 5. View data and style overrides

A view stores only the kinds of view data the specification lists in `persistence.view.store` (DISL 11.6); anything else is recomputed on load, by layout or defaults. **Bound values live only in the model**: a coordinate bound to an attribute (DISL 5.8) is never duplicated in `NodeView`. The view-data kinds the specification lists in `persistence.view.viewer` are viewer state, held per viewer: they **MUST NOT** be written to a definition.

**Ephemeral ids.** The keys of a view's `nodes` and `edges` **MUST NOT** name ephemeral ids (DISL 11.5), whose identity does not survive re-reading the model. A stored key that names an id the current reading makes ephemeral **MUST** be ignored, **MUST** be reported by `std.ephemeralViewData`, and **MUST** be dropped when the view data is next written. This covers a definition written before its specification made that id ephemeral.

**Derived elements.** The keys of `nodes` and `edges` **MAY** name derived elements (DISL 4.11) whose ids are stable, so that a user can place a derived node or route a derived edge. A key that names no element once derived elements are computed is kept, written back unchanged and otherwise ignored: it is not an unresolvable reference and produces no finding, because the element it names may be derived again later.

Style overrides are stored in `NodeView.style` and `EdgeView.style`, restricted to the style properties the specification's `persistence.view.styleOverrides` allows, and have the highest precedence when the notation is resolved (DISL 6.1). Resolved theme token values are never stored unless a user explicitly overrides a style.

---

## 6. Ordering, precision and canonical form

A writer orders records and keys, rounds numbers and writes timestamps as the specification's `persistence.ordering`, `omitDefaults`, `precision`, `timestamps` and `canonical` settings declare (DISL 11.7).

**Determinism requirement.** Given the same logical definition and specification, a conforming writer **MUST** produce byte-identical output. In particular: no volatile metadata unless listed in `metadata`; `modifiedAt` only changes when content changes; maps are written in the declared key order; floating-point values are rounded to `precision` then serialized per RFC 8785.

The persisted definition is the canonical form of a diagram; collaboration state (DISL 11.10) is a transport concern, and after a merge the definition is written canonically again.

---

## 7. Fragments

A **fragment** is part of a definition taken out of it: the elements, relations and view data a user copies to the clipboard (`behavior.clipboard.formats`, DISL 9.5) or exports with the export format `did-fragment` (DISL 11.11). Its media type is `application/vnd.did.fragment+json`. A fragment uses the records of section 3 and the rules of sections 4–6, and names its specification in `language` as a definition does. The layout of a fragment beyond that is not specified in DID 0.2.

---

## 8. Processing model

### 8.1 Loading a definition

1. Parse; read `language.id` and `language.version`. A file that cannot be parsed (not a JSON text, or not an object at the top level) produces the finding `std.unparseable` (DISL 8.7), located at the file and, where the parser can tell, at the line and column of the failure; no other finding is evaluated for it, and the diagram shows that finding instead of the model. If the language id differs from the specification's, refuse.
2. If the definition's language version is older, run the specification's **migrations** (DISL 11.9) on the raw structure.
3. Validate against `$defs/Definition` and the specification's metamodel (types exist, attributes typed, references resolvable). Missing and duplicate ids are loaded and reported as section 4 describes. A record in `elements` or `relations` that fails `$defs/ElementRecord` or `$defs/RelationRecord` for any other reason is preserved verbatim as unknown content (section 8.2), left out of the model, and reported as `std.unreadableEntry` with its JSON Pointer (RFC 6901, for example `/elements/4`) as `detail.entry.path` (DISL, section 8.7); the rest of the definition is loaded. The keys of `nodes` and `edges` in a view are not references for this step: a key that names no element is handled in step 5. Records of derived types are unknown content (section 3); stored values of fixed attributes are ignored (section 3).
4. Compute derived elements (DISL 4.11) from the validated model, and recompute derived ids (section 4).
5. Resolve view data, including keys that name derived elements; ignore keys that name no element and keys that name ephemeral ids (section 5). Run layout for elements lacking view data if the specification's layout `trigger` is `onLoadIfMissing` or `always` (DISL 10).
6. Evaluate constraints with `live` timing; show findings, the reader's findings of steps 1 to 5 first (DISL 8.6).
7. Render.

A definition with a **newer** minor or patch language version than the loaded specification is opened with a warning; a newer major version is opened read-only or refused (DISL 11.9).

### 8.2 Unknown content

Elements of unknown types, unknown attributes and unknown view properties in a definition (for example written by a newer language version or a vendor extension) **MUST** be preserved on save. Unknown elements are shown as neutral placeholders ("Unknown element of type X") and reported by `std.typeExists` (DISL 8.7). Readers **MUST NOT** silently drop data.

Unknown content also includes a record kept by step 3 of section 8.1 (`std.unreadableEntry`) and a record of a derived type (section 3). Such a record is not part of the model and is not drawn; it **MUST** be written back verbatim, in its place in record order, until the user removes it.

### 8.3 Saving

1. Evaluate constraints with `save` timing, as DISL 14.5 describes.
2. Serialize the logical definition canonically (section 6). The write rules of sections 3 to 5 apply: ids assigned on load are written, derived elements and values of fixed attributes are not, view keys and suppressions that name ephemeral ids are dropped, and unknown content is written back verbatim.
3. Write files according to the specification's `files.mode` atomically (write to temporary file, then rename).

---

## 9. Conformance

| Class          | Requirements                                                                                                                                                                                                                    |
|----------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Definition** | A definition is conforming to a specification if it validates against `$defs/Definition`, its elements conform to the specification's metamodel after migrations, and it is serialised as sections 3–6 and the specification's persistence settings prescribe. Findings do not make a definition non-conforming. |
| **Reader**     | Implements 8.1 and 8.2 and the reading rules of sections 3 to 5, reads DID 0.1 and 0.2 definitions, and accepts the deprecated aliases of section 11.                                                                         |
| **Writer**     | Implements 8.3, the writing rules of sections 3 to 5 and the determinism requirement of section 6, and writes only the identifiers of this version, never a deprecated alias.                                                  |

---

## 10. Examples

### 10.1 A timeline

This definition was written with the timeline specification of DISL 17.2 (`timeline.dis`). It is a DID 0.1 definition, kept as written: it names the 0.1 schema and `"did": "0.1"`, and it is read as a DID 0.2 definition with the same meaning. Several things to notice:

- Elements are a flat list; relations are separate.
- All placement of tasks and milestones lives in the model (`start`, `end`, `assignee`). The view stores only a moved label and the viewport.
- References (`assignee`, relation ends) are plain ids.
- Unset attributes and defaults are omitted (`status` of *Build pages*, `lag` of most dependencies).

File `timeline.did`:

```json
{
  "$schema": "https://etalii.net/adp/did/schema/0.1/did.schema.json#/$defs/Definition",
  "did": "0.1",
  "language": { "id": "org.example.timeline", "version": "0.4.0" },
  "meta": { "createdAt": "2026-09-21T08:12:00Z", "modifiedAt": "2026-09-25T16:40:00Z", "generator": "Reference runtime 0.1" },
  "diagram": {
    "attributes": {
      "title": "Website relaunch",
      "projectStart": "2026-10-01",
      "projectEnd": "2026-12-18",
      "holidays": ["2026-10-03", "2026-12-24", "2026-12-25"]
    }
  },
  "elements": [
    { "id": "ms_01K5YZT2", "type": "Milestone",
      "attributes": { "title": "Go live", "start": "2026-12-01", "assignee": "res_01K5YX10" } },
    { "id": "res_01K5YX10", "type": "Resource", "attributes": { "name": "Design team" } },
    { "id": "res_01K5YX11", "type": "Resource", "attributes": { "name": "Frontend", "position": 1, "capacity": 2 } },
    { "id": "task_01K5YY01", "type": "Task",
      "attributes": { "title": "Wireframes", "start": "2026-10-01", "end": "2026-10-09",
                      "assignee": "res_01K5YX10", "status": "done", "progress": 1 } },
    { "id": "task_01K5YY02", "type": "Task",
      "attributes": { "title": "Visual design", "start": "2026-10-12", "end": "2026-10-30",
                      "assignee": "res_01K5YX10", "status": "active", "progress": 0.4 } },
    { "id": "task_01K5YY03", "type": "Task",
      "attributes": { "title": "Build pages", "start": "2026-11-03", "end": "2026-11-27",
                      "assignee": "res_01K5YX11" } }
  ],
  "relations": [
    { "id": "dep_01K5Z1A8", "type": "Dependency", "source": "task_01K5YY01", "target": "task_01K5YY02" },
    { "id": "dep_01K5Z1A9", "type": "Dependency", "source": "task_01K5YY02", "target": "task_01K5YY03",
      "attributes": { "lag": 1 } },
    { "id": "dep_01K5Z1B0", "type": "Dependency", "source": "task_01K5YY03", "target": "ms_01K5YZT2" }
  ],
  "views": [
    {
      "id": "v_schedule",
      "viewpoint": "main",
      "name": "Schedule",
      "viewport": { "x": 0, "y": 0, "zoom": 1 },
      "nodes": {
        "task_01K5YY02": { "labels": { "dates": { "dx": 4, "dy": 0 } } }
      },
      "edges": {}
    }
  ]
}
```

### 10.2 Time values and the other 0.2 forms

This DID 0.2 definition was written with a small history specification, `org.example.history`, whose node type `Era` has `start` and `stop` of type `yearMonth` and an enum `kind` whose members declare stored forms, whose node type `Event` has `at` of type `datetime` with `writtenPrecision: "preserve"` and `timezone: "floating"`, and whose relation type `Influence` declares its target end optional. Several things to notice:

- `yearMonth` values are signed astronomical years with at least four digits: `"-3200-01"` is January 3201 BCE, `"0000-01"` is January 1 BCE.
- `at` keeps the form each value was written in: full dates (`"1450-01-01"`), a date-time with an offset (`"2026-09-30T09:00:00Z"`) and a local date-time (`"2026-10-02T14:30"`). A move of *Review of this diagram* writes the new value as a local date-time to the minute.
- `kind` holds the enum's stored forms (`"script"`, `"book"`, `"network"`), not its member keys.
- The relation `inf_web_open` has no `target` yet.
- The second suppression names a `subject`, for a finding about something that is not drawn, instead of an `element`.

File `time-values.did`:

```json
{
  "$schema": "https://etalii.net/adp/did/schema/0.2/did.schema.json#/$defs/Definition",
  "did": "0.2",
  "language": { "id": "org.example.history", "version": "0.2.0" },
  "meta": { "createdAt": "2026-09-30T09:00:00Z", "modifiedAt": "2026-09-30T11:25:00Z", "generator": "Reference runtime 0.2" },
  "diagram": {
    "attributes": {
      "title": "Writing and the web",
      "from": "-3200-01",
      "to": "2026-12"
    }
  },
  "elements": [
    { "id": "era_cuneiform", "type": "Era",
      "attributes": { "name": "Cuneiform", "start": "-3200-01", "stop": "-0100-12", "kind": "script" } },
    { "id": "era_codex", "type": "Era",
      "attributes": { "name": "Codex", "start": "0000-01", "stop": "1450-12", "kind": "book" } },
    { "id": "era_web", "type": "Era",
      "attributes": { "name": "World Wide Web", "start": "1989-03", "stop": "2026-09", "kind": "network" } },
    { "id": "ev_moveable_type", "type": "Event",
      "attributes": { "name": "Moveable type in Mainz", "at": "1450-01-01" } },
    { "id": "ev_proposal", "type": "Event",
      "attributes": { "name": "Information management: a proposal", "at": "1989-03-12" } },
    { "id": "ev_first_site", "type": "Event",
      "attributes": { "name": "First website online", "at": "1991-08-06" } },
    { "id": "ev_draft", "type": "Event",
      "attributes": { "name": "Draft of this diagram started", "at": "2026-09-30T09:00:00Z" } },
    { "id": "ev_review", "type": "Event",
      "attributes": { "name": "Review of this diagram", "at": "2026-10-02T14:30" } }
  ],
  "relations": [
    { "id": "inf_codex_web", "type": "Influence", "source": "era_codex", "target": "era_web" },
    { "id": "inf_web_open", "type": "Influence", "source": "era_web",
      "attributes": { "note": "Not yet connected" } }
  ],
  "views": [
    {
      "id": "v_eras",
      "viewpoint": "main",
      "name": "Eras",
      "viewport": { "x": 0, "y": 0, "zoom": 1 },
      "nodes": {
        "ev_review": { "labels": { "name": { "dx": 0, "dy": -8 } } }
      },
      "edges": {}
    }
  ],
  "suppressions": [
    { "constraint": "eraOverlap", "element": "era_web",
      "reason": "The web era is still open.", "by": "editor@example.org", "at": "2026-09-30T11:20:00Z" },
    { "constraint": "sourcesCited", "subject": "bibliography",
      "reason": "The bibliography is kept outside this diagram.", "by": "editor@example.org", "at": "2026-09-30T11:22:00Z" }
  ]
}
```

---

## 11. Deprecated aliases

DID continues the document format of the earlier combined format: DEDL became DISL and DID, and what that format called a document is a DID definition. Readers of DID 0.x **MUST** accept the following identifiers of the earlier format, version 0.1, as deprecated aliases, read them as their DID form, and never write them:

| Deprecated alias                                                                     | DID 0.1                                                                  |
|--------------------------------------------------------------------------------------|--------------------------------------------------------------------------|
| `$schema` `https://etalii.net/adp/dedl/schema/0.1/dedl.schema.json#/$defs/Document`  | `https://etalii.net/adp/did/schema/0.1/did.schema.json#/$defs/Definition` |
| media type `application/vnd.dedl.document+json`                                      | `application/vnd.did.definition+json`                                    |
| media type `application/vnd.dedl.fragment+json`                                      | `application/vnd.did.fragment+json`                                      |
| version key `"dedlDocument": "0.1"`                                                  | `"did": "0.1"`                                                           |
| export format `dedl-fragment`                                                        | `did-fragment`                                                           |

A definition carries exactly one of the two version keys; the schema accepts either and marks the old one `deprecated`. The aliases of a specification are listed in DISL section 18. The old schema address stays published unchanged, so files that name it keep validating. A legacy fixture in the earlier form, in `specifications/did/legacy/`, is validated through these aliases on every change to this repository. The aliases are removed no earlier than DID 1.0.

---

## 12. Changes from 0.1

DID 0.2 changes DID 0.1 only in the places below. Everything not listed keeps its 0.1 meaning. No valid DID 0.1 definition becomes invalid or changes meaning: every change either loosens the schema or applies only to input that DID 0.1 rejected or did not describe. The corresponding changes to DISL are listed in DISL's own section "Changes from 0.1".

| # | Change | Sections | Reason |
|---|---|---|---|
| 1 | A reader loads records with missing or duplicate ids and reports them (`std.missingId`, `std.duplicateId`); the first in record order keeps a duplicated id. Writers still never write them. With `missing: "assign"`, the new id is written on the next save. Ids compare as `persistence.ids.compare` declares, and derived ids are recomputed on load. | 4, 8.1 | Tolerant loading. |
| 2 | A record that fails its schema for another reason is kept verbatim as unknown content and reported as `std.unreadableEntry` with its JSON Pointer; an unparseable file gives `std.unparseable`. "Problems" are now called findings. | 8.1, 8.2, 9 | Findings instead of refusals. |
| 3 | View keys and suppressions never name ephemeral ids; stored ones are ignored, reported (`std.ephemeralViewData` for view keys) and dropped on the next write. View-data kinds listed in `persistence.view.viewer` are never written. Suppressions may name a `subject` instead of an `element`, with exactly one of the two. | 3, 5 | Ephemeral ids; findings on undrawn subjects. |
| 4 | Records of derived types are never written, and are read as unknown content; view keys and suppressions may name derived elements with stable ids; unresolved view keys are kept and ignored; derived elements are computed after validation and before view data is resolved. | 3, 5, 8.1 | Derived elements. |
| 5 | Enum values are stored in their `value` form; fixed attributes are never stored, and a stored one is ignored with a warning. | 3 | Enum stored forms and fixed attributes. |
| 6 | A relation's `target` may be absent when its relation type's target end is optional. | 3 | Stubs. |
| 7 | `yearMonth` values are `"±YYYY-MM"` with at least four year digits; `datetime` values keep their written form (a full date or a local date-time) under `writtenPrecision: "preserve"` or `timezone: "floating"`. | 3 | Time values. |

---

## Appendix A — JSON Schema

The normative JSON Schema is published as `did.schema.json` (JSON Schema draft 2020-12), `$id` `https://etalii.net/adp/did/schema/0.2/did.schema.json`. Its root, `#/$defs/Definition`, validates **definitions**, and every definition example validates against it. A definition that names the 0.1 `$id` is validated against this schema.

| `$defs`          | Validates                                                                                   |
|------------------|---------------------------------------------------------------------------------------------|
| `Definition`     | A stored diagram (section 3).                                                               |
| `ElementRecord`  | An element record.                                                                          |
| `RelationRecord` | A relation record (`target` optional since 0.2).                                            |
| `ViewRecord`     | A view record.                                                                              |
| `NodeView`       | The view data of a node.                                                                    |
| `EdgeView`       | The view data of an edge.                                                                   |

`QualifiedId`, `SemVer` and `Point` are referenced from `https://etalii.net/adp/disl/schema/0.2/disl.schema.json`. As in DISL, all objects forbid unknown properties except extension properties matching `^x-`. Rules that JSON Schema cannot express — conformance to the specification's metamodel, id uniqueness, resolvable references, the value forms of section 3 and whether a relation may omit its target — are the job of a DID validator that has the specification loaded.

---

## Appendix B — Glossary

| Term               | Meaning                                                                                                        |
|--------------------|----------------------------------------------------------------------------------------------------------------|
| **Definition**     | A diagram a user created of a diagram type, stored as a `.did` file.                                           |
| **Document**       | The content a user works on in a host; for a diagram, its stored form is a definition.                         |
| **Finding**        | The result of evaluating a rule or reading a model, with a severity and where it is: an element, a source location, or an undrawn subject. DID 0.1 and DISL 0.1 called it a problem. |
| **Fragment**       | Part of a definition, copied or exported (section 7).                                                          |
| **Runtime**        | Software in a host that loads a specification and lets users create and change definitions with it.            |
| **Specification**  | A DISL file (`.dis`) in which a tool engineer specifies one diagram type.                                      |
| **Tool engineer**  | The person who specifies a diagram type in DISL.                                                               |
| **View data**      | Per-diagram placement and presentation data stored separately from the model (section 5).                      |

Every other term is defined in DISL, Appendix C.
