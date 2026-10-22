# Automotive

A European car maker building both electric and combustion vehicles, traced from the mines to the people who drive them: lithium, cobalt, nickel, copper, iron ore and natural rubber at one end, private buyers and company fleets at the other, and the refineries, cathode and cell plants, tier-1 suppliers and the assembly plant in between.

## Authored for this repository

**This example was written for ADP.** The supply chain diagram is a notation of this repository's own, so no published corpus of `.supply` documents exists to vendor. The subject is the one the user gave: *"the automotive industry"*.

**The figures are illustrative, not sourced.** Quantities and volumes are of a plausible order of magnitude for one large manufacturer in one year, so that the flow bands have a believable spread of widths. They are not market data and must not be quoted as such.

## What it shows

| Part | In this example |
|---|---|
| Groups | Seven regions, from South America to Europe |
| Stages | All seven: sources, processors, producers, integrators, a hub, outlets and consumers |
| Flows | 25, from ore to cars, each with a product, a volume, a unit and a step |
| Converging chains | The battery chain and the body, electronics, seating and tyre chains all meet at the vehicle plant |
| Steps | Each node and flow states its own step, from 0.1 million vehicles to 100 kt of copper |

Select the vehicle plant to see every mine that feeds it light up upstream and both kinds of buyer downstream. Select the Kolwezi cobalt mines to see which cars depend on them.

## What it does not demonstrate

- **Authored positions.** No node states `x` or `y`, so the whole diagram is placed by the layout. Dragging a node writes its position.
- **A cycle.** Recycling would close the loop from buyers back to the refinery; the layout breaks cycles, but this example has none.
- **A node outside every group**, or an empty group.
- **A document that breaks the rules.** The broken documents are the test fixtures under `../../backend/EtAlii.Adp.Diagram.SupplyChain.Tests/Fixtures/`.
