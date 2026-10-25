---
id: definitions
type: module-design
title: Tool definitions catalogue
status: draft
parent: architecture
depends-on:
  - disl
  - did
  - desl
  - ded
  - fbl
  - atd
---

# Tool definitions catalogue

## Responsibility

The concrete tool types ADP ships, written in this repository's languages: diagram types in `diagrams/` (DISL `.dis`, FBL bindings, ATD descriptions, traces and conformance fixtures), designer types in `designers/` (DESL `.des`, FBL bindings, ATD descriptions, document schemas and example documents), and `editors/`, a placeholder until EDSL has content. Every file here is validated against its language's schema by `ci-validators`.

## Boundary

- It consumes the languages and never extends them. A need a tool type finds goes back to the owning language module as a specification change first (constitution I).
- The worked examples of a language stay beside that language in `specifications/<name>/`. This folder holds shipped tool types only.
- A tool type is named after its origin `<vendor>/<type>`, which gives the language id `net.etalii.adp.<vendor>.<type>`. The full naming rules are in `diagrams/README.md`.
- A `.md` file beside a tool type holds what its language cannot express. It is input to hosts, not a normative specification. *(Inferred from the folder READMEs; unconfirmed.)*

## Availability per host

Which of the tool types in this folder each host runs, as of 2026-10-25, read from each host's `develop` branch. ✓ means the host ships the tool type, ◐ that it is being built there, and – that the host does not have it. Eclipse (`host-eclipse`) and Claude (`host-claude`) hold no code yet and run none of them, so they have no column.

| Tool type | Origin | Kind | Standalone | Desktop | VS Code | IntelliJ | Notion |
|---|---|---|---|---|---|---|---|
| Agent activity diagram | `etalii/agent-activity-diagram` | diagram | ✓ | ✓ | ✓ | – | – |
| Agent Behavior Modelling | `etalii/agent-behavior-modelling` | diagram | ✓ | ✓ | ✓ | – | – |
| Ansible project structure | `ansible/structure` | diagram | ✓ | ✓ | – | – | – |
| Azure DevOps pipeline | `azure-devops/pipeline` | diagram | ✓ | ✓ | – | – | – |
| C4 system context diagram | `c4/context` | diagram | ✓ | ✓ | – | – | – |
| C4 container diagram | `c4/container` | diagram | ✓ | ✓ | – | – | – |
| C4 component diagram | `c4/component` | diagram | ✓ | ✓ | – | – | – |
| C4 system landscape diagram | `c4/system-landscape` | diagram | ✓ | ✓ | – | – | – |
| C4 dynamic diagram | `c4/dynamic` | diagram | ✓ | ✓ | – | – | – |
| C4 deployment diagram | `c4/deployment` | diagram | ✓ | ✓ | – | – | – |
| Causal loop diagram | `systems/causal-loop-diagram` | diagram | ✓ | ✓ | – | – | – |
| Databricks bundle | `databricks/bundle` | diagram | ✓ | ✓ | – | – | – |
| Databricks job | `databricks/job` | diagram | ✓ | ✓ | – | – | – |
| Databricks pipeline | `databricks/pipeline` | diagram | ✓ | ✓ | – | – | – |
| Dependency graph | `generic/dependencies` | diagram | ✓ | ✓ | – | – | – |
| Functional decomposition graph | `etalii/functional-decomposition-graph` | diagram | ✓ | ✓ | – | – | – |
| Gartner hype cycle graph | `gartner/hypecycle-graph` | diagram | ✓ | ✓ | ✓ | – | ✓ |
| Helm chart | `helm/chart` | diagram | ✓ | ✓ | – | – | – |
| Mind map | `freeplane/mindmap` | diagram | ✓ | ✓ | ✓ | ✓ | – |
| .NET dependency graph | `dotnet/dependency-graph` | diagram | ✓ | ✓ | – | – | – |
| OWL ontology | `w3c/owl` | diagram | ✓ | ✓ | – | – | – |
| RDF graph | `w3c/rdf` | diagram | ✓ | ✓ | – | – | – |
| Sankey diagram | `etalii/sankey` | diagram | ✓ | ✓ | – | – | – |
| SHACL shapes | `w3c/shacl` | diagram | ✓ | ✓ | – | – | – |
| SKOS concept scheme | `w3c/skos` | diagram | ✓ | ✓ | – | – | – |
| SPARQL query | `w3c/sparql` | diagram | ✓ | ✓ | – | – | – |
| Supply chain diagram | `etalii/supply-chain` | diagram | ✓ | ✓ | – | – | – |
| Timeline diagram | `generic/timeline` | diagram | ✓ | ✓ | – | – | – |
| Wardley map | `wardley/map` | diagram | ✓ | ✓ | – | – | – |
| Hierarchical Requirement Designer | `etalii/hierarchical-requirement` | designer | ✓ | ◐ | – | – | – |
| Knowledge designer | `etalii/knowledge` | designer | ✓ | ✓ | ✓ | – | – |

- **Standalone** runs every tool type here, each as a module under `src/diagrams/` or `src/designers/`. How far each one has come (implemented, prototype or work in progress) is recorded in that host's `docs/tools.md`, the catalogue of record.
- **Desktop** runs the diagrams through 19 crates ported from the standalone host: one crate serves the six C4 types, one the three Databricks types, and one the RDF family (RDF, OWL, SHACL and SKOS). The Hierarchical Requirement Designer is being added by spec 036.
- **VS Code** interprets its five tool types from copies of these definitions in its `definitions/` folder.
- **IntelliJ** ships the mind map as hand-written code (approach 1 of `architecture` *Tool-type approaches*), not from `mindmap.dis`. Its draw.io diagram (`jgraph/drawio`) has no definition here.
- **Notion** publishes one add-on, the Gartner hype cycle graph.
- **Editors** have no definitions here while EDSL is a placeholder. The standalone host ships JSON, Markdown, plain text, XML and YAML editors as host code, and the desktop host ports the same five. Desktop, VS Code and IntelliJ also give `.atd` files language support.

When a host gains or drops a tool type, this table changes with it.
