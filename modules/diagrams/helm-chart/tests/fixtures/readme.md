# Fixtures

Three chart trees the module tests read from disk exactly as the backend will:

* `well-formed/` - a v2 chart exercising the whole inventory: two dependencies (one
  aliased with a condition, one plain), the redis one vendored unpacked, a values stack
  with an override layer, schema, lock, crds, every template role, and a foreign file the
  reader must ignore.
* `broken/` - Chart.yaml parses but has no version; values.yaml does not parse; a
  template's kind is templated and therefore unknown by design.
* `unconventional/` - apiVersion v1 (legacy: requirements.yaml/requirements.lock), a
  not-SemVer version, `type: library`, a sealed `.tgz`, and a vendored chart with deeper
  nesting that is summarized by count, never recursed.

`.gitattributes` here says `* -text`: these bytes are test subjects (the zero-writes proof
compares them against themselves), so git must never rewrite their line endings - the same
reasoning the root `.gitattributes` records for the sibling modules' fixtures.

## One chart per finding of helm-chart-disl-fbl known by reading

Eleven more charts, one for each finding of the `helm-chart-disl-fbl` specification that is known only by reading and concerns a folder's content. They exist so the parity transcript (`Parity/helm-chart.transcript.json`) shows each finding, or shows that it does not occur. Each was written to make its finding visible, never to make it go away.

| Folder | Finding | Exists to prove |
| --- | --- | --- |
| `includes-per-name/` | A4 | One template including three names, two from one partial: one relation per included name, to the first partial that defines it (`parity.fullname` is defined by `_helpers.tpl` and `_names.tpl`), with the name in the relation's id. A partial including another is a relation too. |
| `open-ends/` | A5 | An unvendored dependency (`common`) beside a vendored one, and an include no local partial defines (`common.labels.standard`) beside one that resolves: each open end is a relation with an empty target, its own id and rows. |
| `finding-shapes/` | A7 | Two unreadable files (`values.yaml`, `crds/broken.yaml`): one `helm.unreadable-yaml` each. Three dependencies under the effective name `cache`, one through an alias: one `helm.name-collision`, at the second. A lock with two entries nothing declares: one `helm.lock-drift` per entry. |
| `finding-order/` | A8 | Three unreadable files whose module order (`values.yaml`, `values-b.yaml`, `crds/x.yaml`) is not their ordinal order, findings of five codes, and both shapes of `helm.lock-drift`: the two without a line (declared, not locked) before the two with one, and the stale entries in the order of their names rather than their lines. |
| `same-named-dependencies/` | A10 | Two dependencies with one name and one constraint (`redis`, vendored), and two with one name and two constraints (`nats`, unvendored), both configured by `values.yaml`: what the nodes, the relations, their ids and their rows are today. |
| `unselected-files/` | B3 | A legacy chart whose reader takes what the binding's file rules do not select: `values-prod.yml`, `requirements.lock` and the files under `charts/sub/templates/`; and names matched ignoring case (`values-extra.YAML`, `templates/Notes.TXT`, `crds/thing.YML`). The `values*` prefix and `Chart.yaml` are written in lower case on purpose: the reader finds them through the file system's own matching, which ignores case on Windows and not on Linux, and a fixture spelled otherwise would read differently on the two. |
| `empty-directories/` | B4 | Directories as entries: `charts/nochart/` (no `Chart.yaml`, only a readme) is a Subchart, and the deeper count of `charts/holder` counts a directory, a file and an empty directory. **Git cannot store an empty directory**, so the transcript's generator makes three in its copy of this folder: `crds/` (a CRDs node of zero files), `charts/hollow/` (a Subchart with no file at all) and `charts/holder/charts/empty/`. The generator names them in `HelmTranscript.EmptyDirectories` and the transcript lists them as `emptyDirectoriesMade`. |
| `scalars-as-written/` | B7 | Scalars a YAML reader types otherwise: `version: 1.0` and `appVersion: 1.16` kept as written (and `1.0` reported as not SemVer), `deprecated: "true"` counted although it is a string, and conditions on `"true"` and `True` (on) and `yes` and `1` (not a boolean). `values-duplicate.yaml` repeats a key, which YamlDotNet refuses in its own words. |
| `template-lines/` | B8 | Two includes on one line and two defines on one line, an include inside a YAML comment, an include split over two lines (not found), an indented `kind:` (ignored), a file that states nothing and `templates/.helmignore` (both still nodes), a test hook by its path, and `lone-cr.yaml`, whose lines end in a lone CR: split on LF only, it is one line, and its kind is not found. |
| `not-a-chart/` | B10 | A registered folder with no `Chart.yaml` (it has a `Chart.yml`, values and a template): exactly one `helm.not-a-chart` warning at the folder, and no element. |
| `finding-lines/` | B11 | Where each finding points: `helm.version-not-semver` at the line of `version` (9), not the entry (3); `helm.empty-templates` at the entry; `helm.condition-missing` at the dependency's line; `helm.lock-drift` for a declared dependency at `Chart.lock` with no line, and for a stale entry at its line. |

The `-text` attribute of this folder's `.gitattributes` covers all of them; `lone-cr.yaml` depends on it, since git would otherwise be free to rewrite its line endings.
