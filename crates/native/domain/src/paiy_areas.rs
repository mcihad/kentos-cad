//! The areas Planlı Alanlar İmar Yönetmeliği's article 4 defines for an independent unit (docs/adr/8029; ADV-22, M29),
//! each group of the building program's measured from the drawing:
//!
//! - **BB brüt alanı** (4/ğ-1): the area its outer contour encloses, the exterior wall's outer face on the façade and
//!   the wall's axis where it meets another unit or a common area; shafts, light wells and galleries out. Each room's
//!   outer-face outline (`SpaceMethod::Gross`) together, less every other room's axis outline (`Centerline`, the voids'
//!   too) on its level: a wall between two of its rooms is its own, one it shares is cut at its axis.
//! - **BB net alanı** (4/ğ-3): its rooms' clean area between the walls, voids (shaft, gallery) and outdoor rooms
//!   (balcony, terrace) out. KentOS's wall styles keep their plaster inside the wall (İç sıva 2 cm), so a room's inner
//!   faces are the plastered ones and the article's 2.5 cm allowance is not taken off again.
//! - **Ortak alan payı** and **BB genel brüt alanı** (4/ğ-2): the common areas' gross shared out by the program's rule
//!   (`kentos.share/1`, docs/adr/0784), over the units' total gross areas; the unit's total gross and its part.
//! - **BB toplam brüt alanı** (4/ğ-4; docs/adr/8031): the unit's gross and its annexes' (groups of kind `annex`, Kat
//!   Mülkiyeti Kanunu 2/a, 6): an annex is measured as a group of its own and summed into its unit.
//! - **Arsa payı** (docs/adr/8031; `kentos.land/1`): the units' land shares by their total gross and value factors.
//!
//! A room drawn by its outline (no seed) cannot be measured to another line: its own rings stand for all three.

use kentos_contracts::{Entity, SpaceFields, SpaceMethod};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region;

use crate::Document;
use crate::arch_values::core_areas;
use crate::space_follow::SpaceSearch;

/// A program group's article-4 areas, m².
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaiyGroup {
    pub gross: f64,
    pub net: f64,
    /// Its part of the common areas' gross (an independent unit's; none: the program shares nothing).
    pub share: Option<f64>,
    /// Each storey's part of its gross and net, in the order the rooms first stand on them (the metrekare cetveli's).
    pub levels: Vec<(Option<String>, f64, f64)>,
    /// An independent unit's annexes' gross and net areas summed (docs/adr/8031); 0 for another group.
    pub annex_gross: f64,
    pub annex_net: f64,
}

impl PaiyGroup {
    /// BB toplam brüt alanı (4/ğ-4): its gross and its annexes'.
    pub fn total(&self) -> f64 {
        self.gross + self.annex_gross
    }

    /// BB genel brüt alanı (4/ğ-2): its total gross and its part.
    pub fn general(&self) -> Option<f64> {
        self.share.map(|s| self.total() + s)
    }
}

/// One room as the article measures it.
struct Room {
    level: Option<String>,
    group: Option<String>,
    counts: bool,
    void: bool,
    gross: Vec<Area>,
    axis: Vec<Area>,
    net: f64,
}

/// `s` measured to its walls' `method` line: found again from its seed, else its own rings.
fn measured(search: &mut SpaceSearch<'_>, s: &SpaceFields, method: SpaceMethod) -> Vec<Area> {
    if s.seed.is_some() && s.method != method {
        let mut other = s.clone();
        other.method = method;
        let found = search.found_preserving_legacy(&other);
        if found.broken.is_none() && !found.parts.is_empty() {
            return core_areas(&found.parts);
        }
    }
    core_areas(&s.parts)
}

impl Document {
    /// Each group of the building program with rooms, its article-4 areas (docs/adr/8029), in the program's order.
    pub fn paiy_areas(&self) -> Vec<(String, PaiyGroup)> {
        let program = self.program();
        if program.units.is_empty() {
            return Vec::new();
        }
        let mut search = SpaceSearch::new(self);
        let mut rooms = Vec::new();
        for e in self.entities() {
            let Entity::Space(s) = e else { continue };
            let x = &s.space;
            let group = x.unit.clone().filter(|u| program.unit(u).is_some());
            let outdoor = x
                .usage
                .as_deref()
                .and_then(|u| program.brief(u))
                .is_some_and(|b| b.outdoor);
            let net = if x.void || outdoor {
                0.0
            } else {
                search.net_area(x).unwrap_or_else(|| {
                    kentos_geometry_core::geom::space::area(&core_areas(&x.parts))
                })
            };
            rooms.push(Room {
                level: x.arch.level.clone(),
                group,
                counts: !x.void && !outdoor,
                void: x.void,
                gross: measured(&mut search, x, SpaceMethod::Gross),
                axis: measured(&mut search, x, SpaceMethod::Centerline),
                net,
            });
        }
        let mut out: Vec<(String, PaiyGroup)> = program
            .units
            .iter()
            .filter(|u| {
                rooms
                    .iter()
                    .any(|r| !r.void && r.group.as_deref() == Some(u.id.as_str()))
            })
            .map(|u| {
                let net = rooms
                    .iter()
                    .filter(|r| r.counts && r.group.as_deref() == Some(u.id.as_str()))
                    .map(|r| r.net)
                    .sum();
                (
                    u.id.clone(),
                    PaiyGroup {
                        gross: 0.0,
                        net,
                        share: None,
                        levels: Vec::new(),
                        annex_gross: 0.0,
                        annex_net: 0.0,
                    },
                )
            })
            .collect();
        let levels: std::collections::BTreeSet<&Option<String>> =
            rooms.iter().map(|r| &r.level).collect();
        for level in levels {
            // Each group's contour on the level: its rooms' outer-face outlines, every other room's axis outline out.
            let mut contours: Vec<(usize, Vec<Area>, f64)> = Vec::new();
            for (k, (id, _)) in out.iter().enumerate() {
                let own: Vec<Area> = rooms
                    .iter()
                    .filter(|r| {
                        !r.void && r.level == *level && r.group.as_deref() == Some(id.as_str())
                    })
                    .flat_map(|r| r.gross.iter().cloned())
                    .collect();
                if own.is_empty() {
                    continue;
                }
                let others: Vec<Area> = rooms
                    .iter()
                    .filter(|r| {
                        r.level == *level && (r.void || r.group.as_deref() != Some(id.as_str()))
                    })
                    .flat_map(|r| r.axis.iter().cloned())
                    .collect();
                let joined = region::union_areas(&own);
                let cut = if others.is_empty() {
                    joined
                } else {
                    region::subtract_areas(&joined, &region::union_areas(&others))
                };
                let a = kentos_geometry_core::geom::space::area(&cut);
                contours.push((k, cut, a));
            }
            // Where a shared wall meets the façade both neighbours reach its outer half past the axis: that bit is
            // halved between them (the axis is the wall's middle, so halving is exact).
            let mut gross: Vec<f64> = contours.iter().map(|(_, _, a)| *a).collect();
            for i in 0..contours.len() {
                for j in i + 1..contours.len() {
                    let both = region::intersect_area_sets(&[
                        contours[i].1.clone(),
                        contours[j].1.clone(),
                    ]);
                    let o = kentos_geometry_core::geom::space::area(&both);
                    if o > 0.0 {
                        gross[i] -= o / 2.0;
                        gross[j] -= o / 2.0;
                    }
                }
            }
            for ((k, _, _), g) in contours.iter().zip(gross) {
                let id = out[*k].0.clone();
                let net: f64 = rooms
                    .iter()
                    .filter(|r| {
                        r.counts && r.level == *level && r.group.as_deref() == Some(id.as_str())
                    })
                    .map(|r| r.net)
                    .sum();
                out[*k].1.gross += g;
                out[*k].1.levels.push((level.clone(), g, net));
            }
        }
        // Each independent unit's annexes (docs/adr/8031), summed into it.
        for u in program
            .units
            .iter()
            .filter(|u| u.kind == kentos_contracts::UnitKind::Unit)
        {
            let (mut gross, mut net) = (0.0, 0.0);
            for a in program.annexes_of(&u.id) {
                if let Some((_, g)) = out.iter().find(|(id, _)| *id == a.id) {
                    gross += g.gross;
                    net += g.net;
                }
            }
            if let Some((_, g)) = out.iter_mut().find(|(id, _)| *id == u.id) {
                g.annex_gross = gross;
                g.annex_net = net;
            }
        }
        // The common areas' gross shared out by the program's rule, over the total gross areas.
        if program.share_rule.is_some() {
            let of = |id: &str| {
                out.iter()
                    .find(|(u, _)| u == id)
                    .map_or(0.0, |(_, g)| g.total())
            };
            if let Some((_, parts)) = kentos_contracts::shares(&program, &of) {
                for (id, part) in parts {
                    if let Some((_, g)) = out.iter_mut().find(|(u, _)| *u == id) {
                        g.share = part.share.parse::<f64>().ok();
                    }
                }
            }
        }
        out
    }

    /// Arsa payı (docs/adr/8031; Kat Mülkiyeti Kanunu 3, 12/a): the independent units' land shares by their total gross
    /// and value factors, over the program's whole, else the parcel's area; none without units; the words when they
    /// cannot be given.
    pub fn land_shares(&self) -> Result<Option<kentos_contracts::LandShares>, String> {
        let program = self.program();
        let areas = self.paiy_areas();
        let total = |id: &str| {
            areas
                .iter()
                .find(|(u, _)| u == id)
                .map_or(0.0, |(_, g)| g.total())
        };
        let parcel = self
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.zoning)
            .map(|z| z.parcel_area);
        kentos_contracts::land_shares(&program, &total, parcel)
    }
}
