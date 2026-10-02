//! The overlap control (docs/adr/0162 §1–§2): while it is on, a new area
//! drawn by its outline loses what overlaps the visible areas of the overlap
//! layers (its own layer, or the chosen ones) before it is written. What is
//! left is the core's (`ops::adjoin::Neighbours::avoid`); the tools write it
//! as one object, its holes and parts as they are. The web's is
//! `apps/web/src/tools/overlap.ts`; both play
//! `fixtures/interaction/v1/overlap.json`.

use kentos_contracts::{AreaPart, EntityGeometry, RingGeometry};
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region::net_area;
use kentos_geometry_core::ops::adjoin::Neighbours;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::log::Level;
use crate::points::wire;
use crate::tool::{Context, Overlap};

/// The layers whose areas a new area going on `layer` must not overlap;
/// none while the mode is Serbest.
pub(crate) fn overlap_layers(cx: &Context<'_>, layer: &str) -> Vec<String> {
    match cx.draft.overlap {
        Overlap::Allow => Vec::new(),
        Overlap::Layer => vec![layer.to_owned()],
        Overlap::Layers => cx.overlap_layers.to_vec(),
    }
}

/// A ring's box, generous on arcs: an arc stays within twice its radius of its ends.
fn ring_box(r: &Ring) -> (Vec2, Vec2) {
    let n = r.pts.len();
    let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
    let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
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
        // Twice the radius of the arc of bulge k on this chord (none for a straight edge).
        let pad = if k > 0.0 {
            chord * (1.0 + k * k) / (2.0 * k)
        } else {
            0.0
        };
        min = Vec2::new(min.x.min(p.x - pad), min.y.min(p.y - pad));
        max = Vec2::new(max.x.max(p.x + pad), max.y.max(p.y + pad));
    }
    (min, max)
}

/// What the overlap control leaves of a new area: what to write (none
/// when it was covered) and how many it overlapped.
pub(crate) struct Clipped {
    pub areas: Vec<Area>,
    pub overlapped: usize,
}

/// The overlap control on a new area going on `layer`: `None` to write it
/// as drawn (the mode is Serbest, or it overlaps nothing); else what is
/// left. The neighbours are the visible objects of the overlap layers whose
/// box meets the area's.
pub(crate) fn clip_new_area(cx: &mut Context<'_>, area: &Area, layer: &str) -> Option<Clipped> {
    clip_new_areas(cx, std::slice::from_ref(area), layer)
}

/// The overlap control on the parts of one new area (Bitişik alan's, §3):
/// as [`clip_new_area`], the neighbours those whose box meets the parts' box,
/// each counted once.
pub(crate) fn clip_new_areas(cx: &mut Context<'_>, areas: &[Area], layer: &str) -> Option<Clipped> {
    let layers = overlap_layers(cx, layer);
    if layers.is_empty() {
        if cx.draft.overlap == Overlap::Layers {
            cx.say(
                Level::Warn,
                "Seçili katmanlarda önle kipinde seçili katman yok: alan olduğu gibi yazıldı. Katmanları Çakışma hücresinin menüsünden seçin.",
            );
        }
        return None;
    }
    let (min, max) = areas.iter().map(|a| ring_box(&a.outer)).fold(
        (
            Vec2::new(f64::INFINITY, f64::INFINITY),
            Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(min, max), (a, b)| {
            (
                Vec2::new(min.x.min(a.x), min.y.min(a.y)),
                Vec2::new(max.x.max(b.x), max.y.max(b.y)),
            )
        },
    );
    let sets: Vec<Vec<Area>> = cx
        .spatial
        .in_rect(min, max, true)
        .into_iter()
        .filter_map(|s| cx.doc.get(s))
        .filter(|e| layers.iter().any(|l| *l == e.base().layer_id))
        .map(|e| areas_of_entity(&shape(e)))
        .filter(|set| !set.is_empty())
        .collect();
    if sets.is_empty() {
        return None;
    }
    let neighbours = Neighbours::new(sets);
    let mut overlapped = std::collections::BTreeSet::new();
    let mut left = Vec::new();
    for area in areas {
        let got = neighbours.avoid(area);
        overlapped.extend(got.overlapped);
        left.extend(got.areas);
    }
    (!overlapped.is_empty()).then_some(Clipped {
        areas: left,
        overlapped: overlapped.len(),
    })
}

fn ring(r: &Ring) -> RingGeometry {
    RingGeometry {
        pts: r.pts.iter().map(|p| wire(*p)).collect(),
        bulges: r.bulges.clone(),
        zs: None,
    }
}

/// The areas left as one area's geometry: the first its own fields, the
/// others its parts (docs/adr/0143).
pub(crate) fn clipped_geometry(areas: &[Area]) -> Option<EntityGeometry> {
    let (first, rest) = areas.split_first()?;
    let holes = |a: &Area| (!a.holes.is_empty()).then(|| a.holes.iter().map(ring).collect());
    let parts: Vec<AreaPart> = rest
        .iter()
        .map(|a| {
            let r = ring(&a.outer);
            AreaPart {
                pts: r.pts,
                bulges: r.bulges,
                holes: holes(a),
                zs: None,
            }
        })
        .collect();
    let r = ring(&first.outer);
    Some(EntityGeometry::Polygon {
        pts: r.pts,
        bulges: r.bulges,
        holes: holes(first),
        zs: None,
        parts: (!parts.is_empty()).then_some(parts),
    })
}

/// The overlap control's report: what it took, or that nothing was left to write.
pub(crate) fn say_clipped(cx: &mut Context<'_>, clipped: &Clipped) {
    if clipped.areas.is_empty() {
        cx.say(
            Level::Warn,
            "Yeni alan komşu alanların içinde kalıyor; alan eklenmedi.",
        );
    } else {
        let text = format!(
            "Çakışma önlendi: {} komşu alanla örtüşen kısım çıkarıldı.",
            clipped.overlapped
        );
        cx.say(Level::Info, text);
    }
}

/// The area written: what is left, every part, less its holes.
pub(crate) fn written_area(areas: &[Area]) -> f64 {
    areas.iter().map(net_area).sum()
}
