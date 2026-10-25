---
id: definitions
type: module-design
title: Tool definitions catalogue
status: draft
parent: architecture
depends-on:
  - disl
  - did
  - desl
  - ded
  - fbl
  - atd
---

# Tool definitions catalogue

## Responsibility

The concrete tool types ADP ships, written in this repository's languages: diagram types in `diagrams/` (DISL `.dis`, FBL bindings, ATD descriptions, traces and conformance fixtures), designer types in `designers/` (DESL `.des`, FBL bindings, ATD descriptions, document schemas and example documents), and `editors/`, a placeholder until EDSL has content. Every file here is validated against its language's schema by `ci-validators`.

## Boundary

- It consumes the languages and never extends them. A need a tool type finds goes back to the owning language module as a specification change first (constitution I).
- The worked examples of a language stay beside that language in `specifications/<name>/`. This folder holds shipped tool types only.
- A tool type is named after its origin `<vendor>/<type>`, which gives the language id `net.etalii.adp.<vendor>.<type>`. The full naming rules are in `diagrams/README.md`.
- A `.md` file beside a tool type holds what its language cannot express. It is input to hosts, not a normative specification. *(Inferred from the folder READMEs; unconfirmed.)*
