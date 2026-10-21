# hello-world - example provenance

The simplest possible Helm chart, vendored as an ADP `helm/chart` example. It demonstrates
the bare anatomy the diagram draws: `Chart.yaml` metadata, one default `values.yaml`, a
`templates/` folder (deployment, service, service account, `_helpers.tpl` partial and
`NOTES.txt`) - no dependencies, no overrides, no lock.

* **Source**: https://github.com/helm/examples (`charts/hello-world`), commit `4888ba8fb8180dd0c36d1e84c1fcafc6efd81532`
* **Chart version**: 0.1.0 (appVersion 1.16.0), `apiVersion: v2`
* **Retrieved**: 2026-09-03
* **License**: Apache-2.0, re-verified from the cloned `LICENSE` text at acquisition; that
  file is copied beside this one. The upstream repository carries no `NOTICE` file.
* **Local additions/changes**: this `provenance.md`, the copied `LICENSE`, and the
  `helm-chart.adp` registration (added once the diagram type shipped). Text files are
  line-ending-normalized by the repository's checkout policy (`.gitattributes`); no
  upstream file's content was edited. This file is not named `readme.md` so it can never
  collide with a chart's own upstream `README.md`.
