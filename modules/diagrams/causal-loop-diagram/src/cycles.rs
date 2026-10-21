//! The feedback cycles the links form, in the order the tool has always found them (the standalone host's
//! `CycleFinder.cs`, `CycleFinderResult.cs` and `CausalLoopCycles.cs` at `8b6ccfc`), and the CEL function
//! `causalCycles` of the specification's plugin `net.etalii.adp.systems.cldCycles` (DISL section 13.1.1) that runs it.
//!
//! DISL 12.4's `diagram.cycles` finds the same cycles and sorts them; the tool reports them as its search finds them,
//! and the two orders differ over two documents of the standalone corpus (`lenient-spellings.cld` and
//! `weight-forms.cld`). The order decides the order of the unlabelled-loop findings and which of several shortest
//! cycles Claim a loop picks, so the module supplies the plugin (`causal-loop-diagram.md`, "Other differences: the order
//! of cycles"); without it the specification falls back to `diagram.cycles`.
//!
//! The search is the standalone host's rendering of Johnson's: for each start variable in model order, the cycles
//! whose least variable by that order is the start, over the variables at or after it, each a depth-first walk of the
//! links in model order. It stops at the bound and says so: a check that examined fewer cycles than the document holds
//! must not report that none is unlabelled.

use std::collections::{HashMap, HashSet};

use adp_cel::Value;
use adp_disl::{DislElement, DislPluginFunctions};

use crate::reading::{Link, Model, Variable, text};

/// How many cycles are enumerated before the search stops and reports.
pub const DEFAULT_BOUND: usize = 1000;

/// What a cycle search produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cycles {
    /// Each cycle as the variables it runs through, in order, starting from its least member.
    pub cycles: Vec<Vec<String>>,
    /// Whether the bound stopped the search before it finished.
    pub truncated: bool,
}

/// The elementary cycles of `model`'s links, at most `bound` of them; a name written twice is one variable, the
/// first, as `causalCycles` hands the finder the first variable of each name.
pub fn find(model: &Model, bound: usize) -> Cycles {
    assert!(bound >= 1, "a cycle search is bounded by at least one cycle");
    let mut known: HashSet<&str> = HashSet::new();
    let order: Vec<&str> = model
        .variables
        .iter()
        .map(|v| v.name.as_str())
        .filter(|name| known.insert(name))
        .collect();
    // A link to a variable the document never declares is not walked; the findings report it.
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for id in &order {
        let mut next: Vec<&str> = Vec::new();
        for link in &model.links {
            if link.from == *id && known.contains(link.to.as_str()) && !next.contains(&link.to.as_str()) {
                next.push(link.to.as_str());
            }
        }
        adjacency.insert(id, next);
    }

    let mut search = Search {
        adjacency: &adjacency,
        cycles: Vec::new(),
        bound,
        truncated: false,
    };
    for start in 0..order.len() {
        if search.truncated {
            break;
        }
        let root = order[start];
        let allowed: HashSet<&str> = order[start..].iter().copied().collect();
        let mut state = Walk {
            root,
            allowed,
            blocked: HashSet::new(),
            blocked_on: HashMap::new(),
            stack: Vec::new(),
        };
        search.circuit(root, &mut state);
    }
    Cycles {
        cycles: search.cycles,
        truncated: search.truncated,
    }
}

struct Search<'a> {
    adjacency: &'a HashMap<&'a str, Vec<&'a str>>,
    cycles: Vec<Vec<String>>,
    bound: usize,
    truncated: bool,
}

struct Walk<'a> {
    root: &'a str,
    allowed: HashSet<&'a str>,
    blocked: HashSet<&'a str>,
    blocked_on: HashMap<&'a str, Vec<&'a str>>,
    stack: Vec<&'a str>,
}

impl<'a> Search<'a> {
    /// Johnson's CIRCUIT: whether a cycle was closed through `vertex`.
    fn circuit(&mut self, vertex: &'a str, walk: &mut Walk<'a>) -> bool {
        let mut found = false;
        walk.stack.push(vertex);
        walk.blocked.insert(vertex);
        let next: Vec<&'a str> = self.adjacency[vertex]
            .iter()
            .copied()
            .filter(|n| walk.allowed.contains(n))
            .collect();
        for &step in &next {
            if step == walk.root {
                if self.cycles.len() >= self.bound {
                    self.truncated = true;
                    break;
                }
                self.cycles.push(walk.stack.iter().map(|s| (*s).to_owned()).collect());
                found = true;
            } else if !walk.blocked.contains(step) {
                if self.circuit(step, walk) {
                    found = true;
                }
                if self.truncated {
                    break;
                }
            }
        }
        if found {
            unblock(vertex, walk);
        } else {
            // Nothing closed through here: unblock this vertex when a neighbour later does.
            for &step in &next {
                let waiting = walk.blocked_on.entry(step).or_default();
                if !waiting.contains(&vertex) {
                    waiting.push(vertex);
                }
            }
        }
        walk.stack.pop();
        found
    }
}

fn unblock<'a>(vertex: &'a str, walk: &mut Walk<'a>) {
    walk.blocked.remove(vertex);
    let Some(waiting) = walk.blocked_on.get_mut(vertex) else {
        return;
    };
    // Taken as a snapshot: unblocking a dependant can add to this set while it is walked.
    let pending = std::mem::take(waiting);
    for dependant in pending {
        if walk.blocked.contains(dependant) {
            unblock(dependant, walk);
        }
    }
}

/// A directed cycle's canonical form: its members rotated to start at the ordinally least, joined, so two
/// statements of one loop that begin at different variables compare equal.
pub fn canonical_signature(cycle: &[String]) -> String {
    if cycle.is_empty() {
        return String::new();
    }
    let start = (1..cycle.len()).fold(0, |least, index| {
        if cycle[index].as_bytes() < cycle[least].as_bytes() {
            index
        } else {
            least
        }
    });
    (0..cycle.len())
        .map(|offset| cycle[(start + offset) % cycle.len()].as_str())
        .collect::<Vec<_>>()
        .join("\0")
}

/// The plugin `net.etalii.adp.systems.cldCycles`, for loading the specification.
pub fn plugin_functions() -> DislPluginFunctions {
    DislPluginFunctions::none().add("causalCycles", causal_cycles)
}

/// `causalCycles(variables, links, max)`: the finder over the elements given, each cycle a list of the first
/// variable of each name it runs through.
fn causal_cycles(arguments: &[Value]) -> Result<Value, String> {
    let variables = elements(arguments.first())?;
    let links = elements(arguments.get(1))?;
    let bound = match arguments.get(2) {
        Some(Value::Int(bound)) if *bound >= 1 && *bound <= i64::from(i32::MAX) => *bound as usize,
        _ => return Err("causalCycles takes a bound of at least 1.".to_owned()),
    };
    let mut by_name: HashMap<String, DislElement> = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    for variable in variables {
        let name = text(&variable, "name");
        if let std::collections::hash_map::Entry::Vacant(first) = by_name.entry(name.clone()) {
            names.push(name);
            first.insert(variable);
        }
    }
    let model = Model {
        variables: names.iter().map(|n| Variable::new(n, n)).collect(),
        links: links
            .iter()
            .map(|l| Link::new(&text(l, "causeName"), &text(l, "effectName")))
            .collect(),
    };
    let found = find(&model, bound);
    Ok(Value::list(
        found
            .cycles
            .iter()
            .map(|cycle| Value::list(cycle.iter().map(|name| by_name[name].to_value()).collect()))
            .collect(),
    ))
}

fn elements(argument: Option<&Value>) -> Result<Vec<DislElement>, String> {
    let refused = || "causalCycles takes lists of elements.".to_owned();
    match argument {
        Some(Value::List(items)) => items
            .iter()
            .map(|i| DislElement::from_value(i).ok_or_else(refused))
            .collect(),
        _ => Err(refused()),
    }
}
