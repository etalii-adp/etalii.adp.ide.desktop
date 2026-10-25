---
id: ded
type: module-design
title: Designer Definition Language (DED)
status: draft
parent: languages
depends-on:
  - desl
---
# Designer Definition Language (DED)

Module node for `specifications/ded/`, the definition language for designers. The normative content is [DED-specification.md](DED-specification.md) (`ded-specification`), with its schema `ded.schema.json` and its examples (`.ded, or a format the designer type's bindings read, such as .md`) beside it. This node records only what the language is for and where its boundary runs.

## Responsibility

How one designer a user created of a DESL-specified type is stored: as JSON, or in a format that the designer type's bindings read, such as Markdown with a line-1 envelope.

## Boundary

Owns the stored designer only. The `.md` files beside its specification are validated as DED Markdown definitions, which is why `validate-examples.py` excludes `SPEC.md` from them.
