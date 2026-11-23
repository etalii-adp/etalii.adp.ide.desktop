# Value stream map

The companion of the diagram type `generic/value-stream-map`: what it is for, the file a person or an agent writes, how the diagram computes its figures, the worked examples with their totals, and what the diagram does not do. The specification is [value-stream-map.dis](value-stream-map.dis), the binding [value-stream-map.fbl](value-stream-map.fbl), the file's JSON Schema [value-stream-map.schema.json](value-stream-map.schema.json) and the tool description [value-stream-map.atd](value-stream-map.atd). It was specified by [spec 084](../../specs/084-value-stream-map/value-stream-map.spec.md).

## What the diagram is for

A value stream is one path from a trigger, a demand or an event, to an outcome, value for whoever asked. Mapping how a stream actually runs, with the time its work waits and the places where it passes from one performer to the next, is how a service organisation finds where improvement pays.

The diagram draws a stream's steps from left to right in flow order, the trigger first and the outcome last, with each step's activity, practices, performer and times, the wait as a band under the step, and each hand-off as a marked flow. It computes the stream's lead time, process time, flow efficiency and hand-offs, marks the three longest waits, and keeps a future state beside the current one, with the difference between them.

The activity list and the practice list belong to the document. A new map starts from one of two lists, six activities of a value chain or eight of a lifecycle, and every name in them can be changed, so that a map does not age with any framework's version.

## The file (`.vsm`)

One YAML file. A complete map with one state:

```yaml
name: Onboard a new employee
unit: hours
activities:
  - name: engage
  - name: deliver and support
practices:
  - name: service request management
current:
  trigger: { id: t1, name: Hire confirmed }
  steps:
    - id: s1
      name: Request accounts
      activity: engage
      practices: [service request management]
      performer: hiring manager
      processTime: PT15M
      waitTime: PT0M
    - id: s2
      name: Create accounts
      activity: deliver and support
      practices: [service request management]
      performer: service desk
      processTime: PT30M
      waitTime: P1D
    - id: s3
      name: Hand over laptop
      activity: deliver and support
      practices: [service request management]
      performer: service desk
      processTime: PT20M
      waitTime: PT4H
  outcome: { id: o1, name: Employee can work }
  flows:
    - { from: t1, to: s1 }
    - { from: s1, to: s2 }
    - { from: s2, to: s3 }
    - { from: s3, to: o1 }
```

Its lead time is 1,745 minutes (29.08 h), its process time 65 minutes (1.08 h), its flow efficiency 3.7 % and it has one hand-off, from the hiring manager to the service desk.

## Instructions for agents

Write a map from this section alone; [value-stream-map.schema.json](value-stream-map.schema.json) checks its shape.

- **The keys.** At the root: `name` (required, what the stream is called), `unit` (`minutes`, `hours` or `days`; the unit totals are shown in, default `hours`), `activities` and `practices` (lists of `{ name }`, an activity with an optional `color: "#RRGGBB"`), `current` (required) and `future` (optional). A state holds `trigger` (`{ id, name }`), `steps` (a list), `outcome` (`{ id, name }`) and `flows` (a list). A step holds `id` and `name` (required), `activity` (a name from `activities`), `practices` (a list of names from `practices`), `performer` (a team, a role or an agent), `processTime`, `waitTime` and `note`.
- **Flows.** A flow is `{ from: <id>, to: <id> }` between two elements of the same state, from the trigger or a step to a step or the outcome. With `share: 0.8` it is an alternative, taking that part of its source's work; the alternatives leaving one step sum to 1. With `rework: 0.1` it is rework, sending that part of its source's work back to an earlier step. A flow never has both.
- **Ids.** Every trigger, step and outcome has an id, unique across the file: `t`, `s` or `o` followed by a number. A new element takes the next number of its kind (after `s6` comes `s7`, in either state). Flows and vocabulary entries have no id: a flow is named by its ends, an entry by its name. Two flows between the same two elements are one too many.
- **Durations.** `processTime` and `waitTime` are ISO 8601 durations of days, hours, minutes and seconds: `PT0M`, `PT20M`, `PT3H`, `P2D`, `P1DT4H`. Months and years are refused, and a day is 24 hours: calendars and working hours are not modelled. A missing time counts as zero and is reported.
- **The future state.** It has the shape of `current`. Each element copied from the current state carries `basedOn: <id of the current element>`; the diagram writes this when it creates the future state, and an agent writing one by hand writes it too. New elements of the future state have no `basedOn`.
- **What not to write.** Do not write `view:`. It holds the state the canvas shows, `view: { state: current }`, and only the diagram writes it; deleting it loses nothing but that choice. Do not write totals, hand-offs or marks: they are computed.
- **Names outside the lists.** An activity or practice not in the document's lists is kept as written and reported (`vsm.unknown-activity`, `vsm.unknown-practice`). Add the name to the list rather than removing it from the step.
- **Keys the diagram does not know** are kept as written, byte for byte, and a host lists them as the step's or the map's unknown keys; nothing written is ever dropped. A flow with both `share` and `rework` is read as no flow at all: it is kept as written, and the file schema refuses it.

## How the totals are computed

Every figure is a function of the specification ([value-stream-map.dis](value-stream-map.dis), `functions`), evaluated by the host; no host computes them in code of its own (research R4).

- **Visits.** The trigger is visited once. A step's visits are the sum, over the forward flows into it, of the source's visits times the flow's share (1 for a plain flow). A rework flow from step *b* back to step *a* with rate *r* repeats every step on the forward path from *a* to *b*, both included, so their visits are multiplied by 1 / (1 − *r*). A step no forward path reaches from the trigger has zero visits and is left out of the totals.
- **Lead time** is the sum over the state's steps of visits × (process time + wait time).
- **Process time** is the sum over the state's steps of visits × process time.
- **Flow efficiency** is process time divided by lead time, shown as a percentage with one decimal, and a dash when the lead time is zero.
- **Hand-offs.** A hand-off is a forward flow between two steps whose performers differ; it is drawn with a marker and never stored. Their number is the sum over hand-off flows of how often each is taken: its source's visits times its share, times the factor of a rework loop it lies inside, so that rework and alternatives make it fractional.
- **The longest waits.** A step is marked when fewer than three steps of its state wait longer, so ties mark more than three.
- **The unit.** Totals are computed in minutes and shown in the document's `unit`, with at most two decimals; a day is 1,440 minutes.
- **When nothing is computed.** A state's totals are not computed while it has no step, more than 64 steps, no trigger or more than one, or a cycle of forward flows; the summary says so and a finding names the cause. Sixty-four is the deepest recursion a tool description admits (ATD7004), and value streams mapped by hand stay far below it (research R10).
- **The comparison.** While both states are computed, each figure is shown for the current state, the future state, and the difference (future minus current).

## Worked examples

Both examples are invented. Their totals were worked by hand from the rules above (spec 084, SC-002).

### Restore service

[examples/restore-service.vsm](examples/restore-service.vsm): an alert is triaged by operations, logged and diagnosed by the service desk, fixed and verified by the platform team, and closed by the desk. Six steps in a line, no alternative and no rework, so every step is visited once.

| Step | Performer | Process | Wait |
|---|---|---|---|
| s1 Triage alert | operations | 10 min | 0 min |
| s2 Log incident | service desk | 5 min | 15 min |
| s3 Diagnose | service desk | 20 min | 60 min |
| s4 Fix configuration | platform team | 30 min | 120 min |
| s5 Verify fix | platform team | 15 min | 30 min |
| s6 Close incident | service desk | 5 min | 45 min |

Current state: lead time 85 + 270 = 355 min (5.92 h), process time 85 min (1.42 h), flow efficiency 85 / 355 = 23.9 %. Hand-offs: s1 → s2 (operations to service desk), s3 → s4 (to the platform team), s5 → s6 (back to the desk): 3. Longest waits marked: s4, s3, s6.

The future state lets the platform team close the incident itself (s12), which removes a hand-off, and shortens the waits before diagnosis (s9, 20 min) and the fix (s10, 30 min). Lead time 85 + 140 = 225 min (3.75 h), process time 85 min, flow efficiency 37.8 %, hand-offs 2 (s7 → s8, s9 → s10). Longest waits marked: s12 (45 min), s10 (30 min), s11 (30 min).

| Figure | Current | Future | Difference |
|---|---|---|---|
| Lead time | 5.92 h | 3.75 h | −2.17 h |
| Process time | 1.42 h | 1.42 h | 0 h |
| Flow efficiency | 23.9 % | 37.8 % | |
| Hand-offs | 3 | 2 | −1 |

### Request fulfilment

[examples/request-fulfilment.vsm](examples/request-fulfilment.vsm): a request is logged and approved; 80 % are fulfilled from the catalogue and 20 % are built, and a tenth of the built ones fail their check and are built again. It has one alternative split 80/20 and one rework loop at 10 %.

| Step | Performer | Process | Wait | Visits |
|---|---|---|---|---|
| s1 Log request | service desk | 5 min | 10 min | 1 |
| s2 Approve | approver | 5 min | 240 min | 1 |
| s3 Fulfil from catalogue | service desk | 15 min | 30 min | 0.8 |
| s4 Build custom | platform team | 120 min | 1,440 min | 0.2 / 0.9 = 0.2222 |
| s5 Check result | platform team | 20 min | 60 min | 0.2222 |
| s6 Deliver | service desk | 10 min | 20 min | 0.8 + 0.2 = 1 |

s4 and s5 lie on the rework loop, so their visits are the 0.2 the alternative brings times 1 / (1 − 0.1). The flow s5 → s6 leaves the loop and is taken 0.2 times, so s6 is visited 0.8 + 0.2 = 1 time.

Current state: lead time 15 + 245 + 36 + 346.67 + 17.78 + 30 = 690.44 min (11.51 h), process time 5 + 5 + 12 + 26.67 + 4.44 + 10 = 63.11 min (1.05 h), flow efficiency 9.1 %. Hand-offs: s1 → s2 (1), s2 → s3 (0.8), s2 → s4 (0.2) and s5 → s6 (0.2): 2.2. Longest waits marked: s4 (1 day), s2 (4 h), s5 (1 h).

The future state shortens the approval's wait to 30 min (s8) and the build's to 4 h (s10). Lead time 213.78 min (3.56 h), process time 63.11 min, flow efficiency 29.5 %, hand-offs 2.2. Longest waits marked: s10 (4 h), s11 (1 h), and s8 and s9 (30 min each, tied).

| Figure | Current | Future | Difference |
|---|---|---|---|
| Lead time | 11.51 h | 3.56 h | −7.94 h |
| Process time | 1.05 h | 1.05 h | 0 h |
| Flow efficiency | 9.1 % | 29.5 % | |
| Hand-offs | 2.2 | 2.2 | 0 |

### Walk-throughs of findings

- **A step outside the flow.** Add `- { id: s13, name: Order spare part, performer: platform team, processTime: PT10M, waitTime: PT10M }` to the current state of the restore-service example with no flow to it. It is drawn in column 0, its visits are zero, the totals and the marked waits stay as above, and `vsm.unconnected-step` says: *No flow reaches "Order spare part" from the trigger; it is left out of the totals.*
- **Shares that do not add up.** Change the request-fulfilment alternatives to `share: 0.8` and `share: 0.3`. The schema accepts the file, since each share lies between 0 and 1; `vsm.shares` stands at s2: *The alternatives leaving "Approve" take 110 % of its work, not 100 %.* The totals are computed with the shares as written.

## Findings

A map that breaks a rule still opens and draws: every breach is a finding, never a refusal, so nothing anyone wrote is lost (FR-007).

| Code | Severity | Says |
|---|---|---|
| `vsm.empty-stream` | info | The state has no step yet. |
| `vsm.missing-trigger` | warning | The state has no trigger; its totals are not computed. |
| `vsm.missing-outcome` | warning | The state has no outcome. |
| `vsm.second-trigger` | warning | A second trigger of one state; a stream starts once. |
| `vsm.second-outcome` | warning | A second outcome of one state; a stream ends once. |
| `vsm.unconnected-step` | warning | No flow reaches the step from the trigger; it is left out of the totals. |
| `vsm.missing-wait` | info | The step has no wait time; it counts as zero. |
| `vsm.missing-process-time` | info | The step has no process time; it counts as zero. |
| `vsm.shares` | warning | The alternatives leaving a step do not take 100 % of its work. |
| `vsm.unmarked-cycle` | error | A cycle of forward flows; mark the flow that goes back as rework. Until then the state's totals are not computed. |
| `vsm.rework-forward` | warning | Rework goes to a step that does not come before its source. |
| `vsm.too-many-steps` | warning | A state has more than 64 steps; its totals are not computed. |
| `vsm.unknown-activity` | info | The activity is not one of the stream's activities; it is kept as written. |
| `vsm.unknown-practice` | info | A practice is not one of the stream's practices; it is kept as written. |

Two gestures are refused instead, because the file could not hold them: a flow between the two states (`vsm.cross-state`), and a second flow between the same two elements.

## The starting lists

A new map is created empty, with a trigger and an outcome, and the toolbox offers two starting lists (the specification's `toolbox.templates`): the *Six-activity value chain* (plan, improve, engage, design and transition, obtain/build, deliver and support) and the *Eight-activity lifecycle* (discover, design, acquire, build, transition, operate, deliver, support), each with a short practice list (incident management, change enablement, service request management, monitoring and event management, problem management). They are short labels a user can change, rename or remove; no text of any framework's publications is part of this definition (FR-011).

## What this diagram does not do

- It is not a process engine: nothing is executed, routed or assigned.
- It is not a business process notation: a step has no gateways, events or message flows, and branching is limited to alternatives with shares and rework with rates.
- It is not a measurement system: times are entered or estimated by people or written by agents, never collected from a ticketing system.
- It keeps no calendars or working hours: a day is 24 hours, and a duration in months or years is refused.
- It holds at most two states, current and future, in one document.

## What stays with the host

- **The undo history.** Every edit is one command, and Create future state, with everything it copies, is one step.
- **How the band and the hand-off are drawn.** The notation states the band's text, colour and place under a step, and the dashed style of a hand-off flow; how a host renders them is its own.
- **An agent's change to an open map.** The binding reads the file again with byte-preserving splices, so a change written by an agent is shown, with its totals recomputed, without reopening the map.

## Notes

- The first host to build this diagram, and its own specification with conformance cases, are chosen after this definition merges. SC-003 (an agent writes a valid map from this page alone) and SC-004 (a practitioner maps six steps in ten minutes) are measured there.
- The step budget was lowered from the 200 of research R10 to 64 on 2026-11-23, so that the tool description states the same recursion bound as the specification.
