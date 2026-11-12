//! `cad.readjustment.sheets` v1 (plan/map-cad.md MAP-11, docs/adr/2680): Ayırma çapı paftaları. One sheet per parcel
//! partly inside the boundary (Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik m. 13: a separation sheet per parcel,
//! every piece outside on it) from KentOS's separation sheet (`sys:ayirma-capi`) or the given template: the numbers and
//! parts are `cad.readjustment.scope`'s, the parts' corners its numbered points. The points are written on a layer of
//! the parcel's own under the group “Ayırma çapı noktaları”, labelled with their numbers; the sheet's map (fitted to the
//! parcel at a standard scale, as `cad.sheets.fit` fits) freezes the other parcels' point layers, its coordinate list
//! reads the parcel's, its tables take the areas and the parts written with the policy's places and rounding. The
//! points, the layers and the sheets are one undo step “Ayırma çapı paftaları”. The shared cases are
//! `fixtures/commands/v1/cad.readjustment.sheets.json`.
//!
//! docs/adr/3480: an outside part given a parcel number (m. 13/3, docs/adr/3080) is written too, on a layer of the
//! parcel's numbered pieces beside its points (“‹Ada/Parsel› dış parçalar”): the part as an area with its parcel, name
//! and number as attributes, and a point inside it labelled with the number (`label_at`); the sheet's map shows both of
//! the parcel's layers, its coordinate list the points only. docs/adr/4280: with the user's `numberHeight` the number is
//! a text fitted inside its part instead of the point (`fitted`). docs/adr/5080: a number outside on a leader
//! (docs/adr/4680) is kept clear of what the sheet's map shows (`separation_number_clear`).

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::{
    CommandResult, EntitiesCreate, EntityGeometry, LayerNode, LayerNodeType, NewObject, ReadjustmentScopeParcel,
    ReadjustmentSheets, ReadjustmentSheetsMade, ScopePart, ScopeRing, SeparationSheet, SheetPicture, SheetTemplateSource,
    SheetsEdit, Vec2,
};
use kentos_domain::{Document, NewLayer};
use kentos_geometry_core::entity::TextPlace;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::bulge::{bulge_ring_area, reverse_bulge_path};
use kentos_geometry_core::geom::region::{intersect_area_sets, net_area};
use kentos_geometry_core::numeric::{RoundingMode, parse_decimal, subtract_exact};
use kentos_sheet::model::VarValue;
use kentos_sheet::ops::{Op, SetItemProps};
use kentos_sheet::template::{InstanceOptions, Template, VariableValue};
use kentos_sheet::{ItemKind, MapView, create, expr, fit, template};

use crate::checks::{self, Stop, error};
use crate::{ExecutionContext, codes, readjustment_scope, separation_number_clear, sheets_edit, sheets_fit};

/// The layer of a parcel's numbered pieces outside is named “‹Ada/Parsel› dış parçalar” (docs/adr/3480).
const PIECES: &str = "dış parçalar";
/// The template when none is given: KentOS's separation sheet.
pub const DEFAULT_TEMPLATE: &str = "sys:ayirma-capi";
/// The undo step's name.
pub const STEP: &str = "Ayırma çapı paftaları";
/// The group the parcels' point layers are in.
pub const GROUP: &str = "Ayırma çapı noktaları";
/// A sheet's name before the parcel's: “Ayırma çapı 101/10”.
pub const SHEET_WORD: &str = "Ayırma çapı";
/// The template's items the sheets fill, by their ids in the template.
const MAP: &str = "harita";
const COORDINATES: &str = "koordinatlar";
const AREAS: &str = "yuzolcumleri";
const PARTS: &str = "kisimlar";
/// The ground around the parcel on each side, a percent of its size (`cad.sheets.fit`'s default).
const MARGIN: u8 = 5;
/// The template's questions each parcel answers from its own attributes.
const OWN: [&str; 5] = ["il", "ilce", "mahalle", "ada", "parsel"];

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A stop of the scope's, its place under `scope.`.
fn under(stop: Stop) -> Stop {
    match stop {
        Stop::Failed(mut e) => {
            e.path = Some(e.path.take().map_or_else(|| "scope".to_owned(), |p| format!("scope.{p}")));
            Stop::Failed(e)
        }
        other => other,
    }
}

/// The parts' names on the sheet: Giren and Girmeyen, numbered on a side that has more than one (each piece outside
/// takes a parcel number of its own, Article 13(3)).
pub fn part_names(parts: &[ScopePart]) -> Vec<String> {
    let on = |side: &str| parts.iter().filter(|p| p.side == side).count();
    let (inside, outside) = (on("inside"), on("outside"));
    let (mut i, mut o) = (0, 0);
    parts
        .iter()
        .map(|p| {
            let (word, n, of) = if p.side == "inside" {
                i += 1;
                ("Giren", i, inside)
            } else {
                o += 1;
                ("Girmeyen", o, outside)
            };
            if of > 1 { format!("{word} {n}") } else { word.to_owned() }
        })
        .collect()
}

/// Where an outside part's number stands (docs/adr/3480): on the horizontal line at the half height of the outer
/// ring's corners, the middle of the widest stretch inside the part (its rings' crossings paired in order; a straight
/// edge crosses when its ends lie on either side, the upper end counted; an arc where its circle meets the line within
/// its sweep, its end not counted); the first of equals. Inside the part whatever its shape.
pub fn label_at(part: &ScopePart) -> Option<[f64; 2]> {
    let outer = part.rings.first()?;
    let (lo, hi) = outer.pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(p[1]), hi.max(p[1])));
    let y = (lo + hi) / 2.0;
    let xs = crossings_at(part, y);
    let best = xs.chunks_exact(2).max_by(|p, q| (p[1] - p[0]).total_cmp(&(q[1] - q[0])).then(std::cmp::Ordering::Greater))?;
    Some([(best[0] + best[1]) / 2.0, y])
}

/// Where the horizontal line at `y` crosses a part's rings, sorted (a straight edge when its ends lie on either side,
/// the upper end counted; an arc where its circle meets the line within its sweep, its end not counted).
pub(crate) fn crossings_at(part: &ScopePart, y: f64) -> Vec<f64> {
    let mut xs: Vec<f64> = Vec::new();
    for r in &part.rings {
        let n = r.pts.len();
        for i in 0..n {
            let (a, b) = (r.pts[i], r.pts[(i + 1) % n]);
            let bulge = r.bulges.as_ref().and_then(|v| v.get(i).copied()).unwrap_or(0.0);
            if bulge == 0.0 {
                if (a[1] > y) != (b[1] > y) {
                    xs.push(a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
                }
                continue;
            }
            // The arc's circle: its centre on the chord's normal, the sweep 4·atan(bulge) from `a`.
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let k = (1.0 - bulge * bulge) / (4.0 * bulge);
            let (cx, cy) = ((a[0] + b[0]) / 2.0 - dy * k, (a[1] + b[1]) / 2.0 + dx * k);
            let r_ = dx.hypot(dy) * (1.0 + bulge * bulge) / (4.0 * bulge.abs());
            let h = r_ * r_ - (y - cy) * (y - cy);
            if h < 0.0 {
                continue;
            }
            let sweep = 4.0 * bulge.atan();
            let a0 = (a[1] - cy).atan2(a[0] - cx);
            for x in [cx - h.sqrt(), cx + h.sqrt()] {
                let d = (((y - cy).atan2(x - cx) - a0) * sweep.signum()).rem_euclid(std::f64::consts::TAU);
                if d < sweep.abs() {
                    xs.push(x);
                }
            }
        }
    }
    xs.sort_by(f64::total_cmp);
    xs
}

/// docs/adr/4280: the shares of the asked height a number is tried at, the largest first.
const SHARES: [f64; 10] = [1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1];

fn point(p: [f64; 2]) -> kentos_geometry_core::Vec2 {
    kentos_geometry_core::Vec2::new(p[0], p[1])
}

/// A part's rings as the overlay takes them: its outer ring counter-clockwise, its holes clockwise.
fn part_areas(part: &ScopePart) -> Vec<Area> {
    let ring = |r: &ScopeRing, ccw: bool| {
        let pts: Vec<_> = r.pts.iter().copied().map(point).collect();
        if (bulge_ring_area(&pts, r.bulges.as_deref()) > 0.0) == ccw {
            Ring { pts, bulges: r.bulges.clone() }
        } else {
            let back = reverse_bulge_path(&pts, r.bulges.as_deref(), true);
            Ring { pts: back.pts, bulges: back.bulges }
        }
    };
    let Some(outer) = part.rings.first() else { return Vec::new() };
    vec![Area { outer: ring(outer, true), holes: part.rings[1..].iter().map(|h| ring(h, false)).collect() }]
}

/// The number's text: centred at `at`, turned `rotation` degrees, `height` metres tall.
fn number_text(number: &str, at: [f64; 2], rotation: f64, height: f64) -> kentos_commands::Entity {
    kentos_commands::Entity::Text(kentos_commands::TextEntity {
        base: kentos_commands::EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: String::new(),
            color: None,
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p: Vec2 { x: at[0], y: at[1] },
        text: number.to_owned(),
        height,
        rotation,
        align: Some(kentos_commands::TextAlign::MiddleCenter),
        width_factor: None,
        mask: false,
        label_of: None,
        label_scale: None,
        paragraph: Default::default(),
        face: Default::default(),
        path: None,
        annotative: None,
    })
}

/// Whether a text's box, as the drawing measures it (docs/adr/0145: its typeface's advances, 0.23 of its height under
/// the baseline and 1.15 over it), lies inside the part: the part's overlap with the box is the box (to 1e-9 of it).
fn text_inside(part: &[Area], text: &kentos_commands::Entity, font: kentos_geometry_core::text::Font) -> bool {
    let shape = crate::geometry::shape(text);
    let Some(ring) = TextPlace::of(&shape).map(|t| t.outline(font)) else { return false };
    let block = Area { outer: Ring { pts: ring, bulges: None }, holes: Vec::new() };
    let whole = net_area(&block);
    let shared: f64 = intersect_area_sets(&[part.to_vec(), vec![block]]).iter().map(net_area).sum();
    whole > 0.0 && shared >= whole * (1.0 - 1e-9)
}

/// A part turned `angle` radians about `about`.
fn turned(part: &ScopePart, about: [f64; 2], angle: f64) -> ScopePart {
    let (s, c) = angle.sin_cos();
    let mut out = part.clone();
    for r in &mut out.rings {
        for p in &mut r.pts {
            let (x, y) = (p[0] - about[0], p[1] - about[1]);
            *p = [about[0] + x * c - y * s, about[1] + x * s + y * c];
        }
    }
    out
}

/// docs/adr/4280: the turns a number is tried at, degrees: horizontal, then along the outer ring's longest straight edge
/// (the first of equals), read from the left (in (−90°, 90°]), when that is not horizontal.
fn turns(part: &ScopePart) -> Vec<f64> {
    let mut out = vec![0.0];
    let Some(outer) = part.rings.first() else { return out };
    let n = outer.pts.len();
    let mut longest: Option<(f64, [f64; 2], [f64; 2])> = None;
    for i in 0..n {
        let straight = outer.bulges.as_ref().and_then(|b| b.get(i).copied()).unwrap_or(0.0) == 0.0;
        let (a, b) = (outer.pts[i], outer.pts[(i + 1) % n]);
        let length = (b[0] - a[0]).hypot(b[1] - a[1]);
        if straight && longest.is_none_or(|(l, _, _)| length > l) {
            longest = Some((length, a, b));
        }
    }
    if let Some((_, a, b)) = longest {
        let mut angle = (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees();
        if angle > 90.0 {
            angle -= 180.0;
        } else if angle <= -90.0 {
            angle += 180.0;
        }
        if angle != 0.0 {
            out.push(angle);
        }
    }
    out
}

/// A label's place, turn (degrees), height, share of the height asked, whether it fits and (docs/adr/4680) its leader's
/// tip.
type Fitted = ([f64; 2], f64, f64, f64, bool, Option<[f64; 2]>);

/// docs/adr/4280: where and how big a number's text stands inside its part: at each turn (`turns`), its place by
/// `label_at` in the part turned so (the widest stretch of the middle line, turned back); the largest share of `height`
/// (`SHARES`) whose box lies inside the part, the first turn among equals. None fits: horizontal at the least share.
///
/// Docs/adr/4680, with `leader`: only the full height is tried; when it does not fit, the number goes outside at the full
/// height, horizontal, on a leader from the part's label place running east to one text height past the part's last
/// boundary crossing on that line (where the landing starts).
fn fitted(part: &ScopePart, number: &str, height: f64, font: kentos_geometry_core::text::Font, leader: bool) -> Option<Fitted> {
    let areas = part_areas(part);
    let origin = *part.rings.first()?.pts.first()?;
    let mut best: Option<([f64; 2], f64, f64)> = None;
    for turn in turns(part) {
        let at = if turn == 0.0 {
            label_at(part)?
        } else {
            let r = turn.to_radians();
            let local = label_at(&turned(part, origin, -r))?;
            let (s, c) = r.sin_cos();
            let (x, y) = (local[0] - origin[0], local[1] - origin[1]);
            [origin[0] + x * c - y * s, origin[1] + x * s + y * c]
        };
        for share in SHARES.iter().copied().take(if leader { 1 } else { SHARES.len() }) {
            if best.is_some_and(|(_, _, s)| s >= share) {
                break;
            }
            if text_inside(&areas, &number_text(number, at, turn, height * share), font) {
                best = Some((at, turn, share));
                break;
            }
        }
    }
    let least = SHARES[SHARES.len() - 1];
    Some(match best {
        Some((at, turn, share)) => (at, turn, height * share, share, true, None),
        None if leader => {
            let tip = label_at(part)?;
            let east = crossings_at(part, tip[1]).last().copied()?;
            ([east + height, tip[1]], 0.0, height, 1.0, false, Some(tip))
        }
        None => (label_at(part)?, 0.0, height * least, least, false, None),
    })
}

/// The numbered outside parts' labels (docs/adr/3480): each outside part given a number, its name and place; with the
/// user's height (paper mm) at the sheet's scale, the number's fitted text (docs/adr/4280). Docs/adr/5080: with a
/// leader, the numbers that fit inside first, then each one going outside in the parts' order, placed clear of the
/// drawing the sheet's map shows (`drawing`), the texts inside and the leaders placed before it.
fn labels_of(
    p: &ReadjustmentScopeParcel,
    given: Option<&[String]>,
    height: Option<f64>,
    leader: bool,
    scale: u32,
    font: kentos_geometry_core::text::Font,
    drawing: Option<&dyn Fn(&[&ScopePart], f64, f64) -> separation_number_clear::Obstacles>,
) -> Vec<kentos_commands::SeparationLabel> {
    let Some(given) = given else { return Vec::new() };
    let names = part_names(&p.parts);
    let mut parts = Vec::new();
    let mut labels: Vec<kentos_commands::SeparationLabel> = p
        .parts
        .iter()
        .zip(&names)
        .filter(|(q, _)| q.side == "outside")
        .zip(given)
        .filter_map(|((q, name), number)| {
            let number = number.trim().to_owned();
            let mut label = kentos_commands::SeparationLabel {
                part: name.clone(),
                number: number.clone(),
                at: label_at(q)?,
                height: None,
                rotation: None,
                share: None,
                fits: None,
                leader: None,
                clear: None,
                clashes: Vec::new(),
            };
            if let Some(mm) = height {
                let (at, turn, h, share, fits, tip) = fitted(q, &number, mm * f64::from(scale) / 1000.0, font, leader)?;
                label.at = at;
                label.height = Some(h);
                label.rotation = Some(turn);
                label.share = Some(share);
                label.fits = Some(fits);
                label.leader = tip;
            }
            parts.push(q);
            Some(label)
        })
        .collect();
    let (Some(drawing), Some(mm)) = (drawing, height) else { return labels };
    let outside: Vec<usize> = (0..labels.len()).filter(|&i| labels[i].leader.is_some()).collect();
    if outside.is_empty() {
        return labels;
    }
    let h = mm * f64::from(scale) / 1000.0;
    let widest = outside.iter().map(|&i| note_width(&labels[i].number, h, font)).fold(0.0, f64::max);
    let mut obstacles = drawing(&outside.iter().map(|&i| parts[i]).collect::<Vec<_>>(), h, widest);
    for l in labels.iter().filter(|l| l.leader.is_none()) {
        if let (Some(h), Some(turn)) = (l.height, l.rotation) {
            let text = number_text(&l.number, l.at, turn, h);
            if let Some(t) = TextPlace::of(&crate::geometry::shape(&text)) {
                obstacles.add_box(t.outline(font), &l.part);
            }
        }
    }
    for i in outside {
        let Some(tip) = labels[i].leader else { continue };
        let Some(placed) = separation_number_clear::place(parts[i], tip, &labels[i].number, h, font, &obstacles) else { continue };
        let l = &mut labels[i];
        l.at = placed.bend;
        l.clear = Some(placed.clashes.is_empty());
        l.clashes = placed.clashes.clone();
        obstacles.add_leader(&placed, &l.part);
    }
    labels
}

/// A one-line text's width, `height` tall, as the drawing measures it.
fn note_width(number: &str, height: f64, font: kentos_geometry_core::text::Font) -> f64 {
    let ring = TextPlace::line(kentos_geometry_core::Vec2::new(0.0, 0.0), number, height, 0.0, None, None).outline(font);
    ring.iter().map(|q| q.x).fold(f64::NEG_INFINITY, f64::max) - ring.iter().map(|q| q.x).fold(f64::INFINITY, f64::min)
}

/// The numbered outside parts as objects (docs/adr/3480): each part as an area and a point at its label's place, both
/// with the parcel, the part's name and its number as attributes; the point labelled with the number.
fn part_objects(p: &ReadjustmentScopeParcel, labels: &[kentos_commands::SeparationLabel]) -> Vec<NewObject> {
    let names = part_names(&p.parts);
    let mut out = Vec::new();
    for l in labels {
        let Some(q) = p.parts.iter().zip(&names).find(|(_, n)| **n == l.part).map(|(q, _)| q) else { continue };
        let attrs = BTreeMap::from([
            ("Parsel".to_owned(), p.parcel.clone()),
            ("Kısım".to_owned(), l.part.clone()),
            ("ParselNo".to_owned(), l.number.clone()),
        ]);
        let ring = |r: &ScopeRing| kentos_commands::RingGeometry {
            pts: r.pts.iter().map(|c| Vec2 { x: c[0], y: c[1] }).collect(),
            bulges: r.bulges.clone(),
            zs: None,
        };
        let Some(outer) = q.rings.first() else { continue };
        let object = |geometry: EntityGeometry, label: Option<String>| NewObject {
            geometry,
            color: None,
            line_weight: None,
            line_type: None,
            line_pattern: None,
            attrs: Some(attrs.clone()),
            label,
            symbol: None,
            label_of: None,
            label_scale: None,
            transparency: None,
            order: None,
        };
        let holes: Vec<_> = q.rings[1..].iter().map(ring).collect();
        out.push(object(
            EntityGeometry::Polygon {
                pts: ring(outer).pts,
                bulges: outer.bulges.clone(),
                holes: (!holes.is_empty()).then_some(holes),
                zs: None,
                parts: None,
            },
            None,
        ));
        // docs/adr/4680: a number outside on a leader, from the part's label place to where its landing starts.
        if let (Some(tip), Some(height)) = (l.leader, l.height) {
            out.push(object(
                EntityGeometry::Leader {
                    pts: vec![Vec2 { x: tip[0], y: tip[1] }, Vec2 { x: l.at[0], y: l.at[1] }],
                    text: Some(l.number.clone()),
                    height,
                    rotation: 0.0,
                    arrow: None,
                    arrow_size: None,
                    mask: false,
                    annotative: None,
                    arms: None,
                },
                None,
            ));
            continue;
        }
        // docs/adr/4280: with a height, the number as its fitted text; else a point labelled with it (docs/adr/3480).
        match (l.height, l.rotation) {
            (Some(height), Some(rotation)) => out.push(object(
                EntityGeometry::Text {
                    p: Vec2 { x: l.at[0], y: l.at[1] },
                    text: l.number.clone(),
                    height,
                    rotation,
                    align: Some(kentos_commands::TextAlign::MiddleCenter),
                    width_factor: None,
                    mask: false,
                    box_width: None,
                    line_spacing: None,
                    runs: Vec::new(),
                    face: Default::default(),
                    path: None,
                    layout: None,
                    annotative: None,
                },
                None,
            )),
            _ => out.push(object(
                EntityGeometry::Point { p: Vec2 { x: l.at[0], y: l.at[1] }, z: None, parts: None },
                Some(l.number.clone()),
            )),
        }
    }
    out
}

/// A ring as point numbers, closed by its first: “1-2-3-4-1”.
fn ring_text(r: &ScopeRing) -> String {
    let mut n: Vec<String> = r.numbers.iter().map(u32::to_string).collect();
    if let Some(first) = n.first().cloned() {
        n.push(first);
    }
    n.join("-")
}

/// A part's boundary as point numbers: its outer ring, then each hole's (“1-2-3-4-1 (delik 5-6-7-8-5)”).
pub fn boundary_text(part: &ScopePart) -> String {
    let mut text = part.rings.first().map(ring_text).unwrap_or_default();
    for hole in part.rings.iter().skip(1) {
        text.push_str(&format!(" (delik {})", ring_text(hole)));
    }
    text
}

/// The table of areas (Article 13(2)'s items, and what the part outside keeps by Article 31(2)), square metres: the
/// record as written (its comma a point when it reads as a decimal), the computed values at the policy's places by its
/// rounding, the difference of the written values exactly (computed − registered, with the record's places or the
/// policy's, the more), the scope's `outsideRecorded` (registered − the part inside); “—” for what is not there.
pub fn area_rows(p: &ReadjustmentScopeParcel, scale: u32, mode: RoundingMode) -> Vec<Vec<String>> {
    let none = || "—".to_owned();
    let w = |v: f64| readjustment_scope::computed(v, scale, mode);
    let record = p.recorded_text.as_deref().map(|t| t.replace(',', ".")).filter(|t| parse_decimal(t).is_ok());
    let registered = record.clone().or_else(|| p.recorded_text.clone()).unwrap_or_else(none);
    let calculated = w(p.calculated);
    let difference = calculated.as_deref().zip(record.as_deref()).and_then(|(c, r)| {
        subtract_exact(c, r).ok().and_then(|d| readjustment_scope::padded(&d, readjustment_scope::places(r).max(scale)))
    });
    vec![
        vec!["Tescilli yüzölçümü".to_owned(), registered],
        vec!["Hesaplanan yüzölçümü".to_owned(), calculated.unwrap_or_else(none)],
        vec!["Yüzölçümü farkı".to_owned(), difference.unwrap_or_else(none)],
        vec!["Yanılma sınırı".to_owned(), w(p.limit).unwrap_or_else(none)],
        vec!["Giren kısım".to_owned(), w(p.inside).unwrap_or_else(none)],
        vec!["Girmeyen kısım".to_owned(), w(p.outside).unwrap_or_else(none)],
        vec!["Girmeyen kısma kalan (tescilli − giren)".to_owned(), p.outside_recorded.clone().unwrap_or_else(none)],
    ]
}

/// The table of parts: each part's name, its boundary as point numbers and its area at the policy's places; with the
/// parcel numbers given for its outside parts (m. 13/3, docs/adr/3080), each outside part's name says its number.
pub fn part_rows(p: &ReadjustmentScopeParcel, scale: u32, mode: RoundingMode, numbers: Option<&[String]>) -> Vec<Vec<String>> {
    let mut outside = numbers.into_iter().flatten();
    part_names(&p.parts)
        .into_iter()
        .zip(&p.parts)
        .map(|(name, part)| {
            let name = match (part.side == "outside").then(|| outside.next()).flatten() {
                Some(n) => format!("{name} · parsel {}", n.trim()),
                None => name,
            };
            vec![
                name,
                boundary_text(part),
                readjustment_scope::computed(part.area, scale, mode).unwrap_or_else(|| "—".to_owned()),
            ]
        })
        .collect()
}

/// An object's parcel name as the scope writes it (`Ada/Parsel`, else the one there is); none without either or when
/// the id names no object.
fn parcel_name(doc: &Document, uid: &str) -> Option<String> {
    let e = kentos_domain::Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s))?;
    let attr = |k: &str| e.base().attrs.get(k).map(|v| v.trim().to_owned()).unwrap_or_default();
    let (ada, parsel) = (attr("Ada"), attr("Parsel"));
    match (ada.is_empty(), parsel.is_empty()) {
        (false, false) => Some(format!("{ada}/{parsel}")),
        (true, false) => Some(parsel),
        (false, true) => Some(ada),
        (true, true) => None,
    }
}

/// The parcel numbers checked (docs/adr/3080): each for a partial parcel of the scope, one for each of its outside
/// parts, not empty, at most 20 characters, none twice in a parcel.
fn numbers_ok(
    doc: &Document,
    parcels: &[ReadjustmentScopeParcel],
    partial: &[&ReadjustmentScopeParcel],
    numbers: Option<&std::collections::BTreeMap<String, Vec<String>>>,
) -> Result<(), Stop> {
    for (uid, given) in numbers.into_iter().flatten() {
        let at = format!("numbers.{uid}");
        let Some(p) = partial.iter().find(|p| p.uid == *uid) else {
            // Named by its Ada/Parsel when it has one (docs/adr/3880), and why: a parcel of the scope wholly inside the
            // boundary or wholly outside it has no part outside to number; any other is not among the scope's parcels.
            let of_scope = parcels.iter().find(|q| q.uid == *uid);
            let named = of_scope.map(|q| q.parcel.clone()).filter(|n| !n.is_empty()).or_else(|| parcel_name(doc, uid));
            let why = match of_scope.map(|q| q.kind.as_str()) {
                Some("whole") => "düzenleme sınırının tamamen içinde, dışarıda parçası yok",
                Some("outside") => "düzenleme sınırının tamamen dışında, düzenlemeye girmiyor",
                _ => "kapsamda kısmen giren bir parsel değil",
            };
            let said = match named {
                Some(name) => format!("“{name}” parseli {why}; numaralar kısmen giren parsellerin dışarıda kalan parçalarına verilir."),
                None => format!(
                    "“{uid}” kapsamda kısmen giren bir parsel değil; numaralar kısmen giren parsellerin dışarıda kalan parçalarına verilir."
                ),
            };
            return Err(fail(codes::INVALID_INPUT, said, &at));
        };
        let outside = p.parts.iter().filter(|q| q.side == "outside").count();
        if given.len() != outside {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{}” parselinin düzenleme dışında {outside} parçası var; {} parsel numarası verildi.", p.parcel, given.len()),
                &at,
            ));
        }
        for (k, n) in given.iter().enumerate() {
            let t = n.trim();
            if t.is_empty() || t.chars().count() > 20 {
                return Err(fail(codes::INVALID_INPUT, "Parsel numarası boş olamaz ve en çok 20 karakterdir.".into(), &format!("{at}[{k}]")));
            }
            if given[..k].iter().any(|m| m.trim() == t) {
                return Err(fail(
                    codes::INVALID_INPUT,
                    format!("“{}” parselinin iki parçasına aynı parsel numarası (“{t}”) verilmiş.", p.parcel),
                    &at,
                ));
            }
        }
    }
    Ok(())
}

/// The parcel's points as the drawing's: a point each, labelled with its number, its parcel and number as attributes.
fn point_objects(p: &ReadjustmentScopeParcel) -> Vec<NewObject> {
    p.points
        .iter()
        .enumerate()
        .map(|(i, c)| NewObject {
            geometry: EntityGeometry::Point { p: Vec2 { x: c[0], y: c[1] }, z: None, parts: None },
            color: None,
            line_weight: None,
            line_type: None,
            line_pattern: None,
            attrs: Some(BTreeMap::from([
                ("Parsel".to_owned(), p.parcel.clone()),
                ("Nokta".to_owned(), (i + 1).to_string()),
            ])),
            label: Some((i + 1).to_string()),
            symbol: None,
            label_of: None,
            label_scale: None,
            transparency: None,
            order: None,
        })
        .collect()
}

/// The group named “Ayırma çapı noktaları” at the top of the tree, if the drawing has one.
fn group_of(doc: &Document) -> Option<&LayerNode> {
    doc.layers().nodes().iter().find(|n| n.kind == LayerNodeType::Group && n.name == GROUP)
}

/// The template `source` names, or KentOS's separation sheet.
fn template_of(source: Option<&SheetTemplateSource>) -> Result<Template, Stop> {
    match source {
        None => template::system_template(DEFAULT_TEMPLATE).cloned().ok_or_else(|| {
            fail(codes::UNKNOWN_TEMPLATE, format!("“{DEFAULT_TEMPLATE}” sistem şablonu yok."), "source")
        }),
        Some(SheetTemplateSource::System { id }) => template::system_template(id).cloned().ok_or_else(|| {
            fail(
                codes::UNKNOWN_TEMPLATE,
                format!("“{id}” sistem şablonu yok; cad.sheets.set(details=true) ile şablonları listeleyin."),
                "source.id",
            )
        }),
        Some(SheetTemplateSource::Custom { template }) => template::read_template(&template.to_string())
            .map_err(|e| Stop::failed(error(&e.code, e.message, e.path.map(|p| format!("source.template.{p}"))))),
    }
}

fn core(e: kentos_sheet::SheetError) -> Stop {
    Stop::failed(error(&e.code, e.message, e.path))
}

/// A sheet to make: its parcel, its operations (the sheet, its map's ground, its tables' rows), its map and coordinate
/// list in the book, and the answer's row.
struct Planned {
    parcel: ReadjustmentScopeParcel,
    ops: Vec<Op>,
    map: String,
    coordinates: Option<String>,
    out: SeparationSheet,
}

struct Laid {
    planned: Vec<Planned>,
    pictures: Vec<SheetPicture>,
    group: Option<String>,
}

fn prepare(doc: &Document, input: &ReadjustmentSheets) -> Result<Laid, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let scope = readjustment_scope::run(doc, &input.scope).map_err(under)?;
    // The scope read the rounding; its places and rounding write the tables.
    let mode = RoundingMode::parse(input.scope.rounding.trim()).unwrap_or(RoundingMode::HalfEven);
    let partial: Vec<&ReadjustmentScopeParcel> = scope.parcels.iter().filter(|p| p.kind == "partial").collect();
    if partial.is_empty() {
        return Err(fail(
            codes::NO_PARTIAL_PARCELS,
            "Düzenleme sınırına kısmen giren parsel yok; ayırma çapı yalnız bir kısmı düzenleme sahasında kalan parsel için \
             düzenlenir (Yönetmelik m. 13/1)."
                .to_owned(),
            "scope",
        ));
    }
    for p in &partial {
        if p.no_identity {
            return Err(fail(
                codes::NO_IDENTITY,
                format!("“{}” parselinin Ada'sı ya da Parsel'i yok; ayırma çapı paftası parselin kimliğiyle adlandırılır.", p.uid),
                "scope",
            ));
        }
        if p.duplicate {
            return Err(fail(
                codes::DUPLICATE_PARCEL,
                format!("“{}” iki parselin kimliği; ayırma çapı paftası parselin kimliğiyle adlandırılır, önce kimliklerden birini düzeltin.", p.parcel),
                "scope",
            ));
        }
    }
    numbers_ok(doc, &scope.parcels, &partial, input.numbers.as_ref())?;
    // docs/adr/4280: the numbers' height on paper, the user's.
    if let Some(mm) = input.number_height
        && !(mm.is_finite() && mm > 0.0)
    {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Parsel numarasının yazı boyu 0'dan büyük bir milimetre olmalı (yazılan {mm})."),
            "numberHeight",
        ));
    }
    // docs/adr/4680: the leader is the numbers' text's.
    if input.number_leader && input.number_height.is_none() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Kılavuz, numaranın yazısıyla olur: numaranın yazı boyunu (numberHeight) da verin.".into(),
            "numberLeader",
        ));
    }
    let template = template_of(input.source.as_ref())?;
    let options: InstanceOptions = serde_json::from_value(input.options.clone())
        .map_err(|e| fail(codes::INVALID_OPTIONS, format!("Şablon seçenekleri okunamadı: {e}"), "options"))?;
    if options.name.is_some() {
        return Err(fail(
            codes::INVALID_OPTIONS,
            format!("Ayırma çapı paftalarının adları parsellerinden gelir (“{SHEET_WORD} 101/10”); options.name verilmez."),
            "options.name",
        ));
    }
    if options.center.is_some() {
        return Err(fail(
            codes::INVALID_OPTIONS,
            "Her paftanın haritası kendi parselini ortalar; options.center verilmez.".to_owned(),
            "options.center",
        ));
    }
    let parcel_asks: Vec<String> = OWN.iter().map(|q| expr::fold(q)).collect();
    for (i, v) in options.values.iter().enumerate() {
        if parcel_asks.contains(&expr::fold(&v.name)) {
            return Err(fail(
                codes::INVALID_OPTIONS,
                format!("“{}” her paftada parselin kendi özniteliğinden gelir; cevap olarak verilmez.", v.name),
                &format!("options.values[{i}].name"),
            ));
        }
    }
    let items = &template.sheet.items;
    let map_at = items
        .iter()
        .position(|i| i.id == MAP && matches!(&i.kind, ItemKind::Map(m) if matches!(m.view, MapView::Fixed(_))))
        .ok_or_else(|| {
            fail(
                codes::SHEET_NOT_FIXED_MAP,
                "Şablonda “harita” kimlikli sabit görünüm penceresi yok; ayırma çapının haritası ona yerleşir.".to_owned(),
                "source",
            )
        })?;
    let at = |id: &str, fits: fn(&ItemKind) -> bool| items.iter().position(|i| i.id == id && fits(&i.kind));
    let coordinates_at = at(COORDINATES, |k| matches!(k, ItemKind::CoordinateList(_)));
    let areas_at = at(AREAS, |k| matches!(k, ItemKind::Table(_)));
    let parts_at = at(PARTS, |k| matches!(k, ItemKind::Table(_)));
    let asked: BTreeSet<String> = template.variables.iter().map(|v| expr::fold(&v.name)).collect();
    let book = match doc.sheets() {
        Some(s) => kentos_sheet::project::read_project_sheets(s)
            .map_err(|e| Stop::failed(error(codes::BOOK_UNREADABLE, e.message, None)))?
            .0,
        None => Default::default(),
    };
    let names: BTreeSet<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    let mut taken: BTreeSet<String> = book.sheets.iter().map(|s| s.id.clone()).collect();
    let group = group_of(doc);
    let layers: BTreeSet<&str> = group.map(|g| g.children.iter().map(|c| c.name.as_str()).collect()).unwrap_or_default();
    let mut laid = Laid { planned: Vec::new(), pictures: Vec::new(), group: group.map(|g| g.id.clone()) };
    // docs/adr/5080: what the numbers outside must not touch: the drawing as the sheets' maps show it, but the
    // separation sheets' own point and piece layers (each map freezes the other parcels').
    let font = crate::geometry::drawing_font(doc.settings().drawing_font);
    let own: BTreeSet<String> = group.map(|g| g.children.iter().map(|c| c.id.clone()).collect()).unwrap_or_default();
    let drawing = |parts: &[&ScopePart], h: f64, width: f64| {
        separation_number_clear::Obstacles::of_drawing(doc, &own, &separation_number_clear::reach_of(parts, h, width), font)
    };
    for p in partial {
        let name = format!("{SHEET_WORD} {}", p.parcel);
        if names.contains(name.as_str()) {
            return Err(fail(
                codes::SEPARATION_SHEET_EXISTS,
                format!("“{name}” paftası zaten var; yeniden yapmak için önce o paftayı ve “{GROUP} › {}” katmanını silin.", p.parcel),
                "scope",
            ));
        }
        if layers.contains(p.parcel.as_str()) {
            return Err(fail(
                codes::SEPARATION_SHEET_EXISTS,
                format!("“{GROUP} › {}” katmanı zaten var; yeniden yapmak için önce o katmanı ve “{name}” paftasını silin.", p.parcel),
                "scope",
            ));
        }
        let sheet = (1..).map(|k| format!("ayirma-capi-{k}")).find(|id| !taken.contains(id)).unwrap_or_default();
        taken.insert(sheet.clone());
        let ids = create::ids_for(&template, &sheet);
        let mut own = options.clone();
        own.name = Some(name.clone());
        for ((q, folded), value) in OWN.iter().zip(&parcel_asks).zip([&p.il, &p.ilce, &p.mahalle, &p.ada, &p.parsel]) {
            if asked.contains(folded) {
                own.values.push(VariableValue { name: (*q).to_owned(), value: VarValue::Text(value.clone()) });
            }
        }
        let made = create::from_template(&template, &ids, &own, None).map_err(core)?;
        for picture in made.pictures {
            if !laid.pictures.iter().any(|q| q.sha256 == picture.meta.sha256) {
                let bytes = template::base64_decode(&picture.data).ok_or_else(|| {
                    fail("bad_asset", "Şablon resmi okunamadı.".to_owned(), "source.template.assets")
                })?;
                laid.pictures.push(SheetPicture { sha256: picture.meta.sha256, bytes });
            }
        }
        let item = template::instantiate(&template, &ids, &own).map_err(core)?.sheet.items.swap_remove(map_at);
        let bounds = sheets_fit::object_extent(doc, Some(std::slice::from_ref(&p.uid)))
            .map_err(|e| Stop::failed(*e))?
            .ok_or_else(|| fail(codes::ENTITY_NOT_FOUND, format!("“{}” parselinin kapsamı yok.", p.uid), "scope"))?;
        // The map's ground: the parcel's box fitted at a standard scale, or centred at the scale given.
        let centre = [bounds[0] + (bounds[2] - bounds[0]) / 2., bounds[1] + (bounds[3] - bounds[1]) / 2.];
        let (patch, scale) = match options.scale {
            Some(s) => (serde_json::json!({"kind": {"view": {"center": {"x": centre[0], "y": centre[1]}}}}), s),
            None => match fit::map(&item, bounds, MARGIN, true).map_err(core)? {
                Op::SetItemProps(SetItemProps { patch, .. }) => {
                    let s = patch["kind"]["view"]["scale"].as_u64().and_then(|s| u32::try_from(s).ok()).unwrap_or(0);
                    (patch, s)
                }
                _ => (serde_json::json!({"kind": {}}), 0),
            },
        };
        // The scale chosen stays: the map's view is locked against a slip of the wheel.
        let mut patch = patch;
        patch["kind"]["viewLocked"] = serde_json::Value::Bool(true);
        let mut ops = made.ops;
        ops.push(Op::SetItemProps(SetItemProps { id: item.id.clone(), patch }));
        let given = input.numbers.as_ref().and_then(|m| m.get(&p.uid)).map(Vec::as_slice);
        let (areas, parts) = (area_rows(p, input.scope.scale, mode), part_rows(p, input.scope.scale, mode, given));
        for (at, rows) in [(areas_at, &areas), (parts_at, &parts)] {
            if let Some(i) = at {
                ops.push(Op::SetItemProps(SetItemProps {
                    id: ids.items[i].clone(),
                    patch: serde_json::json!({"kind": {"source": {"type": "fixed", "rows": rows}}}),
                }));
            }
        }
        laid.planned.push(Planned {
            parcel: p.clone(),
            ops,
            map: item.id.clone(),
            coordinates: coordinates_at.map(|i| ids.items[i].clone()),
            out: SeparationSheet {
                sheet: ids.sheet.clone(),
                name,
                uid: p.uid.clone(),
                parcel: p.parcel.clone(),
                layer: None,
                scale,
                center: centre,
                points: u32::try_from(p.points.len()).unwrap_or(u32::MAX),
                areas,
                parts,
                labels: labels_of(
                    p,
                    given,
                    input.number_height,
                    input.number_leader,
                    scale,
                    font,
                    Some(&drawing),
                ),
            },
        });
    }
    Ok(laid)
}

/// docs/adr/5080: a warning for each number outside that touches something wherever it goes.
fn crowded(laid: &Laid) -> Vec<kentos_commands::CommandWarning> {
    let mut out = Vec::new();
    for p in &laid.planned {
        for l in p.out.labels.iter().filter(|l| l.clear == Some(false)) {
            out.push(kentos_commands::CommandWarning {
                code: "number_crowded".to_owned(),
                message: format!(
                    "“{}” parselinin {} parçasının numarası ({}) dışarıda hiçbir şeye değmeyen bir yere konamadı; en az şeye \
                     değdiği yere kondu ({} nesne ya da numara). Numaranın yazı boyunu küçültün ya da kılavuzu çizimde taşıyın.",
                    p.parcel.parcel,
                    l.part,
                    l.number,
                    l.clashes.len()
                ),
                path: Some(format!("numbers.{}", p.parcel.uid)),
            });
        }
    }
    out
}

/// The answer a plan gives: the sheets without their layers, nothing written.
fn planned(doc: &Document, laid: Laid) -> ReadjustmentSheetsMade {
    ReadjustmentSheetsMade {
        group: None,
        sheets: laid.planned.into_iter().map(|p| p.out).collect(),
        uids: Vec::new(),
        revision: doc.revision().to_string(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, laid: Laid) -> CommandResult<ReadjustmentSheetsMade> {
    let warnings = crowded(&laid);
    let Laid { planned, pictures, group } = laid;
    let step = cx.doc.begin_group(STEP);
    let group = match group {
        Some(g) => g,
        None => match cx.doc.add_layer(NewLayer::group(GROUP), None, false) {
            Ok(id) => id,
            Err(r) => {
                cx.doc.cancel_group(step);
                return fail(codes::INVALID_GROUP_NAME, r.to_string(), "scope").into();
            }
        },
    };
    let mut layers = Vec::new();
    let mut uids = Vec::new();
    for p in &planned {
        let layer = match cx.doc.add_layer(NewLayer::layer(p.parcel.parcel.clone()), Some(&group), false) {
            Ok(id) => id,
            Err(r) => {
                cx.doc.cancel_group(step);
                return fail(codes::LAYER_NOT_FOUND, r.to_string(), "scope").into();
            }
        };
        let create = EntitiesCreate {
            layer_id: layer.clone(),
            objects: point_objects(&p.parcel),
            operation: None,
            expected_revision: None,
        };
        match crate::create::write_in(cx, create) {
            CommandResult::Completed { output, .. } => uids.extend(output.created),
            other => {
                cx.doc.cancel_group(step);
                return crate::trim::relay(other);
            }
        }
        // The numbered pieces outside on a layer of their own beside the points (docs/adr/3480): the coordinate list
        // reads the points only, the map shows both.
        let mut own = vec![layer.clone()];
        if !p.out.labels.is_empty() {
            let pieces = match cx.doc.add_layer(NewLayer::layer(format!("{} {PIECES}", p.parcel.parcel)), Some(&group), false) {
                Ok(id) => id,
                Err(r) => {
                    cx.doc.cancel_group(step);
                    return fail(codes::LAYER_NOT_FOUND, r.to_string(), "scope").into();
                }
            };
            let create = EntitiesCreate {
                layer_id: pieces.clone(),
                objects: part_objects(&p.parcel, &p.out.labels),
                operation: None,
                expected_revision: None,
            };
            match crate::create::write_in(cx, create) {
                CommandResult::Completed { output, .. } => uids.extend(output.created),
                other => {
                    cx.doc.cancel_group(step);
                    return crate::trim::relay(other);
                }
            }
            own.push(pieces);
        }
        layers.push(own);
    }
    // Every point layer of the group now, the ones made before too: each map shows its own parcel's only.
    let all: Vec<String> = cx
        .doc
        .layers()
        .get(&group)
        .map(|g| g.children.iter().map(|c| c.id.clone()).collect())
        .unwrap_or_default();
    let mut ops: Vec<serde_json::Value> = Vec::new();
    let mut sheets = Vec::new();
    for (p, mine) in planned.into_iter().zip(layers) {
        let layer = mine[0].clone();
        let mut own: Vec<Op> = p.ops;
        let frozen: Vec<serde_json::Value> =
            all.iter().filter(|c| !mine.contains(c)).map(|c| serde_json::json!({"layer": c, "frozen": true})).collect();
        if !frozen.is_empty() {
            own.push(Op::SetItemProps(SetItemProps {
                id: p.map.clone(),
                patch: serde_json::json!({"kind": {"layerOverrides": frozen}}),
            }));
        }
        if let Some(list) = &p.coordinates {
            own.push(Op::SetItemProps(SetItemProps {
                id: list.clone(),
                patch: serde_json::json!({"kind": {"source": {"type": "layer", "layer": layer}, "naming": {"type": "given"}}}),
            }));
        }
        ops.extend(own.iter().map(|op| serde_json::to_value(op).unwrap_or_default()));
        let mut out = p.out;
        out.layer = Some(layer);
        sheets.push(out);
    }
    let edit = SheetsEdit { ops, pictures, replace_pictures: false, expected_revision: None };
    match sheets_edit::write_execute(cx, edit) {
        CommandResult::Completed { .. } => {}
        other => {
            cx.doc.cancel_group(step);
            return crate::trim::relay(other);
        }
    }
    cx.doc.end_group(step);
    CommandResult::Completed {
        output: ReadjustmentSheetsMade { group: Some(group), sheets, uids, revision: cx.doc.revision().to_string() },
        warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ReadjustmentSheets) -> CommandResult<()> {
    match prepare(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The sheets execute would make now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ReadjustmentSheets) -> CommandResult<ReadjustmentSheetsMade> {
    match prepare(cx.doc, input) {
        Ok(laid) => {
            let warnings = crowded(&laid);
            CommandResult::Completed { output: planned(cx.doc, laid), warnings }
        }
        Err(stop) => stop.into(),
    }
}

/// Writes the points and makes the sheets: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ReadjustmentSheets) -> CommandResult<ReadjustmentSheetsMade> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_READJUSTMENT_SHEETS,
        kentos_commands::CAD_READJUSTMENT_SHEETS_VERSION,
        input,
        |cx, input| match prepare(cx.doc, &input) {
            Ok(laid) => write(cx, laid),
            Err(stop) => stop.into(),
        },
    )
}
