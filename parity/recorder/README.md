# Parity recorder

The recorder of etalii.adp spec 030 ([contracts/parity.md](https://github.com/etalii-adp/etalii.adp/blob/develop/specs/030-desktop-native-rust/contracts/parity.md), task T049). It replays each edit script under `parity/scripts/` through the standalone host's gRPC API, at the reference commit `8b6ccfc`, and writes what the standalone host wrote as the goldens under `parity/goldens/`. It is the only .NET code in this repository: it is not a member of the Cargo workspace, the Build workflow does not build or run it, and nothing of it ships. It runs only when goldens are recorded again.

## Running it

It needs the .NET 10 SDK and the standalone host checked out at the reference commit as a git worktree beside this repository, at `../standalone-8b6ccfc` (another folder is passed with `-p:StandaloneRoot=<folder>`):

```bash
git -C ../etalii.adp.ide.standalone worktree add --detach ../standalone-8b6ccfc 8b6ccfc
```

Then, from the repository's root, one command records every script:

```bash
dotnet run --project parity/recorder
```

`dotnet run --project parity/recorder -- parity/scripts/generic/timeline` records the scripts under one folder, and a path to a `.json` records one script. Options:

| Option | Effect |
|---|---|
| `--elements` | Prints every element of the document as the diagram stream sends it when it opens: ids, types, positions. For writing a script. |
| `--verbose` | Lets the standalone host's own log through. |
| `--any-commit` | Records from a worktree that is not at `8b6ccfc`. |
| `--allow-dirty` | Records from a worktree with local changes. |

Without the last two the recorder refuses, because a golden is only evidence of the standalone host's behaviour when it was recorded from the reference commit's code.

## How it runs the standalone host

The recorder compiles against the standalone host's own sources in the worktree (a project reference to `src/backend/EtAlii.Adp.TestSupport.Host`, so `dotnet run` builds the standalone host with `dotnet build`, in Debug) and runs it in process, through `HostFixture`, the host the standalone host's own flow tests drive. That is the standalone host's `Program`, every module, every gRPC service, in the `developer` environment, served over an in-memory connection rather than a port, with two registrations replaced: its project list and its problem cache live in a temporary folder.

It does not start `EtAlii.Adp.Backend.Service` as a process because that process keeps both in `Environment.SpecialFolder.ApplicationData` and has no setting to move them: replaying a script would add its temporary project to the person's own project list and problem cache. Nothing else is needed headless: no browser, no certificate, no port.

Each script gets a host of its own. Its fixture folder is copied into a new project folder in the temporary folder, the recorder signs in with `AuthenticationService.DeveloperSession`, the developer session a Debug build in the developer environment issues for `LocalAuthenticator:Username` without a credential, adds the folder with `ProjectService.AddProject`, opens the context stream (`ContextService.Watch`) and the document (`DiagramService.Open`, then `DiagramService.UpdateView` over everything) on one connection, as a browser tab does, selects the document and replays the steps. The temporary folder is deleted when the host is.

## Edit scripts

A script is `parity/scripts/<tool>/<name>.json`, its goldens are `parity/goldens/<tool>/<name>/`, and `fixture` and `registration` are paths under `parity/fixtures/`. The folder holding them is copied whole as the project, and the registration (else the fixture) is the document opened. Steps name actions and properties by the standalone host's own ids (`mindmap.add-child`, `timeline.begin`, `c4.rename`, `history.undo`).

| Step | What is sent |
|---|---|
| `{ "select": [] }` | `ContextService.Select`: the document's explorer entry, nothing inside it. |
| `{ "select": ["<element>"] }` | `ContextService.Select`: the document's entry with the element as its child, from the diagram canvas. One element per step: a selection carries one. |
| `{ "action": "<id>", "args": { ... } }` | `ContextService.ExecuteAction` with `action_id`. Its source is `args.target` (an element of the document, sent with the document's entry as `diagram_entry_id`), `args.entry` (an explorer entry by its name, a folder or a file, as its context menu sends it: `hierarchy.add`) or, without either, the connection's selection; a `history.*` action without a target is sent against the project. A prompt it pushes on the context stream is answered with `ContextService.SubmitInteraction`: `args.value` (and `args.text`, a dialog's text field) for one answer, `args.answers` (a list of values or `{ "value", "text" }`) for a prompt that leads to another (`next_prompt`), `args.confirm: true` for a confirmation (an empty value). A prompt left unanswered is dismissed with `ContextService.CancelInteraction`, as Escape dismisses it: an add that then asks for a name keeps the name it was given. |
| `{ "property": { "row", "value", "target"? } }` | `ContextService.SetProperty`. `row` is a property id (`mindmap.notes`) or a row's label as the grid shows it (`Notes`), which `ContextService.DescribeProperties` turns into its id. Its source is `target`, else the selection. |
| `{ "move": { "refs": [...], "parent", "index"? } }` | `DiagramService.MoveElement` with `new_parent_id` and `index` (-1 appends): a re-parent, as the mind map moves. |
| `{ "move": { "refs": [...], "to": { "x", "y" } } }` | `DiagramService.MoveElement` with `position`, in the module's own coordinates (the timeline's are seconds since 1970 and 60 per row). |
| `{ "move": { "refs": [...], "dx", "dy" } }` | The same, at the element's position on the diagram stream plus `dx` and `dy`. |
| `{ "undo": n }`, `{ "redo": n }` | `ContextService.ExecuteAction` with `history.undo` or `history.redo` against the project, n times. |
| `{ "save": true }` | Nothing: every command is written by the standalone host's history as it runs, so a diagram has nothing left to save. |
| `{ "table": { "kind", "row"?, "column"?, "view"?, "values"?, "index"?, "target"?, "settings"? } }` | `DesignerService.Edit` of a table (see "Tables" below): one `TableGesture` with these fields, the gesture kinds being the designer's own (`setCell`, `addRow`, `addSort`, `toggleGroup`, `addView`, ...). Without `view` the gesture names the view the connection shows, as the Knowledge designer's panel sends every gesture. The step waits for the edit's `TableEditOutcome` on the stream and is refused when the call answers with a sentence or the outcome says it was not written. |
| `{ "view": "<view id>" }` | `DesignerService.SetTableView`: the view the connection shows, which the standalone host writes as the view the file is left in; the step waits for that edit's outcome (three seconds, after which it is taken to have written nothing, as for the view already shown). |
| `{ "save": { "path"?, "content" } }` | `EditorService.SaveText` of a text editor's content (the document, or `path`). |

Every step may carry `"expect": "refused"`, for an edit the standalone host refuses: the recorder then fails when it is accepted. Without it, a refused step fails the recording, so a script never half-records.

### Tables

A script whose tool is a designer whose document is a table - `etalii/knowledge` - is not opened as a diagram. The connection is the tab's one stream, `WorkspaceService.Watch` (no `ContextService.Watch` and no `DiagramService.Open`), and the registration is opened on it with `DesignerService.OpenTable` under a stream id of the recorder's, then `DesignerService.SetTableWindow` asks for the first 60 lines, as a tab does once it is laid out. The recorder reads the table stream for its baseline, the view the connection shows (from the baseline and every structure change) and the edits' outcomes. `table` and `view` steps are sent as the table above says; `undo` and `redo` run on the project's history as for a diagram, which is where the standalone host's `KnowledgeSessionFactory` writes every edit.

### Ids the standalone host generates

A mind map node, a timeline element or connection added by a step gets an id the standalone host generates at random (`ShortGuid.NewShortGuid()`), so a later step cannot name it in advance. `$N` in an id names the N-th element (from 1) and `$cN` the N-th connection the standalone host generated so far in the replay, in the order the diagram stream reported them (a connection being an element sent with its two ends, or one whose type ends in `+connection`, `+relationship` or `+relation`, the dependency graph's): `{ "select": ["$2"] }`, `"target": "rel:discovery->$1"`. These are an addition to the contract's script format.

A table sends no elements, so after every step of a table the recorder reads the project's files for ids that were never there: a ShortGuid is 25 characters of base 36, and an id in none of the fixture's files and not seen before is generated, in the order it stands in the files, with the type `etalii/knowledge+id`. `$N` in a `table` step's `row`, `column`, `view`, `target`, `values` and `settings` names the N-th of them: a row added, an option made from a cell's list, a view added. A step should make at most one such id, so that the order it stands in the file is the order it was made in.

`recorded.json` lists every generated id with the step that made it, its element type and whether it is a connection. The bytes of a golden hold those ids, so a replay in the desktop host matches a golden only when it generates the same ids at the same steps, or when it compares the two with each generated id put in place of its own.

## Goldens

The recorder replaces `parity/goldens/<tool>/<name>/` with the bytes of every file of the project the script changed or added, at its path in the project, and `recorded.json`: the script, the standalone commit, the recorder version, the platform, the time, the edit count, the changed and removed files, the generated ids, and what each step sent and how it was answered. `.gitattributes` marks `parity/goldens/**` and `parity/fixtures/**` `-text`, so git keeps both byte for byte.

## What it does about the standalone host's own behaviour

- **A file written with the platform's line ending.** The C4 layout sidecar (`*.layout.json`) and the Wardley map's identity sidecar (`*.identities.json`) are written with `System.Text.Json`'s indented output, whose line ending is `Environment.NewLine`: CRLF when recorded on Windows, LF on Linux. A golden is the standalone host's bytes on the platform in `recorded.json`.
- **A write that fails on Windows.** The standalone host validates a changed file in the background, reading it; a write landing while it reads can fail (`File.Replace`, 0x80070497), and the step answers "could not be written" (or, for a file beside the document such as a layout sidecar, the command succeeds and a notice on the context stream says so). The recorder waits for the stream to fall quiet and then 750 ms after every step, as a person's next edit would, and when a write fails all the same it runs the whole replay again on a fresh host, up to three times. A golden is always one clean run.
- **An explorer entry renewed under the document.** When the standalone host replaces a file on disk the document's explorer entry can be renewed, and a selection naming the old one is refused. A browser tab learns the new entry from its hierarchy stream; the recorder looks the entry up again before every step.

## What the scripts cannot record

- **Adding an element to a C4 view.** At `8b6ccfc` no add of the C4 module can be reached through the API. `C4ContextActionProvider` never offers `c4.add-container` or `c4.add-component`: it keeps only the kinds `C4Placement.Refuse(C4Workspace.Empty, kind, parentId: "")` accepts, and a container or component without a parent is refused there. `c4.add-person` and `c4.add-softwaresystem` are offered only for an empty element id, which the element-source resolver refuses ("This item is no longer available"), and without a source the selection is the document's explorer entry, whose hierarchy scope offers no C4 action ("That action is not available for this item"). The C4 script edits, moves and relabels what the model already holds instead.
- **Undoing a Wardley map's removal or rename.** Such an undo restores the whole text (`RestoreDocumentCommandHandler`) and reloads it twice, once through the store and once through the file watcher, and an element the restored text brings back gets a fresh ShortGuid in each reload. The two reloads can run at once: in one recording both reconciled against the identities from before the undo and minted two sets, the second kept, and in another the second found the first's. Which set the sidecar is left with is a race, not behaviour, so the Wardley script undoes neither; an undo of a one-line edit writes through the store's own save and is recorded as any other step.
- **Folding a mind map branch.** `mindmap.toggle-fold` is per-connection view state: it writes nothing, so it has no golden.
