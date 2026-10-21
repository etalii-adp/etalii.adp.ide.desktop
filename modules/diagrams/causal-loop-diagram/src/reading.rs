//! A causal loop diagram as the module's layouts and cycle finder read it (the standalone host's `CausalLoopModel`,
//! `CausalLoopVariable` and `CausalLoopLink` at `8b6ccfc`, built by `CausalLoopBody.Read`): the variables and the
//! causal links in model order, each named as the binding reads it.

use adp_disl::{DislDiagram, DislElement};

/// One variable: the name links and loops refer to it by, and its label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub label: String,
}

impl Variable {
    pub fn new(name: &str, label: &str) -> Self {
        Variable {
            name: name.to_owned(),
            label: label.to_owned(),
        }
    }

    /// What the canvas draws: the label where there is one, the name otherwise.
    pub fn display(&self) -> &str {
        if self.label.is_empty() { &self.name } else { &self.label }
    }

    /// The element id the registration stores its position under.
    pub fn element_id(&self) -> String {
        format!("variable:{}", self.name)
    }
}

/// One causal link, by the names of its two ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub from: String,
    pub to: String,
}

impl Link {
    pub fn new(from: &str, to: &str) -> Self {
        Link {
            from: from.to_owned(),
            to: to.to_owned(),
        }
    }
}

/// What a `.cld` states, as far as the layouts and the cycle finder read it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Model {
    /// In model order; a name written twice is listed twice, as the body states it.
    pub variables: Vec<Variable>,
    /// In model order.
    pub links: Vec<Link>,
}

impl Model {
    /// The variables and causal links of `diagram`.
    pub fn of(diagram: &DislDiagram) -> Model {
        Model {
            variables: diagram
                .nodes_of_type("Variable")
                .iter()
                .map(|v| Variable::new(&text(v, "name"), &text(v, "label")))
                .collect(),
            links: diagram
                .relations_of_type("CausalLink")
                .iter()
                .map(|l| Link::new(&text(l, "causeName"), &text(l, "effectName")))
                .collect(),
        }
    }

    /// The variables with a name written twice kept once, the first, as everything else reads it (finding A7).
    pub fn distinct_variables(&self) -> Vec<Variable> {
        let mut seen = std::collections::HashSet::new();
        self.variables
            .iter()
            .filter(|v| seen.insert(v.name.clone()))
            .cloned()
            .collect()
    }
}

/// `element`'s text attribute `name`, without the quotes it may have been written with; empty for none.
pub fn text(element: &DislElement, name: &str) -> String {
    let value = element.value_of(name);
    let text = value.as_str().unwrap_or_default();
    match text.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
        Some(unquoted) if text.len() >= 2 => unquoted.to_owned(),
        _ => text.to_owned(),
    }
}
