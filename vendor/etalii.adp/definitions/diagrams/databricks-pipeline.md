# Databricks pipeline: what DISL and FBL do not say

This is the companion to [databricks-pipeline.dis](databricks-pipeline.dis). The definition states the pipeline diagram as the standalone tool implements it today, in `src/diagrams/databricks/` of [etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone): its metamodel, notation, toolbox, menus, rows, rules, operations, simulated update, layout choice and persistence, and, in its `x-databricks` block, the wire ids the tool uses for each of them. That module serves three tools from one engine: this one, [databricks-job](databricks-job.dis) and [databricks-bundle](databricks-bundle.dis).

This page lists what DISL cannot express about this tool, in 0.3, in which the definition is written, and still in 0.5; then every language gap the conversion has found, DISL's and FBL's, each with its construct, the document that shows it and what would resolve it; and what the standalone module keeps in code beside the definitions, with the gap or the host concern that keeps each class. Each item is a finding of the standalone specification [databricks-disl-fbl](../../specs/069-standalone-databricks-disl-fbl/), whose `requirements.md` gives its evidence; every sentence, code and severity quoted here is taken from that specification's transcript of the tool.

## A8: the order of findings, and whose findings they are

**What the tool does.** The tool judges the whole file, not the reading that asked. Opened as a pipeline, a bundle's YAML reports the findings of every job it declares, then the bundle's, then those of every pipeline it declares, whatever pipeline the `.adp` registration's `resource:` names. For each pipeline it reports `databricks.no-libraries`, then `databricks.schema-without-catalog`.

**Why DISL 0.3 cannot say it.** A definition evaluates its constraints over its own reading: the pipeline definition sees one pipeline, and it does not see a job's tasks or the bundle's targets. DISL 8.6 orders the findings of one definition; it has no way to join those of three.

**What the definition does instead.** It declares each of the pipeline's rules with its code, severity, sentence and location, in the tool's order, and leaves the other readings' findings and their place to the host, which joins the three definitions' findings in the tool's order.

## A10: target and compute exist without an entry

**What the tool does.** The pipeline reading always draws a node `target`, labelled with as much of `catalog.schema` as the settings name, and a node `compute`, labelled `serverless` or `cluster` as `serverless` says. Neither has an entry of its own in the settings; the one `notifications` node, drawn only when the pipeline has notifications, stands for the whole list. Each can be moved, and its position is stored in the registration under its fixed id.

**Why DISL 0.3 cannot say it.** DISL creates an element from what the reader reads, or derives one from other elements (4.11). Whether a derived element may exist while the entry it would be derived from does not, and keep a stored position, is not stated, and nothing else makes an element that no entry of the file stands for.

**What the definition does instead.** `Destination` and `Compute` have a multiplicity of exactly one and `Notifications` of at most one, each with its fixed id; that the first two are there without an entry is the host's.

## The gaps

Nine, each a candidate change for this repository, numbered as the conversion's requirements number them (section C); none was added by the implementation, which found one behaviour to settle with the binding instead (below). The same nine stand on all three companion pages, because the three tool types are three readings of one file. The documents named are the fixtures of the standalone test project, under [`src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks.Tests/Fixtures/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks.Tests/Fixtures), whose `readme.md` says which finding each was written for, and the example `examples/lakehouse/` of the module; the parity transcript `Parity/databricks.transcript.json` beside them records what the tool does with each. None of the nine is worked around in a definition by stating something the tool does not do. FBL 0.10 (etalii.adp's feature `066-fbl-conversion-gaps`) answers gaps 1, 2, 4, 5, the third part of 6, and 7, each below with the section that answers it and the fixture under `specifications/fbl/fixtures/` that shows it; the binding the three readings share is [`databricks.fbl`](../../specifications/fbl/databricks.fbl), written from the conversion's appendix draft in FBL 0.10's constructs. Gaps 8 and 9 are DISL's and stay open.

| # | Language | Construct | Finding | Status |
| --- | --- | --- | --- | --- |
| 1 | FBL | Several readings with different metamodels over one binding | B1 | Answered by FBL 0.10 (`databricks-readings`) |
| 2 | FBL | A resource capture per reading | B2 | Answered by FBL 0.10 (`databricks-resources`) |
| 3 | FBL | A YAML scalar as written | B4 | Answered by FBL 0.5 |
| 4 | FBL | Writing into an empty or one-line flow collection | B12 | Answered by FBL 0.10 (`databricks-flow-empty`, `databricks-flow-one-line`) |
| 5 | FBL | A refusal's sentence | B19 | Answered by FBL 0.10 (`databricks-refusals`) |
| 6 | FBL | Which rule plans an add; two containers created by one insert; a nested mapping written for a YAML sequence item | B17 | The first two answered by FBL 0.4 and 0.7; the third by FBL 0.10 (`databricks-library-add`); a resource added under its key by FBL 0.14 (`databricks-resource-add`) |
| 7 | FBL | A slot read when another is absent; a lowercased key placeholder | B18, B20 | Answered by FBL 0.10 (`databricks-secondary`, `databricks-template-key`) |
| 8 | DISL | A placeholder node for a reference that names nothing; an element with no entry of its own that keeps a stored position | A9, A10 | Open; DISL's, since what is drawn for an end that names nothing, and for an element with no entry, is the metamodel's to say and not a binding's |
| 9 | DISL | The order of findings across rules and readings; the first free name of a pattern | A7, A8, A11 | Open; DISL's, since the order of findings and a fresh name come from how a specification is evaluated, not from how a body is read |

### 1. FBL 9.2 and 5.1: several readings with different metamodels over one binding (B1)

- **Construct.** FBL 9.2 makes every reading of a body use one binding, and opens a tool type that names another read-only. One binding for the three readings then meets FBL 5.1, under which an entry becomes at most one element: `resources.pipelines.<key>` is a `Resource` to the bundle and the `Pipeline` to the pipeline reading, and `resources.jobs.<key>` is a `Resource` to the bundle and the diagram itself to the job reading. A `typeMap` (DISL 11.2) renames a binding's types per definition, but can hide another reading's type only `as: "header"`, which it is not.
- **Shown by.** `examples/lakehouse/databricks.yml`, opened as the bundle by `databricks.adp` and as the pipeline by `databricks.pipeline.adp`, both editable in the transcript; `two-jobs-two-pipelines.yml` under all three readings.
- **Answered.** FBL 0.10: a rule's `origins` lists the readings it is read for, and an entry becomes at most one element per reading (5.1), so `resources.pipelines.<key>` is a `Resource` to the bundle and the `Pipeline` to the pipeline reading through two rules, with no `typeMap` value added; readings of different origins through one binding are all editable and share one open body and its history (9.1, 9.2). `databricks.fbl` is that binding, and `databricks-readings` reads one body as the bundle, the job and the pipeline, with an edit in each.
- **Meanwhile.** Nothing stands in: `DatabricksBody` reads the body through `databricks.fbl` as the three readings of one open body, and `DatabricksReadings` builds each reading's model from the binding's reading (task 11).

### 2. FBL 8.2 and 9.1: a resource capture per reading (B2)

- **Construct.** `registration.resource.capture` is one capture name for the whole binding. This file needs `job` for the job reading, `pipeline` for the pipeline reading and none for the bundle, which sees every resource; without a header the library selects the first value, so under one capture the bundle would draw one pipeline. FBL 9.1 keeps one model per open body, while two job registrations of one file select different jobs.
- **Shown by.** `two-jobs-two-pipelines.yml`, whose two job and two pipeline registrations each show and edit their own resource; the two `registration-resource` divergences of `databricks.pipeline.adp` (both copies of the example) in the standalone's `RealFiles/divergences.json`.
- **Answered.** FBL 0.10: `registration.resource.byOrigin` gives each origin its capture, or `null` to the bundle, which then sees every resource, and the default without a header applies per origin (8.2); one open body keeps one model per origin, of which each registration's selection is a view (9.1); a header naming a resource the body does not hold opens the reading empty with `fbl.missing-resource` (7.4). `databricks-resources` shows it on `two-jobs-two-pipelines.yml`, and `databricks-rename-shared` an edit of one reading read by another.
- **Meanwhile.** Nothing stands in: the binding selects each reading's resource by `registration.resource.byOrigin`, and `DatabricksSelection` names the resource of the target's registration (`ContextTarget.RegistrationPath`) for the canvas, the menus, the rows, the drops and the commands alike (finding B3, repaired; task 11).

### 3. FBL 4.1.5 and 5.2: a YAML scalar as written (B4)

- **Construct.** FBL 0.2 typed a plain scalar by the YAML core schema, and a value that did not convert made its entry unreadable: a task key `01`, a bundle named `2024` or `null`, `num_workers: ${var.workers}` and `serverless: ${var.serverless}`, which would have cost the cluster or the whole pipeline in a file that belongs to another tool.
- **Shown by.** `typed-scalars.yml`, `variables-in-typed-places.yml`.
- **Answered.** FBL 0.5, by etalii.adp's feature `015-fbl-scalar-as-written`, gives every scalar its written text (4.1.5) and a slot `read: "text"`, and keeps an entry whose value does not convert readable with a `fallback` (5.2); the standalone library reads all three. What is left is the binding's: each attribute read as text or given a fallback, when the binding is written. It no longer stands in the way of the second stage.
- **Meanwhile.** Nothing stands in: the binding reads the module's slots as the text they were written with (task 11).

### 4. FBL 4.3 and 6.2: writing into an empty or one-line flow collection (B12)

- **Construct.** A YAML flow collection is one value whose only writable span is the whole collection (4.3), and no splice opens one into block form or adds an item inside it, so neither a task added to `tasks: []` nor a dependency added to `depends_on: [{task_key: a}]` can be planned. A JSON array written on one line is answered since FBL 0.3 (6.2).
- **Shown by.** `flow-collections.yml`, `flow-dependencies.yml`, `settings-one-line.json`.
- **Answered.** FBL 0.10, 4.3 and 6.2 (*Flow sequences*): an insert into an empty flow sequence opens it into a block sequence and its undo restores `[]`; an insert into a one-line flow sequence writes `, ` and the item before its `]`, and a removal takes the item and one separator; a flow sequence over several lines, like a flow mapping, takes no insert. `databricks-flow-empty` (`flow-collections.yml`) and `databricks-flow-one-line` (`flow-dependencies.yml`) show it, and `dependency-graph-empty-flow-sequence` keeps its splices.
- **Meanwhile.** Nothing stands in for the edit: the commands write into a flow sequence as FBL 0.10 says (task 12). The sentence ruled on 2026-10-19, "This list is written on one line, which this diagram cannot change yet; edit it in the text editor.", is kept for what FBL still refuses inside a one-line list, a new entry with a skeleton, by `DatabricksEdits.Sentence`.

### 5. FBL 5.2 and 6.2: a refusal's sentence (B19)

- **Construct.** An attribute's `empty: "refuse"` and `absent: { "json": "refuse" }`, an insert's `when` and a missing container refuse with a sentence the library builds from type names; only `readOnly` takes a text of its own (3.4). The tool's sentences are its own: "The job has no tasks section to add into.", and `The settings file has no "schema" to rewrite.`, which a planner's sentence matches only because the binding happens to be called `settings`.
- **Shown by.** `bundle.yml` (the transcript's task added to a file without `tasks:`), `legacy-target.json` (the transcript's schema rewrite).
- **Answered.** FBL 0.10: an attribute binding's `refusals` (`empty`, `absent`) and an insert's (`when`, `container`) give the sentence a host shows, as `readOnly` gives one (5.2, 6.2, 6.4), and a fixture's `refused` is then that sentence (15.3). `databricks.fbl` gives the module's own sentences, and `databricks-refusals` shows "The job has no tasks section to add into." and the sentence of an empty refusal.
- **Meanwhile.** The commands refuse in their own words before the plan, as the definitions' checks do, and the binding's `refusals` give the rest (task 12). `DatabricksEdits.Sentence` stands in for the sentences the binding cannot give: one that names the value it refuses ("There is no 'shiny' task type to add."), and gap 4's for a one-line list.

### 6. FBL 6.2 and 6.3: which rule plans an add, two containers in one insert, a nested mapping for a YAML item (B17)

- **Construct.** A dropped task's skeleton depends on its type (three types are reachable from the palette, seven are known to the writer); a resource added to a file without `resources:` creates two containers at once; and a YAML library, `- notebook:` with `path:` beneath it, is a nested mapping whose `kind` and `path` the example binding binds by expression, so the one-place rule (3.3) leaves it unwritable, while its `emit` covers JSON only.
- **Shown by.** `every-task-type.yml`, `no-resources.yml`, and the libraries of `examples/lakehouse/databricks.yml`.
- **Answered in part.** An add is planned by the first rule of its type whose insert's `when` holds (6.2), so a rule per task type carries that type's skeleton; and every missing level of a container is created by one insert (6.2, *Missing levels*, FBL 0.4). *(FBL 0.14)* A resource is added as a member of `resources.jobs` or `resources.pipelines`, under the key the add gives, after the last of its kind, with its missing levels (6.2, *A new member, named by its key*); `databricks-resource-add` shows it, from the module's `bundle.yml`.
- **Answered, the rest.** FBL 0.10, 6.2 (`keys`): an add writes an attribute bound with `child` under the levels of `child`, as a set creates them, so a library added to a YAML pipeline is `- notebook:` with `path:` below it and reads back as the library added; `databricks-library-add` shows it.
- **Meanwhile.** Nothing stands in for a library or a resource: both are planned through the binding (task 12), a resource as FBL 0.14's member insert of the rules `newJobResource` and `newPipelineResource`, which write its name as the text its key is. A new task's starter values by its type, a wheel's entry point, a SQL task's warehouse and a condition's operator and right side, are the job commands' `StarterValues`, beside the binding's rule per task type.

### 7. FBL 5.2 and 13: a slot read when another is absent, and a lowercased key placeholder (B18, B20)

- **Construct.** A pipeline's schema is read from `schema`, else from the legacy `target`, and written to `schema`; `override` is the reverse, a second slot that wins on read when it is present. A new document's key is the file's base name lowercased; FBL 13's `{key}` replaces what is not an ASCII letter, digit or `_` and prefixes `_` before a leading digit, and does not lowercase.
- **Shown by.** `legacy-target.yml`, `legacy-target.json`; the transcript's `newDocuments` section.
- **Answered.** FBL 0.10: an attribute binding's `secondary`, a slot read only when the attribute's own is absent and removed when the attribute is written (5.2), so `schema` reads `target` and a set writes `schema` and removes `target`, shown by `databricks-secondary`; and the placeholder `{key:lower}` (13), shown by `databricks-template-key`, where `My_Job.yml` gives the key `my_job`.
- **Meanwhile.** The binding reads and writes the schema (tasks 11 and 12); `ServiceCollectionAddDatabricksExtension.EmptyDocument` still writes the three new documents, whose bytes the binding's `template.byOrigin` equals (B20, ruled to keep the factory's bytes).

### 8. DISL 4.11 and 6.10: a placeholder node, and an element without an entry (A9, A10)

- **Construct.** A9: a `depends_on` naming no task draws one movable node per missing key, with a stored position, where DISL keeps a relation without a source and draws nothing, and a stub (6.10) is a line for a relation without a *target*. A10: the bundle reading always draws `bundle`, and the pipeline reading `target` and `compute`, each under a fixed id with a stored position, where DISL does not say whether a derived element may exist while its source entry does not. Both are written out on the companion pages: A9 on the [job's](databricks-job.md), A10 on the [bundle's](databricks-bundle.md) and the [pipeline's](databricks-pipeline.md).
- **Shown by.** `missing-upstream.yml` (A9); `no-bundle-key.yml`, `empty.yml`, `broken.yml` (A10).
- **Would resolve it.** A relation end that names nothing drawn as a placeholder node of a declared type, built from the written key, with view data under an id made from it; and a derived element allowed with no source entry, keeping the view data of its fixed id.
- **Meanwhile.** `DatabricksElementMapper` draws the missing task's stub; `DatabricksReadings.FixedBundle` and `FixedPipeline` emit the elements the file has no entry for.

### 9. DISL 8.6 and 11.5: the order of findings, and the first free name (A7, A8, A11)

- **Construct.** A7 and A8: the tool reports the findings of the whole file, every job's, then the bundle's, then every pipeline's, whichever reading asked, and within a job task by task; a definition evaluates its own reading, and neither DISL 8.6 nor `constraints.order` interleaves several rules per element. A11: a dropped task is `<type>_<n>` with the first free `n` from 1, where CEL has no range to search and `persistence.ids.suffix` numbers only a second holder, from 2. All three are written out on the companion pages: A8 on each, A11 on the [job's](databricks-job.md).
- **Shown by.** `finding-order.yml`, on which none of the 120 orders `constraints.order` can be given yields the tool's order (`DatabricksFindings.Tests`, `ConstraintsOrderAlone_DoesNotGiveTheToolsOrder`); `two-jobs-two-pipelines.yml`, whose findings are the same under every reading (A7); `fresh-key.yml` (A11).
- **Would resolve it.** An order key per finding, such as its element's place in reading order before the rule's, and the findings of the readings of one body joined by the runtime (the C4 conversion's gap 10 is the same order key); for A11, a `suffix` that numbers the first id too, counting from a given number, or a CEL function for the first name of a pattern that no element has.
- **Meanwhile.** `DatabricksFindings` joins and orders; `DatabricksFreeKey` is the function `freeTaskKey` of the job definition's plugin `net.etalii.adp.databricks.taskKeys`.

### Found while implementing, and not a gap

- **A relation from the second holder of a key.** A model built from records whose natural ids repeat matches a relation's ends by key, so the dependencies written under the second task of a duplicated key landed on the first (`duplicate-key-on-a-circle.yml`, where it added two cycle findings). The first stage gives the second holder an ephemeral id and keeps its written key as a host attribute. What a binding should read there is for the binding step to settle against that fixture; it is not yet a construct either language lacks.
- **A library of a bare settings file is added through `databricks-pipeline.fbl`.** `databricks.fbl` reads a bare settings file's libraries as the draft does, but plans every library add by its rules for a bundle's pipeline: an add is planned by the first rule of its type whose insert's `when` holds (FBL 6.2), and that `when` sees the attributes of the add, not the body, so a rule for `/libraries/*` placed after them never plans one and one placed before them would plan the bundle's too. Its `settingsLibrary` rule therefore has no insert, where the draft gave it one; whether a bare settings file's library add needs a construct, or the pipeline reading of such a file keeps its own binding, is for the binding step to settle against `settings-one-line.json`.

## Stand-ins

Code that stands in for a gap while it stands, named so that nobody reads it as the design:

| Stand-in | For | Where |
| --- | --- | --- |
| `DatabricksReadings.FixedBundle`, `FixedPipeline` | Gap 8, finding A10 | The module |
| The missing task's stub in `DatabricksElementMapper` | Gap 8, finding A9 | The module |
| `DatabricksFindings` | Gap 9, findings A7 and A8 (Q3) | The module, joining what the three definitions evaluate |
| `DatabricksFreeKey` | Gap 9, finding A11 | The plugin `net.etalii.adp.databricks.taskKeys` of the job definition, supplied by the module |
| `DatabricksEdits.Sentence` | Gaps 4 and 5, finding B19: today's sentence for a refusal the library words itself, one that names the refused value and one for a one-line list | The module's commands |
| `StarterValues` of the job commands | Gap 6: a new task's starter values by its type | The module's commands |
| `useSimulatedRun.ts` | Runtime gap A6: no player for `simulate` in the standalone canvas library | The module's client |

## What stays code

Every class of the standalone module's backend project, [`src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks), as `git ls-files` lists them, beside the gap, the host concern or the stage that keeps it, as the conversion's first stage leaves it: the palettes, the menus, the rows, the removal confirmation and the findings are derived from these three definitions and the canvases are compiled from them, while the body is still read by the module's parsers and written by its commands. A class marked **waits** is kept for the second stage, which reads and writes the body through one binding once gaps 1 and 2 are answered, and names the task of the conversion that removes or changes it. The hand-written providers' logic and `DatabricksRuleSet` are no longer here: they are the oracle in the test project's `Parity/`.

| Class | Kept by |
| --- | --- |
| `DatabricksDefinition` | Host: the three definitions, each loaded once as a `DislDiagramDefinition`, and the palettes, menus, rows, removal confirmation and a dropped task's key derived from the one the target's reading names, mapped to the `x-databricks` wire ids; which reading of the file holds an element id. |
| `DatabricksBody`, `DatabricksReadings` | Host: the one binding, embedded, and the three readings of one open body (FBL 0.10); each reading's DISL model built from the binding's reading under its definition, a second task of one key read at its place under an id of its own. Gap 8: `FixedBundle` and `FixedPipeline`, the elements the file has no entry for (A10). |
| `DatabricksSyntax` | Q4, findings B6 and B7: whether YamlDotNet opens the file at all, its message the sentence of `databricks.unparseable`; nothing is read from the tree it builds. |
| `DatabricksFindings` | Gap 9 (A7, A8, Q3): the file's findings joined from the three definitions in the tool's order. |
| `DatabricksFreeKey` | Gap 9 (A11): `freeTaskKey`, the first free `<type>_<n>`. |
| `DatabricksValidator` | Host: the file's findings as the host's problems, one validator per origin, as a call to `DatabricksFindings`; a body that does not parse is one problem at the parser's line (B7). |
| `DatabricksToolboxProvider`, `DatabricksContextActionProvider`, `DatabricksContextPropertyProvider` | Host: handing out the derived palette, menus and rows, and building the module's command for a chosen entry, an edited row or a drop; the three simulated entries answered as nothing by their `.simulated.` marker. They hold no palette, menu or row of their own. |
| `DatabricksSelection` | Host: what an element id names in the file's readings, and the resource of the target's registration, which the binding selects (B3). |
| `DatabricksContextSourceResolver` | Host: resolving a selection to an element of a file of its type. |
| `DatabricksNewPlacement`, `DatabricksRelationGesture` | Host: the stateless gesture ids `new:{x},{y}` and `rel:{from}->{to}`. |
| `DatabricksElementMapper` | Host: the `.proto` payloads and the viewport filter (A15). Gap 8: the drawn stub of a missing task (A9). |
| `DatabricksJobLayout`, `DatabricksBundleLayout`, `DatabricksPipelineLayout` | Host: the three layouts, which the definitions name as plugins (A15). |
| `DatabricksSession`, `DatabricksDocumentStore`, `IDatabricksDocumentStore` | Host: the shared document lifecycle and its reload, one session per open reading over one document and one history, and a move as one undo step through core's `SetRegistrationLayoutCommand`. **Waits**: a move written through FBL's registration writer (task 13, Requirement 5.5). |
| `ServiceCollectionAddDatabricksExtension`, `Diagram` | Host: the three registrations and their services, and the new document of each type (`EmptyDocument`), whose bytes the binding's `template.byOrigin` equals (B20). |
| `DatabricksModel`, `BundleModel`, `BundleResource`, `BundleTarget`, `JobModel`, `JobTask`, `JobCluster`, `JobDependency`, `PipelineModel`, `PipelineLibrary`, `PipelineNotification`, `UnknownNode` | The three readings as the layouts, the mapper and the commands read them, taken from the binding's (`DatabricksReadings`). |
| `DatabricksEdits` | Host: every edit one model change in the reading it edits, planned into splices by `EtAlii.Adp.Specification.Fbl` through the binding and stored as the lines it changes, each with the ending the library planned (task 12, which settles B8, B11, B12, B13, B15 and B16; the line-splicing writers have left the product). Gaps 4 and 5: `Sentence` (B19). |
| `InsertDatabricksTaskCommand`, `RemoveDatabricksTaskCommand`, `ConnectDatabricksTasksCommand`, `DisconnectDatabricksTasksCommand`, `RenameDatabricksTaskCommand`, `SetDatabricksRunIfCommand`, `SetDatabricksClusterCommand`, `AddDatabricksResourceCommand`, `SetDatabricksBundleNameCommand`, `InsertDatabricksLibraryCommand`, `RemoveDatabricksLibraryCommand`, `SetDatabricksPipelineScalarCommand` and their handlers, `DatabricksEditHandler`, `DatabricksStepsCommand` | Host: the command records the history stack and the wire carry, each written through the binding by `DatabricksEdits`, and the undo that restores the whole body; a renamed task's stored position moved in the same step (B16). Gap 6: the job commands' `StarterValues`. |

Beside the backend, `api/databricks.proto` holds the payloads. On the client, `BundleCanvas.tsx`, `JobCanvas.tsx` and `PipelineCanvas.tsx` compile their canvases from the three definitions, `DatabricksCanvas.tsx` builds the payloads they read, and `databricksBindings.ts` states what the canvas library cannot read from a definition, each entry with its reason. `useSimulatedRun.ts` plays the simulated run, deploy and update, standing in for runtime gap A6, and **waits** for a player of `simulate` in the canvas library (task 14).

## Sources

- The standalone module: [`src/diagrams/databricks/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/databricks), its parity transcript, fixtures and oracle in [`backend/EtAlii.Adp.Diagram.Databricks.Tests/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks.Tests).
- The conversion's requirements, design and tasks: [`specs/069-standalone-databricks-disl-fbl/`](../../specs/069-standalone-databricks-disl-fbl/), whose findings (A1 to A15, B1 to B22), gaps (section C) and questions (Q1 to Q6) this page cites by number.
- [DISL](../../specifications/disl/DISL-specification.md), sections 4.11, 6.10, 8.6, 11.2 and 11.5; [FBL](../../specifications/fbl/FBL-specification.md), sections 3.3, 3.4, 4.1.5, 4.3, 5.1, 5.2, 6.2, 8.2, 9.1, 9.2 and 13.
