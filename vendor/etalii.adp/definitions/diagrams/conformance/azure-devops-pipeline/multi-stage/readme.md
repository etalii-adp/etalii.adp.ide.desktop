# Azure Pipelines test fixtures

`.yml` documents the pipeline module's document, parser, graph, layout and rules are tested
against.

`.gitattributes` marks this folder `-text`, so these bytes survive checkout on every platform.
Requirement 3.1 says a pipeline ADP did not change comes back byte-identical, and the tests
check that by comparing written bytes against the file on disk — with `core.autocrlf` on, git
would otherwise rewrite the line endings and the test would be measuring git rather than the
writer. The rule is scoped to this folder rather than to `*.yml`, because `.yml` is far too
common an extension to claim repository-wide — the same reasoning that makes it a *shared*
extension in Requirement 2.1. **Replace a fixture only by committing what an authoring tool
actually wrote.**

## Provenance

**None of these is a canonical Microsoft sample, and none of them claims to be.** Microsoft
publishes its YAML in documentation prose rather than as downloadable pipeline files, so there
is no equivalent of Structurizr's Big Bank workspace to fetch. Each file below was therefore
**written for this repository**, from the schema and the worked examples in Microsoft's own
documentation, and each says which construct it exists to prove. Where a file reproduces a
documented example, the page it came from is named.

That is a weaker claim than "canonical", and this readme has to keep saying so: a hand-written
file quietly relabelled canonical is exactly what Requirement 3's corpus must not contain,
because it would make the round-trip test a test of ADP's own idea of YAML.

### Realistic pipelines

| File | What it exists to prove |
|---|---|
| `multi-stage.yml` | The shape the dependency graph exists to make readable: five stages with fan-out (two deployments from one `Test`) and fan-in (`Notify` on both), a deployment job with an `environment` and a `runOnce` strategy, `trigger: manual`, `condition` with `and(succeeded(), …)` and `always()`, `continueOnError`, `timeoutInMinutes`, and two jobs with no `dependsOn` so they run in parallel — the job default, which is the opposite of the stage default. |
| `jobs-only.yml` | No `stages` key: the schema's implicit single stage (Requirement 4.2). |
| `steps-only.yml` | Neither `stages` nor `jobs`: one implicit stage containing one implicit job. |
| `templates.yml` + `templates/` | Templates at all three levels — `stages`, `jobs`, `steps`. Two resolve by relative path inside the workspace; `deploy-stages.yml@shared` names a repository resource and deliberately **cannot** be followed, which is what Requirement 5.3 has to report rather than hide. |
| `extends.yml` + `templates/pipeline.yml` | `extends`, where the pipeline's real shape is the template's and what the file contributes is parameters (Requirement 5.5). |

### Edge cases

| File | What it guards |
|---|---|
| `edge-comments.yml` | Comments in every position: leading, trailing on a key, trailing on a sequence entry, inside a mapping, separated by a blank line, and a `#` inside a block scalar that is **script content, not a comment**. |
| `edge-expressions.yml` | `${{ }}` at compile time and `$[ ]`-style runtime conditions, in `displayName`, `condition` and around a whole stage — including `${{ if }}`, where whether a stage exists at all is not knowable statically (Requirements 3.4, 8.4). |
| `edge-anchors.yml` | YAML anchors, aliases and a merge key. A serialiser would expand these and lose the author's intent, which is the clearest single argument for the concrete syntax tree. Also carries `dependsOn: []`. |
| `edge-indentation.yml` | Sequence entries not indented under their key — valid YAML, and precisely the shape a reformatter silently "fixes". |
| `edge-crlf.yml` | CRLF throughout. Every other file is LF, so the pair proves the writer preserves what it read rather than imposing a house style. |
| `edge-broken-graph.yml` | Deliberately wrong, so the validator has something real to catch: a dangling `dependsOn`, a two-stage cycle, and a stage nothing can reach (Requirements 10.2–10.4). It is **not** expected to be a valid pipeline; it is expected to round-trip byte-identically anyway, because a broken pipeline is exactly when a user needs ADP not to make it worse. |

## What a fixture is not for

These files are not a schema conformance suite. The module models the constructs Requirement 4
names and preserves everything else verbatim (Requirement 3.3), so a fixture's job is to carry
constructs the module does **not** model — `resources`, `variables`, task `inputs`, block
scalars — past the writer untouched. A fixture that only contained what the parser understands
would prove nothing about the guarantee that matters.
