# prometheus - example provenance

The prometheus community chart, vendored as an ADP `helm/chart` example. It demonstrates
the dependency side of the diagram: four `dependencies[]` entries (alertmanager,
kube-state-metrics, prometheus-node-exporter, prometheus-pushgateway), every one carrying
a `condition:` switched in the default `values.yaml`, and none vendored under `charts/` -
so all four draw as **Unvendored** open ends, which is deliberately not a validation
finding. It also ships a real `Chart.lock` (pinned versions beside declared constraints)
and a `values.schema.json` (the schema node).

* **Source**: https://github.com/prometheus-community/helm-charts (`charts/prometheus`), commit `7c44761d0dbdc57c839c4586da9517a0060b1bea`
* **Chart version**: 29.27.0 (appVersion v3.14.0), `apiVersion: v2`
* **Retrieved**: 2026-09-03
* **License**: Apache-2.0, re-verified from the cloned `LICENSE` text at acquisition; that
  file is copied beside this one. The upstream repository carries no `NOTICE` file.
* **Local additions/changes**: this `provenance.md`, the copied `LICENSE`, and the
  `helm-chart.adp` registration (added once the diagram type shipped). The upstream `ci/`
  folder of test values variants is kept as-is - the diagram ignores foreign content
  without complaint, which is itself worth demonstrating. Text files are
  line-ending-normalized by the repository's checkout policy; no upstream file's content
  was edited. This file is not named `readme.md` so it can never collide with the chart's
  own upstream `README.md`.
