//! A loaded DISL specification (DISL section 3.1; the standalone host's `DislSpecification`): a typed view over the
//! JSON, inheritance flattened, and every expression compiled in its context. What this view does not type stays
//! readable as JSON through [`DislSpecification::root`] and each part's `json`, and every `x-` property is kept
//! unread and unchanged (section 2.8). The id rules of section 11.5 and the built-in settings of section 8.1 are
//! read here too.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use adp_cel::{Environment, Program};

use super::metamodel::{DislExpression, DislFunctionDeclaration, DislIdRule, DislMetamodel};
use crate::behavior::extension::DislActionExtension;
use crate::expressions::contexts;
use crate::json::{self, Json, NOTHING};

/// A program compiled on first use, or why it does not compile.
pub(crate) type Compiled = Result<Program, String>;

/// A loaded specification; cloning it shares it. Made by [`crate::DislLoader`], which refuses a specification with
/// errors rather than giving a view of it.
#[derive(Clone)]
pub struct DislSpecification(pub(crate) Rc<SpecificationData>);

pub(crate) struct SpecificationData {
    root: Json,
    metamodel: DislMetamodel,
    functions: Vec<DislFunctionDeclaration>,
    environment: Environment,
    expressions: Vec<DislExpression>,
    index: HashMap<String, usize>,
    ids: DislIdRule,
    id_types: HashMap<String, DislIdRule>,
    id_prefixes: HashMap<String, String>,
    /// The programs compiled on first use for expressions the loader does not walk, by context, source and bindings.
    pub(crate) compiled: RefCell<HashMap<(String, String, String), Compiled>>,
    /// The runtime that runs `extension` actions, when one loaded the specification.
    actions: Option<Rc<dyn DislActionExtension>>,
}

impl fmt::Debug for DislSpecification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DislSpecification({})", self.language_id())
    }
}

impl PartialEq for DislSpecification {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// A rule as declared; its prefix only when it is one string, which a map (top level only) is not.
fn rule_of(json: &Json) -> DislIdRule {
    DislIdRule {
        strategy: json::string_of(json, "strategy").unwrap_or("uuid-v7").to_owned(),
        expression: json::string_of(json, "expression").map(str::to_owned),
        prefix: json::string_of(json, "prefix").map(str::to_owned),
    }
}

impl DislSpecification {
    pub(crate) fn new(
        root: Json,
        metamodel: DislMetamodel,
        functions: Vec<DislFunctionDeclaration>,
        environment: Environment,
        expressions: Vec<DislExpression>,
        actions: Option<Rc<dyn DislActionExtension>>,
    ) -> Self {
        let index = expressions
            .iter()
            .enumerate()
            .map(|(i, e)| (e.pointer.clone(), i))
            .collect();
        let ids = json::get(&root, "persistence")
            .filter(|p| p.is_object())
            .and_then(|p| json::get(p, "ids"))
            .unwrap_or(&NOTHING)
            .clone();
        let id_types = json::members(&ids, "types")
            .map(|(n, t)| (n.clone(), rule_of(t)))
            .collect();
        let id_prefixes = json::members(&ids, "prefix")
            .filter_map(|(n, p)| p.as_str().map(|p| (n.clone(), p.to_owned())))
            .collect();
        DislSpecification(Rc::new(SpecificationData {
            ids: rule_of(&ids),
            root,
            metamodel,
            functions,
            environment,
            expressions,
            index,
            id_types,
            id_prefixes,
            compiled: RefCell::new(HashMap::new()),
            actions,
        }))
    }

    /// The runtime that runs `extension` actions, when one loaded the specification.
    pub(crate) fn action_extension(&self) -> Option<Rc<dyn DislActionExtension>> {
        self.0.actions.clone()
    }

    /// The specification's JSON, as it was read.
    pub fn root(&self) -> &Json {
        &self.0.root
    }

    /// The DISL version it targets (section 2.9): `0.1`, `0.2` or `0.3`.
    pub fn disl_version(&self) -> &str {
        json::string_of(&self.0.root, "disl")
            .or_else(|| json::string_of(&self.0.root, "dedl"))
            .unwrap_or("")
    }

    pub fn language_id(&self) -> &str {
        json::string_of(json::member(&self.0.root, "language"), "id").unwrap_or("")
    }

    /// The top-level `x-` properties, unread and unchanged.
    pub fn extensions(&self) -> Vec<(String, Json)> {
        json::extensions(&self.0.root)
    }

    /// The metamodel (section 4), inheritance flattened.
    pub fn metamodel(&self) -> &DislMetamodel {
        &self.0.metamodel
    }

    /// The user functions (section 3.4), in declaration order.
    pub fn functions(&self) -> &[DislFunctionDeclaration] {
        &self.0.functions
    }

    /// Every compiled expression outside the functions, in document order.
    pub fn expressions(&self) -> &[DislExpression] {
        &self.0.expressions
    }

    /// The per-evaluation cost limit (section 2.5): `language.limits.celCost`, one million by default.
    pub fn cost_limit(&self) -> i64 {
        self.0.environment.budget
    }

    /// The compiled expression at `pointer`.
    pub fn expression_at(&self, pointer: &str) -> Option<&DislExpression> {
        self.0.index.get(pointer).map(|i| &self.0.expressions[*i])
    }

    /// An environment for `context` (section 12.3): the library, the user functions and the context's variables,
    /// plus `bindings`; for compiling what a runtime builds itself.
    pub fn environment(&self, context: &str, bindings: &[&str]) -> Environment {
        let mut environment = self.0.environment.clone();
        environment.declare_variables(contexts::variables_of(context).iter().copied());
        environment.declare_variables(bindings.iter().copied());
        environment
    }

    /// The id rule that applies to `type_name` (section 11.5.2): its own, the nearest in its linearisation, or the
    /// top-level one; with the prefix it gives the type (section 11.5): the rule's own, else the top level's for the
    /// nearest type in the linearisation that the map names.
    pub fn id_rule_of(&self, type_name: &str) -> DislIdRule {
        let linearisation: Vec<String> = self
            .0
            .metamodel
            .type_of(type_name)
            .map(|t| t.linearisation.clone())
            .unwrap_or_else(|| vec![type_name.to_owned()]);
        let rule = linearisation
            .iter()
            .find_map(|c| self.0.id_types.get(c))
            .unwrap_or(&self.0.ids)
            .clone();
        if rule.prefix.is_some() {
            return rule;
        }
        let prefix = linearisation
            .iter()
            .find_map(|c| self.0.id_prefixes.get(c).cloned())
            .or_else(|| self.0.ids.prefix.clone());
        DislIdRule { prefix, ..rule }
    }

    /// The setting of the built-in `id` in `constraints.builtIn` (section 8.1); nothing when it has none.
    pub(crate) fn built_in_setting(&self, id: &str) -> &Json {
        let constraints = json::member(&self.0.root, "constraints");
        json::member(json::member(constraints, "builtIn"), id)
    }

    /// Whether the built-in `id` is reported (section 8.1): unless its setting's `enabled` is `false`.
    pub(crate) fn is_built_in_enabled(&self, id: &str) -> bool {
        let setting = self.built_in_setting(id);
        !matches!(json::get(setting, "enabled"), Some(Json::Bool(false)))
    }
}
