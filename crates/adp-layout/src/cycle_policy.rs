//! What longest-path layering does with an edge that closes a cycle (etalii.adp spec 030, FR-018), ported from the
//! standalone host's `EtAlii.Adp.Layout/CyclePolicy.cs` at `8b6ccfc`.
//!
//! The layouts that layer a graph agree on longest-path layering and disagree only on cycles, which none of their
//! formats forbids outright. Each passes the rule it already has, so joining them moves no picture; agreeing on one
//! rule is a separate decision (etalii.adp spec 025, behaviour change B-03). A rule is added here by the first layout
//! that needs it.

/// What [`crate::layering::longest_path`] does with an edge that closes a cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CyclePolicy {
    /// A depth-first walk meets a node already on its stack and counts that node's depth as zero, so the edge into it
    /// still pushes its dependent one layer on and the cycle is not chased.
    BackEdgeIsZero,

    /// A depth-first walk meets a node already on its stack and leaves the edge into it out, so the edge pushes nothing
    /// on: a cycle's members sit no further right than their other dependencies put them.
    BackEdgeIgnored,

    /// As [`CyclePolicy::BackEdgeIsZero`], but every node's layer comes from a walk of its own that remembers nothing,
    /// not even within itself. On a graph without cycles that is the same layering; on a cycle every member is measured
    /// from itself, so no member's layer depends on which was walked first. A node reached along several paths is
    /// measured once per path, so the cost grows with the number of paths and it suits small graphs only.
    BackEdgeIsZeroPerNode,

    /// Kahn's algorithm: a node is placed once every node before it is. A node on a cycle, or downstream of one, is
    /// never released, so it keeps its floor raised only by the predecessors that were. Fed edges with the cycles
    /// already taken out, it is plain longest-path layering.
    Kahn,
}
