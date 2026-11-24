# SLO map

The companion of the diagram type `openslo/slo`: what it is for, which OpenSLO objects it draws and how it reads them, how it computes an objective's error budget, the worked budgets of its fixtures, what it reports, and what it does not do. The specification is [openslo-slo.dis](openslo-slo.dis), the binding [openslo-slo.fbl](openslo-slo.fbl) and the tool description [openslo-slo.atd](openslo-slo.atd). It was specified by [spec 085](../../specs/085-openslo-slo-map/openslo-slo-map.spec.md).

## What the diagram is for

A service level objective is a commitment: this share of what a service does, over this window, goes well. [OpenSLO](https://openslo.com) writes such commitments as YAML: a Service, the SLOs on it, the indicators (SLIs) they measure, the data sources those read, and the alert policies that tell someone when an error budget burns. The files are text, kept beside the code, and nothing in them shows how the objects hang together or what a target of 99.95 % actually allows.

The SLO map draws an OpenSLO file, or a folder of them, as the service with its SLOs around it, each SLO's objectives and indicator beside it, the data sources outermost and the alerting on the ring of the SLOs. Each objective states its target as a percentage, its window and its error budget in the budgeting method's own terms and as a time of complete failure. A reference that names nothing in the reading is drawn to a dashed placeholder, so a gap in the commitments shows. Every edit writes the OpenSLO file with byte-preserving splices: a changed target replaces that number and nothing else, and the file's comments stay.

## What it reads

OpenSLO `openslo/v1`, every document of a YAML stream (FBL 0.22): a file of `.yaml` or `.yml` whose first document has `apiVersion: openslo/v1`. Other YAML files are left to the tools that claim them.

| Object | Drawn as | Its name | What the diagram reads |
|---|---|---|---|
| `Service` | Service | `metadata.name` | `displayName`, `description` |
| `SLO` | SLO | `metadata.name` | `spec.service`, `spec.indicatorRef` or `spec.indicator`, the first `spec.timeWindow` item (`duration`, `isRolling`, `calendar.startTime`, `calendar.timeZone`), `spec.budgetingMethod`, `spec.objectives`, `spec.alertPolicies` |
| an entry of `spec.objectives` | Objective, inside its SLO | its position, from 1 | `displayName`, `op`, `value`, `target`, `targetPercent`, `timeSliceTarget`, `timeSliceWindow`, `indicatorRef`, `compositeWeight` |
| `SLI` | SLI | `metadata.name` | `thresholdMetric` or `ratioMetric` (`good`, `bad`, `total`), each `metricSource`'s `metricSourceRef` or `type` |
| `DataSource` | DataSource | `metadata.name` | `spec.type` |
| `AlertPolicy` | AlertPolicy | `metadata.name` | `alertWhenNoData`, `alertWhenResolved`, `alertWhenBreaching`, `conditions`, `notificationTargets` |
| `AlertCondition` | AlertCondition | `metadata.name` | `severity`, `condition` (`kind`, `op`, `threshold`, `lookbackWindow`, `alertAfter`) |
| `AlertNotificationTarget` | AlertNotificationTarget | `metadata.name` | `spec.target` |
| any other document | listed, not drawn | the line it starts on | its `kind` and `apiVersion`, kept exactly as written |

### References and inline objects

OpenSLO names one object from another in two ways, and the diagram draws both.

- **By reference**: `service: checkout`, `indicatorRef: checkout-errors`, `alertPolicyRef`, `conditionRef`, `targetRef`, `metricSourceRef`. The diagram joins the two objects when the reading holds one of that kind with that name, and otherwise joins the referring object to a dashed placeholder, *Missing*, labelled with the kind and the name.
- **Inline**: `spec.indicator` in an SLO, an alert policy with `kind: AlertPolicy` in `spec.alertPolicies`, a condition or a notification target with its `kind` inside a policy. The inline object is an element of its own, contained in its owner and marked *inline*; editing it edits the owner's document.

An OpenSLO name is unique within its kind, not across kinds, so an element's id is its kind and its name, `SLI:checkout-errors`; an inline object's id adds its owner's, `SLO:checkout-availability/SLI:checkout-errors`, and an objective's is its SLO's with its position, `SLO:checkout-availability/objective:1`.

### A folder of files

A registration whose body is a folder (`openslo-slo.fbl#folder`) reads every `.yaml` and `.yml` file under it, file by file, so that an SLO in one file and the service it names in another are drawn joined. The folder is read-only: a file is changed by opening it on its own.

## The file, for agents

An agent that writes OpenSLO for this diagram writes one object per document, separated by `---`, in any order:

```yaml
apiVersion: openslo/v1
kind: Service
metadata:
  name: checkout
  displayName: Checkout
---
apiVersion: openslo/v1
kind: SLI
metadata:
  name: checkout-errors
spec:
  ratioMetric:
    good:
      metricSource:
        type: Prometheus
        spec:
          query: sum(rate(http_requests_total{code!~"5.."}[5m]))
    total:
      metricSource:
        type: Prometheus
        spec:
          query: sum(rate(http_requests_total[5m]))
---
apiVersion: openslo/v1
kind: SLO
metadata:
  name: checkout-availability
spec:
  service: checkout
  indicatorRef: checkout-errors
  timeWindow:
    - duration: 28d
      isRolling: true
  budgetingMethod: Occurrences
  objectives:
    - displayName: Available
      target: 0.999
```

- **Names** are RFC 1123 labels: at most 63 lower-case letters, digits and hyphens, starting and ending with a letter or a digit. A reference names an object by exactly that name.
- **Reference or inline**: write a reusable object (an SLI two SLOs share, a policy several SLOs use) as a document of its own and refer to it; write an object only one owner uses inline if you prefer. An SLO has `indicator` or `indicatorRef`, never both.
- **A target** is `target`, a ratio in [0, 1), or `targetPercent`, a percentage in [0, 100), never both. A threshold metric's objective also has `op` (`lt`, `lte`, `gt`, `gte`) and `value`.
- **Durations** are OpenSLO's shorthand, a whole number and one unit: `s` seconds, `m` minutes, `h` hours, `d` days, `w` weeks, `M` months, `Q` quarters, `Y` years. `1m` is one minute, not one month. A bare number in `timeSliceWindow` is minutes.
- **Budgeting methods**: `Occurrences` counts good events against all events; `Timeslices` cuts the window into slices of `timeSliceWindow` and counts a slice bad when its good ratio is below `timeSliceTarget`, so a Timeslices objective needs both; `RatioTimeslices` averages the slices' ratios, and needs `timeSliceWindow`.
- **One time window**: OpenSLO allows one item in `timeWindow`. `isRolling: false` needs `calendar` with `startTime` and `timeZone`.
- **Never write** a key OpenSLO does not define for the diagram's sake: positions a user pins go to the `.adp` registration's layout block, not into the OpenSLO file.
- **Check a written file** with OpenSLO's own validator, `oslo validate -f <file>` ([OpenSLO/oslo](https://github.com/OpenSLO/oslo)): the diagram's findings say what it can draw and compute, and `oslo` says whether the file is valid OpenSLO.

## How the error budget is computed

For each objective, with its SLO's window:

1. **The target as a share**: `target`, or `targetPercent` ÷ 100. Shown as a percentage.
2. **The window in seconds**: the duration's number times its unit: a minute 60, an hour 3,600, a day 86,400, a week 604,800. A calendar unit has no fixed length, so the diagram takes its shortest: a month 28 days, a quarter 89, a year 365, and says *at least*.
3. **The time of complete failure the budget allows**: (1 − share) × window, shown in days, hours, minutes and seconds, rounded down to the second.
4. **The budget in the method's terms**: for `Occurrences`, the share of events that may fail, (1 − share) × 100 %; for `Timeslices`, the slices that may be bad, the time of step 3 ÷ the slice size, rounded down; for `RatioTimeslices`, the average good ratio the slices may fall to, the target itself.

An objective with no target, or an SLO with no window or one the shorthand does not read, has no budget: the objective says *Error budget not computed: see the findings*.

## Worked budgets

Every objective of the fixtures, worked by hand from the steps above. The diagram's functions `budgetTime`, `budgetSlices` and `budgetText` compute the same.

| Fixture | SLO | Objective | Window | Target | Method | Time of failure allowed | In the method's terms |
|---|---|---|---|---|---|---|---|
| `openslo-slo-read/occurrences` | search-slo | Good experience | 2 weeks rolling | 99 % | Occurrences | 0.01 × 1,209,600 s = 12,096 s = 3 h 21 min 36 s | 1 % of events may fail |
| `openslo-slo-read/ratio-timeslices` | main-page | Good | 2 weeks rolling | 99 % | RatioTimeslices | 12,096 s = 3 h 21 min 36 s | the slices' average good ratio may fall to 99 % |
| `openslo-slo-read/timeslices` | service-availability | Internal objective | 1 minute calendar-aligned | 99.5 % | Timeslices | 0.005 × 60 s = 0.3 s, shown 0 s | 0.3 s ÷ 60 s: 0 bad slices of 1 minute |
| `openslo-slo-read/timeslices` | service-availability | SLA | 1 minute calendar-aligned | 99 % | Timeslices | 0.01 × 60 s = 0.6 s, shown 0 s | 0 bad slices of 1 minute |
| `openslo-slo-read/low-traffic` | service-availability | Objective | 1 week calendar-aligned | 99.95 % | Timeslices | 0.0005 × 604,800 s = 302.4 s = 5 min 2 s | 302.4 s ÷ 60 s: 5 bad slices of 1 minute |
| `openslo-slo-read/specification-samples` | foo-slo | Foo Total Errors | 4 weeks rolling | 98 % | Occurrences | 0.02 × 2,419,200 s = 48,384 s = 13 h 26 min 24 s | 2 % of events may fail |
| `openslo-slo-folder` | search-slo | Fast searches | 28 days rolling | 99 % | Occurrences | 0.01 × 2,419,200 s = 24,192 s = 6 h 43 min 12 s | 1 % of events may fail |
| `openslo-slo-folder` | search-slo | Tolerable searches | 28 days rolling | 99.9 % | Occurrences | 0.001 × 2,419,200 s = 2,419.2 s = 40 min 19 s | 0.1 % of events may fail |
| `openslo-slo-edits`, as read | checkout-availability | Available | 28 days rolling | 99 % | Occurrences | 24,192 s = 6 h 43 min 12 s | 1 % of events may fail |
| `openslo-slo-edits`, after its edits | checkout-availability | Available | 7 days rolling | 99.5 % | Occurrences | 0.005 × 604,800 s = 3,024 s = 50 min 24 s | 0.5 % of events may fail |
| `openslo-slo-edits`, after its edits | checkout-availability | Most requests | 7 days rolling | 95 % | Occurrences | 0.05 × 604,800 s = 30,240 s = 8 h 24 min | 5 % of events may fail |
| `openslo-slo-read/findings-clean` | checkout-availability | Available | 28 days rolling | 99.9 % | Occurrences | 2,419.2 s = 40 min 19 s | 0.1 % of events may fail |

OpenSLO's `examples/budgeting-method/timeslices-slo.yaml` writes `duration: 1m` with the comment *Contracts often are for 1 month time window*. OpenSLO's shorthand reads `1m` as one minute, and so does the diagram, which is why that SLA allows no time at all; a month is `1M`.

## Findings

Findings never stop a save.

| Code | Severity | When |
|---|---|---|
| `openslo.missing-reference` | info | A reference names no object of its kind in the reading; the object is joined to a placeholder. |
| `openslo.no-alerting` | info | An SLO names no alert policy and holds none inline: nobody is told when its budget burns. |
| `openslo.target-range` | error | `target` outside [0, 1) or `targetPercent` outside [0, 100). |
| `openslo.target-twice` | error | Both `target` and `targetPercent`, or neither. |
| `openslo.indicator-twice` | error | Both `indicator` and `indicatorRef`, or neither where one is required. |
| `openslo.window-count` | warning | `timeWindow` with more than one item; the diagram reads the first. |
| `openslo.timeslice-fields` | error | A Timeslices objective without `timeSliceTarget` or `timeSliceWindow`. |
| `openslo.conflicting-windows` | warning | Two SLOs on one SLI whose windows differ. |
| `openslo.unknown-kind` | info | A document the diagram does not draw: another kind, or not OpenSLO. |
| `openslo.version` | warning | An OpenSLO document of another version, such as `openslo.com/v2alpha`: not drawn, kept as written. |
| `openslo.name` | error | A name that is not an RFC 1123 label. |

The fixtures `openslo-slo-read/findings-*` hold one input per finding, each clean apart from that finding, with the element and the line it stands at.

## Layout

The force layout in tiers: the service in the middle, its SLOs and the alert policies on the next ring with the placeholders, then objectives, SLIs, conditions and notification targets, the data sources and the documents not drawn outermost. Relations pull, every element pushes the others apart, and a position a user pins is kept in the registration's layout block, never in the OpenSLO file.

## What this diagram does not do

- It does not query a data source or show how much of a budget has burned: it draws the commitments, not their measurement.
- It reads OpenSLO `v1` only. A document of another version is listed and kept as written, never edited.
- It does not check a file against OpenSLO's schema; `oslo validate` does.

## Gaps

What the definition cannot yet say, recorded rather than worked around:

- **A folder reads one document per file.** A folder subject's file rules have no `stream`, so a file of several documents contributes its first one when read through the folder binding. Opened on its own, the file is read whole.
- **Another version is read-only per document, not per file.** DISL and FBL cannot make a whole reading read-only on a condition, so a v1 file holding one v2alpha document stays editable, and only that document is kept unedited.
- **An inline object cannot see its document's version.** An alert condition inside an inline alert policy inside an SLO of another version is read; such a file is reported by `openslo.version` all the same.
- **`openslo.missing-reference` is always info.** The plan asked for an error when the reading is a folder; DISL cannot tell a constraint which kind of body it reads.
- **`metadata.labels` and `annotations` are not bound.** They are kept as written.
- **Relations are derived in the specification.** Each reference is a DISL derived relation computed from names, rather than an FBL relation rule, because a reference may name nothing and is then drawn to a placeholder the binding cannot hold.
- **The definition's findings are not replayed by the repository's validator.** It checks what a fixture reads and the bytes an edit writes; the findings each `findings-*` fixture states are for a host to replay.

## What stays with the host

- **The undo history.** Every edit is one command; adding an SLO with its first objective is one step.
- **How a placeholder and a reference are drawn.** The notation states the dashed placeholder and the style of an edge to it; how a host renders them is its own.
- **An agent's change to an open file.** The binding reads the file again with byte-preserving splices, so a document an agent adds is drawn without reopening the map.

## Notes

- The read fixtures use OpenSLO's own documents, vendored with their licence in [vendored/openslo](vendored/openslo/README.md).
- T030 of spec 085 (2026-11-23): following *The file, for agents* alone, a Service `checkout` and an SLO `checkout-latency` with an inline SLI and one Timeslices objective (`target`, `timeSliceTarget`, `timeSliceWindow: 5m`) were written. `oslo validate -f` (OpenSLO/oslo, built from its latest source that day) answered `Valid!` for that file and for the example above. Read through `openslo-slo.fbl#openslo`, it gave the four elements expected (the Service, the SLO, its SLI and its objective, each contained where they belong), and a fixture stating the SLI under the Service failed as it should. The file was written by the session that built the diagram, because a fresh agent could not be started, so it shows that the section is enough to write a valid file, not that an agent with no other context finds it so.
