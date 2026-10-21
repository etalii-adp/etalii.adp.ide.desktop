# C4 examples

Two ADP projects you can open and click around in. They are not fixtures — the corpus under
`../backend/EtAlii.Adp.Diagram.C4.Tests/Fixtures/` exists to pin parser behaviour, one construct
at a time, and reads like it. These read like architecture.

| Project | What it is for |
|---|---|
| [`reference/`](reference/) | Every construct ADP's C4 support understands, in one model. Read it to find out what is supported and how to write it. |
| [`industrial-plant/`](industrial-plant/) | A manufacturing execution system for a bottling plant. Read it to see the notation used on a problem with real constraints — an isolated control network, a plant that cannot stop, and equipment nobody deploys. |

## Opening one

Open the **project folder** — `reference/` or `industrial-plant/`, not the `architecture/`
folder inside it. `body:` headers in the `.adp` files are resolved against the project root, and
they name paths like `architecture/courier.dsl`.

Every `.adp` file in the explorer is a diagram. Open several: they are views of one model, so
renaming an element in one view renames it everywhere, and the change lands in the `.dsl` as a
one-line diff.

## What is in a project

```
reference/
  architecture/
    courier.dsl              the model - one document, every view declared in it
    courier.layout.json      positions someone dragged, per view; ADP's own file
    courier.adp              a diagram: c4/context, and the owner of courier.dsl
    containers.adp           a diagram: c4/container over the same model
    ...                      one .adp per view
    code-level.adp           c4/code, which has no canvas and says so
    docs/                    prose, pointed at by !docs
    decisions/               ADRs, pointed at by !adrs
```

**The `.dsl` is the model.** It is Structurizr DSL — a format ADP does not own — so ADP edits it
by the line and keeps everything it does not model, comments included, byte for byte.

**The `.adp` files are the diagrams.** Each is three lines at most: a MIME type, an optional
`body:` naming the model it opens, and an optional `view:` naming which view within it.

**Exactly one `.adp` per project owns the model** — the one whose name matches it
(`courier.adp` ↔ `courier.dsl`). It has no `body:` header, because the body is derived from its
own name. Deleting or renaming *that* file takes the model with it; deleting any of the others
removes only that diagram. This is worth knowing before you tidy up a project.

**The `.layout.json` is ADP's.** Positions belong to a view, and views belong to a model, so one
sidecar sits beside the `.dsl` and holds them all. It is an optimisation and never a dependency:
delete it and every diagram still opens, laid out by the engine instead.

## Both models are certified, not just plausible

Each `.dsl` is checked by the real Structurizr CLI:

```bash
java -cp "<structurizr-cli>/lib/*" com.structurizr.cli.StructurizrCliApplication validate -workspace reference/architecture/courier.dsl
```

`validate` accepts both, and `inspect` — Structurizr's own model-quality rules — reports **nothing**
on either. So these are not merely files ADP can read; they are models the C4 ecosystem considers
well formed. `ExamplesTests` in the C4 test project keeps them that way from ADP's side: every
model parses and round-trips, every `.adp` resolves to a view that exists, every `body:` header
resolves inside its project, and nothing breaks a C4 rule.
