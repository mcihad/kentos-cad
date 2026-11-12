//! The place of an event object on its route (docs/adr/0224 §5, 0526, 0569): how an event's attributes are read, how
//! far the object lies from the place its route gives it and where it moves to. Olay kaymasını denetle (İşlemler)
//! and the events a project's link keeps on their routes ([`crate::event_follow`]) place events through here, so there
//! is one rule: the core's (`ops::lrs::events_run`, `point_drift`, `line_drift`); this module only reads cells and
//! carries the result back into the object.

use std::collections::{BTreeMap, HashMap};

use kentos_commands::{
    AreaPart, Entity, EntityBase, LineEntity, PathEntity, PointEntity, Vec2 as Point, js_trim,
};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::ops::lrs::{
    EventIn, EventOut, Fault, Route, RoutePart, line_drift, point_drift,
};

use crate::geometry::route as route_of;

fn p(q: Vec2) -> Point {
    Point { x: q.x, y: q.y }
}

/// The routes of objects by their id: the id field's value, trimmed (an empty one no route); of several objects of
/// an id the first.
pub struct Routes<'a> {
    pub ids: Vec<String>,
    pub routes: Vec<Route>,
    pub objects: Vec<&'a Entity>,
    pub index: HashMap<String, usize>,
    /// Objects of an id that came before.
    pub again: usize,
}

pub fn routes_by_id<'a>(objects: &[&'a Entity], field: &str) -> Routes<'a> {
    let mut out = Routes {
        ids: Vec::new(),
        routes: Vec::new(),
        objects: Vec::new(),
        index: HashMap::new(),
        again: 0,
    };
    for e in objects {
        let Some(route) = route_of(e) else {
            continue;
        };
        let id = js_trim(e.base().attrs.get(field).map_or("", String::as_str));
        if id.is_empty() {
            continue;
        }
        if out.index.contains_key(id) {
            out.again += 1;
            continue;
        }
        out.index.insert(id.to_owned(), out.ids.len());
        out.ids.push(id.to_owned());
        out.routes.push(route);
        out.objects.push(e);
    }
    out
}

/// The warning of ids that came again.
pub fn again_text(again: usize) -> String {
    format!("{again} rota kimliği birden çok kez var; ilki kullanıldı.")
}

/// A cell's number (docs/adr/0180): a sign, digits and one decimal point or comma.
pub fn read_measure(text: &str) -> Option<f64> {
    kentos_geometry_core::ops::cogo::read_number(text)
}

/// Why a cell gives no number.
pub fn cell_reason(field: &str, cell: &str) -> String {
    let t = js_trim(cell);
    if t.is_empty() {
        format!("“{field}” boş")
    } else {
        format!("“{field}” sayı değil: {t}")
    }
}

/// A route's parts as a polyline on `layer_id` with `attrs` (a part's bulges only when one bends).
pub fn polyline_of(parts: &[RoutePart], layer_id: &str, attrs: BTreeMap<String, String>) -> Entity {
    let bends = |q: &RoutePart| {
        q.bulges
            .as_ref()
            .filter(|b| b.iter().any(|&x| x != 0.0))
            .cloned()
    };
    let (first, rest) = parts
        .split_first()
        .map_or((None, &[][..]), |(f, r)| (Some(f), r));
    let empty = RoutePart {
        pts: Vec::new(),
        bulges: None,
        ms: Vec::new(),
    };
    let first = first.unwrap_or(&empty);
    Entity::Polyline(PathEntity {
        space_separator: None,
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer_id.to_owned(),
            color: None,
            attrs,
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        pts: first.pts.iter().copied().map(p).collect(),
        bulges: bends(first),
        holes: None,
        zs: None,
        ms: Some(first.ms.clone()),
        parts: (!rest.is_empty()).then(|| {
            rest.iter()
                .map(|q| AreaPart {
                    pts: q.pts.iter().copied().map(p).collect(),
                    bulges: bends(q),
                    holes: None,
                    zs: None,
                    ms: Some(q.ms.clone()),
                })
                .collect()
        }),
        closed: false,
        widths: None,
    })
}

/// Which attributes of an event object say where it is: its route, its measure (a point) or its from and to (a
/// line), its offset and which side a positive offset lies on.
pub struct Rules<'a> {
    pub route: &'a str,
    /// A point's measure field, or a line's from and to.
    pub measures: Vec<&'a str>,
    pub offset: Option<&'a str>,
    /// +1: a positive offset lies right of the measure's direction; −1: left.
    pub turn: f64,
}

impl Rules<'_> {
    /// Whether the objects are lines (two measures) rather than points.
    pub fn line(&self) -> bool {
        self.measures.len() == 2
    }
}

/// What an event's own attributes say.
pub struct Reading {
    /// The route's id, trimmed (it may name no route).
    pub id: String,
    pub nums: Vec<f64>,
    /// The offset as the core takes it (the side turned in).
    pub offset: f64,
}

/// An event's attributes as a reading, or why they give none (the cell, in the table's words).
pub fn read(e: &Entity, rules: &Rules<'_>) -> Result<Reading, String> {
    let attrs = &e.base().attrs;
    let cell = |name: &str| attrs.get(name).map_or("", String::as_str);
    let id = js_trim(cell(rules.route)).to_owned();
    let mut nums = Vec::new();
    for f in &rules.measures {
        match read_measure(cell(f)) {
            Some(n) => nums.push(n),
            None => return Err(cell_reason(f, cell(f))),
        }
    }
    let mut offset = 0.0;
    if let Some(field) = rules.offset
        && !js_trim(field).is_empty()
        && !js_trim(cell(field)).is_empty()
    {
        match read_measure(cell(field)) {
            Some(n) => offset = n,
            None => return Err(cell_reason(field, cell(field))),
        }
    }
    Ok(Reading {
        id,
        nums,
        offset: offset * rules.turn,
    })
}

/// A reading on its route (by its place in the list the core takes; none: no such route) as the core's input.
pub fn event_in(reading: &Reading, route: Option<usize>, line: bool) -> EventIn {
    EventIn {
        route,
        m: (!line).then(|| reading.nums[0]),
        from: line.then(|| reading.nums[0]),
        to: line.then(|| reading.nums[1]),
        offset: Some(reading.offset),
    }
}

/// An object's shape as a route's parts (a point none).
fn parts_of(e: &Entity) -> Option<Vec<RoutePart>> {
    route_of(e).map(|r| r.parts)
}

/// How far the object lies from the place the core gave it; none when there is nothing to compare (a point with no
/// point, a line with no parts).
pub fn drift_of(e: &Entity, out: &EventOut) -> Option<f64> {
    match (e, out.p, &out.parts) {
        (Entity::Point(q), Some(at), _) => Some(point_drift(Vec2 { x: q.p.x, y: q.p.y }, at)),
        (_, _, Some(parts)) => {
            Some(parts_of(e).map_or(f64::INFINITY, |now| line_drift(&now, parts)))
        }
        _ => None,
    }
}

/// The object moved to its place, of its own kind (its other fields stay its own); none when a line cannot take a
/// bent path.
pub fn moved(e: &Entity, out: &EventOut) -> Option<Entity> {
    let shape = match (e, out.p, out.parts.as_deref()) {
        (Entity::Point(q), Some(at), _) => Entity::Point(PointEntity {
            p: p(at),
            ..q.clone()
        }),
        (Entity::Line(l), _, Some([one])) => {
            let straight = one
                .bulges
                .as_ref()
                .is_none_or(|b| b.iter().all(|&x| x == 0.0));
            if one.pts.len() != 2 || !straight {
                return None;
            }
            Entity::Line(LineEntity {
                a: p(one.pts[0]),
                b: p(one.pts[1]),
                za: None,
                zb: None,
                ma: one.ms.first().copied().flatten(),
                mb: one.ms.get(1).copied().flatten(),
                ..l.clone()
            })
        }
        (Entity::Polyline(_), _, Some(parts)) => {
            polyline_of(parts, &e.base().layer_id, BTreeMap::new())
        }
        _ => return None,
    };
    // Its shape, of its own kind: the fields every object has stay its own.
    let mut shaped = shape;
    *shaped.base_mut() = e.base().clone();
    Some(shaped)
}

/// Why the core could not place an event, without lengths (the follow's warning; the tool's table says them in the
/// project's unit).
pub fn fault_words(f: &Fault) -> &'static str {
    match f {
        Fault::Outside { .. } => "ölçü rotanın dışında",
        Fault::NoMeasures => "rotanın ölçüsü yok",
        Fault::Same => "başlangıç ve bitiş ölçüsü aynı",
        Fault::Offset => "öteleme yapılamadı",
        Fault::NoRoute => "rota yok",
        Fault::NoMeasure => "ölçü yok",
        Fault::FewPoints => "ikiden az nokta",
    }
}
