# Courier Tracking — the reference project

Open this folder in ADP.

A parcel tracking system, small enough to hold in your head and wide enough to use every C4
construct ADP supports. If you want to know whether something is supported and how to write it,
it is in [`architecture/courier.dsl`](architecture/courier.dsl) with a comment saying why.

## The seven diagrams

| File | Type | Shows |
|---|---|---|
| [`courier.landscape.adp`](architecture/courier.landscape.adp) | `c4/system-landscape` | Every system in the estate, with no single one in focus |
| [`courier.adp`](architecture/courier.adp) | `c4/context` | Courier Tracking, its people, and what it depends on |
| [`courier.containers.adp`](architecture/courier.containers.adp) | `c4/container` | The applications and stores inside it, and the protocols between them |
| [`courier.api-components.adp`](architecture/courier.api-components.adp) | `c4/component` | Inside the API application |
| [`courier.parcel-scanned.adp`](architecture/courier.parcel-scanned.adp) | `c4/dynamic` | What happens, in order, when a courier scans a parcel |
| [`courier.production.adp`](architecture/courier.production.adp) | `c4/deployment` | Where Production runs, with instance counts |
| [`code-level.adp`](architecture/code-level.adp) | `c4/code` | The one type with no canvas — it explains itself instead |

Open `courier.adp` and `courier.containers.adp` at the same time and rename a container in either. The
other updates, because both are views of one document.

## What each construct in the model is there to show

**People, systems, containers, components** — the four static levels, nested three deep
(`tracking` → `api` → `bookingController`). A component may only live inside a container; the
model is written so that constraint is visible.

**`group`** — a label over elements rather than a container. Survives the round trip and is drawn
as a boundary by tools that support it.

**Tags** — `"Existing System"` marks the three systems that were already there, and ADP draws
them muted. `"Database"` and `"Queue"` drive the cylinder and pipe shapes. `"Staff"` is a
custom tag styled in the `styles` block, to show tags are not a fixed list.

**Relationships at every level** — person→system, system→system, person→container,
container→container, component→component, and component→external-system. Every one names its
technology. ADP only *warns* about a missing technology between containers, but Structurizr
inspects for one everywhere, and this model is written to satisfy the stricter of the two.

**`deploymentEnvironment`** — with nodes nested four deep, an `infrastructureNode`, container
instances, and instance counts (`"web-*" ... "" 3`). The count sits *after* the tags, which is
the argument order most easily got backwards. Note that replication is drawn between the two
database *servers*, not between the two container instances on them: a relationship between
instances is refused, because an instance inherits the relationships of what it deploys.

**Views** — all six kinds ADP draws, plus `c4/code` which it does not. `autoLayout` appears with
a direction (`tb`, `lr`) and bare. `ApiComponents` uses `exclude` after `include *` to take one
element back out.

**`styles`** — the document's own palette. C4 is notation-independent, so the palette belongs to
the document rather than to ADP, and ADP reads this block rather than imposing colours.

**Things ADP keeps but does not read** — `!identifiers`, `!docs`, `!adrs`, `configuration`, and
comments in every position (`//`, `#`, `/* */`, before the workspace, inside blocks, between
elements). All of it round-trips byte for byte. That is the guarantee that makes it safe to point
ADP at a `.dsl` somebody else wrote.

## Where the prose lives

[`architecture/docs/`](architecture/docs/) and [`architecture/decisions/`](architecture/decisions/)
are pointed at by `!docs` and `!adrs`. ADP has no documentation model yet, so it carries them
untouched; Structurizr renders them beside the diagrams. They are included because a reference
project that shows only boxes would be missing the half that explains them.
