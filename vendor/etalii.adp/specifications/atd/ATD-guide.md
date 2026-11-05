# Writing a tool type in ATD: a guide for tool engineers

This guide shows you how to describe a tool type in ATD, the ADP tool language, one step at a time. You write a diagram, a designer and an editor, and you learn how to check, format, validate and debug what you wrote, and how to let an agent write a description for you.

You need no earlier knowledge of ATD. You should know what a diagram is, and be able to run a command in a terminal. The normative text is [the ATD specification](ATD-specification.md); this guide is informative, and where the two disagree the specification is right. Words such as *tool type*, *binding* and *finding* mean what [the terminology](../../docs/terminology.md) says they mean.

Every complete `.atd` file printed here, at every step of its walk, checks without findings under `atd check`, and every `atd` command printed here was run, with the output shown. They were run with `atd` built from `etalii.adp.ide.desktop` at commit `81c1fdc` (`atd --version` prints `atd 1.0.0 (ATD 0.1)`). What this guide says about the VS Code plug-in and the desktop host's debugger comes from their documentation and tests, and was not run for it.

## 1. Before you start

### 1.1 What a description does today

A **tool description** states one tool type completely: its identity, its model, its surface, its behaviour, the bindings to the files it reads and writes, and its logic.

ATD is in phase 1 of a longer move (specification, section 1.2). **No host runs a tool from a description yet.** The hosts keep running tools from the `.dis`, `.des` and `.fbl` files. So you will not see your tool open in a host by writing a description. What you can do with a description today is check it, format it, edit it with language support in three hosts, and, for a diagram, replay recorded conformance cases through its runtime and debug its script.

### 1.2 Which kind you are writing

Every tool type is of exactly one kind. Ask these questions in this order:

1. Is typing text the main thing the user does? Then it is an **editor**.
2. Are elements drawn connected to each other? Then it is a **diagram**.
3. Is it laid out visually, with nothing connected, and filled in rather than typed? Then it is a **designer**.

The kind decides which constructs the description may hold. A designer cannot declare a relation type, for example, and an editor cannot have a notation. The table in section 14.4 of the specification is the full list, and the checker enforces it.

### 1.3 The tools

`atd` is the command-line tool. The desktop host's repository, `etalii.adp.ide.desktop`, publishes it for Windows, macOS and Linux in its releases named `atd-v<version>`. To build it from a clone of that repository, run this there, which leaves it at `target/release/atd`:

```bash
cargo build --release -p adp-atd-cli
```

A release can lag behind the repository, and this guide's output comes from a build of the repository. Run `atd` with no arguments to see the commands of the one you have.

`atd` reads ATD's grammar and rules from what was built into it. `--spec-dir <dir>` only changes which folder `atd tables` reads, so editing a grammar file in your clone of etalii.adp does not change what `atd check` accepts.

You can write a description in any program that edits text. The VS Code and IntelliJ plug-ins and the desktop host add colours, findings as you type, completion, hover, go to definition, rename and formatting to `.atd` files. VS Code also has the commands *ADP: New Tool Description*, *ADP: Check Tool Description* and *ADP: Validate Tool Description*, which give the results `atd new`, `atd check` and `atd validate` give ([the plug-in's documentation](https://github.com/etalii-adp/etalii.adp.ide.vscode/blob/develop/docs/atd.md)).

Make an empty folder for each tool type and run the commands from it. A description's files must stay together in one folder, since an import cannot reach outside it.

## 2. The shape of a description

The quickest way to see a whole description is to ask for one. `atd new` writes a small working tool of the kind you name:

```bash
atd new --kind editor --origin example/notes --name "Notes" --out .
```

It prints the path of the file it wrote, `notes.atd`, and never overwrites a file that already exists. This is the whole file:

```atd
atd 0.1

/// Notes: an editor whose text is the document, with an outline of its headings.
editor example/notes version "0.1.0" {
  label "Notes"

  model {
    element Heading {
      attribute title: string required
      labelAttribute: title
    }
  }

  surface text {
    files "*.notes.txt"
    outline Heading by title
    fold Heading
  }

  binding text family lines {
    label "Notes"
    claims files "*.notes.txt"

    element Heading as heading line r"^# (?<title>.+)$" {
      attribute title from group "title" { empty: refuse }
      insert { place: end, emit: "# {title}" }
      remove {}
    }
  }
}
```

Read it from the top:

- **The header**, `atd 0.1`, comes first in every file. It names the version of ATD the file is written in.
- **The tool construct**, `editor example/notes version "0.1.0" { … }`, names the kind, the origin and the tool type's own version. The **origin** is `<vendor>/<type>`. If the tool type already has a `.dis` or `.des` file, the description uses its origin, and its `label` is its display name.
- **`///` comments** document the construct below them, and language support shows them on hover. `//` comments do not document anything.
- **Members** fill the braces, in any order: here a `label`, a `model`, a surface and a `binding`.
- **Settings** are members too, written `key: value`. In `labelAttribute: title`, the colon makes it a setting. The name of a setting and the values it takes come from DISL, DESL or FBL: section 14.6 of the specification says which, and the checker holds a setting to that schema.
- **A value** is a literal, a list, a map, `@title` (an attribute), `$color.surface` (a theme token) or `( … )`, a CEL expression. CEL is the expression language ATD uses everywhere an expression is needed.

`atd new` also takes `--kind diagram` and `--kind designer`. In real work, start from what it writes for your kind and change it. In the three walks below we start smaller, so that you meet each part in turn.

## 3. A diagram: the mind map

A mind map is a diagram: ideas branch from one central topic, and the branches are drawn connected. Each node is an element, and each line is a relation. The file a user keeps is a Freeplane `.mm` file, which belongs to Freeplane as much as to ADP.

Make a folder and work in it. After each step, run:

```bash
atd check mindmap.atd
```

When the description is clean, `atd check` prints nothing and exits with 0. Checking the root file checks every file it imports. Always name the root, or a folder that holds it: a part named alone is checked without its root, and gives findings such as `No element type` that say nothing about your description.

### Step 1: name the tool type

Create `mindmap.atd` with the header and the tool construct:

```atd
atd 0.1

/// Ideas branch from one central topic. A trimmed mind map, after DISL's example `mindmap.dis` and FBL's
/// `mindmap.fbl`: one node type nested to any depth, read from and written to a Freeplane `.mm` file.
diagram freeplane/mindmap version "0.1.0" {
  label "Mind map"
}
```

A diagram is the one kind that checks with nothing in it. A designer or an editor must have its surface from the start.

### Step 2: state the model

The model says what a document holds. Add this inside the tool's braces, after the label:

```atd
  model {
    /// A topic in the map. The one node without a parent is the central topic.
    element Node {
      label "Node"
      attribute text: string default "" { label: "Text" order: 1 }
      attribute notes: text default "" { markup: plain label: "Notes" order: 2 }
      attribute link: string { label: "Link" order: 3 }
      /// Freeplane's `FOLDED="true"`: it seeds each viewer's fold state and is never written by ADP.
      attribute folded: bool readonly default false { label: "Collapsed" group: "View" order: 4 }
      attribute position: Side readonly { label: "Side" order: 5 }
      contains Node ordered
      labelAttribute: text
    }

    /// The line from a node to its parent; not stored, but derived for every node that has a parent.
    relation Branch from Node to Node {
      directed: true
      allowSelfLoops: false
      allowParallel: false
      acyclic: true
      derived: (diagram.nodes.filter(n, n.parent != null).map(n, {'source': n.parent, 'target': n}))
    }

    /// Freeplane's `POSITION` on a first-level node: the side of the central topic its branch is drawn on.
    enum Side {
      right { label: "Right (Freeplane 1.11)" }
      left { label: "Left (Freeplane 1.11)" }
      bottom_or_right { label: "Right (Freeplane 1.12)" }
      top_or_left { label: "Left (Freeplane 1.12)" }
    }
  }
```

- `element Node` is an element type. An `attribute` has a name, a type and, optionally, `required`, `many`, `readonly` and `default`. Its block holds settings.
- `contains Node ordered` says a node holds nodes, in an order that matters. That is what makes the map a tree.
- `labelAttribute: text` says which attribute names a node.
- `relation Branch from Node to Node` is a relation type, which only a diagram may have. The `derived` setting is a CEL expression in parentheses: a branch is not stored, but computed from each node's parent.

### Step 3: say how it looks, in a part of its own

Notation tends to grow, so we keep it in a second file, a **part**. A part has the header, then `part of <origin>`, then members without surrounding braces. Create `mindmap.notation.atd`:

```atd
atd 0.1
part of freeplane/mindmap

// The mind map's notation, toolbox and layout, in a part of their own (ATD §5.2).

coordinates {
  system canvas { kind: cartesian, x: { kind: linear, unit: px }, y: { kind: linear, unit: px }, orientation: y-down }
  default canvas
}

notation {
  theme {
    tokens: { color.surface: "#FFFFFF", color.border: "#E2E8F0", color.text: "#0F172A", color.primary: limegreen }
    modes: { dark: { color.surface: "#1E293B", color.border: "#334155", color.text: "#F1F5F9" } }
    defaultMode: light
    followSystem: true
  }
  /// textWidth = characters × 14 × 0.55, a character being a UTF-16 code unit, so every host lays the map out alike.
  textMetric: { kind: average, advance: 0.55, count: utf16, lineHeight: 1.4 }

  style node {
    fill: $color.surface
    stroke: { color: $color.border, width: 1.5 }
    font: { size: 14, color: $color.text }
    padding: [6, 10]
  }

  node Node {
    shape rect
    style: node
    size: {
      width: (math.round(max(32.0, textWidth(self.text, 14.0) + 20.0) * 100.0) / 100.0)
      height: (textHeight(1, 14.0) + 12.0)
      resizable: false
    }
    label @text as text { editable: inline, position: center, wrap: none }
    label ([self.notes != "" ? "•" : "", self.view.collapsed && self.children.size() > 0 ? "⊕" : ""].filter(s, s != "").join(" ")) as indicators {
      position: { anchor: [1, 0], align: end }
    }
    anchors: { mode: sides, sides: [left, right] }
    container: { nesting: none, collapsible: true, highlightOnDrop: true }
    state dropTarget { stroke: { color: $color.primary, width: 2.5, dash: [6, 3] } }
    deletable: (self.parent != null)
    doubleClick: editLabel
  }

  edge Branch {
    line { stroke: { color: $color.border, width: 1.5 }, routing: bezier }
    marker source none
    marker target none
    selectable: false
    deletable: false
    layer: belowNodes
  }
}

layout {
  default mindmap
  algorithm mindmap capability layout.twoSidedTree { spacing: 24, levelGap: 48 }
}
```

Then bring the part into the description, and state the capability its layout uses, by adding these two lines to `mindmap.atd`, right after the label:

```atd
  import "mindmap.notation.atd"
  requires capability layout.twoSidedTree@^1
```

- `node Node` draws the element type `Node`, and `edge Branch` draws the relation type `Branch`.
- `label @text` shows an attribute, and `editable: inline` lets the user edit it in place. The second label is a CEL expression, so it is computed.
- `$color.surface` reads a token of the `theme`, which has its own values in the dark mode.
- A **capability** is a named algorithm the platform provides, for work the script is not meant for. A two-sided tree layout is one. You state each capability you use with `requires capability <name>@<range>`, and you pass its arguments as settings. Section 13.6 of the specification lists the catalogue, and each capability's contract is in `capabilities/<name>/contract.json`. If your layout is an ordinary one, you need no capability: the template from `atd new --kind diagram` uses DISL's `algorithm: layered`.

### Step 4: add what the user can do

An **operation** is a gesture on an element, and it runs as one step of the undo history. Its statements are actions, and its logic is **script**. Add these to the tool's braces in `mindmap.atd`:

```atd
  reason atStart "This node is already the first under its parent."

  operation addChild "Add child" for Node {
    doc "Adds a node as the last child, then edits its text in place."
    icon: "mdi-subdirectory-arrow-right"
    shortcut: "Insert"
    create Node { text: siblingName(self, "Node") } in self as added
    select added
    editLabel added { label: text }
  }

  operation addSibling "Add sibling" for Node {
    doc "Adds a node right after this one under the same parent, then edits its text in place."
    icon: "mdi-plus"
    shortcut: "Enter"
    enabled self.parent != null
    create Node { text: siblingName(self.parent, "Node") } in self.parent after self as added
    select added
    editLabel added { label: text }
  }

  operation promote "Move up a level" for Node {
    doc "Makes the node a sibling of its parent, placed right after it, with its whole branch."
    shortcut: "Ctrl+Left"
    enabled self.parent != null && self.parent.parent != null
    reparent self to self.parent.parent after self.parent
  }

  operation moveFirst "Move to the top of its branch" for Node {
    enabled self.parent != null
    unavailable [{ when: (self.positionIn(self.parent.children) == 0), reason: atStart }]
    reorder self before self.parent.children[0]
  }

  operation editNotes for Node {
    label (self.notes != "" ? "Edit notes" : "Add notes")
    param notes: text { label: "Notes" }
    set self.notes = p.notes
  }

  operation toggleFold for Node {
    doc "Collapses or expands the branch for this viewer only: nothing is written and nothing lands on the undo history."
    label (self.view.collapsed ? "Expand" : "Collapse")
    shortcut: "Space"
    enabled self.children.size() > 0
    view self { collapsed: !self.view.collapsed }
  }

  /// The name for a new node under `parent`: `base`, else `base` and the first free number from 2.
  fn siblingName(parent: Node, base: string) -> string {
    let taken = parent.children.map(c, c.text)
    if !(base in taken) {
      return base
    }
    for i in range(2, 1000) limit 1000 {
      let candidate = base + " " + string(i)
      if !(candidate in taken) {
        return candidate
      }
    }
    fail "No free name for a node under '" + parent.text + "'."
  }
```

- `for Node` says which elements the operation is offered on, and `self` is the one it runs on. `enabled` is a CEL condition, and `p` holds the parameters.
- `create`, `select`, `editLabel`, `reparent`, `reorder`, `set` and `view` are actions. Section 12.3 of the specification lists them all.
- `fn` declares a script function. It has local variables, `if` and `for`, and it is called from CEL by its name. A script is deterministic: it reads no clock, no random numbers and no files.
- Every loop must state a `limit`, the most iterations you expect. Reaching it stops the call with a finding rather than running on.
- A function with a result type must return or `fail` on every path, and `fail` refuses the gesture with that message.

### Step 5: put the operations in the toolbox

Now that the operations exist, the toolbox can name them. Add this at the end of `mindmap.notation.atd`:

```atd
toolbox {
  generic: { select: true, pan: true }
  group mindmap "Mind map" {
    tool node "Node" { kind: operation, operation: addChild, icon: "mdi-card-plus-outline", mode: drop }
  }
  context for Node {
    placement: menu
    tool "Add child" { kind: operation, operation: addChild, shortcut: "Insert" }
    tool "Add sibling" { kind: operation, operation: addSibling, shortcut: "Enter", visible: (self.parent != null) }
    tool "Rename…" { kind: editLabel, shortcut: "F2" }
    tool { kind: operation, operation: toggleFold, label: (self.view.collapsed ? "Expand" : "Collapse"), visible: (self.children.size() > 0) }
  }
}
```

Take care with the names here. `operation: addChild` is a setting, and this `atd` does not report an operation name in a toolbox that resolves to nothing, so a misspelling passes the check unnoticed.

### Step 6: add the rules

A **constraint** is a rule of the running tool, and a broken one gives the user a finding. Add this to `mindmap.atd`:

```atd
  constraints {
    builtin std.missingId {
      severity: info
      code: "mindmap.missing-id"
      message: ("The node '" + self.text + "' had no ID in the file; it was given one, which is written with the next save.")
    }

    constraint unnamedCentralTopic for Node {
      label "Named central topic"
      when self.parent == null
      rule self.text.trim() != ""
      severity warning
      message ("The map '" + diagram.file + "' has an unnamed central topic.")
      code: "mindmap.unnamed-root"
      attribute: text
    }

    constraint keepCentralTopic for Node {
      kind: delete
      rule self.parent != null
      message "The root node cannot be removed."
    }
  }
```

`when` says where the rule applies, and `rule` must hold there. `kind: delete` makes `keepCentralTopic` refuse the gesture of deleting the node, rather than report on the model. Do not confuse these with the findings of `atd check`: those are about your description, and these are about a user's document.

### Step 7: read and write the file

A **binding** says how a file maps to the model, in both directions. Its meaning is FBL's, written as text. Every edit is written as a **splice**, a minimal replacement of a range of bytes, and everything outside it stays byte for byte as it was. Add this to `mindmap.atd`:

```atd
  persistence {
    ids: { strategy: "uuid-v4", prefix: "ID_", stable: true, missing: assign }
    view: { store: [], viewer: [collapsed, viewport, selection], initial: { collapsed: (self.folded) }, revealExpands: true }
  }

  /// A Freeplane mind map: the `xml` family. Every construct ADP does not model is kept byte for byte.
  binding freeplane family xml {
    label "Freeplane mind map"
    claims { extensions: [".mm"], origins: ["freeplane/mindmap"], marker: { pattern: r"^<map[\s>]", lines: 5 } }
    text { newline: lf }
    header { line: r"^<map[\s>]", required: true }

    element Node as node at "/map/**/node" {
      parent node slot children
      id from attribute "ID"
      attribute text from attribute "TEXT" {
        override: { child: "richcontent[@TYPE='NODE']", text: true }
        content: "html-paragraphs"
      }
      attribute notes from text {
        child: "richcontent[@TYPE='NOTE']"
        content: "html-paragraphs"
        empty: remove
      }
      attribute link from attribute "LINK" { empty: remove }
      attribute folded from attribute "FOLDED" { readOnly: "Folding is a viewer's state and is never written to the map." }
      attribute position from attribute "POSITION" { readOnly: "The side follows Freeplane's layout and is kept as Freeplane wrote it." }
      insert { place: "last-child", emit: "<node TEXT=\"{text}\" ID=\"{id}\"/>" }
      remove {}
    }

    relation Branch as branch at "/map/**/node/node" {
      source from parent "ID"
      target from attribute "ID"
      readOnly: "A branch is the nesting of one node in another; move or remove the node instead."
    }

    template """<map version="freeplane 1.11.5">
<node TEXT="{base}" ID="{newid:node}"/>
</map>
"""
  }
```

- `family xml` chooses how the file is read. The families are `yaml`, `json`, `xml`, `lines`, `blocks`, `bullets` and `grammar`, and `family capability <name>` reads a format through a capability. Section 19.1 of the specification names an example of each.
- `claims` says which files the binding takes. `header` says what such a file must start with.
- `element Node as node at "…"` is a rule. It reads every entry the selector finds as a `Node`, and `as node` names the rule, so that `parent node` can refer to it.
- `attribute text from attribute "TEXT"` binds a model attribute to a **slot**, a place in the entry. The checker refuses a rule that binds an attribute the model does not declare.
- `insert` and `remove` say how a new node is written and an old one taken out. `template` is the text of a new file.

### What you have now

Your two files now hold the same members as [`mindmap.atd`](mindmap.atd) and [`mindmap.notation.atd`](mindmap.notation.atd) beside this guide, though perhaps in another order: the order of members does not change their meaning.

That example is trimmed. The real mind map's description is [`definitions/diagrams/mindmap.atd`](../../definitions/diagrams/mindmap.atd), with the parts it imports. It has its binding in a part of its own, more script (sibling naming that follows the standalone host's), a trace and conformance cases. Sections 7 and 8 of this guide use it.

## 4. A designer: the Hierarchical Requirement Designer

**A note on the example.** This walk should be the Knowledge designer, and it will be. The Knowledge designer has `definitions/designers/knowledge.des` and `knowledge.fbl` but no tool description yet; spec 076 writes it. Nothing about it could be checked here, so this guide uses the designer that does have a description, the Hierarchical Requirement Designer, and will move to Knowledge when spec 076 lands. The Knowledge designer has a table surface: until then, [`reading-list.atd`](reading-list.atd) and what `atd new --kind designer` writes are the examples of `surface table`.

The Hierarchical Requirement Designer shows the requirements of one subject as boxes that contain boxes, kept in one Markdown file that a person and an agent both write. Nothing is drawn connected: an item contains, and a reference is a value in a description. That is why it is a designer and not a diagram.

This walk builds a trimmed description, without categories, states, stored filters or column widths. Work in a new folder, and after each step run:

```bash
atd check hierarchical-requirement.atd
```

### Step 1: the model and its surface

A designer must have one form surface from the start, so the first file already has one. Create `hierarchical-requirement.atd`:

```atd
atd 0.1

/// The requirements of one subject as nested boxes, kept in one Markdown file a person and an agent both read and write.
designer etalii/hierarchical-requirement version "0.1.0" {
  label "Hierarchical Requirement Designer"

  model {
    /// One level of the hierarchy, a part of the subject. Nested to any depth.
    element Item {
      label "Item"
      labelAttribute: title
      /// The one line on its bullet.
      attribute title: string required
      /// The lines indented under its bullet.
      attribute description: text
      /// The tags it carries, each naming a member of the tag list by name, ignoring case.
      attribute tags: string many { pattern: r"^[^\s#]+$" }
      /// Folded on the page. Kept in the registration's collapsed block, not in the file.
      attribute collapsed: bool default false
      contains Requirement, Item ordered
    }

    /// One requirement, in an item. It contains nothing.
    element Requirement {
      label "Requirement"
      labelAttribute: title
      /// The requirement itself.
      attribute title: string required
      /// Optional clarification.
      attribute description: text
      attribute tags: string many { pattern: r"^[^\s#]+$" }
      attribute collapsed: bool default false
    }
  }

  surface hierarchy {
    levels Item { title: title, description: description }
    entries Requirement { title: title, description: description }
  }
}
```

A form surface does not draw anything of its own. It says which element types and attributes play which part of the layout, and a host's component for that kind of surface does the rest. `surface hierarchy` has `levels`, the boxes that nest, and `entries`, what stands in them. The levels' type must contain itself and the entries' type, and the checker tells you when it does not (`ATD6002`).

Notice what is missing compared with a diagram: there is no `notation`, `toolbox`, `layout` or `relation`. The checker refuses each of these in a designer.

### Step 2: labels and notes

Add these to the `model` block, after `Requirement`:

```atd
    /// A tag's colour: the Knowledge designer's enumeration, Notion's ten names.
    enum Colour {
      default { label: "Default" }
      gray { label: "Gray" }
      brown { label: "Brown" }
      orange { label: "Orange" }
      yellow { label: "Yellow" }
      green { label: "Green" }
      blue { label: "Blue" }
      purple { label: "Purple" }
      pink { label: "Pink" }
      red { label: "Red" }
    }

    /// What an assessment is. The file writes the stored form and reads it in any case.
    enum AssessmentKind {
      question { value: "Question", label: "Question", icon: "mdi-help-circle-outline" }
      remark { value: "Remark", label: "Remark", icon: "mdi-comment-text-outline" }
      warning { value: "Warning", label: "Warning", icon: "mdi-alert-outline" }
      idea { value: "Idea", label: "Idea", icon: "mdi-lightbulb-outline" }
    }

    /// A question, remark, warning or idea about one entry, written by an agent or a person.
    element Assessment {
      label "Assessment"
      icon: "mdi-comment-question-outline"
      labelAttribute: text
      attribute kind: AssessmentKind required
      /// The number of the one item or requirement it is about. A mention, so that it is kept as written and reported
      /// when it names nothing.
      attribute about: string required { mentions: { pattern: r"^(?<id>\S+)$", types: [Item, Requirement] } }
      /// What follows the colon.
      attribute text: string
      /// The lines indented under it.
      attribute detail: text
      /// `[x]` in the file. A resolved assessment stays and no longer counts towards the icon.
      attribute resolved: bool default false
    }

    /// A member of the tag list, kept in the registration's tags block in its order.
    element Tag {
      label "Tag"
      labelAttribute: name
      /// No whitespace: typing a space in the designer writes `-`.
      attribute name: string required { pattern: r"^[^\s#]+$" }
      attribute colour: Colour default default
    }
```

Then give them their parts, by adding these to the `surface hierarchy` block:

```atd
    /// Drawn as tags below the title.
    labels tags { on: [Item, Requirement], options: { type: Tag, name: name, colour: colour }, prefix: "#" }
    notes Assessment { about: about, kind: kind, text: text, detail: detail, resolved: resolved }
    collapsed: collapsed
```

The `mentions` setting on `about` is how a designer holds a reference: as a value, filled in like any other, and never drawn connected. A diagram draws its relations, and a designer only lists them.

### Step 3: the numbers

Items and requirements keep a number that never changes, and both parties point at an entry by it. The last number given is kept in the document itself. Add this to the `model` block:

```atd
    /// The requirement document. Its DED envelope (line 1) is the binding's header, not an attribute of the model.
    document {
      /// A requirement stands in an item, so std.containment reports one at the top level.
      contains Item, Assessment, Tag
      /// The last item number the counter gave: the mark of Item's counter, on line 2 of the file.
      attribute lastItem: string { pattern: r"^ITEM-[0-9]+$" }
      /// The last requirement number the counter gave: the mark of Requirement's counter.
      attribute lastRequirement: string { pattern: r"^REQ-[0-9]+$" }
    }
```

Then add this to the tool's braces, and `ids: shown` to the `surface hierarchy` block, so that the number is shown beside the title:

```atd
  persistence {
    format: fbl
    ids: {
      missing: assign
      types: {
        Item: { strategy: counter, prefix: "ITEM-", digits: 4, mark: lastItem, placeholder: "NEW", doc: "ITEM-0001 onwards; ITEM-NEW is numbered with the next edit." }
        Requirement: { strategy: counter, prefix: "REQ-", digits: 4, mark: lastRequirement, placeholder: "NEW", doc: "REQ-0001 onwards, counted apart from the items." }
        Assessment: {
          strategy: derived
          expression: ('assessment/' + string(self.positionIn(document.elementsOfType('Assessment'))))
          ephemeral: true
          reason: "An assessment is known by its place, which changes when the file is edited, so nothing can be kept for it."
          doc: "Nothing refers to an assessment."
        }
        Tag: { strategy: derived, expression: ('tag/' + self.name.lowerAscii()) }
      }
    }
  }
```

A counter's `mark` must be a `string` attribute of `document { … }`, and the checker holds you to that (`ATD6003`).

### Step 4: what the user can do

Operations work as they do in a diagram. `for document` makes one an operation of the document itself. A designer's `move` names a parent and a sibling rather than a point on a canvas. Add these to the tool's braces:

```atd
  /// The control at the end of the page, and the first item of an empty document: an item at the top level, after the
  /// last one. The host selects its title for typing.
  operation addTopItem "New item" for document {
    create Item { title: 'New item' }
  }

  /// The control at the end of an item's content: a sub-item after the item's last child.
  operation addItem "New item" for Item {
    create Item { title: 'New item' } in self
  }

  /// The control at the end of an item's content: a requirement after the item's last requirement, or before its first
  /// sub-item when it has none.
  operation addRequirement "New requirement" for Item {
    let requirements = self.childrenOfType('Requirement')
    let items = self.childrenOfType('Item')
    if requirements.size() > 0 {
      create Requirement { title: 'New requirement' } in self after requirements[requirements.size() - 1]
    }
    if requirements.size() == 0 && items.size() > 0 {
      create Requirement { title: 'New requirement' } in self before items[0]
    }
    if requirements.size() == 0 && items.size() == 0 {
      create Requirement { title: 'New requirement' } in self
    }
  }

  /// Typing in a title. The binding refuses an empty title.
  operation setTitle "Title" for Item, Requirement {
    param title: string required { minLength: 1 }
    set title = p.title
  }

  /// Dragging a requirement to another item or another place. Its number, and every reference and assessment naming
  /// it, are unchanged.
  operation moveRequirement "Move" for Requirement {
    param item: Item required { doc: "The item it goes into." }
    param after: Requirement { doc: "The requirement it goes after there; absent, where a new requirement would go, or to the front within its own item." }
    let requirements = p.item.childrenOfType('Requirement').filter(r, r != self)
    let items = p.item.childrenOfType('Item')
    if self.parent == p.item {
      if has(p.after) {
        reorder self after p.after
      }
      if !has(p.after) && requirements.size() > 0 {
        reorder self before requirements[0]
      }
    } else {
      if has(p.after) {
        move self into p.item after p.after
      }
      if !has(p.after) && requirements.size() > 0 {
        move self into p.item after requirements[requirements.size() - 1]
      }
      if !has(p.after) && requirements.size() == 0 && items.size() > 0 {
        move self into p.item before items[0]
      }
      if !has(p.after) && requirements.size() == 0 && items.size() == 0 {
        move self into p.item
      }
    }
  }

  /// The control to the left of a title. Written to the registration's collapsed block, outside the history.
  operation toggleCollapsed for Item, Requirement {
    label (self.collapsed ? 'Expand' : 'Collapse')
    set collapsed = !self.collapsed
  }

  /// From the icon at the top right of an entry, or its list: an open assessment at the end of Assessments.
  operation addAssessment "New assessment" for Item, Requirement {
    param kind: AssessmentKind default question
    param text: string required
    create Assessment { kind: p.kind, about: self.id, text: p.text }
  }

  /// A switch between `[ ]` and `[x]`: one replace-value of one character.
  operation resolveAssessment for Assessment {
    label (self.resolved ? 'Reopen' : 'Resolve')
    set resolved = !self.resolved
  }
```

### Step 5: the Markdown file

The binding goes in a part. It uses the `bullets` family, which reads the list items under a heading as a tree. Create `hierarchical-requirement.markdown.atd`:

```atd
atd 0.1
part of etalii/hierarchical-requirement

// The designer's Markdown binding, in a part of its own, trimmed from hierarchical-requirement.markdown.atd.

/// A requirement document is one Markdown file: the `bullets` family. Line 1 is DED's envelope, the binding's required
/// header and its claim's marker. Line 2 is the mark line. Everything else is unbound and kept byte for byte.
binding markdown family bullets {
  label "Requirement document, Markdown"
  claims {
    extensions: [".md"]
    shared: true
    marker: { pattern: r"^<!--[ \t]+ded:[ \t]+[^ \t]+[ \t]+designer:[ \t]+etalii/hierarchical-requirement[ \t]+-->[ \t]*$", lines: 1 }
    origins: ["etalii/hierarchical-requirement"]
  }
  text { newline: crlf, indent: 2, finalNewline: true }
  header { line: r"^<!--[ \t]+ded:[ \t]+[^ \t]+[ \t]+designer:[ \t]+[^ \t]+[ \t]+-->[ \t]*$", required: true }

  /// The hierarchy: the lines after the first heading whose text is Requirements, at any level and in any case.
  section requirements r"^Requirements$" { caseInsensitive: true }
  /// The assessments: the lines after the first heading whose text is Assessments, by the same rule.
  section assessments r"^Assessments$" { caseInsensitive: true }

  /// The mark line, `<!-- last: ITEM-0003 REQ-0004 -->`. The type map maps `Marks` as the document, so the two groups
  /// are the document's attributes `lastItem` and `lastRequirement`.
  element Marks as marks line r"^<!--[ \t]+last:(?:[ \t]+(?<lastItem>ITEM-\d+))?(?:[ \t]+(?<lastRequirement>REQ-\d+))?[ \t]+-->[ \t]*$" {
    attribute lastItem from group "lastItem"
    attribute lastRequirement from group "lastRequirement"
    insert { place: after-header, emit: "<!-- last:[ {lastItem}][ {lastRequirement}] -->" }
  }

  /// An item: `ITEM-` and its number (or the placeholder NEW), whitespace, its title, and its tags at the end of the
  /// line. Its parent is the item it is nested in.
  element Item as item line r"^(?<id>ITEM-(?:\d+|NEW))(?:[ \t]+|$)(?<title>.*?)(?<words>(?:[ \t]+#[^\s#]+)*)[ \t]*$" {
    within requirements
    id from group "id"
    parent item, requirement
    attribute title from group "title" { empty: refuse }
    attribute tags from group "words" { word: r"^#(?<value>[^\s#]+)$", empty: remove }
    attribute description from continuation { empty: remove }
    insert { place: last-child, container: requirements, create: { at: end-of-document, text: "## Requirements" }, emit: "{id} {title}" }
    remove {}
  }

  /// A requirement: `REQ-` and its number (or the placeholder NEW), whitespace, its title, then its tags.
  element Requirement as requirement line r"^(?<id>REQ-(?:\d+|NEW))(?:[ \t]+|$)(?<title>.*?)(?<words>(?:[ \t]+#[^\s#]+)*)[ \t]*$" {
    within requirements
    id from group "id"
    parent item, requirement
    attribute title from group "title" { empty: refuse }
    attribute tags from group "words" { word: r"^#(?<value>[^\s#]+)$", empty: remove }
    attribute description from continuation { empty: remove }
    insert { place: last-child, container: requirements, create: { at: end-of-document, text: "## Requirements" }, emit: "{id} {title}" }
    remove {}
  }

  /// Any other bullet under Requirements. The type map maps `Unreadable` as unreadable, so it is reported by
  /// std.unreadableEntry at the bullet, and its bytes, with everything nested in it, are kept and not read.
  element Unreadable as unreadable-requirement line r"^.*$" {
    within requirements
  }

  /// An assessment: an optional checkbox, its kind, the number of the entry it is about, a colon and its text.
  element Assessment as assessment line r"^(?:\[(?<done>[ xX])\][ \t]+)?(?<kind>Question|Remark|Warning|Idea)[ \t]+(?<about>[^\s:]+):[ \t]*(?<text>.*?)[ \t]*$" {
    within assessments
    caseInsensitive: true
    attribute resolved from group "done" { map: { " ": "false", x: "true" }, default: false }
    attribute kind from group "kind" { map: { Question: "Question", Remark: "Remark", Warning: "Warning", Idea: "Idea" } }
    attribute about from group "about" { empty: refuse }
    attribute text from group "text" { empty: keep }
    attribute detail from continuation { empty: remove }
    insert { place: end, container: assessments, create: { at: end-of-document, text: "## Assessments" }, emit: r"\[{resolved}\] {kind} {about}: {text}" }
    remove {}
  }

  /// Any other bullet under Assessments, reported by std.unreadableEntry and kept as written.
  element Unreadable as unreadable-assessment line r"^.*$" {
    within assessments
  }

  registration {
    createOnFirstPlacement: true
    blocks: {
      tags: { order: kept, keys: names, doc: "The tag list in its order, `<name>: <colour>`." }
      collapsed: { order: ordinal, keys: ids, history: false, doc: "The collapsed items and requirements, `<number>: yes`, outside the history, so that undo takes back the last change to the requirements and not the last box folded." }
    }
  }

  /// A new requirement document: the envelope, the mark line, the title and the two sections. One string, so that its
  /// line ends are its escapes, whatever line ends this file has.
  template "<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->\r\n<!-- last: ITEM-0001 -->\r\n# {base}\r\n\r\n## Requirements\r\n\r\n- ITEM-0001 First part\r\n\r\n## Assessments\r\n"
}
```

Then bring it in with this line in `hierarchical-requirement.atd`, right after the label:

```atd
  import "hierarchical-requirement.markdown.atd"
```

And add these to the `persistence` block. The first says that two of the binding's types are not types of the model, and the second says what the `.adp` registration beside the file keeps:

```atd
    typeMap: {
      Marks: { as: document, doc: "Line 2 of the file holds the last numbers the counters gave." }
      Unreadable: { as: unreadable, doc: "A bullet under Requirements or Assessments in no known form: reported by std.unreadableEntry, kept with everything nested in it, and not read." }
    }
    settings: {
      tags: { type: Tag, key: name, value: colour, initial: [], doc: "The tag list starts empty: every tag an entry carries is an implicit member." }
      collapsed: { attribute: collapsed, of: [Item, Requirement], doc: "Only a collapsed entry is stored, `<number>: yes`." }
    }
```

The model, the surface and the binding are checked as one unit. Each key of `settings` must be a block that every binding's `registration` declares (`ATD6004`), and an element mapped `as: unreadable` binds no attribute (`ATD6001`).

### What you have now

The complete designer is [`definitions/designers/hierarchical-requirement.atd`](../../definitions/designers/hierarchical-requirement.atd) with its part [`hierarchical-requirement.markdown.atd`](../../definitions/designers/hierarchical-requirement.markdown.atd). What this walk left out is more of the same: categories and states as further `labels`, `filters`, `widths`, and references in descriptions through `mentions`. Its conformance cases are recorded in `definitions/designers/conformance/hierarchical-requirement/`, but they cannot be replayed yet, as section 8 explains.

## 5. An editor: the Markdown editor

In an editor, typing text is the core interaction. The text is the document, so an editor's model is not what the user edits: it is what the tool reads from the text. For the Markdown editor, that is the headings, which it shows in an outline.

Work in a new folder, and after each step run:

```bash
atd check markdown.atd
```

### Step 1: which files, and how they are coloured

An editor must have one text surface from the start. Create `markdown.atd`:

```atd
atd 0.1

/// The Markdown editor: a Markdown file as text, with an outline of its headings above the text and a preview of the
/// rendered document beside it.
editor editor/markdown version "1.0.0" {
  label "Markdown editor"

  surface text {
    files "*.md", "*.markdown"
    /// Colours by the token classes of the Markdown syntax. Above 20,000 lines the text is shown without colour.
    tokens markdown { largestColouredLines: 20000 }
  }
}
```

`files` lists what the editor opens, and `tokens` names the syntax that colours the text. The origins of the five editor types are those of the desktop host's catalogue: `editor/plain`, `editor/markdown`, `editor/json`, `editor/xml` and `editor/yaml`.

### Step 2: how the text behaves

These settings state what the desktop host's editors do with a file and a buffer. Add them to the `surface text` block:

```atd
    /// How the file is opened and saved, as every editor of the desktop host opens one: UTF-8 with or without a
    /// byte-order mark, or ASCII; refused, with the reason, above 5 MB (before a byte is read), when it looks binary or
    /// when it is not UTF-8. Every line keeps its own line ending, a new line takes the dominant one, a last line keeps
    /// whether it had an ending, and the byte-order mark is written back exactly when the file had one, so an
    /// unchanged file saves back byte for byte.
    open: {
      encodings: ["UTF-8", "ASCII"]
      byteOrderMark: keep
      largestBytes: 5242880
      binary: refuse
      otherEncodings: refuse
      lineEndings: keep
      newLineEnding: dominant
    }
    /// The buffer: one cursor and one selection; Tab writes four spaces; pasted text has its CRLF made LF; no soft
    /// wrap; a gutter with the line numbers and the current line marked. Typing, the movements by character, word, line
    /// and document, copy, cut and paste are the host's gestures, and grouping undo word by word is the host's undo
    /// stack (ATD §17).
    buffer: {
      cursors: 1
      tab: "    "
      paste: { newline: lf }
      softWrap: false
      gutter: lineNumbers
      currentLine: marked
    }
    /// Where a finding is said: drawn under its range by severity, and listed in the problems panel at its line and
    /// column, both from 1, columns counted in characters.
    findings: { line: 1, column: 1, columns: characters }
```

A description states everything about a tool type except what section 17 of the specification calls the host's concerns: rendering, gestures, text measurement, file dialogs, opening links and the undo stack. That is why the comment above says that typing is the host's.

### Step 3: read the headings

An outline needs a model, and a model of text needs a reader. Markdown's headings are read by the capability `reader.markdown`, so add this to the tool's braces:

```atd
  requires capability reader.markdown@^1

  model {
    /// An ATX heading outside fenced code: one to six `#`, whitespace and a title.
    element Heading {
      label "Heading"
      labelAttribute: title
      /// The number of `#`, from 1 to 6.
      attribute level: int required
      /// The heading's text without its closing `#` and the whitespace around them.
      attribute title: string required
      /// The heading's line, from 1: choosing the heading in the outline scrolls the text to it.
      attribute line: int required
    }
  }

  /// The text read as its headings: the binding of the outline. It plans no edit, since the editor writes the text.
  binding outline family capability reader.markdown {
    label "Markdown"
    claims files "*.md", "*.markdown"
    body { kind: file }
  }
```

And add the outline to the `surface text` block:

```atd
    /// The headings above the text, in the order of the text, each by its level; choosing one scrolls the text to its
    /// line. They are read again whenever the text changes.
    outline Heading by level
```

`family capability reader.markdown` says that a capability reads the body. If your format were simple enough to declare, you would use a family instead, as the template of `atd new --kind editor` does at the start of section 2 with `family lines`.

### Step 4: the preview

The preview is a second capability, of the `render` extension point. Add this line to the tool's braces, next to the other `requires`:

```atd
  requires capability render.markdown@^1
```

And this to the `surface text` block:

```atd
    /// The rendered document beside the text: CommonMark with tables, strikethrough and task lists, made again
    /// whenever the text changes.
    preview capability render.markdown { side: right }
```

### What you have now

Your file holds the same members as [`definitions/editors/markdown.atd`](../../definitions/editors/markdown.atd). The JSON editor, in [`json.atd`](../../definitions/editors/json.atd) beside it, also shows `check with` and `complete from`, and [`notes.atd`](notes.atd) beside this guide shows `fold`, a quick fix and a `bullets` binding.

## 6. Checking

`atd check` takes files or folders, and checks every description whose files are among them. Its findings are about your description, one per line, each with its file, line, column, severity and code:

```bash
atd check mindmap.atd
```

Suppose that in step 3 of the mind map you had written `label @txt` for `label @text`. This is what you get, with exit code 1:

```text
mindmap.notation.atd:36:11: error ATD2001: No attribute `txt` in element type `Node`. Did you mean `text`?
```

Most findings of this sort offer the closest names. The codes are listed in section 15 of the specification, and they go by range:

| Codes | What went wrong |
|---|---|
| `ATD0xxx` | The header: no version, or a version this `atd` does not know. |
| `ATD1xxx` | A syntax error. The file does not parse, so nothing else is checked until it does. |
| `ATD2xxx` | A name that resolves to nothing, or that is declared twice; an import that fails. |
| `ATD3xxx` | A construct your kind does not admit, or a designer or editor without its surface. |
| `ATD4xxx` | A CEL expression of the wrong type, or with an unknown variable. |
| `ATD5xxx` | A setting its schema does not have, or a value it does not admit. |
| `ATD6xxx` | The model, the surface and the bindings do not fit each other. |
| `ATD7xxx` | The script: a call to a clock or a file, an impure function where only reading is allowed, a path that does not return. |
| `ATD8001` | A binding's grammar that is not LALR(1). |
| `ATD9xxx` | Validation: a difference from a conformance case, or a capability the host lacks. |

Fix the first finding first. When a file does not parse you get its syntax errors only, since whatever the parser skipped would otherwise be reported again as names that resolve to nothing.

`atd check --json` gives the same findings as data, for a program or an agent. Each finding carries its position, its message, its `suggestions` and its `fixes`, each fix a title and its edits in byte offsets:

```bash
atd check --json mindmap.atd
```

```json
{
  "findings": [
    {
      "code": "ATD2001",
      "severity": "error",
      "file": "mindmap.notation.atd",
      "line": 36,
      "column": 11,
      "endLine": 36,
      "endColumn": 15,
      "message": "No attribute `txt` in element type `Node`. Did you mean `text`?",
      "suggestions": [
        "text"
      ],
      "fixes": [
        {
          "title": "Change to `text`",
          "edits": [
            {
              "file": "mindmap.notation.atd",
              "start": 1141,
              "end": 1145,
              "text": "@text"
            }
          ]
        }
      ]
    }
  ]
}
```

`specifications/atd/refused/` holds descriptions that must be refused, each with the findings it must give. Read them to see what a finding looks like in context, and `refused/README.md` lists them. Two more forms of `atd check` compare against such a file, and you meet them mostly when you change the checker: `atd check --expect <findings.json> <file>`, and `atd check --corpus <folder>`.

`atd tree <file>` prints the abstract tree the checker reads, as JSON. It helps when you cannot tell how a line was understood.

## 7. Formatting

`atd fmt` rewrites files in the canonical layout of section 16 of the specification. It never reorders members, and it keeps every comment with its construct. `atd fmt --check` writes nothing: it lists the files that are not in that layout, and exits with 1 if there are any.

```bash
atd fmt --check mindmap.atd mindmap.notation.atd
atd fmt mindmap.atd mindmap.notation.atd
```

A block of settings goes on one line when it holds no comment and fits in 120 columns, and is otherwise split over several. So the formatter will change some layout you copied from this guide, such as the block of `attribute notes` in the mind map's binding. The examples here are printed as they stand in the repository, and not all of those are in the canonical layout. A file that does not parse is left alone.

## 8. Validating

`atd check` asks whether your description is well formed. **Validation** asks whether it does what the tool type does: it replays recorded conformance cases through the description's own runtime, and reports each difference.

### Conformance cases

A case is a folder `conformance/<type>/<case>/` beside the description, holding input files and a `conformance.json`: what reading the inputs yields, the gestures made, and what the reference host produced for each, byte for byte. You do not write these by hand. They are recorded from a host's existing module, and a case changes only by recording it again. `atd record` does that, for a tool type the desktop host has a module and parity scripts for. It is part of the build of section 1.3, and not of the released `atd`. From `definitions/editors/`, this records the Markdown editor's cases again, with the description's path given as seen from a case's folder:

```bash
atd record editor/markdown --cases conformance/markdown --description ../../../markdown.atd
```

`atd record --capability <name> --cases <dir>` records a capability's fixtures in the same way. A new tool type with no module yet has no cases to record, so all you can do for it today is check it.

### Running validation

From the root of etalii.adp:

```bash
atd validate definitions/diagrams/mindmap.atd
```

The last line says how many cases passed, and the exit code is 0 only if all did. To run one case, use `--case <name>`, and to leave one out, use `--except <name>`. `--json` gives the result as data. Each difference is one line at the construct of the description that produced the value, with its case, step and part, and the expected and actual values:

```bash
atd validate --case refusals definitions/diagrams/mindmap.atd
```

```text
definitions/diagrams/mindmap.atd:269:5: error ATD9001: case 'refusals', step 3, refused: expected refused: The root node cannot be moved., got refused: A Node cannot be moved into its own branch.
definitions/diagrams/mindmap.atd:269:5: error ATD9001: case 'refusals', step 4, refused: expected refused: A node cannot be moved into its own branch., got refused: A Node cannot be moved into its own branch.
0 of 1 case(s) passed.
```

A case that passes prints only the last line:

```bash
atd validate --case unnamed-root definitions/diagrams/mindmap.atd
```

```text
1 of 1 case(s) passed.
```

**Validation works for diagrams only for now**, and not yet for every case. This is the message for the other two kinds:

```text
atd validate: A designer's description has no runtime yet; diagrams' have.
```

And the descriptions are not all passing yet. On the commit this guide was written from, the mind map's case `unnamed-root` passes and its other five do not, so read a failing case as a place where the description or the runtime still has work to do, and not as your own mistake. Spec 035's task T132 is what closes this, and spec 076 closes it for five tool types.

### Traces

A **trace**, `<name>.trace.json` beside a description, shows that the description is complete: it names, for each item of the tool type's companion file and each behaviour read in a host's code, the part of the description that states it, or says it is the host's concern. Check that its references resolve with:

```bash
atd check --trace definitions/diagrams/mindmap.trace.json
```

`mindmap.trace.json` with `mindmap.md` beside this guide is a small worked example, and section 2.5 of the specification gives the rules. A description of a new tool type, with no companion file, needs no trace.

## 9. Debugging

When a case shows a difference and you do not see why, debug the script in the desktop host. Debugging has no command line. It runs on a diagram's description, as validation does, on a description with conformance cases, and it writes no file of the description or of the case.

1. Open the folder that holds the description in the desktop host.
2. In the explorer, open the context menu of the root `.atd` file and choose *Validate Tool Description*. The validation view opens with one of the description's cases. Its left pane runs the description's own tool on a working copy of the case's document, and its right pane lists each difference. Choosing a difference opens the `.atd` at the construct that produced the value.
3. Turn on the *Debugger* toggle. The description's text appears with a gutter.
4. Press the gutter on a line of a function, an operation, a hook or a binding rule to set a breakpoint. In the mind map's `definitions/diagrams/mindmap.atd`, try the first statement of `fn siblingName`, with the case `example-1`, whose steps add nodes. A breakpoint is listed with a field for a CEL condition, and it then stops only where that condition holds, which is how you reach the one faulty iteration of a loop.
5. Choose *Debug Case* (the platform's modifier key with F5). The case runs until it stops at your breakpoint, and the text scrolls to the line.
6. Read the panes: the call stack, the selected frame's locals and arguments, the elements, the transaction (the commands so far, in order) and the budget (steps used of allowed, and the recursion depth).
7. Move with *Continue* (F5), *Step Over* (F10), *Step Into* (F11) and *Step Out* (Shift+F11). *Stop* (Shift+F5) discards the whole transaction, so the working copy is as it was before the gesture.

Saving a file of the description checks it again, and runs the case again when it checks clean. If it does not, the last result stays beside the findings. So you can fix a defect and see the difference disappear without restarting.

A breakpoint on a binding rule stops once for each entry the rule reads, with the entry's line and the attributes read so far.

These are the three defects that the desktop host's tests seed, and they are good first exercises on a copy of a description: a wrong comparison in sibling naming, an off-by-one bound in the causal loop's cycle enumeration, and a missing `else`.

## 10. Letting an agent write one

ATD's grammar is LALR(1), and it is published in a form that model services accept, so an agent can write a description under a grammar that admits only valid syntax. What is left is what the grammar cannot say, and `atd check --json` tells the agent that as data. Give the agent these:

1. **The flattened grammar**, [`grammar/atd.flat.lark`](grammar/atd.flat.lark). It is the whole grammar in one file, with its rule `start: description`, which the model services look for. Where the service constrains its output to a Lark grammar, pass this file as the constraint. Where it does not, give it to the agent as text. `atd-grammar.py --check` confirms in the Build workflow that LLGuidance's Lark front end accepts the file.
2. **Two examples.** Choose one of the kind the agent is to write, and one with the binding family it will need. [`reading-list.atd`](reading-list.atd), [`notes.atd`](notes.atd), [`binding-lines.atd`](binding-lines.atd) and the root and part of the mind map are all short.
3. **The description of the new tool type:** what its elements are, what the user does, which file it keeps its model in, and which kind it is.
4. **The findings, in a loop.** Save what the agent writes, run `atd check --json <file>`, and hand back the findings until there are none. `fixes` and `suggestions` give the agent the closest names.

A request to the agent could read as follows:

```text
Write a tool description in ATD 0.1, using only the attached grammar, atd.flat.lark. Here are two examples of what a valid description looks like. The tool type is a diagram with the origin example/influence: variables and the influences between them, one statement per line in a .cld file. After you write the file I will give you the findings of `atd check --json`; change the file until there are none. The grammar admits text the checker refuses, so read every finding's message and suggestions. Do not invent settings: a setting not in the examples must exist in DISL, DESL or FBL.
```

Spec 035's success criterion SC-009 asks for this within three rounds of findings, in at least four of five trials. Its task T152 records the trials.

## 11. Where to read further

| You want to know | Read |
|---|---|
| What each keyword means | [Specification](ATD-specification.md), Appendix B. Language support shows the same on hover. |
| Which ATD construct carries a property of DISL, DESL or FBL | Appendix C |
| Every setting of a construct | The DISL, DESL or FBL schema that Appendix C names |
| Every action and statement of the script | Sections 12.2 and 12.3 |
| Which capabilities exist | Section 13.6 and `capabilities/` |
| Every finding | Section 15.1 |
| More examples | Section 19, and every description under `definitions/` |
