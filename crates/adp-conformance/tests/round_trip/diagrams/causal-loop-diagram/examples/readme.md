# Causal loop examples

Three ADP projects you can open and click around in. They are not fixtures — the fixtures under
`../backend/EtAlii.Adp.Diagram.CausalLoop.Tests/` exist to pin parser behaviour one construct at
a time, and read like it. These read like systems.

| Project | What it is for |
|---|---|
| [`reference/`](reference/) | Every construct ADP's causal loop support understands, in one diagram. Read it to find out what is supported and how to write it — and to see a loop whose label the arithmetic disagrees with. |
| [`on-call/`](on-call/) | Why an on-call rotation gets worse on its own. Read it to see the notation carrying a problem you probably recognise. |
| [`business-philosophy/`](business-philosophy/) | Whether a Business Philosophy study is applicable at a university of applied sciences: experts, philosophers, practical tools, core values, latent wisdom, stories, wicked problems, insight over solutions, and art as expression. Read it to see a larger model - eighteen variables, ten loops - where every cycle is named and the arrangement is authored. |

## Opening one

Open the folder. Each project is a `.adp` registration beside its `.cld` body, and the `.adp` is
what appears in the explorer as a diagram. Nothing needs configuring first.

## Authored for this repository

**All three examples were written for ADP.** None is derived from, nor attributed to, any external
source. That is the honest-fallback path Requirement 11 leads with, and it is where this task
landed after checking five candidate sources and rejecting every one on licence grounds:

| Source | Why it was rejected |
|---|---|
| `github.com/nocomplexity/causalloopdiagram` | GPL-3.0 — share-alike |
| `github.com/elbazjosh/AutoCLD` | No licence file at all, which reserves all rights |
| The Wikipedia article and its figures | CC BY-SA — share-alike |
| The MetaSD model library | Per-model author permission rather than a named licence, and Vensim stock-and-flow models rather than causal loop diagrams |
| `github.com/bear96/System-Dynamics-Bot` | CC BY-NC-4.0 — the NonCommercial term fails the same test the share-alike ones did |

The last was found by a fresh search when this task ran, and is the only candidate carrying an
actual corpus of causal loop diagrams rather than a tool. Its licence rules it out. AutoCLD's
*algorithm* is cited in the design — Johnson's elementary cycles, which is published
independently of that repository — but none of its content is used here.

## The label disagreement, in both directions

The whole point of this diagram type in ADP is that a loop's `R`/`B` label is **checkable**: a
loop is reinforcing when it runs through an even number of negative links, and balancing when the
count is odd. ADP counts, reports the disagreement, and changes nothing — the author's label may
be the intent and an arrow may be the mistake, and the tool is not entitled to guess which.

Both examples carry a case, and they are different cases on purpose:

- **`reference/` disagrees deliberately.** `R3` claims reinforcing where three negative links
  make it balancing. It is there so a reader can see the finding without writing a broken file
  themselves. A test pins it, so a future edit cannot quietly "fix" it and leave this page
  describing something the file no longer does.
- **`on-call/` agreed only after the tool corrected the author.** Its third loop was first
  written as `B3` — "automation would help, if there were time", the obvious reading of a
  balancing loop being starved. ADP reported the disagreement, and it was right: two negative
  links is an even count, so that path is *reinforcing*. The exit does not merely fail to open;
  the busier the team gets, the faster it closes. The loop is now `R3`, and the model says
  something truer than the author first believed.

The second case is the better advertisement, and it happened by accident while writing these
files.

## Every cycle named

`business-philosophy/` is the larger case. Its twenty-eight links form exactly ten cycles, and
each is claimed by a named loop whose label agrees with its arithmetic: eight reinforcing, two
balancing. So it opens with no finding at all - not an unlabelled cycle, not a disagreement - and
a test holds it there, because in a model this dense one added link quietly creates a cycle
nobody named. Its one variable outside every loop, the weight of wicked problems, is outside on
purpose: the study does not claim to solve them, only to understand them better.
