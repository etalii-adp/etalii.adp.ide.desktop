# Northgate Bottling MES — an industrial architecture

Open this folder in ADP.

A manufacturing execution system for a beverage bottling plant: three filling lines, a warehouse,
and SAP above it all. Where [`reference/`](../reference/) is written to show every construct, this
one is written to show the notation carrying its weight on a problem with awkward, real
constraints.

## Read the deployment diagram first

That is unusual advice, and it is the point of this example.

Most systems can be understood from the container diagram. This one cannot, because the
constraint that shapes every decision does not appear there: **the control network is isolated,
and exactly one container is allowed to cross into it.**

The plant follows the Purdue model. Levels 1–2 — the PLCs and SCADA that physically move fillers,
cappers and palletisers — sit on an air-gapped VLAN with no route to the enterprise network. The
only path across is the plant DMZ, where the OPC UA Gateway runs, and the industrial firewall
permits exactly one thing: OPC UA outbound, initiated by the gateway. Nothing initiates inbound.

[`bottling-mes.plant-deployment.adp`](architecture/bottling-mes.plant-deployment.adp) is where that is visible. The
`"OT"` and `"DMZ"` tags are styled so the zones read at a glance.

## The six diagrams

| File | Type | Shows |
|---|---|---|
| [`bottling-mes.plant-landscape.adp`](architecture/bottling-mes.plant-landscape.adp) | `c4/system-landscape` | Every system on the site, OT and IT together |
| [`bottling-mes.adp`](architecture/bottling-mes.adp) | `c4/context` | The MES, the four roles that use it, and the six systems around it |
| [`bottling-mes.mes-containers.adp`](architecture/bottling-mes.mes-containers.adp) | `c4/container` | What the MES is made of, and which part crosses into the control network |
| [`bottling-mes.order-service-components.adp`](architecture/bottling-mes.order-service-components.adp) | `c4/component` | Inside the Order Service |
| [`bottling-mes.batch-release.adp`](architecture/bottling-mes.batch-release.adp) | `c4/dynamic` | Releasing a batch, from last pallet to ERP confirmation |
| [`bottling-mes.plant-deployment.adp`](architecture/bottling-mes.plant-deployment.adp) | `c4/deployment` | Three zones and the firewall between them |

## Three modelling decisions worth arguing about

**SCADA and the PLCs are a separate software system, not containers.** This is the one people
push back on. A container is something *you* deploy; the control layer is Ignition and a rack of
Siemens S7s that predate the MES by a decade and will outlive it. Changing ladder logic is a
validated change with a safety case attached. Drawing it inside the MES boundary would claim the
MES team owns it, and they do not.

**The component view names its includes instead of using `include *`.** A component diagram is
about one container. On a model this size `*` drags in every container in the plant — the andon
board has nothing to say about how a works order is sequenced. Listing what belongs is the
difference between a component diagram and a picture of everything.

**Everything the line needs is on premises.** The plant data centre exists because the line does
not stop when the WAN does; the reasoning, and the incident behind it, are in
[`decisions/0002`](architecture/decisions/0002-keep-the-plant-running-without-the-corporate-network.md).
Only two integrations may depend on the WAN, and both degrade rather than stop.

## Where the prose lives

[`architecture/docs/`](architecture/docs/) explains why the deployment view comes first and why
SCADA is not a container. [`architecture/decisions/`](architecture/decisions/) holds the two ADRs
that shaped the model: one gateway out of the control network, and a plant that keeps running
without the corporate network.
