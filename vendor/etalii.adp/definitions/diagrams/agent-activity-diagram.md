# Agent activity diagram

The companion to [agent-activity-diagram.dis](agent-activity-diagram.dis). The `.dis` specifies the diagram type in DISL 0.4 as far as DISL reaches, and [agent-activity-diagram.fbl](agent-activity-diagram.fbl) binds its file; this page holds everything else a host needs to build the same tool: the file format written out for agents, the reference layout, how a link opens in each host, and the instruction text an agent is given. [agent-activity-diagram.schema.json](agent-activity-diagram.schema.json) is the file's JSON Schema.

The diagram type was specified in the standalone IDE, [etalii-adp/etalii.adp.ide.standalone](https://github.com/etalii-adp/etalii.adp.ide.standalone), as the spec-workflow specification `agent-activity-diagram` (requirements, design and tasks, approved 2026-10-09), origin `etalii/agent-activity-diagram`. These files are its definition: where they and that specification disagree, these files win (its Requirement 1.6). `standalone:` marks a path in that repository. The definition was written by etalii.adp's spec [014-agent-activity-diagram](../../specs/014-agent-activity-diagram/).

## What the diagram is for

An agent activity diagram gives developers one picture of where what work is happening: which agents work on which specification of which project, on which branch and in which folder, and on what system. Agents keep it current by editing one plain YAML file as they work; a person reads it as a picture, arranges it and folds away what they do not need.

- **Elements.** A project is one body of work, such as a repository. A specification is one piece of specified work in a project, with a status and a list of tasks inside it. An agent is one worker on a specification; a thread of a project chat is an agent. A location is where an agent's work is: a branch, and a folder, the worktree's or `Default` for the repository's main checkout, with the pull requests opened from it listed inside. An environment is a system the work happens on: a local machine, a cloud environment, a container, a WSL system or another kind.
- **Relations.** A project to its specifications, a specification to its agents, an agent to its locations, a location to its environment, and nothing else. A specification has at most one project, an agent at most one specification, a location at most one agent and one environment. An environment is one element however many locations use it.
- **Who writes it.** Agents write the elements, the tasks and the pull requests. The person's part is where elements are locked, which groups are folded and whether archived specifications are shown; it is kept in the same file, and agents leave it alone.
- The notation is ADP's own, which is why the origin is `etalii/`.
- The Notion "Tools" database row: kind Diagram, state Specified, the page holding what the diagram is for and how it functions.

## The file format (`.aad`)

One YAML file with the extension `.aad`, registered by an `.adp` file beside it whose origin line is `etalii/agent-activity-diagram`. The binding `aad` in [agent-activity-diagram.fbl](agent-activity-diagram.fbl) reads and writes it; this section says the same for a person or an agent writing it by hand.

```yaml
agent-activity-diagram: 1
# What the agents of one project were doing on 2026-10-08, kept by agents and by ADP.
projects:
  - id: p-standalone
    name: etalii.adp.ide.standalone
    link: https://github.com/etalii-adp/etalii.adp.ide.standalone
  - id: p-adp
    name: etalii.adp
specifications:
  - id: s-knowledge
    project: p-standalone
    name: Knowledge designer
    link: .spec-workflow/specs/knowledge-designer/requirements.md
    status: progressing
    tasks:
      - id: t-k2
        title: Trial the three bindings
        status: progressing
        updated: 2026-10-08T23:44:00+02:00
      # Waits on Peter's answer about the table header.
      - id: t-k6
        title: Table component
        status: input-required
        updated: 2026-10-08T21:10:00+02:00
        link: https://github.com/etalii-adp/etalii.adp.ide.standalone/pull/150
  - id: s-rider
    project: p-standalone
    name: Rider warnings cleanup
    status: archived
agents:
  - id: a-dev2
    name: Developer 2
    specification: s-knowledge
    link: https://claude.ai/code
locations:
  - id: l-kd
    agent: a-dev2
    environment: e-fractal
    folder: .claude/worktrees/kd
    folderLink: C:/git/etalii.adp.ide.standalone/.claude/worktrees/kd
    branchLink: https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/features/knowledge-designer
    branch: features/knowledge-designer
    pullRequests:
      - id: pr-150
        title: "150: Knowledge designer, tasks 2 and 6"
        updated: 2026-10-08T23:50:00+02:00
        link: https://github.com/etalii-adp/etalii.adp.ide.standalone/pull/150
environments:
  - id: e-fractal
    name: Fractal
    kind: local-machine
view:
  showArchived: false
  placements:
    - element: s-knowledge
      x: 240
      y: -80
  groups:
    - element: s-knowledge
      group: pending
      collapsed: false
```

- **The header.** The first key is `agent-activity-diagram: 1`, the format's version. A file of a later version opens read-only.
- **Five lists**, each left out while it is empty and never written `[]`: `projects`, `specifications`, `agents`, `locations`, `environments`. A new file is the header alone, with CRLF line endings; each list is created at the end of the file with its first entry and removed with its last.
- **Keys of each entry, in this order:**

| List | Keys | Required |
| --- | --- | --- |
| `projects` | `id`, `name`, `link` | `id`, `name` |
| `specifications` | `id`, `project`, `name`, `link`, `status`, `tasks` | `id`, `name`, `status` |
| `tasks` (in a specification) | `id`, `title`, `status`, `updated`, `link` | `id`, `title`, `status` |
| `agents` | `id`, `name`, `specification`, `link` | `id`, `name` |
| `locations` | `id`, `agent`, `environment`, `folder`, `folderLink`, `branchLink`, `branch`, `pullRequests` | `id`, `branch` |
| `pullRequests` (in a location) | `id`, `title`, `updated`, `link` | `id`, `title` |
| `environments` | `id`, `name`, `kind`, `link` | `id`, `name`, `kind` |

- **A relation is a key on the element at its "one" end**, naming the other element by its id: a specification's `project`, an agent's `specification`, a location's `agent` and `environment`. A relation has no id and no entry of its own. The file therefore cannot say that an agent has two specifications, and an agent that changes what it works on changes one line. A key that names nothing is kept and reported.
- **Statuses** of a specification are written `pending`, `progressing`, `input-required`, `finished` or `archived`; of a task `progressing`, `pending`, `input-required` or `finished`. An environment's **kind** is `local-machine`, `cloud`, `container`, `wsl` or `other`. A status is stored as written and is not computed from the tasks.
- **`updated`** on a task and a pull request is when it was last changed, as an ISO 8601 moment with its offset, written plain. Rows are drawn most recent first within their group; one without it comes after those that have one, in file order. Their order in the file does not matter, so nothing ever has to move in it.
- **`folder` left out means `Default`.** An empty optional key is removed, not written empty.
- **Ids.** ADP writes a ShortGuid: a version 4 GUID written as 25 characters of base 36. Anyone may write any id of letters, digits, `_`, `.` and `-`; readable ones such as those above are encouraged, since an agent finds its own entries again by them. Ids are unique across the whole file and never change on a rename.
- **A link** is an `http` or `https` address, or a path. A relative path is relative to the activity file's folder; an absolute one, such as `C:/git/...`, is taken as written. Anything else is kept, reported as `aad.link-not-openable` and never opened. A location has two: `branchLink` for its branch and `folderLink` for its folder.
- **The person's part** is the mapping `view`: `showArchived` (absent means `false`); `placements`, one entry per locked element and nothing for the others; and `groups`, one entry per group whose state differs from its default. A group is a task status as written, or `pullRequests` for a location's pull requests. ADP creates `view` at the end of the file with the first of them and removes it when the last is gone (FBL 0.4). Agents leave `view` alone. When an element is removed, its placement and group states go with it on ADP's next write, without a finding.
- **Comments, key order, indentation, line endings and keys the diagram does not read** are kept byte for byte. An edit in ADP changes only the bytes it concerns: the fixtures `specifications/fbl/fixtures/agent-activity-read/`, `agent-activity-edits/` and `agent-activity-new/` show each kind of edit. A key the diagram does not read is reported at severity info.

## Instructions for agents

The text an agent is given so that it keeps the file current. A host ships it beside its examples (standalone Requirement 8.3); this is its source.

> The file `<name>.aad` is the overview of who works on what. Keep your own entries in it current; never touch another agent's entries, and never touch `view`.
>
> - **Starting work on a specification:** add yourself under `agents` with an id you will recognise (`a-<your name>`), your `name`, and `specification:` naming the specification's id. Add the specification under `specifications` first if it is not there, with its `project` and `status: progressing`.
> - **Changing a status:** change the `status` of the specification or the task in place, and set the task's `updated` to now, with your offset.
> - **Adding a task:** add it under its specification's `tasks` with an `id`, `title`, `status` and `updated`. Create `tasks:` right after `status:` if the specification has none.
> - **Adding a location:** add it under `locations` with `agent:` naming you, `environment:` naming the system you run on (add it under `environments` if it is not there), `branch`, and `folder` when you work in a worktree.
> - **Adding a pull request:** add it under your location's `pullRequests` with an `id`, `title`, `updated` and `link`. Create `pullRequests:` right after `branch:` if there is none.
> - **Finishing:** set the tasks you finished to `finished`, the specification to `finished` when all of it is done, and remove your agent entry and its locations when you stop.
> - Write a list's first entry under a new key; never write `[]`. Keep keys in the order of the table in agent-activity-diagram.md. Leave out a key you have no value for. Keep comments you find.

## Layout

The `.dis` declares the layout as DISL 0.4's `force` with the tiers Project, Specification, Agent, Location, Environment, a gap of 24, `respect: "pinned"` and `trigger: "onChange"`, and DISL 10.4 states what its result must satisfy: tiers in order from the centre, no overlap, locked elements fixed, the same file giving the same picture, and little movement on a live change. This is the reference algorithm, so that a second host draws a picture close to the first. A host **may** differ in its numbers while it keeps those properties; it computes positions in the client, where the elements' sizes are known, and never writes them to the file.

1. **Tiers.** Each element's tier is the index of its type in `tiers`: projects 0, specifications 1, agents 2, locations 3, environments 4. An empty tier takes no ring.
2. **Ring radii.** Going outwards, the ring of tier k has the radius R(k) = max(R(k-1) + 220, n(k) × (w(k) + 24) / 2π), where n(k) is the number of elements in the tier, w(k) the widest of them, and R of the first ring that holds elements is 0 when it holds one element and n × (w + 24) / 2π otherwise.
3. **Start positions.** The i-th element of tier k in file order (counting from 0) starts at the angle 2π × i / n(k) + 0.5 × k radians, at radius R(k), measured from the origin with y downwards. A locked element starts and stays at its stored position.
4. **Forces, 300 steps.** In each step, in file order, every element that is not locked gets the sum of: a spring along each of its relations, 0.05 × (d − 200) towards the other end, with d the distance between the two centres; a repulsion from every other element, 20000 / max(d, 1)² away from it; and a pull towards its ring, 0.1 × (R(tier) − r) along the line from the origin, with r its distance from the origin. Its move is that sum, cut to at most 30 × (1 − s / 300) for step s. Two elements at the same centre are pushed apart along the angle of the first's start position.
5. **No overlap.** Then, up to 50 passes in file order, any two elements whose rectangles grown by half the gap overlap are pushed apart along the axis of the smaller overlap, by half the overlap each, or the whole of it for the one that is not locked when the other is. Two locked elements are left where they are.
6. **Live changes.** When the file changes while the diagram is open, the layout starts from the positions on screen, a new element starts at its start position, and step 4 runs 100 steps instead of 300 before step 5. The picture after a run of live changes may therefore differ from the one the same file gives when it is opened; what the person wants kept, they lock.
7. **Motion.** Elements move from their old to their new positions over 250 ms, and not at all when the person has asked their system for reduced motion.

A drag locks the element where it is dropped (DISL 10.4, "a drag pins"). An element added from the toolbox is locked where it was dropped; one an agent adds is not locked.

## Opening a link

DISL 0.4's `open` action opens an `http` or `https` address in the platform's browser without giving that page access to the runtime, reveals a path through the host, and refuses anything else (DISL 9.4). Per host:

- **Standalone:** an address through `window.open` with `noopener` and `noreferrer`; a path inside the project through `revealPath`, which also opens a file ADP has a tool for; any other path in a small dialog that shows it, says why it was not opened, and offers to copy it.
- **IntelliJ, VS Code and Eclipse:** an address through the IDE's external browser; a path inside the project selected in the project view and opened when the IDE has an editor for it; any other path shown with an offer to copy it.
- **Notion:** an address in a new tab; a path is shown with an offer to copy it, since a page cannot reach the reader's files.

## Interaction

- **Adding** an element is a drop from the toolbox, one tool per element type, and none for a task or a pull request; the element is locked where it was dropped and its name is edited in place. On the wire a tool drops as `add-<tool>` (`x-aad.tools`); every other name is DISL's own.
- **Connecting** two elements draws the one relation the pair allows. Connecting an agent that already works on a specification is refused with the reason `aad.second-specification` names; the person removes the first line themselves.
- **Removing** an element clears every key that names it, so its relations go with it in one undoable step, together with its placement and group states.
- **Tasks and pull requests** are added from their element's menu, renamed with F2 or a double-click, and removed from their own menu. A task's status is set from its menu or the property grid. They are never reordered by hand.
- **Lock and unlock** are on every element's menu, and *Unlock all positions* on the canvas's menu while anything is locked.
- **A group heading** collapses or expands its group, and the element grows or shrinks with its relations following. A task that moves into a collapsed group does not expand it.
- **The switch** *Show archived* on the canvas shows or hides archived specifications with their relations, and nothing else.
- **An outside change** to the file, by an agent, is shown without reopening, with the view where the person left it. It clears the undo history of the diagram (FBL 7.3), so that undo never puts back a file older than that change. An edit made in ADP is written against the file as it is on disk at that moment, or refused with a reason when what it changes is gone.

## Rules

- **Endpoints.** Each relation type joins one pair of types, and DISL's built-in `std.endpoints` refuses every other pair and reports one found in a file.
- **One specification per agent** is the shape of the file: an agent has one `specification` key. A gesture that would add a second is refused by `aad.second-specification`.
- **Information findings** for an incomplete picture, which never stop the file opening: `aad.agent-without-specification`, `aad.agent-without-location`, `aad.project-without-specification`, `aad.specification-without-project`, `aad.location-without-agent`, `aad.location-without-environment`.
- **Warnings:** `aad.link-not-openable` for a link that is neither an `http` or `https` address nor a path.
- **Errors** reported by DISL's built-ins and FBL: a key naming no element (`fbl.dangling-reference`), a key naming an element of the wrong type (`std.endpoints`), an id used twice. The file still opens, everything readable is drawn, and nothing is removed.

## Notation

- **Shapes:** a project is a folder, a specification a rounded rectangle, an agent a pill, a location a rectangle, an environment a flat hexagon. Each type also has its own fill in both themes.
- **A specification** shows its name, its status in words in the status colour, and its outline in the status colour, dashed when archived. Its tasks are listed beneath under one heading per status, each with its count.
- **A location** shows its branch, a line, then its folder, then its pull requests under one heading, collapsed by default.
- **Badges:** a link symbol at the top right where an element has a link (a location's is its branch's), and a lock at the top left where it is locked. A list item with a link ends in a link symbol.
- **Relations** are straight lines without a label or a marker.
- **Long names** are cut with an ellipsis and shown in full on hover; an element does not grow without limit.

## What DISL 0.4 and FBL 0.4 cannot state yet

- **The reference layout's numbers.** DISL 10.4 states the properties of a tiered `force` layout, not its constants; they are above, under *Layout*, by the design's decision L5.
- **`updated` on every edit.** DISL has no way to say that every edit of a row sets one of its attributes. The `.dis` sets `updated` on a new row (its default, `env.now`) and when its status is set (`setTaskStatus`); a host **must** also set it when it renames a row or changes its link (standalone Requirement 9.3).
- **The status label** is written as a CEL conditional over the status values, because CEL in DISL has no function giving an enum value's label.
- **The link of a location's folder** is drawn as the link symbol of a one-item compartment, because a label's `link` is not activatable as a badge's `onClick` is.

## Known gaps

- **No host implements it yet.** The standalone IDE's implementation is that specification's tasks 5 onwards.

## The examples

A host ships its own examples; the standalone IDE's are to be `adp-one-day` (this project's work on one day), `every-state` (each status, kind and link form, locked elements, a group off its default) and `two-projects` (two projects sharing an environment), with the instruction text above beside them. The example above is the one the fixtures read.

## Sources

- Specification: `standalone:.spec-workflow/specs/agent-activity-diagram/requirements.md`, `design.md` and `tasks.md` at `08d917f`.
- Languages: DISL 0.4 (`specifications/disl/`), FBL 0.4 (`specifications/fbl/`), and the research of this repository's spec 014 (`specs/014-agent-activity-diagram/research.md`).
- Notion: the "Tools" database row for Agent activity diagram.
- Conversation: Peter's rulings of 2026-10-09 that tasks and pull requests are ordered by when they were last updated, and that the person's part stays under `view`, with FBL 0.4 creating and removing it.
