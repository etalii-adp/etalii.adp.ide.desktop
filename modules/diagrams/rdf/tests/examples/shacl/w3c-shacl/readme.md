# W3C SHACL specification examples

Example shapes copied from the SHACL specification itself — the source that defines what a
shapes graph *is*, and the one corpus guaranteed to use each construct the way the language
intends rather than the way one vendor happened to.

- **Source**: <https://www.w3.org/TR/shacl/> — *Shapes Constraint Language (SHACL)*,
  W3C Recommendation 20 July 2017.
- **Retrieved**: 2026-09-04.
- **License**: W3C Software and Document License, 2023 version. Re-verified at acquisition
  against <https://www.w3.org/copyright/software-license/>, which redirects to the 2023
  version; the license text is vendored beside this readme as `LICENSE`.
- **Notice of modification** (the license requires changes to be stated): this file and the
  header of `spec-examples.ttl` are the modifications. Three example blocks were copied
  verbatim from the specification and concatenated into one document, and a five-line prefix
  header was added because the specification states its prefixes in prose rather than
  repeating them in every example. No triple was altered, reordered within a block, or
  reformatted — the specification's own tab indentation is preserved byte for byte.

## What the blocks are, and why these three

| Block | From | Why it is here |
| --- | --- | --- |
| `ex:ExampleNodeShapeWithPropertyShapes`, `ex:ExamplePropertyShape` | §2.1 property shapes | Blank-node property shapes beside an IRI-named one, and a **sequence path** `(ex:knows ex:email)` — the reading prints it as `ex:knows/ex:email` rather than expanding its plumbing |
| `ex:OrConstraintExampleShape` | §4.6.1 `sh:or` | A **logical combinator** whose operands are blank, and a `sh:targetNode` — a second target kind |
| `ex:PersonAddressShape` | §4.6.1 `sh:or` | `sh:or` nested *inside* a property shape, with `sh:class` — the constraint summary case |

Together they cover what shacl-diagram Requirement 9.4 asks one example to exercise: blank-node
property shapes, targets of at least two kinds (`sh:targetClass` and `sh:targetNode`), a logical
combinator, and a complex path.

## What this file is not

It is not a validator fixture. Nothing here is executed against data — this module draws shapes
and never runs them (Requirement 4). The targets name classes no file here describes, which is
the normal case for a shapes graph and is drawn as a plain statement rather than reported as a
problem.

## `shacl-shacl.ttl` — the shapes that validate shapes graphs

Appendix C of the same recommendation, vendored **byte-for-byte unmodified**: it declares its
own prefixes, so unlike `spec-examples.ttl` it needed no header and carries no local change at
all. Same source, same retrieval date, same license.

It is here because it is the largest real shapes graph the specification itself publishes — 405
lines, ten node shapes, sixty-odd property shapes — and because of what it is *about*. These are
the shapes W3C wrote to check that a shapes graph obeys SHACL's own syntax rules, so opening it
in this reading draws the very structures it constrains. A card here is a shape describing what
a shape may look like.

It also earns its keep as a test of this module's vocabulary table. The file uses close to every
term SHACL Core defines, and the validator's unknown-term rule — the one that catches the typo
that silently disables a constraint — reports nothing on it. A term the recommendation uses that
the table had missed would have surfaced here as a warning rather than in somebody's real file.
