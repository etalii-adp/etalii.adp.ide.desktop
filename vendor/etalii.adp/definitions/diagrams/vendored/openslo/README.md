# OpenSLO documents

Real OpenSLO documents, read by the SLO map's fixtures under `specifications/fbl/fixtures/openslo-slo-read/` (spec 085, T012). They are kept byte for byte (`.gitattributes` marks this folder `-text`) and are never edited here.

| | |
|---|---|
| Repository | [OpenSLO/OpenSLO](https://github.com/OpenSLO/OpenSLO) |
| Commit | `e0259752f252276485cfa2708dac2db00c2f3464` |
| Licence | Apache License 2.0, read from the repository's own `LICENSE` at that commit and copied verbatim as [LICENSE.md](LICENSE.md); the repository has no `NOTICE` file |

| File here | Source at that commit |
|---|---|
| `examples/budgeting-method/occurences-slo.yaml` | the same path, byte for byte |
| `examples/budgeting-method/ratio-timeslices.yaml` | the same path, byte for byte |
| `examples/budgeting-method/timeslices-slo.yaml` | the same path, byte for byte |
| `examples/treat-low-traffic-as-equally-important/timeslices-slo.yaml` | the same path, byte for byte |
| `specification-samples.yaml` | the five concrete samples of `website/docs/specification.md`, its 8th, 14th, 16th, 18th and 19th yaml blocks (an SLO, an AlertPolicy, an AlertCondition and two AlertNotificationTargets), each block's text unchanged, joined with `---` lines in the order they appear there |

## What this corpus does not show

- **No Service, SLI or DataSource as a document of its own.** Every SLI is inline and every data source a `type` or a `metricSourceRef` naming nothing in the file; the folder fixture `openslo-slo-folder` and the findings inputs, written for spec 085, cover them.
- **No file of several objects as OpenSLO publishes it.** OpenSLO's examples hold one SLO each, and the specification's samples stand in separate blocks; the stream here is assembled from them.
- **No `targetPercent`, `compositeWeight`, composite SLO, `bad` metric or `thresholdMetric` with a `metricSourceRef`.**
- **No alert policy referenced from an SLO**: the SLO samples' `alertPolicies` is empty or absent, so every SLO here reports `openslo.no-alerting`.
- **No calendar unit** (`M`, `Q`, `Y`), so the *at least* of a calendar window is not exercised; and `timeslices-slo.yaml` writes `duration: 1m` with a comment saying a month, which reads as one minute.
- **No other OpenSLO version.** The `openslo.com/v2alpha` input of `findings-version` is written for spec 085.
