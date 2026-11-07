//! `file.exchange.check` v1 (plan/map-cad.md MAP-17, docs/adr/1660): Devir denetimi. The drawing's layers written to
//! an exchange format by KentOS's writer (the format's `file.export.*` on a copy of the drawing), read back by KentOS's
//! reader (`file.import.*`) into an empty copy (the settings and layers, no objects), and compared:
//!
//! - **Matching:** an object read back with an original's persistent id is that one's; the others by their class
//!   (point, line, area; a text, a block … by its kind) and the nearest original of the class by the centres of their
//!   boxes, then the lines and areas left by the nearest of any class (a circle read back as a line) (FlatGeobuf writes in
//!   Hilbert order, a Shapefile splits a layer by geometry, GML renames the types: neither order nor layer is kept).
//! - **Numeric:** each vertex read back measured to the original's geometry exactly (points, lines, arcs of
//!   polylines and areas, circles, arcs; an ellipse, a spline or a NURBS by its outline of 1024 pieces), the heights
//!   and measures vertex by vertex where the counts agree, the length's and the area's differences.
//! - **Semantic:** the losses of [`EXCHANGE_LOSSES`], each with its detail and the originals' ids.
//!
//! The file is written in a folder of its own under the system's temporary folder and removed with it, whatever the
//! outcome. The open drawing is not changed: the writer runs on a copy.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use kentos_commands::{
    CallOp, CommandError, CommandResult, CommandWarning, EXCHANGE_LOSSES, EXCHANGE_TOLERANCE, EXCHANGE_UIDS_LISTED,
    Entity, ExchangeCheck, ExchangeChecked, ExchangeKindCount, ExchangeLoss, ExchangeNumeric, PathEntity, Vec2, codes,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::{entity_area, entity_bounds, entity_length, entity_outline, entity_vertices};
use kentos_native_application::ExecutionContext;
use kentos_native_application::geometry::shape;
use serde_json::{Value, json};

fn refused(code: &str, message: String, path: Option<String>) -> CommandResult<Value> {
    CommandResult::Failed { error: CommandError { code: code.into(), message, path, ..CommandError::default() } }
}

/// What an object is to the formats: a point, a line or an area; anything else they do not carry as geometry.
fn class(e: &Entity) -> &'static str {
    match e {
        Entity::Point(_) => "point",
        Entity::Polygon(_) | Entity::Circle(_) => "area",
        Entity::Line(_) | Entity::Polyline(_) | Entity::Arc(_) | Entity::Ellipse(_) | Entity::Spline(_) | Entity::Nurbs(_) => {
            "line"
        }
        _ => "other",
    }
}

fn path_curved(p: &PathEntity) -> bool {
    let bent = |b: &Option<Vec<f64>>| b.as_ref().is_some_and(|b| b.iter().any(|x| *x != 0.0));
    bent(&p.bulges)
        || p.holes.iter().flatten().any(|h| bent(&h.bulges))
        || p.parts.iter().flatten().any(|q| bent(&q.bulges) || q.holes.iter().flatten().any(|h| bent(&h.bulges)))
}

/// Whether the object has a curve: an arc, a circle, an ellipse, a spline, a NURBS or a bulged edge.
fn curved(e: &Entity) -> bool {
    match e {
        Entity::Circle(_) | Entity::Arc(_) | Entity::Ellipse(_) | Entity::Spline(_) | Entity::Nurbs(_) => true,
        Entity::Polyline(p) | Entity::Polygon(p) => path_curved(p),
        _ => false,
    }
}

/// The vertices' heights and measures in order (a path's first part, then its holes, then its other parts).
fn heights(e: &Entity) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    match e {
        Entity::Point(p) => (vec![p.z], vec![None]),
        Entity::Line(l) => (vec![l.za, l.zb], vec![None, None]),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let of = |v: &Option<Vec<Option<f64>>>, n: usize| v.clone().unwrap_or_else(|| vec![None; n]);
            let mut z = of(&p.zs, p.pts.len());
            let mut m = of(&p.ms, p.pts.len());
            for h in p.holes.iter().flatten() {
                z.extend(of(&h.zs, h.pts.len()));
                m.extend(vec![None; h.pts.len()]);
            }
            for q in p.parts.iter().flatten() {
                z.extend(of(&q.zs, q.pts.len()));
                m.extend(of(&q.ms, q.pts.len()));
                for h in q.holes.iter().flatten() {
                    z.extend(of(&h.zs, h.pts.len()));
                    m.extend(vec![None; h.pts.len()]);
                }
            }
            (z, m)
        }
        _ => (Vec::new(), Vec::new()),
    }
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    if l2 == 0.0 {
        return dist(p, a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0);
    dist(p, Vec2 { x: a.x + t * dx, y: a.y + t * dy })
}

/// The distance from `p` to the arc of centre `c`, radius `r`, from angle `a0` counter-clockwise by `sweep`
/// (radians, 0..2π]; its ends otherwise.
fn to_arc(p: Vec2, c: Vec2, r: f64, a0: f64, sweep: f64, ends: (Vec2, Vec2)) -> f64 {
    let tau = std::f64::consts::TAU;
    let ang = (p.y - c.y).atan2(p.x - c.x);
    let rel = (ang - a0).rem_euclid(tau);
    if rel <= sweep || p == c {
        (dist(p, c) - r).abs()
    } else {
        dist(p, ends.0).min(dist(p, ends.1))
    }
}

/// The distance from `p` to the edge from `a` to `b` bulged by `bulge` (the tangent of a quarter of its angle;
/// positive counter-clockwise).
fn to_edge(p: Vec2, a: Vec2, b: Vec2, bulge: f64) -> f64 {
    if bulge == 0.0 {
        return to_segment(p, a, b);
    }
    let chord = dist(a, b);
    if chord == 0.0 {
        return dist(p, a);
    }
    let theta = 4.0 * bulge.atan();
    let r = chord / (2.0 * (theta / 2.0).sin().abs());
    // The centre: from the chord's middle along its left normal by chord·(1 − b²)/(4b) (a semicircle's is the middle).
    let (mx, my) = ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let (ux, uy) = ((b.x - a.x) / chord, (b.y - a.y) / chord);
    let k = chord * (1.0 - bulge * bulge) / (4.0 * bulge);
    let c = Vec2 { x: mx - uy * k, y: my + ux * k };
    let (sa, sb) = ((a.y - c.y).atan2(a.x - c.x), (b.y - c.y).atan2(b.x - c.x));
    let sweep = theta.abs();
    if bulge > 0.0 { to_arc(p, c, r, sa, sweep, (a, b)) } else { to_arc(p, c, r, sb, sweep, (a, b)) }
}

fn ring_dist(p: Vec2, pts: &[Vec2], bulges: Option<&Vec<f64>>, closed: bool) -> f64 {
    let n = pts.len();
    if n == 1 {
        return dist(p, pts[0]);
    }
    let edges = if closed { n } else { n - 1 };
    (0..edges)
        .map(|i| to_edge(p, pts[i], pts[(i + 1) % n], bulges.and_then(|b| b.get(i).copied()).unwrap_or(0.0)))
        .fold(f64::INFINITY, f64::min)
}

/// The distance from `p` to the original's geometry.
fn to_original(o: &Entity, p: Vec2) -> f64 {
    match o {
        Entity::Point(e) => {
            let mut d = dist(p, e.p);
            for q in e.parts.iter().flatten() {
                d = d.min(dist(p, q.p));
            }
            d
        }
        Entity::Line(l) => to_segment(p, l.a, l.b),
        Entity::Polyline(e) | Entity::Polygon(e) => {
            let area = matches!(o, Entity::Polygon(_));
            let mut d = ring_dist(p, &e.pts, e.bulges.as_ref(), area || e.closed);
            for h in e.holes.iter().flatten() {
                d = d.min(ring_dist(p, &h.pts, h.bulges.as_ref(), true));
            }
            for q in e.parts.iter().flatten() {
                d = d.min(ring_dist(p, &q.pts, q.bulges.as_ref(), area));
                for h in q.holes.iter().flatten() {
                    d = d.min(ring_dist(p, &h.pts, h.bulges.as_ref(), true));
                }
            }
            d
        }
        Entity::Circle(c) => (dist(p, c.c) - c.r).abs(),
        Entity::Arc(a) => {
            let at = |t: f64| Vec2 { x: a.c.x + a.r * t.cos(), y: a.c.y + a.r * t.sin() };
            let sweep = (a.a1 - a.a0).rem_euclid(std::f64::consts::TAU);
            let sweep = if sweep == 0.0 { std::f64::consts::TAU } else { sweep };
            to_arc(p, a.c, a.r, a.a0, sweep, (at(a.a0), at(a.a1)))
        }
        other => {
            let outline: Vec<Vec2> = entity_outline(&shape(other), 1024.0).into_iter().map(|q| Vec2 { x: q.x, y: q.y }).collect();
            if outline.len() < 2 {
                return outline.first().map_or(f64::NAN, |q| dist(p, *q));
            }
            outline.windows(2).map(|w| to_segment(p, w[0], w[1])).fold(f64::INFINITY, f64::min)
        }
    }
}

/// The places of an object read back that are measured: its vertices; a circle's four quarter points, an arc's ends
/// and middle.
fn places(e: &Entity) -> Vec<Vec2> {
    match e {
        Entity::Circle(c) => (0..4)
            .map(|k| {
                let t = k as f64 * std::f64::consts::FRAC_PI_2;
                Vec2 { x: c.c.x + c.r * t.cos(), y: c.c.y + c.r * t.sin() }
            })
            .collect(),
        Entity::Arc(a) => {
            let sweep = (a.a1 - a.a0).rem_euclid(std::f64::consts::TAU);
            [a.a0, a.a0 + sweep / 2.0, a.a0 + sweep].iter().map(|t| Vec2 { x: a.c.x + a.r * t.cos(), y: a.c.y + a.r * t.sin() }).collect()
        }
        other => entity_vertices(&shape(other)).into_iter().map(|q| Vec2 { x: q.x, y: q.y }).collect(),
    }
}

fn plain(text: &str) -> Option<f64> {
    let t = text.trim();
    let body = t.strip_prefix('-').unwrap_or(t);
    let (w, f) = body.split_once('.').unwrap_or((body, "0"));
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (digits(w) && digits(f)).then(|| t.parse::<f64>().ok()).flatten()
}

/// Two attribute values the same: as text (trimmed), or as the same plain number (`12.50` and `12.5`).
fn same_value(a: &str, b: &str) -> bool {
    a.trim() == b.trim() || matches!((plain(a), plain(b)), (Some(x), Some(y)) if x == y)
}

struct Losses {
    found: BTreeMap<(usize, String), (u32, Vec<String>)>,
}

impl Losses {
    fn add(&mut self, kind: &str, detail: impl Into<String>, uid: &str) {
        let order = EXCHANGE_LOSSES.iter().position(|k| *k == kind).unwrap_or(EXCHANGE_LOSSES.len());
        let e = self.found.entry((order, detail.into())).or_default();
        e.0 += 1;
        if e.1.len() < EXCHANGE_UIDS_LISTED && !uid.is_empty() {
            e.1.push(uid.to_owned());
        }
    }
}

fn messages(answer: &Value, key: &str) -> Vec<String> {
    answer[key].as_array().map(|w| w.iter().filter_map(|x| x["message"].as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

/// The temporary folder of one check, removed when it goes.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs the check (`validate`: its input only).
pub fn run(doc: &Document, input: &ExchangeCheck, op: CallOp) -> CommandResult<Value> {
    let tolerance = input.tolerance.unwrap_or(EXCHANGE_TOLERANCE);
    if !(tolerance.is_finite() && tolerance > 0.0 && tolerance <= 10.0) {
        return refused(
            codes::INVALID_TOLERANCE,
            format!("Tolerans {tolerance} geçerli değil: 0'dan büyük, en çok 10 metre olmalı."),
            Some("tolerance".into()),
        );
    }
    for (i, id) in input.layers.iter().enumerate() {
        if doc.layers().get(id).is_none() {
            return refused(codes::LAYER_NOT_FOUND, format!("“{id}” kimlikli katman çizimde yok."), Some(format!("layers[{i}]")));
        }
    }
    if op == CallOp::Validate {
        return CommandResult::Completed { output: Value::Null, warnings: Vec::new() };
    }
    // The process, the time and a count: two checks at once (a clock of microseconds) never share a folder.
    static COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let folder = std::env::temp_dir().join(format!("kentos-devir-{}-{stamp}-{n}", std::process::id()));
    if let Err(e) = std::fs::create_dir_all(&folder) {
        return refused(
            codes::FILE_UNWRITABLE,
            format!("Geçici klasör {} açılamadı: {e}. Sistemin geçici klasörüne yazılabildiğini denetleyin.", folder.display()),
            None,
        );
    }
    let scratch = Scratch(folder);
    let file = scratch.0.join(format!("devir.{}", input.format.extension()));
    let path = file.display().to_string();
    let (export, import) = input.format.commands();
    // The writer on a copy: its call is the copy's, the open drawing keeps no row of it.
    let mut copy = doc.clone();
    let mut asked = json!({"path": path, "overwrite": true});
    if !input.layers.is_empty() {
        asked["layers"] = json!(input.layers);
    }
    let written = match crate::file_commands::run(&mut copy, export, CallOp::Execute, asked) {
        Ok(v) => v,
        Err(why) => return refused(codes::EXCHANGE_UNREADABLE, format!("{export} çalışmadı: {why}"), None),
    };
    if written["status"] != "completed" {
        let e = &written["error"];
        return refused(
            e["code"].as_str().unwrap_or(codes::EXCHANGE_UNREADABLE),
            format!("Yazıcı reddetti: {}", e["message"].as_str().unwrap_or("")),
            Some(format!("export.{}", e["path"].as_str().unwrap_or(""))),
        );
    }
    // The reader into an empty copy: the settings and layers, no objects.
    let Ok(mut empty) = Document::from_snapshot_v2(doc.to_snapshot_v2_of(|_| false)) else {
        return refused(codes::EXCHANGE_UNREADABLE, "Çizimin boş kopyası kurulamadı.".into(), None);
    };
    let read = {
        let mut cx = ExecutionContext::new(&mut empty);
        kentos_native_application::registry::run(&mut cx, import, CallOp::Execute, json!({"path": path}))
    };
    let read = match read {
        Ok(v) if v["status"] == "completed" => v,
        Ok(v) => {
            return refused(
                codes::EXCHANGE_UNREADABLE,
                format!(
                    "KentOS'un {} okuyucusu KentOS'un yazdığı dosyayı reddetti: {}",
                    input.format.title(),
                    v["error"]["message"].as_str().unwrap_or("")
                ),
                None,
            );
        }
        Err(e) => return refused(codes::EXCHANGE_UNREADABLE, format!("{import} çalışmadı: {e:?}"), None),
    };
    let output = compare(doc, &empty, input, tolerance, messages(&written, "warnings"), messages(&read, "warnings"));
    let warnings: Vec<CommandWarning> = Vec::new();
    CommandResult::Completed { output: serde_json::to_value(output).unwrap_or(Value::Null), warnings }
}

/// The objects the check takes: those of the layers asked (a group gives its layers), else every object.
fn originals(doc: &Document, layers: &[String]) -> Vec<Slot> {
    let ids: Vec<String> = layers.iter().flat_map(|l| doc.layers().leaves_of(l).into_iter().map(|n| n.id.clone())).collect();
    doc.entities()
        .filter(|e| layers.is_empty() || ids.contains(&e.base().layer_id))
        .map(|e| Slot(e.base().id))
        .collect()
}

fn centre(e: &Entity) -> Vec2 {
    let b = entity_bounds(&shape(e));
    Vec2 { x: (b.min_x + b.max_x) / 2.0, y: (b.min_y + b.max_y) / 2.0 }
}

fn compare(
    doc: &Document,
    back: &Document,
    input: &ExchangeCheck,
    tolerance: f64,
    writer: Vec<String>,
    reader: Vec<String>,
) -> ExchangeChecked {
    let olds = originals(doc, &input.layers);
    let news: Vec<Slot> = back.entities().map(|e| Slot(e.base().id)).collect();
    let uid = |d: &Document, s: Slot| d.uid(s).map(|u| u.to_string()).unwrap_or_default();
    let mut pair: Vec<Option<Slot>> = vec![None; olds.len()];
    let mut taken = vec![false; news.len()];
    // By persistent id.
    let index: HashMap<String, usize> = olds.iter().enumerate().map(|(i, s)| (uid(doc, *s), i)).collect();
    for (k, s) in news.iter().enumerate() {
        if let Some(&i) = index.get(&uid(back, *s))
            && pair[i].is_none()
        {
            pair[i] = Some(*s);
            taken[k] = true;
        }
    }
    // By class and the nearest centre, then (a circle read back as a line, an area's hole as a line of its own) by the
    // nearest centre of any class; ties to the first original.
    for any_class in [false, true] {
        for (k, s) in news.iter().enumerate() {
            if taken[k] {
                continue;
            }
            let Some(e) = back.get(*s) else { continue };
            let (c, at) = (class(e), centre(e));
            let best = olds
                .iter()
                .enumerate()
                .filter(|(i, o)| {
                    // A text, a block, a dimension … only with one of its own kind.
                    pair[*i].is_none()
                        && doc.get(**o).is_some_and(|o| match (class(o), c) {
                            ("other", _) | (_, "other") => !any_class && o.kind() == e.kind(),
                            (a, b) => any_class || a == b,
                        })
                })
                .map(|(i, o)| (i, doc.get(*o).map_or(f64::INFINITY, |o| dist(centre(o), at))))
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            if let Some((i, _)) = best {
                pair[i] = Some(*s);
                taken[k] = true;
            }
        }
    }
    let mut losses = Losses { found: BTreeMap::new() };
    let mut numeric = ExchangeNumeric::default();
    let mut not_written: BTreeMap<String, u32> = BTreeMap::new();
    let bigger = |slot: &mut Option<f64>, v: f64| {
        if v.is_finite() && slot.is_none_or(|s| v > s) {
            *slot = Some(v);
        }
    };
    let mut matched = 0;
    for (i, o_slot) in olds.iter().enumerate() {
        let Some(o) = doc.get(*o_slot) else { continue };
        let ouid = uid(doc, *o_slot);
        let Some(n) = pair[i].and_then(|s| back.get(s)) else {
            *not_written.entry(o.kind().to_owned()).or_default() += 1;
            losses.add("notWritten", o.kind(), &ouid);
            continue;
        };
        matched += 1;
        if o.kind() != n.kind() {
            losses.add("kindChanged", format!("{} → {}", o.kind(), n.kind()), &ouid);
        }
        if curved(o) && !curved(n) {
            losses.add("chorded", o.kind(), &ouid);
        }
        // Numeric: the places read back to the original's geometry.
        let offset = places(n).into_iter().map(|p| to_original(o, p)).fold(0.0, f64::max);
        if numeric.offset.is_none_or(|m| offset > m) {
            numeric.offset = Some(offset);
            numeric.offset_uid = Some(ouid.clone());
        }
        if offset > tolerance {
            numeric.above += 1;
        }
        let ((zo, mo), (zn, mn)) = (heights(o), heights(n));
        let some = |v: &[Option<f64>]| v.iter().any(Option::is_some);
        if some(&zo) && !some(&zn) {
            losses.add("zLost", "", &ouid);
        } else if !some(&zo) && some(&zn) {
            losses.add("zAdded", "", &ouid);
        }
        if some(&mo) && !some(&mn) {
            losses.add("mLost", "", &ouid);
        }
        if zo.len() == zn.len() {
            for (a, b) in zo.iter().zip(&zn) {
                if let (Some(a), Some(b)) = (a, b) {
                    bigger(&mut numeric.height, (a - b).abs());
                }
            }
            for (a, b) in mo.iter().zip(&mn) {
                if let (Some(a), Some(b)) = (a, b) {
                    bigger(&mut numeric.measure, (a - b).abs());
                }
            }
        }
        let (so, sn) = (shape(o), shape(n));
        if let (Some(a), Some(b)) = (entity_length(&so), entity_length(&sn)) {
            bigger(&mut numeric.length, (a - b).abs());
        }
        if class(o) == "area"
            && let (Some(a), Some(b)) = (entity_area(&so), entity_area(&sn))
        {
            bigger(&mut numeric.area, (a.abs() - b.abs()).abs());
        }
        // Attributes, style and identity.
        let (ao, an) = (&o.base().attrs, &n.base().attrs);
        for (k, v) in ao {
            match an.get(k) {
                None => losses.add("attrLost", k.clone(), &ouid),
                Some(w) if !same_value(v, w) => losses.add("attrChanged", k.clone(), &ouid),
                _ => {}
            }
        }
        for k in an.keys().filter(|k| !ao.contains_key(*k)) {
            losses.add("attrAdded", k.clone(), &ouid);
        }
        let (bo, bn) = (o.base(), n.base());
        if bo.color.is_some() && bo.color != bn.color {
            losses.add("styleLost", "color", &ouid);
        }
        if bo.line_weight.is_some() && bo.line_weight != bn.line_weight {
            losses.add("styleLost", "lineWeight", &ouid);
        }
        if bo.symbol.is_some() && bo.symbol != bn.symbol {
            losses.add("styleLost", "symbol", &ouid);
        }
        if uid(back, pair[i].unwrap_or(Slot(0))) != ouid {
            losses.add("identityLost", "", &ouid);
        }
    }
    for (k, s) in news.iter().enumerate() {
        if !taken[k]
            && let Some(e) = back.get(*s)
        {
            losses.add("extra", e.kind(), "");
        }
    }
    ExchangeChecked {
        format: input.format,
        tolerance,
        objects: olds.len().min(u32::MAX as usize) as u32,
        matched,
        not_written: not_written.into_iter().map(|(kind, count)| ExchangeKindCount { kind, count }).collect(),
        numeric,
        losses: losses
            .found
            .into_iter()
            .map(|((order, detail), (count, uids))| ExchangeLoss {
                kind: EXCHANGE_LOSSES.get(order).copied().unwrap_or("other").to_owned(),
                detail: (!detail.is_empty()).then_some(detail),
                count,
                uids,
            })
            .collect(),
        writer,
        reader,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{DocumentSnapshotV2, ExchangeFormat};

    fn drawing() -> Document {
        let text = include_str!("../../../../fixtures/exchange/v1/devir.json");
        let snapshot: DocumentSnapshotV2 = serde_json::from_str(text).expect("the drawing");
        Document::from_snapshot_v2(snapshot).expect("opens")
    }

    fn checked(format: ExchangeFormat) -> ExchangeChecked {
        let input = ExchangeCheck { format, layers: Vec::new(), tolerance: None };
        match run(&drawing(), &input, CallOp::Execute) {
            CommandResult::Completed { output, .. } => serde_json::from_value(output).expect("the answer"),
            other => panic!("{other:?}"),
        }
    }

    fn loss(r: &ExchangeChecked, kind: &str, detail: Option<&str>) -> u32 {
        r.losses.iter().find(|l| l.kind == kind && l.detail.as_deref() == detail).map_or(0, |l| l.count)
    }

    /// GeoPackage keeps the ids, the arcs (as circular strings), Z and M: the text is not written, the circle comes
    /// back an area, the colour is lost; every vertex within a nanometre.
    #[test]
    fn a_geopackage_trip_keeps_ids_arcs_heights_and_measures() {
        let r = checked(ExchangeFormat::Gpkg);
        assert_eq!((r.objects, r.matched), (8, 7));
        assert_eq!(r.not_written, [ExchangeKindCount { kind: "text".into(), count: 1 }]);
        assert_eq!(loss(&r, "kindChanged", Some("circle → polygon")), 1);
        assert_eq!(loss(&r, "styleLost", Some("color")), 1);
        for k in ["chorded", "zLost", "mLost", "identityLost", "attrLost", "extra"] {
            assert!(r.losses.iter().all(|l| l.kind != k), "{k}: {:?}", r.losses);
        }
        assert!(r.numeric.offset.is_some_and(|o| o < 1e-9) && r.numeric.above == 0, "{:?}", r.numeric);
    }

    /// DXF has no M and no ids of its own here; the parcel's hole is a polyline of its own.
    #[test]
    fn a_dxf_trip_loses_measures_and_splits_the_hole() {
        let r = checked(ExchangeFormat::Dxf);
        assert!(r.not_written.is_empty(), "{:?}", r.not_written);
        assert_eq!(loss(&r, "mLost", None), 1);
        assert_eq!(loss(&r, "identityLost", None), 8);
        assert_eq!(loss(&r, "kindChanged", Some("polygon → polyline")), 1);
        assert_eq!(loss(&r, "extra", Some("polyline")), 1);
        assert_eq!(loss(&r, "chorded", Some("polyline")), 0, "bulges kept");
    }

    /// GeoParquet writes arcs as chords; the manhole's corners stay on its circle (the writer's centre is worked from
    /// a corner, not from the map's coordinates).
    #[test]
    fn a_geoparquet_trip_chords_the_arcs_within_the_tolerance() {
        let r = checked(ExchangeFormat::Parquet);
        assert_eq!(loss(&r, "chorded", Some("circle")), 1);
        assert_eq!(loss(&r, "chorded", Some("polyline")), 1);
        assert!(r.numeric.offset.is_some_and(|o| o < 1e-6), "{:?}", r.numeric);
    }

    #[test]
    fn the_refusals_and_the_drawing_untouched() {
        let doc = drawing();
        let before = doc.revision();
        let bad = ExchangeCheck { format: ExchangeFormat::Gpkg, layers: Vec::new(), tolerance: Some(0.0) };
        assert!(matches!(run(&doc, &bad, CallOp::Execute), CommandResult::Failed { error } if error.code == "invalid_tolerance"));
        let bad = ExchangeCheck { format: ExchangeFormat::Gpkg, layers: vec!["yok".into()], tolerance: None };
        assert!(matches!(run(&doc, &bad, CallOp::Execute),
            CommandResult::Failed { error } if error.code == "layer_not_found" && error.path.as_deref() == Some("layers[0]")));
        let only = ExchangeCheck { format: ExchangeFormat::Shp, layers: vec!["baca".into()], tolerance: None };
        let CommandResult::Completed { output, .. } = run(&doc, &only, CallOp::Execute) else { panic!() };
        assert_eq!(output["objects"], 1);
        assert_eq!(doc.revision(), before);
    }
}
