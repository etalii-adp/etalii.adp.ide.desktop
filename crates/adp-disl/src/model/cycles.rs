//! The elementary cycles of a diagram over the relations of one type (DISL section 12.4, `diagram.cycles` and
//! `diagram.cyclesTruncated`; the standalone host's `DislCycles`), at most a bound of them.
//!
//! Johnson's elementary cycles algorithm: for each node in model order, the cycles whose member that comes first in
//! model order is that node are found over the nodes at or after it, so each cycle is found once, starting at that
//! member. The order is section 12.4's by construction: every node's successors are walked in model order, so the
//! depth-first search closes the cycles of one start in lexicographic order of their members' positions, and the
//! bound keeps the first cycles of that order; the search stops at the first cycle past it.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::element::DislElement;

/// The cycles over `relations` among `nodes`, at most `max`, and whether the bound cut them.
pub(crate) fn find(nodes: &[DislElement], relations: &[DislElement], max: i64) -> (Vec<Vec<DislElement>>, bool) {
    let position: HashMap<usize, usize> = nodes.iter().enumerate().map(|(i, n)| (n.key(), i)).collect();
    let mut successors: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); nodes.len()];
    for relation in relations {
        if let (Some(source), Some(target)) = (relation.source(), relation.target())
            && let (Some(from), Some(to)) = (position.get(&source.key()), position.get(&target.key()))
        {
            successors[*from].insert(*to);
        }
    }

    let mut search = Search {
        blocked: vec![false; nodes.len()],
        waiting: vec![HashSet::new(); nodes.len()],
        stack: Vec::new(),
        root: 0,
        found: Vec::new(),
        truncated: false,
        successors: &successors,
        max,
    };
    for start in 0..nodes.len() {
        if search.truncated {
            break;
        }
        search.from(start);
    }
    let cycles = search
        .found
        .iter()
        .map(|cycle| cycle.iter().map(|i| nodes[*i].clone()).collect())
        .collect();
    (cycles, search.truncated)
}

struct Search<'a> {
    blocked: Vec<bool>,
    waiting: Vec<HashSet<usize>>,
    stack: Vec<usize>,
    root: usize,
    found: Vec<Vec<usize>>,
    truncated: bool,
    successors: &'a [BTreeSet<usize>],
    max: i64,
}

impl Search<'_> {
    fn from(&mut self, root: usize) {
        self.root = root;
        for index in root..self.successors.len() {
            self.blocked[index] = false;
            self.waiting[index].clear();
        }
        self.circuit(root);
    }

    /// Johnson's CIRCUIT: whether a cycle back to the root was closed through `vertex`.
    fn circuit(&mut self, vertex: usize) -> bool {
        let mut closed = false;
        self.stack.push(vertex);
        self.blocked[vertex] = true;

        let nexts: Vec<usize> = self.successors[vertex]
            .iter()
            .copied()
            .filter(|n| *n >= self.root)
            .collect();
        for next in nexts.iter().copied() {
            if next == self.root {
                if self.found.len() as i64 >= self.max {
                    self.truncated = true;
                    break;
                }
                self.found.push(self.stack.clone());
                closed = true;
            } else if !self.blocked[next] && self.circuit(next) {
                closed = true;
            }
            if self.truncated {
                break;
            }
        }

        if closed {
            self.unblock(vertex);
        } else {
            for next in nexts {
                self.waiting[next].insert(vertex);
            }
        }
        self.stack.pop();
        closed
    }

    fn unblock(&mut self, vertex: usize) {
        self.blocked[vertex] = false;
        let waiting: Vec<usize> = self.waiting[vertex].drain().collect();
        for dependant in waiting {
            if self.blocked[dependant] {
                self.unblock(dependant);
            }
        }
    }
}
