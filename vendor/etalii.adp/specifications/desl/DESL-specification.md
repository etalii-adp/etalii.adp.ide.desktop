# DESL — Designer Specification Language

**Specification, version 0.2 (Draft)**

|   |   |
|---|---|
| Date | 2026-10-14 |
| Kind | designer |
| Role | specification language |
| File extension | `.des` |
| Paired language | DED, the Designer Definition Language (`.ded`) |
| Document schema | `desl.schema.json` (JSON Schema, draft 2020-12), `$defs/Specification` |
| Builds on | [FBL](../fbl/FBL-specification.md) 0.7 for storage |
| Expression language | CEL — Common Expression Language (https://cel.dev) |
| Licence | [Apache License 2.0](https://github.com/etalii-adp/etalii.adp/blob/develop/LICENSE) (`Apache-2.0`) |

---

## Status of this document

This is a draft, version 0.2. It holds what the designer types defined so far need and nothing more (constitution principle V), and any construct may change before 1.0 (constitution principle IV). DESL 0.1 (2026-10-09, spec 013) holds what the Knowledge designer (`definitions/designers/knowledge.des`) needs. DESL 0.2 (2026-10-14, spec 028, for the Hierarchical Requirement Designer) adds five constructs: the surface kind `hierarchy` (section 7.7), the id strategy `counter` (section 5.3.5), the facet `mentions` with the built-in `std.mentions` (sections 4.3, 6.5), the action `move` (section 8.2), and `persistence.settings`, the part of the model a document's registration keeps (section 5.5). Every valid 0.1 specification is a valid 0.2 specification with the same meaning. DESL defines every construct it uses; it takes nothing from the specification languages of the other kinds of tool. Sections and paragraphs marked *(informative)* explain intent; everything else is *normative*.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY** and **OPTIONAL** are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in bold capitals.

---

## Table of contents

1. [Introduction](#1-introduction)
2. [Foundations](#2-foundations)
3. [The document](#3-the-document)
4. [Metamodel](#4-metamodel)
5. [Persistence](#5-persistence)
6. [Constraints and findings](#6-constraints-and-findings)
7. [Surface](#7-surface)
8. [Operations](#8-operations)
9. [The CEL environment](#9-the-cel-environment)
10. [Processing model](#10-processing-model)
11. [Conformance](#11-conformance)
- [Appendix A — JSON Schema](#appendix-a--json-schema)

---

## 1. Introduction

### 1.1 What DESL is

A **designer** is the kind of ADP tool that lays its content out as a form-based visual layout in which nothing is connected ([terminology](../../docs/terminology.md)). In DESL a tool engineer specifies one designer type: the model its documents hold, where and how they are stored, which models are valid, how the model is presented on the designer's surface, and what each gesture on that surface changes. A DESL file (`.des`) holds one designer type. What users create of that type is stored as DED definitions ([DED](../ded/DED-specification.md)).

### 1.2 An example *(informative)*

```json
{
  "$schema": "https://etalii.net/adp/desl/schema/0.1/desl.schema.json#/$defs/Specification",
  "desl": "0.1",
  "language": { "id": "net.etalii.adp.knowledge", "version": "0.1.0", "origin": "etalii/knowledge", "label": "Knowledge" },
  "metamodel": { "types": { "Property": { … }, "Row": { … }, "Cell": { … }, "View": { … } } },
  "persistence": { "format": "fbl", "bindings": ["knowledge.fbl#yaml", "knowledge.fbl#json", "knowledge.fbl#xml"],
                   "ids": { "strategy": "uuid-v4", "encoding": "base36" } },
  "surface": { "kind": "table", "columns": { "type": "Property", … }, "rows": { "type": "Row" }, … },
  "operations": { "addProperty": { … } }
}
```

`minimal-table.des` beside this document is a complete, small example, and *(DESL 0.2)* `minimal-hierarchy.des`, with its binding `minimal-hierarchy.fbl`, one of the `hierarchy` kind.

### 1.3 Relation to FBL and DED *(informative)*

A designer's documents are DED definitions, stored through FBL ([FBL-specification.md](../fbl/FBL-specification.md)): a document's body is a file that the bindings in `persistence.bindings` read and write by splices, so a user's file keeps its bytes wherever nothing changed. DESL says what the model is and what may happen to it; FBL says how the model is read from a body and how an edit becomes splices; DED says what a document of any designer type holds. *(DESL 0.2)* What of the model a document's registration keeps beside the body (section 5.5) is stored in the blocks a binding declares there (FBL section 8.1).

FBL serves every kind of tool and names the constructs it relies on in a specification language's words. For a designer, those constructs are DESL's: the element types of section 4, the type map and ids of section 5, the findings of section 6 and the transaction of section 10.3. FBL section 1.3 lists the correspondence.

---

## 2. Foundations

### 2.1 Serialization

A DESL specification is a JSON text (RFC 8259) encoded in UTF-8 without a byte-order mark, whose top-level value is an object. Its file extension **SHOULD** be `.des`. Duplicate keys within one JSON object are a specification error. Property order carries no meaning except where this document says so: arrays are ordered, and so are the values of an enumeration and the rules of `constraints`.

### 2.2 Identifiers

- A **simple identifier** (`$defs/SimpleId`) matches `^[A-Za-z_][A-Za-z0-9_]*$`. Element type names, attribute names, parameter names, operation names and function names **MUST** be simple identifiers, so that they are valid CEL identifiers.
- A **qualified identifier** (`$defs/QualifiedId`) matches `^[A-Za-z_][A-Za-z0-9_-]*(\.[A-Za-z_][A-Za-z0-9_-]*)*$` and names languages and finding codes (`knowledge.no-title`).
- Identifiers are case-sensitive. Two identifiers of one namespace **MUST NOT** differ only in case.
- A **type reference** (`$defs/TypeRef`) names an element type of the metamodel; where the schema allows, a list of them (`$defs/TypeRefs`) is given. A reference to an undeclared name is a specification error.

**Namespaces.** Element types and enumerations share one namespace. Constraints, operations and functions each have their own.

**Reserved names.** These **MUST NOT** be the names of attributes of an element type or of the document, because they are members of every element or variables in CEL (sections 9.2 and 9.3): `id`, `type`, `parent`, `slot`, `children`, `document`, `self`, `item`, `p`, `count`, `selection`, and any name beginning with `_` or `$`. An operation's parameters are read as members of `p`, so they may use these names. CEL's keywords (`in`, `as`, `break`, `const`, `continue`, `else`, `for`, `function`, `if`, `import`, `let`, `loop`, `package`, `namespace`, `return`, `var`, `void`, `while`, `true`, `false`, `null`) are excluded too. A body that stores a value under one of these keys maps it to another attribute name in its binding (FBL section 5.2) or in the type map (section 5.2).

### 2.3 Localized text, messages and reasons

A **LocalizedText** (`$defs/LocalizedText`) is a plain string or an object mapping BCP 47 language tags to strings. A runtime picks the best match for the user's locale (RFC 4647 lookup), then `language.defaultLocale` (default `"en"`), then the first entry. `cel` is not a language tag and **MUST NOT** be used as one.

A **Message** (`$defs/Message`) is a LocalizedText or a CEL value `{ "cel": …, "resultType"?, "doc"? }` returning `string` or `map(string, string)` keyed by locale. It is the shape of every sentence and label shown to the user: constraint messages, operation labels, confirmations and reasons. In a Message position an object with a `cel` property **MUST** be read as CEL. A runtime **MUST** evaluate a CEL Message each time it presents it, and **MUST** show a runtime-worded default and log a diagnostic when it fails to evaluate. The labels of the language, types, attributes and enumeration values, and `doc` objects, stay LocalizedText.

A **Reason** (`$defs/Reason`) says why something is refused or unavailable. It is a Message, which always applies, or `{ "when": Expression, "message": Message, "doc"?: Doc }`, which applies while `when` holds. Reasons are used as ordered lists: the **applicable** reason is the first that applies, and a runtime **MUST** show that one, and only that one, as the explanation, wherever the user is looking.

### 2.4 Documentation

Every object of a specification **MAY** carry `doc`, a LocalizedText (its summary) or a **Doc** object (`$defs/Doc`):

| Property      | Type | Meaning |
|---------------|------|---------|
| `summary`     | LocalizedText | One sentence of plain text, for tooltips and hover cards. |
| `description` | LocalizedText | A longer explanation in CommonMark. |
| `rationale`   | LocalizedText | Why the object exists. |
| `examples`    | `{title, description, value}`[] | Illustrations; `value` is any JSON. |
| `seeAlso`     | `{title, href}`[] | Links: absolute URIs, or relative to the specification's location. |
| `tags`        | string[] | Keywords. |
| `audience`    | (`"user"`, `"author"`, `"developer"`)[] | Intended readers; `"author"` is the tool engineer. |
| `since`       | string | The language version the object appeared in. |
| `deprecated`  | bool or `{since, message, replacedBy}` | Marks the object deprecated. |
| `helpUrl`     | string | A page with more help. |

A **label** is the short display name of an object. Without `label`, a runtime derives one from the identifier by splitting camel case and capitalising the first word only (`activeView` → "Active view").

### 2.5 Expressions

Every dynamic value is CEL (section 9). A property of type **Expression** (`$defs/Expression`) is a string of CEL, or `{ "cel": …, "resultType"?, "doc"? }`; when `resultType` is given, a validator **MUST** check that the expression's static type is assignable to it. In actions (section 8.2) every value is an Expression, so a string literal is written with CEL quotes: `"'Untitled'"`.

Expressions have no side effects. A runtime **MUST** impose a cost limit using CEL's cost estimation, `language.limits.celCost` (default 1 000 000), and a validator **SHOULD** reject an expression whose worst-case estimate exceeds it. At run time, an expression that fails is handled by its position:

| Position | On an evaluation error |
|----------|------------------------|
| A constraint's `rule`, `when` or `severity` | Reported as a finding of the constraint, with its severity and the message "could not be evaluated: …"; never silently passes. |
| A Message or a Reason's `when` | The position's default text is shown, or the reason does not apply; a diagnostic is logged. |
| An operation's actions, or a confirmation | The whole transaction is rolled back and the error reported to the user. |
| A derived id (5.3.2) | `std.missingId` on the element, as section 5.3.4 says. |

Every expression is evaluated in one **context** (9.3), which determines its variables. A validator **MUST** type-check every expression in its context, with the attribute types of the metamodel.

### 2.6 Extension properties

Any object of a specification **MAY** contain properties whose names begin with `x-`. Their content is unconstrained. A conforming tool **MUST** ignore extension properties it does not understand and **MUST** keep them when it rewrites a specification.

### 2.7 Versioning

`desl` (required) is the DESL version the specification is written in, as `"major.minor"`: `"0.1"` or `"0.2"`. Each is a superset of the one before, and a runtime **MUST** read a specification of an earlier minor version with its meaning. A runtime **MUST** refuse a specification of a higher major version than it supports, and **SHOULD** read one of a newer minor version, ignoring what it does not know, with a warning.

`language.version` (required) is the version of the designer type, a SemVer 2.0.0 string. A change that can make existing documents invalid or change their meaning is a **major** change; adding optional constructs is a **minor** change; documentation and wording are **patch** changes.

---

## 3. The document

### 3.1 Top-level object

A DESL document is validated by `desl.schema.json#/$defs/Specification`:

| Property      | Type | Req. | Description |
|---------------|------|------|-------------|
| `$schema`     | URI | – | The schema's `$id` with `#/$defs/Specification`. |
| `desl`        | `"0.1"`, `"0.2"` | ✓ | The DESL version (2.7). |
| `language`    | Language | ✓ | The designer type's identity (3.2). |
| `functions`   | map SimpleId → Function | – | Reusable CEL functions (3.3). |
| `metamodel`   | Metamodel | ✓ | What a document holds (section 4). |
| `persistence` | Persistence | ✓ | How documents are stored (section 5). |
| `constraints` | Constraints | – | What makes a document valid (section 6). |
| `surface`     | Surface | ✓ | How the model is laid out (section 7). |
| `operations`  | map SimpleId → Operation | – | What each gesture changes (section 8). |
| `doc`         | Doc | – | Documentation of the designer type as a whole. |
| `x-*`         | any | – | Extensions (2.6). |

### 3.2 Language

| Property        | Type | Req. | Description |
|-----------------|------|------|-------------|
| `id`            | qualified identifier | ✓ | Globally unique id of the designer type; reverse-DNS style is **RECOMMENDED**. |
| `version`       | SemVer string | ✓ | The designer type's version (2.7). |
| `origin`        | string `<vendor>/<type>` | ✓ | The designer type's origin, matching `^[a-z0-9-]+/[a-z0-9-]+$` (`"etalii/knowledge"`). It is the first line of every registration of the type's documents (FBL section 8.1), the `designer` every DED definition of the type names, and the type's key in a host's catalogue. Successive versions of one designer type keep the same origin. |
| `label`         | LocalizedText | – | Display name ("Knowledge"). |
| `doc`           | Doc | – | Shown in the runtime's help. |
| `defaultLocale` | BCP 47 tag | – | Default `"en"`. |
| `locales`       | string[] | – | Locales every LocalizedText **SHOULD** translate into. |
| `authors`       | `{name, email, url}`[] | – | The tool engineers who maintain it. |
| `license`       | SPDX expression | – | The specification's licence. |
| `homepage`      | URI | – | Project page. |
| `icon`          | string | – | The designer type's icon, by name. |
| `limits`        | `{celCost, maxElements}` | – | The cost limit of one expression (2.5), and a soft limit on the elements of a document beyond which a runtime **SHOULD** warn. |

### 3.3 Functions

`functions` declares reusable CEL functions, called by name in any expression. A **Function** (`$defs/Function`) has positional `params`, each `{name, type, doc}` with a CEL type or a metamodel type, a `returns` type, and its body `cel`. The body sees its parameters only, and `document` when `uses` lists it. A function **MAY** call functions declared before it; a function that calls itself, directly or through others, is a specification error.

---

## 4. Metamodel

The metamodel says what a document of the designer type can hold: the kinds of elements, their attributes, and how they nest. Nothing in it is drawn connected; a link from one element to another is a reference attribute (4.7), shown as a value.

### 4.1 Overview

| Property   | Type | Description |
|------------|------|-------------|
| `document` | `{attributes, children, doc}` | The attributes of the document as a whole: for a table, its name and the view it was last left in. *(DESL 0.2)* `children`, a Containment (4.6), says which types the document's own elements may be. |
| `enums`    | map → Enum | Enumerations (4.4). |
| `types`    | map → ElementType | **Required.** The element types (4.5). |
| `doc`      | Doc | |

### 4.2 Primitive types

| Type       | CEL type | Stored as | Facets |
|------------|----------|-----------|--------|
| `string`   | `string` | a string, one line | `minLength`, `maxLength`, `pattern` |
| `text`     | `string` | a string, any number of lines | as `string` |
| `int`      | `int` | an integer | `min`, `max` |
| `number`   | `double` | a number | `min`, `max` |
| `bool`     | `bool` | `true` or `false` | – |
| `date`     | `timestamp` | `"YYYY-MM-DD"` | – |
| `datetime` | `timestamp` | an RFC 3339 string | `timezone`: `"utc"`, or `"preserve"`, which keeps the offset each value was written with |
| `time`     | `duration` since midnight | `"HH:MM[:SS]"` | – |

How a value is written in a body (a YAML scalar's style, a JSON number's form, an XML attribute) is the binding's (FBL section 6.2).

### 4.3 Attributes

An **Attribute** (`$defs/Attribute`) is one named value of an element, of the document, or of an operation's parameters.

| Property                  | Type | Description |
|---------------------------|------|-------------|
| `type`                    | primitive, enumeration or element type | **Required.** An element type makes the attribute a reference (4.7). |
| `label`, `doc`            | | Display name and help. |
| `required`                | bool | The attribute must have a value; checked by `std.required` (6.5). Default `false`. |
| `default`                 | literal | The value of an attribute that has none stored. |
| `many`                    | bool | The attribute holds a list. Default `false`. |
| `readOnly`                | bool | Not editable by the user; operations may still set it. |
| `min`, `max`              | number or string | Bounds of an `int` or `number`; checked by `std.facets` (6.5). |
| `minLength`, `maxLength`, `pattern` | | Facets of a `string` or `text`; checked by `std.facets`. |
| `timezone`                | `"utc"`, `"preserve"` | How a `datetime` keeps its zone (4.2). |
| `mentions`                | `{pattern, types}` | *(DESL 0.2)* A `string` or `text` attribute whose value names elements by id (below). |

**Mentions** *(DESL 0.2)*. A `mentions` facet makes the text of a `string` or `text` attribute name elements of the same document where it is written. `pattern` is a regular expression in FBL's common subset (FBL section 2.5) with a named group `id`; `types` (TypeRefs, **required**) are the element types a mention may name.

- A runtime finds the matches of `pattern` in the value from left to right, each starting after the one before it ends. A match in which the group `id` takes part is a **mention**, naming the element whose id is that group's text; a match in which it takes no part is not, so an alternative before it can say what is not a mention: `\[(?:ITEM|REQ)-\d+\]\([^)]*\)|\[(?<id>(?:ITEM|REQ)-\d+)\]` reads `[REQ-0002]` as a mention and the Markdown link `[REQ-0002](https://…)` as none.
- A mention resolves to the element with its id, as a reference does (4.7), the first holder of a duplicated id included (5.3.4). A mention that names no element of `types` is reported by `std.mentions` (6.5), located at the mention itself.
- A mention is text, and no runtime changes it: when the element it names is deleted, the text is kept as written and the mention is reported. This is what sets it apart from a reference, which a deletion unsets (4.7).

Reading an attribute that has no stored value yields its `default`, otherwise the zero value of its CEL type (`''`, `0`, `0.0`, `false`, `[]`, `null` for a reference). `has(self.attr)` tests whether a value is stored. A writer leaves out a value that is not stored.

### 4.4 Enumerations

An **Enum** (`$defs/Enum`) lists its values in display order, by key: `values` (**required**, map simple identifier → EnumValue), `ordered` (the values have a meaningful order), `label` and `doc`. An **EnumValue** has `value`, its stored form (a non-empty string, default its key, unique within the enumeration), `label`, `icon` and `doc`. A reader **MUST** map a stored form back to its key; CEL always sees the key. A stored form no value has is kept as written and reported by `std.facets`.

### 4.5 Element types

An **ElementType** (`$defs/ElementType`) is one kind of element a document holds.

| Property         | Type | Description |
|------------------|------|-------------|
| `label`, `doc`   | | Display name and help. |
| `icon`           | string | The type's icon, by name. |
| `attributes`     | map → Attribute | Its attributes. |
| `children`       | Containment | What its elements contain (4.6). |
| `labelAttribute` | attribute name | The attribute that names one of its elements in lists, findings and pickers. Default: the first required `string` attribute, else the first `string` attribute. |

An element has one type. `isA(t)` (9.2) is true for an element of type `t`.

### 4.6 Containment

**Containment** (`$defs/Containment`) makes elements nest: a property contains its options, a row its cells, a view its settings. A contained element has exactly one parent, and deleting the parent deletes what it contains.

| Property  | Type | Description |
|-----------|------|-------------|
| `allowed` | TypeRef[] | The types that may be direct children. |
| `ordered` | bool | Child order is meaningful and kept. |
| `slots`   | map → `{allowed, max, doc}` | Named places for children, when an element holds several kinds apart: a view's `columns`, `sorts` and `filter`. Each child records its slot. |
| `doc`     | Doc | |

An element type without `children` contains nothing. Elements without a parent are the document's own, in document order (10.2). *(DESL 0.2)* `metamodel.document.children` allows the types they may be, as an element type's `children` does; without it, they may be of any type.

### 4.7 References

A **reference** is an attribute whose type is an element type. It stores the target's id, is resolved to the element in CEL, and is not containment. A reference that names no element is kept as written and reported by `std.references` (6.5); in CEL it reads `null`. Deleting an element **MUST** unset, in the same transaction, every reference to it that the specification's operations do not delete or change themselves.

A reference to an element of another document, such as a knowledge file's relation to the rows of another file, is not a reference in this sense: it is a value that names the other document and an id in it, resolved by the host (section 5.4).

---

## 5. Persistence

| Property   | Type | Description |
|------------|------|-------------|
| `format`   | `"fbl"` | **Required.** The only format of DESL 0.1 and 0.2. |
| `bindings` | BindingRef[] | **Required**, at least one. One FBL binding per format family a document of this type may be written in, each `<document>#<name>` (FBL section 2.3), resolved against the specification's own location. |
| `typeMap`  | map binding type → TypeMapEntry | How the bindings' types and attributes map onto the metamodel (5.2). |
| `ids`      | IdStrategy | How elements get their ids (5.3). |
| `settings` | map block name → Setting | *(DESL 0.2)* The part of the model a document's registration keeps, by the name of the block each part is kept in (5.5). |
| `doc`      | Doc | |

### 5.1 Bindings

- The bindings **MUST** read the same model: for the same content, every binding **MUST** yield the same elements, attributes and findings, so that a document's format never changes what it means.
- Two bindings **MUST NOT** claim the same extension or the same file name (FBL section 12.1). A host opening a body uses the binding whose claims take it (FBL section 8.1).
- Each binding's `template` is the body of a new document in its format. A host that creates a document **MUST** let the user choose the format at that moment, and **MUST NOT** offer to change a document's format afterwards: that would rewrite every byte.
- A rule of a binding produces elements of the type its `type` names: an element type of the metamodel, or a type the type map maps.

### 5.2 Type map

A binding may name its types and attributes as its format does rather than as the metamodel does. `typeMap` maps each binding type, by its name, to a **TypeMapEntry** (`$defs/TypeMapEntry`):

| Property         | Type | Description |
|------------------|------|-------------|
| `as`             | `"document"`, `"header"`, `"unreadable"` or TypeRef | **Required.** What the binding's entries of this type become. |
| `attributes`     | map binding attribute → attribute name or `null` | The binding's attribute names mapped to the metamodel's; unlisted attributes keep their names, and `null` leaves one out of the model. |
| `hostAttributes` | string[] | Attributes the binding reads and writes that are not attributes of the model. |
| `doc`            | Doc | |

- Without an entry, a binding type and its attributes **MUST** have the names of an element type and its attributes.
- `as: "document"` makes the entry's attributes the document's (`metamodel.document`). `"header"` and `"unreadable"` keep the entry out of the model; an `unreadable` one is reported by `std.unreadableEntry` (6.5).
- `hostAttributes` are not visible in CEL, on the surface or to constraints, and a writer keeps their values.
- *(DESL 0.2)* When the body has no entry of a type mapped as `document` and a transaction sets a document attribute that type binds, the change is planned as an add of such an entry, by its rule's `insert`; a rule without one refuses the set.

### 5.3 Identifiers

`persistence.ids` (`$defs/IdStrategy`) says how elements are identified: one rule for every type and, in `types`, rules for single types that override it.

| Property     | Type | Description |
|--------------|------|-------------|
| `strategy`   | `"uuid-v4"`, `"uuid-v7"` (default), `"derived"` | How ids are made: a random UUID, a time-ordered UUID, or computed from the model (5.3.2). A rule in `types` may also be *(DESL 0.2)* `"counter"`, counted up per type (5.3.5). |
| `encoding`   | `"hex"` (default), `"base36"` | The text form of a UUID (5.3.1). |
| `stable`     | bool | An id never changes once it is made. Default `true`. No effect on derived ids. |
| `missing`    | `"assign"` (default), `"ephemeral"` | What happens to an element read without a usable id (5.3.4). Top level only. |
| `types`      | map TypeRef → IdRule | Per-type rules (5.3.2). Top level only. |
| `doc`        | Doc | |

An **IdRule** (`$defs/IdRule`) has `strategy`, `expression` (**required** for `derived`), `ephemeral` (bool or Expression), `reason` (a Reason) and `doc`, and *(DESL 0.2)* for `counter` the properties of section 5.3.5.

#### 5.3.1 Generation

- Ids **MUST** be treated as opaque strings: a runtime **MUST NOT** decode one to recover a UUID, a time or a key. A hand-written id **MUST** be accepted whatever the strategy. *(DESL 0.2)* The one exception is the `counter` strategy, which reads the number of an id that is its prefix followed by digits (5.3.5) and treats every other id as opaque.
- `hex` is the RFC 9562 hyphenated lowercase form. `base36` is the 128-bit value as an unsigned integer in lowercase base 36, left-padded with `0` to 25 characters.
- An id for a new element **MUST** be generated once, when the transaction is first applied. Redoing the transaction **MUST** re-create the element with the same id, and a redo that would duplicate an id **MUST** be refused.

#### 5.3.2 Per-type rules and derived ids

- A rule in `types` applies to the named type; properties it leaves out are taken from the top-level rule.
- With `derived`, an element's id **MUST** be the value of `expression`, evaluated in the `identity` context (9.3), which returns a string. It **MUST** be recomputed after the document is read and in every transaction before constraints are checked (10.3). An identity expression **MUST NOT** read the id of any element other than an ancestor of `self`; a validator **MUST** reject one that visibly does. A runtime computes ids parents before children.
- A derived id **MUST NOT** be altered to make it unique: two elements with one derived id are reported by `std.duplicateId` (5.3.4).
- When a derived id changes in a transaction, the runtime **MUST** rewrite every reference to it in the same transaction.

```json
"ids": { "strategy": "uuid-v4", "encoding": "base36", "stable": true,
  "types": { "Cell": { "strategy": "derived", "expression": "self.parent.id + '/' + self.property.id" } } }
```

A cell is its row and its property: the cell of row `r1` for property `p3` is `r1/p3`, and it stores no id of its own.

#### 5.3.3 Ephemeral ids

An element is **ephemeral** when its rule's `ephemeral` is true, or an expression that is true for it. Its id identifies it within one reading of the document only, such as a filter condition numbered by its place.

- A runtime **MUST NOT** store a reference by an ephemeral id, or anything else keyed by one.
- A gesture whose effect would be such a write **MUST** be refused before it is applied, with the rule's `reason` or, without one, a runtime sentence saying that the element's identity does not survive re-reading.
- An ephemeral element **MAY** be selected, edited, and targeted by findings and operations within the session.

#### 5.3.4 Missing and duplicate ids

- All elements of one document share one id space.
- A runtime **MUST** open a document in which ids are missing or duplicated, and **MUST** report each case by `std.missingId` or `std.duplicateId` (6.5). It **MUST NOT** refuse the document for it.
- Of the elements that share an id, the first in reading order (10.2) **MUST** keep it: every reference resolves to it. The second and later are shown, and are ephemeral until the duplication is resolved.
- With `missing: "assign"`, an element without a usable id **MUST** receive a new one from its rule. The new id **MUST NOT** be written when the document is opened or validated, and **MUST** be written with the next edit that writes the document. With `missing: "ephemeral"`, such an element is ephemeral.
- A runtime **MUST NOT** change an id to resolve a duplicate, except by a user's action.

#### 5.3.5 Counted ids *(DESL 0.2)*

With `strategy: "counter"`, given per type in `types`, a type's ids are a prefix and a number counted up per document:

| Property      | Type | Description |
|---------------|------|-------------|
| `prefix`      | string | **Required.** What every id of the type starts with: `REQ-`. |
| `digits`      | int ≥ 1 | The least number of digits a new number is written with, padded with `0`: `4` gives `REQ-0012`. A number that needs more is written with more, and no existing id changes. Default `1`. |
| `mark`        | attribute name | **Required.** A `string` attribute of the document (`metamodel.document`) that holds the last id the strategy gave. |
| `placeholder` | string | What a writer puts after the prefix in place of a number: with `NEW`, an element written `REQ-NEW` has no number yet. |

- **The number of an id** is the integer its digits make, when the id is the prefix followed by digits only: `REQ-0012` and `REQ-12` have the number 12. They remain different ids. Every other id has no number.
- **A new id** is the prefix and one more than the higher of the mark's number and the highest number of any id in the document that has the prefix. The mark is set to the new id in the same transaction, so a number is not given again after the element that carried it is deleted, and an undo that takes the element back takes the mark back with it.
- **A missing mark**, or one without a number, reads as no mark: the count continues from the highest number in the document, and the mark is written with the next new id.
- **An element read with the placeholder** has no usable id. It is reported by `std.missingId`, and under `missing: "assign"` receives a new id as section 5.3.4 says, with the next edit that writes the document, in reading order.
- **A hand-written number** is accepted when no other element has the id (5.3.1), and the count continues past it.
- **Duplicates** follow section 5.3.4 unchanged. On an element that `std.duplicateId` reports, a runtime **SHOULD** offer to give it a new id; that is the user's action section 5.3.4 asks for, and writes the new id in place of the old one, which no reference can name (5.3.4).
- Counted ids are stable: a runtime **MUST NOT** change one for any other reason.

### 5.4 Steps that change two documents

A step that changes two documents at once (a two-way reference between two files, a renamed file that others name) **MUST** be one undoable step in the host, whose undo restores both bodies, and which writes neither when either edit is refused. FBL keeps one history per body (FBL section 7.1); making the two one step is the host's.

### 5.5 Settings in the registration *(DESL 0.2)*

`persistence.settings` maps a block of a document's registration to part of the model, so that a designer type keeps beside the body what the body's readers do not need. Each key is the name of a block that every binding of `persistence.bindings` declares in `registration.blocks` (FBL section 8.1), and each value a **Setting** (`$defs/Setting`) in one of two forms:

| Form | Properties | Meaning |
|------|------------|---------|
| A list | `type`, `key`, `value`, `initial` | The elements of `type` are the block's entries, in the block's order: the attribute `key` is an entry's key and the attribute `value` its value. `initial` lists the elements, by their attribute values, that a document has while the block is absent. |
| A state | `attribute`, `of` | The attribute `attribute` of the elements of the types `of` is kept in the block, keyed by the element's id. Only a value that differs from the attribute's default is stored. |

- **Values.** A value is kept as its written form: a string as itself, an enumeration value as its stored form (4.4), a number as JSON writes it, and a bool as `yes` or `no`. FBL refuses a value that is empty or holds `: ` (FBL section 8.4), so the attribute's facets **SHOULD** rule such values out.
- **One transaction.** A change to such an element or attribute is planned as an edit of the registration, in the same transaction as any change of the body it is made with (FBL section 8.4).
- **Absent or complete.** A list's block is absent, and the document has the `initial` elements, or it holds every element of the type: the first change writes the whole list. A state's block holds the values that differ from the default, and none other.
- **Ids.** An element of a list's type has a derived id made from its key (5.3.2), so `persistence.ids.types` **MUST** give the type a `derived` rule. A state's block keys an element by its id, so the binding declares it with `keys: "ids"`, and an element whose id is ephemeral (5.3.3) keeps its value for the session only.
- **Actions.** The ordinary actions change these elements: `create`, `set`, `delete` and `reorder` (8.2). No second set of actions exists for them.
- **Without the registration** a document has the `initial` elements of each list and the default of each state, and works as it does with one: deleting the registration loses only what its blocks held.
- The elements of a list's type are the document's own (they have no parent) and stand in the block's order. A list's binding block is declared with `order: "kept"`, so that a `reorder` keeps its place.

---

## 6. Constraints and findings

Constraints say what makes a document valid. They are written in CEL, evaluated by the runtime and by headless validators, and reported to the user as findings. A document with findings is still opened, shown and saved.

### 6.1 Overview

| Property      | Type | Description |
|---------------|------|-------------|
| `rules`       | Constraint[] | The constraints, in order. |
| `defaults`    | `{severity, timing}` | Defaults for every rule: a severity, and when rules are evaluated, from `"live"` (after every transaction), `"save"` (before a document is written) and `"explicit"` (when the user asks). Default severity `"error"`, timing `["live", "save"]`. |
| `blockSaveOn` | `"never"`, `"error"` | Whether a document with errors may not be saved. Default `"never"`. |
| `doc`         | Doc | |

### 6.2 Constraint object

| Property      | Type | Description |
|---------------|------|-------------|
| `id`          | identifier | **Required**, unique. |
| `code`        | qualified identifier | The code every finding of the rule reports (`knowledge.no-title`). |
| `label`, `doc`| | Its name and explanation, shown with every finding. |
| `scope`       | `"document"` or TypeRefs | What the rule is evaluated for: once for the document, or once per element of the types, with `self` bound to it. Default `"document"`. |
| `when`        | Expression | The rule is evaluated only where this holds. |
| `rule`        | Expression → bool | **Required.** The condition that must hold. |
| `severity`    | `"error"`, `"warning"`, `"info"`, `"hint"`, or `{cel}` | Default from `defaults`. |
| `message`     | Message | The finding's text. |
| `target`      | Expression | The element or elements the finding is attached to. Default `self`, or the document for `scope: "document"`. |
| `enforcement` | `"report"`, `"prevent"`, `"prevent-and-report"` | `prevent`: a transaction that would make the rule false for an element it held for is rolled back, with the message as its refusal. Default `"report"`. |
| `enabled`     | bool | Default `true`. |

```json
{ "id": "severalTitles", "code": "knowledge.several-titles", "scope": ["Property"], "severity": "error",
  "rule": "self.positionIn(document.elementsOfType('Property').filter(p, p.title)) == 0",
  "message": "Only one property can be the title; this one is not used as the title." }
```

The rule holds for the first title property only, so every later one is a finding.

### 6.3 Findings

Evaluating constraints, and reading a document, produce **findings**. A finding carries:

| Property     | Type | Description |
|--------------|------|-------------|
| `constraint` | constraint id or built-in id | The rule that raised it. **Required.** |
| `code`       | qualified identifier | The rule's code, when it has one. |
| `severity`   | `"error"`, `"warning"`, `"info"`, `"hint"` | **Required.** |
| `message`    | string | The rule's Message, evaluated. **Required.** |
| `target`     | element or elements | What the finding is about; the document for a rule scoped to it. |
| `location`   | SourceLocation | Where in the file it is. |
| `subject`    | string | A display string naming something that is not an element, such as an unreadable entry. |

Every finding **MUST** carry at least one of `target`, `location` and `subject`. A runtime **MUST** show findings in a list, where choosing one reveals its element or goes to its location, and beside the element they target.

A **SourceLocation** is `{file, line?, column?, length?}`: `file` relative to the document's own folder with `/` separators, `line` and `column` 1-based, and `column` and `length` counted in Unicode code points. When a finding targets an element, its location is where the reader recorded that element.

**Order.** Findings **MUST** be listed: first those the reader raised (`std.unparseable`, `std.unreadableEntry`, `std.missingId`, `std.duplicateId`) in reading order; then the other built-ins, in the order of the table in 6.5; then declared constraints in the order of `rules`, each by its scope elements in document order.

**Parse failure.** A reader that cannot parse a body **MUST** report exactly one `std.unparseable` finding, at the parser's line and column when it has them, and no other finding located in that body; no constraint is evaluated over it. Whether the document then opens empty or read-only is FBL's (FBL section 9).

### 6.4 Gesture refusals

A constraint with enforcement `prevent` or `prevent-and-report` **MUST** refuse a transaction that would make it false for an element it held for, before anything is written, and **MUST** show its message, evaluated in the working state, as the refusal. A refused transaction changes nothing.

### 6.5 Built-in constraints

The metamodel and the reader raise these by themselves. They behave as declared constraints; their texts are worded by the runtime.

| Id                    | Raised for | Severity |
|-----------------------|------------|----------|
| `std.required`        | an attribute with `required: true` that has no value, or *(DESL 0.2)* the empty value of its type | error |
| `std.facets`          | a value outside `min`, `max`, `minLength`, `maxLength` or `pattern`, or an enumeration value no value of the enumeration has | error |
| `std.references`      | a reference that names no element | error |
| `std.containment`     | a child whose type its parent's `children`, or its slot, does not allow, or *(DESL 0.2)* an element without a parent whose type `metamodel.document.children` does not allow | error |
| `std.unparseable`     | the reader, once per body that cannot be parsed (6.3) | error |
| `std.unreadableEntry` | the reader, per entry of a parsed body that cannot become an element: an unknown key, a value that does not convert, an entry in no known form; the entry is kept as written | warning |
| `std.missingId`       | the reader or runtime, per element without a usable id, or whose derived id cannot be computed (5.3.4) | warning |
| `std.duplicateId`     | the reader or runtime, per element whose id equals that of an element earlier in reading order (5.3.4) | warning |
| `std.mentions`        | *(DESL 0.2)* a mention that names no element of the types its `mentions` facet lists (4.3) | warning |

A reader **MUST** report the reader's four as they occur, and **MUST NOT** fail to open a document because of any of them.

---

## 7. Surface

### 7.1 Overview

`surface` says how the model is laid out for the user, by naming which types and attributes of the metamodel play which part. It names no designer type of its own; a host's component for a `kind` serves all designer types of that kind.

| Property | Type | Description |
|----------|------|-------------|
| `kind`   | `"table"`, *(DESL 0.2)* `"hierarchy"` | **Required.** The layout: a table of rows and columns (7.2 to 7.6), or boxes that contain boxes (7.7). |

The other properties of `surface` are those of its kind. Sections 7.2 to 7.6 define the `table` kind, and section 7.7 the `hierarchy` kind.

### 7.2 The table

A **table** shows the elements of one type as **rows**, the elements of another type as **columns**, and at each crossing one **cell** that holds the row's value for that column. A table has **views**, elements that store how it is shown: which columns, in which order and width, sorted, filtered and grouped how.

| Property     | Type | Description |
|--------------|------|-------------|
| `columns`    | Columns | **Required.** The type whose elements are the columns (7.3). |
| `rows`       | `{ type }` | **Required.** The type whose elements are the rows, in document order unless a view sorts them. |
| `cells`      | Cells | **Required.** The type whose elements are the cells (7.3). |
| `views`      | Views | **Required.** The type whose elements are the views, and the types of their settings (7.4). |
| `valueTypes` | map SimpleId → ValueType | **Required.** One entry per value type a column may have (7.5). |

### 7.3 Columns and cells

| Columns property | Type | Description |
|------------------|------|-------------|
| `type`      | TypeRef | **Required.** The columns' element type. Document order is column order. |
| `name`      | attribute | **Required.** The attribute that names a column. |
| `valueType` | attribute | **Required.** The attribute whose value, a key of `valueTypes`, is the column's value type. |
| `title`     | attribute | The boolean attribute that marks the one column that titles a row. |
| `options`   | `{ type, name, colour }` | The element type contained in a column that lists the values a choice may take, with the attributes that name and colour one. |
| `parent`    | attribute | The boolean attribute that marks a column whose values are the row's parent within the same document, for nesting. |

| Cells property | Type | Description |
|----------------|------|-------------|
| `type`   | TypeRef | **Required.** The cells' element type, contained in a row. |
| `column` | attribute | **Required.** The reference attribute that names the cell's column. |
| `items`  | TypeRef | The element type contained in a cell that holds one of several values. |

A row has at most one cell per column. A row without a cell for a column has no value for it, which is shown empty. A cell **MUST NOT** carry an id of its own beyond what its row and column give it.

### 7.4 Views

| Views property | Type | Description |
|----------------|------|-------------|
| `type`     | TypeRef | **Required.** The views' element type. Document order is the order of the view tabs. |
| `name`     | attribute | **Required.** |
| `active`   | attribute | An attribute of the document (`metamodel.document`) naming the view the document was last left in; absent, the first view. |
| `settings` | map role → TypeRef | The element types contained in a view that store its settings, by role: `columns` (a column's visibility, width and wrap), `sorts`, `conditions`, `filterGroups` and `groups` (the order, hidden state and collapsed state of a group). |

A table **MUST** always have at least one view. A view stores only what differs from the default: a column with no settings is visible, at the default width, after the columns that have settings, in column order.

### 7.5 Value types

A **value type** says what a column's cells hold and how the surface treats it:

| Property      | Type | Description |
|---------------|------|-------------|
| `label`       | LocalizedText | **Required.** |
| `key`         | attribute | **Required.** The attribute of the cells' type that holds the value or, with `many`, of the items' type that holds one value. A cell holds its value under the key of its column's type, so a value that does not fit its column's type is still readable and is reported, never discarded. |
| `many`        | bool | The cell holds any number of values as items (7.3). Default false. |
| `editor`      | `text`, `number`, `checkbox`, `date`, `dateTime`, `time`, `choice`, `choices`, `rows` | **Required.** The editor a host opens over a cell: free text, a number, a checkbox, a date, a date and time, a time, one option, several options, or rows of a document, searched by title. |
| `comparisons` | string[] | **Required.** The comparisons a filter condition on this type offers, by the operator stored in the condition. `is-empty` and `is-not-empty` are offered for every type and **MUST NOT** be listed. |
| `sort`        | `text`, `number`, `false-first`, `chronological`, `option-order`, `title` | **Required.** How values compare when sorted: by text ignoring case, numerically, unchecked first, by time, by the options' order (of the first value, for several), or by the first related row's title. Empty values sort last in both directions. |
| `converts`    | map value type → conversion | What a value becomes when its column's type changes to another (7.6). |

### 7.6 Conversions

| Conversion | From → to | Result |
|------------|-----------|--------|
| `written-form` | any → a text type | The value's written form: the text a cell shows. |
| `parse` | text → number, checkbox, date, date and time, time | The value parsed in its target type's form (a number as JSON writes one, `true` or `false`, ISO 8601); a text that does not parse is not converted. |
| `match-option` | text → a choice | The option whose name equals the text, ignoring case, created when there is none. |
| `wrap` | one value → several | The one value as the only item. |
| `unwrap` | several → one value | The only item, when the cell holds exactly one. |

A change of a column's value type **MUST** convert every cell whose value the change's conversion converts, in the same step. A value with no conversion to the new type, or one its conversion cannot convert, **MUST** be kept as it is, under its old key, and reported as a finding on its cell. No value is discarded by a change of type.

### 7.7 The hierarchy *(DESL 0.2)*

A **hierarchy** shows **levels**, elements drawn as boxes that contain boxes, and **entries**, elements a level holds that contain nothing. Labels name members of lists kept per document, notes stand at the level or entry they are about, and stored filters narrow and sort the page. Its roles name the metamodel's types and attributes:

| Property    | Type | Description |
|-------------|------|-------------|
| `levels`    | `{type, title, description}` | **Required.** The levels' element type, which contains itself and the entries' type, and its `string` title and `text` description attributes. |
| `entries`   | `{type, title, description}` | **Required.** The entries' element type, and its title and description attributes. |
| `labels`    | map attribute name → `{on, options, prefix}` | Attributes whose values name members of a list: `on`, the types (of levels and entries) the attribute is on; `options`, `{type, name, colour}`, the element type whose elements are the list's members, with the attributes that name and colour one; `prefix`, what a filter's word for one of its members starts with, before the member's name (7.7.3), by default the attribute's name and `:`. |
| `notes`     | `{type, about, kind, text, detail, resolved}` | The notes' element type: `about`, a `string` attribute with a `mentions` facet (4.3) naming the level or entry a note is about; `kind`, an enumeration whose values carry icons (4.4); `text` and `detail`, its text; `resolved`, a bool. |
| `collapsed` | attribute name | The `bool` attribute, on the levels' and the entries' types, that folds a level or an entry. |
| `filters`   | `{type, name, query}` | The stored filters' element type, with the attributes that name one and hold its query (7.7.3). |
| `ids`       | `"shown"` (default), `"hidden"` | Whether an element's id is shown beside its title. |

#### 7.7.1 Layout

Every host draws the same page:

- One page, which scrolls from top to bottom, with no second pane beside it.
- A level is a box. Its entries, then its levels, stand to its right, each in the order of the document, and the box is as high as what stands to its right. Neighbouring boxes share their borders. A level's top-level elements stand one below the other at the left edge of the page.
- Each level and entry shows its title, its id when `ids` is `shown`, and its description when it has one, under the title. A mention in a description is drawn as a box holding the id and the current title of the element it names, and a broken one as broken.
- A collapsed level or entry shows its id, its title, its labels and its notes icon only.
- **Focus.** The page shows as many levels as fit its width, marks a level whose deeper levels do not fit, and lets the user make any level the left edge of the page, with a trail of the levels above it that leads back to each of them. Focus is the viewer's and is not stored.
- **Notes.** A level or entry shows, at its top right, the icon of the kind of its one unresolved note, an icon that stands for several when it has more, and a way to add the first when it has none. A collapsed level's icon also stands for the unresolved notes of everything inside it. Choosing the icon opens the notes it stands for as a list.

How a host draws a label, a menu or a list is not DESL's: a designer type's companion document names the looks.

#### 7.7.2 Labels and implicit members

A label attribute is a `string`, or a `string` with `many`, whose value names a member of its list by the member's `name`, ignoring case. A level's or entry's labels are drawn below its title, each in its member's colour.

A value that names no member is an **implicit member** of the list all the same: an element of the options type, with the id its derived rule gives it, standing after the stored members in the order of its first appearance in the document, with the colour attribute's default, and spelt as it first appears. Every operation on members serves it. Such a value is never reported. When a list is written (5.5), its implicit members become stored members, after the others, in their order.

Renaming a member **MUST** rename every value that names it in the same transaction, and deleting one **MUST** remove every such value; a specification states both with its operations.

#### 7.7.3 Filters

A filter's `query` is words separated by spaces:

| Word | Selects |
|------|---------|
| `<prefix><name>` | Entries whose label attribute with that `prefix` names that member, ignoring case: `#security` with the prefix `#`, `category:functional` with the default prefix of `category`. |
| `item:pattern` | Levels whose title matches the pattern, with everything in them. `*` is any run of characters and `?` one character, in any case; the pattern holds no space. |
| `sort:key` | The order of the entries within each level: `number` (by id), `title`, or the name of a label attribute, ascending; a label sorts by its member's place in its list, and an entry without it stands last. Of several `sort:` words the first counts. |

Words of one kind are alternatives, and kinds combine. A level is shown when an `item:` word selects it or one above it, or no `item:` word is given, and it holds an entry that is shown or no entry word is given. Narrowing and sorting change nothing in the document. A stored filter is an element of the `filters` type, which `persistence.settings` keeps in the registration (5.5).

---

## 8. Operations

### 8.1 Operations

An **Operation** (`$defs/Operation`) is a named command that one gesture of the surface runs. A designer **SHOULD** declare one operation per gesture, so that every host makes the same model changes for the same gesture; which gesture starts each is the host's, and the specification's prose beside the `.des` names them.

| Property      | Type | Description |
|---------------|------|-------------|
| `label`       | Message | **Required.** Menu and button text; context `operation` without `p`. |
| `doc`         | Doc | |
| `for`         | TypeRefs, `"selection"` or `"document"` | **Required.** What the operation applies to: an element of the types (bound as `self`), the selected elements (bound as `selection`), or the document. |
| `params`      | map → Attribute | Its parameters, given by the gesture, as `p` in its expressions. |
| `unavailable` | Reason[] | Why it cannot run now; context `operation` without `p`. While one applies, the gesture is disabled with that reason, and refused with it when it is invoked anyway. |
| `confirm`     | Message or Confirmation | The question asked before it runs (8.3). |
| `actions`     | Action[] | **Required.** Its body, run as one transaction (10.3). |

### 8.2 Actions

Actions are a small, closed set of steps. **Every value in an action is an Expression** (2.5).

| Action    | Form | Effect |
|-----------|------|--------|
| `set`     | `{ "set": { "attr": expr, … }, "target": expr }` | Assign attributes of `target` (default `self`); `target` may be `document`. |
| `unset`   | `{ "unset": ["attr", …], "target": expr }` | Remove stored values. *(DESL 0.2)* Planned as a set of each attribute to no value (FBL section 11.2). |
| `create`  | `{ "create": { "type": expr, "attributes": {…}, "parent": expr, "slot": expr, "after": expr, "before": expr }, "as": "name" }` | Create an element, in `parent` (default: the document) and its `slot`; the new element is `name` in later actions. |
| `delete`  | `{ "delete": expr }` | Delete an element or a list of them, with what they contain. |
| `reorder` | `{ "reorder": { "target": expr, "by": expr } }`, or with `after` or `before` in place of `by` | Change only an element's place among its siblings: `by` a signed number of places, or directly after or before a sibling. |
| `move`    | `{ "move": { "target": expr, "parent": expr, "slot": expr, "after": expr, "before": expr } }` | *(DESL 0.2)* Move `target` (default `self`), with everything it contains, into `parent` (`null`: among the document's own elements) and its `slot`, directly after or before a sibling there, else last. |
| `let`     | `{ "let": { "name": expr } }` | Bind a name for later actions. |
| `if`      | `{ "if": expr, "then": [ … ], "else": [ … ] }` | Run one list or the other. |
| `forEach` | `{ "forEach": expr, "as": "name", "do": [ … ] }` | Run a list for each item of a list, bound as `name` (default `item`). |

Every action **MAY** carry `when`, which skips it unless true, and `doc`.

- **Order of evaluation.** Actions **MUST** run in order against the transaction's working state, and every expression, `when` included, **MUST** see the effects of every action run before it, including earlier iterations of an enclosing `forEach`. The list of a `forEach` is evaluated once, before its first iteration; a `let` value once, where it appears.
- **Positional create.** `after` and `before` (at most one) name a sibling. The new element **MUST** be placed directly after or before it, and the sibling **MUST** have the same parent (or both have none); otherwise the transaction is rolled back with an error. The parent's `children` **MUST** be `ordered`. Without either, a new child is appended last.
- **Reorder.** A `reorder` **MUST** change only the order among siblings. A `by` that would move an element past the first or last place rolls the transaction back.
- **Move** *(DESL 0.2)*. A `move` changes an element's parent, slot and place and nothing else: the element keeps its id, its attributes and everything it contains. `after` and `before` (at most one) name a sibling in the new parent, as for a positional create. A move into the element itself or into something it contains, or into a parent whose `children`, or slot, does not allow the element's type, rolls the transaction back. It is planned as FBL's `move` change (FBL sections 6.4, 11.2), so the element's bytes travel unchanged, re-indented in the `bullets` family. A `move` whose parent and slot are the element's own is planned as a `reorder`.

```json
"moveProperty": { "label": "Move", "for": ["Property"],
  "params": { "after": { "type": "Property" } },
  "actions": [
    { "reorder": { "after": "p.after" }, "when": "has(p.after)" },
    { "reorder": { "before": "document.elementsOfType('Property')[0]" }, "when": "!has(p.after)" } ] }
```

Dragging a column header to the far left moves its property before the first one; anywhere else, after the property it was dropped behind.

### 8.3 Confirmation

`confirm` is a Message, which asks every time, or a **Confirmation** (`$defs/Confirmation`):

| Property    | Type | Description |
|-------------|------|-------------|
| `message`   | Message | **Required.** The question; it may use `count`. |
| `when`      | Expression → bool | Ask only while it holds. Default `true`. |
| `count`     | Expression → int | A number the question is about, bound as `count`. |
| `threshold` | int | Ask only when `count >= threshold`. Default `0`. |
| `doc`       | Doc | |

A runtime **MUST** ask before the transaction when `when` holds and, with `count`, `count >= threshold`; otherwise it **MUST** proceed without asking. Its expressions are evaluated in the `operation` context, with `p` once the parameters are known.

---

## 9. The CEL environment

### 9.1 Base language

DESL uses CEL as specified at https://github.com/google/cel-spec, with these standard extensions **REQUIRED**: **strings** (`lowerAscii`, `upperAscii`, `trim`, `split`, `join`, `replace`, `indexOf`, `substring`, `format`), **lists** (`distinct`, `flatten`, `range`, `slice`, `sort`, `sortBy`), **optional types** (`?.`, `.?`, `orValue`) and the `cel.bind` macro. The macros `all`, `exists`, `exists_one`, `map`, `filter` and `has` are available as in core CEL. Strings order by Unicode code point, and sorts are stable.

### 9.2 Types

| CEL type | Members |
|----------|---------|
| `Element` | `id` (string), `type` (string), every attribute as a field (typed by the metamodel; a reference resolves to an `Element` or `null`), `isA(typeName) → bool`, `parent → Element?` (`null` for the document's own elements), `slot → string` (the slot it is kept in, `''` without one), `children → list(Element)` (in order), `childrenOfType(t) → list(Element)`, `ancestors() → list(Element)` (the parent first), `ancestorsOfType(t) → list(Element)` (in the same order), `positionIn(list) → int` (9.4), `label() → string` (the value of its `labelAttribute`). |
| `Document` | `elements → list(Element)` (every element, in document order), `elementsOfType(t) → list(Element)`, `elementById(id) → Element?`, and the document's attributes as fields. |

Attribute types map to CEL types as section 4.2 says; enumeration values are strings, their keys; `many` attributes are lists.

### 9.3 Contexts

The context of an expression determines its variables. A validator type-checks each expression in exactly one context.

| Context      | Used by | Variables |
|--------------|---------|-----------|
| `constraint` | a constraint's `when`, `rule`, `severity`, `message` and `target` | `self` (an element, or the document for `scope: "document"`), `document` |
| `identity`   | the `expression` and `ephemeral` of an id rule (5.3.2) | `self`, `document`; `self.id` is not readable, and the ids readable are those of `self`'s ancestors |
| `operation`  | an operation's `label`, `unavailable`, `confirm` and actions | `self` (for `for` naming types) or `selection` (for `"selection"`), `document`, `p` (not in `label` and `unavailable`), `count` in a confirmation, and the names `create … as`, `let` and `forEach … as` bind |
| `function`   | a function's body | its parameters, and `document` when its `uses` lists it |

### 9.4 Function library

Besides the members of 9.2, these are available in every context:

| Function | Description |
|----------|-------------|
| `e.positionIn(l) → int` | The zero-based position of element `e` in list `l`, comparing elements by identity, never by id, or −1. Lists of elements are in reading order (10.2), so `positionIn` works while ids are being computed and when ids are duplicated: `self.positionIn(group) == 0` holds for the first of a group only. |
| `uniqueName(name, names) → string` | `name` when no string of `names` equals it ignoring case; otherwise `name + ' ' + string(n)` for the smallest `n` ≥ 2 that no string of `names` equals ignoring case. |
| `l.isUnique() → bool` | All items of the list differ. |

Implementations **MUST** provide cost estimates for these functions.

### 9.5 Determinism

Expressions in the `constraint` and `identity` contexts **MUST** be deterministic given the document: they have no source of time, randomness or viewer state. Lists of elements are in document order, so results are the same in every host.

---

## 10. Processing model

### 10.1 Loading a DESL specification

1. **Parse** the JSON; reject duplicate keys.
2. **Check the version**: `desl` major version supported.
3. **Validate** against `desl.schema.json#/$defs/Specification`.
4. **Resolve names**: every type, enumeration, operation and function named exists; every type the surface names is an element type of the metamodel, and every attribute it names is an attribute of the type it is named for; `columns.options.type` and `cells.items` are among the children of the columns' and the cells' types; each value type's `key` is an attribute of the cells' type, or of the items' type with `many`; every binding in `persistence.bindings` exists, and every rule `type` in it names an element type or a key of `typeMap`. *(DESL 0.2)* For a `hierarchy` surface: every type a role names is an element type and every attribute it names an attribute of that type, the levels' type contains itself and the entries' type, each label attribute is on every type of its `on`, and `collapsed` is on the levels' and the entries' types. Every type a `mentions` facet names is an element type. A `counter` rule's `mark` is a `string` attribute of the document. Every key of `persistence.settings` is a block every binding declares in `registration.blocks`, a list's `type`, `key` and `value` are a type and its attributes, and a state's `attribute` is an attribute of each type of its `of`.
5. **Check** what the schema cannot: reserved attribute names (2.2), a positional create into an unordered parent (8.2), identity expressions that read other ids (5.3.2), functions that call themselves (3.3), and every other **MUST** of this document.
6. **Compile CEL**: parse and type-check every expression in its context (9.3), and estimate its cost against `language.limits.celCost`.

A validator reports every problem with the JSON Pointer of its location, a severity (`error` makes the specification unusable; `warning` does not) and a message.

### 10.2 Opening a document

A host opens a document as FBL section 14.2 says, with the binding of section 5.1 its claims choose, and builds the model in this order:

1. read the elements in **reading order**: the order of their first bytes in the body, which is document order; report `std.unreadableEntry`, `std.missingId` and `std.duplicateId` as they occur;
2. give an element without a usable id a new id under `missing: "assign"`, held in memory and written with the next edit, or treat it as ephemeral under `missing: "ephemeral"`, and treat every second and later holder of a duplicated id as ephemeral (5.3.4);
3. compute the derived ids (5.3.2), parents before children, letting references follow;
4. evaluate the constraints with `live` timing and lay out the surface.

Unknown content is kept: an entry the binding cannot read stays in the body, byte for byte, and is reported (6.5).

### 10.3 The editing transaction

Every gesture, operation or quick change of a cell is one **transaction**:

1. apply the operation's actions to a working state of the model (8.2);
2. recompute derived ids, and rewrite every reference to an id that changed (5.3.2);
3. check the constraints with enforcement `prevent`: a violation rolls the transaction back, and its message is the refusal (6.4);
4. plan the changes as one edit of the body, by the binding's splices (FBL section 6.4); an edit FBL refuses rolls the transaction back with FBL's refusal;
5. commit, as one undo step, and evaluate the `live` constraints.

Transactions are atomic: all of their effects apply or none. Undo and redo restore the body as it was before and after the step, and the model is read from it again. A step that changes two documents is one transaction over both (5.4).

### 10.4 Saving

A body is written by FBL's splices (FBL section 6.6), keeping every byte the transaction did not change. With `blockSaveOn: "error"`, a host **MUST** ask before writing a document that has errors, and **MUST** still let it be written.

---

## 11. Conformance

| Class | Requirements |
|---|---|
| **DESL specification** | Validates against `$defs/Specification` and passes the checks of section 10.1 without errors. |
| **Validator** | Implements section 10.1 and reports problems with JSON Pointers. |
| **Host** | Reads a DESL specification, opens its documents through FBL as a declared host of the bindings' families (FBL section 15.1), keeps the constraints of section 6, lays out the surface of section 7 for each `kind` it supports (`table`, *(DESL 0.2)* `hierarchy`), runs the operations of section 8 as the transactions of section 10.3, and reads every DED definition of the type (DED section 4). It states which kinds it supports; a document of a kind it does not support opens read-only, if at all, with the reason. |

---

## Appendix A — JSON Schema

The normative schema is [`desl.schema.json`](desl.schema.json) beside this document (JSON Schema draft 2020-12, `$id` `https://etalii.net/adp/desl/schema/0.1/desl.schema.json`, kept at the address 0.1 gave it while DESL is a draft; it accepts `"desl": "0.1"` and `"0.2"`). It defines every construct DESL uses in its own `$defs`: `Specification`, `Language`, `Function`, `Metamodel`, `ElementType`, `Containment`, `Attribute`, `Enum`, `EnumValue`, `Persistence`, `TypeMapEntry`, `IdStrategy`, `IdRule`, `Constraints`, `Constraint`, `Operation`, `Confirmation`, `Action`, `Surface` (one shape per `kind`: *(DESL 0.2)* `Table` or `Hierarchy`), `Columns`, `Cells`, `Views`, `ValueType`, `Conversion`, *(DESL 0.2)* `Mentions`, `Setting`, `HierarchyMembers`, `Labels`, `Notes`, `Filters`, `SimpleId`, `QualifiedId`, `TypeRef`, `TypeRefs`, `SemVer`, `LocalizedText`, `Message`, `Reason`, `Doc`, `Expression`, `CelValue` and `CelSource`. The one schema it references is FBL's, for `BindingRef`. `python .github/scripts/validate-examples.py` validates every `*.des` against it and performs the name checks of section 10.1, step 4.
