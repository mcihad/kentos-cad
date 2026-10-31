//! A room's interior elevations drawn from the model (docs/adr/0799, ADV-06f): what `cad.elevations.edit` writes and
//! `cad.elevations.query` compares with what is written. Only what the model holds is drawn:
//!
//! - A side is a straight edge of the space's outer rings (numbered from 0, part by part); one looks at it from inside
//!   the room, so its drawing runs from the edge's end to its start when the ring turns counter-clockwise (from its start
//!   to its end when clockwise). A curved side is not unrolled (`curvedSide`).
//! - The walls on the space's level whose left or right face lies on the side (parallel, within 1 mm) cover the
//!   stretch their straight segments' ends project onto; a stretch no face covers is open (`open`).
//! - Its height is the space's (floor to ceiling); else the lowest of its walls' heights (a wall's own, else its
//!   level's); else not known: only the floor line is drawn. A wall lower than it draws its top across its stretch.
//! - An opening of one of those walls draws its jambs where its points on the side's face project, from its sill (absent:
//!   the floor) to its head; without a head its jambs are dashed up to the side's height (`headUnknown`). A niche is seen
//!   only from its own face; a flue never (`flue`); an opening not where its wall says is not drawn (`brokenOpening`).
//! - What stands in the room and has no height in the model is listed, not drawn: holes, columns, stairs, ramps, blocks.
//!
//! Each side is a frame (or a floor line), its walls' tops, its openings, their marks, its title and two dimensions,
//! drawn left to right from the origin, `gap` apart. Each object has a role (`2/opening/<uid>`) the record keeps with
//! its id, so drawing again keeps the same objects.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandWarning, ELEVATION_TOOL, ElevationLeft, ElevationLeftOut, ElevationRecord, ElevationSide,
    ElevationState, Entity, LayerDerivation, OpeningForm, OpeningSide, SpaceMethod, Vec2, WallJustify,
};
use kentos_domain::{Document, Slot, Uuid};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::checks::{Stop, error};
use crate::codes;

/// How far a wall's face may lie from a side and still be its face, metres.
const ON_FACE: f64 = 1e-3;
/// How parallel a wall's segment and a side must be (the sine of the angle between them).
const PARALLEL: f64 = 1e-6;
/// The paper height of the elevation's words and dimensions, metres (2.5 mm).
const PAPER: f64 = 0.0025;
/// How far right of the space's box the first side goes when no origin is given, metres.
const BESIDE: f64 = 3.0;
/// The space between two sides when none is given, metres.
pub const GAP: f64 = 1.0;

/// What the elevation of a space draws: its sides, its objects by role (in order), what it left out and its warnings.
#[derive(Clone, Debug)]
pub struct Drawn {
    pub sides: Vec<ElevationSide>,
    pub items: Vec<(String, Entity)>,
    pub left_out: Vec<ElevationLeftOut>,
    pub warnings: Vec<CommandWarning>,
    pub at: Vec2,
    pub gap: f64,
    /// The space's number and name (“101 Salon”), or “Mahal”.
    pub label: String,
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The side's letter: A … Z, then AA, AB …
pub fn letter(k: u32) -> String {
    let mut n = k as u64 + 1;
    let mut out = Vec::new();
    while n > 0 {
        let r = ((n - 1) % 26) as u8;
        out.push((b'A' + r) as char);
        n = (n - 1) / 26;
    }
    out.iter().rev().collect()
}

/// A space's number and name, or “Mahal”.
pub fn label_of(s: &kentos_commands::SpaceEntity) -> String {
    let words: Vec<&str> = [s.space.number.as_deref(), s.space.name.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        "Mahal".into()
    } else {
        words.join(" ")
    }
}

/// Even-odd: whether `p` is inside the rings (their chords).
fn inside(rings: &[&[Vec2]], p: Vec2) -> bool {
    let mut odd = false;
    for r in rings {
        let n = r.len();
        for i in 0..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                odd = !odd;
            }
        }
    }
    odd
}

fn pt(x: f64, y: f64) -> Value {
    json!({ "x": x, "y": y })
}

/// An object of the elevation from its JSON: the layer's, with no id yet.
fn entity(layer: &str, mut v: Value) -> Entity {
    v["id"] = json!(0);
    v["layerId"] = json!(layer);
    v["attrs"] = json!({});
    serde_json::from_value(v).expect("an elevation's object is a valid entity")
}

/// A wall segment's face lying on a side.
struct Face {
    wall: String,
    segment: usize,
    left: bool,
    lo: f64,
    hi: f64,
    height: Option<f64>,
}

/// The elevation of the space `uid`'s sides (`sides`: all when empty), from `at` (none: beside the space), `gap`
/// apart, its objects on `layer`. Refusals: the space's id, its method, the sides.
pub(crate) fn draw(
    doc: &Document,
    uid: &str,
    sides: &[u32],
    at: Option<Vec2>,
    gap: Option<f64>,
    layer: &str,
) -> Result<Drawn, Stop> {
    let space_path = "operation.space";
    if !crate::checks::is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
            space_path,
        ));
    }
    let Some(e) = Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
    else {
        return Err(fail(
            codes::ENTITY_NOT_FOUND,
            format!("“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir."),
            space_path,
        ));
    };
    let Entity::Space(s) = e else {
        return Err(fail(
            codes::NOT_A_SPACE,
            "Nesne mahal değil; iç görünüş bir mahalden üretilir.".into(),
            space_path,
        ));
    };
    if s.space.method != SpaceMethod::Net {
        return Err(fail(
            codes::ELEVATION_SPACE_NOT_NET,
            "Mahalin sınırı duvarların iç yüzleri değil (eksen ya da brüt); iç görünüş yalnız net sınırlı mahalden üretilir. Mahalin yöntemini Net yapın.".into(),
            space_path,
        ));
    }
    let settings = doc.settings();
    let h = PAPER * settings.plot_scale;
    let level = s.space.arch.level.clone();
    let level_height = level.as_ref().and_then(|id| {
        settings
            .arch
            .as_ref()?
            .levels
            .iter()
            .find(|l| &l.id == id)?
            .height
    });
    // The sides: every outer ring's edges, part by part.
    struct Edge {
        a: Vec2,
        b: Vec2,
        bulge: f64,
        ccw: bool,
    }
    let mut edges = Vec::new();
    for part in &s.space.parts {
        let r = &part.outer.pts;
        let n = r.len();
        let mut twice = 0.0;
        for i in 0..n {
            let (p, q) = (r[i], r[(i + 1) % n]);
            twice += p.x * q.y - q.x * p.y;
        }
        for i in 0..n {
            let bulge = part
                .outer
                .bulges
                .as_ref()
                .and_then(|b| b.get(i).copied())
                .unwrap_or(0.0);
            edges.push(Edge {
                a: r[i],
                b: r[(i + 1) % n],
                bulge,
                ccw: twice > 0.0,
            });
        }
    }
    let mut chosen: Vec<u32> = Vec::new();
    if sides.is_empty() {
        chosen = (0..edges.len() as u32).collect();
    } else {
        for (j, &k) in sides.iter().enumerate() {
            if k as usize >= edges.len() {
                return Err(fail(
                    codes::ELEVATION_NO_SIDES,
                    format!(
                        "Mahalin {k} numaralı yanı yok; yanlar 0'dan {} numaraya kadardır.",
                        edges.len().saturating_sub(1)
                    ),
                    &format!("operation.sides[{j}]"),
                ));
            }
            if chosen.contains(&k) {
                return Err(fail(
                    codes::INVALID_INPUT,
                    format!("{k} numaralı yan iki kez verildi; her yanı bir kez verin."),
                    &format!("operation.sides[{j}]"),
                ));
            }
            chosen.push(k);
        }
        chosen.sort_unstable();
    }
    if chosen.iter().all(|&k| edges[k as usize].bulge != 0.0) {
        return Err(fail(
            codes::ELEVATION_NO_SIDES,
            "Çizilecek düz bir yan yok: istenen yanların hepsi yaylı. Yaylı duvarın iç görünüşü açılmaz.".into(),
            if sides.is_empty() { space_path } else { "operation.sides" },
        ));
    }
    if let Some(g) = gap {
        crate::checks::finite(g, "Yanların arası", "Sonlu bir sayı verin.", "operation.gap")?;
        if !(g > 0.0 && g <= 100.0) {
            return Err(fail(
                codes::INVALID_INPUT,
                "Yanların arası sıfırdan büyük, en çok 100 m olmalı.".into(),
                "operation.gap",
            ));
        }
    }
    if let Some(p) = at {
        crate::checks::point(p, "İç görünüşün başlangıcının", "operation.at")?;
    }
    let gap = gap.unwrap_or(GAP);
    let at = at.unwrap_or_else(|| {
        let mut lo = Vec2 { x: f64::INFINITY, y: f64::INFINITY };
        let mut hi = f64::NEG_INFINITY;
        for p in s.space.parts.iter().flat_map(|p| &p.outer.pts) {
            hi = hi.max(p.x);
            lo.x = lo.x.min(p.x);
            lo.y = lo.y.min(p.y);
        }
        Vec2 { x: hi + BESIDE, y: lo.y }
    });
    let label = label_of(s);
    let walls: Vec<(String, &kentos_commands::WallEntity)> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Wall(w) if w.wall.arch.level == level => {
                Some((doc.uid(Slot(e.base().id))?.to_string(), w))
            }
            _ => None,
        })
        .collect();
    let openings: Vec<(String, &kentos_commands::OpeningEntity)> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Opening(o) => Some((doc.uid(Slot(e.base().id))?.to_string(), o)),
            _ => None,
        })
        .collect();

    let mut out = Drawn {
        sides: Vec::new(),
        items: Vec::new(),
        left_out: Vec::new(),
        warnings: Vec::new(),
        at,
        gap,
        label: label.clone(),
    };
    let (mut curved, mut open, mut unknown, mut headless) = (0u32, 0u32, 0u32, 0u32);
    let mut ox = at.x;
    let oy = at.y;
    for &k in &chosen {
        let edge = &edges[k as usize];
        if edge.bulge != 0.0 {
            curved += 1;
            out.left_out.push(ElevationLeftOut {
                why: ElevationLeft::CurvedSide,
                side: Some(k),
                uid: None,
            });
            continue;
        }
        let (a, b) = (edge.a, edge.b);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 1e-9 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let ccw = edge.ccw;
        let xof = |p: Vec2| {
            if ccw {
                (b.x - p.x) * ux + (b.y - p.y) * uy
            } else {
                (p.x - a.x) * ux + (p.y - a.y) * uy
            }
        };
        let w = |x: f64, y: f64| pt(ox + x, oy + y);
        // The faces on the side.
        let mut faces: Vec<Face> = Vec::new();
        for (wuid, wall) in &walls {
            let f = &wall.wall;
            let pts = &f.pts;
            let n = pts.len();
            let count = if f.closed { n } else { n.saturating_sub(1) };
            let lf = f.offset
                + match f.justify {
                    WallJustify::Left => 0.0,
                    WallJustify::Center => f.width / 2.0,
                    WallJustify::Right => f.width,
                };
            let rf = lf - f.width;
            for j in 0..count {
                let bulge = f.bulges.as_ref().and_then(|b| b.get(j).copied()).unwrap_or(0.0);
                if bulge != 0.0 {
                    continue;
                }
                let (p, q) = (pts[j], pts[(j + 1) % n]);
                let (vx, vy) = (q.x - p.x, q.y - p.y);
                let vl = (vx * vx + vy * vy).sqrt();
                if vl <= 1e-9 {
                    continue;
                }
                let (vx, vy) = (vx / vl, vy / vl);
                if (ux * vy - uy * vx).abs() > PARALLEL {
                    continue;
                }
                let e = vx * (a.y - p.y) - vy * (a.x - p.x);
                let left = if (e - lf).abs() <= ON_FACE {
                    true
                } else if (e - rf).abs() <= ON_FACE {
                    false
                } else {
                    continue;
                };
                let (x0, x1) = (xof(p), xof(q));
                let lo = x0.min(x1).max(0.0);
                let hi = x0.max(x1).min(len);
                if hi - lo <= ON_FACE {
                    continue;
                }
                faces.push(Face {
                    wall: wuid.clone(),
                    segment: j,
                    left,
                    lo,
                    hi,
                    height: f.height.or(level_height),
                });
            }
        }
        let mut side_walls: Vec<String> = Vec::new();
        for f in &faces {
            if !side_walls.contains(&f.wall) {
                side_walls.push(f.wall.clone());
            }
        }
        let height = s.space.height.or_else(|| {
            faces
                .iter()
                .filter_map(|f| f.height)
                .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.min(x))))
        });
        // The stretches no face covers.
        let mut spans: Vec<(f64, f64)> = faces.iter().map(|f| (f.lo, f.hi)).collect();
        spans.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.total_cmp(&q.1)));
        let mut gaps = Vec::new();
        let mut reach = 0.0;
        for (lo, hi) in &spans {
            if *lo - reach > ON_FACE {
                gaps.push([reach, *lo]);
            }
            reach = reach.max(*hi);
        }
        if len - reach > ON_FACE {
            gaps.push([reach, len]);
        }
        if !gaps.is_empty() {
            open += 1;
        }
        let role = |what: &str| format!("{k}/{what}");
        let mut items: Vec<(String, Entity)> = Vec::new();
        match height {
            Some(hh) => items.push((
                role("frame"),
                entity(
                    layer,
                    json!({"kind": "polyline", "pts": [w(0.0, 0.0), w(len, 0.0), w(len, hh), w(0.0, hh)], "closed": true}),
                ),
            )),
            None => {
                unknown += 1;
                items.push((
                    role("floor"),
                    entity(layer, json!({"kind": "line", "a": w(0.0, 0.0), "b": w(len, 0.0)})),
                ));
            }
        }
        if let Some(hh) = height {
            for f in &faces {
                if let Some(fh) = f.height
                    && fh < hh - 1e-9
                {
                    items.push((
                        role(&format!("top/{}/{}", f.wall, f.segment)),
                        entity(layer, json!({"kind": "line", "a": w(f.lo, fh), "b": w(f.hi, fh)})),
                    ));
                }
            }
        }
        // The openings of its walls seen on its face.
        struct Seen {
            x0: f64,
            x1: f64,
            uid: String,
            sill: f64,
            head: Option<f64>,
            number: Option<String>,
        }
        let mut seen: Vec<Seen> = Vec::new();
        for (ouid, o) in &openings {
            let f = &o.opening;
            if !side_walls.contains(&f.host) {
                continue;
            }
            if f.broken.is_some() {
                out.left_out.push(ElevationLeftOut {
                    why: ElevationLeft::BrokenOpening,
                    side: Some(k),
                    uid: Some(ouid.clone()),
                });
                continue;
            }
            if f.form == OpeningForm::Flue {
                out.left_out.push(ElevationLeftOut {
                    why: ElevationLeft::Flue,
                    side: Some(k),
                    uid: Some(ouid.clone()),
                });
                continue;
            }
            if f.place.len() != 4 {
                continue;
            }
            let on = |p: Vec2| (ux * (p.y - a.y) - uy * (p.x - a.x)).abs() <= ON_FACE;
            let mut found = None;
            for left in [true, false] {
                if !faces.iter().any(|x| x.wall == f.host && x.left == left) {
                    continue;
                }
                let (p, q) = if left {
                    (f.place[0], f.place[2])
                } else {
                    (f.place[1], f.place[3])
                };
                if on(p) && on(q) {
                    found = Some((left, p, q));
                    break;
                }
            }
            let Some((left, p, q)) = found else {
                continue;
            };
            if f.form == OpeningForm::Niche && (f.side == OpeningSide::Left) != left {
                continue;
            }
            let (xa, xb) = (xof(p), xof(q));
            let x0 = xa.min(xb).max(0.0);
            let x1 = xa.max(xb).min(len);
            if x1 - x0 <= 1e-9 {
                continue;
            }
            if f.head.is_none() {
                headless += 1;
                out.left_out.push(ElevationLeftOut {
                    why: ElevationLeft::HeadUnknown,
                    side: Some(k),
                    uid: Some(ouid.clone()),
                });
                if height.is_none() {
                    continue;
                }
            }
            seen.push(Seen {
                x0,
                x1,
                uid: ouid.clone(),
                sill: f.sill.unwrap_or(0.0),
                head: f.head,
                number: f.number.clone().filter(|n| !n.trim().is_empty()),
            });
        }
        seen.sort_by(|p, q| p.x0.total_cmp(&q.x0).then_with(|| p.uid.cmp(&q.uid)));
        for o in &seen {
            let shape = match o.head {
                Some(top) => json!({"kind": "polyline",
                    "pts": [w(o.x0, o.sill), w(o.x1, o.sill), w(o.x1, top), w(o.x0, top)], "closed": true}),
                None => {
                    let top = height.unwrap_or(o.sill);
                    json!({"kind": "polyline", "lineType": "dashed",
                        "pts": [w(o.x0, top), w(o.x0, o.sill), w(o.x1, o.sill), w(o.x1, top)]})
                }
            };
            items.push((role(&format!("opening/{}", o.uid)), entity(layer, shape)));
        }
        for o in &seen {
            if let (Some(n), Some(top)) = (&o.number, o.head) {
                items.push((
                    role(&format!("mark/{}", o.uid)),
                    entity(
                        layer,
                        json!({"kind": "text", "p": w((o.x0 + o.x1) / 2.0, (o.sill + top) / 2.0),
                            "text": n, "height": h, "rotation": 0, "align": "middleCenter"}),
                    ),
                ));
            }
        }
        items.push((
            role("title"),
            entity(
                layer,
                json!({"kind": "text", "p": w(len / 2.0, -4.0 * h), "text": format!("{label} — {}", letter(k)),
                    "height": h, "rotation": 0, "align": "topCenter"}),
            ),
        ));
        items.push((
            role("width"),
            entity(
                layer,
                json!({"kind": "dimension", "a": w(0.0, 0.0), "b": w(len, 0.0), "offset": -2.0 * h,
                    "height": h, "style": "aligned"}),
            ),
        ));
        if let Some(hh) = height {
            items.push((
                role("height"),
                entity(
                    layer,
                    json!({"kind": "dimension", "a": w(len, 0.0), "b": w(len, hh), "offset": -2.0 * h,
                        "height": h, "style": "aligned"}),
                ),
            ));
        }
        out.sides.push(ElevationSide {
            side: k,
            letter: letter(k),
            length: len,
            height,
            origin: Vec2 { x: ox, y: oy },
            walls: side_walls,
            openings: seen.iter().map(|o| o.uid.clone()).collect(),
            open: gaps,
        });
        out.items.extend(items);
        ox = ox + len + gap;
    }
    // What stands in the room and has no height in the model.
    for part in &s.space.parts {
        for _ in &part.holes {
            out.left_out.push(ElevationLeftOut {
                why: ElevationLeft::Hole,
                side: None,
                uid: None,
            });
        }
    }
    let rings: Vec<&[Vec2]> = s.space.parts.iter().map(|p| p.outer.pts.as_slice()).collect();
    for e in doc.entities() {
        let (why, lvl, points): (ElevationLeft, &Option<String>, Vec<Vec2>) = match e {
            Entity::Column(c) => (ElevationLeft::Column, &c.column.arch.level, vec![c.column.at]),
            Entity::Stair(x) => (ElevationLeft::Stair, &x.stair.arch.level, x.stair.pts.clone()),
            Entity::Ramp(x) => (ElevationLeft::Ramp, &x.ramp.arch.level, x.ramp.pts.clone()),
            Entity::Insert(i) => (ElevationLeft::Fixture, &i.arch.level, vec![i.p]),
            _ => continue,
        };
        if *lvl != level || !points.iter().any(|p| inside(&rings, *p)) {
            continue;
        }
        out.left_out.push(ElevationLeftOut {
            why,
            side: None,
            uid: doc.uid(Slot(e.base().id)).map(|u| u.to_string()),
        });
    }
    let warn = |code: &str, message: String| CommandWarning {
        code: code.into(),
        message,
        path: None,
    };
    if curved > 0 {
        out.warnings.push(warn(
            codes::ELEVATION_CURVED_SIDE,
            format!("{curved} yaylı yan çizilmedi; yaylı duvarın iç görünüşü açılmaz, yalnız düz yanlar çizildi."),
        ));
    }
    if open > 0 {
        out.warnings.push(warn(
            codes::ELEVATION_SIDE_OPEN,
            format!("{open} yanın bir kısmında duvar yüzü yok (ayırıcı ya da boşluk); orada duvar çizilmedi."),
        ));
    }
    if unknown > 0 {
        out.warnings.push(warn(
            codes::ELEVATION_HEIGHT_UNKNOWN,
            format!("{unknown} yanın yüksekliği bilinmiyor (mahalin, duvarın ve katın yüksekliği yok); yalnız döşeme çizgisi çizildi."),
        ));
    }
    if headless > 0 {
        out.warnings.push(warn(
            codes::ELEVATION_HEAD_UNKNOWN,
            format!("{headless} açıklığın üst kotu yok; pervazları kesikli çizildi, üstleri çizilmedi."),
        ));
    }
    let left = out
        .left_out
        .iter()
        .filter(|l| !matches!(l.why, ElevationLeft::CurvedSide | ElevationLeft::HeadUnknown))
        .count();
    if left > 0 {
        out.warnings.push(warn(
            codes::ELEVATION_NOT_DRAWN,
            format!("Odadaki {left} şey çizilmedi (kolon, merdiven, rampa, donatı, baca, boşluk, kopuk açıklık); leftOut listesinde."),
        ));
    }
    Ok(out)
}

/// The digest of what an elevation draws: its objects by role, without their ids.
pub fn digest(items: &[(String, Entity)]) -> String {
    let text = serde_json::to_string(items).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The elevation record of a layer: its derivation names the elevation tool and its parameters read.
pub fn record_of(derivation: &LayerDerivation) -> Option<ElevationRecord> {
    if derivation.tool != ELEVATION_TOOL {
        return None;
    }
    serde_json::from_value(Value::Object(
        derivation.parameters.clone().into_iter().collect(),
    ))
    .ok()
}

/// The elevation record on the layer `id`, with the layer's derivation.
pub fn on_layer(doc: &Document, id: &str) -> Option<(LayerDerivation, ElevationRecord)> {
    let d = doc.layers().get(id)?.derivation.clone()?;
    let r = record_of(&d)?;
    Some((d, r))
}

/// The parameters a record is written as.
pub fn parameters(r: &ElevationRecord) -> BTreeMap<String, Value> {
    match serde_json::to_value(r) {
        Ok(Value::Object(m)) => m.into_iter().collect(),
        _ => BTreeMap::new(),
    }
}

/// An entity as drawn, its id cleared: what a role is compared by.
pub fn bare(e: &Entity) -> Entity {
    let mut e = e.clone();
    e.base_mut().id = 0;
    e
}

/// A layer's elevation as it stands: its state, the members deleted and those changed by hand (only while the model's
/// drawing is unchanged), and what drawing it again gives (none when its space is gone).
pub struct Standing {
    pub state: ElevationState,
    pub missing: Vec<String>,
    pub edited: Vec<String>,
    pub drawn: Option<Drawn>,
}

pub fn standing(doc: &Document, layer: &str, r: &ElevationRecord) -> Standing {
    let missing: Vec<String> = r
        .members
        .values()
        .filter(|u| {
            Uuid::parse_str(u)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .is_none()
        })
        .cloned()
        .collect();
    let Ok(drawn) = draw(doc, &r.space, &r.sides, Some(r.at), Some(r.gap), layer) else {
        return Standing {
            state: ElevationState::Gone,
            missing,
            edited: Vec::new(),
            drawn: None,
        };
    };
    let same = digest(&drawn.items) == r.digest;
    let mut edited = Vec::new();
    if same {
        for (role, want) in &drawn.items {
            let Some(u) = r.members.get(role) else { continue };
            let Some(now) = Uuid::parse_str(u)
                .ok()
                .and_then(|x| doc.slot_of(x))
                .and_then(|s| doc.get(s))
            else {
                continue;
            };
            if &bare(now) != want {
                edited.push(u.clone());
            }
        }
    }
    let state = if same && missing.is_empty() {
        ElevationState::Current
    } else {
        ElevationState::Stale
    };
    Standing {
        state,
        missing,
        edited,
        drawn: Some(drawn),
    }
}

/// The time now, `YYYY-MM-DDTHH:MM:SS.SSSZ` (UTC).
pub fn utc_now() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let text = kentos_geometry_core::time::write(ms as f64, false);
    let text = if text.len() == 10 {
        format!("{text}T00:00:00")
    } else {
        text
    };
    let text = if text.len() == 19 {
        format!("{text}.000")
    } else {
        text
    };
    format!("{text}Z")
}

/// Whether the elevation on a layer is stale (`cad.datasets.describe`'s word for it, docs/adr/0633 §3).
pub fn is_stale(doc: &Document, layer: &str, d: &LayerDerivation) -> Option<bool> {
    let r = record_of(d)?;
    Some(standing(doc, layer, &r).state != ElevationState::Current)
}
