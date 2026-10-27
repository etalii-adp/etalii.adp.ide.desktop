# yaml - examples

What the YAML editor is for, one guarantee or one behaviour per file:

- `service.yaml` - names a schema of the project, `service.schema.json`, and satisfies it: nothing is marked, and completion offers what the schema allows.
- `service-broken.yaml` - breaks that schema in the ways the editor marks: a value of the wrong type, a value outside its list, a deprecated key, a key the schema does not allow, a sequence item missing its required key, a missing key on the mapping, and a value that is wrong only where an alias brings it in.
- `anchors.yaml` - anchors, aliases and a merge key, read as the data they stand for and never written back.
- `no-schema.yaml` - no schema line: colour, YAML's own mistakes and nothing else. Its plain scalars (`1.10`, `yes`, `~`, `0x1f`, a date) are coloured as the type YAML 1.2 reads them as.
- `crlf.yaml` - Windows (CRLF) line endings, which open-and-save must keep.
- `several-documents.yaml` - three documents in one file, each with its own schema line or none, and a `---` inside a block scalar that is not a marker.
- `service.schema.json` - the schema the others name. It is a JSON file and opens in the editor for JSON; it is here because a schema is read from the project, never from the network.

The module's tests open every file in this folder, round-trip it byte for byte, and hold a byte-identical replica under `src/examples/editors/yaml/`.

What these files do **not** show: a schema written in YAML (the tests hold that case), YAML 1.1 readings, and templated YAML such as Helm's, which is not YAML until rendered.
