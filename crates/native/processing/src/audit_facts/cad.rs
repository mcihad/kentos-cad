//! The drawing's CAD facts for the audit facts (`cad`, docs/adr/8064): every layer's name, objects, hatches and line
//! type; the blocks inserted and their layers; the dimensions' typefaces; the grids; every room's number, name, storey
//! code and centre. Their order, rounding and bounds are `kentos_extensions::audit_facts::cad`'s; the EK-3 and EK-4
//! rules that read them are the standards package's Python (catalog/standards/dijital-proje, bim-plan K10).
//!
//! - **A storey's code** is the sheets' (`kentos_sheet::permit::level_codes`, EK-2 Tablo 2.14), so a room's number is
//!   held to the code its storey's sheets carry.
//! - **A dimension's typeface** is its own, else its dimension style's, else the project's drawing typeface, else
//!   Barlow (what draws it).
//! - **A room's centre** is its drawn outline's (the area-weighted centroid of its rings, holes out).
//! - **What an object is to a sheet** (docs/adr/8065): the model's (walls, openings, rooms, slabs, columns, stairs,
//!   blocks placed …, and what a model object draws as its own: a kitchen run's countertop and lines), an annotation tied
//!   to the drawing (dimensions, tags, texts, leaders, tables, marks), or free drawing (lines, polylines, arcs, circles,
//!   curves, points, hatches); a section's state is the section query's.
//! - **Rooms that overlap** (docs/adr/8067, EK-9 Tablo 9.1/13): two rooms of one storey (or both on none) whose drawn
//!   areas share ground, with how much (the geometry core's overlay of the two areas) and where. The rule decides how
//!   much is too much; rooms on other storeys are not compared (a room through two storeys is one storey's).
//! - **Exits** (docs/adr/8124): every door and stair, its declared emergency exit role and why the escape routes
//!   (`kentos_domain::escape`, docs/adr/8035) take it as a storey exit, if they do; the rule suggests and compares.

use std::collections::BTreeMap;

use kentos_commands::{DrawingFont, Entity};
use kentos_domain::Document;
use kentos_domain::arch_values::core_areas;
use kentos_extensions::audit_facts::cad::{
    CadBlock, CadBroken, CadDerived, CadExit, CadFacts, CadGrid, CadLayer, CadModelGap, CadRoom, CadRoomOverlap,
    FontUse,
};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::{intersect_area_sets, net_area};

/// The drawing's CAD facts.
pub fn measure(doc: &Document) -> CadFacts {
    let leaves = doc.layers().leaves();
    // Per layer: objects, hatches, their persistent ids, and the objects by class.
    let mut objects: BTreeMap<&str, Counts> = BTreeMap::new();
    // A kitchen run's countertop and lines are its own, as its cabinets are (docs/adr/0727 §5): the model's.
    let generated: std::collections::BTreeSet<String> = doc
        .casework()
        .iter()
        .flat_map(|r| r.members.iter().map(|m| m.uid.to_string()))
        .collect();
    let mut broken = Vec::new();
    let mut blocks: BTreeMap<String, CadBlock> = BTreeMap::new();
    let mut fonts: BTreeMap<String, u64> = BTreeMap::new();
    let mut grids = Vec::new();
    let mut rooms = Vec::new();
    let mut grounds: Vec<(String, Option<String>, Vec<Area>)> = Vec::new();
    let name_of = |id: &str| leaves.iter().find(|l| l.id == id).map(|l| l.name.clone());
    let codes: BTreeMap<String, String> = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| {
            let levels: Vec<(String, f64)> = a.levels.iter().map(|l| (l.id.clone(), l.elevation)).collect();
            kentos_sheet::permit::level_codes(&levels).into_iter().collect()
        })
        .unwrap_or_default();
    for (uid, e) in doc.entities_with_uids() {
        let layer = e.base().layer_id.as_str();
        let count = objects.entry(layer).or_default();
        count.objects += 1;
        count.first.push(uid.to_string());
        let class = if generated.contains(&uid.to_string()) { Class::Model } else { class(e) };
        match class {
            Class::Model => count.model += 1,
            Class::Annotation => count.annotations += 1,
            Class::Free => {
                count.free += 1;
                count.first_free.push(uid.to_string());
            }
            Class::Other => {}
        }
        match e {
            Entity::Hatch(_) => {
                count.hatches += 1;
                count.first_hatches.push(uid.to_string());
            }
            Entity::Tag(t) if doc.tag_target(&t.tag.of).is_none() => {
                broken.push(CadBroken { kind: "tag".into(), uid: uid.to_string() });
            }
            Entity::Insert(x) => {
                let name = doc.block(x.block).map_or_else(|| x.block.to_string(), |b| b.name.clone());
                let block = blocks.entry(name.clone()).or_insert_with(|| CadBlock { name, inserts: 0, layers: Vec::new() });
                block.inserts += 1;
                block.layers.push(name_of(layer).unwrap_or_else(|| layer.to_owned()));
            }
            Entity::Dimension(d) => {
                let tied = d.refs.as_deref().is_some_and(|r| {
                    [&r.a, &r.b, &r.c].into_iter().any(|x| x.as_ref().is_some_and(|x| x.broken.is_some()))
                });
                if tied {
                    broken.push(CadBroken { kind: "dimension".into(), uid: uid.to_string() });
                }
                let settings = doc.settings();
                let styled = d.look.dim_style.as_deref().and_then(|id| {
                    settings.dimension_styles.iter().find(|s| s.id == id).and_then(|s| s.font)
                });
                let font = d.look.font.or(styled).or(settings.drawing_font);
                *fonts.entry(font_name(font).to_owned()).or_default() += 1;
            }
            Entity::Grid(_) => grids.push(CadGrid { uid: uid.to_string(), layer: layer.to_owned() }),
            Entity::Space(s) => {
                let level = e.own_level().map(str::to_owned);
                let areas = core_areas(&s.space.parts);
                // Its floor and its storey's range (docs/adr/8125; EK-9/13 "doğru katta").
                let range = level.as_deref().and_then(|l| storey_range(doc, l));
                rooms.push(CadRoom {
                    uid: uid.to_string(),
                    number: s.space.number.clone(),
                    name: s.space.name.clone(),
                    level_code: level.as_ref().and_then(|l| codes.get(l).cloned()),
                    level: level.clone(),
                    at: centre(&areas),
                    floor: range.and(doc.room_height(&s.space)).map(|h| h.floor),
                    storey_from: range.map(|r| r.0),
                    storey_to: range.and_then(|r| r.1),
                });
                grounds.push((uid.to_string(), level, areas));
            }
            _ => {}
        }
    }
    let layers: Vec<CadLayer> = leaves
        .iter()
        .map(|l| {
            let c = objects.remove(l.id.as_str()).unwrap_or_default();
            CadLayer {
                id: l.id.clone(),
                name: l.name.clone(),
                objects: c.objects,
                hatches: c.hatches,
                first: c.first,
                first_hatches: c.first_hatches,
                model: c.model,
                annotations: c.annotations,
                free: c.free,
                first_free: c.first_free,
                line_type: l.style.line_type.name().to_owned(),
                line_pattern: l.style.line_pattern.clone(),
            }
        })
        .collect();
    CadFacts {
        layers,
        blocks: blocks.into_values().collect(),
        dimension_fonts: fonts.into_iter().map(|(font, count)| FontUse { font, count }).collect(),
        grids,
        rooms,
        derived: derived(doc),
        broken,
        room_overlaps: overlaps(&grounds),
        model_gaps: model_gaps(doc),
        exits: exits(doc),
    }
}

/// A storey's elevation range over ±0.00 (docs/adr/8125): its level to the next storey's, else to its level plus its
/// height; none for a level the project lacks.
fn storey_range(doc: &Document, level: &str) -> Option<(f64, Option<f64>)> {
    let mut levels: Vec<&kentos_commands::ArchLevel> = doc.settings().arch.as_ref()?.levels.iter().collect();
    levels.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    let i = levels.iter().position(|l| l.id == level)?;
    let l = levels[i];
    let to = levels.get(i + 1).map(|n| n.elevation).or_else(|| l.height.map(|h| l.elevation + h));
    Some((l.elevation, to))
}

/// Every door and stair, its declared exit role and what the escape routes take it as (docs/adr/8124).
#[cfg(feature = "mod-arch")]
fn exits(doc: &Document) -> Vec<CadExit> {
    use kentos_commands::OpeningForm;
    use kentos_native_application::arch_performance::exit_of;
    let by: std::collections::HashMap<String, kentos_domain::escape::ExitWhy> =
        kentos_domain::escape::escape(doc).exits_by.into_iter().map(|x| (x.uid, x.why)).collect();
    doc.entities_with_uids()
        .filter_map(|(uid, e)| {
            let (kind, name) = match e {
                Entity::Opening(o) if o.opening.form == OpeningForm::Door => ("door", o.opening.number.clone()),
                Entity::Stair(_) => ("stair", None),
                _ => return None,
            };
            let uid = uid.to_string();
            // A door stands on its wall's storey.
            let level = match e {
                Entity::Opening(o) => kentos_domain::Uuid::parse_str(&o.opening.host)
                    .ok()
                    .and_then(|u| doc.get(doc.slot_of(u)?))
                    .and_then(|w| w.own_level())
                    .map(str::to_owned),
                _ => e.own_level().map(str::to_owned),
            };
            Some(CadExit {
                kind: kind.to_owned(),
                level,
                name,
                declared: exit_of(e).map(|x| x.exit),
                route: by.get(&uid).map(|w| w.key().to_owned()),
                uid,
            })
        })
        .collect()
}

/// Without Mimari there are no exit roles to read.
#[cfg(not(feature = "mod-arch"))]
fn exits(_: &Document) -> Vec<CadExit> {
    Vec::new()
}

/// EK-6's and EK-7's mandatory rows the model `cad.arch.ifc` writes does not carry (docs/adr/8068): none for a drawing
/// whose model has no element (no building to ask).
#[cfg(feature = "mod-arch")]
fn model_gaps(doc: &Document) -> Vec<CadModelGap> {
    let (building, _, _) = kentos_native_application::arch_ifc::model(doc);
    if building.elements.is_empty() {
        return Vec::new();
    }
    let classes = &kentos_ifc::catalog::base().classes;
    kentos_ifc::required::gaps(&building)
        .into_iter()
        .map(|g| CadModelGap {
            class_tr: classes
                .iter()
                .find(|c| c.class.eq_ignore_ascii_case(&g.class))
                .map_or_else(|| g.class.clone(), |c| c.tr.clone()),
            missing: g.missing.iter().map(|id| id.map(|i| kentos_domain::Uuid::from_u128(i).to_string()).unwrap_or_default()).collect(),
            of: g.of as u32,
            class: g.class,
            path: g.path,
            tr: g.tr,
            table: g.table,
            page: g.page,
        })
        .collect()
}

#[cfg(not(feature = "mod-arch"))]
fn model_gaps(_doc: &Document) -> Vec<CadModelGap> {
    Vec::new()
}

/// Each pair of rooms of one storey (or both on none) whose areas share ground: what they share and its centre. Only
/// the pairs whose boxes meet are overlaid.
fn overlaps(grounds: &[(String, Option<String>, Vec<Area>)]) -> Vec<CadRoomOverlap> {
    let boxes: Vec<Option<[f64; 4]>> = grounds
        .iter()
        .map(|(_, _, areas)| {
            areas.iter().flat_map(|a| a.outer.pts.iter()).fold(None, |b: Option<[f64; 4]>, p| {
                Some(b.map_or([p.x, p.y, p.x, p.y], |b| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]))
            })
        })
        .collect();
    let mut out = Vec::new();
    for i in 0..grounds.len() {
        for j in i + 1..grounds.len() {
            let ((ua, la, aa), (ub, lb, ab)) = (&grounds[i], &grounds[j]);
            let (Some(p), Some(q)) = (boxes[i], boxes[j]) else { continue };
            if la != lb || p[2] <= q[0] || q[2] <= p[0] || p[3] <= q[1] || q[3] <= p[1] {
                continue;
            }
            let shared = intersect_area_sets(&[aa.clone(), ab.clone()]);
            let area: f64 = shared.iter().map(net_area).sum();
            if area > 0.0 {
                out.push(CadRoomOverlap { a: ua.clone(), b: ub.clone(), area, at: centre(&shared) });
            }
        }
    }
    out
}

/// A layer's objects counted.
#[derive(Default)]
struct Counts {
    objects: u64,
    hatches: u64,
    model: u64,
    annotations: u64,
    free: u64,
    first: Vec<String>,
    first_hatches: Vec<String>,
    first_free: Vec<String>,
}

/// What an object is to a sheet (docs/adr/8065): the model's, an annotation tied to the drawing, free drawing, or else.
enum Class {
    Model,
    Annotation,
    Free,
    Other,
}

fn class(e: &Entity) -> Class {
    match e {
        Entity::Wall(_)
        | Entity::Opening(_)
        | Entity::Space(_)
        | Entity::Slab(_)
        | Entity::Grid(_)
        | Entity::Column(_)
        | Entity::Beam(_)
        | Entity::Roof(_)
        | Entity::Railing(_)
        | Entity::Stair(_)
        | Entity::Ramp(_)
        | Entity::Insert(_) => Class::Model,
        Entity::Text(_)
        | Entity::Dimension(_)
        | Entity::Leader(_)
        | Entity::Table(_)
        | Entity::Tag(_)
        | Entity::ViewMark(_) => Class::Annotation,
        Entity::Point(_)
        | Entity::Line(_)
        | Entity::Polyline(_)
        | Entity::Polygon(_)
        | Entity::Circle(_)
        | Entity::Arc(_)
        | Entity::Ellipse(_)
        | Entity::Spline(_)
        | Entity::Nurbs(_)
        | Entity::Xline(_)
        | Entity::Ray(_)
        | Entity::Hatch(_) => Class::Free,
        _ => Class::Other,
    }
}

/// The sections and elevations with their states (`kentos_native_application::sections_query`, docs/adr/8042 §5).
#[cfg(feature = "mod-arch")]
fn derived(doc: &Document) -> Vec<CadDerived> {
    use kentos_commands::BuildingSectionState as S;
    kentos_native_application::sections_query::rows(doc)
        .into_iter()
        .map(|r| CadDerived {
            kind: if r.name.ends_with("görünüşü") { "elevation" } else { "section" }.into(),
            state: match r.state {
                S::Current => "current",
                S::Stale => "stale",
                S::NotDrawn => "notDrawn",
                S::Gone => "gone",
            }
            .into(),
            name: r.name,
            mark: r.mark,
            inserts: u64::from(r.inserts),
        })
        .collect()
}

#[cfg(not(feature = "mod-arch"))]
fn derived(_: &Document) -> Vec<CadDerived> {
    Vec::new()
}

/// A typeface's name as a person says it.
fn font_name(font: Option<DrawingFont>) -> &'static str {
    match font {
        None | Some(DrawingFont::Barlow) => "Barlow",
        Some(DrawingFont::Arimo) => "Arimo",
        Some(DrawingFont::Overpass) => "Overpass",
        Some(DrawingFont::Quicksand) => "Quicksand",
        Some(DrawingFont::ArchitectsDaughter) => "Architects Daughter",
        Some(DrawingFont::CourierPrime) => "Courier Prime",
        Some(DrawingFont::PlexMono) => "IBM Plex Mono",
    }
}

/// The area-weighted centroid of `areas`' rings by their vertices (an outer ring adds, a hole takes away, whichever way
/// round they run), else their points' mean: a place to order the rooms by, not a measure.
fn centre(areas: &[kentos_geometry_core::geom::arrangement::Area]) -> [f64; 2] {
    // A ring's area (unsigned) and its centroid's coordinates times it.
    let moment = |pts: &[Vec2]| {
        let (mut a, mut x, mut y) = (0.0, 0.0, 0.0);
        let n = pts.len();
        for i in 0..n {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            let cross = p.x * q.y - q.x * p.y;
            a += cross;
            x += (p.x + q.x) * cross;
            y += (p.y + q.y) * cross;
        }
        let sign = if a < 0.0 { -1.0 } else { 1.0 };
        (sign * a / 2.0, sign * x / 6.0, sign * y / 6.0)
    };
    let (mut sa, mut sx, mut sy) = (0.0, 0.0, 0.0);
    let mut points: Vec<Vec2> = Vec::new();
    for area in areas {
        let (a, x, y) = moment(&area.outer.pts);
        (sa, sx, sy) = (sa + a, sx + x, sy + y);
        for hole in &area.holes {
            let (a, x, y) = moment(&hole.pts);
            (sa, sx, sy) = (sa - a, sx - x, sy - y);
        }
        points.extend(area.outer.pts.iter().copied());
    }
    if sa > 1e-12 {
        return [sx / sa, sy / sa];
    }
    let n = points.len().max(1) as f64;
    [points.iter().map(|p| p.x).sum::<f64>() / n, points.iter().map(|p| p.y).sum::<f64>() / n]
}

#[cfg(test)]
mod tests;
