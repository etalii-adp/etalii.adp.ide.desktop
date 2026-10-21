# The databricks round-trip corpus

Documents whose bytes are the test subject: the parsers read them, the round-trip tests prove an
untouched document saves byte-identically, and the writers' smallest-diff tests splice them.

- `bundle.yml` - a `databricks.yml`: bundle name, includes, variables, two modelled resource
  kinds plus an unmodelled one (`experiments`), an unmodelled root key (`sync`), and two targets
  of which `prod` overrides a job.
- `job.yml` - a job resource file: five tasks across four types, a condition task with outcome
  edges, a non-default `run_if`, a job cluster and a schedule.
- `pipeline.json` - pipeline settings JSON, read through the same YAML door (JSON is a subset of
  YAML 1.2): catalog and schema, three library kinds, notifications, and an unmodelled key
  (`edition`).
- `crlf-line-endings.yml` / `lf-line-endings.yml` - the same small bundle under each convention;
  they exist to prove the writer preserves whichever the file has.
- `tied-line-endings.yml` - as many LF endings as CRLF, so neither is the majority and the tie
  rule alone decides which ending a newly inserted line takes: CRLF, core's rule
  (backend-centralization Requirement 1.2).
- `no-trailing-newline.yml` - the same bundle without a final newline, which an append must not
  silently add.
- `broken.yml` - not YAML; carried as an error, opened as unavailable, refused on save.

These files are `-text` in the repository root's `.gitattributes` - by a glob naming this
folder's `.yml`/`.yaml`/`.json` specifically, because the extensions are shared repository-wide
and cannot take the by-extension rule the other modules use. This readme is deliberately outside
that glob: it is ordinary text, and the house line-ending style should simply apply.
