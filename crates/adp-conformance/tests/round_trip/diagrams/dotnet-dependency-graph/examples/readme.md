# .NET dependency graph — examples

## `example 1` — `pipeline-toolkit`

A small purpose-built solution: four projects, three packages, and every reference kind this
type draws. **Purpose-built rather than vendored**, so there is no third-party licence question
here — nothing under this folder came from anywhere else.

It is small enough to read in one screen, and it is deliberately not simple. Each of these is a
case the readers get wrong if nobody thought about it:

| What is in it | What it exercises |
|---|---|
| Bare `PackageReference` lines plus a `Directory.Packages.props` | Central package management — the **ordinary** case in a modern solution, not an edge case |
| `Serilog` at `4.4.0` centrally, pinned to `3.1.1` in the test project | A **version conflict**: one element, both versions, marked (Requirement 3.5) |
| `Pipeline.Storage` targeting `net10.0;netstandard2.0` | Multi-targeting, and a project whose frameworks include one that names **no .NET version** |
| `Pipeline.Cli` referencing two projects, one of which references a third | A dependency chain deep enough for the layered layout to have something to say |
| `<Folder>` elements holding both `<Project>` and `<File>` entries | The `.slnx` shape that makes a naive reader turn `Directory.Packages.props` into a node |

## What this corpus does **not** demonstrate

Recorded because a reader who assumes the example covers everything will trust the diagram
further than it has earned:

* **No classic `.sln`.** The example ships `.slnx` only — as does this repository itself, which
  contains no `.sln` anywhere. Classic-format support exists and is covered by unit fixtures,
  but no example here exercises it end to end.
* **No unresolvable reference.** Every project the solution names is present, and every package
  resolves to a version. The failure paths — a missing project file, an unparseable `.csproj`, a
  version nothing declares — are covered by tests rather than by an example, because an example
  that failed to open would be a poor advertisement for a type whose whole promise is that it
  opens with what it could resolve.
* **No transitive packages.** Only direct `PackageReference` declarations become edges, which is
  the requirements' recorded decision. A real solution's restore graph is far larger than what
  this diagram draws, deliberately.
* **No package descriptions, unless you have restored these packages.** Descriptions come from
  the **local NuGet cache only**. On a machine that has never restored `Serilog`, the property
  grid says so rather than showing a blank — which is the accepted, explicit cost of the
  workspace-files-only decision, and worth seeing rather than reading about.
* **Not a scale subject.** Four projects is a readable example, not a demanding one. The scale
  question is answered against `EtAlii.Adp.slnx` — roughly thirty backend projects plus sixty-one
  diagram modules — which is why the requirements asked for both and not either.

## The scale subject

`EtAlii.Adp.slnx`, this repository's own solution, is the second subject the requirements name.
It is **not copied into this folder**: it is already here, it changes with the repository, and a
stale copy would be worse than none. To look at it, register it through Add on
`src/backend/EtAlii.Adp.slnx`.
