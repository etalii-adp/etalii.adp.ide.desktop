# FAIR Data Point navigation shapes

A shapes graph from a deployed system rather than from a specification: the FAIR Data Point
reference implementation ships these to describe how its metadata layers link together, and
they are read by the running server.

- **Source**: <https://github.com/FAIRDataTeam/FAIRDataPoint> —
  `src/main/resources/defaultNavigationShacl.ttl` on the `develop` branch.
- **Retrieved**: 2026-09-04.
- **License**: MIT, Copyright (c) 2017 FAIR Data Team. Re-verified at acquisition by fetching
  the repository's own `LICENSE`, which is vendored beside this readme unmodified.
- **Local changes**: none. The file is byte-for-byte as published, renamed from
  `defaultNavigationShacl.ttl` to `navigation-shapes.ttl` so the folder reads plainly.

## Why this one

It is small, real, and every shape in it is doing a job. Five node shapes, each with a
`sh:targetClass` aimed at a class held in *another* vocabulary — `r3d:Repository`,
`dcat:Catalog`, `dcat:Dataset` — which is exactly the situation this reading exists to draw
honestly: the targets point outside the file, so they are drawn as declarations on their cards
and no edge dangles into nothing.

Its property shapes are all blank nodes, as the world writes them, so it exercises the
identity boundary on real data: the rows draw and read, and nothing offers to edit them in
place. And its `sh:node` references form a genuine cycle — a catalogue points at its datasets
and each dataset points back at its catalogue through `dct:isPartOf` — which the reading draws
as edges without needing to break anything, because it is drawing a description rather than
walking a hierarchy.
