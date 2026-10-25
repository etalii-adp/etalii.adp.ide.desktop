---
id: did
type: module-design
title: Diagram Definition Language (DID)
status: draft
parent: languages
depends-on:
  - disl
---
# Diagram Definition Language (DID)

Module node for `specifications/did/`, the definition language for diagrams. The normative content is [DID-specification.md](DID-specification.md) (`did-specification`), with its schema `did.schema.json` and its examples (`.did`) beside it. This node records only what the language is for and where its boundary runs.

## Responsibility

How one diagram a user created of a DISL-specified type is stored.

## Boundary

Owns the stored diagram only. Its vocabulary of elements and relations comes from the DISL specification it names. DID here is never the W3C's Decentralized Identifiers.
