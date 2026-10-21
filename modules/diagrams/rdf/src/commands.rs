//! Every edit of the family, as a command on the project's history (etalii.adp spec 030, task T083), and the position
//! a drag stores in the registration.
//!
//! Ported from the standalone host at `8b6ccfc`: `Commands/RdfEditHandler.cs` and its commands (`AddRdfTripleCommand`,
//! `RemoveRdfTripleCommand`, `RemoveRdfResourceCommand`, `RenameRdfTermCommand`, `ReplaceRdfObjectLiteralCommand`,
//! `AddRdfPrefixCommand`), `Skos/SkosCommands.cs`, `Shacl/Commands/`, and the core's `DocumentEditRun.cs`,
//! `RestoreDocumentCommand.cs` and `SetRegistrationLayoutCommandHandler.cs`. Each edit runs on a copy of the body's
//! lines and its reading, and is written whole; its undo is the whole text as it was, deliberately, because these
//! operations touch several distant ranges at once and a snapshot cannot be wrong. The redo of that undo runs the edit
//! again against the restored text.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_history::{Applied, Command, Refusal};

use crate::lines::LineDocument;
use crate::model::{IriTerm, LiteralTerm, RdfModel, RdfTerm, RdfTriple};
use crate::parser;
use crate::registration_layout;
use crate::shacl::writer as shacl_writer;
use crate::store;
use crate::vocabulary::{rdf, skos};
use crate::writer;

/// One edit, as its standalone command said it.
#[derive(Debug, Clone, PartialEq)]
pub enum RdfEdit {
    /// One more triple; the object an IRI where `object_iri` is set, a literal otherwise (`AddRdfTripleCommand`).
    AddTriple {
        subject: String,
        predicate: String,
        object: ObjectValue,
    },
    /// One triple, matched by its three values (`RemoveRdfTripleCommand`).
    RemoveTriple {
        subject: String,
        predicate: String,
        object: ObjectValue,
    },
    /// A resource and every triple it is subject or object of (`RemoveRdfResourceCommand`).
    RemoveResource { iri: String },
    /// A term renamed everywhere (`RenameRdfTermCommand`).
    RenameTerm { old_iri: String, new_iri: String },
    /// One literal object rewritten in place (`ReplaceRdfObjectLiteralCommand`).
    ReplaceLiteral {
        subject: String,
        predicate: String,
        old: ObjectValue,
        new_lexical: String,
        new_language: String,
        new_datatype: String,
    },
    /// One more prefix declaration (`AddRdfPrefixCommand`).
    AddPrefix { prefix: String, iri: String },
    /// A concept with its type, preferred label and scheme (`AddSkosConceptCommand`).
    AddSkosConcept {
        concept: String,
        label: String,
        language: String,
        scheme: String,
    },
    /// Every asserted triple of a hierarchy or related pair (`DisconnectSkosPairCommand`).
    DisconnectSkosPair { a: String, b: String, hierarchy: bool },
    /// A property row on a shape, with the constraints the add-row dialog offers (`AddShaclPropertyRowCommand`).
    AddShaclPropertyRow {
        shape: String,
        path: String,
        options: shacl_writer::ShaclPropertyShapeOptions,
    },
    /// A target declared on a shape (`AddShaclTargetCommand`).
    AddShaclTarget {
        shape: String,
        predicate: String,
        term: String,
    },
    /// A target withdrawn (`RemoveShaclTargetCommand`).
    RemoveShaclTarget {
        shape: String,
        predicate: String,
        term: String,
    },
    /// A new node shape (`CreateShaclNodeShapeCommand`).
    CreateShaclNodeShape { shape: String },
    /// A shape switched off or on (`SetShaclDeactivatedCommand`).
    SetShaclDeactivated { shape: String, deactivated: bool },
    /// A shape's `sh:name` or `sh:description` (`SetShaclLiteralCommand`).
    SetShaclLiteral {
        shape: String,
        predicate: String,
        value: String,
    },
    /// A shape and the blank subtrees only it reaches (`RemoveShaclShapeCommand`).
    RemoveShaclShape { shape: String },
    /// The body's whole text put back (`RestoreDocumentCommand`).
    RestoreDocument { text: String },
}

/// An object as a command carries it: an IRI, or a literal's lexical form, language and datatype, empty for absent.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObjectValue {
    pub iri: String,
    pub lexical: String,
    pub language: String,
    pub datatype: String,
}

impl ObjectValue {
    pub fn iri(iri: impl Into<String>) -> Self {
        ObjectValue {
            iri: iri.into(),
            ..ObjectValue::default()
        }
    }

    pub fn literal(lexical: impl Into<String>, language: impl Into<String>, datatype: impl Into<String>) -> Self {
        ObjectValue {
            iri: String::new(),
            lexical: lexical.into(),
            language: language.into(),
            datatype: datatype.into(),
        }
    }

    /// The term it states.
    fn term(&self) -> RdfTerm {
        if !self.iri.is_empty() {
            return RdfTerm::Iri(IriTerm::new(self.iri.clone(), ""));
        }
        RdfTerm::Literal(LiteralTerm::new(
            self.lexical.clone(),
            (!self.datatype.is_empty()).then(|| self.datatype.clone()),
            (!self.language.is_empty()).then(|| self.language.clone()),
        ))
    }
}

/// The triple a command addresses, matched by its three values (`RdfEditHandler.TripleOf`).
pub fn triple_of<'a>(
    model: &'a RdfModel,
    subject: &str,
    predicate: &str,
    object: &ObjectValue,
) -> Option<&'a RdfTriple> {
    model.triples.iter().find(|t| {
        t.subject.is_iri(subject)
            && t.predicate.iri == predicate
            && if object.iri.is_empty() {
                t.object.as_literal().is_some_and(|l| {
                    l.lexical == object.lexical
                        && l.language.as_deref().unwrap_or_default() == object.language
                        && l.datatype.as_deref().unwrap_or_default() == object.datatype
                })
            } else {
                t.object.is_iri(&object.iri)
            }
    })
}

/// One edit of one body.
#[derive(Debug, Clone, PartialEq)]
pub struct RdfCommand {
    pub body_path: PathBuf,
    pub edit: RdfEdit,
}

impl RdfCommand {
    pub fn new(body_path: &Path, edit: RdfEdit) -> Self {
        RdfCommand {
            body_path: body_path.to_path_buf(),
            edit,
        }
    }

    /// The edit run (`DocumentEditRun.Run`): the body loaded, the edit made on a copy of its lines and their reading,
    /// the copy written, and the restore of the text before it as its undo.
    pub fn run(&self, documents: &mut DiagramDocuments) -> Result<RdfCommand, String> {
        let document = store::get_or_load(documents, &self.body_path);
        if let Some(reason) = &document.unreadable {
            return Err(format!(
                "{} could not be read, so nothing can be edited until it can: {reason}",
                store::file_name(&self.body_path)
            ));
        }
        let before = document.text.clone();
        let mut copy = LineDocument::parse(&before);
        let model = parser::parse(&copy).unwrap_or_default();
        let refusal = apply(&self.edit, &mut copy, &model);
        if !refusal.is_empty() {
            return Err(refusal);
        }
        store::save(documents, &self.body_path, &copy.text())?;
        Ok(RdfCommand {
            body_path: self.body_path.clone(),
            edit: RdfEdit::RestoreDocument { text: before },
        })
    }
}

/// Applies `edit` to `body`, whose reading is `model`: an empty string, or the refusal.
pub fn apply(edit: &RdfEdit, body: &mut LineDocument, model: &RdfModel) -> String {
    match edit {
        RdfEdit::AddTriple {
            subject,
            predicate,
            object,
        } => writer::add_triple(body, model, subject, predicate, &object.term()),
        RdfEdit::RemoveTriple {
            subject,
            predicate,
            object,
        } => match triple_of(model, subject, predicate, object) {
            Some(triple) => writer::remove_triple(body, model, &triple.clone()),
            None => "That triple is not in the file as it stands, so there is nothing to remove.".to_owned(),
        },
        RdfEdit::RemoveResource { iri } => writer::remove_resource(body, model, iri),
        RdfEdit::RenameTerm { old_iri, new_iri } => writer::rename_term(body, model, old_iri, new_iri),
        RdfEdit::ReplaceLiteral {
            subject,
            predicate,
            old,
            new_lexical,
            new_language,
            new_datatype,
        } => match triple_of(model, subject, predicate, old) {
            Some(triple) => writer::replace_object_literal(
                body,
                model,
                &triple.clone(),
                new_lexical,
                (!new_language.is_empty()).then_some(new_language.as_str()),
                (!new_datatype.is_empty()).then_some(new_datatype.as_str()),
            ),
            None => "That value is not in the file as it stands, so there is nothing to rewrite.".to_owned(),
        },
        RdfEdit::AddPrefix { prefix, iri } => writer::add_prefix(body, model, prefix, iri),
        RdfEdit::AddSkosConcept {
            concept,
            label,
            language,
            scheme,
        } => add_skos_concept(body, model, concept, label, language, scheme),
        RdfEdit::DisconnectSkosPair { a, b, hierarchy } => disconnect_skos_pair(body, model, a, b, *hierarchy),
        RdfEdit::AddShaclPropertyRow { shape, path, options } => {
            shacl_writer::append_property_shape_block(body, model, shape, path, options)
        }
        RdfEdit::AddShaclTarget { shape, predicate, term } => {
            shacl_writer::add_target(body, model, shape, predicate, &writer::iri_term(term))
        }
        RdfEdit::RemoveShaclTarget { shape, predicate, term } => {
            shacl_writer::remove_target(body, model, shape, predicate, term)
        }
        RdfEdit::CreateShaclNodeShape { shape } => shacl_writer::create_node_shape(body, model, shape),
        RdfEdit::SetShaclDeactivated { shape, deactivated } => {
            shacl_writer::set_deactivated(body, model, shape, *deactivated)
        }
        RdfEdit::SetShaclLiteral {
            shape,
            predicate,
            value,
        } => shacl_writer::set_literal(body, model, shape, predicate, value),
        RdfEdit::RemoveShaclShape { shape } => shacl_writer::remove_shape_with_subtrees(body, model, shape),
        RdfEdit::RestoreDocument { text } => {
            *body = LineDocument::parse(text);
            String::new()
        }
    }
}

/// Three splices under one restore: the type, the preferred label in the display language, and the scheme where one
/// is named, the model read again between them (`AddSkosConceptCommandHandler`).
fn add_skos_concept(
    body: &mut LineDocument,
    model: &RdfModel,
    concept: &str,
    label: &str,
    language: &str,
    scheme: &str,
) -> String {
    let refusal = writer::add_triple(body, model, concept, rdf::TYPE, &writer::iri_term(skos::CONCEPT));
    if !refusal.is_empty() {
        return refusal;
    }
    let reread = parser::parse(body).unwrap_or_default();
    let label = RdfTerm::Literal(LiteralTerm::new(
        label,
        None,
        (!language.is_empty()).then(|| language.to_owned()),
    ));
    let refusal = writer::add_triple(body, &reread, concept, skos::PREF_LABEL, &label);
    if !refusal.is_empty() || scheme.is_empty() {
        return refusal;
    }
    let reread = parser::parse(body).unwrap_or_default();
    writer::add_triple(body, &reread, concept, skos::IN_SCHEME, &writer::iri_term(scheme))
}

/// Every asserted direction of one pair, re-found on a fresh reading after each splice (`DisconnectSkosPairCommandHandler`).
fn disconnect_skos_pair(body: &mut LineDocument, model: &RdfModel, a: &str, b: &str, hierarchy: bool) -> String {
    let is =
        |t: &RdfTriple, s: &str, p: &str, o: &str| t.subject.is_iri(s) && t.predicate.iri == p && t.object.is_iri(o);
    let mut removed = 0;
    loop {
        let current = if removed == 0 {
            model.clone()
        } else {
            parser::parse(body).unwrap_or_default()
        };
        let triple = current
            .triples
            .iter()
            .find(|t| {
                if hierarchy {
                    is(t, a, skos::BROADER, b) || is(t, b, skos::NARROWER, a)
                } else {
                    is(t, a, skos::RELATED, b) || is(t, b, skos::RELATED, a)
                }
            })
            .cloned();
        let Some(triple) = triple else {
            return if removed > 0 {
                String::new()
            } else {
                "That connection is not in the file as it stands, so there is nothing to disconnect.".to_owned()
            };
        };
        let refusal = writer::remove_triple(body, &current, &triple);
        if !refusal.is_empty() {
            return refusal;
        }
        removed += 1;
    }
}

fn label_of(edit: &RdfEdit) -> &'static str {
    match edit {
        RdfEdit::AddTriple { .. } => "Add statement",
        RdfEdit::RemoveTriple { .. } => "Remove statement",
        RdfEdit::RemoveResource { .. } => "Remove resource",
        RdfEdit::RenameTerm { .. } => "Rename",
        RdfEdit::ReplaceLiteral { .. } | RdfEdit::SetShaclLiteral { .. } => "Change",
        RdfEdit::AddPrefix { .. } => "Declare prefix",
        RdfEdit::AddSkosConcept { .. } => "Add concept",
        RdfEdit::DisconnectSkosPair { .. } => "Disconnect",
        RdfEdit::AddShaclPropertyRow { .. } => "Add property row",
        RdfEdit::AddShaclTarget { .. } => "Add target",
        RdfEdit::RemoveShaclTarget { .. } => "Remove target",
        RdfEdit::CreateShaclNodeShape { .. } => "Add node shape",
        RdfEdit::SetShaclDeactivated { .. } => "Change activation",
        RdfEdit::RemoveShaclShape { .. } => "Remove shape",
        RdfEdit::RestoreDocument { .. } => "Restore",
    }
}

impl Command<DiagramDocuments> for RdfCommand {
    fn label(&self) -> String {
        label_of(&self.edit).to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let inverse = self.run(documents).map_err(Refusal::new)?;
        Ok(Applied::undoable(Undo {
            inverse,
            change: self.clone(),
        }))
    }
}

/// The undo of a change: its recorded restore, whose own redo is the change itself (`HistoryStack.UndoAsync`).
struct Undo {
    inverse: RdfCommand,
    change: RdfCommand,
}

/// The redo of a change: the change run again, whose undo is the restore recorded the first time.
struct Redo {
    inverse: RdfCommand,
    change: RdfCommand,
}

impl Command<DiagramDocuments> for Undo {
    fn label(&self) -> String {
        self.change.label()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        self.inverse.run(documents).map_err(Refusal::new)?;
        Ok(Applied::undoable(Redo {
            inverse: self.inverse.clone(),
            change: self.change.clone(),
        }))
    }
}

impl Command<DiagramDocuments> for Redo {
    fn label(&self) -> String {
        self.change.label()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        self.change.run(documents).map_err(Refusal::new)?;
        Ok(Applied::undoable(Undo {
            inverse: self.inverse.clone(),
            change: self.change.clone(),
        }))
    }
}

/// The refusal of a registration that is gone.
pub const REGISTRATION_GONE: &str = "The registration file is no longer there.";

/// Stores one element's position in its registration's `layout:` block (`SetRegistrationLayoutCommand`).
#[derive(Debug, Clone, PartialEq)]
pub struct SetRegistrationLayout {
    pub registration: PathBuf,
    pub element_id: String,
    pub x: f64,
    pub y: f64,
}

/// Removes one element's entry from the `layout:` block, the inverse of a first position.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveRegistrationLayout {
    pub registration: PathBuf,
    pub element_id: String,
}

fn read_registration(registration: &Path) -> Result<String, Refusal> {
    if !registration.is_file() {
        return Err(Refusal::new(REGISTRATION_GONE));
    }
    fs::read(registration)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|e| Refusal::new(format!("Could not store the position: {e}")))
}

impl Command<DiagramDocuments> for SetRegistrationLayout {
    fn label(&self) -> String {
        "Move".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let text = read_registration(&self.registration)?;
        let (written, prior) = registration_layout::set_position(&text, &self.element_id, (self.x, self.y));
        store::save_registration(documents, &self.registration, &written).map_err(Refusal::new)?;
        Ok(match prior {
            Some((x, y)) => Applied::undoable(SetRegistrationLayout { x, y, ..self.clone() }),
            None => Applied::undoable(RemoveRegistrationLayout {
                registration: self.registration.clone(),
                element_id: self.element_id.clone(),
            }),
        })
    }
}

impl Command<DiagramDocuments> for RemoveRegistrationLayout {
    fn label(&self) -> String {
        "Move".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let text = read_registration(&self.registration)?;
        let Some(&(x, y)) = registration_layout::read(&text).get(&self.element_id) else {
            return Err(Refusal::new("That element has no stored position any more."));
        };
        let written = registration_layout::remove_position(&text, &self.element_id).unwrap_or(text);
        store::save_registration(documents, &self.registration, &written)
            .map_err(|e| Refusal::new(e.replace("store", "remove")))?;
        Ok(Applied::undoable(SetRegistrationLayout {
            registration: self.registration.clone(),
            element_id: self.element_id.clone(),
            x,
            y,
        }))
    }
}
