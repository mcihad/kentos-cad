//! A project's address data read from the native document (docs/adr/0229
//! §6): the objects of a locator's layers in the drawing's order, those
//! their layers' filters leave out left out (docs/adr/0211 §1), each with
//! the texts of the attributes the locator names, given to the shared core's
//! locator (`ops::geocode::locator`). Doors are points (a multi-point's
//! first), streets lines, paths and arcs, areas closed areas and circles;
//! any other object is counted. The desktop's Adres tab, the processing
//! tools and Python build from it.

use std::collections::BTreeSet;

use kentos_commands::{AddressLocator, AreaLevel, Entity};
use kentos_domain::Document;
use kentos_geometry_core::ops::geocode::{
    AreaInput, DoorInput, Input, Level, Locator, StreetInput,
};
use kentos_geometry_core::vec2::Vec2;

/// A field's text on an object: its attribute, empty when the field is not named or not set.
fn value(e: &Entity, field: &Option<String>) -> String {
    field
        .as_deref()
        .and_then(|f| e.base().attrs.get(f))
        .cloned()
        .unwrap_or_default()
}

fn named(e: &Entity, field: &str) -> String {
    e.base().attrs.get(field).cloned().unwrap_or_default()
}

/// The project's unit per metre (docs/adr/0165 §2).
pub fn per_metre(doc: &Document) -> f64 {
    doc.settings().unit().per_metre()
}

/// The objects of a layer its filter lets through, with their places in the drawing's order.
fn objects<'d>(
    doc: &'d Document,
    layer: &str,
    out: &std::collections::HashSet<kentos_domain::Slot>,
) -> Vec<(u32, &'d Entity)> {
    doc.by_layer(layer)
        .filter(|e| !out.contains(&kentos_domain::Slot(e.base().id)))
        .map(|e| {
            let slot = kentos_domain::Slot(e.base().id);
            let place = doc.place(slot).unwrap_or(0).min(u64::from(u32::MAX)) as u32;
            (place, e)
        })
        .collect()
}

/// What the core is given for a locator.
pub fn input(doc: &Document, l: &AddressLocator) -> Input {
    let left_out = crate::layer_filter::left_out(doc);
    let mut input = Input {
        per_metre: per_metre(doc),
        ..Input::default()
    };
    if let Some(d) = &l.doors {
        for (order, e) in objects(doc, &d.layer, &left_out) {
            let Entity::Point(p) = e else {
                input.not_doors += 1;
                continue;
            };
            input.doors.push(DoorInput {
                order,
                at: Vec2::new(p.p.x, p.p.y),
                street: named(e, &d.street),
                door: named(e, &d.door),
                neighbourhood: value(e, &d.neighbourhood),
                district: value(e, &d.district),
                province: value(e, &d.province),
                postcode: value(e, &d.postcode),
                building: value(e, &d.building),
                code: value(e, &d.code),
            });
        }
    }
    if let Some(s) = &l.streets {
        for (order, e) in objects(doc, &s.layer, &left_out) {
            if !matches!(e, Entity::Line(_) | Entity::Polyline(_) | Entity::Arc(_)) {
                input.not_streets += 1;
                continue;
            }
            input.streets.push(StreetInput {
                order,
                shape: crate::geometry::shape(e),
                name: named(e, &s.name),
                neighbourhood: value(e, &s.neighbourhood),
                district: value(e, &s.district),
            });
        }
    }
    for a in &l.areas {
        let level = match a.level {
            AreaLevel::Neighbourhood => Level::Neighbourhood,
            AreaLevel::District => Level::District,
            AreaLevel::Province => Level::Province,
        };
        for (order, e) in objects(doc, &a.layer, &left_out) {
            if !matches!(e, Entity::Polygon(_) | Entity::Circle(_)) {
                input.not_areas += 1;
                continue;
            }
            input.areas.push(AreaInput {
                order,
                level,
                shape: crate::geometry::shape(e),
                name: named(e, &a.name),
                district: value(e, &a.district),
            });
        }
    }
    input
}

/// The locator of a project's address data.
pub fn build(doc: &Document, l: &AddressLocator) -> Locator {
    Locator::build(&input(doc, l))
}

/// A project's address data by its id.
pub fn locator<'d>(doc: &'d Document, id: &str) -> Option<&'d AddressLocator> {
    doc.settings().address_locators.iter().find(|l| l.id == id)
}

/// A layer's attribute names, as Adres verisi lists them: its fields in
/// their order, then the attribute names its objects have, in code point
/// order, each once.
pub fn field_names(doc: &Document, layer: &str) -> Vec<String> {
    let mut out: Vec<String> = doc
        .layers()
        .get(layer)
        .map(|l| l.fields.iter().map(|f| f.name.clone()).collect())
        .unwrap_or_default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for e in doc.by_layer(layer) {
        for k in e.base().attrs.keys() {
            seen.insert(k.clone());
        }
    }
    for k in seen {
        if !out.contains(&k) {
            out.push(k);
        }
    }
    out
}
