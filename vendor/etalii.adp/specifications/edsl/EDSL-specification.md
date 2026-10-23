# EDSL — Editor Specification Language

**Specification, no version yet (Placeholder)**

|   |   |
|---|---|
| Kind | editor |
| Role | specification language |
| File extension | `.eds` |
| Paired language | EDD, the Editor Definition Language (`.edd`) |
| Schema | none yet |
| Licence | [Apache License 2.0](https://github.com/etalii-adp/etalii.adp/blob/develop/LICENSE) (`Apache-2.0`) |

---

## Status of this document

This is a placeholder. It holds nothing normative: no construct, no schema and no examples. It exists so that each kind of ADP tool has its specification language and its definition language in one pattern, as the [terminology](../../docs/terminology.md) and the constitution's Structure and Naming section require. Its content is to come, when an editor needs it (constitution principle V).

## Until EDSL has content: ATD's text surface

In phase 1 of the move to one language for every tool type (spec 035), the editor kind is specified by ATD, the ADP tool language: its text surface, [ATD section 10](../atd/ATD-specification.md#10-the-text-surface), states what an editor type does (its files, token classes, folding, outline, completion sources, checks, quick fixes and previews), written from the five editors the hosts implement, and every editor type in scope is described in an `.atd` tool description under `definitions/editors/`. EDSL is written from ATD's text surface later, rather than separately; until then this document stays a placeholder.

## Purpose

In EDSL a tool engineer will specify one editor type: a way of working in which typing text is the core interaction. An EDSL file (`.eds`) will hold one editor type, as a DISL file holds one diagram type. The editors users create of such a type are stored in EDD.

When content arrives, it follows the pattern of [DISL](../disl/DISL-specification.md) and [DID](../did/DID-specification.md): this document, the schema `edsl.schema.json` with `$id` `https://etalii.net/adp/edsl/schema/<version>/edsl.schema.json`, the version key `"edsl": "<version>"`, and examples with the extension `.eds`, side by side in `specifications/edsl/`.
