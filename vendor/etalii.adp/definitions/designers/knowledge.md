# Knowledge designer

[knowledge.des](knowledge.des) specifies ADP's **Knowledge designer** (`etalii/knowledge`) in DESL 0.5 (DESL 0.1 until spec 089 gave it links): its metamodel, its three FBL bindings, its ids, its constraints, its table surface with the value types and their conversions, and one operation per gesture. This file holds what DESL cannot say: the file in each of its three formats, complete enough to write one by hand; the layout and behaviour taken from Notion and where they differ; the conversion table for a change of type; the steps that write two files and the conversion of relations into links (spec 089); entries without an id; and the findings.

**These definition files are authoritative.** The standalone host's specification `knowledge-designer` (in etalii.adp.ide.standalone, `.spec-workflow/specs/knowledge-designer/`) asked for them, and etalii.adp's [spec 013](../../specs/013-knowledge-designer/) delivered them; where either and these files disagree, these files win and the other is amended (standalone Requirement 1.7). Every other host builds the same designer from these files.

## Contents

1. [The two files](#the-two-files)
2. [The table](#the-table)
3. [The file in each format](#the-file-in-each-format)
4. [Values](#values)
5. [Views](#views)
6. [Layout and behaviour](#layout-and-behaviour)
7. [Findings](#findings)
8. [Changing a property's type](#changing-a-propertys-type)
9. [Relations and the steps that write two files](#relations-and-the-steps-that-write-two-files)
10. [Entries without an id](#entries-without-an-id)
11. [Gestures and operations](#gestures-and-operations)
12. [What DESL 0.1 cannot express](#what-desl-01-cannot-express)
13. [Complete examples](#complete-examples)

## The two files

A knowledge document is exactly two files: the **knowledge file**, which holds everything, and its **registration** beside it, which only identifies it. The knowledge file is a DED 0.3 definition ([DED](../../specifications/ded/DED-specification.md)): it begins with DED's envelope, which names DED's version, the designer type `etalii/knowledge` and the file's **document id** (`id`), by which links in other documents find it (FBL section 17.2). A file made before links is a DED 0.1 definition without an id; it is read as it is and converted once (section 9).

- The registration is an `.adp` file whose first line is `etalii/knowledge`, followed by `body: <file name>` when the names differ (FBL section 8). It holds nothing else: no property, row, view, width or position. Deleting it and creating it again changes nothing; the bindings set `createOnFirstPlacement: false` and store no layout.
- The knowledge file is YAML (`.yaml`, `.yml`), JSON (`.json`, or DED's own `.ded`) or XML (`.xml`). Each extension is claimed `shared`, with DED's envelope as its marker (DED section 4): a file is a knowledge file only when its envelope names `etalii/knowledge`, so ordinary YAML, JSON and XML files are never taken for one, and a knowledge file is recognised without its registration.
- The format is chosen when the document is added, from the three bindings' templates, and is never changed afterwards (DESL section 5): a change of format would rewrite every byte.
- A new document is one title property named `Name`, one view named `Table` and no rows; its document id and its ids are new, 8 characters of base 36 each (spec 089, FR-001). The ShortGuids of 25 characters that earlier versions wrote are kept as written.

## The table

The model is the same in every format; only the writing differs. Names in brackets are the metamodel's where they differ from the file's key, because DESL reserves `type` and `parent` (DESL section 2.2) and `targetFile` never reads as the `target` of an action (research R4).

| Entry | Holds | Id |
|---|---|---|
| The root | DED's envelope, `ded` (`0.3`; `0.1` in a file made before links), `designer` (`etalii/knowledge`) and `id` (documentId, the document id), then `name`, `activeView` (a view's id) | the document id |
| `properties` | One entry per property, in column order: `id`, `name`, `type` (valueType), `title`, `target` (targetFile), `limit`, `backlinkName`, `parent` (isParent), and `options` for a selection | stored |
| `properties/*/options` | One entry per option, in order: `id`, `name`, `colour` | stored |
| `views` | One entry per view, in tab order: `id`, `name`, `filterMatch`, `columns`, `sorts`, `filter`, `groupBy`, `hideEmptyGroups`, `freeze`, `groupOrder`, `hiddenGroups`, `collapsed` | stored |
| `rows` | One entry per row: `id`, `cells` | stored |
| `rows/*/cells` | One entry per value: `property` and the value under its type's key | derived: `<row>/<property>` |
| `rows/*/cells/*/options`, `rows/*/cells/*/rows` | One entry per value of several: `option` or `row` | derived: `<cell>/<value>` |

Only `id`, `name` and `type` of a property, `id` and `name` of an option and of a view, and `id` of a row are required. Every other key is written only when it differs from its default: `title`, `parent`, `wrap` and `hideEmptyGroups` are `false`, `visible` is `true`, `limit` is `none`, `colour` is `default`, `direction` is `ascending`, `filterMatch` is `all`.

- **A cell is an entry of its own** naming its property by id, so one unreadable value costs one cell and never its row, and nothing in the file is keyed by a name the file defines (standalone design, L1).
- **A value sits under a key named for its type** (section 4), so the file says what each value is without looking up its property, and a value that no longer fits its property's type stays readable.
- **Several values are several entries** (`options:` and `rows:` lists of `option:` or `row:` items), each added, removed and moved by its own splice in all three formats (L2).
- **Everything that names a property, an option, a view or a row names it by id.** Renaming changes one place. A removed property or option takes the cells, items and view settings naming it with it (`by: "id"`, FBL section 5.7).
- **Keys and elements the designer does not know are kept**, byte for byte, through every edit, and reported (`knowledge.unknown-key`, info). Comments are kept too.
- [knowledge.schema.json](knowledge.schema.json) is the JSON Schema of the YAML and JSON forms. It refuses unknown keys, which the designer keeps but reports; what it cannot check (that a cell names a property of the table, one title) are the constraints of `knowledge.des`.

## The file in each format

All three are written with CRLF line endings and two-space indentation when ADP creates them; a file written otherwise keeps its own style, and new entries follow the style of their siblings (FBL section 6.3).

**YAML.** A mapping with the envelope first and the root keys above, then `properties:`, `views:` and `rows:` as block sequences of mappings. A string that would read as something else (a date, a time, a number, `true`) is double-quoted; dates and times are always strings. Keys of a new entry are written in the order of the table above.

**JSON.** One object with the same keys. ADP writes each property, view and row as one line, with its options, settings and cells inside it on that line, so a row is one line of the file and a diff shows one line per changed row. A list written on one line is created with its first item and removed with its last (FBL section 6.2, 0.3). Comments are not allowed in JSON, so a JSON knowledge file has none.

**XML.** The root element is DED's, `<ded version="0.3" designer="etalii/knowledge" id="…" name="…" activeView="…">`, and holds `<properties>`, `<views>` and `<rows>`. Every key becomes an attribute; every list becomes repeated child elements, named in the singular:

| YAML and JSON | XML |
|---|---|
| `properties: [ {…} ]` | `<properties><property …/></properties>` |
| a property's `options: [ {…} ]` | `<option …/>` children of `<property>` |
| a view's `columns`, `sorts` | `<column …/>`, `<sort …/>` children of `<view>` |
| a view's `filter`: conditions and groups | `<condition …/>` and `<filterGroup match="…">` children of `<view>`; a group's conditions are its children |
| a view's `groupOrder`, `hiddenGroups`, `collapsed` | `<groupOrder key="…"/>`, `<hiddenGroup key="…"/>`, `<collapsed key="…"/>` children of `<view>` |
| `rows: [ { id, cells } ]` | `<rows><row id="…"><cell …/></row></rows>` |
| a cell's `options: [ {option} ]`, `rows: [ {row} ]` | `<item option="…"/>`, `<item row="…"/>` children of `<cell>` |

In XML the order among a view's children of different kinds carries no meaning; the order among children of one kind does. An element that loses its last child closes itself again (`<view …/>`), and its first child opens it. Booleans are `true` and `false`, numbers are written as JSON writes them, and the five predefined entities escape what must be escaped.

The three are equivalent: the same table written in each reads as the same elements with the same attributes and the same findings. The fixture `specifications/fbl/fixtures/knowledge-equivalence` reads the examples below in all three, and `knowledge-yaml`, `knowledge-json` and `knowledge-xml` take one table through every kind of edit with the exact bytes each writes.

## Values

| Type (`type`) | Cell key | Written as | Editor | Comparisons | Sort |
|---|---|---|---|---|---|
| Text (`text`) | `text` | a string | free text | is, is-not, contains, does-not-contain, starts-with, ends-with | by text, ignoring case |
| Number (`number`) | `number` | a number, in its shortest form | a number | equals, does-not-equal, greater-than, less-than, at-least, at-most | numeric |
| Checkbox (`checkbox`) | `checked` | `true` or `false` | a checkbox | is-checked, is-not-checked | unchecked first |
| Date (`date`) | `date` | `YYYY-MM-DD` | a date, typed or picked | is, is-before, is-after, is-on-or-before, is-on-or-after | chronological |
| Date and time (`dateTime`) | `dateTime` | ISO 8601 with its offset, as entered (`2026-10-08T09:30:00+02:00`, `…Z`) | a date and a time | as date | chronological |
| Time (`time`) | `time` | `HH:MM` or `HH:MM:SS` | a time | as date | chronological |
| Selection (`selection`) | `option` | an option's id | one option, picked or typed to create | is, is-not | the options' order |
| Multiple selection (`multipleSelection`) | `options` | a list of `option:` items | several options, picked or typed to create | contains, does-not-contain | the options' order, by the first value |
| Relation (`relation`) | `rows` | a list of `row:` items, each a link to a row of the target file (section 9) | rows of the target file, in Notion's relation picker (section 9) | contains, does-not-contain | by the first related row's title |

Every type also offers `is-empty` and `is-not-empty`. A filter condition stores its comparison as `operator` and the value it compares with under the same key a cell would use. Empty values sort last in both directions.

**Colours** are Notion's ten names: `default`, `gray`, `brown`, `orange`, `yellow`, `green`, `blue`, `purple`, `pink`, `red`. Each host draws each as one token for the light theme and one for the dark theme, with its own colours.

**The title property** is the one text property with `title: true`. Its value names a row wherever the row is shown outside its own table, as in another table's relation cell. It cannot be deleted, hidden or given another type.

## Views

A file has at least one view. A view stores only what differs from the default:

- **Columns.** A property without a `columns` entry is visible, at the default width, without wrapping and without a calculation. The listed columns come first, in their order; the others follow in property order. `width` is in pixels at 100% zoom, at least 32.
- **Calculations.** A column's `calculate` names what the line under it shows of the rows in sight, as Notion's calculations do: `count-all`, `count-values`, `count-unique`, `count-empty`, `count-not-empty`, `percent-empty` and `percent-not-empty` for every type; `earliest`, `latest` and `date-range` for dates and dates and times; `sum`, `average`, `median`, `min`, `max` and `range` for numbers. Absent, the column shows none. In `knowledge.des` these are the enum `Calculation`, whose identifiers are the same names in camel case (`countAll`); the bindings map one to the other. A calculation that does not suit its property's type, as after a change of type, shows nothing and is kept as written.
- **Frozen columns.** `freeze` names a property by id: the columns up to and including it, in the view's order, stay in sight when the table scrolls sideways. Absent, none is frozen.
- **Sorts.** In order: the first sorts first. A view with sorts does not let rows be dragged.
- **Filter.** A list of conditions and groups combined by `filterMatch`. A group (`match: all` or `any`, and `conditions`) nests to three levels counting the view's own list.
- **Grouping.** `groupBy` names a selection, multiple selection, checkbox or relation property, or the parent relation. Each value is a group, plus one for rows without a value; a row with several values appears under each. A group's **key** is the option's id, the related row's id, `true` or `false`, or `none` for rows without a value. `groupOrder` lists keys in the order shown (unlisted groups follow in their natural order), `hiddenGroups` the groups not shown, `collapsed` the groups shown collapsed, and `hideEmptyGroups` hides groups without rows. Grouped by the parent relation, rows are nested under their parents to any depth, and `collapsed` lists the parent rows whose children are hidden.
- `activeView` on the root names the view the file was last left in; absent, the first view. Switching views is an edit, saved and undoable like any other (standalone Requirement 6.10).
- Scroll position, the selection, the cell being edited and a search typed in the bar are never stored.

Group settings belong to the grouping: grouping by another property, or ungrouping, removes them. Deleting an option removes the group settings keyed by it. Deleting a property removes its columns, and with them their calculations, and a `freeze` naming it.

## Layout and behaviour

The Knowledge designer looks and works as Notion's table view does (standalone Requirement 7), drawn with the host's own theme, fonts and controls in both themes. It copies none of Notion's icons, fonts or artwork.

- **Views as tabs** above the table, with a `+` to add one. A tab's menu renames, duplicates and deletes; tabs are reordered by dragging.
- **The bar** beside the tabs: search, filter, sort, group and property visibility. Search shows only the rows with a value containing the typed text, for as long as the view is open. Under it, each sort and each filter is a removable item; clicking one opens it for editing.
- **The header row**: one header per visible column with its type's icon (`knowledge.des`, enum `ValueType`) and name. Dragging a header moves the property; dragging its right border sets the view's width. A `+` after the last column adds a property.
- **A column's menu**: rename, change type, filter by it, sort ascending, sort descending, group by it, hide in this view, wrap, freeze up to this column, insert left, insert right, duplicate and delete. A selection's menu also lists its options, to rename, recolour, reorder by dragging and delete.
- **The calculation line** under the last row of the table and of every group: under each column, its calculation or, on hover, a menu to choose one.
- **Rows** beneath, and a *new* row at the bottom of the table and of every group. A row added in a group gets the value that puts it there. A new row stays visible in the view until the view is next opened, even when the filter would hide it.
- **Cells** are edited in place, over the cell, by their type's editor. Options and relations open a searchable list that also creates: typing a name that is not an option and confirming creates the option. Selection values are rounded tags in their option's colour.
- **Relations** are created as in Notion: choose the target file (in the host's lookup, which offers only knowledge files of the workspace, *this file* first), one row or no limit, optionally show it on the target too and name that column (`backlinkName`), then confirm.
- **Keyboard**: the arrow keys move between cells, Enter starts and commits an edit, Escape cancels it, Tab moves to the next cell, and a key adds a row. Selected rows are deleted with Delete. The table exposes the roles of a grid.
- **Selection**: one or more rows; a single selected row's properties appear in the host's property grid.

**Where it differs from Notion** (standalone Requirement 7.7), and only here: properties, views and files are local and plain text; there are no pages inside rows, no people, no comments and no sharing; the property types are the nine above; only the table view exists (no board, calendar, gallery, list, timeline or chart), with no sub-groups.

## Findings

Reading never fails on content: what cannot be accepted is a finding at its place, and everything readable is shown. Nothing unreadable is removed on the next write.

| Code | Severity | Where | Stated by |
|---|---|---|---|
| `knowledge.no-title` | warning | the table | constraint `noTitle` |
| `knowledge.several-titles` | warning | the second and later title properties | constraint `severalTitles` |
| `knowledge.title-not-text` | error | the title property | constraint `titleIsText` |
| `knowledge.duplicate-property-name` | error | the second and later properties of one name, ignoring case | constraint `uniquePropertyNames` |
| `knowledge.unknown-property` | warning | a cell naming no property | constraint `unknownProperty` |
| `knowledge.value-of-another-type` | warning | a cell whose value is under another type's key | constraint `valueOfAnotherType` |
| `knowledge.unknown-option` | warning | a value naming no option of its property | constraints `unknownOption`, `unknownItemOption` |
| `knowledge.relation-over-limit` | warning | a cell holding several rows of a relation limited to one | constraint `relationOverLimit` |
| `knowledge.view-names-missing-property` | warning | a column, sort, condition, grouping or freeze naming no property; the view opens without it | constraints `viewNamesMissingProperty`, `groupByMissingProperty`, `freezeMissingProperty` |
| `knowledge.filter-too-deep` | warning | a filter group nested deeper than three levels | constraint `filterDepth` |
| `knowledge.no-view` | warning | the table; it opens as if it had one view with no settings | constraint `lastView` |
| `knowledge.unknown-type` | warning | a property whose `type` is none of the nine; its cells are shown as text and cannot be edited | the host |
| `knowledge.unresolved-target` | warning | a relation property still in the form before links whose target file is missing, unreadable or has no document id, so that it cannot be converted yet (section 9) | the host |
| `knowledge.unresolved-row` | warning | a value of such a relation naming a row that is not in the target file | the host |
| `link.malformed`, `link.broken-document`, `link.broken-element`, `link.ambiguous-document` | as FBL section 17.6 says | a relation's `target`, or a related row, that is a link and does not resolve (section 9) | the host, by FBL section 17 |
| `knowledge.parent-cycle` | error | a row that would be its own ancestor through the parent relation | the host: DESL's expressions do not recurse over values |
| `knowledge.missing-id` | info | a property, option, view or row without an id (section 10) | the host |
| `knowledge.unknown-key` | info | a key or element the designer does not know; kept | the host |
| `knowledge.newer-version` | warning | a file whose DED version (`ded`) is newer than 0.3; it opens read-only (DED section 4) | the host |

FBL's and DESL's own findings apply too (DESL section 6.5): an entry that cannot be read is `std.unreadableEntry` and keeps its bytes; a file that is not well-formed is `std.unparseable`, opens empty and read-only, and is never written; a file without DED's envelope is unreadable; two entries with one id are `std.duplicateId`.

## Changing a property's type

A change of type converts, in the same step, every value that has a meaning in the new type; a value that has none keeps its old key and bytes and is reported as `knowledge.value-of-another-type` on its cell. No value is discarded (DESL section 7.6, standalone Requirement 3.4).

| From ↓ to → | Text | Number | Checkbox | Date, date and time, time | Selection | Multiple selection | Relation |
|---|---|---|---|---|---|---|---|
| Text | | parsed | `true`, `false` | parsed as ISO 8601 | the option of that name, ignoring case, created when there is none | as selection, one item | kept |
| Number | its written form | | kept | kept | kept | kept | kept |
| Checkbox | `true`, `false` | kept | | kept | kept | kept | kept |
| Date, date and time, time | its written form | kept | kept | kept (each to another) | kept | kept | kept |
| Selection | the option's name | kept | kept | kept | | the option as the only item | kept |
| Multiple selection | the names, joined by `, ` | kept | kept | kept | the only item, when there is one | | kept |
| Relation | the related rows' titles, joined by `, ` | kept | kept | kept | kept | kept | |

*Kept* means not converted. Converting writes the new key where the binding's key order puts it and removes the old one (fixture step "A number converted to text"). Options created by a conversion are created in the same step. Changing a type away from relation keeps the property's `target`, `limit` and `backlinkName` until the type changes back or the property is deleted; the title property's type does not change.

## Relations and the steps that write two files

A relation property names its target file by a **link without an element**, `$<document-id>@<file name>` (`target`; FBL section 17.1), and holds, per cell, a **link** to each related row, `$<document-id>@<file name>#<row id>.<title property's name>=<title>`. A row is linkable (`knowledge.des`, `Row`): its element id is its id, and its name is the value of the file's title property, so the name property of a link to a row is that property's name (`Name` in the examples). A relation to the file itself is a link to its own document id. The host resolves links within the workspace, reads the target file read-only for the rows' titles, and watches it (FBL section 17.3).

- **Load.** When a knowledge file is loaded, every link it holds is checked as FBL section 17.4 says: a link whose target was renamed, moved or retitled is rewritten in place, and the edit is written with the next write of the file; a link that does not resolve keeps its text and is reported (`link.*`, section 7). Nothing is written to a target file.
- **The relation picker** is DESL's (DESL section 7.5.1) and Notion's: typing searches the target's rows by title; the rows the cell links to are chips that a click removes; picking a row adds a link to it, and with `limit: one` replaces the link the cell holds; when nothing matches, *Create* adds a row with that title to the target and the link to the cell, as one step over the two files (below).
- **Linked from.** The links that resolve to a row, from any document of the workspace (a mind map node, a timeline element, a dependency graph node or a relation cell of any knowledge file), are its backlinks (FBL section 17.5), computed and never stored. The row's menu lists them as **Linked from (N)**, each with the consuming document's name and the name of the element that holds it; activating one opens that document with that element selected. The backlink columns below are the part of them that relations name with `backlinkName`.
- **A broken link** in a cell is a chip with the name the link caches and a broken mark; its finding (`link.*`, FBL section 17.6) stands at the cell, and a click on the chip offers the finding's fixes rather than opening a target.
- **Backlinks.** A relation with `backlinkName` shows, in its target file, a read-only column of that name listing, per row, the rows of this file that link to it. It is computed from the workspace's links (FBL section 17.5) and stored in neither file, so a two-way relation is one relation: deleting it deletes its backlink column too, and the target file is never edited for it.

These steps change two files, and every host **MUST** make each one undoable step (DESL section 5.4): plan the edit of both bodies first, write neither when either edit is refused, write both, and record one history entry whose undo restores both files' bytes.

| Step | This file | The other file |
|---|---|---|
| Create a row from the relation picker | add the link to the cell | add the row, with a new id and its title |
| Delete a backlink from the target's backlink column | – (the target file is the one being edited) | remove the link from the cell that holds it |

Renaming or moving a target file needs no step: the links that name it still resolve by its document id, and each consuming file rewrites the name when it is next loaded. Deleting a row that rows of other files link to first says how many; their links are then broken (`link.broken-element`) and are not removed. A link whose target is missing is never removed by ADP (standalone Requirement 5.7).

**The conversion of a file made before links** (spec 089, FR-026, research R9). A file whose relations name their target by a path (`target: provinces.yaml`, `.` for itself), hold row ids (`row: nh`), and may carry `counterpart` and `computed`, is read as it is: `knowledge.fbl` reads both forms and writes only the new one. When such a file is loaded and is not opened read-only, the host converts it once, as one undoable step that writes this file alone:

1. **Its own id first.** A file without a document id is given one, and its `ded` becomes `0.3` (FBL section 17.4).
2. **Each relation whose target file has a document id** becomes a link: `target` becomes `$<id>@<file name>`, and each of its cells' row ids `r` becomes the link to row `r`, named by the target's title property and the row's title, or `$<id>@<file name>#r` when the target holds no such row, which is then reported as `link.broken-element`. No related row is lost: a cell holds as many links after the conversion as it held row ids before.
3. **Its counterpart's name.** A relation with `counterpart` gets `backlinkName`, the name of the property `counterpart` names in the target file, read read-only, and loses `counterpart`.
4. **A counterpart property** (`computed: true`) of this file is removed once no relation still in the old form names it as its counterpart; until then it is kept and shown as the backlink column it already was, and removed on a later load.
5. **A relation whose target has no id yet**, or is missing, is left as it is and reported (`knowledge.unresolved-target`). The target gets its id when it is itself loaded, and the relation is converted on a later load of this file. No file is written while another is loaded except for its own id, so converting never writes a second file.

The fixture [`specifications/fbl/fixtures/links-workspace/`](../../specifications/fbl/fixtures/links-workspace/) holds a file made before links, `knowledge/cities.yaml`, and the file its conversion yields, `knowledge/cities.converted`; `validate-examples.py` converts the one and compares it with the other on every pull request.

**The parent relation** is a relation to the file itself with `parent: true` and `limit: one`. A row's parent is the row its cell names; grouping a view by it nests rows under their parents, and a change that would make a row its own ancestor is refused (`knowledge.parent-cycle` when a file already holds one).

## Entries without an id

A property, option, view or row written without an id (by hand or by a language model) is read with an id the host makes up, is shown, and is reported as `knowledge.missing-id` (info). The first edit made in ADP to the file writes a new id, 8 characters of base 36, into every such entry, in the same step (DESL `ids.missing: "assign"`, DESL section 5.3.4), so references to it can be stored. Cells, items and view settings have no id of their own and never need one.

## Gestures and operations

`knowledge.des` declares one operation per gesture, so every host makes the same model changes for the same gesture. The gesture that starts each is the host's; this host-independent mapping is what the standalone host offers:

| Gesture | Operation |
|---|---|
| `+` after the last column; insert left or right | `addProperty` (`after` the column to the left) |
| Column menu: rename, change type, duplicate, delete | `renameProperty`, `changePropertyType`, `duplicateProperty`, `deleteProperty` |
| Dragging a header | `moveProperty` |
| Use as title, use as parent | `setTitleProperty`, `setParentRelation` |
| New relation | `createRelation` (with the two-file step above) |
| Typing a new option in a cell, option menu | `addOption`, `renameOption`, `recolourOption`, `moveOption`, `deleteOption` |
| *New* row, Delete on selected rows, dragging a row | `addRow`, `deleteRows`, `moveRow` |
| Editing a cell | `setText`, `setNumber`, `setChecked`, `setDate`, `setDateTime`, `setTime`, `setOption`, `addItem`, `removeItem`, `clearCell` |
| View tab: `+`, rename, duplicate, delete, drag, click | `addView`, `renameView`, `duplicateView`, `deleteView`, `moveView`, `switchView` |
| Hide, show, resize, wrap, reorder a column in a view | `setColumn`, `moveColumn`, `resetColumn` |
| The calculation line | `calculate`, `removeCalculation` |
| Column menu: freeze up to this column, unfreeze | `freeze`, `unfreeze` |
| Sort bar | `addSort`, `setSortDirection`, `moveSort`, `removeSort` |
| Filter bar | `addCondition`, `setCondition`, `removeCondition`, `addFilterGroup`, `setMatch` |
| Group menu, group headings | `groupBy`, `ungroup`, `hideEmptyGroups`, `setGroup`, `unsetGroup`, `moveGroup` |

## What DESL 0.1 cannot express

- The file's three forms and how XML writes lists (section 3): the bindings say it, DESL only names them.
- The layout and behaviour taken from Notion (section 6): DESL's `surface` names which types play which part, not how a host draws them.
- Conversions beyond their kind: what a written form is, that a multiple selection's names are joined by `, ` (section 8).
- The steps that write two files and their atomicity (section 9): DESL requires it of a host, FBL keeps one history per body.
- Relation values across files: a row of another file is a plain id here, resolved by the host, so its findings are the host's.
- A parent cycle, which needs a recursive walk over values.
- The view logic itself (how a filter, a sort and a grouping select and order rows): stated by the value types' comparisons and sorts and by section 5, and implemented by each host.

[knowledge.atd](knowledge.atd) carries this specification in ATD, with the three bindings in [knowledge.yaml.atd](knowledge.yaml.atd), [knowledge.json.atd](knowledge.json.atd) and [knowledge.xml.atd](knowledge.xml.atd), each `knowledge.fbl`'s binding of its name. Of the list above it states the three forms and how XML writes lists (its bindings), the relation values across files (a related row is a link, a plain string of the rule `cellRow`, which the host resolves by FBL section 17) and the parent cycle (the constraint `parentCycle`, whose walk up the parent relation is the function `reaches` of its script). What remains is the host's, and none of it is a gap of the language: how the table is drawn and how a view selects and orders its rows are rendering, which no case compares; the steps that write two files keep one history entry for both files, which is the host's undo stack; and the findings that need another file or the reader's own wording (`knowledge.unresolved-target`, `knowledge.unresolved-row`, `knowledge.unknown-key`, `knowledge.missing-id`, `knowledge.unknown-type`, `knowledge.newer-version`) are the host's, as section 7 says. The description writes some operations as the desktop host's module does them, which `knowledge.des` words otherwise: an unticked checkbox removes its cell, a new view is named View and leaves the file in the view it was in, and a value equal to its attribute's default is written as none. A condition's value is set with its operator (`setCondition`), and a new option typed in a cell is one operation, `setNewOption`, given the colour the host picked for it; `knowledge.des` states both as the description does (spec 076, Peter's ruling of 2026-11-07).

## Complete examples

The examples in [examples/](examples/) are one table, Cities, written in each format: cities of the Low Countries and a few beyond them. Its relation Province links to the rows of [examples/provinces.yaml](examples/provinces.yaml). Each validates against [knowledge.schema.json](knowledge.schema.json) (XML through the mapping of section 3) and reads through its binding without findings; the validator checks both on every pull request.

### YAML ([examples/cities.yaml](examples/cities.yaml))

```yaml
ded: "0.3"
designer: etalii/knowledge
id: c4m8x2tr
# Cities of the Low Countries, kept by hand and by ADP.
name: Cities
activeView: v1
properties:
  - id: p1
    name: Name
    type: text
    title: true
  - id: p2
    name: Country
    type: selection
    options:
      - id: o1
        name: Netherlands
        colour: blue
      - id: o2
        name: Belgium
        colour: yellow
      - id: b4ff5lo4rezgty4ge7siqndey
        name: Germany
      - id: dn1qx45l0fu88pip2l9ua7joz
        name: France
      - id: 7h1eyx85f462uj3sb6jlxdxvu
        name: Finland
      - id: 2xc52nefzictwc49wy8ldxh7t
        name: Italy
  - id: p3
    name: Population
    type: number
  - id: p4
    name: Tags
    type: multipleSelection
    options:
      - id: o3
        name: Port
      - id: o4
        name: Capital
        colour: red
  - id: p5
    name: Province
    type: relation
    target: $p7v3k9qd@provinces.yaml
    limit: one
  - id: p6
    name: Visited
    type: checkbox
  - id: p7
    name: Founded
    type: date
  - id: p8
    name: Updated
    type: dateTime
  - id: p9
    name: Opens
    type: time
views:
  - id: v1
    name: All cities
    columns:
      - property: p3
        width: 120
    sorts:
      - property: p3
        direction: descending
    filter:
      - property: p2
        operator: is
        option: o1
    groupBy: p2
  - id: v2
    name: Map
rows:
  - id: r1
    cells:
      - property: p1
        text: Amsterdam
      - property: p2
        option: o1
      - property: p3
        number: 931298
      - property: p4
        options:
          - option: o3
          - option: o4
      - property: p5
        rows:
          - row: $p7v3k9qd@provinces.yaml#nh.Name=Noord-Holland
      - property: p6
        checked: true
      - property: p7
        date: "1275-10-27"
      - property: p8
        dateTime: "2026-10-08T09:30:00Z"
      - property: p9
        time: 09:00
  - id: r2
    cells:
      - property: p1
        text: Antwerp
      - property: p2
        option: o2
      - property: p4
        options:
          - option: o3
  - id: 45ako65f2m2wasyxq3mam8ikt
    cells:
      - property: p2
        option: o1
      - property: p3
        number: 345345
      - property: p1
        text: Sevenum
      - property: p4
        options:
          - option: o3
  - id: a8q7ddqb6zyubcyf5vuky6iwb
    cells:
      - property: p2
        option: o1
      - property: p1
        text: Venlo
  - id: 8heh3sdyy0a63fyq7gicize5a
    cells:
      - property: p2
        option: b4ff5lo4rezgty4ge7siqndey
      - property: p1
        text: Berlin
  - id: 7pa30fmuw51bv1vyl5jnmf5jh
    cells:
      - property: p2
        option: dn1qx45l0fu88pip2l9ua7joz
      - property: p1
        text: Paris
  - id: 8rj09k224141g98x6rguz7f5h
    cells:
      - property: p2
        option: 7h1eyx85f462uj3sb6jlxdxvu
      - property: p1
        text: Helsinki
  - id: e1v5f01oi4bx1syl7x6ty3zeo
    cells:
      - property: p2
        option: 2xc52nefzictwc49wy8ldxh7t
      - property: p1
        text: Venice
  - id: 1dytemy0l8cen9xxkpcxzlgwq
    cells:
      - property: p1
        text: Ahlen
      - property: p2
        option: b4ff5lo4rezgty4ge7siqndey
```

### JSON ([examples/cities.json](examples/cities.json))

```json
{
  "ded": "0.3",
  "designer": "etalii/knowledge",
  "id": "c4m8x2tr",
  "name": "Cities",
  "activeView": "v1",
  "properties": [
    { "id": "p1", "name": "Name", "type": "text", "title": true },
    { "id": "p2", "name": "Country", "type": "selection", "options": [{ "id": "o1", "name": "Netherlands", "colour": "blue" }, { "id": "o2", "name": "Belgium", "colour": "yellow" }, { "id": "b4ff5lo4rezgty4ge7siqndey", "name": "Germany" }, { "id": "dn1qx45l0fu88pip2l9ua7joz", "name": "France" }, { "id": "7h1eyx85f462uj3sb6jlxdxvu", "name": "Finland" }, { "id": "2xc52nefzictwc49wy8ldxh7t", "name": "Italy" }] },
    { "id": "p3", "name": "Population", "type": "number" },
    { "id": "p4", "name": "Tags", "type": "multipleSelection", "options": [{ "id": "o3", "name": "Port" }, { "id": "o4", "name": "Capital", "colour": "red" }] },
    { "id": "p5", "name": "Province", "type": "relation", "target": "$p7v3k9qd@provinces.yaml", "limit": "one" },
    { "id": "p6", "name": "Visited", "type": "checkbox" },
    { "id": "p7", "name": "Founded", "type": "date" },
    { "id": "p8", "name": "Updated", "type": "dateTime" },
    { "id": "p9", "name": "Opens", "type": "time" }
  ],
  "views": [
    { "id": "v1", "name": "All cities", "columns": [{ "property": "p3", "width": 120 }], "sorts": [{ "property": "p3", "direction": "descending" }], "filter": [{ "property": "p2", "operator": "is", "option": "o1" }], "groupBy": "p2" },
    { "id": "v2", "name": "Map" }
  ],
  "rows": [
    { "id": "r1", "cells": [{ "property": "p1", "text": "Amsterdam" }, { "property": "p2", "option": "o1" }, { "property": "p3", "number": 931298 }, { "property": "p4", "options": [{ "option": "o3" }, { "option": "o4" }] }, { "property": "p5", "rows": [{ "row": "$p7v3k9qd@provinces.yaml#nh.Name=Noord-Holland" }] }, { "property": "p6", "checked": true }, { "property": "p7", "date": "1275-10-27" }, { "property": "p8", "dateTime": "2026-10-08T09:30:00Z" }, { "property": "p9", "time": "09:00" }] },
    { "id": "r2", "cells": [{ "property": "p1", "text": "Antwerp" }, { "property": "p2", "option": "o2" }, { "property": "p4", "options": [{ "option": "o3" }] }] },
    { "id": "45ako65f2m2wasyxq3mam8ikt", "cells": [{ "property": "p2", "option": "o1" }, { "property": "p3", "number": 345345 }, { "property": "p1", "text": "Sevenum" }, { "property": "p4", "options": [{ "option": "o3" }] }] },
    { "id": "a8q7ddqb6zyubcyf5vuky6iwb", "cells": [{ "property": "p2", "option": "o1" }, { "property": "p1", "text": "Venlo" }] },
    { "id": "8heh3sdyy0a63fyq7gicize5a", "cells": [{ "property": "p2", "option": "b4ff5lo4rezgty4ge7siqndey" }, { "property": "p1", "text": "Berlin" }] },
    { "id": "7pa30fmuw51bv1vyl5jnmf5jh", "cells": [{ "property": "p2", "option": "dn1qx45l0fu88pip2l9ua7joz" }, { "property": "p1", "text": "Paris" }] },
    { "id": "8rj09k224141g98x6rguz7f5h", "cells": [{ "property": "p2", "option": "7h1eyx85f462uj3sb6jlxdxvu" }, { "property": "p1", "text": "Helsinki" }] },
    { "id": "e1v5f01oi4bx1syl7x6ty3zeo", "cells": [{ "property": "p2", "option": "2xc52nefzictwc49wy8ldxh7t" }, { "property": "p1", "text": "Venice" }] },
    { "id": "1dytemy0l8cen9xxkpcxzlgwq", "cells": [{ "property": "p1", "text": "Ahlen" }, { "property": "p2", "option": "b4ff5lo4rezgty4ge7siqndey" }] }
  ]
}
```

### XML ([examples/cities.xml](examples/cities.xml))

```xml
<ded version="0.3" designer="etalii/knowledge" id="c4m8x2tr" name="Cities" activeView="v1">
  <!-- Cities of the Low Countries, kept by hand and by ADP. -->
  <properties>
    <property id="p1" name="Name" type="text" title="true"/>
    <property id="p2" name="Country" type="selection">
      <option id="o1" name="Netherlands" colour="blue"/>
      <option id="o2" name="Belgium" colour="yellow"/>
      <option id="b4ff5lo4rezgty4ge7siqndey" name="Germany"/>
      <option id="dn1qx45l0fu88pip2l9ua7joz" name="France"/>
      <option id="7h1eyx85f462uj3sb6jlxdxvu" name="Finland"/>
      <option id="2xc52nefzictwc49wy8ldxh7t" name="Italy"/>
    </property>
    <property id="p3" name="Population" type="number"/>
    <property id="p4" name="Tags" type="multipleSelection">
      <option id="o3" name="Port"/>
      <option id="o4" name="Capital" colour="red"/>
    </property>
    <property id="p5" name="Province" type="relation" target="$p7v3k9qd@provinces.yaml" limit="one"/>
    <property id="p6" name="Visited" type="checkbox"/>
    <property id="p7" name="Founded" type="date"/>
    <property id="p8" name="Updated" type="dateTime"/>
    <property id="p9" name="Opens" type="time"/>
  </properties>
  <views>
    <view id="v1" name="All cities" groupBy="p2">
      <column property="p3" width="120"/>
      <sort property="p3" direction="descending"/>
      <condition property="p2" operator="is" option="o1"/>
    </view>
    <view id="v2" name="Map"/>
  </views>
  <rows>
    <row id="r1">
      <cell property="p1" text="Amsterdam"/>
      <cell property="p2" option="o1"/>
      <cell property="p3" number="931298"/>
      <cell property="p4">
        <item option="o3"/>
        <item option="o4"/>
      </cell>
      <cell property="p5">
        <item row="$p7v3k9qd@provinces.yaml#nh.Name=Noord-Holland"/>
      </cell>
      <cell property="p6" checked="true"/>
      <cell property="p7" date="1275-10-27"/>
      <cell property="p8" dateTime="2026-10-08T09:30:00Z"/>
      <cell property="p9" time="09:00"/>
    </row>
    <row id="r2">
      <cell property="p1" text="Antwerp"/>
      <cell property="p2" option="o2"/>
      <cell property="p4">
        <item option="o3"/>
      </cell>
    </row>
    <row id="45ako65f2m2wasyxq3mam8ikt">
      <cell property="p2" option="o1"/>
      <cell property="p3" number="345345"/>
      <cell property="p1" text="Sevenum"/>
      <cell property="p4">
        <item option="o3"/>
      </cell>
    </row>
    <row id="a8q7ddqb6zyubcyf5vuky6iwb">
      <cell property="p2" option="o1"/>
      <cell property="p1" text="Venlo"/>
    </row>
    <row id="8heh3sdyy0a63fyq7gicize5a">
      <cell property="p2" option="b4ff5lo4rezgty4ge7siqndey"/>
      <cell property="p1" text="Berlin"/>
    </row>
    <row id="7pa30fmuw51bv1vyl5jnmf5jh">
      <cell property="p2" option="dn1qx45l0fu88pip2l9ua7joz"/>
      <cell property="p1" text="Paris"/>
    </row>
    <row id="8rj09k224141g98x6rguz7f5h">
      <cell property="p2" option="7h1eyx85f462uj3sb6jlxdxvu"/>
      <cell property="p1" text="Helsinki"/>
    </row>
    <row id="e1v5f01oi4bx1syl7x6ty3zeo">
      <cell property="p2" option="2xc52nefzictwc49wy8ldxh7t"/>
      <cell property="p1" text="Venice"/>
    </row>
    <row id="1dytemy0l8cen9xxkpcxzlgwq">
      <cell property="p1" text="Ahlen"/>
      <cell property="p2" option="b4ff5lo4rezgty4ge7siqndey"/>
    </row>
  </rows>
</ded>
```
