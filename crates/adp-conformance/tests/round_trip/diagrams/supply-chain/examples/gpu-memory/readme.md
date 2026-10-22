# GPUs and memory

The industry behind graphics processors and memory as it stands today, with its suppliers and its consumers: rare earths, gallium, high-purity quartz, neon and process chemicals at one end; data centres, households and PC gamers at the other; and the wafer makers, lithography, logic and memory fabs, HBM stacking, advanced packaging and the assembly of phones, laptops, graphics cards and GPU servers in between.

## Authored for this repository

**This example was written for ADP.** The supply chain diagram is a notation of this repository's own, so no published corpus of `.supply` documents exists to vendor. The subject is the one the user gave: *"the current GPU/RAM industry (plus suppliers/consumers, e.g. rare earth metals, chemicals, smartphones, smaller electronics, computers/laptops etc.)"*.

**The figures are illustrative, not sourced.** Quantities and volumes are of a plausible order of magnitude so that the flow bands have a believable spread of widths. They are not market data and must not be quoted as such. No company is named; each node is a kind of plant in a region.

## What it shows

| Part | In this example |
|---|---|
| Groups | Eight: China's mining and refining, the Americas, Europe, Japan, Taiwan, South Korea, device assembly and the markets |
| Stages | All seven |
| Flows | 38, several fanning out from one fab to phones, laptops, graphics cards and servers |
| Shared dependencies | One wafer maker, one photoresist supplier and one lithography supplier feed both the logic and the memory fabs |
| Steps | From 0.01 million wafers a month to 10 million devices |

Select the HBM stacking node to see that AI data centres depend on it while households do not. Select the lithography tools to see how much of the whole chain hangs on them.

## What it does not demonstrate

- **Authored positions.** The whole diagram is placed by the layout.
- **A cycle**, such as recycled metals returning to the refiners.
- **A node outside every group**, or an empty group.
- **A document that breaks the rules.** The broken documents are the test fixtures under `../../backend/EtAlii.Adp.Diagram.SupplyChain.Tests/Fixtures/`.
