# plain - examples

Two files, one guarantee each:

- `crlf-notes.txt` - Windows (CRLF) line endings, which open-and-save must keep.
- `utf8-bom-notes.txt` - a UTF-8 byte-order mark, written back exactly when - and
  only when - the file arrived with one.

Beyond these, any text file is this editor's material: it is the fallback that
answers for whatever no other editor claims. The module's tests open every file
in this folder, so the examples cannot drift from what the code supports.
