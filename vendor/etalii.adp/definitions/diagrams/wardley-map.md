# Wardley map (`wardley/map`)

This is the companion to [`wardley-map.dis`](wardley-map.dis), the DISL specification of the Wardley map diagram type. The `.dis` holds everything DISL can say: the positioned elements and their attributes, the two links, evolutions, pipelines and the commentary elements, the unit-square coordinate system, the notation, the toolbox, context menus and property forms, the rules and the operations. This file holds everything it cannot: the `.owm` text format and its byte-identical round trip, the identity sidecar, the evolution-stage bands, the legacy pipeline form, the gestures DISL has no words for, and the places where the `.dis` could only approximate what the standalone implementation does.

Every statement here is tied to the source that shows it. Paths are in [etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone) on `develop`; `src/diagrams/wardley-map/` is abbreviated to `wardley-map/`, and `wardley-map/backend/EtAlii.Adp.Diagram.WardleyMap/` to `backend/`. "Req" refers to the standalone spec `wardley-map`, which was removed from the tree and is read with `git show "ece03c36^:.spec-workflow/archive/specs/wardley-map/requirements.md"` (and `design.md`, `tasks.md` beside it).

## Sources

| Source | What it contributed |
|---|---|
| `backend/` (`WardleyParser.cs`, `WardleyWriter.cs`, `WardleyIdentities.cs`, `WardleyValidator.cs`, `WardleyRuleSet.cs`, `WardleyContextActionProvider.cs`, `WardleyContextPropertyProvider.cs`, `WardleyToolboxProvider.cs`, `WardleyAxis.cs`, `WardleyEvolution.cs`, `WardleyEditability.cs`, `WardleyElementMapper.cs`, `Commands/`, `_Model/`) | The model, the `.owm` reader and writer, identities, rules, actions, properties, toolbox, the coordinate conversion and the stages. |
| `wardley-map/client/` (`WardleyCanvas.tsx`, `wardley.css`, `register.ts`, `readme.md`) | The notation: sizes, strokes, dashes, bands, labels and the drag. |
| `wardley-map/api/wardley-map.proto` | The element payload on the wire. |
| `wardley-map/backend/EtAlii.Adp.Diagram.WardleyMap.Tests/Fixtures/readme.md` | The certification of the fixtures against the real Online Wardley Maps parser, and what it found. |
| `wardley-map/examples/` | The shipped examples (`example 1/tea.owm`). |
| `.spec-workflow/archive/specs/wardley-map/` (in history before `ece03c36`) | Requirements 1-15, the design and the 27 tasks, all done. |
| Notion, "Tools" database, row "Wardley map" | Kind Diagram, state Prototype, family "Strategy & landscape mapping", focus areas, purpose and "why specialized" (used in the `.dis` `doc`). |
| `docs/tools.md` in the standalone repository | The catalogue row: Prototype, Diagram, `wardley/map`. |
| The project's conversations | Nothing Wardley-specific beyond the catalogue work; the only earlier mention is the standalone pull request "ansible and wardley styles" (#68). |

## 1. Identity

- **Origin tag** `wardley/map`, title "Wardley map", icon `mdi-chart-line`, description "A value chain positioned against evolution, so a strategy can be argued about rather than asserted." (`backend/Diagram.cs`). The `.dis` uses `net.etalii.adp.wardley.map` as its language id, following the `net.etalii.adp.<vendor>.<type>` pattern of the other definitions.
- **Catalogue**: Diagram, Prototype (standalone `docs/tools.md`; Notion "Tools"). Purpose in Notion: "Map a value chain by how visible each component is to users and how evolved it is."
- **Out of scope by decision** (design.md): editing the `.owm` as text with a live preview, as onlinewardleymaps.com, the VS Code extension and the Obsidian plugin do. The round trip below is what such an editor would need; it is simply not this diagram's job.

## 2. Files: registration and body

- A map is two files: an `.adp` registration whose first line is `wardley/map`, and the body, `<name>.owm` (`backend/ServiceCollection.AddWardleyMap.cs`). DISL's `persistence.files` can name the body but not a registration file owned by the host.
- The `.wm` extension is read as well as `.owm` (`backend/WardleyParser.cs`, Req 2). DISL has one `fileExtension`.
- A new map's body is exactly `title <baseName>\n`, with an LF terminator (`backend/ServiceCollection.AddWardleyMap.cs`). DISL can set a diagram attribute default, but not a default computed from the file name, nor the bytes of an empty document.
- A third file, `<name>.identities.json`, sits beside the body (section 4).

## 3. The `.owm` format

DISL's persistence is a structured serialization (JSON, YAML and the like). A Wardley map is stored in the Online Wardley Maps text DSL instead, so the `.dis` names a required `plugin:net.etalii.adp.wardley.owm` format. What the plugin does is below.

### 3.1 Statements

| Statement | Model | Notes |
|---|---|---|
| `title Text` | diagram `title` | |
| `style Name` | diagram `style` | Kept; not rendered. |
| `size [width, height]` | diagram `size` | Pixels of the reference renderer; kept, not used (the canvas fits the pane). |
| `component Name [v, m] label [dx, dy] (decorators) inertia url(name)` | `Component` | The label offset is in pixels and kept as view data (`labelOffsets`). |
| `anchor Name [v, m]` | `Anchor` | |
| `submap Name [v, m] url(name)` | `Submap` | |
| `url name [address]` | diagram `urls` | |
| `A->B`, `A+>B`, optionally `; context` | `Dependency`, `Flow` | Endpoints are names. |
| `evolve Name m`, `evolve Name->NewName m` | `Evolution` | |
| `pipeline Parent { component Child [m] ... }` | `Pipeline` + `PipelineComponent` | Nested form. |
| `pipeline Parent [a, b]` | `Pipeline` with `form: legacy`, `legacyExtent` | Legacy form, section 5. |
| `note text [v, m]` | `Note` | |
| `annotation N [v, m] text`, `annotation N [[v, m], [v, m], ...] text` | `Annotation` + `AnnotationPin`s | |
| `annotations [v, m]` | diagram `annotationsPosition` | Kept; the legend is not drawn. |
| `accelerator Name [v, m]`, `deaccelerator Name [v, m]` | `Accelerator` | |
| `pioneers|settlers|townplanners [v1, m1, v2, m2]` | `Attitude` | |
| `y_axis`, `x_axis`, `evolution`, `presentation`, any unknown keyword | not modelled | Preserved verbatim (`backend/WardleyParser.cs`, Req 3). |
| `// comment` | not modelled | Anywhere on a line, except where `//` follows `:` (so `https://` in a url is not a comment). |

Coordinates are written `[visibility, maturity]`, in that order; the fixtures readme records that the real parser confirms it ("the module's most error-prone single line"). DISL's placement binds `x` to maturity and `y` to visibility, which is the swap `backend/WardleyAxis.cs` makes: canvas x = maturity, canvas y = 1 − visibility.

### 3.2 Vocabulary settled by certification

The fixtures are certified against the real Online Wardley Maps parser vendored by `cli-owm` 0.0.2 (`Fixtures/readme.md`, `certify.mjs`). Certification found that **`market` and `ecosystem` are decorators, not statement kinds**: `market Foo [0.6, 0.8]` is a parse error and the real form is `component Foo [0.6, 0.8] (market)`. So there are three statement kinds and five decorators, and `inertia` is a separate boolean. This contradicts Req 5.1 and 6.3, which the implementation corrected; the `.dis` follows the implementation. Unknown decorators are ignored when reading and preserved in the text (`backend/WardleyDecorators.cs`).

### 3.3 Byte-identical round trip

- A document ADP did not change is written back byte for byte, including each line's own terminator (LF, CRLF, mixed, or none on the last line) and runs of blank lines (Req 3.1; `backend/WardleyWriter.cs`; fixtures `crlf-line-endings.owm`, `mixed-line-endings.owm`, `no-trailing-newline.owm`, `blank-line-runs.owm`).
- An edit splices only the span of text it changes: a move rewrites the coordinate span, a rename rewrites every span that names the element (links, evolutions, pipelines), a toggle rewrites the decorator or `inertia` span (`backend/WardleyWriter.cs`).
- New statements are appended at the end of the document.
- DISL's `persistence` aims at byte-identical output from two runtimes writing the same model; it has no notion of preserving an existing document's own bytes, comments or unmodelled statements. That is the plugin's whole job.

### 3.4 Undo

- A move is undone by restoring the exact original line (`backend/Commands/RestoreWardleyLineCommand.cs`).
- Add, remove and rename are undone by restoring the whole previous document text (`backend/Commands/WardleyElementCommandHandlers.cs`).
- A reload caused by an external change to the file is not an undoable step.

DISL says every transaction is undoable; it does not say how the stored text is restored, which here decides whether an undone edit leaves the file byte-identical.

## 4. Identifiers

- The `.owm` text has no element ids; references are by name. ADP gives every element a ShortGuid (base36) and keeps them in the sidecar `<name>.identities.json`, keyed by a kind plus a natural key: component name; link source, target and kind; pipeline parent; pipeline parent plus child name; note text; annotation number; accelerator; attitude (`backend/WardleyIdentities.cs`, `backend/WardleyIdentityKeys.cs`, `_Model/WardleyIdentityKind.cs`).
- The sidecar is an optimisation. A missing or stale one is reconciled against the document on load, so ids survive edits made in other tools where the natural key still matches (`WardleyIdentities.Reconcile.Tests.cs`).
- A rename carries the element's id to its new name, because the writer matches names by span rather than by text.
- The `.dis` declares `ids.strategy: uuid-v4` because DISL has no strategy for "ids held outside the document, keyed by a natural key". Section 11 lists this as a gap.

## 5. Pipelines

- **Nested form** `pipeline Parent { component Child [m] }`: a child has a maturity only, and is drawn at the parent's visibility. Dragging a child changes its maturity only. The `.dis` expresses this with a computed `y` and `movable: {x: true, y: false}`.
- **Legacy form** `pipeline Parent [a, b]`: no children; the two numbers are kept as written (`LegacyExtent`) and written back unchanged. "Add to pipeline…" is not offered on a legacy pipeline. The `.dis` marks `form` and `legacyExtent` read-only and disables the operation, but it cannot say "keep this statement's text as it is", which the plugin does.
- An empty pipeline is allowed (fixture `pipelines-both-forms.owm`).
- The canvas draws **no pipeline box** (`client/WardleyCanvas.tsx`); only its components are drawn. The `.dis` gives `Pipeline` the `none` shape.
- On the wire, adding to and removing from a pipeline are the core's Group and Ungroup requests (`api/wardley-map.proto`, `backend/WardleyContextActionProvider.cs`).

## 6. Notation details DISL does not pin down

### 6.1 Canvas space

- The client draws in a square space of 1000 units with a margin of 90 and a mark radius (`DOT`) of 9 (`client/WardleyCanvas.tsx`). The `.dis` sets canvas bounds 1000 × 1000 and marks of 18 × 18.
- The space is **stretched to the pane's aspect ratio**, so the map always fills its pane. DISL canvas bounds are fixed; it has no "fit and stretch" mode.
- The document's `size [w, h]` is not used for this.

### 6.2 Evolution stages as bands

- The evolution axis is divided into four stages: Genesis [0, 0.175), Custom Built [0.175, 0.4), Product (+rental) [0.4, 0.7), Commodity (+utility) [0.7, 1] (`backend/WardleyEvolution.cs`). The boundaries are the reference renderer's offsets 3.5, 8 and 14 divided by its 20-unit scale.
- The backend sends them to the client as a synthetic element, `wardley/map+evolution-axis`, rather than the client hard-coding them (`backend/WardleyElementTypes.cs`, `backend/WardleyElementMapper.cs`).
- The client draws each stage as a vertical band of increasing opacity (0.35, 0.5, 0.65, 0.8), separated by dashed lines (dash 6 6), with the stage's name as a label at font size 20 (`client/wardley.css`).
- DISL has ordinal axes with bands and alternate grid fills, but no way to divide a **linear** axis into named, unequal ranges with their own fills and labels. The `.dis` carries the stages as the `EvolutionStage` enum and a derived `stage` attribute, which is what the property grid shows.

### 6.3 Axes

- The vertical axis is labelled "Value chain", with end labels "Visible" at the top and "Invisible" at the bottom; the horizontal axis is labelled "Evolution"; both axis lines have stroke width 2 (`client/WardleyCanvas.tsx`). DISL axes have one `label`; the two end labels are not expressible.

### 6.4 Marks and labels

- Component: circle, surface fill, text-colour stroke of width 2. Anchor: filled square. Submap: a ring inside a fainter outer ring (outer stroke 1, opacity 0.6). All match the `.dis`.
- The name label sits beside the mark at (DOT + 6, 4), or at the document's own `label [dx, dy]` offset when the component has one. The label offset is read-only in ADP: it is shown in the property grid when present and written back unchanged, but the label cannot be dragged. DISL's label `position` is static, so the per-element offset is carried as the `labelOffsets` view data the `.dis` declares.
- The decorators and "inertia" are shown as one muted line under the name, joined by " · ", 12 px, 14 below the name's baseline.
- Inertia is also drawn as a vertical bar at the mark's right + 4, from 6 above the mark to 6 below it, stroke width 4 with a round cap. The `.dis` approximates it with a `line` badge.

### 6.5 Links and evolutions

- Dependency: straight, muted, width 1.5, no marker. Flow: text colour, width 3, no marker. Links are not selectable on the canvas; they are selected and removed through the component's context menu or the property grid (`client/WardleyCanvas.tsx`).
- Evolution: a dashed ring (dash 4 3) at the target maturity and the component's visibility, joined to the component by a dashed line (dash 8 6, width 2) with no marker. Neither is selectable nor draggable; the target is changed with "Change evolution target…".

### 6.6 Commentary

- Attitudes: a background rectangle with fill opacity 0.05 and a dashed (4 4) border, labelled with its kind. The corners are written `[v1, m1, v2, m2]` and may be given in any order; the client takes the minimum and maximum of each axis (`client/WardleyCanvas.tsx`). DISL's `x`/`x2` and `y`/`y2` bindings assume `x < x2` and `y < y2`.
- Accelerators: a short rule 16 wide with the name at offset (22, 4); a deaccelerator is dashed.
- Notes: 14 px text.
- Annotations: every occurrence is a circle of radius 11 with stroke width 1.5, carrying the number, with the text as its tooltip. The `.dis` models occurrences as `AnnotationPin` children so each is placed; the standalone model holds them as a coordinate list on one annotation (`_Model/WardleyAnnotation.cs`).
- The annotation legend (`annotations [v, m]`) is parsed and kept but not drawn.

## 7. Interactions

### 7.1 Moving

- Only components, anchors, submaps and pipeline components are draggable. The drag is **clamped** to the map (0..1 on both axes) on the client and then sent as one `MoveElementTo` document command (`client/WardleyCanvas.tsx`, `backend/Commands/MoveWardleyElementCommand.cs`).
- A value typed into the property grid outside 0..1 is **refused**, not clamped (`backend/WardleyContextPropertyProvider.cs`).
- DISL's `std.axisBounds` prevents a placement outside the bounds; it does not distinguish clamping a drag from refusing a typed value.

### 7.2 Adding

- The toolbox order is Component, Anchor, Market, Ecosystem, Submap, Pipeline, Note, Annotation, each with its description text (`backend/WardleyToolboxProvider.cs`). There is no Link entry: links are made from a component's context menu.
- The standalone asks for the name (or the note's text) **before** creating the element, rather than creating it and editing the label. The `.dis` says `after: editLabel`, which is the nearest DISL has.
- New elements land at [0.5, 0.5], because no drop position reaches the toolbox provider. This is a known gap against Req 13.4, which asks for the drop position (section 10).
- A new annotation takes the number one past the highest in use.
- Pipeline is dropped on a component, which it starts the pipeline of, or adds to.

### 7.3 Context actions

Nothing is offered on a read-only file: `backend/WardleyEditability.cs` reads the file system's read-only attribute, and the reason shown is "This map's file is read-only." The `.dis` uses `env.readOnly`.

For a component (and anchor or submap), in this order (`backend/WardleyContextActionProvider.cs`):

- **Rename…** (F2).
- **Remove** (Delete), with the confirmation "Remove '{name}', and every link and evolution that names it?", styled as dangerous. The `.dis` confirmation cannot interpolate the name: an operation's `confirm` is plain localized text.
- **Evolve to…** when the component does not evolve, **Change evolution target…** when it does; **Stop evolving** only when it does.
- **Mark as having inertia** / **Clear inertia**.
- One toggle per decorator, id `wardley.toggle-<decorator>`, labelled **Mark as <decorator>** or **Clear <decorator>**.
- **Link to…** and **Flow to…**, answered by a choice list of the other elements' names; **Remove link…** only when the component has links, again with a choice list.
- **Start a pipeline…** when the component has no pipeline, **Add to pipeline…** when it has a nested one; neither on a legacy pipeline.

For a pipeline component: **Remove from pipeline** (Delete). For a link: **Remove link** (Delete). Notes, annotations, accelerators and attitudes have no actions.

DISL context tools have a static `label`; labels that change with the element's state, and entries answered by a picker of names, are not expressible. The `.dis` lists each entry once with its primary label.

### 7.4 Properties

Groups Identity, Position, Strategy and Links (`backend/WardleyContextPropertyProvider.cs`):

- Component: Name, Kind (read-only), Visibility, Maturity, Evolution stage (read-only), Label offset (read-only, only when present), Evolves to, Becomes (the `->NewName` of an evolve), Inertia (a toggle, always shown), Decorators (a comma list), Url, or "Opens" for a submap (read-only).
- Pipeline component: Name (read-only), Kind "pipeline component", Visibility (the parent's, read-only), Maturity, Stage.
- Link: From, To and Kind (read-only), Context (editable, shown only when present).
- Pipeline: Name, Visibility, Components.

Every read-only row carries its reason as a full sentence. DISL form items have `readOnly` but no reason text.

## 8. Rules DISL cannot state

The standalone rules are `backend/WardleyRuleSet.cs` and `backend/WardleyValidator.cs`:

| Rule | Severity | In the `.dis` |
|---|---|---|
| `wardley.link-target-missing` | error | `std.references` |
| `wardley.evolve-target-missing` | error | `std.references` |
| `wardley.pipeline-parent-missing` | error | `std.references` |
| `wardley.coordinate-out-of-range` | error | `std.facets` on `Fraction`; covers every positioned value, including pipeline components, evolve targets, notes, accelerators, attitudes and annotations |
| `wardley.duplicate-name` | error | `uniqueName`, among components, anchors and submaps only |
| `wardley.anchor-missing` | warning | `anchorMissing`, only when there are components |
| `wardley.url-undefined` | warning | `urlUndefined` |
| `wardley.submap-missing` | warning | **not expressible** |

- **Dangling references are kept.** Because references are names in text, a link or evolve naming a missing component stays in the document and is reported, rather than being impossible as a DISL reference would be. `std.references` reports; it cannot describe a reference that exists only as text.
- **`wardley.submap-missing`** needs the file system: it fires when a submap's url address looks like a local path (no scheme, no `:`), resolves inside the project root, and does not exist. CEL has no file-system access.
- **Locations** are reported as lines of the `.owm` for link problems and as elements otherwise; DISL problems attach to elements and attributes only.
- `url` addresses are untrusted: they are never fetched or opened automatically.

## 9. Runtime behaviour specific to the standalone host

- **Several connections** to one map receive deltas, filtered to the viewport each client is showing (`backend/WardleyElementMapper.cs`, `backend/ServiceCollection.AddWardleyMap.cs`).
- **External edits**: when the `.owm` or its registration changes on disk, the store re-reads it and every open session hears. A read that fails keeps the last good document; only the watcher's delete clears it (`backend/WardleyDocumentStore.cs`). Saves are published atomically.
- **Layout** is manual only; there is no arrange command.
- **Fixtures** are marked `-text` in `.gitattributes` so their bytes survive checkout; no third-party map content is committed, because the Wardley mapping method's material is CC BY-SA 4.0 (`Fixtures/readme.md`).

## 10. Known gaps in the standalone implementation

- Links to pipeline components are reported as missing, because link endpoints are checked against components only (inferred from `backend/WardleyValidator.cs`). The `.dis` also limits link ends to `Positioned`, matching the implementation.
- Only the first `evolve` statement for a component is used (`FirstOrDefault` in `backend/WardleyElementMapper.cs`); the `.dis` warns with `oneEvolutionPerComponent`.
- Toolbox adds land at [0.5, 0.5] instead of at the drop position (Req 13.4).
- The annotation legend is not drawn; `style` and `size` do not affect rendering.
- Multi-selection is out of scope (design.md, after the core context service).

## 11. What DISL 0.1 lacks for this type

- A persistence format that is a **text DSL with byte-preserving round trip**, comments and unmodelled statements; the `.dis` delegates it to a required plugin.
- **Ids held outside the document**, keyed by natural keys, in a sidecar.
- **References by name** that may dangle and are rewritten on rename.
- **Named, unequal ranges on a linear axis** drawn as bands with labels.
- **End labels** on an axis.
- A canvas that **stretches to the pane**.
- **Per-element label offsets** in the model rather than in view data.
- **State-dependent context-menu labels**, and a context tool answered by a **picker of existing elements**.
- An operation **confirmation that interpolates** the element's name.
- A **read-only reason** on form items.
- Rules that consult the **file system**.
- Distinguishing **clamping a drag** from **refusing a typed value**.
