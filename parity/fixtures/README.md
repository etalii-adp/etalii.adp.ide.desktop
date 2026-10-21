# Parity fixtures

The documents the edit scripts under `parity/scripts/` open, copied from the standalone host at the reference commit [`8b6ccfc`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/8b6ccfca4f4a65e43c1a1a73a3224abe7ebf800e) as git holds them (`git show 8b6ccfc:<path>`, so a fixture is the committed bytes, whatever a checkout's line-ending settings make of them; `roadmap.tml` is committed with CRLF line endings). Each folder is copied whole into the project a script runs in, so it holds every file its document reads. `.gitattributes` keeps them byte for byte. Never edit them: copy them again from the standalone host.

| Folder | Copied from (in `etalii.adp.ide.standalone` at `8b6ccfc`) | Script |
|---|---|---|
| `etalii/functional-decomposition-graph/field-service/` | `src/diagrams/functional-decomposition-graph/examples/field-service/` (`field-service.adp`, `field-service.fdg`) | `parity/scripts/etalii/functional-decomposition-graph/field-service.json` |
| `freeplane/mindmap/example-1/` | `src/diagrams/mindmap/examples/example 1/` (`mindmap.adp`, `mindmap.mm`, `design.mm`) | `parity/scripts/freeplane/mindmap/example-1.json` |
| `generic/dependencies/services/` | `src/diagrams/dependency-graph/examples/example-1/` (`services.adp`, `services.dgr`, committed with CRLF line endings) | `parity/scripts/generic/dependencies/services.json` |
| `generic/timeline/roadmap/` | `src/diagrams/timeline/examples/example-2/` (`roadmap.adp`, `roadmap.tml`) | `parity/scripts/generic/timeline/roadmap.json` |
| `etalii/knowledge/cities-yaml/`, `cities-json/`, `cities-xml/` | `src/designers/knowledge/examples/` (`cities.yaml`, `cities.json` or `cities.xml`, and `provinces.yaml`, the table their relation names), each with the registration `cities.adp` (`etalii/knowledge` and `body: cities.<format>`, CRLF), written as the standalone host's `KnowledgeDesignerTests` writes it | `parity/scripts/etalii/knowledge/cities-yaml.json`, `cities-json.json`, `cities-xml.json` |
| `c4/container/courier/` | `src/diagrams/c4/examples/reference/architecture/` (the Courier Tracking model `courier.dsl`, its layout `courier.layout.json`, its seven registrations, and the `docs/` and `decisions/` folders it names) | `parity/scripts/c4/container/courier.json` |
| `systems/causal-loop-diagram/reference/` | `src/diagrams/causal-loop-diagram/examples/reference/` (`reference.adp`, `reference.cld`) | `parity/scripts/systems/causal-loop-diagram/reference.json` |
| `systems/causal-loop-diagram/on-call/` | `src/diagrams/causal-loop-diagram/examples/on-call/` (`on-call.adp`, `on-call.cld`) | `parity/scripts/systems/causal-loop-diagram/on-call.json` |
