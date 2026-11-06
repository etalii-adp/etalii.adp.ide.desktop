//! A designer's settings kept in the registration (DESL section 5.5, FBL section 8.4; etalii.adp spec 076, T059): each
//! `persistence.settings` entry that names a `type` reads the entries of its declared block as elements of that type,
//! its `key` and its `value` the entry's, or its `initial` members while the block is absent; one that names an
//! `attribute` reads that bool as true for an element whose id the block keeps. A change to such an element or
//! attribute is a change of the block, written to the registration.

use adp_cel::{Map, Value};
use adp_disl::{DislChange, DislDiagram, DislSpecification};
use adp_fbl::model::{FblElement, FblModel};
use adp_fbl::planning::{ModelChange, Setting, SettingPosition};
use adp_fbl::registration::RegistrationDocument;
use adp_fbl::text::Span;
use serde_json::Value as Json;

use crate::document::text_of;

/// A setting of a type: its block, its type, the attributes its entries' keys and values are, and its initial members.
struct TypeSetting<'a> {
    block: &'a str,
    type_name: &'a str,
    key: &'a str,
    value: &'a str,
    initial: &'a [Json],
}

/// A setting of a bool attribute: its block and the attribute.
struct AttributeSetting<'a> {
    block: &'a str,
    attribute: &'a str,
    of: Vec<&'a str>,
}

fn type_settings(specification: &DislSpecification) -> Vec<TypeSetting<'_>> {
    let Some(settings) = specification
        .root()
        .pointer("/persistence/settings")
        .and_then(Json::as_object)
    else {
        return Vec::new();
    };
    settings
        .iter()
        .filter_map(|(block, s)| {
            Some(TypeSetting {
                block,
                type_name: s["type"].as_str()?,
                key: s["key"].as_str()?,
                value: s["value"].as_str()?,
                initial: s["initial"].as_array().map_or(&[][..], Vec::as_slice),
            })
        })
        .collect()
}

fn attribute_settings(specification: &DislSpecification) -> Vec<AttributeSetting<'_>> {
    let Some(settings) = specification
        .root()
        .pointer("/persistence/settings")
        .and_then(Json::as_object)
    else {
        return Vec::new();
    };
    settings
        .iter()
        .filter_map(|(block, s)| {
            Some(AttributeSetting {
                block,
                attribute: s["attribute"].as_str()?,
                of: s["of"].as_array()?.iter().filter_map(Json::as_str).collect(),
            })
        })
        .collect()
}

/// The registration's settings read into `fbl`: an element of each entry of a type's block, and the attribute a block
/// keeps on the elements it names.
pub fn read_into(fbl: &mut FblModel, specification: &DislSpecification, registration: Option<&RegistrationDocument>) {
    let block = |name: &str| registration.and_then(|r| r.blocks.iter().find(|b| b.name == name));
    for setting in attribute_settings(specification) {
        let Some(found) = block(setting.block) else { continue };
        for element in fbl
            .elements
            .iter_mut()
            .filter(|e| setting.of.contains(&e.type_name.as_str()))
        {
            if found
                .entries
                .iter()
                .any(|entry| entry.key == element.id && entry.value == "yes")
            {
                element.attributes.insert(setting.attribute, Value::Bool(true));
            }
        }
    }
    for setting in type_settings(specification) {
        let entries: Vec<(String, String)> = match block(setting.block) {
            Some(found) => found.entries.iter().map(|e| (e.key.clone(), e.value.clone())).collect(),
            None => setting
                .initial
                .iter()
                .filter_map(|member| {
                    Some((
                        member[setting.key].as_str()?.to_owned(),
                        member[setting.value]
                            .as_str()
                            .map(str::to_owned)
                            .or_else(|| member[setting.value].as_i64().map(|n| n.to_string()))
                            .unwrap_or_default(),
                    ))
                })
                .collect(),
        };
        for (key, value) in entries {
            let mut attributes = Map::default();
            attributes.insert(setting.key, Value::String(key.clone()));
            attributes.insert(setting.value, Value::String(value));
            fbl.elements.push(FblElement {
                id: format!("{}:{key}", setting.block),
                id_is_stored: false,
                type_name: setting.type_name.to_owned(),
                rule: String::new(),
                is_relation: false,
                attributes,
                parent_id: None,
                parent_slot: None,
                source: None,
                target: None,
                referenced: Vec::new(),
                own_span: Span::default(),
                line: 0,
                file: None,
            });
        }
    }
}

/// `change` as changes of the registration's blocks, when it changes what a setting keeps: none when it changes the
/// body alone. A `Set` that also sets attributes of the body hands those back as the change that stays.
pub fn as_settings(
    specification: &DislSpecification,
    diagram: &DislDiagram,
    registration: Option<&RegistrationDocument>,
    change: &DislChange,
) -> Option<(Vec<ModelChange>, Option<DislChange>)> {
    let has_block = |name: &str| registration.is_some_and(|r| r.blocks.iter().any(|b| b.name == name));
    let setting = |block: &str, key: &str| Setting {
        block: block.to_owned(),
        key: key.to_owned(),
        ..Setting::default()
    };
    let type_of = |id: &str| diagram.element_by_id(id).map(|e| e.type_name());
    // The initial members are written with a block's first change (DESL section 5.5).
    let first_write = |s: &TypeSetting<'_>| -> Vec<ModelChange> {
        if has_block(s.block) {
            return Vec::new();
        }
        diagram
            .nodes_of_type(s.type_name)
            .iter()
            .map(|member| {
                ModelChange::Setting(Setting {
                    value: Some(text_of(&member.value_of(s.value))),
                    ..setting(s.block, &text_of(&member.value_of(s.key)))
                })
            })
            .collect()
    };
    for s in type_settings(specification) {
        match change {
            DislChange::Create {
                type_name, attributes, ..
            } if type_name == s.type_name => {
                let mut changes = first_write(&s);
                changes.push(ModelChange::Setting(Setting {
                    value: Some(attributes.get(s.value).map(text_of).unwrap_or_default()),
                    ..setting(s.block, &attributes.get(s.key).map(text_of).unwrap_or_default())
                }));
                return Some((changes, None));
            }
            DislChange::Set {
                element_id, attributes, ..
            } if type_of(element_id).as_deref() == Some(s.type_name) => {
                let element = diagram.element_by_id(element_id)?;
                let mut key = text_of(&element.value_of(s.key));
                let mut changes = first_write(&s);
                if let Some(value) = attributes.get(s.value) {
                    changes.push(ModelChange::Setting(Setting {
                        value: Some(text_of(value)),
                        ..setting(s.block, &key)
                    }));
                }
                if let Some(renamed) = attributes.get(s.key) {
                    let renamed = text_of(renamed);
                    changes.push(ModelChange::Setting(Setting {
                        rename: Some(renamed.clone()),
                        ..setting(s.block, &key)
                    }));
                    key = renamed;
                }
                let _ = key;
                return Some((changes, None));
            }
            DislChange::Remove { element_id } if type_of(element_id).as_deref() == Some(s.type_name) => {
                let element = diagram.element_by_id(element_id)?;
                let mut changes = first_write(&s);
                changes.push(ModelChange::Setting(Setting {
                    remove_entry: true,
                    ..setting(s.block, &text_of(&element.value_of(s.key)))
                }));
                return Some((changes, None));
            }
            DislChange::Reparent { element_id, index, .. } if type_of(element_id).as_deref() == Some(s.type_name) => {
                let element = diagram.element_by_id(element_id)?;
                let mut changes = first_write(&s);
                changes.push(ModelChange::Setting(Setting {
                    position: Some(SettingPosition {
                        index: Some(*index),
                        ..SettingPosition::default()
                    }),
                    ..setting(s.block, &text_of(&element.value_of(s.key)))
                }));
                return Some((changes, None));
            }
            _ => {}
        }
    }
    for s in attribute_settings(specification) {
        let DislChange::Set {
            element_id,
            type_name,
            attributes,
        } = change
        else {
            continue;
        };
        let Some(value) = attributes.get(s.attribute) else {
            continue;
        };
        if !s.of.contains(&type_name.as_str()) {
            continue;
        }
        let written = match diagram.element_by_id(element_id).and_then(|e| e.read_id()) {
            Some(read) => read,
            None => element_id.clone(),
        };
        let kept = if *value == Value::Bool(true) || text_of(value) == "true" {
            Setting {
                value: Some("yes".into()),
                ..setting(s.block, &written)
            }
        } else {
            Setting {
                remove_entry: true,
                ..setting(s.block, &written)
            }
        };
        let mut rest = attributes.clone();
        rest.remove(s.attribute);
        let stays = (!rest.is_empty()).then(|| DislChange::Set {
            element_id: element_id.clone(),
            type_name: type_name.clone(),
            attributes: rest,
        });
        return Some((vec![ModelChange::Setting(kept)], stays));
    }
    None
}
