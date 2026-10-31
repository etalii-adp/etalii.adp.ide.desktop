//! Plans a model change as one edit (FBL section 6.4; the standalone host's `Planning/EditPlanner`): the splices of
//! every attribute set, element added, moved or removed, in the body's own conventions, or the reason the change
//! cannot be made.

use adp_cel::{Map, Value};

use crate::documents::types::{AttributeBinding, Rule, Slot};
use crate::planning::new_text;
use crate::planning::plan::{Plan, Planning, invalid, refuse};
use crate::planning::{ModelChange, PlanResult, PlannedEdit};
use crate::rules::reading::BodyReading;
use crate::rules::{InsertRequest, SlotChange, SlotRead};
use crate::splice::Operation;
use crate::text::Span;

pub(crate) fn plan(reading: &BodyReading, change: &ModelChange) -> PlanResult {
    if reading.unreadable().is_some() {
        // A save writes the bytes that were read, which an unreadable body keeps (FBL sections 6.6, 7.5): nothing to
        // splice.
        if matches!(change, ModelChange::Save) {
            return PlanResult::Planned(PlannedEdit::default());
        }
        return PlanResult::Refused("The file could not be read, so it is never written.".into());
    }
    if let Some(reason) = &reading.binding_rc().read_only {
        return PlanResult::Refused(if reason.is_empty() {
            "This file is read-only.".into()
        } else {
            reason.clone()
        });
    }
    let mut plan = Plan::new(reading);
    let planned = match change {
        ModelChange::Save => Ok(()),
        ModelChange::Set { id, attributes } => plan_set(&mut plan, id, attributes),
        ModelChange::Add {
            type_name,
            id,
            attributes,
            parent_id,
            index,
            slot,
        } => plan_add(
            &mut plan,
            type_name,
            id.as_deref(),
            attributes,
            parent_id.as_deref(),
            *index,
            slot.as_deref(),
        ),
        ModelChange::AddAfter {
            type_name,
            attributes,
            after,
        } => plan_add_after(&mut plan, type_name, attributes, after),
        ModelChange::Remove { id } => plan_remove(&mut plan, id),
        ModelChange::Move {
            id,
            new_parent_id,
            index,
        } => plan_move(&mut plan, id, new_parent_id.as_deref(), *index),
        ModelChange::StoreId { id, new_id } => plan_store_id(&mut plan, id, new_id),
        ModelChange::Retype { .. } => {
            return PlanResult::Refused(
                "This file's binding cannot change an element's type; only a persistence plugin can.".into(),
            );
        }
        ModelChange::Place { .. } | ModelChange::Identify { .. } | ModelChange::Setting(_) => {
            return PlanResult::Invalid(
                "Placements, stored ids and settings are edits of the registration, not of the body.".into(),
            );
        }
    };
    match planned {
        Ok(()) => match plan.ordered() {
            Ok(splices) => PlanResult::Planned(PlannedEdit {
                splices,
                snapshot: plan.snapshot,
            }),
            Err(overlap) => PlanResult::Invalid(overlap),
        },
        Err(refused) if refused.invalid => PlanResult::Invalid(refused.reason),
        Err(refused) => PlanResult::Refused(refused.reason),
    }
}

fn element(reading: &BodyReading, id: &str) -> Planning<usize> {
    match reading.find(id) {
        Some(index) => Ok(index),
        None => invalid(format!("The model has no element or relation '{id}'.")),
    }
}

fn refuse_read_only(rule: &Rule, gesture: &str) -> Planning {
    if let Some(reason) = &rule.read_only {
        return refuse(if reason.is_empty() {
            format!("A {} cannot be {gesture} in this file.", rule.type_name)
        } else {
            reason.clone()
        });
    }
    Ok(())
}

// ---- set ----

/// The id of an entry that has none, written where its rule reads it from.
fn plan_store_id(plan: &mut Plan<'_>, id: &str, new_id: &str) -> Planning {
    let reading = plan.reading;
    let index = element(reading, id)?;
    let element = reading.element(index);
    let rule = reading.family.rule(element.rule);
    refuse_read_only(rule, "changed")?;
    let Some(from) = rule.id.as_ref().and_then(|i| i.from.as_ref()) else {
        return refuse(format!("A {} keeps no id of its own in this file.", rule.type_name));
    };
    if element.id_read.as_ref().is_some_and(|r| r.present) {
        return refuse(format!("This {} has an id already.", rule.type_name));
    }
    if new_id.is_empty() || reading.find(new_id).is_some() {
        return refuse(format!("'{new_id}' cannot be the id of a {} here.", rule.type_name));
    }
    // The slot an id is read from, written as an attribute's is: the family puts it where an absent value goes.
    let binding = AttributeBinding::of_slot(Slot {
        key: from.key.clone(),
        xml_attribute: from.xml_attribute.clone(),
        child: from.child.clone(),
        group: from.group.clone(),
        node: from.node.clone(),
        node_match: from.node_match.clone(),
        escape: from.escape,
        ..Slot::default()
    });
    let change = SlotChange {
        attribute: "id".into(),
        binding: &binding,
        rule: element.rule,
        read: element.id_read.as_deref().cloned().unwrap_or_else(SlotRead::absent),
        value: Value::from(new_id),
        is_empty: false,
    };
    reading.family.write(plan, element, &[change])
}

fn plan_set(plan: &mut Plan<'_>, id: &str, attributes: &Map) -> Planning {
    let reading = plan.reading;
    let index = element(reading, id)?;
    let element = reading.element(index);
    let element_rule = reading.family.rule(element.rule);
    refuse_read_only(element_rule, "changed")?;
    // An id read from the entry (FBL section 3.3) is rewritten where it was read.
    let id_binding = element_rule.id.as_ref().and_then(|i| i.from.as_ref()).map(|from| {
        AttributeBinding::of_slot(Slot {
            key: from.key.clone(),
            xml_attribute: from.xml_attribute.clone(),
            text: from.text,
            child: from.child.clone(),
            group: from.group.clone(),
            capture: from.capture.clone(),
            node: from.node.clone(),
            node_match: from.node_match.clone(),
            escape: from.escape,
            ..Slot::default()
        })
    });
    let mut changes: Vec<SlotChange<'_>> = Vec::new();
    let mut new_key: Option<String> = None;
    for (attribute, requested) in attributes.iter() {
        let mut value = requested.clone();
        let (rule, binding, read): (usize, Option<&AttributeBinding>, SlotRead);
        if element.is_relation && (attribute == "source" || attribute == "target") {
            let source = attribute == "source";
            binding = if source {
                element_rule.source.as_ref()
            } else {
                element_rule.target.as_ref()
            };
            let end = self::element(reading, &new_text::plain(&value, None))?;
            value = Value::from(reading.element(end).key.as_str());
            read = if source {
                &element.source_read
            } else {
                &element.target_read
            }
            .as_deref()
            .cloned()
            .unwrap_or_else(SlotRead::absent);
            rule = element.rule;
        } else {
            let (r, b) = binding_of(reading, index, attribute);
            rule = r;
            value = referenced_value(reading, reading.family.rule(r), attribute, &value);
            binding = b.or(if attribute == "id" { id_binding.as_ref() } else { None });
            read = match b {
                None if binding.is_some() => element.id_read.as_deref().cloned().unwrap_or_else(SlotRead::absent),
                None => SlotRead::absent(),
                Some(b) => {
                    if r == element.rule
                        && let Some(known) = element.slot(attribute)
                    {
                        known.clone()
                    } else {
                        let mut candidate = element.candidate.clone();
                        candidate.rule = Some(r);
                        reading.read_slot(&candidate, &b.slot, Some(b), element.rule)
                    }
                }
            };
        }
        let Some(binding) = binding else {
            return refuse(format!(
                "A {} keeps no '{attribute}' in this file.",
                element_rule.type_name
            ));
        };
        if binding.is_computed() || binding.parent.is_some() || !read.writable {
            return refuse(match read.reason.as_deref() {
                Some(reason) if !reason.is_empty() => reason.to_owned(),
                _ => format!(
                    "The {attribute} of a {} cannot be changed in this file.",
                    element_rule.type_name
                ),
            });
        }
        // An attribute with a secondary slot writes its own and removes the secondary (FBL 0.10 section 5.2).
        let mut read = read;
        let mut secondary = None;
        if let Some(binding) = binding.secondary.as_deref() {
            let second = reading.family.read(&element.candidate, binding, false);
            if second.present && !second.writable {
                return refuse(second.reason.unwrap_or_default());
            }
            secondary = second.present.then(|| SlotChange {
                attribute: attribute.clone(),
                binding,
                rule,
                read: second,
                value: Value::Null,
                is_empty: true,
            });
        }
        if binding.secondary.is_some() {
            read = reading.family.read(&element.candidate, binding, false);
        }
        // A value a grammar slot's `map` writes as no text is its slot's absence (FBL 0.12 section 4.9).
        let unwritten = reading.family.family_name() == "grammar"
            && new_text::wire(binding, &value, None).is_some_and(|wire| wire.is_empty());
        let empty = new_text::is_empty(&value) || unwritten || (binding.flag && matches!(value, Value::Bool(false)));
        if empty && !binding.flag && binding.empty.as_deref() == Some("refuse") {
            return refuse(
                binding
                    .refusal_empty
                    .clone()
                    .unwrap_or_else(|| format!("The {attribute} of a {} cannot be empty.", element_rule.type_name)),
            );
        }
        if !read.present
            && !empty
            && !binding.flag
            && binding
                .absent
                .iter()
                .any(|(family, absent)| family == reading.family.family_name() && absent == "refuse")
        {
            return refuse(binding.refusal_absent.clone().unwrap_or_else(|| {
                format!(
                    "The {} file has no \"{}\" to rewrite.",
                    reading.binding_rc().name,
                    slot_name(binding)
                )
            }));
        }
        if empty && !read.present && !binding.flag {
            changes.extend(secondary);
            continue;
        }
        if !empty && is_key(reading, index, attribute, binding) {
            let key = new_text::plain(&value, None);
            if key != element.key {
                // The same rules (FBL section 5.7): the element's own, and those a reference names together with it.
                let named_with = |other: usize| {
                    let other = &reading.family.rule(other).name;
                    reading.binding_rc().all_rules().iter().any(|r| {
                        r.attributes.iter().any(|(_, a)| {
                            a.reference
                                .as_ref()
                                .is_some_and(|to| to.to.contains(&element_rule.name) && to.to.contains(other))
                        })
                    })
                };
                if reading.elements().any(|e| {
                    !std::ptr::eq(e, element) && e.key == key && (e.rule == element.rule || named_with(e.rule))
                }) {
                    return refuse(format!("Another {} is already named '{key}'.", element_rule.type_name));
                }
                new_key = Some(key);
            }
        }
        changes.push(SlotChange {
            attribute: attribute.clone(),
            binding,
            rule,
            read,
            value,
            is_empty: empty,
        });
        changes.extend(secondary);
    }
    if !changes.is_empty() {
        reading.family.write(plan, element, &changes)?;
    }
    if let Some(new_key) = new_key {
        rewrite_references(plan, index, &new_key);
    }
    if element_rule.snapshot_undo {
        plan.snapshot = true;
    }
    Ok(())
}

fn slot_name(slot: &Slot) -> String {
    slot.key
        .clone()
        .or_else(|| slot.xml_attribute.clone())
        .or_else(|| slot.group.clone())
        .or_else(|| slot.capture.clone())
        .or_else(|| slot.child.clone())
        .unwrap_or_else(|| "value".into())
}

/// The binding that stores `attribute` for the element: its own rule's, else that of another rule that matches the
/// same entry (a timeline Moment given an end is written by the Period rule's binding, FBL section 5.1).
fn binding_of<'a>(reading: &'a BodyReading, index: usize, attribute: &str) -> (usize, Option<&'a AttributeBinding>) {
    let element = reading.element(index);
    let rules = &reading.binding_rc().rules;
    if let Some(own) = rules[element.rule].attribute(attribute) {
        return (element.rule, Some(own));
    }
    for (r, rule) in rules.iter().enumerate() {
        if r == element.rule {
            continue;
        }
        let Some(other) = rule.attribute(attribute) else {
            continue;
        };
        if reading.family.candidates(r).iter().any(|c| c.entry == element.entry()) {
            return (r, Some(other));
        }
    }
    (element.rule, None)
}

fn is_key(reading: &BodyReading, index: usize, attribute: &str, binding: &AttributeBinding) -> bool {
    let element = reading.element(index);
    if element.key_attribute.as_deref() == Some(attribute) {
        return true;
    }
    element.key_attribute.is_none()
        && reading
            .family
            .rule(element.rule)
            .id
            .as_ref()
            .and_then(|i| i.from.as_ref())
            .is_some_and(|from| same_slot(from, binding))
}

fn same_slot(a: &Slot, b: &Slot) -> bool {
    a.key == b.key
        && a.xml_attribute == b.xml_attribute
        && a.group == b.group
        && a.text == b.text
        && a.child == b.child
        && a.word == b.word
        && a.capture == b.capture
        && a.node == b.node
}

/// A rename (FBL section 5.7): every reference to the old value is rewritten in the same edit, one
/// `rewrite-reference` splice each, word by word inside a group.
fn rewrite_references(plan: &mut Plan<'_>, renamed: usize, new_key: &str) {
    let reading = plan.reading;
    let renamed_element = reading.element(renamed);
    let renamed_rule = reading.family.rule(renamed_element.rule);
    let old_key = renamed_element.key.clone();
    for &other_index in reading.element_indices() {
        let other = reading.element(other_index);
        if other.is_relation {
            if other.source_element == Some(renamed) {
                rewrite(plan, other.source_read.as_deref(), &old_key, new_key);
            }
            if other.target_element == Some(renamed) {
                rewrite(plan, other.target_read.as_deref(), &old_key, new_key);
            }
        }
        let other_rule = reading.family.rule(other.rule);
        for (name, binding) in &other_rule.attributes {
            let Some(reference) = &binding.reference else { continue };
            if !reference.to.contains(&renamed_rule.name) {
                continue;
            }
            if other_index == renamed && renamed_element.key_attribute.as_deref() == Some(name.as_str()) {
                continue;
            }
            let Some(read) = other.slot(name).filter(|r| r.present) else {
                continue;
            };
            let words = read.words.as_ref().map(|w| w.len()).unwrap_or(0);
            if words > 1 || (words == 1 && matches!(other.attributes.get(name), Some(Value::List(_)))) {
                for word in read.words.as_ref().expect("words").iter() {
                    if word.text != old_key {
                        continue;
                    }
                    let mut word_read = SlotRead::new(Value::from(word.text.as_str()), Some(word.span), true, true);
                    word_read.quote = word.quoted.then(|| "\"".to_owned());
                    rewrite(plan, Some(&word_read), &old_key, new_key);
                }
                continue;
            }
            rewrite(plan, Some(read), &old_key, new_key);
        }
    }
}

fn rewrite(plan: &mut Plan<'_>, read: Option<&SlotRead>, old_key: &str, new_key: &str) {
    let Some(read) = read.filter(|r| r.present) else { return };
    let Some(span) = read.span else { return };
    if read.written() != old_key || plan.touches(span) {
        return;
    }
    let text = plan.reading.family.format(read, None, &Value::from(new_key));
    plan.add_span(Operation::RewriteReference, span, text);
}

// ---- add ----

/// What attribute `name` of `rule` writes for `value`: a reference by a value is written as that value of what it names,
/// the model knowing it by its id (FBL section 5.7).
fn referenced_value(reading: &BodyReading, rule: &crate::documents::types::Rule, name: &str, value: &Value) -> Value {
    let Some(reference) = rule.attribute(name).and_then(|binding| binding.reference.as_ref()) else {
        return value.clone();
    };
    let Value::String(id) = value else {
        return value.clone();
    };
    if reference.by == "id" {
        return value.clone();
    }
    match reading.find(id).map(|found| reading.element(found)) {
        Some(element) => {
            let named = reading.family.rule(element.rule);
            if reference.to.contains(&named.name) {
                Value::String(element.text_of(named, &reference.by))
            } else {
                value.clone()
            }
        }
        None => value.clone(),
    }
}

fn plan_add(
    plan: &mut Plan<'_>,
    type_name: &str,
    id: Option<&str>,
    attributes: &Map,
    parent_id: Option<&str>,
    index: Option<i64>,
    slot: Option<&str>,
) -> Planning {
    let reading = plan.reading;
    let all = &reading.binding_rc().rules;
    let rules: Vec<usize> = (0..all.len())
        .filter(|&r| {
            all[r].type_name == type_name
                && reading.family.options.admits(&all[r].origins)
                && (slot.is_none() || all[r].parent.as_ref().and_then(|p| p.slot.as_deref()) == slot)
        })
        .collect();
    if rules.is_empty() {
        // A block rule that adds entries is named by an add as its type (FBL 0.11 section 4.9, *Views*).
        if let Some(block) = reading.binding_rc().blocks.iter().position(|b| b.name == type_name)
            && reading.family.family_name() == "grammar"
        {
            return reading.family.grammar_insert_block(plan, block, attributes);
        }
        return refuse(format!("This file has no place for a {type_name}."));
    }
    // Several rules may write one type. The one meant is a rule the parent given can contain, where any is, and
    // among those the one that binds the most of the attributes given, the first of them on a tie.
    let container = parent_id.and_then(|p| reading.find(p));
    let insertable: Vec<usize> = rules.iter().copied().filter(|&r| all[r].insert.is_some()).collect();
    let contained: Vec<usize> = insertable
        .iter()
        .copied()
        .filter(|&r| {
            container.is_some_and(|c| {
                let container_rule = &all[reading.element(c).rule].name;
                all[r].parent.as_ref().is_some_and(|p| p.rules.contains(container_rule))
            })
        })
        .collect();
    let choices = if contained.is_empty() { &insertable } else { &contained };
    let bound = |r: usize| {
        attributes
            .keys()
            .filter(|name| all[r].attribute(name).is_some())
            .count()
    };
    // What a rule writes of the attributes given: a relation's ends are not attributes of the new entry.
    let values_of = |r: usize| -> Map {
        attributes
            .iter()
            .filter(|(name, _)| !(all[r].is_relation && (*name == "source" || *name == "target")))
            .map(|(name, value)| (name.clone(), referenced_value(reading, &all[r], name, value)))
            .collect()
    };
    // An add is planned by a rule whose `insert.when` holds (FBL section 6.2).
    let holds = |r: usize| {
        all[r]
            .insert
            .as_ref()
            .and_then(|i| i.when.as_deref())
            .is_none_or(|when| reading.insert_allowed(when, &values_of(r)))
    };
    let mut best: Option<usize> = None;
    for &r in choices.iter().filter(|&&r| holds(r)) {
        if best.is_none_or(|b| bound(r) > bound(b)) {
            best = Some(r);
        }
    }
    let Some(rule_index) = best else {
        if let Some(&first) = choices.first() {
            refuse_read_only(&all[first], "added")?;
            let sentence = all[first].insert.as_ref().and_then(|i| i.refusal_when.clone());
            return refuse(sentence.unwrap_or_else(|| format!("This {type_name} cannot be added to the file.")));
        }
        return refuse(match all[rules[0]].read_only.as_deref() {
            Some(text) if !text.is_empty() => text.to_owned(),
            _ => format!("A {type_name} cannot be added to this file."),
        });
    };
    let rule = &all[rule_index];
    refuse_read_only(rule, "added")?;
    let values = values_of(rule_index);
    // A binding's own sentence for an empty value (FBL 0.10 section 6.4). Without one the add is planned as it was, a
    // departure from FBL 0.6 section 6.2 that `the_yaml_family_departs_from_the_fixture_only_as_recorded` records.
    for (name, value) in values.iter() {
        if let Some(binding) = rule.attribute(name)
            && binding.empty.as_deref() == Some("refuse")
            && new_text::is_empty(value)
            && let Some(sentence) = &binding.refusal_empty
        {
            return refuse(sentence.clone());
        }
    }
    let (mut source, mut target) = (None, None);
    if rule.is_relation {
        source = Some(end(reading, type_name, attributes, "source")?);
        target = Some(end(reading, type_name, attributes, "target")?);
    }
    let parent = match parent_id {
        Some(p) => Some(element(reading, p)?),
        None => None,
    };
    if let Some(parent) = parent
        && let Some(containment) = &rule.parent
    {
        let parent_rule = reading.family.rule(reading.element(parent).rule);
        if !containment.rules.contains(&parent_rule.name) {
            return refuse(format!(
                "A {type_name} cannot be placed inside a {}.",
                parent_rule.type_name
            ));
        }
    }
    if rule.id.as_ref().is_some_and(|i| i.from.is_some())
        && let Some(id) = id
        && reading.elements().any(|e| {
            reading
                .family
                .rule(e.rule)
                .id
                .as_ref()
                .is_some_and(|i| i.from.is_some())
                && e.id == id
        })
    {
        return refuse(format!("Another element already has the id '{id}'."));
    }
    let request = InsertRequest {
        rule: rule_index,
        id: id.map(str::to_owned),
        values,
        parent,
        source,
        target,
        index: index.filter(|_| reading.family.places_by_position()),
    };
    let mark = plan.mark();
    reading.family.insert(plan, &request)?;
    // A family whose nesting is its indentation places the entry at its index itself (FBL 0.7 section 6.3).
    if let Some(index) = index.filter(|_| !reading.family.places_by_position()) {
        let siblings = siblings_under(reading, parent, rule);
        place_at(plan, mark, &siblings, index, type_name)?;
    }
    if rule.snapshot_undo {
        plan.snapshot = true;
    }
    Ok(())
}

/// An add planned as its rule's insert says, and then put on a line of its own right after the entry `after`. Only an
/// entry of lines of its own can be put after another; one that cannot is refused.
fn plan_add_after(plan: &mut Plan<'_>, type_name: &str, attributes: &Map, after: &str) -> Planning {
    let reading = plan.reading;
    let anchor = element(reading, after)?;
    let mark = plan.mark();
    plan_add(plan, type_name, None, attributes, None, None, None)?;
    let family = &reading.family;
    let offset = family.entries[reading.element(anchor).entry()].line_span.map(|l| l.end);
    let added = plan.take_since(mark);
    let lines = match (offset, added.as_slice()) {
        (Some(_), [splice]) if splice.operation == Operation::InsertEntry && splice.start == splice.end => {
            family.as_lines(splice.start, &splice.text)
        }
        _ => None,
    };
    let (Some(offset), Some(lines)) = (offset, lines) else {
        return refuse(format!(
            "This file's binding cannot add a {type_name} at that position."
        ));
    };
    let text = family.lines_at(offset, &lines);
    plan.add(Operation::InsertEntry, offset, offset, text);
    Ok(())
}

fn end(reading: &BodyReading, type_name: &str, attributes: &Map, end: &str) -> Planning<usize> {
    let value = attributes.get(end).filter(|v| !v.is_null());
    let Some(value) = value else {
        return refuse(format!("A {type_name} needs a {end}."));
    };
    let id = new_text::plain(value, None);
    match reading.find(&id) {
        Some(e) if !reading.element(e).is_relation => Ok(e),
        _ => refuse(format!("The {end} '{id}' names no element.")),
    }
}

/// Puts the entry an add just planned at `index` among `siblings` (FBL section 6.4): directly before the entry of
/// the sibling at that index, or after the last sibling's entry when the index is their number, negative or past
/// them. With no sibling the entry stays where the rule's insert put it (FBL section 6.3). Only an entry of lines
/// of its own can be put before another; one that cannot, such as a json item with its separator, is refused there.
fn place_at(plan: &mut Plan<'_>, mark: usize, siblings: &[usize], index: i64, type_name: &str) -> Planning {
    let reading = plan.reading;
    let Some(&last) = siblings.last() else {
        return Ok(());
    };
    let before = usize::try_from(index).ok().and_then(|i| siblings.get(i).copied());
    let family = &reading.family;
    let offset = match before {
        Some(b) => family.entries[reading.element(b).entry()].line_span.map(|l| l.start),
        None => family.entries[reading.element(last).entry()].line_span.map(|l| l.end),
    };
    let mut added = plan.take_since(mark);
    let lines = match (offset, added.as_slice()) {
        (Some(_), [splice]) if splice.operation == Operation::InsertEntry && splice.start == splice.end => {
            family.as_lines(splice.start, &splice.text)
        }
        _ => None,
    };
    match (offset, lines) {
        (Some(offset), Some(lines)) => {
            let text = family.lines_at(offset, &lines);
            plan.add(Operation::InsertEntry, offset, offset, text);
            Ok(())
        }
        // After the last sibling is where the rule's insert puts a new entry in such a body.
        _ if before.is_none() => {
            for splice in added.drain(..) {
                plan.add(splice.operation, splice.start, splice.end, splice.text);
            }
            Ok(())
        }
        _ => refuse(format!(
            "This file's binding cannot add a {type_name} at that position."
        )),
    }
}

// ---- move ----

/// A reorder among siblings (FBL sections 5.5 and 6.4): the entry is removed where it is and inserted where it
/// goes, carrying its own bytes - its comments and unbound keys with it.
fn plan_move(plan: &mut Plan<'_>, id: &str, new_parent_id: Option<&str>, index: i64) -> Planning {
    let reading = plan.reading;
    let moved = element(reading, id)?;
    let element_ref = reading.element(moved);
    let rule = reading.family.rule(element_ref.rule);
    refuse_read_only(rule, "moved")?;
    if rule.insert.is_none() || rule.remove.is_none() {
        return refuse(format!("A {} cannot be moved in this file.", rule.type_name));
    }
    let parent = match new_parent_id {
        Some(p) => Some(element(reading, p)?),
        None => None,
    };
    if reading.family.places_by_position() {
        return plan_move_to(plan, moved, parent, index);
    }
    if parent != element_ref.parent {
        return plan_reparent(plan, moved, parent, index);
    }
    let siblings = siblings_under(reading, element_ref.parent, rule);
    let current = siblings.iter().position(|&s| s == moved).unwrap_or(0);
    let index = if index < 0 || index as usize > siblings.len() {
        siblings.len()
    } else {
        index as usize
    };
    if index == current || index == current + 1 {
        return Ok(()); // It is there already: no splice.
    }
    let before = (index < siblings.len()).then(|| siblings[index]);
    reading.family.move_entry(
        plan,
        moved,
        before,
        *siblings.last().expect("the element is among its siblings"),
    )?;
    if rule.snapshot_undo {
        plan.snapshot = true;
    }
    Ok(())
}

/// A move to another parent (FBL section 5.5): a `remove-entry` under the old parent and an `insert-entry` of the
/// entry's own bytes under the new one, at `index` among its children there, in one edit.
fn plan_reparent(plan: &mut Plan<'_>, moved: usize, parent: Option<usize>, index: i64) -> Planning {
    let reading = plan.reading;
    let rule = reading.family.rule(reading.element(moved).rule);
    match (parent, &rule.parent) {
        (Some(p), containment) => {
            let parent_rule = reading.family.rule(reading.element(p).rule);
            if !containment
                .as_ref()
                .is_some_and(|c| c.rules.contains(&parent_rule.name))
            {
                return refuse(format!(
                    "A {} cannot be placed inside a {}.",
                    rule.type_name, parent_rule.type_name
                ));
            }
            if within(reading, p, moved) {
                return refuse(format!("A {} cannot be moved into its own branch.", rule.type_name));
            }
        }
        (None, Some(_)) => {
            return refuse(format!(
                "A {} cannot be moved to the top level of this file.",
                rule.type_name
            ));
        }
        (None, None) => {}
    }
    let siblings = siblings_under(reading, parent, rule);
    let before = usize::try_from(index).ok().and_then(|i| siblings.get(i).copied());
    reading
        .family
        .move_to(plan, moved, parent, before, siblings.last().copied())?;
    if rule.snapshot_undo {
        plan.snapshot = true;
    }
    Ok(())
}

/// A move in a family that places by position (FBL 0.7 sections 5.5 and 6.3), to a sibling's place under the same
/// parent or under another: not into the element itself or anything it contains, and not into a parent its rule cannot
/// be nested in. The index counts the siblings before the move; the family is given it as they are without the element.
fn plan_move_to(plan: &mut Plan<'_>, moved: usize, parent: Option<usize>, index: i64) -> Planning {
    let reading = plan.reading;
    let rule = reading.family.rule(reading.element(moved).rule);
    if let Some(p) = parent {
        if within(reading, p, moved) {
            return refuse(format!("A {} cannot be moved into itself.", rule.type_name));
        }
        let parent_rule = reading.family.rule(reading.element(p).rule);
        if let Some(containment) = &rule.parent
            && !containment.rules.contains(&parent_rule.name)
        {
            return refuse(format!(
                "A {} cannot be placed inside a {}.",
                rule.type_name, parent_rule.type_name
            ));
        }
    }
    let slot = rule.parent.as_ref().and_then(|p| p.slot.as_deref());
    let siblings: Vec<usize> = reading
        .element_indices()
        .iter()
        .copied()
        .filter(|&e| {
            let other = reading.element(e);
            let other_rule = reading.family.rule(other.rule);
            other.parent == parent
                && other_rule.parent.as_ref().and_then(|p| p.slot.as_deref()) == slot
                && (parent.is_some() || other_rule.type_name == rule.type_name)
        })
        .collect();
    let mut index = usize::try_from(index)
        .ok()
        .filter(|&i| i <= siblings.len())
        .unwrap_or(siblings.len());
    if let Some(current) = siblings.iter().position(|&s| s == moved) {
        if index == current || index == current + 1 {
            return Ok(()); // It is there already: no splice.
        }
        if index > current {
            index -= 1;
        }
    }
    reading.family.move_to_position(plan, moved, parent, index)?;
    if rule.snapshot_undo {
        plan.snapshot = true;
    }
    Ok(())
}

/// Whether `element` is `ancestor` or lies under it.
fn within(reading: &BodyReading, element: usize, ancestor: usize) -> bool {
    let mut current = Some(element);
    while let Some(c) = current {
        if c == ancestor {
            return true;
        }
        current = reading.element(c).parent;
    }
    false
}

/// The elements an element of `rule` under `parent` is ordered among: the same parent and containment slot, and at
/// the top level, where there is neither, the same type.
fn siblings_under(reading: &BodyReading, parent: Option<usize>, rule: &Rule) -> Vec<usize> {
    let slot = rule.parent.as_ref().and_then(|p| p.slot.as_deref());
    reading
        .element_indices()
        .iter()
        .copied()
        .filter(|&o| {
            let other = reading.element(o);
            let other_rule = reading.family.rule(other.rule);
            other.parent == parent
                && other_rule.parent.as_ref().and_then(|p| p.slot.as_deref()) == slot
                && (parent.is_some() || other_rule.type_name == rule.type_name)
        })
        .collect()
}

// ---- remove ----

fn plan_remove(plan: &mut Plan<'_>, id: &str) -> Planning {
    let reading = plan.reading;
    let index = element(reading, id)?;
    let element = reading.element(index);
    let rule = reading.family.rule(element.rule);
    refuse_read_only(rule, "removed")?;
    let Some(settings) = &rule.remove else {
        return refuse(format!("A {} cannot be removed from this file.", rule.type_name));
    };
    let mut removed = vec![index];
    for &other in reading.element_indices() {
        if other == index
            || !settings
                .cascade
                .contains(&reading.family.rule(reading.element(other).rule).name)
        {
            continue;
        }
        if references(reading, other, index) && !removed.contains(&other) {
            removed.push(other);
        }
    }
    let span_of = |r: usize| reading.family.entries[reading.element(r).entry()].removal_span();
    let mut outermost: Vec<usize> = removed
        .iter()
        .copied()
        .filter(|&r| !removed.iter().any(|&o| o != r && contains(span_of(o), span_of(r))))
        .collect();
    // Stable: by own start, the last first.
    outermost.sort_by_key(|&r| std::cmp::Reverse(reading.family.entries[reading.element(r).entry()].own.start));
    for target in outermost {
        reading.family.remove(plan, target, &removed)?;
    }
    if removed
        .iter()
        .any(|&r| reading.family.rule(reading.element(r).rule).snapshot_undo)
    {
        plan.snapshot = true;
    }
    Ok(())
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end && outer != inner
}

/// Whether `other` has a reference that names `element` (FBL 6.2): one that resolves to it, as the reading
/// resolves references (FBL 5.4), and not merely one written like its key.
fn references(reading: &BodyReading, other: usize, element: usize) -> bool {
    let o = reading.element(other);
    if o.is_relation && (o.source_element == Some(element) || o.target_element == Some(element)) {
        return true;
    }
    let element_rule = &reading.family.rule(reading.element(element).rule).name;
    let other_rule = reading.family.rule(o.rule);
    for (a, (name, binding)) in other_rule.attributes.iter().enumerate() {
        let Some(reference) = &binding.reference else { continue };
        if !reference.to.contains(element_rule) || other == element {
            continue;
        }
        let Some(value) = o.attributes.get(name) else { continue };
        let names: Vec<String> = match value {
            Value::List(list) => list.iter().map(|v| new_text::plain(v, None)).collect(),
            _ => vec![o.text_of(other_rule, name)],
        };
        if names
            .iter()
            .any(|key| reading.referenced_by((o.rule, a), key) == Some(element))
        {
            return true;
        }
    }
    false
}
