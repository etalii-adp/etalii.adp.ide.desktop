# Designer definitions

This folder holds the files for ADP's designer types, the form-based visual layouts in which nothing is connected:

- **DESL specification files** (`.des`), in which a tool engineer specifies one designer type ([DESL](../../specifications/desl/DESL-specification.md));
- beside each, a `.md` file holding what DESL cannot express, the FBL bindings (`.fbl`) its documents are read and written through, and the JSON Schema and examples of those documents.

| Designer | Origin | Files |
|---|---|---|
| Knowledge | `etalii/knowledge` | `knowledge.des`, `knowledge.md`, `knowledge.fbl`, `knowledge.schema.json`, `examples/`; the tool description `knowledge.atd` with `knowledge.yaml.atd`, `knowledge.json.atd` and `knowledge.xml.atd`, its trace `knowledge.trace.json` and its cases under `conformance/knowledge/` |
| Hierarchical Requirement Designer | `etalii/hierarchical-requirement` | `hierarchical-requirement.des`, `hierarchical-requirement.md`, `hierarchical-requirement.fbl`, `hierarchical-requirement.schema.json`, `examples/web-shop.md`, `examples/web-shop.adp` |
| Post-incident review designer | `etalii/post-incident-review` | `post-incident-review.des`, `post-incident-review.md`, `post-incident-review.fbl`, `post-incident-review.schema.json`, `examples/checkout-latency.md`, `examples/certificate-expiry.md`; its tool description is to follow (spec 086) |

A designer's documents are DED definitions ([DED](../../specifications/ded/DED-specification.md)): each begins with DED's envelope, naming DED's version and its designer type (in a Markdown document, line 1 is that envelope as an HTML comment, DED section 2), and is read and written through the FBL bindings of its designer type's specification. The words used here are defined in [docs/terminology.md](../../docs/terminology.md).

## Tool descriptions

Each type here has a tool description in the ADP tool language ([ATD](../../specifications/atd/ATD-specification.md)): a root `<name>.atd` with its parts beside it, and a `<name>.trace.json` that traces what it carries back to the files it comes from. Its conformance cases are in `conformance/<type>/<case>/`. The Knowledge designer has no description yet (spec 076). A person's document still opens on today's tool, read from the JSON files, and not on the description; the `atd` command line checks and validates the descriptions.

## Links between tools

The knowledge designer's relations are links between documents (spec 089, [FBL section 17](../../specifications/fbl/FBL-specification.md#17-links-between-documents), DESL 7.5.1): a relation names its target file by document id and name, each cell links rows of it, and its rows can be linked to from a mind map, a timeline or a dependency graph. A file made before links is converted once when it is loaded (`knowledge.md`, *Relations and the steps that write two files*). [`specifications/fbl/links-example/`](../../specifications/fbl/links-example/) is a workspace with one document of each of the four tool types that hold links.
