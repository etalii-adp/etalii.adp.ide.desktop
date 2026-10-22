# Refused bindings

The FBL documents in this folder are valid against [`fbl.schema.json`](../fbl.schema.json) and **must be refused** when they are loaded, by a check of [section 14.1](../FBL-specification.md#141-loading-an-fbl-document) that the schema cannot make. They are the other half of the examples beside the specification: those show what a host reads, these show what it refuses. Each document's `doc` names the check it fails and why, and it fails that check only.

`python .github/scripts/validate-examples.py` reads every `*.fbl` here as it reads every other FBL document, and reports it as refused with the first problem it found. The run fails when a document here is accepted, or when the schema already refuses it, since then it shows nothing the schema does not. A host's conformance run loads the same documents and expects a refusal.

| Document | Refused by |
|---|---|
| `insert-constant-collision.fbl` | *(FBL 0.8)* Section 14.1, step 6: an insert's `constant` names a key that the same rule writes through a writable attribute binding (section 6.2). |
