# Field service

A mobile app that helps field service employees tick off their planning and the steps of each task, drawn as a functional decomposition graph: what the employee sees, what they can do, what the app keeps, and what runs behind the screens.

## Authored for this repository

**This example was written for ADP, and it is exempt from the published-data rule by the user's ruling.** The rule is that diagram modules are tested against real published example data rather than hand-written toys. It cannot apply here: the functional decomposition graph is a new notation, so no published corpus of it can exist. The subject is the one the user gave - *"the workflow of a mobile application that helps field service employees tick of their planning and per-task steps"* - and nothing in it is derived from, or attributed to, any external source.

That exemption has a cost worth stating: a hand-written example tests what its author had in mind. Nothing here surprised the parser or the rules the way a real corpus would.

## What it shows

| Part | In this example |
|---|---|
| UI Elements | Planning, owning a Task list with a Task row, and a Task detail with a Step list and a Step row |
| Actions | Open task, Back to list, Tick step, Complete task |
| Data Elements | Planning day, owning a Task, which owns a Step and a Location |
| Functions | Sync queue, which calls Upload photos; Offline cache |
| Comments | Two, one of them explaining the navigation loop below |

**Every relation appears at least once.** That covers UI child, Action, Data, Function and Shows, including a Function owning a Function (Sync queue calls Upload photos) and Data Elements nested two deep (Planning day, Task, Step).

**The navigation round trip is here, and it is legal.** Task list holds a Task row, which offers Open task. Open task **shows** Task detail, which offers Back to list, and Back to list **shows** Task list again. That is a loop, but it closes only through Shows. Shows is navigation, not ownership, so the cycle rule does not apply to it. Ownership on its own is a set of strict trees.

There are named connections (*opens*, *returns to*, *calls* and others) and Descriptions on elements and on connections. A Description is kept in the document but never drawn on the canvas.

## What it does not demonstrate

- **A document that breaks the rules.** Every rule holds here, and the validator reports nothing for it. The broken documents are the test fixtures under `../../backend/EtAlii.Adp.Diagram.FunctionalDecompositionGraph.Tests/Fixtures/`, one per rule.
- **More than one Shows from one Action.** The rules forbid it, so no correct example can show it.
- **Base36 ids.** Every id here is readable, like `task-detail`.
- **An empty Name** on an element that takes one.
- **A Data Element owned by an Action, or a Function owned by a UI Element.** Both are allowed, but neither fitted this app without inventing elements beyond the ones the design lists.
- **A Description on a Comment.**
- **An Action showing its own page.** The only Shows here go to another page.
