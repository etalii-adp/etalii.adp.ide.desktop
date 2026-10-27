# Editor definitions

This folder holds the files for ADP's editor types, in which typing text is the core interaction.

## The tool descriptions

Each of the five editors the hosts implement is described in ATD, the ADP tool language ([ATD](../../specifications/atd/ATD-specification.md), section 10), in one `.atd` file beside this README:

| Description | Origin | Editor | What it states |
|---|---|---|---|
| [plain-text.atd](plain-text.atd) | `editor/plain` | Plain text editor | Every file no other editor takes, opened and saved byte for byte, coloured by its extension's syntax |
| [markdown.atd](markdown.atd) | `editor/markdown` | Markdown editor | The outline of the ATX headings (the capability `reader.markdown`) and the preview (the capability `render.markdown`) |
| [json.atd](json.atd) | `editor/json` | JSON editor | The mistakes of JSON, the JSON Schema its root's `$schema` names, and completion from that schema |
| [xml.atd](xml.atd) | `editor/xml` | XML editor | Well-formedness, the declared encoding, and validation against the XSD or DTD the document names |
| [yaml.atd](yaml.atd) | `editor/yaml` | YAML editor | The mistakes of YAML per document of a stream, and the JSON Schema a `yaml-language-server` comment names |

The origins and display names are those of the desktop host's catalogue. Each description is written from what the desktop host's editor does at its reference commit `fe5feec`, which ports the standalone host's (ATD section 10.1); what every editor shares (how a file is opened and saved, the buffer, where a finding is said) is stated in each.

**No host loads these descriptions** (spec 035, FR-031). The hosts open these files with their own editors, written in code, and run no editor from `.atd` in phase 1 of the move to ATD (ATD section 1.2). The descriptions are checked by `atd check` in the Build workflow, and their conformance cases say what a host that ran them would have to do.

Beside each description:

- `<name>.trace.json` is its trace (ATD section 2.5): every behaviour read in the desktop host's code, with the file and lines at `fe5feec`, and the part of the description that states it or the host's concern it is. The editors have no companion file, since no EDSL specification exists for them; the trace names ATD section 10.1, the record of what the editors do, as its companion, which holds no items.
- `conformance/<name>/<case>/` are its conformance cases (ATD section 2.3), one per document of the editor's examples, which the desktop host's conformance tests round-trip (`crates/adp-conformance/tests/round_trip/editors/`, the standalone host's examples). Each holds every file of that folder byte for byte, so that a schema a document names is there, and what the editor read and the findings it gave, at line and column. They are recorded by `atd record editor/<name> --cases conformance/<name> --description ../../../<name>.atd` and never edited by hand; Git keeps every file there as it is (`.gitattributes`).

## EDSL and EDD

These are the files an editor type will be specified in once an editor needs them:

- **EDSL specification files** (`.eds`), in which a tool engineer specifies one editor type ([EDSL](../../specifications/edsl/EDSL-specification.md));
- **EDD definition files** (`.edd`), each one editor a user created of such a type ([EDD](../../specifications/edd/EDD-specification.md)).

EDSL and EDD are placeholders. The words used here are defined in [docs/terminology.md](../../docs/terminology.md).
