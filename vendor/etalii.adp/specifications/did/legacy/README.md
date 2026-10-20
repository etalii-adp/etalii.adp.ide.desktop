# Legacy fixtures: DID

Files kept byte-for-byte as DEDL 0.1 wrote them, before DEDL became DISL and DID (spec 002, naming convention alignment). They prove that the deprecated aliases of [DID section 11](../DID-specification.md#11-deprecated-aliases) are still read: `.github/scripts/validate-examples.py` validates each one against `did.schema.json` through its alias table on every change. Never edit them, and never copy their identifiers into a new file.

| Legacy fixture | Migrated counterpart |
|---|---|
| `timeline.document.json` | [`specifications/did/timeline.did`](../timeline.did) |

They are removed when DID drops the aliases, no earlier than DID 1.0.
