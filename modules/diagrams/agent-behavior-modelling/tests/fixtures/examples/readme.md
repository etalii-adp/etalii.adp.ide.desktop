# Agent behavior models

Four chat agents written as behavior trees: each `.md` is the instruction file the agent reads, and each `.adp` beside it registers that file as an Agent Behavior Modelling diagram and holds nothing but the visualization.

## Authored for this repository

**These examples were written for ADP, for the same reason the functional decomposition graph's are.** The rule is that diagram modules are tested against real published example data. It cannot apply here: a behavior tree written as an agent's Markdown instructions is a notation this repository defines, so no published corpus of it can exist. Nothing in them is derived from, or attributed to, an external source. A hand-written example tests what its author had in mind, and none of these surprised the parser the way a real corpus would.

## What they show

| Example | What it exercises |
|---|---|
| `pull-request-reviewer` | A Try in order whose first branch skips the work; a Do together gathering context; a Retry; notes under nodes; an Ask approval before guarding the only outward-facing step |
| `customer-support` | Nested Try in order as routing; Ask the user to fill a missing fact; Delegate as the hand-over to a person; an approval around a refund |
| `bug-fixer` | The agentic loop - a Repeat until inside an Only while that keeps the change in scope; a Delegate that links another behavior file |
| `research-assistant` | A Do together of three Delegates as sub-agents; a Repeat until as an evaluator loop; and a `layout:` block in its `.adp`, which lowers the root's children, and everything beneath them, below their computed height |

**Every one of the eleven kinds appears at least once**, and every example opens with the section that tells the agent how to read the tree - the one a new diagram starts with.

## What they do not demonstrate

- **A model that breaks a rule.** The validator reports nothing for these four; the broken shapes are built in the module's tests.
- **An item without a keyword**, which is read as a Do and reported.
- **A Behavior heading that is not level two**, or a tree in a file with more than one Behavior heading.
- **Tabs, `*` or `+` markers, or a loose list with blank lines between items**, all of which the parser reads and the tests cover.
