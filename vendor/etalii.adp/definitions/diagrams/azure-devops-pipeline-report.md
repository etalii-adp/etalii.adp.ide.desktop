# Azure DevOps pipeline: the conversion's delivery report

The Azure DevOps pipeline diagram of etalii.adp.ide.standalone (origin `azure-devops/pipeline`) now derives its toolbox, menus, rows, findings and refusals from [azure-devops-pipeline.dis](azure-devops-pipeline.dis), reads its body through the module's FBL binding with a persistence plugin as the reader, and writes every edit as splices, with an undo that gives the bytes back. This is the report its specification, `azure-devops-pipeline-disl-fbl` in that repository's `.spec-workflow/specs/`, asks for at its last task: every language gap as a candidate change for this repository, every defect repaired on the way, every finding struck, and what still waits. What the module keeps, class by class, is in the companion, [azure-devops-pipeline.md](azure-devops-pipeline.md).

The measure is the draft declared binding [`../../specifications/fbl/azure-devops-pipeline.fbl.draft`](../../specifications/fbl/azure-devops-pipeline.fbl.draft): the binding this tool would have if FBL could state it. Each of its `x-gap` notes is one of the candidate changes below.

## Candidate changes for this repository

Each is a gap of the companion, numbered as there, with the key of the draft binding that would close it. The evidence, the fixture and the class of the module that keeps the behaviour meanwhile are in the companion.

| # | Language | Candidate change | In the draft |
| --- | --- | --- | --- |
| 1 | FBL | A declared binding with a plugin that expands named rules (the "part of its reading" of FBL 11.5 and 17); and for every plugin the body's path, the workspace's edge, the files read as inputs of the result (FBL 11.4's determinism), and the watching of them | `expand` |
| 2 | FBL | None: closed by FBL 0.5's written text (4.1.5, 5.2) | `"as": "text"`, which `read: "text"` and a text attribute's default now say |
| 3 | FBL | Further elements an entry stands for, each with its condition, as the parent of the entries beneath it | `also` |
| 4 | FBL | A selector that passes through keys matching a pattern, such as `${{ if }}`, at any depth, and binds what it passed | `through` |
| 5 | FBL | A list attribute read from one scalar or a sequence of scalars, with its written forms: a scalar for one, a block list for several, `[]` for none | `"list": "scalar-or-sequence"` |
| 6 | FBL | An unset model change, and a slot's `unset` distinct from its `empty` | `unset` |
| 7 | FBL | A reference compared without case and resolved in the scope of the parent | `reference.compare`, `reference.scope` |
| 8 | FBL | An entry's keys in written order, available to a slot or a `when` | (the step rule binds one kind key) |
| 9 | DISL | A stub for a relation whose source is absent, drawn into its target and labelled with the name the end was written with (6.10 draws only an absent target) | (DISL) |
| 10 | DISL | A definition of the `layered` algorithm (D.2, open question 9) | (DISL) |
| 11 | DISL | An order of findings by an expression, such as the parent of the element a rule judged (8.6) | (DISL) |
| 12 | FBL and DISL | A statement of which id a model change names when the specification derives ids (11.5) and the plugin reads its own (11.2), and that a runtime keeps the pair | (both) |

Gaps 11 and 12 were found by the implementation, not by the reading the requirements were written from. The design expected one more that did not occur: the library's cycles come in the same order as the tool's, over the corpus and over a cycle written out of order, so the module's cycle finder is not kept.

## Definition defects repaired

Seven changes to `azure-devops-pipeline.dis` in this repository, each found by holding the tool's derived answers to its hand-written ones: pull requests 281 (the rewrite at DISL 0.3, findings A1 to A4 and A7), 285 (five in the menus and rows), 288 (three in the findings), 292 (the id of an unfollowed `extends`), 304 (four edit rules), 307 (the notation), and the order of the change rules with this report. The companion lists what each corrected.

## Module defects repaired

In etalii.adp.ide.standalone, each with a test seen to fail first:

- **B8**: Rename and Disable wrote the new key inside a block scalar (standalone pull request 405).
- **A9**: a template that names a template gave two references the id `template-0` (405).
- **A8**: drawing and validating a pipeline with two stages of one name threw (410).
- **Reading threw on content**: YamlDotNet refused some text with exceptions other than `YamlException`, and an `extends` that could not be followed threw (424).
- **B18**: undo did not give the bytes back in five ways, and twelve of the transcript's documents were left different after undoing every edit; now none is (438).
- **Removing the only stage that starts the pipeline**, when every stage left would wait for another, was meant to be refused and was not (441).
- **Toolbox drops** did nothing on the pipeline canvas (445).
- **A closed stage or job never showed as closed**, a script step dropped onto a step was refused, an unfollowed template's tooltip named its reference instead of the reason, and an open job's name was drawn behind its steps (450).

## Library work, in the standalone repository

Each was added for every tool, with no module name: `reachable` and `inCycle` (DISL 12.4) and the `view` action reaching the host (pull request 412); the expression form of a derived relation in the model builder (418); deletion and reorder constraints in the gesture evaluator (438); packed badges, drop targets and six other notation features in the client's notation compiler (445).

## Findings struck or amended

Ruled in chat as small amendments to the conversion's requirements, all on 2026-10-17:

- **B9 struck**: a removal's undo already restored the blank lines on `develop`, guarded by `blank-lines-after.yml`.
- **B4 narrowed**: a step under `${{ if }}` is not unwrapped, which `findings/gates-nested.yml` shows.
- **B17 amended**: the new document ends with a final newline.
- **A8 extended**: the throw of a pipeline with two stages of one name, repaired before the conversion went on.
- **Requirement 3.3 amended**: job findings are grouped by kind, not by stage (gap 11).
- **B18 added**: undo did not give the bytes back, measured by the transcript, and repaired by the conversion.

## Behaviour that changed on purpose

Each named in the requirements and ruled by the owner: an edit writes the value's own span in its own style, a new key in the binding's order, a new stage after the last stage of the list rather than inside a `${{ if }}` block, and a removal that takes its leading comments and no blank line (Q1); a key that arrives through a merge is refused (B13); undo gives the bytes back and is refused when the file changed since, with "The file has changed since this edit, so it cannot be undone." (Requirement 5.9); job findings come grouped by kind (gap 11).

## What still waits, and on what

| What | Waits on |
| --- | --- |
| A declared reader for the file, with the plugin for templates only | Gaps 1 and 3 to 8 |
| `move` for a declared `yaml` binding in the standalone library (the conversion's Requirement 8.2) | A declared reader |
| An entry for this tool in the standalone real-file corpus | A declared reader |
| Findings B14 (an item that is not a mapping, a key written twice) and B15 (the sentence of a file that does not parse), and `std.duplicateId` (A8) | Each a small change of its own (Q3) |
| `PipelineLayout` leaving the module | Gap 10 |
| The broken edge leaving the module | Gap 9 |
| `PipelineBody.ReadIdOf` leaving the module | Gap 12 |
