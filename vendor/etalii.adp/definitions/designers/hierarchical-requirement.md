# Hierarchical Requirement Designer

[hierarchical-requirement.des](hierarchical-requirement.des) specifies ADP's **Hierarchical Requirement Designer** (`etalii/hierarchical-requirement`) in DESL 0.2: its metamodel, its ids, what its registration keeps, its `hierarchy` surface and one operation per gesture. [hierarchical-requirement.fbl](hierarchical-requirement.fbl) is its binding `markdown`, in FBL 0.7's `bullets` family, with the template of a new document. This file holds what those two cannot say: the file complete enough to write one by hand, the numbers, the trailing words and their known limit, references, assessments, the registration and what it keeps, the JSON form the Build workflow reads a document into, the looks taken from the Knowledge designer, the gestures, and what the languages cannot state.

**These definition files are authoritative.** etalii.adp's [spec 028](../../specs/028-hierarchical-requirement-designer/) delivered them, and the standalone host builds the designer first, from them. Where a host's specification and these files disagree, these files win and the other is amended.

## Contents

1. [The two files](#the-two-files)
2. [The file](#the-file)
3. [Items and requirements](#items-and-requirements)
4. [Numbers](#numbers)
5. [References](#references)
6. [Assessments](#assessments)
7. [What reading reports](#what-reading-reports)
8. [A new document and its legend](#a-new-document-and-its-legend)
9. [The registration](#the-registration)
10. [Stored filters](#stored-filters)
11. [The JSON form](#the-json-form)
12. [Layout and looks](#layout-and-looks)
13. [Gestures and operations](#gestures-and-operations)
14. [What the languages cannot state](#what-the-languages-cannot-state)
15. [Complete example](#complete-example)

## The two files

A requirement document is one Markdown file for one subject (an application, a product, a process), and optionally its registration beside it.

- **The Markdown file holds everything an agent needs**: every item and requirement with its number, title and description, the tags it carries, a requirement's category and state, the references and the assessments. It is a DED 0.2 definition ([DED](../../specifications/ded/DED-specification.md), section 2): its line 1 is DED's envelope, which names the designer type.
- **The extension is a plain `.md`**, claimed `shared` with the envelope as its marker (DED section 4): a Markdown file is a requirement document when, and only when, its line 1 is the envelope naming `etalii/hierarchical-requirement`. It opens in the designer without a registration.
- **The registration** is an `.adp` file whose line 1 is `etalii/hierarchical-requirement`. It holds only what an agent does not need to read: the colours and order of the tag, category and state lists, what is collapsed, and the stored filters. It is created by the first such setting the person changes. Deleting it loses those and nothing else; an agent never reads or writes it.

## The file

```markdown
<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0003 REQ-0004 -->
# Web shop

## How to read this file

(the legend: prose and fenced examples, never read as entries)

## Requirements

- ITEM-0001 Backend #core
  Everything behind the public interface.
  - REQ-0001 A session ends after 30 minutes without activity. #security category:functional state:accepted
    Activity is any request the user makes. See [REQ-0002] for sign-in.
  - REQ-0002 Users sign in with an e-mail address and a password. #security
  - ITEM-0002 Sessions
    - REQ-NEW Tokens are refreshed without the user noticing.

## Assessments

- [ ] Question REQ-0001: Is 30 minutes right for the mobile app?
- [x] Warning ITEM-0002: Overlaps with the gateway's own session handling.
```

| Part | Where it stands | Who writes it |
|---|---|---|
| Envelope | Line 1, exactly `<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->`; only a byte-order mark may stand before it. | The template, once. Nobody afterwards. |
| Mark line | Line 2, `<!-- last: ITEM-0003 REQ-0004 -->`: the last item number and the last requirement number given, in that order. | The designer, with every numbering. A writer of text leaves it alone in a document that has one; one who writes a new document by hand writes it with the last number of each kind, or leaves it out. |
| Hierarchy | The bullets under the first heading whose text is `Requirements`, at any heading level (`#` to `######`), in any case, up to the next heading with as many `#` or fewer. A heading with more `#` inside it does not end it. | Both. |
| Assessments | The bullets under the first heading whose text is `Assessments`, by the same rule. | Both. |
| Everything else | The title, the legend, any other section, text between the bullets, every fenced block. | The writer. Kept byte for byte, never read as an entry and never reported. |

- A heading is a line with at most three spaces before one to six `#` and a space; a line underlined with `=` or `-` is not a heading here. Only the first heading of each text counts: do not title the document, or another section, `Requirements` or `Assessments`.
- A file with no `Requirements` heading, or with no bullet under it, is an empty document, readable and editable. The first item added appends `## Requirements` at the end of the file; the first assessment added appends `## Assessments` the same way.
- A fenced block (three or more backticks or tildes) is never read, so the legend can show example bullets.
- The designer writes CRLF in a new document. A document written by hand may use CRLF or LF, and an edit afterwards follows the file's own line endings.

## Items and requirements

Every item and every requirement is one bullet, and the nesting of the bullets is the hierarchy.

- **An item** is `- ITEM-0004 <title> <tags>`: a part of the subject. It holds requirements and further items, indented under it, to any depth.
- **A requirement** is `- REQ-0031 <title> <tags> <category> <state>`: its title states the requirement itself. It stands in an item and holds nothing.
- **The marker** is `-`, `*` or `+`, followed by a space. The designer writes a new bullet with the marker and the indentation of its previous sibling, else of its next sibling, else under its item at the item's text column.
- **Nesting** is by indentation: a bullet is nested in the nearest bullet above it whose marker stands further left. A tab counts as four columns. Indent a nested bullet two spaces deeper than the bullet it is in.
- **The description** is the lines indented under a bullet that are not bullets themselves, blank lines between them included, up to its first nested bullet. It is optional, and for a requirement it clarifies. It is read with the indentation up to the bullet's text column taken off. A description holds no list of its own: a bullet in it would be read as a nested entry.
- **The trailing words** stand at the end of the bullet's line: tags `#name`, and on a requirement one category `category:name` and one state `state:name`. The title is what stands between the number and the longest run of such words that ends the line, punctuation included: in `- REQ-0002 Users sign in. #security` the title is `Users sign in.`. They are read in any order; the designer writes the tags first, in the order they were added, then the category, then the state. On a line written in another order, a new tag goes after the last tag, wherever that stands. `category:` and `state:` are written in lower case.
- **A name** (of a tag, a category or a state) has no whitespace; a tag's name has no `#`. Typing a space in the designer writes `-`. Names compare ignoring case, and an entry keeps the spelling it was written with until the member is renamed.
- **Of several** `category:` or `state:` words on one requirement, the first counts; the others stay in the file as written, and nothing is reported. Clearing the first makes the next one count. On an item's line only `#name` words are trailing words: `category:` and `state:` there are part of the title.
- **The known limit.** A title cannot end in a word that reads as a tag, a category or a state: in `- REQ-0005 Results are ranked by #votes` the last word is read as the tag `votes`, and the title is `Results are ranked by`. Write such a word earlier in the title, or in the description. Every entry needs a title: `- ITEM-0001 #core` reads `#core` as the title.
- **Order.** The file's order is kept as written. The designer shows an item's requirements above its sub-items whatever the order, and writes a new requirement after the item's last requirement, or before its first sub-item when it has none, and a new sub-item after the item's last child.

## Numbers

| Rule | |
|---|---|
| Form | `ITEM-` or `REQ-` and four digits, more when the count needs them: after `REQ-9999` comes `REQ-10000`, and no number changes. |
| Counted apart | Items and requirements have a count each, per document: `ITEM-0003` and `REQ-0003` are different entries. |
| A new number | One more than the higher of the mark (line 2) and the highest number in the file, and line 2 is set to it in the same edit. So a number is never given again, also after its entry was deleted, in the designer or as text. |
| Never changed | A number does not change when its entry is moved, renamed, retagged or recategorised, or when anything around it changes. |
| Adding as text | Write `ITEM-NEW` or `REQ-NEW` in place of a number. The entry is shown, reported as having no number yet, and numbered with the next edit the designer writes, never when the file is only opened. |
| A number you choose | Accepted when no other entry has it, and the count continues past it. `REQ-12` is accepted as written; it is not `REQ-0012`, but its number counts. |
| Two entries with one number | The first in the file keeps it. Each later one is shown with a finding, nothing can refer to it, and only the person gives it a new number, from the finding. The designer never renumbers by itself. |
| No mark line | Reads as no mark: the count continues from the highest number in the file, and line 2 is written again with the next numbering. |
| Undo | An undone add gives its number back, since undo restores the file as it was, line 2 included. |

## References

- A reference is a number in square brackets in a description, an assessment's text or its detail: `[REQ-0002]`, `[ITEM-0004]`. It names one requirement or item of the same document. A title holds no references.
- A link with a target, `[REQ-0002](https://example.com)` or `[the provider's page](https://example.com)`, is an ordinary link.
- A reference has no kind: it is a plain link, not "depends on" or "refines".
- The designer never rewrites a reference: numbers never change, and nothing can refer to a duplicate. A reference whose entry no longer exists is kept as written, drawn as broken, and reported.
- In the designer, typing `#` in a description opens a lookup of the document's items and requirements, narrowed by number or title; picking one writes its number in square brackets. The `#` is a key, not what is written.

## Assessments

```markdown
## Assessments

- [ ] Question REQ-0001: Is 30 minutes right for the mobile app?
- [x] Warning ITEM-0002: Overlaps with the gateway's own session handling.
  The gateway's session ends first, so ours never times out alone.
- Idea ITEM-0001: Split the backend into ordering and delivery.
```

- An assessment is `- [ ] <Kind> <number>: <text>`. The kinds are `Question`, `Remark`, `Warning` and `Idea`, read in any case; a thought is a remark.
- `[x]` (or `[X]`) marks it resolved; `[ ]`, or no checkbox at all, is open. A resolved assessment stays in the file and no longer counts towards the entry's icon. An agent resolves one by writing `[x]`; in the designer resolving is a switch.
- The number names exactly one requirement or one item of the same document. An item with no requirements can carry assessments too. An entry written `ITEM-NEW` or `REQ-NEW` cannot be named yet: nothing rewrites an assessment when its entry is numbered, so write the assessment once the entry has its number. The same holds for a reference.
- Lines indented under it are its detail.
- An assessment has no number of its own and nothing refers to one.
- An assessment whose entry no longer exists is kept as written and reported.

## What reading reports

Reading never fails on a fault at one bullet: it is reported there, its bytes are kept, and every other entry stays readable and editable. Text outside the bullets is never reported.

| What is found | Finding | What happens |
|---|---|---|
| No envelope on line 1, or a file that is not UTF-8 | `std.unparseable` | The whole file is unreadable: read-only, this one finding, never written. |
| An envelope that names another designer type or a later DED version | DED's (section 4) | The envelope decides; a later version opens read-only. |
| No `Requirements` heading, or no bullet under it | none | An empty document. |
| A bullet under `Requirements` that is neither `ITEM-` nor `REQ-` and its title | `std.unreadableEntry` | Kept with everything nested in it, and not read. |
| A requirement at the top level, not in an item | `std.containment` | Kept. |
| Any bullet nested under a requirement | `std.containment` | Kept. |
| A bullet under `Assessments` in no known form | `std.unreadableEntry` | Kept. |
| `ITEM-NEW` or `REQ-NEW` | `std.missingId` (a warning) | Shown, and numbered with the next edit. |
| A number an earlier entry carries | `std.duplicateId` | Shown; nothing can refer to it. |
| A reference, or an assessment's number, that names no entry | `std.mentions` (a warning) | The text is kept. |
| An empty title | `std.required` | Kept. |
| A block of the registration the binding does not declare | `fbl.unknown-block` (information) | Kept. |

A finding stays listed when a filter hides its entry, and selecting it shows the entry. The validator in this repository reports the same things of an example document, each a failure at its line, except `std.missingId`.

## A new document and its legend

A document the designer creates is the binding's template, written with CRLF: the envelope, the mark line, the title `# <file name>`, the legend, a starting structure and an empty `Assessments` section.

- **The starting structure** is that of a generic web application: five top items, User interface, Integration, Backend, Data and Processes, with one or two requirements each, and a sub-item under two of them (Accessibility, Security). At most two levels deep and at most three requirements in an item; the person renames, moves and deletes every part of it as any other. Its numbers are written out, `ITEM-0001` to `ITEM-0007` and `REQ-0001` to `REQ-0009`, and line 2 names the last of each.
- **By hand.** A person or an agent who writes a whole new document numbers its entries from `ITEM-0001` and `REQ-0001` upwards and writes line 2 with the last of each, as the template does; entries added later go in as `ITEM-NEW` or `REQ-NEW`. Only line 1 is required. The legend is the author's: a document without it reads the same, and one that copies it may keep its fenced examples, which are never read.
- **The legend**, under `## How to read this file`, tells an agent that has never seen the tool what the two headings hold, that a number is never changed or used again, to add an entry as `ITEM-NEW` or `REQ-NEW`, that lines under a bullet describe it, the trailing words and the starting categories and states, `[REQ-0001]` to refer, the assessment form with `[x]`, and to leave the first two lines alone. Its examples stand in a fenced block, so they are never read. Its exact text is the template's in [hierarchical-requirement.fbl](hierarchical-requirement.fbl), and the [complete example](#complete-example) below carries it.

## The registration

```text
etalii/hierarchical-requirement
body: web-shop.md
tags:
  security: red
  core: default
categories:
  functional: blue
  non-functional: purple
  technical: brown
states:
  proposed: gray
  accepted: green
  rejected: red
  done: blue
collapsed:
  ITEM-0002: yes
filters:
  Open security: #security state:proposed sort:state
```

Line 1 is the origin, then `body: <file name>`, which may be left out when the registration and the document have the same base name, then the blocks, in this order. A block is its name and a colon on a line, then entries `<key>: <value>` indented two spaces; the key is everything before the last `: `, and a value is one line that holds no `: `.

| Block | Holds | Key: value | Order | In the undo history |
|---|---|---|---|---|
| `tags` | The tag list | name: colour | as the person arranged it | yes |
| `categories` | The category list | name: colour | as arranged | yes |
| `states` | The state list | name: colour | as arranged | yes |
| `collapsed` | The collapsed items and requirements | number: `yes` | in the byte order of the numbers as written, so every `ITEM-` before every `REQ-` | no: undo takes back the last change to the requirements, not the last box folded |
| `filters` | The stored filters | name: query | as arranged | yes |

- **Colours** are the Knowledge designer's, Notion's ten names: `default`, `gray`, `brown`, `orange`, `yellow`, `green`, `blue`, `purple`, `pink`, `red`.
- **A block is absent or complete.** While `categories` is absent the list is functional `blue`, non-functional `purple` and technical `brown`; while `states` is absent it is proposed `gray`, accepted `green`, rejected `red` and done `blue`. `tags` and `filters` start empty. The first change to a list writes the whole list.
- **Implicit members.** A tag, a category or a state that an entry carries and its list does not hold is a member all the same: it stands after the stored members, in the order it first appears in the file, with the colour `default`, spelt as it first appears. It is never reported, and an agent never has to touch the registration. When the list is next written, it is stored with the others.
- **What is collapsed** is stored for numbered entries only: an entry without a usable number is collapsed for the session. An entry whose number is no longer in the file is removed with the next write.
- **Deleting the registration** loses the colours, the order, what was collapsed, the stored filters, and a tag, category or state that no entry carries. It loses no tag, category, state, reference or assessment that the file holds, and the starting categories and states come back.

## Stored filters

A filter is words separated by spaces, in the file's own notation:

| Word | Selects |
|---|---|
| `#name` | Entries that carry the tag. |
| `category:name`, `state:name` | Requirements with that category or state. |
| `item:pattern` | Items whose title matches the pattern, with everything in them: `*` is any run of characters and `?` one character, in any case; the pattern has no space, so `?` or `*` stands for one. |
| `sort:key` | The order of the requirements within each item: `number`, `title`, `category` or `state`, ascending. A category or a state sorts by its place in its list, and a requirement without it stands last. Of several `sort:` words the first counts. |

Words of one kind are alternatives, and kinds combine. An item is shown when an `item:` word selects it or one above it, or none is given, and it holds an entry that is shown or no entry word is given. Narrowing, sorting, collapsing and focusing on an item never change the Markdown file; only storing a filter writes the registration.

## The JSON form

The Build workflow reads a requirement document into a JSON form and validates it against [hierarchical-requirement.schema.json](hierarchical-requirement.schema.json), which refines DED's `$defs/Definition`. The form is what reading the file means; nobody writes it.

| In the file | In the JSON form |
|---|---|
| Line 1, `ded:` and `designer:` | `ded` (`"0.2"`), `designer` (`"etalii/hierarchical-requirement"`) |
| Line 2 | `lastItem`, `lastRequirement`, when present |
| A top-level item | an entry of `items` |
| An item's sub-items, and its requirements | its own `items`, and its `requirements`, each in the order of the file |
| The number, `ITEM-NEW` or `REQ-NEW` | `id` |
| The title | `title` |
| The description | `description`: its lines joined by LF, the indentation up to the bullet's text column taken off; absent when empty |
| `#name` words | `tags`, in the order of the line |
| The first `category:` and `state:` word | `category`, `state` |
| An assessment | an entry of `assessments`: `kind` (`question`, `remark`, `warning` or `idea`), `about`, `text`, `detail` (as a description), `resolved` |

A bullet in no known form, a requirement out of place and whatever is nested in either are left out of the form and reported. The model's attribute names in `hierarchical-requirement.des` are the same, with `collapsed` on items and requirements kept in the registration and not in the form.

## Layout and looks

The page is DESL's `hierarchy` surface (DESL section 7.7.1), which every host draws alike:

- One page that scrolls from top to bottom, with no tree or second pane beside it.
- An item is a box. Its requirements, then its sub-items, stand to its right, and the box is as high as what stands to its right; neighbouring boxes share their borders. The top items stand one below the other at the left edge.
- Every item and requirement shows its number beside its title, its tags below the title, and its description under it. A reference in a description is a box holding the number and the current title of the entry it names; selecting it shows that entry, expanding what hides it. A broken one is drawn as broken.
- A control to the left of the title collapses the entry, which then shows only its number, title, tags and assessment icon.
- **Focus**: the page shows as many levels as fit its width and marks an item whose deeper levels do not fit; the person makes any item the left edge of the page, with a trail of the items above it that leads back to each of them. Focus is not stored.
- **Assessments**: at the top right of an entry, the icon of the kind of its one open assessment, an icon that stands for several when it has more, and a way to add the first when it has none. A collapsed item's icon also stands for the open assessments of everything inside it. Pressing the icon lists the assessments it stands for, each with the entry it names, to add, change, remove and resolve. The kinds' icons are in `hierarchical-requirement.des` (enumeration `AssessmentKind`): a question mark, a speech bubble, a warning sign and a light bulb.

**The looks are the Knowledge designer's** ([knowledge.md](knowledge.md), Layout and behaviour), drawn with the host's own theme, fonts and controls in both themes:

- its colours: the ten colour names, each one token for the light theme and one for the dark theme;
- its typography and spacing;
- tags drawn as rounded tags in their member's colour, as a selection value is;
- menus and lists that open as its do: the tag, category and state fields open a searchable list that also creates, so typing a name that is no member and pressing Enter creates it; a list's menu renames, recolours, reorders by dragging and deletes.

Where it differs: there is no table, no column, no view tab and no property grid of cells; what a view stores there is here a stored filter in the registration.

## Gestures and operations

`hierarchical-requirement.des` declares one operation per gesture, so every host makes the same model changes for the same gesture. Every edit is one undoable step and changes only the bytes of the entries it concerns, of the entries it numbers, and of line 2.

| Gesture | Operation | Written to |
|---|---|---|
| The control at the end of the page, or the first item of an empty document | `addTopItem` | The file |
| The control at the end of an item's content | `addRequirement`, `addItem` | The file: the bullet and line 2 |
| The key from a selected item or requirement: a new one directly below it, ready for typing, with no dialog | `addItemBelow`, `addRequirementBelow` | The file |
| Typing in a title or a description, a picked reference included | `setTitle`, `setDescription` | That entry's bytes |
| Dragging an item with everything in it, or a requirement, below another item or to another place | `moveItem`, `moveRequirement` (a reorder within one item, a move to another; no number changes) | The entry's own bytes, re-indented |
| Delete, which asks first when the entry holds anything or a reference or an assessment names it | `deleteEntry` | The file |
| The control to the left of a title | `toggleCollapsed` | The registration's `collapsed` block, outside the history |
| Typing in a tag field, then Enter; the cross on a tag | `addTag` (picks a tag of that name in any case, or creates it), `removeTag` | The file, and the `tags` block for a new tag |
| The tag list: rename, colour, drag, delete | `renameTag`, `recolourTag`, `moveTag`, `deleteTag` (a rename or a delete updates every carrier in the same step) | A rename or a delete: the file and the registration. A colour or a place: the registration only. |
| Picking a category or a state on a requirement, or clearing it | `setCategory`, `clearCategory`, `setState`, `clearState` | The file |
| The category and state lists: add, rename, colour, drag, delete | `addCategory`, `renameCategory`, `recolourCategory`, `moveCategory`, `deleteCategory`, and the same for `State` | As the tag list |
| The assessment icon and its list | `addAssessment`, `changeAssessment`, `resolveAssessment`, `removeAssessment` | The file, under `Assessments`; resolving replaces one character, except on an assessment written without a checkbox, whose line is written again with one |
| Storing the applied filter under a name; renaming or removing a stored one | `storeFilter`, `renameFilter`, `removeFilter` | The registration's `filters` block |
| Narrowing, sorting, focusing on an item, selecting a reference box | none | Nothing |
| Giving a duplicate a new number, from its finding | none: part of the `counter` strategy (DESL section 5.3.5) | The file: the new number and line 2 |

An edit made against a file that has changed since it was read is refused, and the file is read again; edits of two writers are never merged. When an agent writes the file while it is open, the designer reads it again and shows the change, keeping what was collapsed for entries matched by number, and the undo history starts afresh.

## What the languages cannot state

DESL 0.2 cannot say:

- The file format and the legend: the binding says how the file is read and written, and its template holds the legend; DESL only names the binding.
- The looks taken from the Knowledge designer: DESL's surface names which types play which part and what the page's layout is, not how a host draws a tag, a menu or a list.
- Which gesture starts which operation: the table above.

A new requirement's place, after the item's last requirement or before its first sub-item, is given by the operations as a position: FBL's `after-last` puts a new bullet after an item's last child of any kind.

## Complete example

[examples/web-shop.md](examples/web-shop.md) is a web shop three items deep, with every marker, descriptions, tags, categories and states, references and an ordinary link, a requirement added as text and not yet numbered, and an assessment of each kind, one of them resolved. Its registration is [examples/web-shop.adp](examples/web-shop.adp). The validator reads both on every pull request: the document must read with no finding but the `std.missingId` of its `REQ-NEW`.

````markdown
<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0007 REQ-0012 -->
# Web shop

## How to read this file

This file holds the requirements of one subject, kept by people and agents together. Leave its first two lines as they are.

- Under `Requirements` each bullet is an item (`ITEM-`), a part of the subject, or a requirement (`REQ-`), which states one requirement. An item holds requirements and further items, indented under it. A requirement stands in an item and holds nothing.
- A number is never changed and never used again. Add an entry with `ITEM-NEW` or `REQ-NEW` in place of a number: the designer numbers it.
- Lines indented under a bullet that are not bullets describe it. A description holds no bullets.
- A bullet's first line ends with its tags (`#name`), and a requirement's with one category (`category:name`) and one state (`state:name`) after them. A name has no spaces. A title does not end in a word that starts with `#`, `category:` or `state:`.
- The categories are functional, non-functional and technical, and the states proposed, accepted, rejected and done; others can be added.
- Refer to an entry by its number in square brackets: `[REQ-0001]`.
- Under `Assessments` each bullet is a question, remark, warning or idea about one entry, named by its number: `- [ ] Question REQ-0001: text`. Write `[x]` to resolve one. Lines indented under it are its detail.

```markdown
- ITEM-NEW Checkout #core
  What a customer does between the basket and the receipt.
  - REQ-NEW An order is confirmed by e-mail. #mail category:functional state:proposed
    Sent once the payment of [REQ-0001] succeeds.

- [ ] Question REQ-0001: Which payment methods count?
- [x] Idea ITEM-0001: Offer a guest checkout.
```

## Requirements

The requirements of the web shop, from what a customer sees to how the shop is run.

- ITEM-0001 Storefront #customer
  What a customer sees and does in the shop.
  * REQ-0001 A customer finds a product by its name or its description. #search category:functional state:accepted
  * REQ-0002 Every page loads within two seconds on a mobile connection. #performance category:non-functional state:proposed
    Measured at the 95th percentile, on a 4G connection.
  * ITEM-0002 Checkout #payment
    From the basket to the confirmation of the order.
    - REQ-0003 A customer pays by card, iDEAL or PayPal. category:functional state:accepted
      The payment provider's [hosted page](https://example.com/payments) takes the card details.
    - REQ-0004 An order is confirmed by e-mail once its payment succeeds. #mail category:functional state:done
      Sent when [REQ-0003] succeeds, through the service of [ITEM-0005].
    - ITEM-0003 Guest checkout
      + REQ-0005 A customer can order without an account. state:rejected
      + REQ-NEW A guest is offered an account after the order. #customer category:functional
- ITEM-0004 Backend #core
  Everything behind the public interface.
  - REQ-0006 A session ends after 30 minutes without activity. #security category:functional state:accepted
    Activity is any request the customer makes. See [REQ-0007] for signing in.
  - REQ-0007 Customers sign in with an e-mail address and a password. #security category:functional state:accepted
  - ITEM-0005 Mail
    - REQ-0008 Mail goes through one provider, and its outage does not stop an order. category:technical state:proposed
- ITEM-0006 Data
  - REQ-0009 Order data is kept for seven years, as tax law requires. #legal category:non-functional state:accepted
  - REQ-0010 Personal data is removed on request within 30 days. #legal #privacy category:non-functional

## Assessments

- [ ] Question REQ-0006: Is 30 minutes right for the mobile app?
- [x] Warning ITEM-0002: Overlaps with the payment provider's own session handling.
  Resolved: the provider's session ends before ours.
- [ ] remark REQ-0003: Two customers asked for Klarna.
- Idea ITEM-0006: Move orders older than a year to cheaper storage.
````

Line 2 names `ITEM-0007` and `REQ-0012`, above the highest numbers in the file: an item and two requirements were deleted, and their numbers are not given again. The next item added is `ITEM-0008`, and the `REQ-NEW` becomes `REQ-0013` with the next edit.
