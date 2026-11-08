//! `cad.scenarios.impact` v1 (plan/map-cad.md MAP-10, docs/adr/1440): Senaryo ölçüleri, read only. The scenario's
//! pairs (`scenario_pairs`: a base layer and the scenario's layer standing for it) measured side by side: objects,
//! the areas (the shared core's planar `entity_area`), the lines' length, the building capacity (area × KAKS), the
//! areas by use; the decisions that changed, matched by their key; the data's gaps and the assumptions. The base's
//! stamp is a SHA-256 of the base layers' objects (id, layer, kind, attributes, the vertices' bits), so an earlier
//! comparison's stamp says whether the field state has changed since. Asked for, two measures more with the user's
//! values (docs/adr/2040): the impervious area (each use's area × its share) and the area reached (a `from` area whose
//! centroid, the core's `shape_centroid`, lies within the distance of a `to` area by the core's exact distance,
//! arcs included) with its share, and which areas it reached on each side (docs/adr/2260); the area near an
//! infrastructure layer (an area whose centroid lies within the distance of an object of the layer, the side's own
//! layer when the scenario stands in for it) with its share (docs/adr/2660). The shared cases are
//! `fixtures/commands/v1/cad.scenarios.impact.json`.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, Entity, ImpactChange, ImpactLayer, ImpactMeasure, ImpactReach, ImpactUse, LayerNodeType,
    ScenariosImpact, ScenariosImpacted, scenario_pairs,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::{Shape, entity_area, entity_length};
use kentos_geometry_core::geom::centroid::shape_centroid;
use kentos_geometry_core::ops::spatial_query::{distance, geometry_of};
use sha2::{Digest, Sha256};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geometry::shape;
use crate::zoning_check::OPEN_USES;

const FIELD: &str = "Kullanim";
const KEY: &str = "KararKimlik";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// An object's vertices as the stamp and the change take them: a line's two ends, a polyline's and an area's points
/// (an area's holes after), a point's place.
fn vertices(e: &Entity) -> Vec<(f64, f64)> {
    match e {
        Entity::Line(l) => vec![(l.a.x, l.a.y), (l.b.x, l.b.y)],
        Entity::Polyline(p) => p.pts.iter().map(|v| (v.x, v.y)).collect(),
        Entity::Polygon(p) => p
            .pts
            .iter()
            .chain(p.holes.iter().flatten().flat_map(|h| h.pts.iter()))
            .map(|v| (v.x, v.y))
            .collect(),
        Entity::Point(p) => vec![(p.p.x, p.p.y)],
        _ => Vec::new(),
    }
}

/// The base's stamp: every object of the base layers, layer by layer in the pairs' order, in the drawing's order.
fn stamp(doc: &Document, layers: &[String]) -> String {
    let mut h = Sha256::new();
    for layer in layers {
        for e in doc.entities().filter(|e| e.base().layer_id == *layer) {
            let attrs: Vec<String> = e.base().attrs.iter().map(|(k, v)| format!("{k}={v}")).collect();
            let pts: Vec<String> =
                vertices(e).iter().map(|(x, y)| format!("{:016x},{:016x}", x.to_bits(), y.to_bits())).collect();
            h.update(format!("{}|{}|{}|{}|{}\n", e.base().id, layer, e.kind(), attrs.join(";"), pts.join(";")).as_bytes());
        }
    }
    h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// One side's measures over its layers.
#[derive(Default)]
struct Side<'a> {
    objects: usize,
    area: f64,
    length: f64,
    capacity: f64,
    no_kaks: usize,
    no_use: usize,
    no_key: usize,
    uses: BTreeMap<String, f64>,
    keyed: Vec<(String, &'a Entity, f64)>,
    /// Every area: its shape, use, area and persistent id (docs/adr/2040's measures, 2260's ids).
    areas: Vec<(Shape, String, f64, String)>,
}

fn side<'a>(doc: &'a Document, layers: &[String], field: &str, key: &str) -> Side<'a> {
    let mut s = Side::default();
    for layer in layers {
        for e in doc.entities().filter(|e| e.base().layer_id == *layer) {
            s.objects += 1;
            let attr = |k: &str| e.base().attrs.get(k).map(|v| v.trim().to_owned()).unwrap_or_default();
            match e {
                Entity::Polygon(_) => {
                    let area = entity_area(&shape(e)).unwrap_or(0.0);
                    s.area += area;
                    let usage = attr(field);
                    let uid = doc.uid(Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default();
                    s.areas.push((shape(e), usage.clone(), area, uid));
                    if usage.is_empty() {
                        s.no_use += 1;
                    }
                    *s.uses.entry(usage.clone()).or_default() += area;
                    if !OPEN_USES.contains(&usage.as_str()) {
                        match attr("KAKS").replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite()) {
                            Some(k) => s.capacity += area * k,
                            None => s.no_kaks += 1,
                        }
                    }
                    let k = attr(key);
                    if k.is_empty() {
                        s.no_key += 1;
                    } else if !s.keyed.iter().any(|(x, _, _)| *x == k) {
                        s.keyed.push((k, e, area));
                    }
                }
                Entity::Line(_) | Entity::Polyline(_) => s.length += entity_length(&shape(e)).unwrap_or(0.0),
                _ => {}
            }
        }
    }
    s
}

/// A decision's capacity: area × KAKS; none on an open use or without a KAKS.
fn capacity(e: &Entity, area: f64, field: &str) -> Option<f64> {
    let attr = |k: &str| e.base().attrs.get(k).map(|v| v.trim().to_owned()).unwrap_or_default();
    if OPEN_USES.contains(&attr(field).as_str()) {
        return Some(0.0);
    }
    attr("KAKS").replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite()).map(|k| area * k)
}

/// The impervious area of a side: each area × its use's share; the uses with area and no share.
fn impervious(s: &Side<'_>, shares: &BTreeMap<String, f64>) -> (f64, Vec<String>) {
    let mut out = 0.0;
    let mut missing = Vec::new();
    for (_, usage, area, _) in &s.areas {
        match shares.get(usage) {
            Some(k) => out += area * k,
            None if *area > 0.0 && !missing.contains(usage) => missing.push(usage.clone()),
            None => {}
        }
    }
    (out, missing)
}

/// What the access found on a side: the area reached, the `from` areas' whole, and by persistent id the `from` areas
/// reached, those not, and the `to` areas (whether the side has one at all).
struct Reached {
    got: f64,
    whole: f64,
    reached: Vec<String>,
    unreached: Vec<String>,
    targets: Vec<String>,
}

/// The access on a side: the `from` areas whose centroid lies within `d` of a `to` area.
fn reached(s: &Side<'_>, access: &kentos_commands::ImpactAccess) -> Reached {
    let to: Vec<_> = s.areas.iter().filter(|(_, u, _, _)| access.to.contains(u)).collect();
    let targets: Vec<_> = to.iter().filter_map(|(sh, _, _, _)| geometry_of(sh)).collect();
    let mut r = Reached {
        got: 0.0,
        whole: 0.0,
        reached: Vec::new(),
        unreached: Vec::new(),
        targets: to.iter().map(|(_, _, _, uid)| uid.clone()).collect(),
    };
    for (sh, _, area, uid) in s.areas.iter().filter(|(_, u, _, _)| access.from.contains(u)) {
        r.whole += area;
        let near = shape_centroid(sh)
            .and_then(|c| geometry_of(&Shape::Point { p: c, z: None, parts: None }))
            .is_some_and(|point| targets.iter().map(|t| distance(&point, t)).any(|d| d <= access.distance));
        if near {
            r.got += area;
            r.reached.push(uid.clone());
        } else {
            r.unreached.push(uid.clone());
        }
    }
    r
}

/// What the infrastructure measure found on a side (docs/adr/2660): the area near, the measured areas' whole, and how
/// many objects the side's infrastructure layer has.
struct Near {
    got: f64,
    whole: f64,
    objects: usize,
}

/// The area of a side's measured areas (`from`'s uses, or all) whose centroid lies within `d` of an object of `layer`.
fn near(doc: &Document, s: &Side<'_>, layer: &str, infra: &kentos_commands::ImpactInfrastructure) -> Near {
    let lines: Vec<_> = doc
        .entities()
        .filter(|e| e.base().layer_id == layer)
        .filter_map(|e| geometry_of(&shape(e)))
        .collect();
    let mut r = Near { got: 0.0, whole: 0.0, objects: lines.len() };
    let measured = |u: &String| infra.from.iter().all(|f| f.trim().is_empty()) || infra.from.contains(u);
    for (sh, _, area, _) in s.areas.iter().filter(|(_, u, _, _)| measured(u)) {
        r.whole += area;
        let close = shape_centroid(sh)
            .and_then(|c| geometry_of(&Shape::Point { p: c, z: None, parts: None }))
            .is_some_and(|point| lines.iter().map(|t| distance(&point, t)).any(|d| d <= infra.distance));
        if close {
            r.got += area;
        }
    }
    r
}

/// The asked measures' own checks.
fn asked(input: &ScenariosImpact) -> Result<(), Stop> {
    for (code, k) in input.impervious.iter().flatten() {
        if !(k.is_finite() && (0.0..=1.0).contains(k)) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{code}” kullanımının geçirimsizlik payı 0 ile 1 arasında olmalı (yazılan {k})."),
                &format!("impervious.{code}"),
            ));
        }
    }
    if let Some(a) = &input.access {
        if !(a.distance.is_finite() && a.distance > 0.0) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("Erişim uzaklığı 0'dan büyük olmalı (yazılan {}).", a.distance),
                "access.distance",
            ));
        }
        for (list, path, words) in [(&a.from, "access.from", "Erişen"), (&a.to, "access.to", "Erişilen")] {
            if list.iter().all(|u| u.trim().is_empty()) {
                return Err(fail(codes::INVALID_INPUT, format!("{words} kullanım verilmemiş; en az bir kullanım kodu yazın."), path));
            }
        }
    }
    if let Some(i) = &input.infrastructure
        && !(i.distance.is_finite() && i.distance > 0.0)
    {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Altyapı uzaklığı 0'dan büyük olmalı (yazılan {}).", i.distance),
            "infrastructure.distance",
        ));
    }
    Ok(())
}

fn run(doc: &Document, input: &ScenariosImpact) -> Result<ScenariosImpacted, Stop> {
    asked(input)?;
    let id = input.scenario.trim();
    let Some(group) = doc.layers().get(id).filter(|n| n.kind == LayerNodeType::Group && n.scenario.is_some()) else {
        return Err(fail(
            codes::SCENARIO_NOT_FOUND,
            format!("“{id}” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin."),
            "scenario",
        ));
    };
    let pairs: Vec<(String, String)> =
        scenario_pairs(doc.layers().nodes(), group).into_iter().map(|(b, l)| (b.to_owned(), l.to_owned())).collect();
    if pairs.is_empty() {
        return Err(fail(
            codes::NOTHING_TO_COMPARE,
            format!("“{}” senaryosunun hiçbir katmanı bir ana katmanın yerine geçmiyor; karşılaştırılacak bir şey yok.", group.name),
            "scenario",
        ));
    }
    let field = input.field.as_deref().map(str::trim).filter(|f| !f.is_empty()).unwrap_or(FIELD).to_owned();
    let key = input.key.as_deref().map(str::trim).filter(|f| !f.is_empty()).unwrap_or(KEY).to_owned();
    let bases: Vec<String> = pairs.iter().map(|(b, _)| b.clone()).collect();
    let alts: Vec<String> = pairs.iter().map(|(_, l)| l.clone()).collect();
    let (b, a) = (side(doc, &bases, &field, &key), side(doc, &alts, &field, &key));
    let measure = |id: &str, label: &str, unit: &str, base: f64, alternative: f64, method: &str| ImpactMeasure {
        id: id.into(),
        label: label.into(),
        unit: unit.into(),
        base,
        alternative,
        difference: alternative - base,
        method: method.into(),
    };
    let mut measures = vec![
        measure("objects", "Nesne sayısı", "count", b.objects as f64, a.objects as f64, "Karşılaştırılan katmanlardaki bütün nesneler."),
        measure("area", "Alanların toplamı", "m2", b.area, a.area, "Alanların düzlemsel alanı, koordinatlardan (yay ve delikler dahil)."),
        measure("length", "Çizgilerin uzunluğu", "m", b.length, a.length, "Çizgi ve çoklu çizgilerin düzlemsel uzunluğu."),
        measure(
            "capacity",
            "İnşaat alanı (alan × KAKS)",
            "m2",
            b.capacity,
            a.capacity,
            "Her yapılaşan alanın alanı × KAKS; KAKS'ı olmayan alan sayılmaz (Veri eksikleri).",
        ),
    ];
    let mut asked_gaps: Vec<String> = Vec::new();
    if let Some(shares) = &input.impervious {
        let ((x, mb), (y, ma)) = (impervious(&b, shares), impervious(&a, shares));
        let given: Vec<String> = shares.iter().map(|(c, k)| format!("{c} {k}")).collect();
        measures.push(measure(
            "impervious",
            "Geçirimsiz alan",
            "m2",
            x,
            y,
            &format!("Her alanın alanı × kullanımının verilen geçirimsizlik payı ({}); payı verilmeyen kullanım sayılmaz (Veri eksikleri).", given.join(", ")),
        ));
        let mut missing: Vec<String> = mb.into_iter().chain(ma).collect();
        missing.sort();
        missing.dedup();
        if !missing.is_empty() {
            let words: Vec<String> = missing.iter().map(|m| if m.is_empty() { "(kullanımı yok)".to_owned() } else { m.clone() }).collect();
            asked_gaps.push(format!("Geçirimsizlik payı verilmemiş kullanım: {}; geçirimsiz alan onlarsız hesaplandı.", words.join(", ")));
        }
    }
    let mut reach = None;
    if let Some(access) = &input.access {
        let (x, y) = (reached(&b, access), reached(&a, access));
        let (xg, xw, xt, yg, yw, yt) = (x.got, x.whole, !x.targets.is_empty(), y.got, y.whole, !y.targets.is_empty());
        reach = Some(ImpactReach {
            base_reached: x.reached,
            base_unreached: x.unreached,
            base_targets: x.targets,
            alternative_reached: y.reached,
            alternative_unreached: y.unreached,
            alternative_targets: y.targets,
        });
        let (from, to) = (access.from.join(", "), access.to.join(", "));
        measures.push(measure(
            "access",
            &format!("Erişen alan ({from} → {to}, {} m)", access.distance),
            "m2",
            xg,
            yg,
            &format!(
                "Kullanımı {from} olan alanların ağırlık merkezinden kullanımı {to} olan en yakın alana düz uzaklık en çok {} m ise alanı sayılır; merkez alanın içindeyse uzaklık 0.",
                access.distance
            ),
        ));
        let share = |got: f64, whole: f64| if whole > 0.0 { got / whole * 100.0 } else { 0.0 };
        measures.push(measure(
            "accessShare",
            "Erişen alanın payı",
            "%",
            share(xg, xw),
            share(yg, yw),
            &format!("Erişen alanın {from} alanlarının toplamına oranı; {from} alanı olmayan tarafta 0."),
        ));
        for (has_from, has_to, side) in [(xw > 0.0, xt, "mevcut durumda"), (yw > 0.0, yt, "senaryoda")] {
            if has_from && !has_to {
                asked_gaps.push(format!("Erişilecek kullanım ({to}) {side} yok; erişen alan 0."));
            }
        }
    }
    // Altyapıya yakınlık (docs/adr/2660): the layer the scenario stands in for is each side's own.
    if let Some(infra) = &input.infrastructure {
        let id = infra.layer.trim();
        let Some(node) = doc.layers().get(id).filter(|n| n.kind != LayerNodeType::Group) else {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{id}” kimlikli altyapı katmanı çizimde yok; bir katmanın kimliğini verin."),
                "infrastructure.layer",
            ));
        };
        let theirs = pairs.iter().find(|(base, _)| base == id).map_or(id, |(_, l)| l.as_str());
        let (x, y) = (near(doc, &b, id, infra), near(doc, &a, theirs, infra));
        let name = &node.name;
        let uses: Vec<&str> = infra.from.iter().map(|u| u.trim()).filter(|u| !u.is_empty()).collect();
        let (label, which) = if uses.is_empty() {
            (format!("Altyapıya yakın alan ({name}, {} m)", infra.distance), "Alanların".to_owned())
        } else {
            (
                format!("Altyapıya yakın alan ({} → {name}, {} m)", uses.join(", "), infra.distance),
                format!("Kullanımı {} olan alanların", uses.join(", ")),
            )
        };
        measures.push(measure(
            "infrastructure",
            &label,
            "m2",
            x.got,
            y.got,
            &format!(
                "{which} ağırlık merkezinden {name} katmanındaki en yakın nesneye düz uzaklık en çok {} m ise alanı sayılır; hattın kapasitesi ya da bağlantı hakkı değildir.",
                infra.distance
            ),
        ));
        let share = |got: f64, whole: f64| if whole > 0.0 { got / whole * 100.0 } else { 0.0 };
        measures.push(measure(
            "infrastructureShare",
            "Altyapıya yakın alanın payı",
            "%",
            share(x.got, x.whole),
            share(y.got, y.whole),
            "Altyapıya yakın alanın ölçülen alanların toplamına oranı; ölçülen alanı olmayan tarafta 0.",
        ));
        for (n, side) in [(&x, "mevcut durumda"), (&y, "senaryoda")] {
            if n.whole > 0.0 && n.objects == 0 {
                asked_gaps.push(format!("Altyapı katmanında ({name}) {side} nesne yok; yakın alan 0."));
            }
        }
    }
    let mut codes_: Vec<&String> = b.uses.keys().chain(a.uses.keys()).collect();
    codes_.sort();
    codes_.dedup();
    let uses = codes_
        .into_iter()
        .map(|c| {
            let (x, y) = (b.uses.get(c).copied().unwrap_or(0.0), a.uses.get(c).copied().unwrap_or(0.0));
            ImpactUse { code: c.clone(), base: x, alternative: y, difference: y - x }
        })
        .collect();
    // The decisions that changed, matched by their key: the base's in its order, then the alternative's new ones.
    let attr = |e: &Entity, k: &str| e.base().attrs.get(k).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
    let decided = |e: &Entity| -> BTreeMap<String, String> {
        e.base().attrs.iter().map(|(k, v)| (k.clone(), v.trim().to_owned())).filter(|(_, v)| !v.is_empty()).collect()
    };
    let mut changes = Vec::new();
    for (k, eb, ab) in &b.keyed {
        match a.keyed.iter().find(|(x, _, _)| x == k) {
            None => changes.push(ImpactChange {
                key: k.clone(),
                change: "removed".into(),
                base_use: attr(eb, &field),
                alternative_use: None,
                base_area: Some(*ab),
                alternative_area: None,
                base_capacity: capacity(eb, *ab, &field),
                alternative_capacity: None,
            }),
            Some((_, ea, aa)) => {
                let geometry = vertices(eb) != vertices(ea);
                let decision = decided(eb) != decided(ea);
                let change = match (geometry, decision) {
                    (false, false) => continue,
                    (true, false) => "geometry",
                    (false, true) => "decision",
                    (true, true) => "both",
                };
                changes.push(ImpactChange {
                    key: k.clone(),
                    change: change.into(),
                    base_use: attr(eb, &field),
                    alternative_use: attr(ea, &field),
                    base_area: Some(*ab),
                    alternative_area: Some(*aa),
                    base_capacity: capacity(eb, *ab, &field),
                    alternative_capacity: capacity(ea, *aa, &field),
                });
            }
        }
    }
    for (k, ea, aa) in &a.keyed {
        if !b.keyed.iter().any(|(x, _, _)| x == k) {
            changes.push(ImpactChange {
                key: k.clone(),
                change: "added".into(),
                base_use: None,
                alternative_use: attr(ea, &field),
                base_area: None,
                alternative_area: Some(*aa),
                base_capacity: None,
                alternative_capacity: capacity(ea, *aa, &field),
            });
        }
    }
    let mut gaps = Vec::new();
    if b.no_kaks + a.no_kaks > 0 {
        gaps.push(format!(
            "KAKS'ı olmayan yapılaşan alan: mevcut durumda {}, senaryoda {}; inşaat alanı onlarsız hesaplandı.",
            b.no_kaks, a.no_kaks
        ));
    }
    if b.no_use + a.no_use > 0 {
        gaps.push(format!("Kullanımı ({field}) olmayan alan: mevcut durumda {}, senaryoda {}.", b.no_use, a.no_use));
    }
    if b.no_key + a.no_key > 0 {
        gaps.push(format!(
            "Anahtarı ({key}) olmayan alan: mevcut durumda {}, senaryoda {}; değişiklikleri izlenemez.",
            b.no_key, a.no_key
        ));
    }
    gaps.extend(asked_gaps);
    let assumptions = vec![
        format!("Alanlar ve uzunluklar düzlemsel, projenin koordinatlarından (EPSG:{}).", doc.settings().srid),
        "İnşaat alanı = alan × KAKS: brüt alan, terk ve DOP düşülmeden.".to_owned(),
        "Park, yeşil alan, meydan ve yol inşaat alanı taşımaz.".to_owned(),
        "Tek puan yok: ölçüler yan yana verilir; senaryoyu kullanıcı seçer.".to_owned(),
    ];
    let now = stamp(doc, &bases);
    let names = |l: &str| doc.layers().get(l).map_or_else(|| l.to_owned(), |n| n.name.clone());
    Ok(ScenariosImpacted {
        scenario: id.to_owned(),
        name: group.name.clone(),
        layers: pairs.iter().map(|(b, l)| ImpactLayer { base: b.clone(), layer: l.clone(), name: names(b) }).collect(),
        field,
        key,
        measures,
        uses,
        changes,
        gaps,
        assumptions,
        stale: input.since.as_deref().map(|s| s.trim() != now),
        reach,
        stamp: now,
        revision: doc.revision().to_string(),
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ScenariosImpact) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
/// The answer for a window or the drawing's marks (docs/adr/2260), or why it does not read.
pub fn preview(doc: &Document, input: &ScenariosImpact) -> Result<ScenariosImpacted, String> {
    run(doc, input).map_err(crate::road_corridors::words)
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ScenariosImpact) -> CommandResult<ScenariosImpacted> {
    match run(cx.doc, input) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ScenariosImpact) -> CommandResult<ScenariosImpacted> {
    plan(cx, &input)
}
