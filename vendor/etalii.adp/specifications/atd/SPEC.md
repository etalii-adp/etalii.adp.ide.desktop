---
id: atd
type: module-design
title: ADP Tool Language (ATD)
status: draft
parent: languages
references:
  - disl
  - desl
  - fbl
---
# ADP Tool Language (ATD)

Module node for `specifications/atd/`, the cross-kind text language for tool descriptions. The normative content is [ATD-specification.md](ATD-specification.md) (`atd-specification`), with its schema `atd.schema.json` and its examples (`.atd`) beside it. This node records only what the language is for and where its boundary runs.

## Responsibility

The final form of every tool type's specification (spec 035, ruling of 2026-10-22: `.atd` alone in the end, after a transition; see `architecture` *Tool-type approaches*). One text language under one strict LALR(1) grammar (`grammar/*.lark`), in which a tool type of any kind is described completely: identity, model, surface, behaviour, bindings and logic.

## Boundary

Its schema describes the parsed tree, not an authored JSON form. Its version key `atd <major>.<minor>` is the first construct of every file. While phase 1 of spec 035 lasts, descriptions are written and checked beside the DISL, DESL and FBL files, and no host loads from them, so ATD only *references* those languages. Generated files (`atd.tables.json`, `atd.flat.lark`, `highlighting/`) are compared by `atd-grammar.py --check` and never edited by hand.
