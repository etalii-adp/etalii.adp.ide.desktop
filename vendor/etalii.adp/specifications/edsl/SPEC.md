---
id: edsl
type: module-design
title: Editor Specification Language (EDSL)
status: draft
parent: languages
---
# Editor Specification Language (EDSL)

Module node for `specifications/edsl/`, the specification language for editors (placeholder). The normative content is [EDSL-specification.md](EDSL-specification.md) (`edsl-specification`), with its schema `edsl.schema.json` and its examples (`.eds`) beside it. This node records only what the language is for and where its boundary runs.

## Responsibility

Will specify editor types, in which typing text is the core interaction. It is a placeholder with nothing normative until an editor needs it (constitution V).

## Boundary

Placeholder: its document states purpose and extension only, and it has no schema.

Editors exist all the same, as host code: the standalone host's JSON, XML, YAML, Markdown and plain-text editors and the desktop host's ports of them (approach 1 of `architecture` *Tool-type approaches*). The standalone host's JSON and XML editors list what an EDSL specification would have to state (etalii.adp issue 154). ATD (`atd`) already describes a text surface, and it is the final form for every tool type, editors included.
