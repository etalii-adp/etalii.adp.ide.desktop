# EDD — Editor Definition Language

**Specification, no version yet (Placeholder)**

|   |   |
|---|---|
| Kind | editor |
| Role | definition language |
| File extension | `.edd` |
| Paired language | EDSL, the Editor Specification Language (`.eds`) |
| Schema | none yet |
| Licence | [Apache License 2.0](https://github.com/etalii-adp/etalii.adp/blob/develop/LICENSE) (`Apache-2.0`) |

---

## Status of this document

This is a placeholder. It holds nothing normative: no construct, no schema and no examples. It exists so that each kind of ADP tool has its specification language and its definition language in one pattern, as the [terminology](../../docs/terminology.md) and the constitution's Structure and Naming section require. Its content is to come, when an editor needs it (constitution principle V).

## Purpose

EDD will specify how an editor a user created of an editor type is stored. An EDD file (`.edd`) will hold one such editor and name the EDSL specification it was made with, as a DID file does for a diagram.

When content arrives, it follows the pattern of [DISL](../disl/DISL-specification.md) and [DID](../did/DID-specification.md): this document, the schema `edd.schema.json` with `$id` `https://etalii.net/adp/edd/schema/<version>/edd.schema.json`, the version key `"edd": "<version>"`, and examples with the extension `.edd`, side by side in `specifications/edd/`.
