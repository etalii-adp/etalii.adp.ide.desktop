# xml - examples

- `library.xml` with `library.xsd` beside it, named by `xsi:noNamespaceSchemaLocation`: a document that satisfies its schema. The schema itself is an example too: opened, it is checked as a schema.
- `library-broken.xml`: the same library broken against `library.xsd` in each way the editor reports - a missing required attribute, a missing required child, a value that does not fit its type, and an element the schema does not allow there.
- `recipe.xml` with `recipe.dtd`, named by its `DOCTYPE`, with an internal entity: validated against the DTD, with no completion.
- `order.xml` with `order.xsd` and `party.xsd`: two namespaces, each with its own schema, named in pairs by `xsi:schemaLocation`; `order.xsd` imports `party.xsd`.
- `notes.xml`: no schema - a comment, a processing instruction, entity references and a CDATA section, for colour, folding and well-formedness.
- `latin1.xml`: declared and stored as ISO-8859-1. Its characters are the ASCII range ISO-8859-1 shares with UTF-8, because the family's text buffer opens UTF-8 and ASCII only: a Latin-1 byte above 127 is refused rather than opened.

**What these do not demonstrate.** No XSLT is run, and the module's `.xsl`/`.xslt` claim has no example. Nothing here names RELAX NG, Schematron or XSD 1.1; the tests build those cases themselves. All of these were written for this module; none is vendored.

The module's tests open every file in this folder, so the examples cannot drift from what the code supports.
