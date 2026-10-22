# Brightwater Coffee Co.

The income statement of a fictional coffee company for fiscal year 2026, drawn the way quarterly results are usually shown: revenue by segment on the left, then gross profit and the cost of revenue, operating profit and operating expenses, and finally net profit and tax, with the operating expenses broken down into marketing, research and development, and general and administrative costs.

## Authored for this repository

**This example was written for ADP.** The Sankey diagram's `.skv` format is this repository's own, so no published corpus of `.skv` documents exists to vendor. The company is invented, as the user asked: *"Use a fictional company as an example."* Its look follows the two income-statement Sankeys the user supplied as the reference style.

**Every figure is invented.** The amounts add up (segments to revenue, revenue to gross profit and cost, and so on) but describe no real company and must not be quoted as such.

## What it shows

| Part | In this example |
|---|---|
| Nodes | 14, in five columns decided by the flows alone |
| Flows | 13, every one coloured by the node it reaches (`flow-color: target`) |
| Value format | `€{value}M` for the document, and `(€{value}M)` on the cost nodes so a cost reads as a deduction |
| Notes | A year-on-year change under each segment and revenue, a margin under each profit |
| Colours | Four palette words: blue segments, grey revenue, green profits and red costs |

## What it does not demonstrate

- **A custom colour.** Every node uses a palette word; a `#rrggbb` colour is set in the property grid.
- **A stated column.** Every node is placed by its flows.
- **A cycle or a backward flow.** An income statement has neither.
- **A document that breaks the rules.** The broken documents are the test fixtures under `../../backend/EtAlii.Adp.Diagram.Sankey.Tests/Fixtures/`.
