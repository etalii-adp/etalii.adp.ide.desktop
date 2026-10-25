---
id: disl
type: module-design
title: Diagram Specification Language (DISL)
status: draft
parent: languages
depends-on:
  - fbl
---
# Diagram Specification Language (DISL)

Module node for `specifications/disl/`, the specification language for diagrams. The normative content is [DISL-specification.md](DISL-specification.md) (`disl-specification`), with its schema `disl.schema.json` and its examples (`.dis`) beside it. This node records only what the language is for and where its boundary runs.

## Responsibility

How a tool engineer specifies one diagram type: its metamodel, coordinates, notation, toolbox, constraints, behaviour, layout and persistence.

## Boundary

Owns the diagram type. Does not own stored diagrams (that is `did`) or how a foreign file is read and written (that is `fbl`, which DISL points at through `persistence.binding`).
