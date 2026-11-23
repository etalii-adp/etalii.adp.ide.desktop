# Diagram definitions

This folder is for the files of ADP's diagram types, written in the diagram languages of this repository:

- **DISL specification files** (`.dis`), in which a tool engineer specifies one diagram type: its metamodel, coordinates, notation, toolbox, constraints, behavior, layout and persistence ([DISL](../../specifications/disl/DISL-specification.md));
- **DID definition files** (`.did`), each one diagram a user created of such a type ([DID](../../specifications/did/DID-specification.md)).

The worked examples of the languages themselves stay beside their specifications in `specifications/disl/` and `specifications/did/`. The words used here are defined in [docs/terminology.md](../../docs/terminology.md).

## Naming

Every diagram type here is named from its origin, the `<vendor>/<type>` tag the tool catalogue gives it (for example `c4/context` or `w3c/rdf`), so the same tool has one name in every repository:

- **Language id**: `net.etalii.adp.<vendor>.<type>`, the origin with its slash replaced by a dot and nothing else changed (`c4/system-landscape` is `net.etalii.adp.c4.system-landscape`, `etalii/functional-decomposition-graph` is `net.etalii.adp.etalii.functional-decomposition-graph`).
- **Plugin name**: `net.etalii.adp.<vendor>.<plugin>`, with the vendor of the diagram types the plugin serves and the plugin's own part in lower camel case (`net.etalii.adp.c4.structurizrDsl`, `net.etalii.adp.w3c.turtle`). A plugin shared by a family keeps one name in every specification of that family. A plugin that serves any diagram type and belongs to the IDE rather than to a notation uses `ide` as its vendor (`net.etalii.adp.ide.revealPath`).

## Tool descriptions

Each type here also has a tool description in the ADP tool language ([ATD](../../specifications/atd/ATD-specification.md)): a root `<name>.atd` with its parts beside it, and a `<name>.trace.json` that traces what it carries back to the files it comes from. Its conformance cases, recorded from the type's module, are in `conformance/<type>/<case>/`. A person's document still opens on today's tool, read from the JSON files, and not on the description; the `atd` command line checks and validates the descriptions.

## Worked examples

A diagram type's worked examples, documents a person or an agent could have written, are in `examples/`, named for what they show and carrying the extension the type's specification names (`language.fileExtension`). Where the type has a file schema beside its specification (`<name>.schema.json`), `validate-examples.py` checks each example against it. The value stream map's (`generic/value-stream-map`, `.vsm`) are the first: `restore-service.vsm` and `request-fulfilment.vsm`, whose totals its companion works by hand.

## Vendored documents

Documents a diagram type's fixtures read that someone else published are in `vendored/<source>/`, byte for byte, with the source's licence beside them as `LICENSE.md`, verbatim, and a `README.md` naming the repository, the commit, each file's source path and what the corpus does not show. Only permissive licences are taken; a share-alike one is refused. OpenSLO's (`vendored/openslo/`, Apache License 2.0) are the first, read by the SLO map's (`openslo/slo`) fixtures under `specifications/fbl/fixtures/openslo-slo-read/`. They are here rather than beside the fixtures because `specifications/` holds this repository's licence alone (`licence-check.py`).

## Links between tools

Three diagram types here hold links to elements of other documents and can be linked to (spec 089, [FBL section 17](../../specifications/fbl/FBL-specification.md#17-links-between-documents), DISL 4.12): the mind map (`mindmap.dis`, a linked node), the timeline (`timeline.dis`, a linked period or moment) and the dependency graph (`dependency-graph.dis`, a linked node); the fourth is the knowledge designer, in `../designers/`. Each keeps an 8-character document id in its body, offers *Add from another document…*, and states in its companion how a link is stored, drawn when broken, and listed as *Linked from*. [`specifications/fbl/links-example/`](../../specifications/fbl/links-example/) is a workspace with one document of each of the four.
