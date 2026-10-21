//! The radiating layout, as a pure function (etalii.adp spec 030, FR-018, task T054; agent-activity-diagram
//! Requirements 6.5 to 6.10), ported from the standalone host's
//! `src/client/src/canvas/library/layout/tieredForceLayout.test.ts` at `8b6ccfc`.
//!
//! Asserted on a generated diagram shaped like the one it was written for (projects, their specifications, the agents
//! on them, each agent's locations and the environments those run on), because the properties below are about a
//! population and a toy of five elements shows none of them: nothing overlaps in a toy whatever the algorithm does.

use std::collections::HashMap;
use std::time::Instant;

use adp_layout::{
    LayoutAlgorithm, LayoutConnection, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode, Placements,
    TieredForceLayout,
};

const TIERS: [&str; 5] = ["project", "specification", "agent", "location", "environment"];

fn definition() -> LayoutDefinition {
    LayoutDefinition {
        tiers: TIERS.iter().map(|tier| vec![tier.to_string()]).collect(),
        ..LayoutDefinition::with_modes(&[LayoutMode::TieredForce])
    }
}

fn element(id: &str, kind: &str, width: f64, height: f64) -> LayoutElement {
    LayoutElement {
        element_type: Some(kind.to_string()),
        ..LayoutElement::new(id, 0.0, 0.0, width, height)
    }
}

/// A deterministic diagram: `projects` projects, three specifications each, and so on outwards.
fn generate(projects: usize) -> LayoutInput {
    let mut elements = Vec::new();
    let mut connections = Vec::new();
    let environments = ["e-local", "e-cloud", "e-container"];
    for id in environments {
        elements.push(element(id, "environment", 160.0, 48.0));
    }
    for p in 0..projects {
        elements.push(element(&format!("p{p}"), "project", 180.0, 48.0));
        for s in 0..3 {
            let spec = format!("p{p}s{s}");
            // Specifications differ a great deal in height: one has its task groups unfolded.
            elements.push(element(
                &spec,
                "specification",
                240.0,
                if s == 0 { 260.0 } else { 96.0 },
            ));
            connections.push(LayoutConnection::new(format!("p{p}"), spec.clone()));
            for a in 0..2 {
                let agent = format!("{spec}a{a}");
                elements.push(element(&agent, "agent", 150.0, 44.0));
                connections.push(LayoutConnection::new(spec.clone(), agent.clone()));
                let location = format!("{agent}l");
                elements.push(element(&location, "location", 200.0, if a == 0 { 140.0 } else { 72.0 }));
                connections.push(LayoutConnection::new(agent.clone(), location.clone()));
                connections.push(LayoutConnection::new(
                    location,
                    environments[(p + s + a) % environments.len()],
                ));
            }
        }
    }
    // One of each outer type with no relation at all.
    elements.push(element("idle-agent", "agent", 150.0, 44.0));
    elements.push(element("spare-environment", "environment", 160.0, 48.0));
    LayoutInput { elements, connections }
}

fn placed(input: &LayoutInput, previous: Option<&Placements>) -> Placements {
    TieredForceLayout.place(input, &definition(), previous).unwrap()
}

fn overlapping(input: &LayoutInput, positions: &Placements) -> Vec<String> {
    let boxes: Vec<(&str, f64, f64, f64, f64)> = input
        .elements
        .iter()
        .map(|element| {
            let (x, y) = positions
                .get(&element.id)
                .map_or((element.x, element.y), |at| (at.x, at.y));
            (
                element.id.as_str(),
                x - element.width / 2.0,
                x + element.width / 2.0,
                y - element.height / 2.0,
                y + element.height / 2.0,
            )
        })
        .collect();
    let mut pairs = Vec::new();
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            if a.1 < b.2 && b.1 < a.2 && a.3 < b.4 && b.3 < a.4 {
                pairs.push(format!("{}~{}", a.0, b.0));
            }
        }
    }
    pairs
}

fn mean_distance_by_type(input: &LayoutInput, positions: &Placements) -> HashMap<String, f64> {
    let mut sums: HashMap<String, (f64, usize)> = HashMap::new();
    for element in &input.elements {
        let at = positions[&element.id];
        let entry = sums.entry(element.element_type.clone().unwrap()).or_insert((0.0, 0));
        entry.0 += at.x.hypot(at.y);
        entry.1 += 1;
    }
    sums.into_iter()
        .map(|(kind, (total, count))| (kind, total / count as f64))
        .collect()
}

fn distance(positions: &Placements, id: &str) -> f64 {
    positions[id].x.hypot(positions[id].y)
}

/// The diagram with one more agent, connected to `p0s2`.
fn grown(input: &LayoutInput) -> LayoutInput {
    let mut grown = input.clone();
    grown.elements.push(element("new-agent", "agent", 150.0, 44.0));
    grown.connections.push(LayoutConnection::new("p0s2", "new-agent"));
    grown
}

fn with_pinned(input: &LayoutInput, id: &str, x: f64, y: f64) -> LayoutInput {
    let mut pinned = input.clone();
    for element in pinned.elements.iter_mut().filter(|element| element.id == id) {
        element.x = x;
        element.y = y;
        element.pinned = true;
    }
    pinned
}

#[test]
fn places_the_tiers_outwards_from_the_centre_in_the_declared_order() {
    let input = generate(2);
    let means = mean_distance_by_type(&input, &placed(&input, None));

    let order: Vec<f64> = TIERS.iter().map(|tier| means[*tier]).collect();
    let mut sorted = order.clone();
    sorted.sort_by(f64::total_cmp);
    assert_eq!(order, sorted);
    assert!(order[0] < order[1]);
}

#[test]
fn leaves_no_two_elements_overlapping_tall_and_short_alike() {
    // The planted defect this was seen to fail against: the separating pass removed.
    for projects in [1, 2, 4] {
        let input = generate(projects);

        assert_eq!(
            overlapping(&input, &placed(&input, None)),
            Vec::<String>::new(),
            "{projects} project(s)"
        );
    }
}

/// Not one of the standalone host's tests: the placements its own `tieredForceLayout.ts` gives for the two-project
/// diagram, recorded in `tests/fixtures/tiered-force-two-projects.json`, so the port is held to the same picture and
/// not only to the same properties.
#[test]
fn gives_the_picture_the_standalone_host_gives() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiered-force-two-projects.json");
    let golden: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let golden = golden.as_object().unwrap();

    let positions = placed(&generate(2), None);

    assert_eq!(positions.len(), golden.len());
    for (id, at) in golden {
        let (x, y) = (at[0].as_f64().unwrap(), at[1].as_f64().unwrap());
        assert_eq!((positions[id].x, positions[id].y), (x, y), "{id}");
    }
}

#[test]
fn gives_the_same_picture_twice() {
    let first = placed(&generate(3), None);
    let second = placed(&generate(3), None);

    assert_eq!(second, first);
}

#[test]
fn places_an_element_with_no_relation_on_its_own_tier_not_in_the_middle_and_not_adrift() {
    let input = generate(2);
    let positions = placed(&input, None);
    let means = mean_distance_by_type(&input, &positions);

    assert!(distance(&positions, "idle-agent") > means["specification"] * 0.8);
    assert!(distance(&positions, "idle-agent") < means["location"] * 1.2);
    assert!(distance(&positions, "spare-environment") > means["location"] * 0.8);
}

#[test]
fn puts_a_single_project_in_the_centre() {
    let input = generate(1);

    assert!(distance(&placed(&input, None), "p0") < 120.0);
}

#[test]
fn a_pinned_element_gets_no_placement_and_the_others_keep_clear_of_where_it_is() {
    let input = generate(2);
    let pinned = with_pinned(&input, "p0s0", 900.0, -700.0);

    let positions = placed(&pinned, None);

    assert!(!positions.contains_key("p0s0"));
    assert_eq!(positions.len(), input.elements.len() - 1);
    assert_eq!(overlapping(&pinned, &positions), Vec::<String>::new());
}

#[test]
fn a_pinned_element_draws_what_is_connected_to_it_towards_it() {
    let input = generate(1);
    let free = placed(&input, None);
    let moved = placed(&with_pinned(&input, "p0s1", 1500.0, 0.0), None);

    // Its two agents end up nearer the pinned place than they were without the pin.
    for agent in ["p0s1a0", "p0s1a1"] {
        let before = (free[agent].x - 1500.0).hypot(free[agent].y);
        let after = (moved[agent].x - 1500.0).hypot(moved[agent].y);
        assert!(after < before, "{agent}");
    }
}

#[test]
fn from_where_the_picture_is_moves_the_elements_a_change_does_not_touch_only_a_little() {
    // The planted defect this was seen to fail against: the previous placements ignored, so the pass starts from
    // nothing and the same addition moves everything.
    let input = generate(4);
    let settled = placed(&input, None);
    let grown = grown(&input);

    let after = placed(&grown, Some(&settled));

    // Everything on the other three projects' side of the picture.
    let moves = input
        .elements
        .iter()
        .filter(|element| {
            !element.id.starts_with("p0")
                && !element.id.starts_with("e-")
                && element.element_type.as_deref() != Some("environment")
        })
        .map(|element| {
            (after[&element.id].x - settled[&element.id].x).hypot(after[&element.id].y - settled[&element.id].y)
        });
    assert!(moves.fold(0.0, f64::max) < 60.0);
    assert_eq!(overlapping(&grown, &after), Vec::<String>::new());
}

#[test]
fn what_a_layout_pass_depends_on_is_unchanged_by_anything_that_does_not_place_differently_and_changed_by_anything_that_does()
 {
    let input = generate(1);
    let signature_of = |input: &LayoutInput| TieredForceLayout.signature(input).unwrap();
    let signature = signature_of(&input);
    let changed = |change: &dyn Fn(usize, &mut LayoutElement)| {
        let mut changed = input.clone();
        for (index, element) in changed.elements.iter_mut().enumerate() {
            change(index, element);
        }
        signature_of(&changed)
    };

    // An unpinned element's own position is not an input: the layout decides it.
    assert_eq!(
        changed(&|_, element| {
            element.x = 5.0;
            element.y = 9.0;
        }),
        signature
    );
    assert_ne!(
        changed(&|index, element| {
            if index == 0 {
                element.height += 18.0;
            }
        }),
        signature
    );
    assert_ne!(
        changed(&|index, element| {
            if index == 0 {
                element.pinned = true;
            }
        }),
        signature
    );
    let mut fewer = input.clone();
    fewer.connections.remove(0);
    assert_ne!(signature_of(&fewer), signature);
}

/// At size (agent-activity-diagram Requirement 11.2): about two hundred elements, which is a team of a dozen projects.
#[test]
fn at_about_two_hundred_elements_still_leaves_nothing_overlapping_and_still_gives_the_same_picture_twice() {
    let input = generate(12);
    let first = placed(&input, None);

    assert!(input.elements.len() >= 190);
    assert_eq!(overlapping(&input, &first), Vec::<String>::new());
    assert_eq!(placed(&input, None), first);
}

/// The pass is held to a multiple of a reference workload timed beside it: one sweep over every pair of elements, the
/// arithmetic a single step of the layout does. A slower machine slows both alike, so the ratio says how many sweeps'
/// worth of work a pass is, which is what a regression changes, and not how fast the machine is. The two are timed in
/// turn, round after round, and the fastest of each is kept, since a busy machine slows a run and never speeds one up.
#[test]
fn at_about_two_hundred_elements_places_one_addition_without_laying_everything_out_from_nothing() {
    // The planted defect this was seen to fail against: the step counts raised tenfold.
    let input = generate(12);
    let settled = placed(&input, None);
    let grown = grown(&input);

    let after = placed(&grown, Some(&settled));
    assert_eq!(overlapping(&grown, &after), Vec::<String>::new());

    let count = grown.elements.len();
    let sweep = || {
        let mut total = 0.0f64;
        for i in 0..count {
            for j in i + 1..count {
                let dx = (j - i) as f64 * 1.5 + total * 1e-12;
                let dy = (i + 1) as f64 * 0.5;
                let distance = dx.hypot(dy);
                total += (dx / distance) * (26000.0 / (distance * distance));
            }
        }
        total
    };
    let timed = |work: &dyn Fn()| {
        let started = Instant::now();
        work();
        started.elapsed().as_secs_f64()
    };
    let mut reference = f64::INFINITY;
    let mut pass = f64::INFINITY;
    for _ in 0..12 {
        // Fifty sweeps a round, so the reference is long enough to time.
        reference = reference.min(
            timed(&|| {
                for _ in 0..50 {
                    std::hint::black_box(sweep());
                }
            }) / 50.0,
        );
        pass = pass.min(timed(&|| {
            std::hint::black_box(placed(&grown, Some(&settled)));
        }));
    }

    // The standalone host measured about 105 sweeps, and about 555 with the step counts raised tenfold; its bound sits
    // between the two.
    assert!(pass / reference < 420.0, "a pass took {} sweeps", pass / reference);
}
