//! Parçaları birleştir and Parçalara ayır for lines and points (docs/adr/0174
//! §4), beside the areas' (`area`): lines and polylines become one
//! multi-part polyline in the first one's place (a line's ends' elevations
//! its part's), points one multi-point object; a multi-part polyline or a
//! multi-point object comes apart into its parts, the first keeping its
//! place and persistent id, the others new with its data. Touching lines are
//! not chained: that is Birleştir's. The web's are in `areaTools.ts`.

use kentos_contracts::{AreaPart, EditOperation, Entity, EntityEdit, EntityGeometry, PointPart};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::entity_length;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_native_application::geometry::shape;

use crate::edge;
use crate::log::Level;
use crate::tool::Context;

/// What Parçaları birleştir says of a selection of more than one kind.
pub const MIXED: &str =
    "Parçaları birleştir aynı türden nesneleri birleştirir: alanları, çizgileri ya da noktaları.";
/// What it says of a single line or polyline.
pub const ONE_LINE: &str = "Parçaları birleştirmek için en az iki çizgi ya da çoklu çizgi seçin.";
/// What it says of a single point.
pub const ONE_POINT: &str = "Parçaları birleştirmek için en az iki nokta seçin.";

/// The selection by what Parçaları birleştir joins: how many objects
/// enclose an area, the lines (a line, an open polyline, a multi-part
/// polyline) and the points, in the selection's order.
#[derive(Default)]
pub struct Kinds {
    pub areas: usize,
    pub lines: Vec<Slot>,
    pub points: Vec<Slot>,
}

impl Kinds {
    /// How many of the three kinds the selection holds.
    pub fn present(&self) -> usize {
        usize::from(self.areas > 0)
            + usize::from(!self.lines.is_empty())
            + usize::from(!self.points.is_empty())
    }
}

/// What the objects at `slots` are to Parçaları birleştir; other kinds
/// (text, arcs …) are none of them.
pub fn kinds(slots: &[Slot], doc: &Document) -> Kinds {
    let mut k = Kinds::default();
    for &slot in slots {
        let Some(e) = doc.get(slot) else {
            continue;
        };
        match e {
            Entity::Point(_) => k.points.push(slot),
            Entity::Line(_) => k.lines.push(slot),
            Entity::Polyline(p) if p.parts.is_some() || areas_of_entity(&shape(e)).is_empty() => {
                k.lines.push(slot)
            }
            _ if !areas_of_entity(&shape(e)).is_empty() => k.areas += 1,
            _ => {}
        }
    }
    k
}

/// The lines at `slots` as one multi-part polyline in the first one's
/// place: their parts in the selection's order, each object's own first; a
/// line becomes a polyline (`replace`, its data kept); the others go.
pub fn join_lines(slots: &[Slot], cx: &mut Context<'_>) {
    let doc = &*cx.doc;
    let mut parts: Vec<AreaPart> = Vec::new();
    for &slot in slots {
        match doc.get(slot) {
            Some(Entity::Line(l)) => parts.push(AreaPart {
                pts: vec![l.a, l.b],
                bulges: None,
                holes: None,
                zs: (l.za.is_some() || l.zb.is_some()).then(|| vec![l.za, l.zb]),
            }),
            Some(Entity::Polyline(p)) => {
                parts.push(AreaPart {
                    pts: p.pts.clone(),
                    bulges: p.bulges.clone(),
                    holes: None,
                    zs: p.zs.clone(),
                });
                parts.extend(p.parts.iter().flatten().map(|q| AreaPart {
                    holes: None,
                    ..q.clone()
                }));
            }
            _ => {}
        }
    }
    let pieces = parts.len();
    let mut parts = parts.into_iter();
    let Some(first) = parts.next() else {
        return;
    };
    let geometry = EntityGeometry::Polyline {
        pts: first.pts,
        bulges: first.bulges,
        zs: first.zs,
        parts: Some(parts.collect()),
    };
    let uid = edge::uid(doc, slots[0]);
    let mut changes = vec![match doc.get(slots[0]) {
        Some(Entity::Polyline(_)) => EntityEdit::Update { uid, geometry },
        _ => EntityEdit::Replace {
            uid,
            geometry,
            keep_data: Some(true),
        },
    }];
    changes.extend(slots[1..].iter().map(|&s| EntityEdit::Remove {
        uid: edge::uid(doc, s),
    }));
    if edge::write(EditOperation::PartsJoin, changes, cx).is_none() {
        return;
    }
    cx.selection.set(vec![slots[0]]);
    let length = cx
        .doc
        .get(slots[0])
        .and_then(|e| entity_length(&shape(e)))
        .unwrap_or(0.0);
    let total = cx.format().length(length);
    cx.say(
        Level::Success,
        format!(
            "{} çizgi tek çoklu çizgide birleşti: {pieces} parça, toplam {total}.",
            slots.len()
        ),
    );
}

/// The points at `slots` as one multi-point object in the first one's
/// place, each with its elevation, in the selection's order; the others go.
pub fn join_points(slots: &[Slot], cx: &mut Context<'_>) {
    let doc = &*cx.doc;
    let mut points: Vec<PointPart> = Vec::new();
    for &slot in slots {
        if let Some(Entity::Point(p)) = doc.get(slot) {
            points.push(PointPart { p: p.p, z: p.z });
            points.extend(p.parts.iter().flatten().cloned());
        }
    }
    let n = points.len();
    let mut points = points.into_iter();
    let Some(first) = points.next() else {
        return;
    };
    let geometry = EntityGeometry::Point {
        p: first.p,
        z: first.z,
        parts: Some(points.collect()),
    };
    let mut changes = vec![EntityEdit::Update {
        uid: edge::uid(doc, slots[0]),
        geometry,
    }];
    changes.extend(slots[1..].iter().map(|&s| EntityEdit::Remove {
        uid: edge::uid(doc, s),
    }));
    if edge::write(EditOperation::PartsJoin, changes, cx).is_none() {
        return;
    }
    cx.selection.set(vec![slots[0]]);
    cx.say(
        Level::Success,
        format!("{} nokta tek nesnede birleşti: {n} nokta.", slots.len()),
    );
}

/// A multi-part polyline's or multi-point object's changes for Parçalara
/// ayır: its own first part in its place, each other one new with its data
/// (`add`, keepData); with how many are new. None for anything else.
pub fn split(e: &Entity, uid: &str) -> Option<(Vec<EntityEdit>, usize)> {
    let add = |geometry| EntityEdit::add(uid.to_owned(), geometry, Some(true));
    match e {
        Entity::Polyline(p) if p.parts.as_ref().is_some_and(|q| !q.is_empty()) => {
            let parts = p.parts.as_deref().unwrap_or(&[]);
            let mut changes = vec![EntityEdit::Update {
                uid: uid.to_owned(),
                geometry: EntityGeometry::Polyline {
                    pts: p.pts.clone(),
                    bulges: p.bulges.clone(),
                    zs: p.zs.clone(),
                    parts: None,
                },
            }];
            changes.extend(parts.iter().map(|q| {
                add(EntityGeometry::Polyline {
                    pts: q.pts.clone(),
                    bulges: q.bulges.clone(),
                    zs: q.zs.clone(),
                    parts: None,
                })
            }));
            Some((changes, parts.len()))
        }
        Entity::Point(p) if p.parts.as_ref().is_some_and(|q| !q.is_empty()) => {
            let parts = p.parts.as_deref().unwrap_or(&[]);
            let mut changes = vec![EntityEdit::Update {
                uid: uid.to_owned(),
                geometry: EntityGeometry::Point {
                    p: p.p,
                    z: p.z,
                    parts: None,
                },
            }];
            changes.extend(parts.iter().map(|q| {
                add(EntityGeometry::Point {
                    p: q.p,
                    z: q.z,
                    parts: None,
                })
            }));
            Some((changes, parts.len()))
        }
        _ => None,
    }
}
