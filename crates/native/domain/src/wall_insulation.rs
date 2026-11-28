//! İçte uygulanan ısı ve ses yalıtımı (docs/adr/8151; Binaların Yangından Korunması Hakkında Yönetmelik 20/3:
//! "Duvarlarda iç kaplamalar ile içte uygulanacak ısı ve ses yalıtımları"): each wall's insulation layers on its inner
//! side, the rooms they face, and whether that side was found. The classes the standards package reads.
//!
//! - **An insulation layer** is a component of the wall bound to a project material of the category Yalıtım (by its
//!   `materialId`, else its material's name or id, case aside).
//! - **An interior wall** (no exterior role, docs/adr/8082, 8124): inside the building on both faces, every insulation
//!   layer is inside; the rooms on both sides.
//! - **An exterior wall:** its inner face is the side a room of its storey lies on, found walking out from the middle
//!   of its longest straight segment in steps of 2 cm (as far as its width and 50 cm); its inner layers are those from
//!   that face to its first load-bearing layer (a material of the category Taşıyıcı), the layers outside it are the
//!   façade's (BYKHY 27, not 20/3). A wall with no load-bearing layer keeps all its insulation inside (the strict
//!   side). With a room on neither side or both, its inner face is not found: all its insulation layers, `side_known`
//!   false.

use kentos_contracts::{ArchMaterial, Entity, MaterialCategory, WallComponent};
use kentos_formats::walls::core_wall;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::inside_area;
use kentos_geometry_core::vec2::Vec2;

use crate::Document;

/// A wall's inner insulation (docs/adr/8151): its persistent id and storey, whether it is exterior, its insulation
/// layers on the inner side, the rooms on that side (both sides for an interior wall) by persistent id, and whether the
/// inner side was found.
#[derive(Clone, Debug, PartialEq)]
pub struct InnerInsulation {
    pub wall: String,
    pub level: Option<String>,
    pub exterior: bool,
    pub layers: Vec<InsulationLayer>,
    pub rooms: Vec<String>,
    pub side_known: bool,
}

/// An insulation layer: the component's name and its project material (name and id).
#[derive(Clone, Debug, PartialEq)]
pub struct InsulationLayer {
    pub component: String,
    pub material: String,
    pub material_id: String,
}

/// How far a side is walked past the wall's width, and the step, m.
const BEYOND: f64 = 0.50;
const STEP: f64 = 0.02;

/// A component's project material: by its `materialId`, else its material's name or id (case aside).
fn material_of<'a>(c: &WallComponent, materials: &'a [ArchMaterial]) -> Option<&'a ArchMaterial> {
    if let Some(id) = c.material_id.as_deref() {
        return materials.iter().find(|m| m.id == id);
    }
    let name = c
        .material
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())?;
    let folded = name.to_lowercase();
    materials
        .iter()
        .find(|m| m.id == name || m.name.to_lowercase() == folded)
}

impl Document {
    /// Each wall with an insulation layer inside, in the drawing's order (docs/adr/8151).
    pub fn inner_insulations(&self) -> Vec<InnerInsulation> {
        let Some(arch) = self.settings().arch.as_ref() else {
            return Vec::new();
        };
        let materials = arch.materials.as_slice();
        if !materials
            .iter()
            .any(|m| m.category == MaterialCategory::Insulation)
        {
            return Vec::new();
        }
        let exterior: std::collections::BTreeSet<String> = self
            .fire_elements()
            .into_iter()
            .filter(|e| e.kind == "wall" && e.exterior)
            .map(|e| e.uid)
            .collect();
        let rooms: Vec<(String, Option<String>, Vec<Area>)> = self.axis_rooms();
        let mut out = Vec::new();
        for (uid, e) in self.entities_with_uids() {
            let Entity::Wall(w) = e else { continue };
            if w.wall.curtain.is_some() || w.wall.components.is_empty() {
                continue;
            }
            let kinds: Vec<Option<MaterialCategory>> = w
                .wall
                .components
                .iter()
                .map(|c| material_of(c, materials).map(|m| m.category))
                .collect();
            if !kinds.contains(&Some(MaterialCategory::Insulation)) {
                continue;
            }
            let uid = uid.to_string();
            let level = e.own_level().map(str::to_owned);
            let Shape::Wall {
                pts, wall: spec, ..
            } = core_wall(&w.wall)
            else {
                continue;
            };
            // The rooms on each side, walking out from the middle of the longest straight segment.
            let side = |sign: f64| -> Option<String> {
                let (a, b) = pts.windows(2).map(|p| (p[0], p[1])).max_by(|x, y| {
                    let l = |s: &(Vec2, Vec2)| (s.1.x - s.0.x).hypot(s.1.y - s.0.y);
                    l(x).total_cmp(&l(y))
                })?;
                let len = (b.x - a.x).hypot(b.y - a.y);
                if len <= 0.0 {
                    return None;
                }
                let (dx, dy) = ((b.x - a.x) / len, (b.y - a.y) / len);
                let m = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                let steps = ((spec.width + BEYOND) / STEP).ceil() as usize;
                (1..=steps).find_map(|k| {
                    let s = sign * STEP * k as f64;
                    let p = Vec2::new(m.x - dy * s, m.y + dx * s);
                    rooms
                        .iter()
                        .find(|(_, l, areas)| {
                            *l == level && areas.iter().any(|x| inside_area(x, p))
                        })
                        .map(|(r, _, _)| r.clone())
                })
            };
            let (left, right) = (side(1.0), side(-1.0));
            let is_exterior = exterior.contains(&uid);
            let insulation = |i: usize| kinds[i] == Some(MaterialCategory::Insulation);
            let layer = |i: usize| {
                let c = &w.wall.components[i];
                let m = material_of(c, materials);
                InsulationLayer {
                    component: c.name.clone(),
                    material: m.map(|m| m.name.clone()).unwrap_or_default(),
                    material_id: m.map(|m| m.id.clone()).unwrap_or_default(),
                }
            };
            let n = kinds.len();
            let (indices, side_rooms, known): (Vec<usize>, Vec<String>, bool) = if !is_exterior {
                (
                    (0..n).filter(|&i| insulation(i)).collect(),
                    left.iter().chain(right.iter()).cloned().collect(),
                    true,
                )
            } else {
                // From the inner face to the first load-bearing layer.
                let structure = |i: &usize| kinds[*i] == Some(MaterialCategory::Structure);
                match (&left, &right) {
                    (Some(r), None) => {
                        let stop = (0..n).find(structure).unwrap_or(n);
                        (
                            (0..stop).filter(|&i| insulation(i)).collect(),
                            vec![r.clone()],
                            true,
                        )
                    }
                    (None, Some(r)) => {
                        let stop = (0..n).rev().find(structure).map_or(0, |i| i + 1);
                        (
                            (stop..n).filter(|&i| insulation(i)).collect(),
                            vec![r.clone()],
                            true,
                        )
                    }
                    _ => (
                        (0..n).filter(|&i| insulation(i)).collect(),
                        Vec::new(),
                        false,
                    ),
                }
            };
            if indices.is_empty() {
                continue;
            }
            out.push(InnerInsulation {
                wall: uid,
                level,
                exterior: is_exterior,
                layers: indices.into_iter().map(layer).collect(),
                rooms: side_rooms,
                side_known: known,
            });
        }
        out
    }
}
