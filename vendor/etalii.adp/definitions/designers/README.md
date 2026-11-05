# Designer definitions

This folder holds the files for ADP's designer types, the form-based visual layouts in which nothing is connected:

- **DESL specification files** (`.des`), in which a tool engineer specifies one designer type ([DESL](../../specifications/desl/DESL-specification.md));
- beside each, a `.md` file holding what DESL cannot express, the FBL bindings (`.fbl`) its documents are read and written through, and the JSON Schema and examples of those documents.

| Designer | Origin | Files |
|---|---|---|
| Knowledge | `etalii/knowledge` | `knowledge.des`, `knowledge.md`, `knowledge.fbl`, `knowledge.schema.json`, `examples/` |
| Hierarchical Requirement Designer | `etalii/hierarchical-requirement` | `hierarchical-requirement.des`, `hierarchical-requirement.md`, `hierarchical-requirement.fbl`, `hierarchical-requirement.schema.json`, `examples/web-shop.md`, `examples/web-shop.adp` |

A designer's documents are DED definitions ([DED](../../specifications/ded/DED-specification.md)): each begins with DED's envelope, naming DED's version and its designer type (in a Markdown document, line 1 is that envelope as an HTML comment, DED section 2), and is read and written through the FBL bindings of its designer type's specification. The words used here are defined in [docs/terminology.md](../../docs/terminology.md).

## Tool descriptions

Each type here has a tool description in the ADP tool language ([ATD](../../specifications/atd/ATD-specification.md)): a root `<name>.atd` with its parts beside it, and a `<name>.trace.json` that traces what it carries back to the files it comes from. Its conformance cases are in `conformance/<type>/<case>/`. The Knowledge designer has no description yet (spec 076). A person's document still opens on today's tool, read from the JSON files, and not on the description; the `atd` command line checks and validates the descriptions.
