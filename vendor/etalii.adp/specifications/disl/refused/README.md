# Refused specifications

The DISL specifications in this folder are valid against [`disl.schema.json`](../disl.schema.json) and **must be refused** when they are loaded, by a check of [section 14.1](../DISL-specification.md#141-loading-a-specification), step 6, that the schema cannot make. They are the other half of the examples beside the specification: those show what a host reads, these show what it refuses. Each document's root `doc` names the check it fails and why, and it fails that check only.

`python .github/scripts/validate-examples.py` reads every `*.dis` here as it reads every other DISL specification, and reports it as refused with the first problem it found. The run fails when a document here is accepted, or when the schema already refuses it, since then it shows nothing the schema does not. A host's conformance run loads the same documents and expects a refusal.

| Document | Refused by |
|---|---|
| `relation-flag-widened.dis` | *(DISL 0.6)* Section 4.7, relation flags: `Strong` states `allowParallel: true` under `Link`'s `false`, which widens it. |
| `relation-flag-directed.dis` | *(DISL 0.6)* Section 4.7, relation flags: `Mutual` states `directed: false`, which differs from the default `true` of its supertype `Link`. |
| `relation-flag-conflict.dis` | *(DISL 0.6)* Section 4.7, relation flags: `Both` states no `allowSelfLoops`, and has `true` from `Loop` and `false` from `Plain`. |
