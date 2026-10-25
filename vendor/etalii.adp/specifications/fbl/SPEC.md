---
id: fbl
type: module-design
title: Format Binding Language (FBL)
status: draft
parent: languages
---
# Format Binding Language (FBL)

Module node for `specifications/fbl/`, the cross-kind binding language. The normative content is [FBL-specification.md](FBL-specification.md) (`fbl-specification`), with its schema `fbl.schema.json` and its examples (`.fbl`) beside it. This node records only what the language is for and where its boundary runs.

## Responsibility

How a tool reads and writes a model that lives in another tool's file (Structurizr DSL, YAML, `.mm` and so on) through byte-preserving splices, and the `.adp` registration. It also defines the contract for persistence plugins.

## Boundary

Serves every kind and depends on none: DISL and DESL point at it, not the other way round. It owns bindings, bodies, splices, readings, folder subjects and the plugin contract. Round-trip fixtures live under `fixtures/`.
