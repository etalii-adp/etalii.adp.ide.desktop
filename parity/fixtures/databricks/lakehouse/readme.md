# Lakehouse — example

One coherent Databricks Asset Bundle, drawn three ways:

- **`databricks.adp`** opens `databricks.yml` as the **bundle** diagram: the bundle, its
  declared pipeline resource, and the `dev`/`prod` target frames - with `prod`'s override of
  the pipeline drawn as a reference edge (the job override stays invisible here, because the
  job lives in an included file the bundle diagram does not follow).
- **`databricks.pipeline.adp`** opens the *same* file as the **pipeline** diagram, picked out
  by its `resource:` header: three libraries flowing into *Bronze to gold* and on to
  `${var.catalog}.gold`, with the serverless badge and the notification satellite.
- **`resources/nightly-ingest.adp`** opens the job resource as the **job** diagram: five tasks
  across four types, a condition task whose outcomes branch (`publish` on `"true"`,
  `alert_on_empty` on `"false"` with the non-default `run_if: AT_LEAST_ONE_FAILED`), a cluster
  binding, and a schedule.

What it demonstrates, per databricks-diagrams Requirement 13: registrations that open from the
explorer with no setup; the `resource:` header selecting one declaration from a multi-resource
file; reference edges (the target override); outcome edges and `run_if`; and configs plausible
enough to deploy - variables, a quartz schedule, real task shapes.

Positions come from the computed layouts until someone drags a box; a drag lands in the
`.adp`'s `layout:` block, never in these files - which is the point of the layout-in-`.adp`
rule this family exercises.
