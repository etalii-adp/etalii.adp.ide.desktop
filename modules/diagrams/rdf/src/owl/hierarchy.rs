//! The asserted subclass graph's shape (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Owl/OwlClassHierarchy.cs` at `8b6ccfc`, shared by the layout (column depths) and the validator (cycle warnings):
//! strongly connected components over class-to-superclass adjacency, and the depth of each class with every cycle
//! collapsed to one depth so the walk always terminates.

use std::collections::{HashMap, HashSet};

use adp_layout::cycle_policy::CyclePolicy;
use adp_layout::layering;
use indexmap::IndexMap;

/// Class to its asserted superclasses, in the order the classes were met.
pub type Supers = IndexMap<String, Vec<String>>;

/// The cycles: every strongly connected component with more than one member, plus every self-loop.
pub fn cycles(supers: &Supers) -> Vec<Vec<String>> {
    components(supers)
        .into_iter()
        .filter(|component| {
            component.len() > 1 || supers.get(&component[0]).is_some_and(|own| own.contains(&component[0]))
        })
        .collect()
}

/// Column depth per class: a root at 0, every other class one past its deepest asserted superclass, a cycle's members
/// sharing one depth.
pub fn depths(supers: &Supers) -> HashMap<String, i32> {
    let components = components(supers);
    let mut component_of: HashMap<&str, usize> = HashMap::new();
    for (i, component) in components.iter().enumerate() {
        for member in component {
            component_of.insert(member, i);
        }
    }
    let mut edges: Vec<(String, String)> = Vec::new();
    for (child, parents) in supers {
        for parent in parents {
            if let Some(&parent_component) = component_of.get(parent.as_str())
                && parent_component != component_of[child.as_str()]
            {
                edges.push((
                    components[parent_component][0].clone(),
                    components[component_of[child.as_str()]][0].clone(),
                ));
            }
        }
    }
    let component_depth = layering::longest_path(
        components.iter().map(|c| c[0].clone()),
        edges,
        CyclePolicy::BackEdgeIsZero,
    );
    component_of
        .iter()
        .map(|(member, &i)| ((*member).to_owned(), component_depth[&components[i][0]]))
        .collect()
}

/// Tarjan's strongly connected components over the adjacency, deterministic in key order.
fn components(supers: &Supers) -> Vec<Vec<String>> {
    struct Walk<'a> {
        supers: &'a Supers,
        index_of: HashMap<String, usize>,
        low_of: HashMap<String, usize>,
        on_stack: HashSet<String>,
        stack: Vec<String>,
        components: Vec<Vec<String>>,
        counter: usize,
    }

    impl Walk<'_> {
        fn connect(&mut self, node: &str) {
            self.index_of.insert(node.to_owned(), self.counter);
            self.low_of.insert(node.to_owned(), self.counter);
            self.counter += 1;
            self.stack.push(node.to_owned());
            self.on_stack.insert(node.to_owned());

            if let Some(parents) = self.supers.get(node) {
                for parent in parents {
                    if !self.supers.contains_key(parent) && !self.index_of.contains_key(parent) {
                        continue; // a superclass with no own subclass entry cannot be in a cycle
                    }
                    match self.index_of.get(parent).copied() {
                        None => {
                            self.connect(parent);
                            let low = self.low_of[node].min(self.low_of[parent]);
                            self.low_of.insert(node.to_owned(), low);
                        }
                        Some(parent_index) if self.on_stack.contains(parent) => {
                            let low = self.low_of[node].min(parent_index);
                            self.low_of.insert(node.to_owned(), low);
                        }
                        Some(_) => {}
                    }
                }
            }

            if self.low_of[node] == self.index_of[node] {
                let mut component = Vec::new();
                loop {
                    let member = self.stack.pop().expect("the node itself is on the stack");
                    self.on_stack.remove(&member);
                    let done = member == node;
                    component.push(member);
                    if done {
                        break;
                    }
                }
                component.reverse();
                self.components.push(component);
            }
        }
    }

    let mut walk = Walk {
        supers,
        index_of: HashMap::new(),
        low_of: HashMap::new(),
        on_stack: HashSet::new(),
        stack: Vec::new(),
        components: Vec::new(),
        counter: 0,
    };
    for node in supers.keys() {
        if !walk.index_of.contains_key(node) {
            walk.connect(node);
        }
    }
    walk.components
}
