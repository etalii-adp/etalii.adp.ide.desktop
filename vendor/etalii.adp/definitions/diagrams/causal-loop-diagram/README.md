# Writing a causal loop diagram (`.cld`) with an LLM

A `.cld` file is the text body of ADP's causal loop diagram ([`causal-loop-diagram.dis`](../causal-loop-diagram.dis), read and written through the FBL binding [`specifications/fbl/causal-loop-diagram.fbl#cld`](../../../specifications/fbl/causal-loop-diagram.fbl)). It is plain UTF-8 text, one statement per line: a `causal-loop 1` header, then variables, the links between them and the loops they form. Positions are not part of it; a host keeps them in the `.adp` registration and lays out a diagram that has none.

To generate one, write text the grammar below accepts, and keep to these rules, which a grammar cannot hold and the hosts report as findings:

- Write `causal-loop 1` as the first statement, and every variable before the links and loops that name it.
- Give every variable a unique name; a link's two ends and a loop's members must each be the name of a declared variable.
- State each cause and effect pair at most once, and give every link a polarity, `+` or `-`: a link without one is unknown, and every loop through it is undecidable.
- A loop lists its members in the order the arrows run, so there is a link from each member to the next and from the last back to the first. Its identifier is unique and claims a polarity: `R…` (reinforcing) when the cycle has an even number of `-` links, `B…` (balancing) when the number is odd. Claim every feedback loop you mean; a cycle no loop names is reported.
- Quoted text holds no `"` and no line break. Write at most one `title` and one `flow`, and keep their values in the ranges the comments give.

The grammar is LALR(1) and strict: it builds and parses with Lark's `parser="lalr"` and `strict=True`, and so with the EtAlii Lark packages (`@etalii/lark`, `EtAlii.Lark`, `etalii-lark`). It is a little stricter than the readers, which also take a bare word after a link, a second header and a file without one; never write those. The same grammar is the file [`cld.lark`](cld.lark) beside this guide, for a tool that loads a grammar from a file or constrains generation with one; the copy below is kept identical to it by `python .github/scripts/cld-grammar-check.py`, which the Build workflow runs.

```lark
// A .cld file: one statement per line, the header first.
start: _preamble* header _NL _line* _last?

_preamble: COMMENT? _NL                // comments and blank lines may come first
_line: statement? _NL
_last: statement                       // a last line without its newline

header: "causal-loop" VERSION          // always write: causal-loop 1

?statement: title
          | flow
          | variable
          | link
          | loop
          | COMMENT

// ---- diagram-wide values for an exported flow page (all optional) --------
title: "title" STRING
flow: "flow" flow_value*
?flow_value: STRENGTH | DELAY | INERTIA | SPEED | DENSITY | HEATMAP

// ---- a quantity that can go up or down --------------------------------
variable: "variable" name label? variable_value*
name: NAME | STRING                    // the identifier links and loops use; never ""
label: STRING | NAME                   // what a reader sees; quote it
?variable_value: START | PUSH

// ---- an arrow from a cause to an effect -------------------------------
link: "link" name "->" name link_word*
?link_word: polarity | DELAYED | FLIPPED | WEIGHT | note
note: STRING                           // free text shown on the link

// ---- a claim about a feedback cycle -----------------------------------
// R... claims reinforcing, B... claims balancing; the members, in the order
// the loop runs, are variable names (unquoted), the last links back to the first.
loop: "loop" NAME loop_title NAME+
loop_title: STRING | NAME

// ---- words ------------------------------------------------------------
VERSION: /[0-9]+/
NAME: /[A-Za-z_][A-Za-z0-9_.-]*/
STRING: /"[^"\r\n]*"/
polarity: "+" -> same                    // they move together
        | "-" -> opposite                // they move opposite
        | "s" -> same | "S" -> same      // older spellings the readers still take;
        | "o" -> opposite | "O" -> opposite  // write + and - instead
DELAYED: "delayed"
FLIPPED: "flipped"

_NUM: /[+-]?(?:[0-9]+\.?[0-9]*|\.[0-9]+)(?:[eE][+-]?[0-9]+)?/
WEIGHT:   "weight=" _NUM             // an annotation, never evaluated
START:    "start=" _NUM              // starting level, 0 to 1
PUSH:     "push=" _NUM               // outside push, -4 to 4
STRENGTH: "strength=" _NUM           // 0.5 to 8
DELAY:    "delay=" _NUM              // 1 to 12
INERTIA:  "inertia=" _NUM            // 0.2 to 5
SPEED:    "speed=" _NUM              // 0.1 to 4
DENSITY:  "density=" _NUM            // 0.5 to 8
HEATMAP:  "heatmap=" ("backlog" | "pressure" | "throughput" | "level")

COMMENT: /#[^\r\n]*/                   // a whole line only
_NL: /\r?\n/
%ignore /[ \t]+/
```

A small valid file:

```text
causal-loop 1
title "Workload and burnout"

variable workload "Workload"
variable fatigue "Fatigue"
variable output "Output" start=0.6

link workload -> fatigue +
link fatigue -> output - delayed "tiredness builds over weeks"
link output -> workload -

loop R1 "Falling behind" workload fatigue output
```

Here `R1` is right: the cycle has two `-` links, an even number, so it reinforces.
