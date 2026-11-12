//! Çakışma and Topoloji as a command's input (docs/adr/0162, 0622 §9): a new area written by `cad.polygon.create`,
//! the shape commands (`shapes.rs`) and `cad.entities.adjoin` loses what overlaps the shown areas of the layers it
//! avoids, and shares corners with the shown lines, paths and areas it touches. The command reads no session state:
//! the layers and the switches are its input (`AreaRules`); the neighbours are the host's geometry store's shown
//! objects (the drawing's own store when the host has none). The shared core does the work (`ops::adjoin`); the
//! tools pass the session's Çakışma and Topoloji and write through the commands.

use kentos_commands::{
    AreaPart, AreaRules, CommandWarning, EditOperation, EntitiesEdit, EntityEdit, RingGeometry,
    Vec2 as Wire,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::adjoin::{Neighbours, junctions};
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::ops::topology_edit::{Neighbour, SAME};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{Stop, error};
use crate::codes;
use crate::geometry::{edit_geometry, shape};

/// What the rules make of a new area: the areas to write as one object (the first its own fields, the others its
/// parts), the neighbours' corners and what to say.
#[derive(Clone, Debug, Default)]
pub(crate) struct Ruled {
    pub areas: Vec<Area>,
    /// The neighbours given corners of the new area, as `cad.entities.edit`'s `vertexAdd` writes them.
    pub edits: Vec<EntityEdit>,
    pub warnings: Vec<CommandWarning>,
}

/// The avoided layers: each a layer the drawing has, not a group.
pub(crate) fn check(doc: &Document, rules: Option<&AreaRules>) -> Result<(), Stop> {
    let Some(rules) = rules else {
        return Ok(());
    };
    for (i, id) in rules.avoid.iter().enumerate() {
        let path = format!("rules.avoid[{i}]");
        match doc.layers().get(id) {
            None => {
                return Err(Stop::failed(error(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "Kaçınılacak “{id}” katmanı çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    Some(path),
                )));
            }
            Some(node) if node.kind != kentos_commands::LayerNodeType::Layer => {
                return Err(Stop::failed(error(
                    codes::NOT_A_LAYER,
                    format!(
                        "Kaçınılacak “{}” bir grup; yalnız katmanların alanlarından kaçınılır.",
                        node.name
                    ),
                    Some(path),
                )));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// A ring's box, generous on arcs (an arc stays within twice its radius of its ends), `pad` round.
fn grow(b: &mut Bounds, r: &Ring, pad: f64) {
    let n = r.pts.len();
    for (i, p) in r.pts.iter().enumerate() {
        let q = r.pts[(i + 1) % n];
        let k = r
            .bulges
            .as_ref()
            .and_then(|b| b.get(i))
            .copied()
            .unwrap_or(0.0)
            .abs();
        let chord = (q.x - p.x).hypot(q.y - p.y);
        let arc = if k > 0.0 {
            chord * (1.0 + k * k) / (2.0 * k)
        } else {
            0.0
        };
        let d = pad + arc;
        b.min_x = b.min_x.min(p.x - d);
        b.min_y = b.min_y.min(p.y - d);
        b.max_x = b.max_x.max(p.x + d);
        b.max_y = b.max_y.max(p.y + d);
    }
}

fn areas_box(areas: &[Area], pad: f64) -> Bounds {
    let mut b = Bounds {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        max_y: f64::NEG_INFINITY,
    };
    for a in areas {
        grow(&mut b, &a.outer, pad);
    }
    b
}

fn slot(id: f64) -> Option<Slot> {
    (id >= 0.0 && id <= f64::from(u32::MAX) && id.fract() == 0.0).then_some(Slot(id as u32))
}

fn warn(code: &str, message: String, path: &str) -> CommandWarning {
    CommandWarning {
        code: code.into(),
        message,
        path: Some(path.into()),
    }
}

/// The rules on `areas` (one new area's parts): `None` when they change nothing. The neighbours are `store`'s shown
/// objects (the host's, or one made of the drawing).
pub(crate) fn apply(
    doc: &Document,
    store: Option<&Store>,
    areas: Vec<Area>,
    rules: Option<&AreaRules>,
) -> Result<Option<Ruled>, Stop> {
    let Some(rules) = rules.filter(|r| !r.is_empty()) else {
        return Ok(None);
    };
    let own;
    let store = match store {
        Some(store) => store,
        None => {
            own = crate::geometry_store::store_of(doc);
            &own
        }
    };
    let mut out = Ruled {
        areas,
        ..Ruled::default()
    };
    let mut changed = false;
    if !rules.avoid.is_empty() {
        let sets: Vec<Vec<Area>> = store
            .in_rect(&areas_box(&out.areas, 0.0), true)
            .into_iter()
            .filter_map(slot)
            .filter_map(|s| doc.get(s))
            .filter(|e| rules.avoid.iter().any(|l| *l == e.base().layer_id))
            .map(|e| areas_of_entity(&shape(e)))
            .filter(|set| !set.is_empty())
            .collect();
        if !sets.is_empty() {
            let neighbours = Neighbours::new(sets);
            let mut overlapped = std::collections::BTreeSet::new();
            let mut left = Vec::new();
            for area in &out.areas {
                let got = neighbours.avoid(area);
                overlapped.extend(got.overlapped);
                left.extend(got.areas);
            }
            if !overlapped.is_empty() {
                if left.is_empty() {
                    return Err(Stop::failed(error(
                        codes::AREA_COVERED,
                        "Yeni alan kaçınılan katmanlardaki alanların içinde kalıyor; yazılacak bir kısmı yok. Başka bir yere çizin ya da Çakışma'yı kapatın."
                            .into(),
                        Some("rules.avoid".into()),
                    )));
                }
                out.warnings.push(warn(
                    codes::AREA_CLIPPED,
                    format!(
                        "Çakışma önlendi: {} komşu alanla örtüşen kısım çıkarıldı.",
                        overlapped.len()
                    ),
                    "rules.avoid",
                ));
                out.areas = left;
                changed = true;
            }
        }
    }
    if rules.topology {
        let mut found = Vec::new();
        let mut list = Vec::new();
        for s in store
            .in_rect(&areas_box(&out.areas, SAME), true)
            .into_iter()
            .filter_map(slot)
        {
            let Some(e) = doc.get(s) else {
                continue;
            };
            let shape = shape(e);
            let shares = match shape {
                Shape::Line { .. } | Shape::Polyline { .. } | Shape::Polygon { .. } => true,
                Shape::Point { .. } => rules.points,
                _ => false,
            };
            if shares {
                found.push(s);
                list.push(Neighbour {
                    shape,
                    locked: doc.layers().is_locked(&e.base().layer_id),
                });
            }
        }
        if !list.is_empty() {
            let got = junctions(&out.areas, &list, rules.points);
            out.edits = got
                .edited
                .into_iter()
                .filter_map(|(index, shape)| {
                    let uid = doc.uid(*found.get(index)?)?;
                    Some(EntityEdit::Update {
                        uid: uid.to_string(),
                        geometry: edit_geometry(shape)?,
                    })
                })
                .collect();
            if !out.edits.is_empty() {
                out.warnings.push(warn(
                    codes::CORNERS_JOINED,
                    format!(
                        "Topolojik düzenleme: {} komşu nesneye yeni alanın {} köşesi eklendi.",
                        out.edits.len(),
                        got.given
                    ),
                    "rules.topology",
                ));
            }
            if got.taken > 0 {
                out.warnings.push(warn(
                    codes::CORNERS_JOINED,
                    format!(
                        "Topolojik düzenleme: yeni alana komşulardan {} köşe eklendi.",
                        got.taken
                    ),
                    "rules.topology",
                ));
            }
            if got.taken > 0 || !out.edits.is_empty() {
                out.areas = got.areas;
                changed = true;
            }
            if got.locked > 0 {
                out.warnings.push(warn(
                    codes::NEIGHBOURS_LOCKED,
                    format!(
                        "Kilitli katmandaki {} komşu nesneye köşe eklenmedi.",
                        got.locked
                    ),
                    "rules.topology",
                ));
                changed = true;
            }
        }
    }
    Ok(changed.then_some(out))
}

/// Writes the neighbours' corners into the open step through `cad.entities.edit`'s checks (`vertexAdd`: their
/// elevations carried along the edge).
pub(crate) fn write_neighbours(
    cx: &mut crate::ExecutionContext<'_>,
    edits: Vec<EntityEdit>,
) -> Result<(), Stop> {
    if edits.is_empty() {
        return Ok(());
    }
    let input = EntitiesEdit {
        operation: EditOperation::VertexAdd,
        changes: edits,
        expected_revision: None,
    };
    match crate::edit::write_in(cx, input) {
        kentos_commands::CommandResult::Completed { .. } => Ok(()),
        kentos_commands::CommandResult::Failed { error }
        | kentos_commands::CommandResult::Conflict { error }
        | kentos_commands::CommandResult::NeedsInput { error } => Err(Stop::failed(error)),
        // An edit neither queues nor is cancelled.
        _ => Ok(()),
    }
}

fn wire_ring(r: &Ring) -> RingGeometry {
    RingGeometry {
        pts: r.pts.iter().map(|p| Wire { x: p.x, y: p.y }).collect(),
        bulges: r.bulges.clone(),
        zs: None,
    }
}

/// The ruled areas as one area's fields: its outer ring, bulges, holes and parts (docs/adr/0143).
pub(crate) type Fields = (
    Vec<Wire>,
    Option<Vec<f64>>,
    Option<Vec<RingGeometry>>,
    Option<Vec<AreaPart>>,
);

pub(crate) fn fields(areas: &[Area]) -> Option<Fields> {
    let (first, rest) = areas.split_first()?;
    let holes = |a: &Area| (!a.holes.is_empty()).then(|| a.holes.iter().map(wire_ring).collect());
    let parts: Vec<AreaPart> = rest
        .iter()
        .map(|a| {
            let r = wire_ring(&a.outer);
            AreaPart {
                ms: None,
                pts: r.pts,
                bulges: r.bulges,
                holes: holes(a),
                zs: None,
            }
        })
        .collect();
    let r = wire_ring(&first.outer);
    Some((
        r.pts,
        r.bulges,
        holes(first),
        (!parts.is_empty()).then_some(parts),
    ))
}

/// The area a polygon's input gives: its outer ring and holes.
pub(crate) fn area_of(
    pts: &[Wire],
    bulges: Option<&[f64]>,
    holes: Option<&[RingGeometry]>,
) -> Area {
    let ring = |pts: &[Wire], bulges: Option<&[f64]>| Ring {
        pts: pts.iter().map(|p| Vec2::new(p.x, p.y)).collect(),
        bulges: bulges.map(<[f64]>::to_vec),
    };
    Area {
        outer: ring(pts, bulges),
        holes: holes
            .unwrap_or_default()
            .iter()
            .map(|h| ring(&h.pts, h.bulges.as_deref()))
            .collect(),
    }
}
