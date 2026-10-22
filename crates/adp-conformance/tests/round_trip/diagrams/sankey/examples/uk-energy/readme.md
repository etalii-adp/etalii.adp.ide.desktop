# UK energy

A possible UK energy system in 2050, in terawatt-hours a year: where the energy comes from (imports, reserves, nuclear, wind, sun, waves and biomass), what it is converted into (solid, liquid and gaseous fuels, thermal generation, the electricity grid, hydrogen) and where it ends up (industry, homes, commerce, transport, agriculture, exports and losses).

## Source

**The data is the plotly.js test mock [`test/image/mocks/sankey_energy.json`](https://github.com/plotly/plotly.js/blob/master/test/image/mocks/sankey_energy.json)**, vendored on 2026-10-04. The file states no licence of its own, so the plotly.js repository's licence applies: the MIT License, vendored verbatim beside this file as `LICENSE.md`. The mock is plotly's copy of the energy example of Mike Bostock's [d3-sankey](https://github.com/d3/d3-sankey), whose figures come from the UK Department of Energy and Climate Change's 2050 Calculator.

**Two changes were made in converting it.** The mock splits *Nuclear → Thermal generation* into four links, two of them labelled "made-up" to exercise plotly's link labels; they are one flow here, with their sum, 839.978 TWh, which is the original d3 figure. Each node's id is its label in lower-case words joined by hyphens. The colours are new: plotly cycles its default palette through the nodes, and this example colours them by energy carrier instead, with the flows taking their source's colour (`flow-color: source`).

## What it shows

| Part | In this example |
|---|---|
| Nodes | 48, in columns decided by the flows alone |
| Flows | 68 |
| Converging and diverging flows | Thermal generation gathers nuclear, gas and solid fuel and splits into the grid, district heating and losses |
| Long and short paths | Demand is reached both in one step (pumped heat to homes) and in five (coal reserves to industry, through coal, solid fuel, thermal generation and the grid) |
| Thin flows | Several under 1 TWh beside several over 500, so the minimum band thickness is visible |

## What it does not demonstrate

- **Notes, value formats per node or custom colours.** Every node is a label, a value and a palette word.
- **A cycle.** The data has none.
- **The figures being current.** They are one scenario of a 2050 calculator, not measurements.
