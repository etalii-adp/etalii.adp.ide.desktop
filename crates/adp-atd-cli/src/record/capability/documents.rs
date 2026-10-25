//! The documents a capability's fixtures are recorded on: every parity script of a tool type, opened by the desktop
//! host's module as the script opens it, and again as its last step leaves it, which is the standalone host's golden.

use adp_disl::DislDiagram;
use adp_parity::{Case, Observed};
use adp_theme::Kind;
use serde_json::Value as Json;

/// One document as the module read it.
pub struct Opened {
    /// The fixture's name: the tool's, the script's, and `read` or `golden`.
    pub name: String,
    /// Where the document comes from, for the fixture's `doc`.
    pub source: String,
    pub diagram: DislDiagram,
    pub body: Vec<u8>,
    /// What the canvas draws of it, where it was asked for.
    pub drawing: Option<Json>,
}

/// Every parity script of `origin`, opened and as its last step leaves it; with `drawing`, each with what the canvas
/// draws.
pub fn opened(origin: &str, drawing: bool) -> Result<Vec<Opened>, String> {
    let tool = origin.rsplit('/').next().unwrap_or(origin);
    let names: Vec<String> = adp_parity::script_names()
        .into_iter()
        .filter(|name| name.rsplit_once('/').is_some_and(|(of, _)| of == origin))
        .collect();
    if names.is_empty() {
        return Err(format!("no parity script records `{origin}`"));
    }
    let mut all = Vec::new();
    for name in names {
        let case = Case::load(&name)?;
        let script = name.rsplit('/').next().unwrap_or(&name).to_owned();
        let mut read: Option<Opened> = None;
        let mut last: Option<Opened> = None;
        let verdict = adp_parity::replay_observed(&case, &mut |observed: &Observed| {
            let Some(document) = observed.documents.get(observed.session.body_path()) else {
                return;
            };
            let seen = Opened {
                name: String::new(),
                source: String::new(),
                diagram: document.model().diagram.clone(),
                body: document.body_bytes().to_vec(),
                drawing: drawing
                    .then(|| {
                        adp_shell::recording::drawing_of(origin, observed.session, observed.documents, Kind::Light).ok()
                    })
                    .flatten(),
            };
            if observed.step.is_none() {
                read = Some(seen);
            } else {
                last = Some(seen);
            }
        })?;
        if !verdict.matches() {
            return Err(format!(
                "{name}: the replay does not end in the golden's bytes:\n{verdict}"
            ));
        }
        if let Some(mut opened) = read {
            opened.name = format!("{tool}-{script}-read");
            opened.source = format!(
                "parity/fixtures/{} as parity/scripts/{name}.json opens it",
                case.script.fixture
            );
            all.push(opened);
        }
        if let Some(mut closed) = last {
            closed.name = format!("{tool}-{script}-golden");
            closed.source = format!("parity/goldens/{name}/, as the last step of parity/scripts/{name}.json leaves it");
            all.push(closed);
        }
    }
    Ok(all)
}

/// `value` rounded to 0.01, with `-0` read as `0`.
pub fn round2(value: f64) -> f64 {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}
