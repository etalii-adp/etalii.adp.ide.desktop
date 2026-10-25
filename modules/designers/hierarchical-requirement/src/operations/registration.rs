//! One change of a list the registration keeps, the tags, categories, states or filters, made in the same step as the
//! body's (the standalone host's `RegistrationEdit` at a7fdf03; spec 029 R17; FBL 0.7 section 8.4). A registration the
//! body does not have yet is created first, outside the edit, as the binding's `createOnFirstPlacement` says; the
//! first change of a list its block does not hold writes the whole list, its members as the document has them with
//! the change applied (DESL 0.2 sections 5.5 and 7.7.2).

use std::rc::Rc;

use adp_fbl::registration::create_registration;
use adp_fbl::{ModelChange, OpenRegistration, PlanResult, Setting};

use super::lists::ListFiles;
use crate::definition::{self, ORIGIN};
use crate::model::Document;

/// The registration's bytes before and after `setting`: before is the registration as it is, or as it is created
/// when the body has none; or the sentence saying why it cannot be written.
pub fn apply(files: ListFiles<'_>, document: &Document, setting: Setting) -> Result<(Vec<u8>, Vec<u8>), String> {
    let binding = definition::binding();
    let created = create_registration(
        &binding,
        ORIGIN,
        files.registration_name,
        files.body_name,
        files.body,
        files.registration,
    )?;
    let mut open = OpenRegistration::open_with(created.clone(), &binding);
    let document = document.clone();
    open.absent_members = Some(Rc::new(move |block: &str| {
        let listed = members(&document, block);
        (!listed.is_empty()).then_some(listed)
    }));
    match open.change(&ModelChange::Setting(setting)) {
        PlanResult::Planned(_) => Ok((created, open.bytes().to_vec())),
        PlanResult::Refused(reason) | PlanResult::Invalid(reason) => Err(reason),
    }
}

/// A list's members as the document has them, by block, implicit ones included: what a first change of an absent
/// block writes (DESL 0.2 sections 5.5 and 7.7.2). The filters have no initial or implicit members, so an absent block
/// of them has none to write.
pub fn members(document: &Document, block: &str) -> Vec<(String, String)> {
    let list = match block {
        "tags" => &document.tags,
        "categories" => &document.categories,
        "states" => &document.states,
        _ => return Vec::new(),
    };
    list.iter().map(|m| (m.name.clone(), m.colour.clone())).collect()
}
