//! Reading a drawing without writing it: its objects by page (the web's
//! and the server's wire form, each with its persistent id) and an object's
//! measures from the shared geometry core (TODOS.md AI-05, AI-06).

use kentos_contracts::{Bounds, Entity};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{entity_area, entity_bounds, entity_length};
use kentos_native_application::geometry::shape;
use serde::Serialize;

use crate::HeadlessError;

/// The most objects one page gives; a program reads on with the cursor.
pub const PAGE_MAX: usize = 10_000;

/// Which objects a page reads.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filter {
    /// Only on this layer (a layer's id).
    pub layer: Option<String>,
    /// Only of these kinds (`line`, `polygon`, `text`…).
    pub kinds: Option<Vec<String>>,
    /// Only those whose box meets this one.
    pub bbox: Option<Bounds>,
    /// After this object (its persistent id), in the drawing's order.
    pub after: Option<String>,
    /// At most this many, at most [`PAGE_MAX`].
    pub limit: Option<usize>,
}

/// One object as a page gives it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Item<'a> {
    /// Its persistent id (lowercase UUID text with hyphens).
    pub uid: String,
    pub entity: &'a Entity,
}

/// A page of objects in the drawing's order; `next` continues it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Page<'a> {
    pub items: Vec<Item<'a>>,
    /// The cursor of the next page (the last object's id); none at the end.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

/// An object's measures in the project's units, from its source geometry
/// (not a drawn approximation; CLAUDE.md §23.3).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measure {
    pub uid: String,
    pub kind: String,
    /// Plane area of a closed shape (m²); none for an open one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<f64>,
    /// Length of a line or path, perimeter of a closed shape (m); none for a point or text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    pub bounds: Bounds,
}

pub fn slot_of(doc: &Document, uid: &str) -> Result<Slot, HeadlessError> {
    let id = Uuid::parse_str(uid).map_err(|_| {
        HeadlessError::new(
            "invalid_input",
            format!("“{uid}” bir nesne kimliği değil: küçük harfli, tireli UUID olmalı."),
        )
    })?;
    doc.slot_of(id).ok_or_else(|| {
        HeadlessError::new(
            "unknown_object",
            format!("{uid} kimlikli nesne bu çizimde yok (silinmiş ya da başka bir çizimin)."),
        )
    })
}

fn uid_text(doc: &Document, slot: Slot) -> String {
    doc.uid(slot).map(|u| u.to_string()).unwrap_or_default()
}

fn meets(a: &Bounds, b: &Bounds) -> bool {
    a.min_x <= b.max_x && b.min_x <= a.max_x && a.min_y <= b.max_y && b.min_y <= a.max_y
}

pub fn page<'a>(doc: &'a Document, filter: &Filter) -> Result<Page<'a>, HeadlessError> {
    let limit = filter.limit.unwrap_or(1000).clamp(1, PAGE_MAX);
    let start = match &filter.after {
        Some(uid) => Some(slot_of(doc, uid)?),
        None => None,
    };
    let objects: Box<dyn Iterator<Item = &Entity>> = match &filter.layer {
        Some(layer) => Box::new(doc.by_layer(layer)),
        None => Box::new(doc.entities()),
    };
    let mut past = start.is_none();
    let mut items = Vec::new();
    let mut more = false;
    for e in objects {
        let slot = Slot(e.base().id);
        if !past {
            past = Some(slot) == start;
            continue;
        }
        if let Some(kinds) = &filter.kinds
            && !kinds.iter().any(|k| k == e.kind())
        {
            continue;
        }
        if let Some(bbox) = &filter.bbox {
            let b = entity_bounds(&shape(e));
            let b = Bounds {
                min_x: b.min_x,
                min_y: b.min_y,
                max_x: b.max_x,
                max_y: b.max_y,
            };
            if !meets(&b, bbox) {
                continue;
            }
        }
        if items.len() == limit {
            more = true;
            break;
        }
        items.push(Item {
            uid: uid_text(doc, slot),
            entity: e,
        });
    }
    let next = if more {
        items.last().map(|i| i.uid.clone())
    } else {
        None
    };
    Ok(Page { items, next })
}

pub fn measure(doc: &Document, uid: &str) -> Result<Measure, HeadlessError> {
    let slot = slot_of(doc, uid)?;
    let Some(e) = doc.get(slot) else {
        return Err(HeadlessError::new(
            "unknown_object",
            format!("{uid} kimlikli nesne bu çizimde yok."),
        ));
    };
    let s = shape(e);
    let b = entity_bounds(&s);
    Ok(Measure {
        uid: uid.to_owned(),
        kind: e.kind().to_owned(),
        area: entity_area(&s),
        length: entity_length(&s),
        bounds: Bounds {
            min_x: b.min_x,
            min_y: b.min_y,
            max_x: b.max_x,
            max_y: b.max_y,
        },
    })
}
