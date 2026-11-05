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
