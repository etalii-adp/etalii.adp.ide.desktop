# json - examples

Hand-written for this module; nothing here is taken from elsewhere.

- `service.schema.json` - a JSON Schema (draft 2020-12) for a service's settings: required properties, an enumeration, a deprecated property, descriptions on every property and a nested object. Opened on its own it is a document whose `$schema` is a dialect, so it is checked as a schema.
- `service.json` - satisfies the schema beside it. Open it to see colour, completion (Ctrl+Space inside the object) and the schema's title in the status line.
- `service-broken.json` - breaks the same schema in the ways the specification names: an empty name, a port that is a string, a log level the enumeration does not allow, the deprecated `verbose` (a warning), a repeated array item, a nested object without its required property and with a negative value, and a property the schema does not allow. Its `$schema` property is not in the schema's properties either, and the editor does not report that.
- `no-schema.json` - names no schema: colour and the mistakes of JSON itself, and the status line says "No schema".
- `crlf-settings.json` - every line ends in CRLF.
- `utf8-bom-settings.json` - starts with a UTF-8 byte-order mark, holds characters outside ASCII and has no final newline.

The module's tests open every file in this folder and save it unchanged, and require the bytes back; `.gitattributes` keeps git from rewriting the line endings of the `.json` files here, so the LF files stay LF and the CRLF file stays CRLF on every machine. The combined project holds a byte-identical replica under `src/examples/editors/json/`.

What these examples do not show: a `$ref` to a second schema file, a schema in another dialect than 2020-12, and a document large enough to reach the size limit. The tests of `jsonAnalysis` cover those with documents of their own.
