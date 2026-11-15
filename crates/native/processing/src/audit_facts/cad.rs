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

use std::collections::BTreeMap;

use kentos_commands::{DrawingFont, Entity};
use kentos_domain::Document;
use kentos_domain::arch_values::core_areas;
use kentos_extensions::audit_facts::cad::{CadBlock, CadFacts, CadGrid, CadLayer, CadRoom, FontUse};
use kentos_geometry_core::Vec2;

/// The drawing's CAD facts.
pub fn measure(doc: &Document) -> CadFacts {
    let leaves = doc.layers().leaves();
    // Per layer: objects, hatches, their persistent ids.
    let mut objects: BTreeMap<&str, (u64, u64, Vec<String>, Vec<String>)> = BTreeMap::new();
    let mut blocks: BTreeMap<String, CadBlock> = BTreeMap::new();
    let mut fonts: BTreeMap<String, u64> = BTreeMap::new();
    let mut grids = Vec::new();
    let mut rooms = Vec::new();
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
        count.0 += 1;
        count.2.push(uid.to_string());
        match e {
            Entity::Hatch(_) => {
                count.1 += 1;
                count.3.push(uid.to_string());
            }
            Entity::Insert(x) => {
                let name = doc.block(x.block).map_or_else(|| x.block.to_string(), |b| b.name.clone());
                let block = blocks.entry(name.clone()).or_insert_with(|| CadBlock { name, inserts: 0, layers: Vec::new() });
                block.inserts += 1;
                block.layers.push(name_of(layer).unwrap_or_else(|| layer.to_owned()));
            }
            Entity::Dimension(d) => {
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
                rooms.push(CadRoom {
                    uid: uid.to_string(),
                    number: s.space.number.clone(),
                    name: s.space.name.clone(),
                    level_code: level.as_ref().and_then(|l| codes.get(l).cloned()),
                    level,
                    at: centre(&core_areas(&s.space.parts)),
                });
            }
            _ => {}
        }
    }
    let layers: Vec<CadLayer> = leaves
        .iter()
        .map(|l| {
            let (objects, hatches, first, first_hatches) = objects.remove(l.id.as_str()).unwrap_or_default();
            CadLayer {
                id: l.id.clone(),
                name: l.name.clone(),
                objects,
                hatches,
                first,
                first_hatches,
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
    }
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
