# Refused tool descriptions

The tool descriptions in this folder **must be refused** when they are checked: each `<name>.atd` gives exactly the findings its `<name>.findings.json` states (`atd.schema.json#/$defs/Findings`), by code, severity, file, line and column, the message not being compared. They are the other half of the examples beside the specification: those check without findings, these show what a checker refuses and where (ATD §15). The expected findings were written by hand from ATD §14 and §15, before any checker existed; a checker that gives others is wrong, not the files.

`python .github/scripts/validate-examples.py` validates every findings file against the schema and fails when a refused `.atd` has none; every one parses but `syntax-error.atd`, which shows a syntax error. The checker is run on the folder with `atd check --corpus specifications/atd/refused` (contracts/checker.md), in the Build workflow's `descriptions` job, and the run fails when a file gives other findings than its own. A file that is part of another's description (`import-cycle-b.atd`, `part-of-other-origin.part.atd`) is checked as its description, so it states the same findings, in the root's or its own file.

| Description | Refused with |
|---|---|
| `action-in-label.atd` | `ATD7002` at action-in-label.atd:11:27: `bump` changes the model (`set` at line 14), so it cannot be called from a label, which only reads. |
| `binding-undeclared-attribute.atd` | `ATD6001` at binding-undeclared-attribute.atd:18:17: The binding reads `owner`, which element type `Task` does not declare. |
| `binding-grammar-conflict.atd` | `ATD8001` at binding-grammar-conflict.atd:29:7: The grammar is not LALR(1): a Reduce/Reduce conflict on $END between `item : NAME ";"` and `other : NAME ";"`, at the head of `item`, the rule in conflict defined first (§11.9). |
| `cel-type.atd` | `ATD4001` at cel-type.atd:11:10: A constraint's rule is a `bool`; this expression is an `int`. |
| `counter-mark-not-string.atd` | `ATD6003` at counter-mark-not-string.atd:20:76: The counter of `Item` keeps its mark in `lastItem`, which is an `int`; a mark is a `string` attribute of the document. |
| `designer-relation.atd` | `ATD3001` at designer-relation.atd:10:5: A designer has no relation types; `relation Broader` is admitted in diagrams only. |
| `editor-canvas.atd` | `ATD3002` at editor-canvas.atd:14:3: An editor has a text surface; `notation` belongs to the canvas of a diagram. |
| `hierarchy-label-not-on-type.atd` | `ATD6002` at hierarchy-label-not-on-type.atd:23:5: The label attribute `tags` is not an attribute of `Requirement`, which its `on` names. |
| `host-attribute-unknown.atd` | `ATD2001` at host-attribute-unknown.atd:15:32: No host attribute `storedID` in `persistence`'s `typeMap`. Did you mean `storedId`? |
| `import-cycle-a.atd` | `ATD2003` at import-cycle-b.atd:4:1: The imports `import-cycle-a.atd` → `import-cycle-b.atd` → `import-cycle-a.atd` form a cycle. |
| `import-cycle-b.atd` | `ATD2003` at import-cycle-b.atd:4:1: The imports `import-cycle-a.atd` → `import-cycle-b.atd` → `import-cycle-a.atd` form a cycle. |
| `missing-return.atd` | `ATD7003` at missing-return.atd:10:6: Not every path of `grade` returns a `string` or fails: the path where `value >= 50` is false ends without one. |
| `part-of-other-origin.atd` | `ATD2007` at part-of-other-origin.part.atd:2:9: This part says it is part of `example/two`, but `part-of-other-origin.atd` imports it into `example/one`. |
| `part-of-other-origin.part.atd` | `ATD2007` at part-of-other-origin.part.atd:2:9: This part says it is part of `example/two`, but `part-of-other-origin.atd` imports it into `example/one`. |
| `recursion-over-64.atd` | `ATD7004` at recursion-over-64.atd:10:41: A function recurses at most 64 frames deep; `recursion 100` asks for more. |
| `script-clock.atd` | `ATD7001` at script-clock.atd:11:29: `now` reads the clock; a script is deterministic and calls only CEL's library and the description's functions. |
| `setting-without-block.atd` | `ATD6004` at setting-without-block.atd:19:17: The registration of binding `outline` keeps no block `collapsed`, which `persistence` names among its settings. |
| `settings-schema.atd` | `ATD5001` at settings-schema.atd:11:35: `thickness` is not a setting of a stroke (DISL `$defs/Stroke`). Did you mean `width`? |
| `syntax-error.atd` | `ATD1001` at syntax-error.atd:7:22: Found `required` where `:` and the attribute's type were expected. |
| `typemap-unreadable-attribute.atd` | `ATD6001` at typemap-unreadable-attribute.atd:40:17: `Unreadable` is mapped as unreadable, so its rule `unreadable-entry` binds no attribute; it binds `title`. |
| `unknown-attribute.atd` | `ATD2001` at unknown-attribute.atd:12:23: No attribute `titel` in element type `Task`. Did you mean `title` or `tier`? |
| `unknown-member.atd` | `ATD2001` at unknown-member.atd:13:15: No attribute `titel` in element type `Task`. Did you mean `title`? |
| `unknown-style.atd` | `ATD2011` at unknown-style.atd:13:14: No style `crad`. Did you mean `card`? |
| `unknown-binding-rule.atd` | `ATD2004` at unknown-binding-rule.atd:14:14: No rule `iten` in binding `outline`. Did you mean `item`? |
| `unknown-capability.atd` | `ATD2010` at unknown-capability.atd:5:23: No capability `layout.radialTree` in the catalogue. Did you mean `layout.twoSidedTree`? |
| `unknown-element-type.atd` | `ATD2002` at unknown-element-type.atd:11:10: No element type `Tsak`. Did you mean `Task`? |
| `unknown-function.atd` | `ATD2005` at unknown-function.atd:11:10: No function `nonBlank`. Did you mean `notBlank`? |
| `unknown-version.atd` | `ATD0001` at unknown-version.atd:1:5: This description is written in ATD 9.4, which this reader does not know; it reads ATD 0.1. |
