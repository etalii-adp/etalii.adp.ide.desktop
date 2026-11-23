# Links between tools: a worked example

This folder is one workspace (FBL section 17.3) with one document of each tool type that holds links (spec 089): a knowledge file, a mind map, a timeline and a dependency graph, all about one shop relaunch. Every link in it resolves and is current, so opening any of the four rewrites nothing. It is written for readers; the cases a host must handle, broken, stale, ambiguous and old-form links among them, are in [`fixtures/links-workspace/`](../fixtures/links-workspace/).

| File | Tool type | Document id | Links to |
|---|---|---|---|
| [`features.yaml`](features.yaml) | Knowledge designer (`etalii/knowledge`) | `k7q2m9xa` | itself: Checkout needs Sign-in, through the relation Needs, whose backlinks show as Needed by |
| [`ideas.mm`](ideas.mm) | Mind map (`freeplane/mindmap`) | `p4r8t1zc` | a feature, a timeline milestone and a dependency graph node, each as a linked node |
| [`roadmap.tml`](roadmap.tml) | Timeline (`generic/timeline`) | `r6d3w8ne` | two features as linked periods, a mind map node and a dependency graph node as linked moments |
| [`services.dgr`](services.dgr) | Dependency graph (`generic/dependencies`) | `s2v5b9hu` | a feature, a timeline milestone and the mind map's central topic as linked nodes, one of them depending on a node of the graph |

How to read a link, taking the timeline's `$k7q2m9xa@features.yaml#checkout.Name=Checkout`:

- `$k7q2m9xa` is the document id of the target, kept in the target itself: the knowledge file's envelope `id`, the mind map's `adp-id` attribute on its central topic, the top-level `id` of a timeline or a graph (FBL 17.2). It survives a move or a rename of the file.
- `@features.yaml` is the target's file name, used only when the id does not pick one document, as after a copy (FBL 17.3).
- `#checkout` is the id of the element within it: a knowledge row, a mind map node (`ID_…`), a timeline element or a graph node.
- `.Name` names the property that names the element, and `=Checkout` caches that name. A host checks the cached name when it loads the document that holds the link, and rewrites it when the target was renamed (FBL 17.4).

What the four tool types do with a link:

- A **knowledge** relation names a knowledge file, and its cells link rows of that file; a knowledge file links no diagram. The relation picker is Notion's (knowledge.md, *Relations and the steps that write two files*).
- A **mind map** linked node is an ordinary Freeplane node whose `TEXT` is the cached name and whose `adp-link` attribute holds the link, so Freeplane opens the map unchanged (mindmap.md, section 3.8).
- A **timeline** linked element and a **dependency graph** linked node are entries of `elements` with a `link` key, placed by their own `begin`, `end` and `row`, or `x` and `row`, and connected like any other (timeline.md, *The document on disk*; dependency-graph.md, section 1.1).
- Every element that can be linked to shows its backlinks as **Linked from**, computed from the workspace and stored nowhere (FBL 17.5).

`validate-examples.py` reads this folder on every pull request. It checks that each document has an id, that each diagram links the other kinds, that every link resolves without a finding and would not be rewritten, that the knowledge file is valid against its schema, and that each linked node's `TEXT` is its cached name.
