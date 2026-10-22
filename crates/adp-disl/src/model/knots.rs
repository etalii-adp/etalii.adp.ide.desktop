//! The knots of a diagram over the relations of one type (DISL section 12.4, `diagram.knots`; the standalone host's
//! `DislKnots`): its strongly connected components that have more than one member or a self-loop, members in model
//! order, components ordered by their first member.
//!
//! Tarjan's algorithm, iterative so that a deep graph cannot exhaust the stack; the order is imposed afterwards, so the
//! result does not depend on the order the search happens to meet the nodes in.

use std::collections::HashMap;

use super::element::DislElement;

/// The knots over `relations` among `nodes`.
pub(crate) fn find(nodes: &[DislElement], relations: &[DislElement]) -> Vec<Vec<DislElement>> {
    let position: HashMap<usize, usize> = nodes.iter().enumerate().map(|(i, n)| (n.key(), i)).collect();
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut self_loop = vec![false; nodes.len()];
    for relation in relations {
        if let (Some(source), Some(target)) = (relation.source(), relation.target())
            && let (Some(&from), Some(&to)) = (position.get(&source.key()), position.get(&target.key()))
        {
            successors[from].push(to);
            if from == to {
                self_loop[from] = true;
            }
        }
    }

    let mut knots: Vec<Vec<usize>> = components(&successors)
        .into_iter()
        .filter(|component| component.len() > 1 || self_loop[component[0]])
        .map(|mut component| {
            component.sort_unstable();
            component
        })
        .collect();
    knots.sort_by_key(|component| component[0]);
    knots
        .into_iter()
        .map(|component| component.into_iter().map(|index| nodes[index].clone()).collect())
        .collect()
}

fn components(successors: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let count = successors.len();
    let mut index = vec![usize::MAX; count];
    let mut low = vec![0; count];
    let mut on_stack = vec![false; count];
    let mut stack = Vec::new();
    let mut components = Vec::new();
    let mut counter = 0;

    for start in 0..count {
        if index[start] != usize::MAX {
            continue;
        }
        let mut work = vec![(start, 0)];
        while let Some((node, next)) = work.pop() {
            if next == 0 {
                index[node] = counter;
                low[node] = counter;
                counter += 1;
                stack.push(node);
                on_stack[node] = true;
            }
            if next < successors[node].len() {
                work.push((node, next + 1));
                let successor = successors[node][next];
                if index[successor] == usize::MAX {
                    work.push((successor, 0));
                } else if on_stack[successor] {
                    low[node] = low[node].min(index[successor]);
                }
                continue;
            }
            if low[node] == index[node] {
                let mut component = Vec::new();
                loop {
                    let member = stack.pop().expect("the node is on the stack");
                    on_stack[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                components.push(component);
            }
            if let Some(&(parent, _)) = work.last() {
                low[parent] = low[parent].min(low[node]);
            }
        }
    }
    components
}
