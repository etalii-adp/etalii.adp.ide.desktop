# Ansible project structure (`ansible/structure`)

[`ansible-structure.dis`](ansible-structure.dis) specifies the Ansible project structure diagram in DISL 0.3: its metamodel, notation, empty canvas, read-only property rows with their reasons, four of its five findings, the reparenting refusal, its ids and its wire ids. Its folder is read through the FBL 0.2 binding `ansible-structure.fbl#ansible` by the plugin `net.etalii.adp.ansible.folder`. This page holds what DISL 0.3 and FBL 0.2 cannot state about the tool, each point tied to the folder of the parity transcript that shows it, and what would resolve it. Anything the two languages can state is in the definition or the binding, not here.

| | |
|---|---|
| Origin | `ansible/structure` |
| Kind | Diagram |
| State | ⚗️ Prototype (standalone) |
| Display name | Ansible project structure |
| Purpose | See how the playbooks, roles and inventories of an Ansible project include and depend on each other. |
| Why specialized | An Ansible project spreads its structure over many folders and includes; a graph shows the dependencies without touching Ansible's own files. |
| Implemented in | [etalii.adp.ide.standalone `src/diagrams/ansible-structure/`](https://github.com/etalii-adp/etAlii.adp.ide.standalone/tree/develop/src/diagrams/ansible-structure) |

Paths below are in [etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone) on `develop` unless they say otherwise; the module is `src/diagrams/ansible-structure/` (backend `backend/EtAlii.Adp.Diagram.AnsibleStructure/`, client `client/`, wire `api/ansible-structure.proto`).

## Sources

- The standalone module: code, tests and fixtures under `src/diagrams/ansible-structure/`, and the examples under `src/examples/diagrams/ansible-structure/`.
- The parity transcript `backend/EtAlii.Adp.Diagram.AnsibleStructure.Tests/Parity/ansible-structure.transcript.json`, generated from today's code by task 1 of the standalone specification `ansible-structure-disl-fbl` (commit `dce3c4d2a`, merged in pull request 328). Every sentence, label, code and severity in `ansible-structure.dis` is copied from it, except the empty-canvas sentence, which the transcript does not record and which is copied from `client/AnsibleCanvas.tsx`. The folders named below by their short name are under `Parity/Folders/` beside it (one per finding, listed in its `readme.md`) or the four trees under `Fixtures/`.
- The standalone specification `ansible-structure-disl-fbl` (requirements, design, tasks), whose findings A1 to A18 and B1 to B12 and open questions Q1 to Q5 are cited below by number, with the user's amendment of 2026-10-11 recorded at the top of its tasks.
- The original standalone specifications `ansible-structure-diagram` and `ansible-refinements`, removed from the tree in `ece03c36`, cited as R*n.m* and refinements R*n.m*.

## The subject and what the plugin delivers

- **The subject is a folder.** A registration (`.adp`) inside the folder makes it a diagram; its first line is the origin `ansible/structure`. There is no document extension and no body file (`Diagram.cs`: `Subject: DiagramSubject.Folder`). ADP never infers that a folder is an Ansible project, so the binding's `recognise` is empty and recognises every folder (R2.3).
- **Nothing in the folder is written.** The binding is read-only with the tool's sentence, "An Ansible structure diagram shows the folder as it is and changes nothing in it. Edit the file this value comes from in a text editor.", which is also the refusal of a property write; the plugin is never asked to plan (FBL 11.4, `plans: []`). `ZeroWrites.Tests.cs` snapshots the fixture trees byte for byte and by modification time around open, render, select, validate and describe.
- **Live update.** The module watches the folder and all subfolders and settles for 400 ms, restarting on each change, before one re-read of the whole folder, which is the binding's `settle`: the library's folder body (`FolderBody`) reads the folder through the binding and the plugin and settles, and the module's `AnsibleFolder` holds the watcher that feeds it, with `AnsibleFolders` keeping one open folder per registration for the connections that view it. The position a user drags is written into the registration and comes back through the same watcher.
- **What is delivered, and when.** The whole folder is read when it opens, and the session then narrows what it sends to the viewport: a view receives the nodes its viewport intersects, plus one hop of relation partners so that every drawn relation has both ends, plus every relation whose ends are both delivered; a relation without a target is delivered with its source. A change is diffed against what the connection holds and sent as removals and additions. Nothing folds and nothing is edited, so no group, ungroup or change delta is ever sent.
- **What the plugin delivers.** Every node of the metamodel with the attributes `ansible-structure.dis` declares, among them a playbook's `plays` (one item per play, with `ordinal`, `name`, `hosts` and `line`) and an inventory's `groups` and `variableFolders`; every relation, resolved, with a relation whose target is missing or an expression delivered without a target (b7-missing-and-expression); and one finding per file that does not parse, with YamlDotNet's message less its position prefix as the reason. The ids, the derived attributes, every property row but an inventory's groups and variable folders, the other four codes' findings and the reparenting refusal come from the definition.

The plugin is the module's reader (`AnsibleProjectReader`, `AnsibleYaml`, `AnsibleGraph`) behind the FBL contract, reading the files it is handed instead of the disk (standalone design, `AnsibleFolderPlugin`). Why it is a plugin is finding B1: a role is a folder with no file in it, its contents are counts, a playbook is a file whose whole top level has a shape, a task file is drawn only when something includes it, an inventory is YAML or else INI, and a target resolves by a collapsed relative path or not at all. None of these is a selector or a slot (FBL 11.5). How it reads, which no binding can state while the reader is a plugin:

| What | Where it is looked for | How it is recognised |
|---|---|---|
| Playbook | `*.yml` and `*.yaml` directly in the folder, then in `playbooks/` | The file's top level is a non-empty list whose entries are all mappings. Anything else is not a playbook and is ignored without complaint (R1.3). |
| Play | Each entry of a playbook that is not an `import_playbook:` entry | `name:` (may be empty), `hosts:` (may be empty), its line, its `roles:`, and the directive keys of the play mapping itself. Its `tasks:` list is never read: a play's `import_role`, `include_role`, `import_tasks` and `include_tasks` under `tasks:` yield nothing and are reported by nothing (a15-role-named-twice, b12-3-play-include-tasks). |
| `import_playbook` | An entry with an `import_playbook:` scalar | A sibling of the plays, not part of one. Its `when:` is kept. |
| Role | Every subfolder of `roles/` that is not a link | Its name is the folder name. A role with no folder in it is drawn and is hollow. |
| Role contents | `tasks/` (YAML files only), `handlers/`, `templates/`, `files/`, `defaults/`, `vars/` (every file), `meta/` and `library/` (existence only) | Counts of the files directly in each folder; subfolders are not counted, and a counted file is never opened. |
| Role dependency | `roles/<name>/meta/main.yml` or `main.yaml`, key `dependencies:` | Each entry is a bare name, or a mapping with `role:` or else `name:`. Its `when:` is kept. |
| Task directives | Every YAML file in a role's `tasks/`, and each play mapping | Keys `include_tasks`, `import_tasks`, `include_role` and `import_role`, in the short form and any fully qualified form ending in `.<name>`. The value is a scalar, or a mapping with `file:` or else `name:`. |
| Inventory | Each subfolder of `inventories/`, then each of `inventory`, `inventory.yml`, `inventory.yaml`, `hosts`, `hosts.yml` and `hosts.yaml` at the folder root | An environment folder is named for the folder, a root inventory file for its file name. |
| Inventory groups | The files of an environment folder with extension `.yml`, `.yaml` or none, or the root inventory file itself | YAML: every `<top>: children: <group>` key, and every top-level key other than `all` that has `hosts:`. Not YAML: INI, where `[group]` counts one per host line, `[group:children]` adds its members' direct host counts one level deep, `[group:vars]` is ignored, and a range line counts as one. An inventory that does not parse as YAML is never a finding. |
| Variable folders | `group_vars/` and `host_vars/` in each environment folder, and for a root inventory file those at the folder root | The number of files directly inside. |
| Annotations | `ansible.cfg`, `requirements.yml` and `requirements.yaml` at the root, and `collections/requirements.yml` or `.yaml` | Recorded on every playbook and play, not drawn (R4.1). |

- **Order.** Every listing is sorted ordinally before use. The root's playbooks are read, then `playbooks/`, then roles, then inventories (b6-reading-order: `site.yml` before `playbooks/aaa.yml`, which sorts first). Nodes and relations are delivered in that order, and it decides each play's colour index and the stacking inside a rank.
- **Tolerance.** A file that does not parse costs only its own facts, and is reported once with its 1-based line. Each file is recorded at most once, compared ignoring case. An empty YAML file is valid and holds nothing. A folder that is not there, or holds nothing recognised, opens empty with no finding (b5-not-there, b5-nothing-ansible).
- **Links.** A subfolder that is a link is skipped, so a link loop ends the walk; a file that is a link is read (b11-links: `linked.yml` is a playbook, `roles/linked` is not a role).
- **Resolution.** `UsesRole` (from a play's `roles:` or a play mapping's or a role's tasks' `include_role` and `import_role`) and `DependsOn` (from `meta/main.yml`) resolve to a role folder by exact ordinal name. `ImportsPlaybook` resolves to a recognised playbook whose path, relative to the importing file's folder with `.` and `..` collapsed, equals the target, compared ignoring case. `IncludesTasks` resolves to a task file of the same role relative to the including file's folder, compared ignoring case. A target holding `{{` is an expression: shown as written, never guessed at and never reported (R3.4). `Targets` goes from a play, or a single-play playbook, to every inventory that defines a matching group, and there is none when nothing matches.
- **Host patterns** are a deliberate subset of Ansible's: the pattern is split on `:` and `,` and each token trimmed; a token starting with `!` or `&` never makes a relation; `all` and `*` match any inventory that defines a group; a token ending in `*` matches by prefix; any other token matches a group of that exact name.
- **Not read.** No `when:` is evaluated (a `when:` list is joined with ` and `), no variable usage is inferred (R5.8), and nothing is run.

## Language gaps

These are the candidate changes for etalii-adp/etalii.adp that the conversion found, numbered as the standalone specification's section C, with gap 11, which the implementation found, after them. None is closed by the definition or the binding stating something the tool does not do; where the tool keeps a behaviour the languages cannot state, the class that keeps it is named. FBL has moved past 0.2 since the binding was drafted; its later drafts close none of the five FBL gaps below.

| # | Gap | Construct | Shown by | What would resolve it | Until then |
| --- | --- | --- | --- | --- | --- |
| 1 | **Folders in a folder subject** (B2) | FBL 10.2 and 11.2: file rules select files, and the plugin is handed "the relative path and bytes of every file the file rules select". A role with no file in it, a `meta/` with nothing in it and a variable folder with nothing in it never reach a plugin given files only. | b2-empty-folders (`roles/empty` is hollow and reported as `ansible.empty-role`; `roles/metaonly` with an empty `meta/` is not hollow; an empty `group_vars/` is a node with 0 files), b11-links (a linked folder is not a role); the rules `roles`, `roleFolders`, `environments` and `variableFolders` below | A file rule with `kind: "folder"`, whose matches are handed to the plugin as paths | The module's `AnsibleFolderListing` (`FolderExists`, `Folders`) |
| 2 | **A file rule that yields the path and not the bytes** (B3) | FBL 11.2: every selected file's bytes are read and handed over. The tool counts the files of a role's `templates/`, `files/`, `handlers/`, `defaults/` and `vars/` and of the variable folders, and opens none of them. | b3-large-binary (an 8 MiB file under a role's `files/` is counted), b12-7-role-counts; the rules `annotations`, `galaxy`, `roleContents` and `variables` below | `content: false` on a file rule | `AnsibleFolderListing` (`FileCount`), and the linked file of Q2 (`LinkedFiles`) |
| 3 | **What `family` means on a file rule under a plugin reader, and who reports a file that does not parse** (B4) | FBL 10.2 and 10.3: a file rule names a `family`, and a file "that cannot be read" is `std.unparseable`. Whether the host parses a plugin-read file by that family is not said. If it does, an INI `hosts` and a broken `hosts.yml` under `inventories/`, which the tool reads as INI and reports nothing for, gain a finding, and the reason is the library parser's, where the tool gives YamlDotNet's. FBL 7.4 reads a repeated key with a warning, where YamlDotNet refuses the file. | b4-ini-and-broken-hosts: the INI `hosts` is read as INI; the broken root `hosts.yml` is reported as `ansible.unreadable-yaml` at line 4 because it is also read as a candidate playbook, while the one under `inventories/` is silent; `repeated.yml` is reported at line 3 with "Duplicate key hosts" | A statement that a plugin reader parses its own files and its finding is the one reported | The binding gives no file rule a `family`, and the plugin parses with YamlDotNet |
| 4 | **The reading order of a plugin's elements** (B6), **and a relation without a target from a reader** (B7) | FBL 4.1.3 and DISL 8.6 read a folder's files in the ordinal order of their relative paths; whether the order a plugin delivers its elements in is the reading order is not said. FBL 5.4 says "DISL relations always have both ends", and FBL 11.2 does not say a plugin may deliver a relation with no target, which DISL 4.9 allows since 0.2. | b6-reading-order (`site.yml` before `playbooks/aaa.yml`), a8-a12-unreadable-and-order; b7-missing-and-expression (relations with a missing and an expression target, delivered without a target, only the missing ones reported) | The order a plugin delivers in is the reading order; FBL 5.4 and 11.2 allow a relation without a target where the type's target end is `optional` | The plugin's order is kept; the library's `FblElement.Target` is already nullable |
| 5 | **A stale entry whose file is unreadable, not gone** (B9) | FBL 8.5 removes a layout entry whose element is not in the model at the next write and reports `fbl.stale-view-data`. For a folder subject a node is missing whenever its file is momentarily unreadable, and FBL 7.3's last readable model is for a whole body, not one file of a folder. | b9-half-saved: `playbook:half.yml: 100 200` is kept when `playbook:other.yml` is moved, while `half.yml` does not parse | An entry is stale only when no file of the subject is reported unparseable, or after a read in which every file parsed | Positions are written by core's `RegistrationLayout`, which removes nothing and reports nothing (Q2) |
| 6 | **The file in the message of `std.unparseable`** (A8) | DISL 8.7 gives the message of `std.unparseable` one detail, `reason`; the file is in the finding's location and cannot be read by the message. The tool's sentence opens with the file. | a8-a12-unreadable-and-order ("'broken.yml' is not readable YAML: While scanning a multi-line double-quoted scalar, found wrong indentation." and "'roles/r/tasks/broken.yml' is not readable YAML: While scanning a quoted scalar, found unexpected end of stream."), b4-ini-and-broken-hosts, b9-half-saved | `detail.file` in the message of `std.unparseable` | The definition's message is `detail.reason`; the module's `AnsibleFindings` composes "'&lt;file>' is not readable YAML: &lt;reason>" from the finding's file and its reason |
| 7 | **One form row per item of a list** (A9) | DISL 7: a context-menu entry has `forEach`, a form item has not, and a row's id, label and value cannot come from an item. An inventory shows one row per group (id `ansible.group.` and the group's name, the name as label, "1 host" or "N hosts") in the group Groups, and one per variable folder (id `ansible.vars.` and the folder's name, "1 file" or "N files", its reason naming that folder) in the group Variables. | b12-5-root-inventories (`ansible.group.db`, `ansible.group.web`, `ansible.vars.group_vars` with "1 file" and "Defined in group_vars; edit it in a text editor."), b12-2-targets-declaration, b12-6-extensionless-inventory | `forEach` on a form item, with its `id`, `label` and `value` read from `item` | `inventoryInspector` states the Identity rows only; the module's `AnsibleInventoryRows` adds the rest |
| 8 | **The order of findings of a folder subject** (A12) | DISL 8.6 orders reader findings, then built-ins, then rules in declaration order; `constraints.order` orders by code and then by line. The tool reports unreadable files, then missing targets in the order the relations were derived, which interleaves `ansible.role-missing` and `ansible.dangling-import`, then unmatched hosts, then empty roles. Every finding names its own file, and a line orders nothing across files. | a8-a12-unreadable-and-order: two `ansible.unreadable-yaml`, then `ansible.role-missing` at `a.yml:4`, `ansible.dangling-import` at `b.yml:1`, `ansible.role-missing` at `roles/r/meta/main.yml:2`, `ansible.dangling-import` at `roles/r/tasks/main.yml:2`, then `ansible.unmatched-hosts` at `a.yml:1`, then `ansible.empty-role` | `constraints.order` by file in reading order before line, and a way to keep one declared list's order across two codes | The module's `AnsibleFindings` |
| 9 | **Opening an element's source in the host, and a second shortcut** (A13) | DISL's `reveal` scrolls the canvas to an element; nothing opens an element's file or folder in the host's explorer, and an operation has one `shortcut`. Double-click, Enter and Space on the selected node reveal its file or folder. | `client/AnsibleCanvas.tsx`; the transcript's `selection` gives the path each element reveals (a play its playbook's, a `Targets` relation none) | An action that opens an element's source in the host, and more than one shortcut | The definition's `revealFile` operation names the plugin's `reveal` action with Enter; the handler in `client/AnsibleCanvas.tsx` does it, Space included |
| 10 | **The ranked layout** (A14), DISL D.2's open question 9 | DISL 0.3's standard algorithms are not ranks by import depth with every node of a rank at the rank's width and inventories in a band beneath. | Every folder's `payloads` (b12-4-single-play-tint: `playbook:a.yml` at 0, 0 and `playbook:b.yml` at 0, 56, its plays at 168; b12-5-root-inventories: the band at y 96) | A ranked algorithm with a band among DISL's standard ones | `AnsibleLayout` with `AnsibleMetrics`, declared as the plugin's layout `ranks`, with `layered` as fallback |
| 11 | **Formatted text in a message** (found by the canvas's switch to this definition) | DISL 2.3: a Message is a LocalizedText or a CEL string, plain text; `markup` belongs to a `text` attribute (DISL 4), not to a Message. The tool's empty canvas set `playbooks/`, `roles/` and `inventories/` in a monospace font. | `canvas.empty` of `ansible-structure.dis`; the standalone canvas test `client/ansibleCompiledDefinition.test.tsx`, which states the difference (the transcript does not record the empty sentence) | `markup: "plain" \| "markdown"` on a Message, as on a `text` attribute | The canvas draws the sentence as plain text; no class stands in for it |

### The eight file rules the binding cannot carry

The requirements' draft of the binding has fifteen file rules. Eight of them mark, with a key that means nothing to a host, the construct of gap 1 (`x-folder`) or of gap 2 (`x-content: false`). A host ignores an `x-` key, so leaving them in would hand the plugin the bytes of every counted file; the binding the module embeds leaves them out, and `AnsibleFolderListing` supplies what they stood for. They are the evidence of gaps 1 and 2:

| Rule | Glob | Stood for | Gap |
| --- | --- | --- | --- |
| `annotations` | `ansible.cfg` | Its presence only | 2 |
| `galaxy` | `collections/requirements.y*ml` | Its presence only | 2 |
| `roles` | `roles/*` | A role, which is a folder | 1 |
| `roleFolders` | `roles/*/*` | The folders a role holds, `meta/` and `library/` by their existence | 1 |
| `roleContents` | `roles/*/*/*` | The count of a role's files, never opened | 2 |
| `environments` | `inventories/*` | An environment, which is a folder | 1 |
| `variableFolders` | `**/*_vars` | A variable folder, which is a folder | 1 |
| `variables` | `**/*_vars/*` | The count of a variable folder's files, never opened | 2 |

## The stand-ins the module holds

Each is named for the gap it stands in for, and goes when its gap is closed (standalone design, *What remains in the module, and why*):

- **`AnsibleFolderListing`** (gaps 1 and 2): that a folder exists, whether a file is there, the folders under one, how many files a folder holds without their bytes, and the bytes of a file that is a link, which FBL 10.2 never selects and the tool reads (B11, Q2).
- **`AnsibleFindings`** (gaps 6 and 8): composes the sentence of `ansible.unreadable-yaml` as "'&lt;file>' is not readable YAML: &lt;reason>"; puts the findings in today's order; and passes on a `std.unparseable` only when it names a file of the folder, so that a folder FBL 10.3 would make an unreadable body opens empty with no finding (B5, Q2).
- **`AnsibleInventoryRows`** (gap 7): the rows `ansible.group.<name>` and `ansible.vars.<name>` of an inventory.
- **The reveal handler in `client/AnsibleCanvas.tsx`** (gap 9): double-click, Enter and Space.
- **`AnsibleLayout`, `AnsibleMetrics`** (gap 10): the plugin's layout.

## What remains in the module, and why

Every type of the product project `backend/EtAlii.Adp.Diagram.AnsibleStructure/`, checked against a listing of its files, beside the gap or the host concern that keeps it; a type with neither is not there. A stand-in goes when its gap is closed; a host concern is the host's and stays.

| Type | Kept by |
| --- | --- |
| `AnsibleFolderPlugin` | The plugin reader of FBL 11.5 (finding B1): the reading the table above describes is no selector or slot. |
| `AnsibleProjectReader`, `AnsibleYaml`, `AnsibleGraph` | The plugin's reading: the files the binding hands it, parsed with YamlDotNet (gap 3), and the relations resolved against what the folder holds. |
| The records and enums of `_Model/` (`AnsibleProject`, `AnsiblePlaybook`, `AnsiblePlay`, `AnsibleRole`, `AnsibleRoleContents`, `AnsibleTaskFile`, `AnsibleInventory`, `AnsibleInventoryGroup`, `AnsibleVariableFolder`, `AnsibleDirective`, `AnsibleDirectiveKind`, `AnsibleNode`, `AnsibleNodeKind`, `AnsibleEdge`, `AnsibleEdgeKind`, `AnsibleTargetResolution`) | What the plugin reads into, before it hands the model its elements (B1). |
| `AnsibleFolderListing` | Gaps 1 and 2, and the linked file of Q2. |
| `AnsibleFindings` | Gaps 6 and 8. |
| `AnsibleInventoryRows` | Gap 7. |
| `AnsibleLayout`, and `AnsibleMetrics` in `_Model/` | Gap 10. |
| `AnsibleElementMapper` | The host's wire: the model in the element and delta vocabulary of `api/ansible-structure.proto`, which does not change. |
| `AnsibleContextSourceResolver` | The host's selection: an element id resolved against the folder its registration sits in. |
| `AnsibleDefinition` | The definition: the bundled `ansible-structure.dis` and the binding, and what is derived from them. |
| `AnsibleFolder`, `AnsibleFolders` | The opened subject: the library's folder body over the registered folder, the watcher it does not hold, and one open folder per registration. |
| `AnsibleSession`, `AnsibleSessionFactory` | The host's session seam: one connection's viewport and what it has been sent, and the folder of the registration as the subject. |
| `Diagram` | The host's catalog entry: the origin and a folder subject with no extension. |
| `AnsibleValidator` | Core's validator seam: the findings handed over with project-relative locations. |
| `AnsibleContextPropertyProvider` | Core's property seam: the rows, read-only, refusing a write with the property's own reason. |
| `ServiceCollectionAddAnsibleStructureExtension` | The host's registration of the types above. |

Beside them the client keeps two: the reveal handler of `client/AnsibleCanvas.tsx`, which opens a node's file or folder (gap 9), and `client/ansibleBindings.ts`, which says what the library cannot yet read from the definition, Space as a second reveal key among it (gap 9).

## Differences the definition cannot bring in line

- **Two relations with one id** (A15, Q3). A role named twice in one `roles:` list, and a role imported and included by one role's tasks, each give two relations with one id, and both are delivered (a15-role-named-twice); two root inventory files each get the root's `group_vars/`, delivered twice as `vars:group_vars` (b12-5-root-inventories). The canvas draws one of each. DISL 11.5.4 gives the id to the first in reading order and draws the later one with an ephemeral id, so a DISL runtime draws both. The definition switches `std.duplicateId` off, so that nothing is reported about them, as today.
- **The order of findings**, gap 8.
- **A play's position is `ordinal`, not `index`.** The standalone design names the field of a playbook's `plays` items and the play's attribute `index`, which DISL 2.2 reserves; the definition names both `ordinal`, and the plugin delivers that name.
- **`ansible.unreadable-yaml` has one code.** The task asked for a computed `code` on `std.unparseable`; every unreadable file the tool reports is `ansible.unreadable-yaml` and the finding for a folder that is not there is not passed on, so the definition gives the code as a constant.

## Persistence: positions in the registration

The only stored data is a `layout:` block in the registration, written by core's `RegistrationLayout` (`src/backend/EtAlii.Adp.Hierarchy/RegistrationLayout.cs`), which is FBL 8.3's layout block:

```text
ansible/structure
layout:
  playbook:half.yml: 100 200
  playbook:other.yml: 10.5 -20.25
```

- One line per node a user dragged: two-space indent, id, then x and y of the box's top-left corner in canvas units, with up to three decimals, entries ordered by id ordinally. The id is read up to the last `: `, so an id holding a colon is safe. Only the position is stored; a box keeps its computed size. Everything above the block is kept byte for byte, a malformed line is ignored, and a one-line registration gets the block at its first move (refinements R2).
- A stored position wins over the computed one node by node. A stored id the folder no longer produces is ignored when read and **kept** at the next write, since nothing calls `Prune` (b9-half-saved, gap 5); FBL would remove it and report `fbl.stale-view-data`, which the tool does not (Q2).
- The ids are the definition's `derived` expressions: `playbook:` and the path, `play:` with the playbook's path, `#` and the 0-based ordinal (a5-first-plays: `play:one.yml#0` and `play:two.yml#0`), `role:` and the name, `taskfile:`, `inventory:` and `vars:` with the path, and for a relation `edge:`, the source's id, the relation type, and the target as written, joined by `|`, with the target's id appended for `Targets`. Paths are folder-relative with `/`, so the id pattern is widened to admit them. A relation's id uses the target as written, so a missing role's relation keeps its id when the role appears.
- A move is one undoable command (`SetRegistrationLayoutCommand`) and the diagram's only edit. The write is not echoed locally: it returns through the folder watcher.
- A move under a parent is refused with the sentence of the definition's `noReparent` gesture constraint. Three refusals stay the session's, because no DISL construct says them: a move of a relation, "That element is not something this diagram can move."; any move without a history, "This diagram is read-only."; and any move of a diagram opened without a registration, "This diagram was opened without a registration, so there is nowhere to store a position."

## Layout

The definition names the layout `ranks` of the plugin with `layered`, right, as fallback, which keeps the reading direction but not the ranks or the band. The plugin is `AnsibleLayout.cs` with `AnsibleMetrics.cs`, pure and deterministic (R6.1):

- **Metrics**, in canvas units: font size 14, horizontal padding 12 each side, node height 32 (every node is one line), minimum width 72, rank gap 96, row gap 24, band gap 96. A node's width is the larger of 72 and its name's width plus 24, measured with the definition's `textMetric` (`average`, advance 0.55, UTF-16). The backend sends each box and the client draws the box it is given.
- **Ranks**, left to right: a playbook's rank is its longest path of `import_playbook` relations from a playbook nothing imports, and a cycle ends rather than recursing; a play is one rank right of its playbook; all roles share the rank after the deepest playbook or play, and all task files the rank after that.
- **Within a rank**, nodes stack top to bottom in delivery order with the row gap between them. A rank is as wide as its widest node and every node in it takes that width.
- **The band**: inventories and variable folders go in one row beneath the ranks, from x 0 and a band gap below the tallest rank, each at its own width with the rank gap between them, each inventory followed by its variable folders.
- After the layout, every node with a stored position takes it; every other node keeps its computed one, which is recomputed on every change, so a node the user never moved can move when the folder changes.

## Colours and drawing

- The definition's tokens copy the light and dark values of `client/ansible-structure.css`. A play's fill is its hue mixed 18 % into the raised surface (`color().mix()` with 0.82 towards the surface), its hue chosen by the play's project-wide index modulo six. Only a drawn play is tinted; a single-play playbook carries no index and is never tinted, although its play takes one (b12-4-single-play-tint: the next playbook's plays start at 1).
- Kind is never carried by colour alone: a playbook has a 2 px primary outline; a play a primary outline dashed 6 3; a role rounded corners; a task file a dashed 2 2 outline; an inventory and a variable folder the muted outline; a hollow role is dashed 4 4 at 60 % opacity.
- A relation leaves the source's right side and enters the target's left side at mid-height on a horizontal forward bezier, which loops round for a backward relation, with a filled arrow (`arrowFilled`) at the target. A dynamic relation (`include_*`) is dashed 5 4; `DependsOn` is primary, dotted 1 4 with round caps, 2 px; `Targets` is 1 px at 55 % opacity. Its label reads the directive, followed by " when " and the condition when there is one.
- A relation without a target is the definition's `stub`: 48 px out of the source's right side, in the warning colour, dashed 3 3, labelled with the target as written and " (missing)" or " (expression)".
- An empty folder shows the definition's `canvas.empty` sentence, "Nothing here is laid out the way Ansible expects, so there is nothing to draw. Playbooks at the folder root or under playbooks/, roles under roles/, inventories under inventories/.", drawn as plain text. The tool set the three folder names as code until its canvas drew this definition, which no Message can state (gap 11).

## Navigation

- Double-click, Enter or Space on the selected node reveals its file or folder in the explorer: a playbook's file, a play's playbook, a role's folder, a task file, an inventory's folder or file, a variable folder (gap 9).
- A selected relation answers as its declaring file, and its selection text is the directive followed by the target, as in `include_tasks: tls.yml`.

## Known quirks of the tool

Kept byte for byte in this conversion, each in the transcript, and each to be fixed afterwards on purpose with a test seen to fail first (Q4). The definition mirrors each where it states the behaviour.

- **`hosts: ::1` is reported as unmatched**, because the pattern is split on `:` before the implicit check, which leaves the token `1`; `127.0.0.1` is not reported (b12-1-ipv6-loopback). The definition's `hostTokens` splits the same way.
- **A `Targets` relation carries a synthetic declaration**: its rows say "Declared in" `:1` and "Directive" `roles:`, its reason is "Defined in ; edit it in a text editor.", and it selects to an empty path (b12-2-targets-declaration).
- **A play's own tasks are not read**, so a play's `include_tasks` or `import_tasks` naming no file is silent, and so is a play's `import_role` (b12-3-play-include-tasks, a15-role-named-twice). The directive keys are looked for on the play mapping itself.
- **A single-play playbook is never tinted**, although its play takes a colour index (b12-4-single-play-tint).
- **Root inventory files share the root's variable folders**: each root inventory file gets the root's `group_vars/` and `host_vars/`, so with two of them the folder is delivered twice with one id (b12-5-root-inventories).
- **An inventory folder reads groups from every file with no extension**, not only from files named like an inventory (b12-6-extensionless-inventory).
- **A role's counts are uneven**: `handlers`, `templates`, `files`, `defaults` and `vars` count every file, YAML or not; `tasks` counts YAML only (b12-7-role-counts).

## Examples

Under `src/examples/diagrams/ansible-structure/`, each project with its upstream `LICENSE.md` beside it; most hold a registration (`.adp`) with stored positions, which the real-file check of the standalone specification reads (B10: 34 stored keys, 6 of which hold a `/`). The four fixture trees under `backend/EtAlii.Adp.Diagram.AnsibleStructure.Tests/Fixtures/` (`broken`, `infrastructure`, `ini-inventory`, `unconventional`) and the finding folders under `Parity/Folders/` hold no registration in the tree, since ADP never infers one; the transcript writes one in its temporary copy of each folder when it records a move.
