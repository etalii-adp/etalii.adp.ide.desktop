# Legacy fixtures: DISL

Files kept byte-for-byte as they were written before an identifier changed: `erd.dedl` as DEDL 0.1 wrote it, before DEDL became DISL and DID (spec 002, naming convention alignment), and `erd.disl` as DISL 0.1 stored it before specification files took the extension `.dis` (Peter, 2026-09-30). They prove that the deprecated aliases of [DISL section 18](../DISL-specification.md#18-deprecated-aliases) are still read: `.github/scripts/validate-examples.py` validates each one against `disl.schema.json` through its alias table on every change. Never edit them, and never copy their identifiers into a new file.

| Legacy fixture | Migrated counterpart |
|---|---|
| `erd.dedl` | [`specifications/disl/erd.dis`](../erd.dis) |
| `erd.disl` | [`specifications/disl/erd.dis`](../erd.dis) |

They are removed when DISL drops the aliases, no earlier than DISL 1.0.
