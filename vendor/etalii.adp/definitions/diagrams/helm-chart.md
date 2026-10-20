# Helm chart (`helm/chart`)

[`helm-chart.dis`](helm-chart.dis) specifies the Helm chart diagram in DISL 0.3: its metamodel, notation, derived relations, property rows, findings, refusals and wire ids. Its folder is read through the FBL 0.2 binding [`helm-chart.fbl`](../../specifications/fbl/helm-chart.fbl) by the plugin `net.etalii.adp.helm.chartFolder`. This page holds what DISL 0.3 and FBL 0.2 cannot state about the tool, each point tied to the fixture, test or code that shows it, and what would resolve it. Anything the two languages can state is in those two files, not here.

Paths below are in [etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone) on `develop` unless they say otherwise; the module is `src/diagrams/helm-chart/` (backend `backend/EtAlii.Adp.Diagram.HelmChart/`, client `client/`, wire `api/helm-charts.proto`).

## Sources

- The standalone module: code, tests and fixtures under `src/diagrams/helm-chart/`, the examples under `src/examples/diagrams/helm-chart/` and the screenshot `docs/screenshots/helm-chart.png`.
- The parity transcript `backend/EtAlii.Adp.Diagram.HelmChart.Tests/Parity/helm-chart.transcript.json`, generated from today's code by task 1 of the standalone specification `helm-chart-disl-fbl` (commit `2874c4d77`, pull request 303), and reproduced byte for byte by every later task. Every sentence, label, code and severity in `helm-chart.dis` is copied from it. The fixture charts named below are under `backend/EtAlii.Adp.Diagram.HelmChart.Tests/Fixtures/`.
- The standalone specification `helm-chart-disl-fbl` (requirements, design, tasks), whose findings A1 to A15 and B1 to B14 are cited below by number, and the original specification `helm-charts`, removed in `53f68611`, cited as R*n.m*.
- [Helm charts guide](https://helm.sh/docs/topics/charts/) and [chart template guide](https://helm.sh/docs/chart_template_guide/).

## The subject and what the plugin delivers

- **The subject is a folder.** A registration (`.adp`) inside the chart root, beside `Chart.yaml`, makes the folder a diagram; its first line is the origin `helm/chart`. There is no document extension and no body file (`Diagram.cs`: `Subject: DiagramSubject.Folder`). The binding is `registrationOnly` and recognises a folder holding `Chart.yaml`.
- **Nothing in the folder is written.** The binding is read-only with the tool's sentence, so its plugin is never asked to plan (FBL 11.4). `ZeroWrites.Tests.cs` opens, browses, validates and closes a fixture chart and compares every file byte for byte.
- **Live update.** The module watches every file under the chart root and hands each change to the library's folder body, which reads the folder again once no change has arrived for the binding's `settle` of 400 ms (`HelmBody`). The position a user drags is written into the registration, which the folder body watches as the registration: a change to it redraws every open session without reading the folder again (FBL 10.3, finding B12).
- **What is delivered, and when.** The whole diagram is read when it opens, and the session then narrows what it sends to the viewport: `HelmElementMapper.Visible` keeps every node whose box the viewport touches plus one hop outwards, so a relation always has both of its boxes, and `HelmSession.UpdateView` sends the difference against what the connection holds. A chart change is diffed the same way. Nothing folds, so no group or ungroup change is ever sent.
- **What the plugin delivers.** Every node of the metamodel, with `path` and its other stored attributes as `helm-chart.dis` declares them, its file and its line; the stored relations `Resolves` and `Includes`, with or without a target; and one finding per chart-owned YAML file that does not parse, worded as the tool words `helm.unreadable-yaml`. The ids, the three derived relations, every derived attribute, every property row and the other nine codes' findings come from the definition.

The plugin is the module's reader (`HelmChartReader`, `HelmYaml`, `TemplateScan`, `DependencyResolution`) behind the FBL contract, reading the files it is handed instead of the disk (standalone design, `HelmChartFolderPlugin`). How it reads the folder, which no binding can state while the reader is a plugin (gap 1):

- **Inventory.** `Chart.yaml`; `values.yaml` or `values.yml` as the default layer and every other top-level `values*.yaml` or `values*.yml` as an override, the extension compared ignoring case; `values.schema.json` (presence only); every file under `templates/`, recursively; the `.yaml` and `.yml` files under `crds/`, recursively; `charts/` (directories and `*.tgz` files); `Chart.lock`. For `apiVersion: v1`, compared ignoring case, dependencies come from `requirements.yaml` and the lock is `requirements.lock`. Anything else is ignored without complaint (R1.3).
- **Chart.yaml.** `name`, `version` and the line of `version:`, `appVersion`, `apiVersion`, `type` (absent reads as `application`), `description`, `deprecated` (read with .NET's `bool.TryParse`, so `true` and `True` count and `yes` does not), and `dependencies[]` with `name`, `alias`, `version`, `repository` and `condition`, sorted by effective name. An entry without a `name` is skipped. Every scalar is kept as written: `version: 1.0` is `1.0` and fails the SemVer rule with that spelling (fixture `scalars-as-written`).
- **Tolerant YAML.** A file that does not parse is a node marked unreadable with the parser's message and line; the rest of the chart still reads. A subchart's own `Chart.yaml` that does not parse is not a finding.
- **Templates are lines, not YAML.** `TemplateScan.cs` reads `kind:` and `apiVersion:` at column zero (a value containing `{{` is unknown, never guessed), `{{ define "name" }}`, and `{{ include "name" }}` or `{{ template "name" }}` with a literal double-quoted name. Lists are sorted ordinally and deduplicated. A template that cannot be read yields no facts and no error. A file whose name begins with `_` is delivered as a `Partial`.
- **Condition state.** A dependency's `condition` is walked as a dotted path into the parsed default values: `none`, `missing`, `on`, `off` or `unknown` (`HelmChartReader.Resolve`).
- **Vendored content.** Each directory under `charts/` is read one level deep: its `Chart.yaml` (name, version, type), the number of files under its `templates/` and the number of entries, directories included, in its own `charts/`. Each `*.tgz` is sealed: labelled by its file name less a `-<version>` tail whose first character is a digit (`StripVersionTail`), never unpacked.
- **Matching.** A dependency resolves to the first vendored entry whose entry name is its chart name, or its alias when it has one; without one, its `Resolves` has no target and is labelled with its effective name. An included name goes to the first partial that defines it; without one, its `Includes` has no target. Both are the stand-in for gap 7.
- **No execution, no network.** No template rendering, no hooks, no `helm` commands, no registry or cluster access. Reparse points are not followed.

## Language gaps

These are the candidate changes for etalii-adp/etalii.adp that the conversion found, numbered as the standalone specification's section C. None is closed by the definition or the binding stating something the tool does not do; where the tool keeps a behaviour the languages cannot state, the class that keeps it is named.

| # | Gap | Construct | Shown by | What would resolve it | Until then |
| --- | --- | --- | --- | --- | --- |
| 1 | **A mixed reader for a folder subject** (B6) | FBL 10.2 and 11: a binding with a plugin reader has no rules, so `Chart.yaml`, its dependencies, the lock and the values keys, which are plain YAML element rules could bind, are read by the plugin with everything else. The `family` of a plugin-read file rule has no stated effect. | Every fixture; FBL section 17 names "a declared binding, with a plugin for part of its reading" as an outcome without a construct | Rules for the file rules of a declared family, a plugin for the others, one model | The whole folder is one plugin read |
| 2 | **A file as an entry, and the file's path in a rule** (B8) | FBL 10.2: rules see a file's entries, never the file. A template that matches nothing must still be a node, and its role comes from its file's name and place. | `template-lines` (`nothing.yaml`, `.helmignore`, `tests/test-connection.yaml`), `unselected-files` (`Notes.TXT`) | A file rule with `element`, one element per selected file with its relative path as an attribute, and `file` in the CEL context of every rule that reads it | The plugin makes one node per file; the role is derived in DISL from `path` |
| 3 | **Directories as entries, and presence without bytes** (B4, B5) | FBL 10.2 and 11.2: file rules select files, and the plugin is handed "the relative path and bytes of every file the file rules select". An empty `crds/` is a node with a count of 0, a directory under `charts/` without `Chart.yaml` is a subchart, and the deeper count includes directories. The schema and each archive are read for their presence, and a subchart's templates for their number, yet their bytes are handed over and count toward FBL's size limit. | `empty-directories` (`crds/`, `charts/hollow`, `charts/nochart`), `unconventional` (`charts/deep/charts/deeper/marker.txt`: one deeper entry), `HelmFolderEntries.Tests`; the three divergences of the binding on its own in the standalone `RealFiles/divergences.json` (`charts/nochart`, `charts/one`, `charts/deeper`) | `entries: "files"` or `"all"` on a file rule, and `read: "path"` beside the default of bytes | The module's `HelmFolderEntries` hands the plugin directory names, and a file over the size limit is handed with its path and no bytes (standalone F4, F5) |
| 4 | **A folder as a fixture input, and a plugin's conformance** (B9) | FBL 11.4: `read` must be deterministic "in every host's implementation of the plugin", yet `helm.unreadable-yaml` carries YamlDotNet's own sentence and line, by design, and a round-trip fixture's `input` is one file, so a folder cannot be pinned. | `broken`, `finding-order`, `finding-shapes`, `scalars-as-written` (`Duplicate key replicaCount`) | A folder as a fixture input, and a statement of what a plugin's finding may say that another host must repeat | The standalone transcript and its cross-check pin the plugin |
| 5 | **Several entries from one statement, and a YAML scalar as written** (B7, B8) | FBL 4.6: a statement is one entry, so a line with two `include` actions cannot yield two references. The module also splits lines on LF only, where FBL 2.6 ends a line at a lone CR as well. FBL's YAML reading types plain scalars by the core schema, where the tool keeps every scalar as written and reads booleans with `bool.TryParse`. | `template-lines` (`two-per-line.yaml`, `split-action.yaml`, `lone-cr.yaml`), `scalars-as-written` | Several entries per statement; a slot that reads a scalar's written text everywhere a declared YAML binding would need it | The plugin reads both |
| 6 | **`suggest` for a folder body** (B14) | FBL 12.1: `suggest.contains` is defined over "a body's first 64 KiB", which a folder has not. The tool suggests itself on a folder that holds `Chart.yaml` (R2.3). | `Diagram.cs`; the binding, which therefore suggests nothing | `suggest` by the entries a folder holds | The module's own suggestion |
| 7 | **A derived relation without a target** (A5) | DISL 4.11.3: a derived relation's `target` is an Expression yielding an element, and an item whose ends do not satisfy the type is dropped, so a relation whose `target` end is `optional` can be stored without a target but not derived without one. `Resolves` and `Includes` are two lines of CEL each, and either may be open. | `open-ends` (`common` unvendored, `common.labels.standard` not defined here), `includes-per-name`, `same-named-dependencies` | `target` as an Expression that may yield `null` where the type's target end is `optional` | The plugin delivers both as stored relations (the module's `HelmOpenEndRelations`); `Declares`, `Overrides` and `Configures` are derived |
| 8 | **The source location of one attribute, and a computed severity on a built-in** (B10, B11) | DISL 12.2 has `location(attr)`, but under FBL 11.2 a plugin delivers the span "of the entry and of every writable value", and a read-only binding has no writable value, so nothing feeds it: `helm.version-not-semver` points at the line of `version:`, not at the chart's entry. A built-in's `code` may be computed in 0.3 and its `severity` may not, so `std.unparseable` cannot be the error `helm.unreadable-yaml` for a file and the warning `helm.not-a-chart` for a folder that fails `recognise`, which FBL 10.3 makes an unreadable body. | `finding-lines` (`Chart.yaml:9`), `not-a-chart` | Spans of read-only values from a plugin; `severity` as `{cel}` on a built-in | `Chart.versionLine`, read by the rule's `location`; the module's `HelmNotAChart` reports the one warning at the folder and leaves the model empty |
| 9 | **The order of reader findings across a folder's files, and of a group of duplicates** (A8) | DISL 8.6 orders the reader's findings in reading order, which for a folder is the ordinal order of relative paths, and a group of duplicates by the line of its first member. The tool reports unreadable files as `Chart.yaml`, `requirements.yaml`, the values files in stack order, the lock, then the CRD files, and name collisions and condition findings in the order of the dependencies' effective names. | `finding-order` (`values.yaml`, `values-b.yaml`, `crds/x.yaml`, where reading order is the reverse), `finding-shapes` (`values.yaml` before `crds/broken.yaml`), `same-named-dependencies` (`nats` at line 15 before `redis` at line 9) | A declared order of the reader's findings by file, and an order of a code's findings by an expression | The module's `HelmFindingOrder`: the derivation's transcript differed in order alone, in the name collisions of `same-named-dependencies` |

The second half of A8 does not occur. Under `constraints.order`, findings of one code are ordered by the line of the element the rule was evaluated for, not by the line of their location, and every `helm.lock-drift` finding is evaluated for the lock, so they keep the order of the rule's `forEach`: each declared name the lock lacks, then each stale entry by name. `finding-order` (`alpha`, `zeta` at the lock without a line, then `beta` at line 8 before `omega` at line 2) is reproduced by the definition as written.

## The stand-ins the module holds

Each is named for the gap it stands in for, and goes when its gap is closed (standalone design, *The four stand-ins*):

- **`HelmFolderEntries`** (gap 3): hands the folder body, as supplementary entries outside FBL 0.2, the directory names no file rule can select: the `crds/` folder itself, every directory under `charts/`, and every directory in a subchart's own `charts/`. It reads no file. With it, a file over the size limit is handed with its path and no bytes, and costs that file's detail only.
- **`HelmOpenEndRelations`** (gap 7): makes `Resolves` and `Includes` with today's ids, with no target for an open end.
- **`HelmNotAChart`** (gap 8): replaces the folder body's unreadable-body finding for a folder that fails `recognise` by the one warning `helm.not-a-chart` at the folder, and the model is empty.
- **`HelmFindingOrder`** (gap 9): puts each run of findings about dependencies in dependency order.
- **`versionLine` on the chart, and the line of each lock entry** (gap 8): the plugin delivers the lines `location(attr)` would give, and the rules' `location` reads them.

## What remains in the module, and why

Every type of the product project `backend/EtAlii.Adp.Diagram.HelmChart/`, checked against a listing of its files on `develop` after task 9 of the standalone specification, beside the gap or the host concern that keeps it; a type with neither is not there. A stand-in goes when its gap is closed; a host concern is the host's and stays.

| Type | Kept by |
| --- | --- |
| `HelmChartFolderPlugin` | The plugin reader of FBL 11.5 (gaps 1, 2 and 5): the reading the section above describes is no selector or slot. |
| `HelmChartReader`, `HelmYaml`, `TemplateScan`, `DependencyResolution` | The plugin's reading: the files the binding hands it, chart-owned YAML parsed with YamlDotNet with every scalar as written, the templates scanned as lines, and the dependencies matched against what `charts/` holds. |
| The records and enums of `_Model/` (`HelmChart`, `ChartMetadata`, `ValuesFile`, `SchemaFile`, `TemplateFile`, `TemplateFacts`, `TemplateRole`, `CrdsSummary`, `DependencyDeclaration`, `ConditionState`, `VendoredEntry`, `LockFile`, `LockEntry`, `ResolvedDependency`, `DependencyResolutionResult`) | What the plugin reads into, before it hands the model its elements. |
| `HelmFolderEntries` | Gap 3. |
| `HelmOpenEndRelations` | Gap 7. |
| `HelmNotAChart` | Gap 8. |
| `HelmFindingOrder` | Gap 9. |
| `HelmLayout` | The layout plugin `net.etalii.adp.helm.anatomyBands` the definition names (finding A15). |
| `HelmElementMapper` | The host's wire: the model in the element and delta vocabulary of `api/helm-charts.proto`, with today's delivery order of relations and the viewport filter. |
| `HelmContextSourceResolver` | The host's selection: an element id resolved against the folder its registration sits in. |
| `HelmDefinition` | The definition: the bundled `helm-chart.dis` and the binding, and what is derived from them. |
| `HelmBody`, `HelmReading`, `HelmChartStore`, `IHelmChartStore` | The opened subject: the library's folder body over the registered folder, the watcher it does not hold, and one open folder per registration for its viewers. |
| `HelmSession`, `HelmSessionFactory` | The host's session seam: one connection's viewport and what it has been sent, and the move into the registration through core's command. |
| `Diagram` | The host's catalog entry: the origin and a folder subject with no extension. |
| `HelmValidator` | Core's validator seam: the findings of one reading, handed over with project-relative locations. |
| `HelmContextPropertyProvider` | Core's property seam: the rows, read-only, refusing a write with the definition's `std.readOnly`. |
| `ServiceCollectionAddHelmChartsExtension` | The host's registration of the types above. |

Beside them the client keeps `client/helmBindings.ts`, which says what the library cannot yet read from the definition: the label it would compute with CEL, the class names that tie a kind to its theme hue, the forward Bezier route and the role and tab order (DISL 6.15). `client/handWrittenDefinition.ts` is test code only, the oracle the compiled canvas definition is held to.

## Differences the definition cannot bring in line

- **Two dependencies with one effective name** (A10, Q4). Both get the id `dep:` and the name. DISL 11.5.4 gives the id to the first in reading order and draws the second as ephemeral; today both are drawn, and their rows show the first's values (`same-named-dependencies`: both `nats` nodes show the constraint `1.0.0`, although the second declares `2.0.0`). Their relations repeat too: the second `Resolves` from `dep:nats` and from `dep:redis` has the first's id, which `std.duplicateId` would also report under `helm.name-collision`, and the second `Declares` to `dep:redis` (one constraint, `17.0.0`) and `Configures` to either has a derived id the first already holds, so it is not drawn (DISL 4.11.5). The definition switches `std.derivedId` off so that nothing is reported about the second. The repeated `Resolves` was ruled on 2026-10-16: one relation per id is delivered, so the plugin does not deliver the second either, which changes nothing the canvas shows.
- **The order of findings**, gap 9.
- **Severities** (Q3). `helm.legacy-chart` is declared `warning`, as the tool reports it, although its sentence reads as information; `helm.not-a-chart` stays the module's, gap 8.

## Persistence: positions in the registration

The only stored data is a `layout:` block in the registration, written by core's `RegistrationLayout` (`src/backend/EtAlii.Adp.Hierarchy/RegistrationLayout.cs`), which is FBL 8.3's layout block:

```text
helm/chart
layout:
  chart: 1327.665 566.461
  dep:common: 1715.631 587.676
  tpl:templates/_helpers.tpl: 585.618 1594.676
```

- One line per element a user dragged: two-space indent, id, colon, then x and y of the box's top-left corner in canvas units, with up to three decimals, entries ordered by id ordinally, CRLF line endings. Only the position is stored; sizes are fixed per kind.
- A stored position wins over the computed one element by element; an id that no longer exists is ignored when read and dropped at the next write (R6.3). FBL would also report it as `fbl.stale-view-data`, which the tool does not (Q4).
- A move is one undoable command (`SetRegistrationLayoutCommand`) and the diagram's only edit. The client stores the dropped position unrounded.
- A move under a parent is refused, with or without a history, with the sentence of the definition's `noReparent` gesture constraint. Two refusals stay the session's, because no DISL construct says them: a move of a relation is refused with "That element is not something this diagram can move.", and any move, of a node, a relation or an unknown id, made without a history is refused with "This diagram is read-only.". With a history, a move of an unknown id is accepted and written to the registration (transcript, `moves`).

## Layout

The `.dis` names the `net.etalii.adp.helm.anatomyBands` plugin with `layered` as fallback. The plugin is `HelmLayout.cs`, pure and deterministic:

- Five columns, left to right, starting at (40, 40), with 60 between columns and 24 between rows. A column is as wide as its widest node; an empty column takes no room.
  1. Metadata: chart, lock, schema, crds, in that order.
  2. Values: the default layer first, then overrides by path.
  3. Templates: manifests, partials, notes, test hooks; by path within each role.
  4. Dependencies, by effective name.
  5. Vendored content (subcharts and archives), by path.
- Node sizes are the `size.fixed` values in the `.dis`.

## Colours

The `.dis` tokens copy the light and dark values of the theme hues `--color-diagram-helm-{chart,values,template,dependency,subchart}` in `src/client/src/index.css`; fills are the hue mixed 14 % into transparent. A partial travels as its own wire type and is drawn as any template.

## Navigation

- Activating a node with a backing artifact reveals it in the project explorer (`revealPath`). A dependency has no artifact and nothing happens (R8.3). An archive is revealed like any other file, although its row says "there is no file to open".
- R8.1 asks for the file to open in a text editor; the implementation reveals it instead. R8.2 asks that a subchart with its own registration offer to open that diagram; that is not implemented.

## Known gaps of the tool

- **Not drawn on the canvas.** A template's kinds (R4.2), a condition badge and the pinned version beside the constraint (R5.5, R5.7) and the `global:` mark (R5.4) are rows only.
- **Notion's "why specialized"** says the view "shows which values feed which resources". It does not: templates are never rendered and values never resolved, so the diagram shows which values file configures which subchart.
- **Two defaults.** If both `values.yaml` and `values.yml` exist, both are default layers and overrides stack onto the first. Inferred from the code.
- **CEL.** The `.dis` validates against `disl.schema.json` and every expression parses as CEL, but no DISL type checker exists yet, so the expressions are not checked against the metamodel.

## Deliberately out of scope

Template rendering (no Go templates, no Sprig, so no Deployment-to-Service graph); the effective values of an environment (the diagram shows layering, not a merge); editing chart content; registry and cluster connectivity; evaluating `values.schema.json`; security scanning. No version range is ever evaluated: the lock's pin is display data.

## Examples

Under `src/examples/diagrams/helm-chart/`, each with its `.adp` in the chart root and upstream licence and provenance:

| Example | Source (Apache-2.0) | Shows |
| --- | --- | --- |
| `hello-world` | [helm/examples](https://github.com/helm/examples) | The minimal chart, untouched |
| `prometheus` | [prometheus-community/helm-charts](https://github.com/prometheus-community/helm-charts) | Conditional dependencies, all unvendored open ends |
| `nginx` | [bitnami/charts](https://github.com/bitnami/charts) | OCI dependency with the `common` library chart vendored under `charts/`, and ADP-authored `values-dev.yaml` and `values-prod.yaml` for the override stack |

The fixture charts under `backend/EtAlii.Adp.Diagram.HelmChart.Tests/Fixtures/` are protected from line-ending normalization by a local `.gitattributes`.
