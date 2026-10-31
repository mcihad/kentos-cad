//! `cad.spaces.quantities` v1 (docs/adr/0342 §4, plan/cad.md `CAD-ARCH-28`, `29`), read-only: the drawing's spaces,
//! walls and openings measured from their source, those of the levels, phases, usages and layers given; each row says
//! the rule its numbers come from. Nothing is written; the plan answers the same. The shared cases are
//! `fixtures/commands/v1/cad.spaces.quantities.json`.
//!
//! - A space: its rings' area and length; its height its own, else its level's; its volume area × height; a space
//!   from the walls its area by each method too.
//! - A wall: its base line's length; its height its own, else its level's; its gross face length × height; its
//!   openings' area (width × (head − sill), sill absent 0, those with a head and going through; docs/adr/0719 §4: a
//!   niche or flue does not); its net face the gross less the openings; its volume the net face × width less its pocket
//!   doors' cavities (docs/adr/0787 §3: each pocket's length × width along the wall's middle × its door's height).
//! - An opening: its width, its height head − sill, its area; a niche's or flue's depth. The totals' opening area is
//!   the through openings'.
//! - A room's baseboard, a slab, the floor finishes' changes at the passages: `slab_quantities` (docs/adr/0722 §5).
//! - A ramp (docs/adr/0723 §3): its centre line's length, its sloped runs' (the line less its landings), its landings,
//!   its band's plan area (mitred at its turns), its slope as drawn (the rise's size over the sloped length), its start
//!   and end elevations, its railings' length.

use std::collections::BTreeMap;

use kentos_commands::{
    ArchInfo, CommandResult, Entity, OpeningRow, QuantityTotals, SpaceMethod, SpaceRow,
    SpacesQuantified, SpacesQuantities, WallRow,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::spaces::{self, fail};

fn level_height(doc: &Document, arch: &ArchInfo) -> Option<f64> {
    let id = arch.level.as_deref()?;
    doc.settings()
        .arch
        .as_ref()?
        .levels
        .iter()
        .find(|l| l.id == id)?
        .height
}

fn check(doc: &Document, input: &SpacesQuantities) -> Result<SpacesQuantified, Stop> {
    let known: Vec<String> = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.iter().map(|l| l.id.clone()).collect())
        .unwrap_or_default();
    for (k, l) in input.levels.iter().flatten().enumerate() {
        if !known.contains(l) {
            return Err(fail(
                codes::LEVEL_NOT_FOUND,
                format!("Projede “{l}” kimlikli kat yok. Var olan bir katın kimliğini verin."),
                &format!("levels[{k}]"),
            ));
        }
    }
    for (k, l) in input.layers.iter().flatten().enumerate() {
        if doc.layers().get(l).is_none() {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{l}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                &format!("layers[{k}]"),
            ));
        }
    }
    let takes = |e: &Entity, arch: &ArchInfo| {
        input
            .layers
            .as_ref()
            .is_none_or(|ls| ls.contains(&e.base().layer_id))
            && input
                .levels
                .as_ref()
                .is_none_or(|ls| arch.level.as_ref().is_some_and(|l| ls.contains(l)))
            && input
                .phases
                .as_ref()
                .is_none_or(|ps| arch.phase.is_some_and(|p| ps.contains(&p)))
    };
    let uid = |e: &Entity| {
        doc.uid(Slot(e.base().id))
            .map(|u| u.to_string())
            .unwrap_or_default()
    };
    let mut out = SpacesQuantified {
        spaces: Vec::new(),
        walls: Vec::new(),
        openings: Vec::new(),
        slabs: Vec::new(),
        transitions: Vec::new(),
        ramps: Vec::new(),
        stairs: Vec::new(),
        materials: Vec::new(),
        totals: QuantityTotals::default(),
    };
    // What Metraj by material reads (docs/adr/0785 §4).
    let mut wall_parts: Vec<crate::material_quantities::WallPart<'_>> = Vec::new();
    let mut slab_parts: Vec<crate::material_quantities::SlabPart<'_>> = Vec::new();
    // The doors and gaps to the floor (docs/adr/0722 §5), and the voids a slab's area loses, by level.
    let passages = crate::slab_quantities::passages(doc);
    let voids: Vec<(Option<String>, Vec<kentos_commands::SpacePart>)> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Space(s) if s.space.void => Some((s.space.arch.level.clone(), s.space.parts.clone())),
            _ => None,
        })
        .collect();
    let mut rooms: Vec<crate::slab_quantities::Room<'_>> = Vec::new();
    // The openings of each wall, by the wall's id.
    let mut on_wall: BTreeMap<String, Vec<(String, f64)>> = BTreeMap::new();
    // The walls with a niche or flue: their rule says those stay (docs/adr/0719 §4).
    let mut recessed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // Each wall's pocket doors' cavities, m³ (docs/adr/0787 §3).
    let mut pocketed: BTreeMap<String, f64> = BTreeMap::new();
    for e in doc.entities() {
        let Entity::Opening(o) = e else { continue };
        let x = &o.opening;
        let wall = doc.wall_line_of(&x.host).is_some().then(|| x.host.clone());
        if let (Some(w), Some(v)) = (&wall, pockets_volume(doc, x)) {
            *pocketed.entry(w.clone()).or_insert(0.0) += v;
        }
        let height = x.head.map(|h| h - x.sill.unwrap_or(0.0));
        let area = height.map(|h| x.width * h);
        // A niche or flue does not go through: its wall's net face keeps it (docs/adr/0719 §4).
        if let (Some(w), Some(a), false) = (&wall, area, x.form.is_recess()) {
            on_wall.entry(w.clone()).or_default().push((uid(e), a));
        }
        if let (Some(w), true) = (&wall, x.form.is_recess()) {
            recessed.insert(w.clone());
        }
        let host_arch = wall
            .as_ref()
            .and_then(|w| kentos_domain::Uuid::parse_str(w).ok())
            .and_then(|u| doc.slot_of(u))
            .and_then(|s| match doc.get(s) {
                Some(Entity::Wall(w)) => Some(w.wall.arch.clone()),
                _ => None,
            })
            .unwrap_or_default();
        if !takes(e, &host_arch) || input.usages.is_some() {
            continue;
        }
        out.openings.push(OpeningRow {
            uid: uid(e),
            form: x.form,
            style: x.style.clone(),
            number: x.number.clone(),
            wall,
            width: x.width,
            height,
            area,
            depth: x.form.is_recess().then_some(x.depth).flatten(),
        });
    }
    // One borrow owns all temporary source networks for this read-only report.
    let mut source_search = kentos_domain::space_follow::SpaceSearch::new(doc);
    for e in doc.entities() {
        match e {
            Entity::Space(s) => {
                let f = &s.space;
                if !takes(e, &f.arch)
                    || input
                        .usages
                        .as_ref()
                        .is_some_and(|us| f.usage.as_ref().is_none_or(|u| !us.contains(u)))
                {
                    continue;
                }
                let area = spaces::area_of(&f.parts);
                let height = f.height.or_else(|| level_height(doc, &f.arch));
                if !f.void {
                    rooms.push((uid(e), f.arch.level.clone(), &f.parts, f.finishes.floor.clone()));
                }
                let by_method = f.seed.map(|_| {
                    SpaceMethod::ALL
                        .iter()
                        .filter_map(|m| {
                            let mut g = f.clone();
                            g.method = *m;
                            let g = source_search.found_preserving_legacy(&g);
                            g.broken
                                .is_none()
                                .then(|| (m.name().to_owned(), spaces::area_of(&g.parts)))
                        })
                        .collect()
                });
                out.spaces.push(SpaceRow {
                    uid: uid(e),
                    number: f.number.clone(),
                    name: f.name.clone(),
                    usage: f.usage.clone(),
                    level: f.arch.level.clone(),
                    method: f.method,
                    columns: f.columns,
                    scope: f.scope,
                    sources: f.sources.clone(),
                    boundary_break: f.broken,
                    void: f.void,
                    area,
                    perimeter: spaces::perimeter_of(&f.parts),
                    height,
                    volume: height.map(|h| area * h),
                    baseboard: (!f.void)
                        .then(|| crate::slab_quantities::baseboard(&f.parts, f.arch.level.as_deref(), &passages)),
                    by_method,
                    rule: {
                        let mut rule = format!("alan: halkaların alanı, delikler düşülür ({}); hacim: alan × yükseklik",f.method.label());
                        if !f.columns.is_ignore() {
                            rule.push_str(&format!("; kolon alanı: {}",f.columns.label()));
                        }
                        if !f.scope.is_project() { rule.push_str("; kaynaklar: yalnız atanmamış"); }
                        if let Some(why)=f.broken {
                            rule.push_str(&format!("; son geçerli sınır: {}",why.words()));
                        }
                        if f.void {
                            rule.push_str("; boşluk: alanı mahal toplamına katılmaz, boşluk toplamındadır");
                        } else {
                            rule.push_str("; süpürgelik: çevreden kenarları boyunca duran geçişlerin (kapı, döşemeye inen boşluk) genişliği düşülür");
                        }
                        rule
                    },
                });
            }
            Entity::Slab(sl) if input.usages.is_none() => {
                if takes(e, &sl.slab.arch) {
                    let r = crate::slab_quantities::slab_row(doc, uid(e), &sl.slab, &voids);
                    slab_parts.push((&sl.slab, r.net_area, r.volume, !r.excluded));
                    out.slabs.push(r);
                }
            }
            Entity::Ramp(r) if input.usages.is_none() => {
                if takes(e, &r.ramp.arch) {
                    out.ramps.push(ramp_row(uid(e), &r.ramp));
                }
            }
            Entity::Stair(s) if input.usages.is_none() => {
                if takes(e, &s.stair.arch) {
                    out.stairs.push(stair_row(uid(e), &s.stair));
                }
            }
            Entity::Wall(w) if input.usages.is_none() => {
                let x = &w.wall;
                if !takes(e, &x.arch) {
                    continue;
                }
                let length = kentos_geometry_core::geom::bulge::bulge_path_length(
                    &crate::walls::core_points(&x.pts),
                    x.bulges.as_deref(),
                    x.closed,
                );
                let height = x.height.or_else(|| level_height(doc, &x.arch));
                let id = uid(e);
                let openings = on_wall.get(&id).cloned().unwrap_or_default();
                let openings_area = total(openings.iter().map(|(_, a)| *a));
                let gross = height.map(|h| length * h);
                let net = gross.map(|g| g - openings_area);
                let pockets = pocketed.get(&id).copied();
                let mut rule = if recessed.contains(&id) {
                    "yüzey: taban çizgisinin uzunluğu × yükseklik, duvarı geçen açıklıkların alanı düşülür (niş ve baca düşülmez); hacim: net yüzey × kalınlık"
                } else {
                    "yüzey: taban çizgisinin uzunluğu × yükseklik, açıklıkların alanı düşülür; hacim: net yüzey × kalınlık"
                }
                .to_owned();
                if pockets.is_some() {
                    rule.push_str(", cebe giren kapıların cebi düşülür (cebin boyu × eni × kapının yüksekliği; yüzeyler cebi düşmez)");
                }
                wall_parts.push((x, net));
                out.walls.push(WallRow {
                    uid: id,
                    style: x.style.clone(),
                    level: x.arch.level.clone(),
                    length,
                    width: x.width,
                    height,
                    gross_area: gross,
                    openings_area,
                    net_area: net,
                    volume: net.map(|n| n * x.width - pockets.unwrap_or(0.0)),
                    pockets_volume: pockets,
                    openings: openings.into_iter().map(|(u, _)| u).collect(),
                    rule,
                });
            }
            _ => {}
        }
    }
    out.transitions = crate::slab_quantities::transitions(&rooms, &passages);
    out.materials = crate::material_quantities::rows(doc.materials(), &wall_parts, &slab_parts);
    let summed = || out.slabs.iter().filter(|s| !s.excluded);
    let edged = |k: kentos_commands::SlabEdgeKind| summed().filter(|s| s.edge == Some(k)).map(|s| s.edge_length).sum::<f64>() + 0.0;
    let (slab_area, slab_volume) = (summed().map(|s| s.net_area).sum::<f64>() + 0.0, summed().map(|s| s.volume).sum::<f64>() + 0.0);
    let (railing_length, parapet_length) =
        (edged(kentos_commands::SlabEdgeKind::Railing), edged(kentos_commands::SlabEdgeKind::Parapet));
    out.totals = QuantityTotals {
        space_area: out.spaces.iter().filter(|s| !s.void).map(|s| s.area).sum::<f64>() + 0.0,
        void_area: out.spaces.iter().filter(|s| s.void).map(|s| s.area).sum::<f64>() + 0.0,
        wall_length: out.walls.iter().map(|w| w.length).sum::<f64>() + 0.0,
        wall_net_area: out.walls.iter().filter_map(|w| w.net_area).sum::<f64>() + 0.0,
        openings: u32::try_from(out.openings.len()).unwrap_or(u32::MAX),
        opening_area: out
            .openings
            .iter()
            .filter(|o| !o.form.is_recess())
            .filter_map(|o| o.area)
            .sum::<f64>() + 0.0,
        baseboard_length: out.spaces.iter().filter_map(|s| s.baseboard).sum::<f64>() + 0.0,
        slab_area,
        slab_volume,
        railing_length,
        parapet_length,
        ramp_area: out.ramps.iter().map(|r| r.area).sum::<f64>() + 0.0,
        ramp_railing_length: out.ramps.iter().map(|r| r.railing_length).sum::<f64>() + 0.0,
        stair_area: out.stairs.iter().map(|s| s.area).sum::<f64>() + 0.0,
        stair_railing_length: out.stairs.iter().map(|s| s.railing_length).sum::<f64>() + 0.0,
        stair_risers: out.stairs.iter().map(|s| s.risers).fold(0u32, u32::saturating_add),
    };
    Ok(out)
}

/// A sum that is +0 when empty: Rust's f64 `Sum` starts from -0 (+0 added makes it +0), so an empty column said “-0.0” in the API's JSON
/// (docs/adr/0776 §2).
fn total(values: impl Iterator<Item = f64>) -> f64 {
    values.sum::<f64>() + 0.0
}

/// A ramp measured (docs/adr/0723 §3): its lengths, landings, plan area and railings from its line, its slope as drawn.
pub(crate) fn ramp_row(uid: String, r: &kentos_commands::RampFields) -> kentos_commands::RampRow {
    let shape = kentos_formats::arch_elements::core_ramp(r);
    let kentos_geometry_core::entity::Shape::Ramp { pts, ramp } = &shape else {
        unreachable!("core_ramp makes a ramp");
    };
    let m = kentos_geometry_core::geom::arch::ramp_measure(pts, ramp);
    let base = r.base.unwrap_or(0.0);
    kentos_commands::RampRow {
        uid,
        level: r.arch.level.clone(),
        width: r.width,
        rise: r.rise,
        slope: (m.run > 0.0).then(|| r.rise.abs() / m.run * 100.0),
        run: m.run,
        length: m.length,
        landings: u32::try_from(m.landings).unwrap_or(u32::MAX),
        area: m.area,
        base,
        top: base + r.rise,
        base_offset: r.base_offset,
        railing_length: m.railing_length,
        rule: "alan: orta çizgi boyunca, dönüşlerde gönyeli şeridin plan alanı; eğimli uzunluk: orta çizgi eksi sahanlıklar; \
               eğim: kot farkı ÷ eğimli uzunluk; korkuluk: kenarlardan 5 cm içerideki çizgiler"
            .into(),
    }
}

/// A stair measured (docs/adr/0724 §5): its walking line's length, its landings, its band's plan area and its
/// railings from its line; a curved one's radius and inner tread.
pub(crate) fn stair_row(uid: String, s: &kentos_commands::StairFields) -> kentos_commands::StairRow {
    let shape = kentos_formats::arch_elements::core_stair(s);
    let kentos_geometry_core::entity::Shape::Stair { pts, stair } = &shape else {
        unreachable!("core_stair makes a stair");
    };
    let m = kentos_geometry_core::geom::arch::stair_measure(pts, stair);
    let rule = if m.radius.is_some() {
        "alan: iki eş merkezli yayın arası (yarıçap × genişlik × dönüş açısı); uzunluk: yürüme çizgisinin yayı; \
         korkuluk: kenarlardan 5 cm içerideki yaylar; iç basamak: basamak × (yarıçap − genişlik/2) ÷ yarıçap"
    } else {
        "alan: yürüme çizgisi boyunca, dönüşlerde gönyeli şeridin plan alanı; uzunluk: yürüme çizgisi, sahanlıklar \
         dahil; korkuluk: kenarlardan 5 cm içerideki çizgiler"
    };
    kentos_commands::StairRow {
        uid,
        level: s.arch.level.clone(),
        width: s.width,
        risers: s.risers,
        riser_height: s.riser_height,
        tread: s.tread,
        height: f64::from(s.risers) * s.riser_height,
        length: m.length,
        landings: u32::try_from(m.landings).unwrap_or(u32::MAX),
        area: m.area,
        railing_length: m.railing_length,
        radius: m.radius,
        inner_tread: m.inner_tread,
        height_from_level: s.height_from_level,
        rule: rule.into(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SpacesQuantities) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &SpacesQuantities,
) -> CommandResult<SpacesQuantified> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the Metraj window shows.
pub fn plan_in(doc: &Document, input: &SpacesQuantities) -> CommandResult<SpacesQuantified> {
    match check(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as the plan.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SpacesQuantities,
) -> CommandResult<SpacesQuantified> {
    plan_in(cx.doc, &input)
}

/// A pocket door's cavities in its wall, m³ (docs/adr/0787 §3): each pocket's length (along the wall's base line, as
/// the door's width) × its width × the door's height (head − sill), scaled on an arc wall to the wall's middle, where
/// the cavity runs (its length there over its length on the base line); none for another opening, one off its wall or
/// without a head.
fn pockets_volume(doc: &Document, x: &kentos_commands::OpeningFields) -> Option<f64> {
    use kentos_geometry_core::geom::{intersect::Edge, opening, wall};
    if !kentos_domain::opening_follow::is_pocket(x) || x.broken.is_some() {
        return None;
    }
    let height = x.head? - x.sill.unwrap_or(0.0);
    let line = doc.wall_line_of(&x.host)?;
    let (r0, r1) = kentos_domain::opening_follow::reach(x);
    let edge = opening::edge_of(&line.path, r0, r1)?;
    let middle = wall::left_face(&line.spec) - line.spec.width / 2.0;
    let scale = match line.path.edges[edge] {
        Edge::Arc { r, sweep, .. } if r > 0.0 => (r - middle * sweep.signum()) / r,
        _ => 1.0,
    };
    let area: f64 = kentos_commands::opening_sizes(x).pockets.iter().map(|p| p.length * p.width).sum();
    (area > 0.0 && height > 0.0).then_some(area * scale * height)
}
