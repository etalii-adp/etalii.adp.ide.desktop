# Databricks bundle: what DISL and FBL do not say

This is the companion to [databricks-bundle.dis](databricks-bundle.dis). The definition states the bundle diagram as the standalone tool implements it today, in `src/diagrams/databricks/` of [etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone): its metamodel, notation, toolbox, menus, rows, rules, operations, simulated deploy, layout choice and persistence, and, in its `x-databricks` block, the wire ids the tool uses for each of them. That module serves three tools from one engine: this one, [databricks-job](databricks-job.dis) and [databricks-pipeline](databricks-pipeline.dis).

This page lists what DISL cannot express about this tool, in 0.3, in which the definition is written, and still in 0.5; then every language gap the conversion has found, DISL's and FBL's, each with its construct, the document that shows it and what would resolve it; and what the standalone module keeps in code beside the definitions, with the gap or the host concern that keeps each class. Each item is a finding of the standalone specification [databricks-disl-fbl](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/.spec-workflow/specs/databricks-disl-fbl), whose `requirements.md` gives its evidence; every sentence, code and severity quoted here is taken from that specification's transcript of the tool.

## A8: the order of findings, and whose findings they are

**What the tool does.** The tool judges the whole file, not the reading that asked. Opened as a bundle, a `databricks.yml` reports the findings of every job it declares, then the bundle's own, then those of every pipeline it declares; opened as a job or a pipeline, the same file reports the bundle's `databricks.no-default-target`, `databricks.multiple-default-targets` and `databricks.override-of-undeclared` too. The bundle's own findings come in that order: the missing default target, then each second default target, then each override of an undeclared resource, target by target.

**Why DISL 0.3 cannot say it.** A definition evaluates its constraints over its own reading: the bundle definition does not see a job's tasks or a pipeline's libraries, and the job and pipeline definitions do not see the bundle's targets. DISL 8.6 orders the findings of one definition; it has no way to join those of three.

**What the definition does instead.** It declares each of the bundle's rules with its code, severity, sentence and location, in the tool's order, and leaves the other readings' findings and their place to the host, which joins the three definitions' findings in the tool's order.

## A10: the bundle node exists without an entry

**What the tool does.** The bundle reading always draws one node with the id `bundle`, labelled with the bundle's name, or `bundle` when it has none. It is drawn when the file has no `bundle:` key, when the file is empty, and when the file cannot be read at all. It can be moved, and its position is stored in the registration under that fixed id. Renaming it in a file without a `bundle:` section is refused: "The file has no bundle: section to name."

**Why DISL 0.3 cannot say it.** DISL creates an element from what the reader reads, or derives one from other elements (4.11). Whether a derived element may exist while the entry it would be derived from does not, and keep a stored position, is not stated, and nothing else makes an element that no entry of the file stands for.

**What the definition does instead.** `Bundle` has a multiplicity of exactly one, an optional `name` and the fixed id `bundle`; the node's presence without an entry is the host's.

## The gaps

Nine, each a candidate change for this repository, numbered as the conversion's requirements number them (section C); none was added by the implementation, which found one behaviour to settle with the binding instead (below). The same nine stand on all three companion pages, because the three tool types are three readings of one file. The documents named are the fixtures of the standalone test project, under [`src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks.Tests/Fixtures/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks.Tests/Fixtures), whose `readme.md` says which finding each was written for, and the example `examples/lakehouse/` of the module; the parity transcript `Parity/databricks.transcript.json` beside them records what the tool does with each. None of the nine is worked around in a definition by stating something the tool does not do.

| # | Language | Construct | Finding | Status |
| --- | --- | --- | --- | --- |
| 1 | FBL | Several readings with different metamodels over one binding | B1 | Open; the conversion's second stage waits on it |
| 2 | FBL | A resource capture per reading | B2 | Open; the second stage waits on it |
| 3 | FBL | A YAML scalar as written | B4 | Answered by FBL 0.5 |
| 4 | FBL | Writing into an empty or one-line flow collection | B12 | Open |
| 5 | FBL | A refusal's sentence | B19 | Open |
| 6 | FBL | Which rule plans an add; two containers created by one insert; a nested mapping written for a YAML sequence item | B17 | The first two answered by FBL 0.4 and 0.7; the third open |
| 7 | FBL | A slot read when another is absent; a lowercased key placeholder | B18, B20 | Open |
| 8 | DISL | A placeholder node for a reference that names nothing; an element with no entry of its own that keeps a stored position | A9, A10 | Open |
| 9 | DISL | The order of findings across rules and readings; the first free name of a pattern | A7, A8, A11 | Open |

### 1. FBL 9.2 and 5.1: several readings with different metamodels over one binding (B1)

- **Construct.** FBL 9.2 makes every reading of a body use one binding, and opens a tool type that names another read-only. One binding for the three readings then meets FBL 5.1, under which an entry becomes at most one element: `resources.pipelines.<key>` is a `Resource` to the bundle and the `Pipeline` to the pipeline reading, and `resources.jobs.<key>` is a `Resource` to the bundle and the diagram itself to the job reading. A `typeMap` (DISL 11.2) renames a binding's types per definition, but can hide another reading's type only `as: "header"`, which it is not.
- **Shown by.** `examples/lakehouse/databricks.yml`, opened as the bundle by `databricks.adp` and as the pipeline by `databricks.pipeline.adp`, both editable in the transcript; `two-jobs-two-pipelines.yml` under all three readings.
- **Would resolve it.** A rule's `readings`, listing the origins it is read for; or a `typeMap` entry `as: "none"`; or 9.2 relaxed for bindings of one FBL document that declare each other compatible.
- **Meanwhile.** The body is read by the module's three parsers, and `DatabricksDisl` builds each reading's model from what they read (the first stage, Q2).

### 2. FBL 8.2 and 9.1: a resource capture per reading (B2)

- **Construct.** `registration.resource.capture` is one capture name for the whole binding. This file needs `job` for the job reading, `pipeline` for the pipeline reading and none for the bundle, which sees every resource; without a header the library selects the first value, so under one capture the bundle would draw one pipeline. FBL 9.1 keeps one model per open body, while two job registrations of one file select different jobs.
- **Shown by.** `two-jobs-two-pipelines.yml`, whose two job and two pipeline registrations each show and edit their own resource; the two `registration-resource` divergences of `databricks.pipeline.adp` (both copies of the example) in the standalone's `RealFiles/divergences.json`.
- **Would resolve it.** `registration.resource` as a map from origin to capture, and a sentence in 9.1 saying that one open body has one model of which each registration's selection is a view.
- **Meanwhile.** `DatabricksHeaders` reads the header, and `DatabricksSelection` selects the resource the target's registration names (`ContextTarget.RegistrationPath`), for the canvas, the menus, the rows, the drops and the commands alike (finding B3, repaired).

### 3. FBL 4.1.5 and 5.2: a YAML scalar as written (B4)

- **Construct.** FBL 0.2 typed a plain scalar by the YAML core schema, and a value that did not convert made its entry unreadable: a task key `01`, a bundle named `2024` or `null`, `num_workers: ${var.workers}` and `serverless: ${var.serverless}`, which would have cost the cluster or the whole pipeline in a file that belongs to another tool.
- **Shown by.** `typed-scalars.yml`, `variables-in-typed-places.yml`.
- **Answered.** FBL 0.5, by etalii.adp's feature `015-fbl-scalar-as-written`, gives every scalar its written text (4.1.5) and a slot `read: "text"`, and keeps an entry whose value does not convert readable with a `fallback` (5.2); the standalone library reads all three. What is left is the binding's: each attribute read as text or given a fallback, when the binding is written. It no longer stands in the way of the second stage.
- **Meanwhile.** The parsers keep every scalar as the text it was written with.

### 4. FBL 4.3 and 6.2: writing into an empty or one-line flow collection (B12)

- **Construct.** A YAML flow collection is one value whose only writable span is the whole collection (4.3), and no splice opens one into block form or adds an item inside it, so neither a task added to `tasks: []` nor a dependency added to `depends_on: [{task_key: a}]` can be planned. A JSON array written on one line is answered since FBL 0.3 (6.2).
- **Shown by.** `flow-collections.yml`, `flow-dependencies.yml`, `settings-one-line.json`.
- **Would resolve it.** A sentence in 4.3 or 6.2 that an insert into an empty flow sequence writes a block sequence holding the entry, and that an insert into a one-line flow sequence writes the item inside it after a `, `, with a conformance fixture each; it is the timeline's gap 2 with the second case added.
- **Meanwhile.** The module refuses such an edit before any splice, with the sentence ruled on 2026-10-19: "This list is written on one line, which this diagram cannot change yet; edit it in the text editor."

### 5. FBL 5.2 and 6.2: a refusal's sentence (B19)

- **Construct.** An attribute's `empty: "refuse"` and `absent: { "json": "refuse" }`, an insert's `when` and a missing container refuse with a sentence the library builds from type names; only `readOnly` takes a text of its own (3.4). The tool's sentences are its own: "The job has no tasks section to add into.", and `The settings file has no "schema" to rewrite.`, which a planner's sentence matches only because the binding happens to be called `settings`.
- **Shown by.** `bundle.yml` (the transcript's task added to a file without `tasks:`), `legacy-target.json` (the transcript's schema rewrite).
- **Would resolve it.** A LocalizedText beside `empty: "refuse"`, `absent`, an insert's `when` and its `create`, as `readOnly` already takes one.
- **Meanwhile.** The module's writers refuse in their own words; under the binding the sentence will come from the definition's check before the plan, or from module code named as standing in for this gap.

### 6. FBL 6.2 and 6.3: which rule plans an add, two containers in one insert, a nested mapping for a YAML item (B17)

- **Construct.** A dropped task's skeleton depends on its type (three types are reachable from the palette, seven are known to the writer); a resource added to a file without `resources:` creates two containers at once; and a YAML library, `- notebook:` with `path:` beneath it, is a nested mapping whose `kind` and `path` the example binding binds by expression, so the one-place rule (3.3) leaves it unwritable, while its `emit` covers JSON only.
- **Shown by.** `every-task-type.yml`, `no-resources.yml`, and the libraries of `examples/lakehouse/databricks.yml`.
- **Answered in part.** An add is planned by the first rule of its type whose insert's `when` holds (6.2), so a rule per task type carries that type's skeleton; and every missing level of a container is created by one insert (6.2, *Missing levels*, FBL 0.4).
- **Would resolve the rest.** An `emit` for a YAML sequence item, or a sentence that a `{key}` slot's `create` with `child` (FBL 0.4) applies to an add as it does to a set, with a conformance fixture adding a library to a YAML pipeline.
- **Meanwhile.** `PipelineWriter` writes the library's lines.

### 7. FBL 5.2 and 13: a slot read when another is absent, and a lowercased key placeholder (B18, B20)

- **Construct.** A pipeline's schema is read from `schema`, else from the legacy `target`, and written to `schema`; `override` is the reverse, a second slot that wins on read when it is present. A new document's key is the file's base name lowercased; FBL 13's `{key}` replaces what is not an ASCII letter, digit or `_` and prefixes `_` before a leading digit, and does not lowercase.
- **Shown by.** `legacy-target.yml`, `legacy-target.json`; the transcript's `newDocuments` section.
- **Would resolve it.** A slot option naming a second slot read only when the first is absent (under a name of its own, since `fallback` is FBL 0.5's for a value that does not convert); and a lowercased key placeholder, or a case option on a template.
- **Meanwhile.** `PipelineParser` and `PipelineWriter` read and write the schema; `ServiceCollectionAddDatabricksExtension.EmptyDocument` writes the three new documents.

### 8. DISL 4.11 and 6.10: a placeholder node, and an element without an entry (A9, A10)

- **Construct.** A9: a `depends_on` naming no task draws one movable node per missing key, with a stored position, where DISL keeps a relation without a source and draws nothing, and a stub (6.10) is a line for a relation without a *target*. A10: the bundle reading always draws `bundle`, and the pipeline reading `target` and `compute`, each under a fixed id with a stored position, where DISL does not say whether a derived element may exist while its source entry does not. Both are written out on the companion pages: A9 on the [job's](databricks-job.md), A10 on the [bundle's](databricks-bundle.md) and the [pipeline's](databricks-pipeline.md).
- **Shown by.** `missing-upstream.yml` (A9); `no-bundle-key.yml`, `empty.yml`, `broken.yml` (A10).
- **Would resolve it.** A relation end that names nothing drawn as a placeholder node of a declared type, built from the written key, with view data under an id made from it; and a derived element allowed with no source entry, keeping the view data of its fixed id.
- **Meanwhile.** `DatabricksElementMapper` draws the missing task's stub; `DatabricksDisl.Fixed` emits the elements the file has no entry for.

### 9. DISL 8.6 and 11.5: the order of findings, and the first free name (A7, A8, A11)

- **Construct.** A7 and A8: the tool reports the findings of the whole file, every job's, then the bundle's, then every pipeline's, whichever reading asked, and within a job task by task; a definition evaluates its own reading, and neither DISL 8.6 nor `constraints.order` interleaves several rules per element. A11: a dropped task is `<type>_<n>` with the first free `n` from 1, where CEL has no range to search and `persistence.ids.suffix` numbers only a second holder, from 2. All three are written out on the companion pages: A8 on each, A11 on the [job's](databricks-job.md).
- **Shown by.** `finding-order.yml`, on which none of the 120 orders `constraints.order` can be given yields the tool's order (`DatabricksFindings.Tests`, `ConstraintsOrderAlone_DoesNotGiveTheToolsOrder`); `two-jobs-two-pipelines.yml`, whose findings are the same under every reading (A7); `fresh-key.yml` (A11).
- **Would resolve it.** An order key per finding, such as its element's place in reading order before the rule's, and the findings of the readings of one body joined by the runtime (the C4 conversion's gap 10 is the same order key); for A11, a `suffix` that numbers the first id too, counting from a given number, or a CEL function for the first name of a pattern that no element has.
- **Meanwhile.** `DatabricksFindings` joins and orders; `DatabricksFreeKey` is the function `freeTaskKey` of the job definition's plugin `net.etalii.adp.databricks.taskKeys`.

### Found while implementing, and not a gap

- **A relation from the second holder of a key.** A model built from records whose natural ids repeat matches a relation's ends by key, so the dependencies written under the second task of a duplicated key landed on the first (`duplicate-key-on-a-circle.yml`, where it added two cycle findings). The first stage gives the second holder an ephemeral id and keeps its written key as a host attribute. What a binding should read there is for the binding step to settle against that fixture; it is not yet a construct either language lacks.

## Stand-ins

Code that stands in for a gap while it stands, named so that nobody reads it as the design:

| Stand-in | For | Where |
| --- | --- | --- |
| `DatabricksDisl` | Gaps 1 and 2 (Q2): each reading's model built from the module's own parsers | The module, until the body is read through the binding |
| `DatabricksDisl.Fixed` | Gap 8, finding A10 | The module |
| The missing task's stub in `DatabricksElementMapper` | Gap 8, finding A9 | The module |
| `DatabricksFindings` | Gap 9, findings A7 and A8 (Q3) | The module, joining what the three definitions evaluate |
| `DatabricksFreeKey` | Gap 9, finding A11 | The plugin `net.etalii.adp.databricks.taskKeys` of the job definition, supplied by the module |
| The refusal of an edit into a one-line list | Gap 4 | The module's writers |
| `useSimulatedRun.ts` | Runtime gap A6: no player for `simulate` in the standalone canvas library | The module's client |

## What stays code

Every class of the standalone module's backend project, [`src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks), as `git ls-files` lists them, beside the gap, the host concern or the stage that keeps it, as the conversion's first stage leaves it: the palettes, the menus, the rows, the removal confirmation and the findings are derived from these three definitions and the canvases are compiled from them, while the body is still read by the module's parsers and written by its commands. A class marked **waits** is kept for the second stage, which reads and writes the body through one binding once gaps 1 and 2 are answered, and names the task of the conversion that removes or changes it. The hand-written providers' logic and `DatabricksRuleSet` are no longer here: they are the oracle in the test project's `Parity/`.

| Class | Kept by |
| --- | --- |
| `DatabricksDefinition` | Host: the three definitions, each loaded once as a `DislDiagramDefinition`, and the palettes, menus, rows, removal confirmation and a dropped task's key derived from the one the target's reading names, mapped to the `x-databricks` wire ids; which reading of the file holds an element id. |
| `DatabricksDisl` | Gaps 1 and 2 (Q2): each reading's model built from the parsers' reading, with the second holder of a key under an ephemeral id; its `Fixed` is gap 8 (A10). **Waits**: deleted when the body is read through the binding (task 11). |
| `DatabricksFindings` | Gap 9 (A7, A8, Q3): the file's findings joined from the three definitions in the tool's order. |
| `DatabricksFreeKey` | Gap 9 (A11): `freeTaskKey`, the first free `<type>_<n>`. |
| `DatabricksValidator` | Host: the file's findings as the host's problems, one validator per origin, as a call to `DatabricksFindings`; a body that does not parse is one problem at the parser's line (B7). |
| `DatabricksToolboxProvider`, `DatabricksContextActionProvider`, `DatabricksContextPropertyProvider` | Host: handing out the derived palette, menus and rows, and building the module's command for a chosen entry, an edited row or a drop; the three simulated entries answered as nothing by their `.simulated.` marker. They hold no palette, menu or row of their own. |
| `DatabricksSelection` | Host: what an element id names in the parsed file, and the resource the target's registration selects (B3). **Waits**: the selection is the binding's once gap 2 is answered (task 11). |
| `DatabricksHeaders` | Gap 2: the `resource:` header of a registration, read as leniently as before (B21). **Waits**: out of the product code with the parsers (task 11). |
| `DatabricksContextSourceResolver` | Host: resolving a selection to an element of a file of its type. |
| `DatabricksNewPlacement`, `DatabricksRelationGesture` | Host: the stateless gesture ids `new:{x},{y}` and `rel:{from}->{to}`. |
| `DatabricksElementMapper` | Host: the `.proto` payloads and the viewport filter (A15). Gap 8: the drawn stub of a missing task (A9). |
| `DatabricksJobLayout`, `DatabricksBundleLayout`, `DatabricksPipelineLayout` | Host: the three layouts, which the definitions name as plugins (A15). |
| `DatabricksSession`, `DatabricksDocumentStore`, `IDatabricksDocumentStore` | Host: the shared document lifecycle and its reload, one session per open reading over one document and one history, and a move as one undo step through core's `SetRegistrationLayoutCommand`. **Waits**: a move written through FBL's registration writer (task 13, Requirement 5.5). |
| `ServiceCollectionAddDatabricksExtension`, `Diagram` | Host: the three registrations and their services, and the new document of each type (`EmptyDocument`). Gap 7: the lowercased key of a new document. **Waits**: the new documents from the binding's `template.byOrigin` (task 11). |
| `BundleParser`, `JobParser`, `PipelineParser` | Gaps 1 and 2: the reading each definition's model is built from. **Waits**: out of the product code when the body is read through the binding (task 11). |
| `DatabricksModel`, `BundleModel`, `BundleResource`, `BundleTarget`, `BundleInclude`, `BundleVariable`, `JobModel`, `JobTask`, `JobCluster`, `JobDependency`, `PipelineModel`, `PipelineLibrary`, `PipelineNotification`, `UnknownNode` | The parsers' reading, which the layouts, the mapper and the commands read too. **Waits**: with the parsers (task 11). |
| `BundleWriter`, `JobWriter`, `PipelineWriter`, `DatabricksSplices` | Gaps 4 to 7: the line-splicing writers, with the repairs of findings B9, B10 and B12 made in the first stage. **Waits**: every edit planned through the binding as splices (task 12), which also settles B8, B11, B13, B15, B16 and the rest of B12. |
| `InsertDatabricksTaskCommand`, `RemoveDatabricksTaskCommand`, `ConnectDatabricksTasksCommand`, `DisconnectDatabricksTasksCommand`, `RenameDatabricksTaskCommand`, `SetDatabricksRunIfCommand`, `SetDatabricksClusterCommand`, `AddDatabricksResourceCommand`, `SetDatabricksBundleNameCommand`, `InsertDatabricksLibraryCommand`, `RemoveDatabricksLibraryCommand`, `SetDatabricksPipelineScalarCommand` and their handlers, `DatabricksEditHandler` | Host: the command records the history stack and the wire carry, and the undo that restores the whole body. **Waits**: written through the binding (task 12). |

Beside the backend, `api/databricks.proto` holds the payloads. On the client, `BundleCanvas.tsx`, `JobCanvas.tsx` and `PipelineCanvas.tsx` compile their canvases from the three definitions, `DatabricksCanvas.tsx` builds the payloads they read, and `databricksBindings.ts` states what the canvas library cannot read from a definition, each entry with its reason. `useSimulatedRun.ts` plays the simulated run, deploy and update, standing in for runtime gap A6, and **waits** for a player of `simulate` in the canvas library (task 14).

## Sources

- The standalone module: [`src/diagrams/databricks/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/databricks), its parity transcript, fixtures and oracle in [`backend/EtAlii.Adp.Diagram.Databricks.Tests/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks.Tests).
- The conversion's requirements, design and tasks: [`.spec-workflow/specs/databricks-disl-fbl/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/.spec-workflow/specs/databricks-disl-fbl), whose findings (A1 to A15, B1 to B22), gaps (section C) and questions (Q1 to Q6) this page cites by number.
- [DISL](../../specifications/disl/DISL-specification.md), sections 4.11, 6.10, 8.6, 11.2 and 11.5; [FBL](../../specifications/fbl/FBL-specification.md), sections 3.3, 3.4, 4.1.5, 4.3, 5.1, 5.2, 6.2, 8.2, 9.1, 9.2 and 13.
