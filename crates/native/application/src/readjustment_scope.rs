//! `cad.readjustment.scope` v1 (plan/map-cad.md MAP-11, docs/adr/2060): Düzenleme kapsamı, read only. The parcels
//! inside a readjustment's boundary with the separation sheet's numbers (Arazi ve Arsa Düzenlemeleri Hakkında
//! Yönetmelik Articles 13 and 31): the parts inside and outside by the shared core's exact area algebra
//! (`intersect_area_sets`, `subtract_areas`: arcs stay arcs), a piece counted when its mean width 2A/P passes the
//! tolerance (as the topology rules, docs/adr/0202 §2); the registered area as written beside the computed one
//! (`entity_area`), the difference and the tolerance of `cad.parcels.check` (the 2022 regulation's Article 8(2)
//! table); the basis: the registered area of a whole parcel within the tolerance, the computed area of a partial
//! parcel's part inside rounded once by the given policy from the area's own shortest decimal form, the error left to
//! the part outside. Where the rule needs a person, no basis. A partial parcel's parts carry their rings, so the
//! separation sheet's boundaries and coordinates come from here too (docs/adr/2280). The shared cases are
//! `fixtures/commands/v1/cad.readjustment.scope.json`.

use kentos_commands::{
    CommandResult, Entity, PARCEL_AREA_RULE, PARCEL_LAYER, ReadjustmentScope, ReadjustmentScopeParcel,
    ReadjustmentScoped, ScopePart, ScopeRing,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::entity::entity_area;
use kentos_geometry_core::geom::arrangement::{Area, Ring, edge_len};
use kentos_geometry_core::geom::bulge::bulge_ring_area;
use kentos_geometry_core::predicates::orientation;
use kentos_geometry_core::vec2::Vec2;
use kentos_geometry_core::geom::region::{intersect_area_sets, net_area, subtract_areas};
use kentos_geometry_core::numeric::{RoundingMode, parse_decimal, round_decimal, subtract_exact, sum_exact};
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::ops::geoprocess::boundary_edges;

use crate::checks::{self, Stop, error};
use crate::geometry::shape;
use crate::parcels_check::{RECORDED_FIELD, recorded_value, tolerance as limit_of};
use crate::{ExecutionContext, codes};

/// The most places a computed basis may have (the rule package's limit, docs/adr/1460).
const MOST_PLACES: u32 = 6;
/// The narrowest piece counted when none is given (metres; the topology rules' first tolerance).
pub const FIRST_TOLERANCE: f64 = 0.01;
const LEAST_TOLERANCE: f64 = 1e-6;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn count(n: usize) -> u32 {
    n.min(u32::MAX as usize) as u32
}

/// Places after the point in plain decimal text.
fn places(text: &str) -> u32 {
    text.split_once('.').map_or(0, |(_, f)| f.len() as u32)
}

/// Exact decimal text written with `scale` places (it has at most that many: padding only).
fn padded(text: &str, scale: u32) -> Option<String> {
    round_decimal(text, scale, RoundingMode::HalfEven).ok()
}

/// A piece's mean width 2A/P (its holes' edges counted in P).
fn width(a: &Area) -> f64 {
    let p: f64 = boundary_edges(std::slice::from_ref(a)).iter().map(edge_len).sum();
    if p > 0.0 { 2.0 * net_area(a) / p } else { 0.0 }
}

/// The pieces with an area, as the overlay gives them.
fn pieces(list: Vec<Area>) -> Vec<Area> {
    list.into_iter().filter(|a| net_area(a) > 0.0).collect()
}

/// A ring as the separation sheet writes it (docs/adr/2280): counter-clockwise when `outer`, else clockwise; a corner
/// where two straight edges go on in one direction left out (the exact predicate); from its westmost, then southmost
/// corner.
fn sheet_ring(r: &Ring, outer: bool) -> ScopeRing {
    let n = r.pts.len();
    let mut pts: Vec<Vec2> = r.pts.clone();
    let mut bulges: Vec<f64> = (0..n).map(|i| r.bulges.as_ref().and_then(|b| b.get(i).copied()).unwrap_or(0.0)).collect();
    if n >= 3 && (bulge_ring_area(&pts, Some(&bulges)) > 0.0) != outer {
        // Reversed: the edge after corner i is the old edge before it, bowing the other way.
        pts = (0..n).map(|i| r.pts[n - 1 - i]).collect();
        bulges = (0..n).map(|i| -bulges[(2 * n - 2 - i) % n]).collect();
    }
    while pts.len() > 3 {
        let m = pts.len();
        let straight_on = |i: usize| {
            let (p, q) = ((i + m - 1) % m, (i + 1) % m);
            let (a, b, c) = (pts[p], pts[i], pts[q]);
            bulges[p] == 0.0
                && bulges[i] == 0.0
                && orientation(a, b, c) == 0
                && (b.x - a.x) * (c.x - b.x) + (b.y - a.y) * (c.y - b.y) > 0.0
        };
        let Some(k) = (0..m).find(|&i| straight_on(i)) else { break };
        pts.remove(k);
        bulges.remove(k);
    }
    let start = (0..pts.len()).min_by(|&a, &b| pts[a].x.total_cmp(&pts[b].x).then(pts[a].y.total_cmp(&pts[b].y)));
    let s = start.unwrap_or(0);
    let m = pts.len();
    let order: Vec<usize> = (0..m).map(|i| (s + i) % m).collect();
    ScopeRing {
        pts: order.iter().map(|&i| [pts[i].x, pts[i].y]).collect(),
        bulges: bulges.iter().any(|b| *b != 0.0).then(|| order.iter().map(|&i| bulges[i]).collect()),
    }
}

/// A piece as a part of the separation sheet: its side, area and rings (the holes in the order of their first corner).
fn sheet_part(a: &Area, side: &str) -> ScopePart {
    let mut holes: Vec<ScopeRing> = a.holes.iter().map(|h| sheet_ring(h, false)).collect();
    holes.sort_by(|p, q| {
        let (a, b) = (p.pts.first().copied().unwrap_or_default(), q.pts.first().copied().unwrap_or_default());
        a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]))
    });
    let mut rings = vec![sheet_ring(&a.outer, true)];
    rings.extend(holes);
    ScopePart { side: side.to_owned(), area: net_area(a), rings }
}

/// The pieces' areas added from +0 (an empty `sum` of floats is −0).
fn total(list: &[Area]) -> f64 {
    list.iter().map(net_area).fold(0.0, |s, a| s + a)
}

/// A computed area as decimal text at `scale` places by `mode`: the value's shortest decimal form (Rust writes floats
/// without an exponent), rounded once.
fn computed(v: f64, scale: u32, mode: RoundingMode) -> Option<String> {
    round_decimal(&format!("{v}"), scale, mode).ok()
}

fn object<'d>(doc: &'d Document, uid: &str, path: &str) -> Result<&'d Entity, Stop> {
    if !checks::is_uid_text(uid) {
        return Err(fail(codes::INVALID_UID, format!("“{uid}” geçerli bir nesne kimliği değil."), path));
    }
    Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .ok_or_else(|| fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), path))
}

/// The basis and what the part outside keeps, or why there is none: the status first.
fn basis_of(
    kind: &str,
    counted_in: usize,
    slivers: usize,
    text: Option<&str>,
    within: Option<bool>,
    inside: f64,
    scale: u32,
    mode: RoundingMode,
) -> (&'static str, Option<String>, Option<String>) {
    if counted_in == 0 && slivers == 0 {
        return ("outside", None, None);
    }
    if slivers > 0 {
        return ("sliver", None, None);
    }
    let Some(text) = text else { return ("noRecord", None, None) };
    // The record as written, its comma a point: the decimal the distribution takes (never the float).
    let written = text.replace(',', ".");
    if recorded_value(text).is_none() || parse_decimal(&written).is_err() {
        return ("invalidRecord", None, None);
    }
    if within == Some(false) {
        return ("needsCorrection", None, None);
    }
    if kind == "whole" {
        return if places(&written) > scale { ("finerThanPolicy", None, None) } else { ("registered", Some(written), None) };
    }
    let Some(basis) = computed(inside, scale, mode) else { return ("invalidRecord", None, None) };
    let rest = subtract_exact(&written, &basis).ok().and_then(|r| padded(&r, places(&written).max(scale)));
    match rest {
        Some(r) if r.starts_with('-') => ("outsideTooSmall", None, Some(r)),
        Some(r) => ("computed", Some(basis), Some(r)),
        None => ("invalidRecord", None, None),
    }
}

fn run(doc: &Document, input: &ReadjustmentScope) -> Result<ReadjustmentScoped, Stop> {
    if input.scale > MOST_PLACES {
        return Err(fail(codes::INVALID_INPUT, format!("Basamak {} en çok {MOST_PLACES} olabilir.", input.scale), "scale"));
    }
    let Some(mode) = RoundingMode::parse(input.rounding.trim()) else {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("“{}” bir yuvarlama yönü değil (half_even, half_away_from_zero …).", input.rounding),
            "rounding",
        ));
    };
    let tolerance = input.tolerance.unwrap_or(FIRST_TOLERANCE);
    if !tolerance.is_finite() || tolerance < LEAST_TOLERANCE {
        return Err(fail(codes::INVALID_INPUT, format!("Tolerans sonlu ve en az {LEAST_TOLERANCE} m olmalı (yazılan {tolerance})."), "tolerance"));
    }
    let field = input.field.as_deref().map(str::trim).filter(|f| !f.is_empty()).unwrap_or(RECORDED_FIELD).to_owned();
    let b = input.boundary.trim();
    let boundary = object(doc, b, "boundary")?;
    let edge = areas_of_entity(&shape(boundary));
    if !matches!(boundary, Entity::Polygon(_)) || edge.is_empty() {
        return Err(fail(codes::NOT_AN_AREA, format!("“{b}” bir alan değil; düzenleme sınırı kapalı bir alan olmalı."), "boundary"));
    }
    let boundary_id = boundary.base().id;
    let (given, chosen): (bool, Vec<&Entity>) = match &input.uids {
        Some(uids) => {
            let mut out = Vec::new();
            for (i, uid) in uids.iter().enumerate() {
                let path = format!("uids[{i}]");
                let e = object(doc, uid, &path)?;
                if !matches!(e, Entity::Polygon(_)) {
                    return Err(fail(
                        codes::NOT_A_PARCEL,
                        format!("“{uid}” bir alan değil; düzenleme kapsamı yalnız alanları alır."),
                        &path,
                    ));
                }
                out.push(e);
            }
            (true, out)
        }
        None => (
            false,
            doc.by_layer(PARCEL_LAYER).filter(|e| matches!(e, Entity::Polygon(_)) && e.base().id != boundary_id).collect(),
        ),
    };
    let mut parcels = Vec::new();
    for e in chosen {
        let s = shape(e);
        let areas = areas_of_entity(&s);
        let inside_pieces = if areas.is_empty() { Vec::new() } else { pieces(intersect_area_sets(&[areas.clone(), edge.clone()])) };
        if !given && inside_pieces.is_empty() {
            continue;
        }
        let outside_pieces = pieces(subtract_areas(&areas, &edge));
        let counted = |list: &[Area]| list.iter().filter(|a| width(a) > tolerance).count();
        let (counted_in, counted_out) = (counted(&inside_pieces), counted(&outside_pieces));
        let slivers = inside_pieces.len() + outside_pieces.len() - counted_in - counted_out;
        let kind = match (counted_in, counted_out) {
            (0, _) => "outside",
            (_, 0) => "whole",
            _ => "partial",
        };
        let (inside, outside) = (total(&inside_pieces), total(&outside_pieces));
        let calculated = entity_area(&s).unwrap_or(0.0);
        let text = e.base().attrs.get(&field).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        let difference = text.as_deref().and_then(recorded_value).map(|r| calculated - r);
        let limit = limit_of(calculated);
        let within = difference.map(|d| d.abs() <= limit);
        let (status, basis, outside_recorded) = basis_of(kind, counted_in, slivers, text.as_deref(), within, inside, input.scale, mode);
        // The separation sheet's parts (docs/adr/2280): a partial parcel's counted pieces, inside then outside.
        let mut parts = Vec::new();
        if kind == "partial" {
            for (list, side) in [(&inside_pieces, "inside"), (&outside_pieces, "outside")] {
                let mut these: Vec<ScopePart> =
                    list.iter().filter(|a| width(a) > tolerance).map(|a| sheet_part(a, side)).collect();
                these.sort_by(|p, q| {
                    let first = |s: &ScopePart| s.rings.first().and_then(|r| r.pts.first().copied()).unwrap_or_default();
                    let (a, b) = (first(p), first(q));
                    a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]))
                });
                parts.extend(these);
            }
        }
        let attr = |k: &str| e.base().attrs.get(k).map(|v| v.trim().to_owned()).unwrap_or_default();
        let (ada, parsel) = (attr("Ada"), attr("Parsel"));
        let parcel = match (ada.is_empty(), parsel.is_empty()) {
            (false, false) => format!("{ada}/{parsel}"),
            (true, _) => parsel.clone(),
            (false, true) => ada.clone(),
        };
        parcels.push(ReadjustmentScopeParcel {
            uid: doc.uid(kentos_domain::Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default(),
            parcel,
            il: attr("İl"),
            ilce: attr("İlçe"),
            mahalle: attr("Mahalle"),
            no_identity: ada.is_empty() || parsel.is_empty(),
            ada,
            parsel,
            kind: kind.to_owned(),
            recorded_text: text,
            calculated,
            difference,
            limit,
            within,
            inside,
            outside,
            pieces: count(counted_out),
            slivers: count(slivers),
            basis,
            outside_recorded,
            status: status.to_owned(),
            duplicate: false,
            parts,
        });
    }
    // The records name a parcel by Ada/Parsel: two in the answer with one name cannot both take a record.
    let names: Vec<String> = parcels.iter().filter(|p| !p.no_identity).map(|p| p.parcel.clone()).collect();
    for p in parcels.iter_mut().filter(|p| !p.no_identity) {
        p.duplicate = names.iter().filter(|n| **n == p.parcel).count() > 1;
    }
    let bases: Vec<&str> = parcels.iter().filter_map(|p| p.basis.as_deref()).collect();
    let most = bases.iter().map(|t| places(t)).max().unwrap_or(0);
    let basis = sum_exact(&bases).ok().and_then(|t| padded(&t, most)).unwrap_or_else(|| "0".to_owned());
    let boundary_area = entity_area(&shape(boundary)).unwrap_or(0.0);
    let inside = parcels.iter().fold(0.0, |s, p| s + p.inside);
    let by_kind = |k: &str| count(parcels.iter().filter(|p| p.kind == k).count());
    Ok(ReadjustmentScoped {
        rule: PARCEL_AREA_RULE.to_owned(),
        field,
        scale: input.scale,
        rounding: input.rounding.trim().to_owned(),
        tolerance,
        boundary_area,
        inside,
        uncovered: boundary_area - inside,
        basis,
        whole: by_kind("whole"),
        partial: by_kind("partial"),
        ready: count(bases.len()),
        review: count(parcels.iter().filter(|p| !matches!(p.status.as_str(), "registered" | "computed" | "outside")).count()),
        parcels,
    })
}

/// Reads only: the same checks as the answer.
pub fn validate(cx: &ExecutionContext<'_>, input: &ReadjustmentScope) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &ReadjustmentScope) -> CommandResult<ReadjustmentScoped> {
    match run(cx.doc, input) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ReadjustmentScope) -> CommandResult<ReadjustmentScoped> {
    plan(cx, &input)
}

/// The scope for a window, or why it does not read.
pub fn preview(doc: &Document, input: &ReadjustmentScope) -> Result<ReadjustmentScoped, String> {
    run(doc, input).map_err(crate::road_corridors::words)
}
