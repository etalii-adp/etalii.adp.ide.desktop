# Azure DevOps pipeline

The DISL specification of this diagram type is [azure-devops-pipeline.dis](azure-devops-pipeline.dis), written at DISL 0.3. This page holds what DISL and FBL cannot express about the tool, each gap with the construct it concerns, the document that shows it, what would resolve it and the class of the standalone module that keeps the tool's behaviour until then. Everything either language can say is said in the `.dis` and is not repeated here. The gaps were found against DISL 0.3 and FBL 0.2 and are stated as they stand at DISL 0.4 and FBL 0.7.

The tool is the Azure DevOps pipeline diagram of the standalone IDE (origin `azure-devops/pipeline`). Every sentence, label, code and id in the `.dis` is the standalone tool's, as its parity transcript records them: `Parity/azure-pipeline.transcript.json` under [`src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline.Tests/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline.Tests) in etalii.adp.ide.standalone. The documents named below are that test project's fixtures; `findings/` is its `Fixtures/findings/` folder. The conversion is specified in that repository's `.spec-workflow/specs/azure-devops-pipeline-disl-fbl/`, whose findings this page cites by number; its delivery report, with every gap as a candidate change for this repository, is [azure-devops-pipeline-report.md](azure-devops-pipeline-report.md).

## What the diagram is for

A CI/CD pipeline's stages, jobs and steps, and which of them wait for which. Pipeline YAML nests stages, jobs and steps inside templates; opening one level at a time keeps a long pipeline readable. The diagram reads a pipeline, it does not run one, and the file stays Azure DevOps' own: only `displayName`, `dependsOn` and `enabled` are ever written, besides whole stages, jobs and steps added, removed and moved.

## What the reader reads

The reader is the standalone module's, through the FBL binding it ships; while gap 1 stands, the binding's reader is a persistence plugin. What it reads, for every document of the corpus:

- **Stages, jobs and steps**, a deployment job's steps gathered per lifecycle hook: `preDeploy`, `deploy`, `routeTraffic` and `postRouteTraffic`. **The steps of `on.failure` and `on.success` are not read**: a hook is read only where its own mapping holds `steps`, and those two hold `failure` and `success` (finding A7, `findings/deployment-on-hooks.yml`).
- **The implicit stage and job** of a pipeline without `stages`, and of one with only `steps` (gap 3).
- **Every value as the text it was written with**, expressions included, none ever evaluated (gap 2).
- **`${{ if }}` and `${{ each }}` around a stage or a job**, at any depth, with the expressions kept as the element's gate, joined with " and " (gap 4). **A step under `${{ if }}` is not unwrapped**: it is read as one step of unknown kind (finding B4, `findings/gates-nested.yml`).
- **Merge keys** expanded, the mapping's own keys winning.
- **Templates** followed inside the workspace only, never over the network, with one of six reasons for one that is not; a template's parameters by name and never by value (gap 1).
- **`dependsOn`** in each of its forms and its absence (gaps 5 and 6), each name resolved without regard to case, in its scope (gap 7).
- **Ids** are paths, never written, as `persistence.ids` states. Two stages of one name share an id and the first is the one a gesture finds (finding A8, gap 12); `std.duplicateId` is switched off until reporting it is ruled on its own.

## The gaps

Twelve, each a candidate change for this repository: the ten of the conversion's requirements, section C, and two the implementation found (11 and 12). One is closed since (2). None is worked around in the `.dis` by stating something the tool does not do.

| # | Gap | Language | State at DISL 0.4 and FBL 0.7 | Kept in the standalone module by |
| --- | --- | --- | --- | --- |
| 1 | A reader declared for the file with a plugin for part of it; a plugin told where its body and workspace are | FBL | Open | `PipelineYamlPlugin` as the whole reader, with `PipelineReadContext` |
| 2 | A slot that reads a YAML scalar as written | FBL | **Closed by FBL 0.5** (written text, 4.1.5) | Nothing beyond gap 1 |
| 3 | One entry, several elements, and a parent that is not an entry | FBL | Open | `PipelineParser` (the plugin's `read`) |
| 4 | Selecting through a key that contains braces, at any depth | FBL | Open | `PipelineParser` |
| 5 | A value that is one scalar or a sequence of scalars | FBL | Open | `PipelineParser`; `PipelinePlanner` for the written forms |
| 6 | Unsetting a slot as distinct from emptying it | FBL | Open | `PipelinePlanner` |
| 7 | A reference by a name compared without case and resolved in a scope | FBL | Open | `PipelineYamlPlugin.Read` |
| 8 | The order of an entry's keys in CEL | FBL | Open | `PipelineParser` |
| 9 | A relation whose source names nothing, drawn | DISL | Narrowed: a stub (6.10) draws an absent target, not an absent source | `PipelineGraphBuilder`, `PipelineElementMapper` |
| 10 | The standard `layered` algorithm | DISL | Open | `PipelineLayout` |
| 11 | Findings of several rules ordered by the element's container | DISL | Open; the tool's order changed instead (ruled 2026-10-17) | Nothing |
| 12 | The id a model change names when the specification derives ids and the plugin reads its own | FBL and DISL | Open | `PipelineBody.ReadIdOf` |

### 1. FBL: a reader that is declared for the file and a plugin for part of it (B2)

- **Construct.** `Binding.reader` is `"declared"` or one `PluginReader`. Nothing states a plugin for a part of the reading, though FBL 11.5 and 17 name this tool as "a declared binding, with a plugin for part of its reading". The whole-plugin form does not reach templates either: a plugin's `read` receives the body's bytes (FBL 11.2), and the standalone runtime's read request tells it neither where the body is nor where the workspace ends, so it cannot open `templates/build-jobs.yml`. `watch` is for folder subjects, and no host calls it for a file body.
- **Determinism, found by the implementation.** FBL 11.4 requires `read` to give the same result for the same bytes. Here the result also depends on the templates the body names, which the bytes do not hold. Within one reading each template is read once, through the resolver's cache.
- **Shown by.** `templates.yml`, `extends.yml`, `extends-other-repository.yml`, `template-in-template.yml` with `templates/nested-jobs.yml`.
- **Would resolve it.** A plugin that expands named rules of a declared binding (the draft's `expand`), and for any plugin: the body's path, the workspace's edge, the files it read as inputs of its result, and the watching of them.
- **Meanwhile.** The binding names the persistence plugin `net.etalii.adp.azure-devops.pipelineYaml` as its reader, and the module hands it `PipelineReadContext` (the body's path and the workspace's template resolver), the one named type beside the contract. A change to a template reaches the diagram through the module's store, as before the conversion. The draft declared binding ([`../../specifications/fbl/azure-devops-pipeline.fbl.draft`](../../specifications/fbl/azure-devops-pipeline.fbl.draft)) measures what is missing.

### 2. FBL: a slot that reads a YAML scalar as written (B1), closed by FBL 0.5

- **Construct.** Under FBL 0.2 a plain scalar was typed by the YAML 1.2 core schema, so a stage named `1.0`, a display name `yes` or a job named `null` would reach the model as a number, a boolean or nothing. The timeline's conversion reported it first; this tool adds that the scalar may be an expression.
- **Shown by.** `findings/scalars-as-text.yml`: the stage `1.0` with the job `null`, read and drawn under those names.
- **Resolved by.** FBL 0.5's written text (section 4.1.5), which a text attribute, an id and a reference read unasked and any slot asks for with `read: "text"` (section 5.2). The draft's `"as": "text"` is that construct. A declared reader of this tool would need nothing more for it; while gap 1 stands the plugin reads every value as written itself.

### 3. FBL: one entry, several elements, and a parent that is not an entry (B3)

- **Construct.** "An entry becomes at most one element or relation" (FBL 5.1), and `parent` needs the nearest enclosing entry a rule matched. A file with only `jobs` has a stage nobody wrote; one with only `steps` has that stage and a job: the root mapping is the diagram, a stage and a job at once.
- **Shown by.** `jobs-only.yml`, `steps-only.yml`.
- **Would resolve it.** Further elements an entry stands for, each with its condition (the draft's `also`).
- **Meanwhile.** `PipelineParser` emits `ImplicitStage` and `ImplicitJob`, which the `typeMap` maps to `Stage` and `Job`.

### 4. FBL: selecting through a key that contains braces, at any depth, with the keys on the way (B4)

- **Construct.** A key containing `{` or `}` cannot be selected (FBL 4.2); a `{gate}` capture with a `when` reaches one level; `**` reaches every level and binds nothing, so the joined gate cannot be read.
- **Shown by.** `findings/gates-nested.yml`, `edge-expressions.yml`.
- **Would resolve it.** A selector that passes through keys matching a pattern and binds what it passed (the draft's `through`).
- **Meanwhile.** `PipelineParser` unwraps a stage or job item that is one `${{` key over a list, at any depth, and joins the expressions into the element's gate.

### 5. FBL: a value that is one scalar or a sequence of scalars (B5)

- **Construct.** A relation rule at `dependsOn/*` finds list items only, and nothing in `dependsOn: Build`; no slot means "the entry's own value". Read as a list attribute, three things are unstated: that a scalar reads as a list of one, how a list is written other than `flow` (the tool writes one name as a scalar, several as a block list and none as `[]`), and removing the key as distinct from writing `[]`.
- **Shown by.** `findings/depends-on-forms.yml`.
- **Would resolve it.** A list read from a scalar or a sequence, with its written forms (the draft's `"list": "scalar-or-sequence"`).
- **Meanwhile.** `PipelineParser` reads both forms, and `PipelinePlanner` writes one name as a scalar, several as a block list (an existing block list keeping its indentation) and none as `[]`, its code commented as standing in for gaps 5 and 6.

### 6. FBL: unsetting a slot as distinct from emptying it (B5)

- **Construct.** "(nothing)" writes `dependsOn: []` and "(the default order)" removes the key; they are different pipelines. `empty` chooses one of them for a binding (FBL 0.6 makes it decide a new entry's key as well), and FBL 11.2 has no model change that unsets.
- **Shown by.** `findings/depends-on-forms.yml` (`Lint` waits for nothing, `Build` follows the default order).
- **Would resolve it.** An unset model change and a slot's `unset` (the draft's `"unset": "remove"`).
- **Meanwhile.** `PipelinePlanner` plans a set to null as `remove-key` and a set to an empty list as `[]`.

### 7. FBL: a reference by a name compared without case and resolved in a scope (B6)

- **Construct.** `dependsOn` is matched without regard to case, the first element of that name wins, and a job's names are those of its own stage. `reference: {to, by}` is exact and file-wide. FBL 0.7's `caseInsensitive` matches a rule's lines and `map` keys, not a reference.
- **Shown by.** `findings/depends-on-case.yml`.
- **Would resolve it.** `reference.compare` and `reference.scope` (the draft's `"compare": "ignore-case"` and `"scope": "parent"`).
- **Meanwhile.** `PipelineYamlPlugin.Read` emits one `DependsOn` per written name with its source resolved so.

### 8. FBL: the order of an entry's keys in CEL (B16)

- **Construct.** A step's kind is its first recognised key in written order; an entry reaches CEL as a map, which has no order.
- **Shown by.** `findings/step-two-kinds.yml`.
- **Would resolve it.** The entry's keys in written order, available to a slot or a `when`.
- **Meanwhile.** `PipelineParser` takes the kind from the first recognised key.

### 9. DISL: a relation whose source names nothing, drawn (A6)

- **Construct.** Under `typeMap`, a relation read with an end that names nothing is kept with that end unset, and "is not drawn" (DISL 11.2, 6.1). DISL's stub (6.10) draws a relation whose *target* is absent, from its source. A `Dependency` runs from what is waited for to what waits, so the end that names nothing here is the *source*. The tool draws it as a broken edge, `edge:?<name>-><to>`, in the danger colour with a `2 4` dash, and marks the stage that waits indeterminate.
- **Shown by.** `findings/dangling-dependency.yml` (`edge:?Pacakge->Deploy`), `edge-broken-graph.yml`.
- **Would resolve it.** A stub for a relation whose source is absent, drawn into its target and labelled with the name the end was written with.
- **Meanwhile.** `PipelineGraphBuilder` keeps the broken edge and `PipelineElementMapper` marks the waiting element indeterminate, both commented as standing in for this gap (Q4). The finding itself is the definition's.

### 10. DISL: the standard `layered` algorithm (A10)

- **Construct.** DISL defines `rowPacked`, `rows` and `tidyTree` (0.3) and the properties of a tiered force layout (0.4, 10.4), and still only names `layered` (D.2, open question 9). The `.dis` states the tool's rules as `adp.*` options.
- **Shown by.** Every document of the corpus; the payloads of the transcript hold every position.
- **The rules, in numbers.** A stage's column is the length of the longest chain of dependencies before it, ignoring dependencies that name nothing and the edge that closes a cycle. A column is as wide as its widest element, columns 80 apart, each filled top-down in written order 32 apart. A closed stage is 220 × 88, a closed job and a step 180 × 56. An open stage is its jobs' width + 2 × 24 wide and their height + 40 + 2 × 24 tall, never smaller than closed, its jobs offset by 24 and 40 + 24. An open job is 228 wide and 40 + 2 × 24 + n × 56 + (n − 1) × 32 tall. An unfollowed template goes below everything else, at the x of the element whose list names it. Positions are deterministic and never written.
- **Would resolve it.** A definition of `layered`.
- **Meanwhile.** `PipelineLayout`, with `PipelineMetrics` and the records it arranges into.

### 11. DISL: findings of several rules ordered by the element's container (found by the implementation)

- **Construct.** The tool judged each stage's jobs in turn: a dangling dependency and then a cycle for one stage, before the next stage's. The `.dis` declares one rule for each, and DISL orders declared findings by rule, and within one rule by element in model order (8.6); `constraints.order` orders by code. So every dangling job dependency comes before every job cycle, and nothing can order findings by the stage a job belongs to.
- **Shown by.** No document of the corpus has job findings of both kinds in two stages; the standalone test `PipelineDerivedFindingsTests.JobFindingsInTwoStages_AreTheRuleSets_GroupedByKindRatherThanByStage` holds the case with a document of its own.
- **Would resolve it.** An order of findings by an expression, such as the element's parent.
- **Meanwhile.** Nothing: the tool now reports job findings grouped by kind, as the definition does, by the owner's ruling of 2026-10-17 (the conversion's Requirement 3.3).

### 12. FBL and DISL: the id a model change names when the specification derives ids (found by the implementation)

- **Construct.** The definition derives its ids (a stage's name, `stage/job`, `job/step-n`, DISL 11.5), and two stages of one name derive one id (finding A8). A plugin reads each element under an id of its own (FBL 11.2), which must be unique, so the two are not the same id. Model changes carry an `id` (FBL 11.2), and neither language says whether it is the specification's derived id or the reader's.
- **Shown by.** `findings/two-stages-one-name.yml`: both stages are drawn as `Build`, and every edit of `Build` reaches the first.
- **Would resolve it.** A statement that a change to an element whose id is derived names it by the id its reader gave it, and that a runtime keeps the pair; or a derived id that is unique by construction for elements the specification lets a user edit.
- **Meanwhile.** `PipelineBody.ReadIdOf` pairs each derived id with the plugin's id by place in the reading, the first of a shared id winning, and `PipelineDefinition.Apply` writes every change under the plugin's id.

## The stand-ins the module holds

Each is named in its code for the gap it stands in for, and goes when its gap is closed:

- **`PipelineReadContext`** (gap 1): the body's path and the workspace's template resolver, handed to the plugin beside the contract.
- **`PipelineYamlPlugin`, with `PipelineParser` and `PipelinePlanner`** (gaps 1, 3 to 8): the reading and the planning a declared binding would do.
- **The `dependsOn` forms in `PipelinePlanner`** (gaps 5 and 6): a scalar, a block list, `[]`, or the key removed.
- **The broken edge in `PipelineGraphBuilder`, and the indeterminate mark in `PipelineElementMapper`** (gap 9).
- **`PipelineLayout`** (gap 10, finding A10).
- **`PipelineBody.ReadIdOf`** (gap 12).

The element mapper's payloads and the session's deltas are the host's, not a gap (finding A11).

## What remains in the module, and why

Every type of the product project `backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/` of `src/diagrams/azure-devops-pipeline/` in etalii.adp.ide.standalone, checked against `git ls-files` of that project at the end of the conversion (its task 12), beside the gap or the host concern that keeps it. `PipelineToolboxProvider` had neither, since core's `DislToolboxProvider` hands out a bundled definition's palette, and was removed by that task. `PipelineWriter`, `PipelineBlocks`, `PipelineEdits`, `PipelineLineIndent`, `PipelineRuleSet`, `PipelineRules`, `PipelineDisl` and the providers' hand-written logic left the product code earlier in the conversion; the ones a test still compares with live in the test project as the oracle.

| Type | Kept by |
| --- | --- |
| `PipelineYamlPlugin` | The persistence plugin the binding names as its reader (gaps 1 and 3 to 8): `read` projects what the parser read onto the binding types, `plan` hands a change to the planner. |
| `PipelineParser` | The plugin's reading: YamlDotNet, every value as written, templates followed in place, the implicit stage and job, gates, both forms of `dependsOn`, a step's kind by its first key (gaps 3, 4, 5, 8). |
| `PipelinePlanner` | The plugin's planning: one model change as splices of the spans it touches, the `dependsOn` forms (gaps 5 and 6). |
| `PipelineReadContext` | Gap 1. |
| `PipelineTemplates` | Template resolution inside the workspace only, with today's six reasons, read once per workspace: the host's security rule (FBL 16) and gap 1. |
| `PipelineModel`, `PipelineStage`, `PipelineJob`, `PipelineStep`, `PipelineStepKind`, `PipelineExecution`, `PipelinePool`, `PipelinePoolOrigin`, `PipelineStrategy`, `PipelineStrategyKind`, `PipelineTemplateReference`, `PipelineTemplateSlot`, `PipelineTemplateUnresolved`, `PipelineTemplateUnresolvedReason` (`_Model/`) | What the plugin reads into before it hands over its elements, and what the layout, the mapper and the selection address. |
| `PipelineGraphBuilder`, `PipelineGraph`, `PipelineGraphNode`, `PipelineEdge`, `PipelineEdgeCondition` | The graph the layout and the mapper read, with the default order of stages and the broken edge (gap 9). |
| `PipelineLayout`, `PipelineMetrics`, `PipelineArrangement`, `PipelinePlacement`, `PipelineSize` | Gap 10. |
| `PipelineElementMapper` | The host's wire: the model in the element vocabulary of `api/azure-pipeline.proto`, with this connection's open stages and jobs (finding A11), and the indeterminate mark of gap 9. |
| `PipelineSession`, `PipelineElementExpandedEventArgs` | The host's session seam: the deltas one connection is sent when a stage opens or the file changes (finding A11). |
| `PipelineViewState`, `PipelineConnectionView` | Viewer state, held per connection by the host: which stages and jobs are open, never written and never undone (DISL 11.6). |
| `PipelineContextSourceResolver`, `PipelineSelectableElement` | The host's selection: an element id resolved, nested, against the file and the workspace. |
| `PipelineDefinition` | The definition: the bundled `.dis` loaded once, and the menus, rows, findings, refusals and operations derived from it, mapped by `x-azure-pipeline`, with viewer state bound for each call. |
| `PipelineBody` | The body read and written through the module's binding (`azure-devops-pipeline.fbl`), the DISL model built from its reading, and gap 12. |
| `IPipelineDocumentStore`, `PipelineDocumentStore` | The shared document lifecycle: one open body per file, opened with its workspace root so that its templates can be followed (gap 1). |
| `PipelineContextActionProvider`, `PipelineContextPropertyProvider`, `PipelineValidator` | Core's seams, handing out what the definition derives. |
| `AddPipelineElementCommand`, `MovePipelineStepCommand`, `RemovePipelineElementCommand`, `RenamePipelineElementCommand`, `SetPipelineDependenciesCommand`, `SetPipelineElementEnabledCommand`, `IPipelineEditCommand`, `PipelineAddKind`, with their handlers and `PipelineEditHandler` | The host's command records, unchanged on the wire; each handler runs the definition's operation and writes its changes through the body. The handlers' base opens the copy through the store, because the workspace root comes with each command. |
| `RestorePipelineSpanCommand` | Undo on the host's history: the library's inverse splices with the digest of the bytes they expect, refused when the file changed since (FBL 7.2), so that undo survives a reload. |
| `Diagram` | The host's catalog entry: the origin and the `.yml` extension. |
| `ServiceCollectionAddAzurePipelineExtension` | The host's registration of the types above, and of core's `DislToolboxProvider` over the definition. |

Beside them the client keeps `client/pipelineBindings.ts`, which says what the canvas library cannot read from the definition: the values the definition computes with CEL, read from the payload the backend sends, the class names that tie a kind to its theme colours, the curve of the "waits for" arrow, the problem mark, the test ids, and the rename F2 starts, which the backend finds by its key. `client/handWrittenDefinition.ts` is test code only, the oracle the compiled canvas definition is held to.

## Definition defects repaired

Each was found by holding the standalone tool's derived answers to its hand-written ones, and corrected here; none was patched in the module:

- **The rewrite at DISL 0.3** (pull request 281): the definition was DISL 0.1 (A1), worked around twelve things DISL and FBL had since answered (A2), named a plugin with three keys beside `format` that DISL 11.2 forbids (A3) and gave no wire ids (A4); its companion said the steps of `on.failure` and `on.success` were read (A7).
- **The menus and rows** (pull request 285): a stage's Depends on showed resolved stages instead of the names as written, the Trigger value carried its quotes, a stage's own pool lost its reason, Kind and Strategy showed enumeration keys, and three add entries were greyed out instead of refusing.
- **The findings** (pull request 288): a dangling dependency and a cycle named the element by its label instead of its name, an unnamed unreachable stage read `''`, and a stage whose dependencies all name nothing did not count as able to start.
- **The id of an unfollowed `extends`** (pull request 292): `template:extends`, as the tool has always given it.
- **Four edit rules** (pull request 304): `noCycleByEdit` walked the dependencies the wrong way, `keepAStart` refused clearing `dependsOn`, `keepTheOnlyStart` judged the stages before the removal, and three sentences had no fallback for an element without a label.
- **The notation** (pull request 307): label alignment, the editable name lines, box corners, badge offsets, the filled arrow, the drop targets, a template's size and its tooltip, which the standalone client had corrected before compiling.
- **The order of the change rules** (with this page's last revision): `readOnlyElements` was declared last, so the module put its sentence in front by hand; it is now declared second, which is the order the tool refuses in.

## Outside the definition, and not a gap

- **The client's payloads**, `PipelineElementPayload` in `api/azure-pipeline.proto`, and the deltas a session sends when a stage opens or the file changes on disk are the host's (finding A11).
- **Undo** is the host's history, which carries the library's inverse splices and refuses when the file changed since the edit; a change made to the file outside the editor reaches every open view and is not put on it.
- **Security**: a template outside the workspace is never read, a repository resource never fetched, and a parameter's value never held in the model. These are today's rules and FBL 16's.

## Sources

- Code: [`src/diagrams/azure-devops-pipeline/`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/develop/src/diagrams/azure-devops-pipeline) in etalii.adp.ide.standalone, and its parity transcript.
- The conversion's specification, `azure-devops-pipeline-disl-fbl`, in that repository's `.spec-workflow/specs/`: its findings A6 to A11 and B1 to B18, and section C, the first ten gaps.
- DISL 0.4 (`specifications/disl/`), FBL 0.7 (`specifications/fbl/`) and the draft declared binding `specifications/fbl/azure-devops-pipeline.fbl.draft` in this repository.
- Microsoft: [YAML schema reference](https://learn.microsoft.com/en-us/azure/devops/pipelines/yaml-schema/), [key concepts](https://learn.microsoft.com/en-us/azure/devops/pipelines/get-started/key-pipelines-concepts), [stages, dependencies and conditions](https://learn.microsoft.com/en-us/azure/devops/pipelines/process/stages).
