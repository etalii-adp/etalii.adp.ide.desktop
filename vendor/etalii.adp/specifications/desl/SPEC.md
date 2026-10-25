---
id: desl
type: module-design
title: Designer Specification Language (DESL)
status: draft
parent: languages
depends-on:
  - fbl
---
# Designer Specification Language (DESL)

Module node for `specifications/desl/`, the specification language for designers. The normative content is [DESL-specification.md](DESL-specification.md) (`desl-specification`), with its schema `desl.schema.json` and its examples (`.des`) beside it. This node records only what the language is for and where its boundary runs.

## Responsibility

How a tool engineer specifies one designer type: a form-based layout in which nothing is connected.

## Boundary

Owns the designer type. Stored designers are `ded`. Designer documents are read and written through FBL bindings (`persistence.bindings`).
