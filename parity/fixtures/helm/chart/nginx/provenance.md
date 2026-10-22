# nginx - example provenance

Bitnami's NGINX chart, vendored as an ADP `helm/chart` example. It demonstrates three
things at once: an OCI-repository dependency (`oci://registry-1.docker.io/bitnamicharts`),
that dependency **vendored** - the `common` library chart sits unpacked under
`charts/common/`, so the declares edge resolves (with `Chart.lock`'s pin beside the
constraint) and the library-chart handling shows; and the values layering - the
ADP-authored `values-dev.yaml` and `values-prod.yaml` stack on the default `values.yaml`.

* **Source**: https://github.com/bitnami/charts (`bitnami/nginx` and `bitnami/common`), commit `8f8032ba37888cdeb20b35a2136fb1e8b5557e97`
* **Chart versions**: nginx 22.1.1 (appVersion 1.29.1); common 2.31.4 (`type: library`, at tag `common/2.31.4`, commit `c6bc59845497f84f740e47075f8af840f150536e` - the exact version the chart's `Chart.lock` pins), both `apiVersion: v2`
* **Retrieved**: 2026-09-03
* **License**: Apache-2.0 (`SPDX-License-Identifier: APACHE-2.0`, Broadcom), re-verified
  from the cloned `LICENSE.md` at acquisition and matching each chart's
  `annotations.licenses: Apache-2.0`; that file is copied beside this one and again inside
  `charts/common/`. The upstream repository carries no `NOTICE` file.
* **Local additions/changes**: this `provenance.md`; the copied `LICENSE.md` files;
  **`values-dev.yaml` and `values-prod.yaml`, which are ADP-authored** to demonstrate the
  override stack and are not upstream content; the unpacking of `common` into
  `charts/common/` (upstream declares it, ADP vendored it from the same source and
  commit); and the `helm-chart.adp` registration (added once the diagram type shipped).
  Text files are line-ending-normalized by the repository's checkout policy; no upstream
  file's content was edited. This file is not named `readme.md` so it can never collide
  with the chart's own upstream `README.md`.
