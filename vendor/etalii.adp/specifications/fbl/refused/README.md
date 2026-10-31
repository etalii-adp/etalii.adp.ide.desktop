# Refused bindings

The FBL documents in this folder are valid against [`fbl.schema.json`](../fbl.schema.json) and **must be refused** when they are loaded, by a check of [section 14.1](../FBL-specification.md#141-loading-an-fbl-document) that the schema cannot make. They are the other half of the examples beside the specification: those show what a host reads, these show what it refuses. Each document's `doc` names the check it fails and why, and it fails that check only.

`python .github/scripts/validate-examples.py` reads every `*.fbl` here as it reads every other FBL document, and reports it as refused with the first problem it found. The run fails when a document here is accepted, or when the schema already refuses it, since then it shows nothing the schema does not. A host's conformance run loads the same documents and expects a refusal.

| Document | Refused by |
|---|---|
| `insert-constant-collision.fbl` | *(FBL 0.8)* Section 14.1, step 6: an insert's `constant` names a key that the same rule writes through a writable attribute binding (section 6.2). |
| `origins-not-claimed.fbl` | *(FBL 0.10)* Section 14.1, step 6: a rule's `origins` lists an origin that is not one of the binding's `claims.origins` (section 5.1). |
| `resource-by-origin-not-claimed.fbl` | *(FBL 0.10)* Section 14.1, step 6: `registration.resource.byOrigin` names an origin that is not one of the binding's `claims.origins` (section 8.2). |
| `grammar-recover-nested-unknown-terminal.fbl` | *(FBL 0.11)* Section 14.1, step 6, which `atd-grammar.py --bindings` makes too: `recover.nested.close` names no terminal of the binding's grammar (section 4.9). |
| `grammar-create-after-unknown.fbl` | *(FBL 0.11)* Section 14.1, step 6: an attribute binding's `create.after` names an attribute the rule does not bind (section 4.9). |
| `grammar-selected-view-without-views.fbl` | *(FBL 0.11)* Section 14.1, step 6: a rule with `selectedView` in a binding with no block rule that has `view` (sections 4.9, 9.3). |
| `grammar-gather-writable.fbl` | *(FBL 0.11)* Section 14.1, step 6: `gather` on an attribute binding that is not read-only (section 4.9). |
| `items-not-many.fbl` | *(FBL 0.12)* Section 14.1, step 6: `items` on an attribute that the tool type the binding is read for declares single-valued (section 5.2). |
| `split-not-many.fbl` | *(FBL 0.13)* Section 14.1, step 6: `split` on an attribute that the tool type the binding is read for declares single-valued (sections 4.9, 5.2). |
| `split-escape-collision.fbl` | *(FBL 0.13)* Section 14.1, step 6: a slot's `split` holds a character of its `escape` (section 4.9). |
| `insert-member-without-capture.fbl` | *(FBL 0.14)* Section 14.1, step 6: a rule that adds a member of a mapping binds no attribute to the capture its `at` ends in (section 6.2). |
| `insert-refusals-not-member.fbl` | *(FBL 0.14)* Section 14.1, step 6: `refusals.key` and `refusals.duplicate` on an insert that adds no member (section 6.2). |
