//! A drawing's picture for a phone, a list or an AI agent (docs/adr/9000 §15): a KCAD document as one SVG, drawn on
//! the server so a client that has no CAD view (a mobile app, a web page) still shows what a project or a file holds.
//!
//! It is a preview, not a plot: lines, paths (bulges sampled), circles, arcs, ellipses, splines (through their
//! points), texts, blocks (their objects placed), hatches (their areas, washed), walls (their reference line as wide as
//! the wall), spaces (washed, with their names) and columns are drawn in their layer's colour on white; hidden layers
//! are left out. Dimensions, leaders, tables, pictures, point clouds and openings are not drawn: the picture says how
//! many objects it left out. Coordinates are written to a millimetre of the drawing, so the same document gives the
//! same bytes.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use kentos_contracts::{
    BlockDefinition, BlockId, DocumentSnapshotV2, Entity, LayerNode, ProjectPermission,
    ProjectStorage, Vec2, WallJustify,
};
use kentos_postgres::Db;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::Blobs;
use crate::error::{AppError, AppResult};
use crate::identity::Actor;

/// What a picture holds besides itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub svg: String,
    pub drawn: usize,
    pub skipped: usize,
}

/// A placement: `p ↦ origin + rotate(scale ⊙ mirror(p - base))`.
#[derive(Clone, Copy, Debug)]
struct Place {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Place {
    const ID: Place = Place {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    fn at(&self, p: Vec2) -> Vec2 {
        Vec2 {
            x: self.a * p.x + self.c * p.y + self.e,
            y: self.b * p.x + self.d * p.y + self.f,
        }
    }

    /// How much this placement grows a length (for widths and text heights).
    fn scale(&self) -> f64 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }

    /// This placement after `inner` (inner first).
    fn then(&self, inner: &Place) -> Place {
        Place {
            a: self.a * inner.a + self.c * inner.b,
            b: self.b * inner.a + self.d * inner.b,
            c: self.a * inner.c + self.c * inner.d,
            d: self.b * inner.c + self.d * inner.d,
            e: self.a * inner.e + self.c * inner.f + self.e,
            f: self.b * inner.e + self.d * inner.f + self.f,
        }
    }

    fn turn(&self) -> f64 {
        kentos_num::atan2(self.b, self.a)
    }
}

enum Mark {
    Stroke {
        pts: Vec<Vec2>,
        closed: bool,
        width: Option<f64>,
    },
    Fill {
        rings: Vec<Vec<Vec2>>,
    },
    Text {
        at: Vec2,
        text: String,
        height: f64,
        turn_deg: f64,
    },
    Dot {
        at: Vec2,
    },
}

struct Drawn {
    color: String,
    mark: Mark,
}

/// The visible layers and their colours: a hex colour as given, a theme token as the paper's ink.
fn layers(nodes: &[LayerNode], out: &mut BTreeMap<String, Option<String>>, shown: bool) {
    for n in nodes {
        let visible = shown && n.visible;
        let color = match n.style.color.as_str() {
            c if c.starts_with('#') && (c.len() == 7 || c.len() == 4) => c.to_owned(),
            "fg-dim" => "#64707d".to_owned(),
            "paper" => "#c8ced4".to_owned(),
            _ => "#1e2833".to_owned(),
        };
        out.insert(n.id.clone(), visible.then_some(color));
        layers(&n.children, out, visible);
    }
}

/// A path's points with each bulged segment sampled as its arc.
fn bulged(pts: &[Vec2], bulges: Option<&Vec<f64>>, closed: bool) -> Vec<Vec2> {
    let Some(bulges) = bulges else {
        return pts.to_vec();
    };
    let n = pts.len();
    let segments = if closed { n } else { n.saturating_sub(1) };
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..segments {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        out.push(p);
        let b = bulges.get(i).copied().unwrap_or(0.0);
        if b.abs() < 1e-9 || !b.is_finite() {
            continue;
        }
        // The arc from p to q whose bulge is tan(θ/4).
        let theta = 4.0 * kentos_num::atan(b);
        let chord = (kentos_num::powi(q.x - p.x, 2) + kentos_num::powi(q.y - p.y, 2)).sqrt();
        if chord < 1e-12 {
            continue;
        }
        let r = chord / (2.0 * kentos_num::sin(theta / 2.0));
        let mid = Vec2 {
            x: (p.x + q.x) / 2.0,
            y: (p.y + q.y) / 2.0,
        };
        let (ux, uy) = ((q.x - p.x) / chord, (q.y - p.y) / chord);
        let h = r * kentos_num::cos(theta / 2.0);
        let c = Vec2 {
            x: mid.x - uy * h,
            y: mid.y + ux * h,
        };
        let a0 = kentos_num::atan2(p.y - c.y, p.x - c.x);
        let steps = ((theta.abs() / (std::f64::consts::PI / 18.0)).ceil() as usize).clamp(2, 72);
        for k in 1..steps {
            let a = a0 + theta * k as f64 / steps as f64;
            out.push(Vec2 {
                x: c.x + r.abs() * kentos_num::cos(a),
                y: c.y + r.abs() * kentos_num::sin(a),
            });
        }
    }
    if !closed && let Some(last) = pts.last() {
        out.push(*last);
    }
    out
}

/// A path moved `d` to its left (negative: right), its corners mitred (a sharp one bevelled at twice `d`).
fn to_left(pts: &[Vec2], d: f64, closed: bool) -> Vec<Vec2> {
    if d.abs() < 1e-12 || pts.len() < 2 {
        return pts.to_vec();
    }
    let n = pts.len();
    let normal = |a: Vec2, b: Vec2| -> Option<(f64, f64)> {
        let len = (kentos_num::powi(b.x - a.x, 2) + kentos_num::powi(b.y - a.y, 2)).sqrt();
        (len > 1e-12).then(|| (-(b.y - a.y) / len, (b.x - a.x) / len))
    };
    (0..n)
        .map(|i| {
            let before = (i > 0 || closed)
                .then(|| normal(pts[(i + n - 1) % n], pts[i]))
                .flatten();
            let after = (i + 1 < n || closed)
                .then(|| normal(pts[i], pts[(i + 1) % n]))
                .flatten();
            let (nx, ny) = match (before, after) {
                (Some(a), Some(b)) => {
                    let (mx, my) = (a.0 + b.0, a.1 + b.1);
                    let len = (mx * mx + my * my).sqrt();
                    let cos = if len > 1e-12 {
                        (mx * a.0 + my * a.1) / len
                    } else {
                        0.0
                    };
                    if cos > 0.5 {
                        (mx / len / cos, my / len / cos)
                    } else {
                        a
                    }
                }
                (Some(a), None) | (None, Some(a)) => a,
                (None, None) => (0.0, 0.0),
            };
            Vec2 {
                x: pts[i].x + nx * d,
                y: pts[i].y + ny * d,
            }
        })
        .collect()
}

fn arc(c: Vec2, r: f64, a0: f64, a1: f64) -> Vec<Vec2> {
    let mut sweep = a1 - a0;
    while sweep <= 0.0 {
        sweep += std::f64::consts::TAU;
    }
    let steps = ((sweep / (std::f64::consts::PI / 36.0)).ceil() as usize).clamp(4, 144);
    (0..=steps)
        .map(|k| {
            let a = a0 + sweep * k as f64 / steps as f64;
            Vec2 {
                x: c.x + r * kentos_num::cos(a),
                y: c.y + r * kentos_num::sin(a),
            }
        })
        .collect()
}

struct Walk<'a> {
    layers: &'a BTreeMap<String, Option<String>>,
    blocks: BTreeMap<BlockId, &'a BlockDefinition>,
    out: Vec<Drawn>,
    skipped: usize,
}

impl Walk<'_> {
    fn color(&self, layer: &str, own: Option<&String>) -> Option<String> {
        let layer_color = self
            .layers
            .get(layer)
            .cloned()
            .unwrap_or(Some("#1e2833".into()))?;
        Some(
            own.filter(|c| c.starts_with('#') && (c.len() == 7 || c.len() == 4))
                .cloned()
                .unwrap_or(layer_color),
        )
    }

    fn push(&mut self, color: &str, place: &Place, mark: Mark) {
        let mark = match mark {
            Mark::Stroke { pts, closed, width } => Mark::Stroke {
                pts: pts.into_iter().map(|p| place.at(p)).collect(),
                closed,
                width: width.map(|w| w * place.scale()),
            },
            Mark::Fill { rings } => Mark::Fill {
                rings: rings
                    .into_iter()
                    .map(|r| r.into_iter().map(|p| place.at(p)).collect())
                    .collect(),
            },
            Mark::Text {
                at,
                text,
                height,
                turn_deg,
            } => Mark::Text {
                at: place.at(at),
                text,
                height: height * place.scale(),
                turn_deg: turn_deg + place.turn().to_degrees(),
            },
            Mark::Dot { at } => Mark::Dot { at: place.at(at) },
        };
        self.out.push(Drawn {
            color: color.to_owned(),
            mark,
        });
    }

    fn entity(&mut self, e: &Entity, place: &Place, depth: u8) {
        let stroke = |pts: Vec<Vec2>, closed: bool| Mark::Stroke {
            pts,
            closed,
            width: None,
        };
        macro_rules! colored {
            ($base:expr) => {
                match self.color(&$base.layer_id, $base.color.as_ref()) {
                    Some(c) => c,
                    None => return,
                }
            };
        }
        match e {
            Entity::Point(p) => {
                let c = colored!(p.base);
                self.push(&c, place, Mark::Dot { at: p.p });
            }
            Entity::Line(l) => {
                let c = colored!(l.base);
                self.push(&c, place, stroke(vec![l.a, l.b], false));
            }
            Entity::Polyline(p) | Entity::Polygon(p) => {
                let c = colored!(p.base);
                let closed = p.closed || matches!(e, Entity::Polygon(_));
                self.push(
                    &c,
                    place,
                    stroke(bulged(&p.pts, p.bulges.as_ref(), closed), closed),
                );
                for hole in p.holes.iter().flatten() {
                    self.push(&c, place, stroke(hole.pts.clone(), true));
                }
                for part in p.parts.iter().flatten() {
                    self.push(
                        &c,
                        place,
                        stroke(bulged(&part.pts, part.bulges.as_ref(), true), true),
                    );
                }
            }
            Entity::Circle(ci) => {
                let c = colored!(ci.base);
                self.push(
                    &c,
                    place,
                    stroke(arc(ci.c, ci.r, 0.0, std::f64::consts::TAU), true),
                );
            }
            Entity::Arc(a) => {
                let c = colored!(a.base);
                self.push(&c, place, stroke(arc(a.c, a.r, a.a0, a.a1), false));
            }
            Entity::Ellipse(el) => {
                let c = colored!(el.base);
                let minor = Vec2 {
                    x: -el.major.y * el.ratio,
                    y: el.major.x * el.ratio,
                };
                let mut sweep = el.t1 - el.t0;
                while sweep <= 0.0 {
                    sweep += std::f64::consts::TAU;
                }
                let pts = (0..=96)
                    .map(|k| {
                        let t = el.t0 + sweep * k as f64 / 96.0;
                        Vec2 {
                            x: el.c.x
                                + el.major.x * kentos_num::cos(t)
                                + minor.x * kentos_num::sin(t),
                            y: el.c.y
                                + el.major.y * kentos_num::cos(t)
                                + minor.y * kentos_num::sin(t),
                        }
                    })
                    .collect();
                self.push(&c, place, stroke(pts, false));
            }
            Entity::Spline(s) => {
                let c = colored!(s.base);
                self.push(&c, place, stroke(s.pts.clone(), s.closed));
            }
            Entity::Text(t) => {
                let c = colored!(t.base);
                self.push(
                    &c,
                    place,
                    Mark::Text {
                        at: t.p,
                        text: t.text.clone(),
                        height: t.height,
                        turn_deg: t.rotation,
                    },
                );
            }
            Entity::Hatch(h) => {
                let c = colored!(h.base);
                let mut rings = vec![h.ring.clone()];
                rings.extend(h.holes.iter().flatten().cloned());
                self.push(&c, place, Mark::Fill { rings });
            }
            Entity::Insert(i) => {
                let _ = colored!(i.base);
                let Some(def) = self.blocks.get(&i.block).copied() else {
                    self.skipped += 1;
                    return;
                };
                if depth >= 8 {
                    self.skipped += 1;
                    return;
                }
                let (sx, sy) = (i.scale, i.y_scale.unwrap_or(i.scale));
                let sy = if i.mirror { -sy } else { sy };
                let (cos, sin) = (kentos_num::cos(i.rotation), kentos_num::sin(i.rotation));
                let local = Place {
                    a: cos * sx,
                    b: sin * sx,
                    c: -sin * sy,
                    d: cos * sy,
                    e: 0.0,
                    f: 0.0,
                };
                let shift = local.at(def.base);
                let local = Place {
                    e: i.p.x - shift.x,
                    f: i.p.y - shift.y,
                    ..local
                };
                let inner = place.then(&local);
                for child in &def.entities {
                    self.entity(child, &inner, depth + 1);
                }
            }
            Entity::Wall(w) => {
                let c = colored!(w.base);
                let fields = &w.wall;
                let width = fields.width.max(0.0);
                // The body's middle: the justification line (the path moved `offset` to its left), then half the
                // width towards the body.
                let across = fields.offset
                    + match fields.justify {
                        WallJustify::Left => -width / 2.0,
                        WallJustify::Center => 0.0,
                        WallJustify::Right => width / 2.0,
                    };
                let path = bulged(&fields.pts, fields.bulges.as_ref(), fields.closed);
                self.push(
                    &c,
                    place,
                    Mark::Stroke {
                        pts: to_left(&path, across, fields.closed),
                        closed: fields.closed,
                        width: Some(width),
                    },
                );
            }
            Entity::Space(s) => {
                let c = colored!(s.base);
                for part in &s.space.parts {
                    let mut rings = vec![bulged(&part.outer.pts, part.outer.bulges.as_ref(), true)];
                    rings.extend(
                        part.holes
                            .iter()
                            .map(|h| bulged(&h.pts, h.bulges.as_ref(), true)),
                    );
                    self.push(&c, place, Mark::Fill { rings });
                }
                if let (Some(name), Some(seed)) = (&s.space.name, s.space.seed) {
                    self.push(
                        &c,
                        place,
                        Mark::Text {
                            at: seed,
                            text: name.clone(),
                            height: 0.3,
                            turn_deg: 0.0,
                        },
                    );
                }
            }
            Entity::Column(col) => {
                let c = colored!(col.base);
                let f = &col.column;
                let (hw, hd) = (f.width / 2.0, f.depth.unwrap_or(f.width) / 2.0);
                let (cos, sin) = (kentos_num::cos(f.rotation), kentos_num::sin(f.rotation));
                let corner = |x: f64, y: f64| Vec2 {
                    x: f.at.x + x * cos - y * sin,
                    y: f.at.y + x * sin + y * cos,
                };
                self.push(
                    &c,
                    place,
                    Mark::Fill {
                        rings: vec![vec![
                            corner(-hw, -hd),
                            corner(hw, -hd),
                            corner(hw, hd),
                            corner(-hw, hd),
                        ]],
                    },
                );
            }
            _ => self.skipped += 1,
        }
    }
}

fn escape(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '&' => "&amp;".to_owned(),
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            '"' => "&quot;".to_owned(),
            c => c.to_string(),
        })
        .collect()
}

/// The document as one SVG, `width` pixels wide at most (its height follows the drawing's extent).
pub fn svg(doc: &DocumentSnapshotV2, width: u32) -> Preview {
    let mut layer_colors = BTreeMap::new();
    layers(&doc.layers, &mut layer_colors, true);
    let mut walk = Walk {
        layers: &layer_colors,
        blocks: doc.blocks.iter().map(|b| (b.id, b)).collect(),
        out: Vec::new(),
        skipped: 0,
    };
    for e in &doc.entities {
        walk.entity(e, &Place::ID, 0);
    }
    let (mut minx, mut miny, mut maxx, mut maxy) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let mut grow = |p: &Vec2| {
        if p.x.is_finite() && p.y.is_finite() {
            minx = minx.min(p.x);
            miny = miny.min(p.y);
            maxx = maxx.max(p.x);
            maxy = maxy.max(p.y);
        }
    };
    for d in &walk.out {
        match &d.mark {
            Mark::Stroke { pts, .. } => pts.iter().for_each(&mut grow),
            Mark::Fill { rings } => rings.iter().flatten().for_each(&mut grow),
            Mark::Text { at, .. } | Mark::Dot { at } => grow(at),
        }
    }
    let drawn = walk.out.len();
    let (skipped, out) = (walk.skipped, walk.out);
    if drawn == 0 || minx > maxx {
        return Preview {
            svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 400 240\" width=\"400\" height=\"240\">\
                  <rect width=\"400\" height=\"240\" fill=\"#ffffff\"/><text x=\"200\" y=\"124\" font-family=\"sans-serif\" \
                  font-size=\"14\" text-anchor=\"middle\" fill=\"#64707d\">Çizimde gösterilecek nesne yok</text></svg>"
                .to_owned(),
            drawn: 0,
            skipped,
        };
    }
    let span = (maxx - minx).max(maxy - miny).max(1e-3);
    let pad = span * 0.03;
    let (w, h) = (maxx - minx + 2.0 * pad, maxy - miny + 2.0 * pad);
    let px_w = f64::from(width.clamp(64, 4096));
    let px_h = (px_w * h / w).clamp(32.0, 4096.0);
    // A world unit in drawing millimetres: written with three decimals at most.
    let unit = 1000.0;
    let x = |v: f64| ((v - minx + pad) * unit).round() / unit;
    let y = |v: f64| ((maxy + pad - v) * unit).round() / unit;
    let hair = span / px_w * 1.2;
    let mut svg = String::with_capacity(drawn * 64);
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w:.3} {h:.3}\" width=\"{px_w:.0}\" height=\"{px_h:.0}\" \
         data-drawn=\"{drawn}\" data-skipped=\"{skipped}\">\
         <title>{}</title><rect width=\"{w:.3}\" height=\"{h:.3}\" fill=\"#ffffff\"/>",
        escape(&doc.name)
    );
    if skipped > 0 {
        let _ = write!(svg, "<desc>{skipped} nesne önizlemede çizilmedi.</desc>");
    }
    let path_of = |pts: &[Vec2], closed: bool| -> String {
        let mut d = String::with_capacity(pts.len() * 16);
        for (i, p) in pts.iter().enumerate() {
            let _ = write!(d, "{}{} {}", if i == 0 { "M" } else { "L" }, x(p.x), y(p.y));
        }
        if closed {
            d.push('Z');
        }
        d
    };
    for d in &out {
        match &d.mark {
            Mark::Fill { rings } => {
                let path: String = rings
                    .iter()
                    .filter(|r| r.len() > 2)
                    .map(|r| path_of(r, true))
                    .collect();
                if !path.is_empty() {
                    let _ = write!(
                        svg,
                        "<path d=\"{path}\" fill=\"{}\" fill-opacity=\"0.16\" fill-rule=\"evenodd\" stroke=\"none\"/>",
                        d.color
                    );
                }
            }
            Mark::Stroke { pts, closed, width } if pts.len() > 1 => {
                let w = width.filter(|w| *w > hair).unwrap_or(hair);
                let _ = write!(
                    svg,
                    "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.4}\" stroke-linejoin=\"round\" stroke-linecap=\"{}\"{}/>",
                    path_of(pts, *closed),
                    d.color,
                    w,
                    if width.is_some() { "butt" } else { "round" },
                    if width.is_some() {
                        " stroke-opacity=\"0.85\""
                    } else {
                        ""
                    }
                );
            }
            Mark::Stroke { .. } => {}
            Mark::Text {
                at,
                text,
                height,
                turn_deg,
            } if !text.trim().is_empty() && *height > 0.0 => {
                let _ = write!(
                    svg,
                    "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{:.4}\" fill=\"{}\" transform=\"rotate({:.2} {} {})\">{}</text>",
                    x(at.x),
                    y(at.y),
                    height,
                    d.color,
                    -turn_deg,
                    x(at.x),
                    y(at.y),
                    escape(text.lines().next().unwrap_or_default())
                );
            }
            Mark::Text { .. } => {}
            Mark::Dot { at } => {
                let _ = write!(
                    svg,
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{:.4}\" fill=\"{}\"/>",
                    x(at.x),
                    y(at.y),
                    hair * 1.6,
                    d.color
                );
            }
        }
    }
    svg.push_str("</svg>");
    Preview {
        svg,
        drawn,
        skipped,
    }
}

/// The width a picture is drawn at when none is asked for, and the bounds of one asked for.
pub const DEFAULT_WIDTH: u32 = 1200;
pub const MIN_WIDTH: u32 = 64;
pub const MAX_WIDTH: u32 = 4096;

/// A project's picture and its entity tag; no picture when the caller already has this one (`If-None-Match`).
#[derive(Debug)]
pub struct Pictured {
    pub tag: String,
    pub picture: Option<Preview>,
}

/// Whether an `If-None-Match` value names `tag` (or is `*`).
pub fn already_has(if_none_match: Option<&str>, tag: &str) -> bool {
    if_none_match.is_some_and(|v| {
        v.split(',')
            .map(|t| t.trim().trim_start_matches("W/"))
            .any(|t| t == "*" || t == tag)
    })
}

/// The counts a picture carries on its root (a cached one is read back with them).
fn counted(svg: String) -> Preview {
    let number = |name: &str| {
        svg.find(&format!("{name}=\""))
            .and_then(|at| {
                let rest = &svg[at + name.len() + 2..];
                rest[..rest.find('"')?].parse().ok()
            })
            .unwrap_or(0)
    };
    let (drawn, skipped) = (number("data-drawn"), number("data-skipped"));
    Preview {
        svg,
        drawn,
        skipped,
    }
}

async fn drawn_from(bytes: Vec<u8>, width: u32) -> AppResult<Preview> {
    tokio::task::spawn_blocking(move || {
        kentos_kcad::read(&bytes)
            .map(|(_, doc)| svg(&doc, width))
            .map_err(|e| {
                AppError::invalid(format!("Dosya bir KentOS çizimi olarak okunamadı: {e}"))
            })
    })
    .await
    .map_err(|e| AppError::Storage(std::io::Error::other(e)))?
}

/// `GET …/projects/{project}/preview.svg`: a file project's newest revision (or `revision`, with
/// `project.history` when it is an earlier one), a database project's objects as they are now. Who reads the
/// project sees its picture (`project.read`); a Veri girişi, who reads its records through forms, does not.
pub async fn project(
    db: &Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    revision: Option<i64>,
    width: u32,
    if_none_match: Option<&str>,
) -> AppResult<Pictured> {
    access.live()?;
    access.require(ProjectPermission::Read)?;
    if access.through_forms() {
        return Err(AppError::forbidden(
            "Veri girişi projenin bütün çizimini görmez; kayıtlarını formla okur.",
        ));
    }
    let width = width.clamp(MIN_WIDTH, MAX_WIDTH);
    let mut tx = db.scoped(access.scope()).await?;
    let (storage, current, data_revision) = crate::files::file_state(&mut tx, access).await?;
    match storage {
        ProjectStorage::File => {
            let r = revision.unwrap_or(current);
            if r <= 0 {
                return Err(AppError::not_found(format!(
                    "“{}” projesinde henüz kaydedilmiş bir çizim yok.",
                    access.name
                )));
            }
            if r != current {
                access.require(ProjectPermission::History)?;
            }
            let row = crate::files::revision(&mut tx, access.tenant, access.project, r)
                .await?
                .ok_or_else(|| {
                    AppError::not_found(format!("“{}” projesinde {r}. revizyon yok.", access.name))
                })?;
            tx.commit().await?;
            let sha = row.2;
            let tag = format!("\"r{r}-{}-w{width}\"", &sha[..16]);
            if already_has(if_none_match, &tag) {
                return Ok(Pictured { tag, picture: None });
            }
            let cache = format!(
                "previews/{}/{}/{r:010}-{sha}-{width}",
                access.tenant, access.project
            );
            if let Ok(bytes) = blobs.read(&cache).await
                && let Ok(text) = String::from_utf8(bytes)
            {
                return Ok(Pictured {
                    tag,
                    picture: Some(counted(text)),
                });
            }
            let picture = drawn_from(blobs.read(&row.7).await?, width).await?;
            // Kept for the next caller; a picture not kept is drawn again, nothing more.
            if let Ok(mut w) = blobs.create(&cache, picture.svg.len() as u64).await
                && w.write(picture.svg.as_bytes()).await.is_ok()
            {
                let _ = w.finish().await;
            }
            Ok(Pictured {
                tag,
                picture: Some(picture),
            })
        }
        ProjectStorage::Database => {
            tx.commit().await?;
            if revision.is_some() {
                return Err(AppError::invalid_at(
                    "revision",
                    "Nesneleri veritabanında saklanan projenin önizlemesi şimdiki hâlidir; revizyon verilmez.",
                ));
            }
            let tag = format!("\"d{data_revision}-w{width}\"");
            if already_has(if_none_match, &tag) {
                return Ok(Pictured { tag, picture: None });
            }
            let snap = crate::snapshot::take(db, access, crate::snapshot::Take::Cut).await?;
            let tag = format!("\"d{}-w{width}\"", snap.revision);
            let picture = drawn_from(snap.bytes, width).await?;
            Ok(Pictured {
                tag,
                picture: Some(picture),
            })
        }
    }
}

/// `GET /v1/dossiers/{id}/files/{file}/preview.svg`: a dossier's KentOS project file, for whoever sees the dossier.
pub async fn dossier_file(
    db: &Db,
    blobs: &Blobs,
    actor: &Actor,
    (dossier, file): (Uuid, Uuid),
    width: u32,
    if_none_match: Option<&str>,
) -> AppResult<Pictured> {
    let (view, _) = crate::portal::files::download(db, blobs, actor, dossier, file).await?;
    if !view.is_project {
        return Err(AppError::invalid(
            "Bu dosyanın önizlemesi yok: yalnız KentOS proje dosyalarının (KCAD) önizlemesi çizilir.",
        ));
    }
    let width = width.clamp(MIN_WIDTH, MAX_WIDTH);
    let tag = format!("\"{}-w{width}\"", &view.sha256[..16]);
    if already_has(if_none_match, &tag) {
        return Ok(Pictured { tag, picture: None });
    }
    let key = crate::blobs::Blobs::dossier_key(dossier, file);
    let picture = drawn_from(blobs.read(&key).await?, width).await?;
    Ok(Pictured {
        tag,
        picture: Some(picture),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DocumentSnapshotV2 {
        let bytes = include_bytes!("../../../../samples/Konut100m2.kcad");
        kentos_kcad::read(bytes).expect("a KCAD sample").1
    }

    #[test]
    fn a_house_plan_is_drawn_with_its_rooms_names_and_the_same_bytes_each_time() {
        let doc = sample();
        let a = svg(&doc, 800);
        assert!(a.drawn > 10, "{} drawn", a.drawn);
        assert!(a.svg.starts_with("<svg") && a.svg.ends_with("</svg>"));
        assert!(a.svg.contains("SALON"), "its rooms' names");
        assert_eq!(
            counted(a.svg.clone()),
            a,
            "a cached picture reads back with its counts"
        );
        assert!(
            already_has(Some("\"x\", W/\"r1-ab-w800\""), "\"r1-ab-w800\"")
                && !already_has(None, "\"a\"")
        );
        assert_eq!(svg(&doc, 800), a, "deterministic");
    }

    #[test]
    fn a_wall_is_as_wide_as_itself_and_lies_where_its_justification_puts_it() {
        let mut doc = sample();
        let layer = doc.layers[0].id.clone();
        let wall = serde_json::from_value::<Entity>(serde_json::json!({
            "kind": "wall", "id": 900, "layerId": layer, "attrs": {},
            "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 4.0, "y": 0.0 }, { "x": 4.0, "y": 3.0 }],
            "width": 0.2, "justify": "left",
        }))
        .expect("a wall");
        doc.entities = vec![wall];
        let drawn = svg(&doc, 400);
        assert_eq!((drawn.drawn, drawn.skipped), (1, 0));
        assert!(
            drawn.svg.contains("stroke-width=\"0.2000\"")
                && drawn.svg.contains("stroke-linecap=\"butt\"")
        );
        // Its left face on the path: its middle a tenth of a metre to the path's right, the corner mitred.
        let middle = to_left(
            &[
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 4.0, y: 0.0 },
                Vec2 { x: 4.0, y: 3.0 },
            ],
            -0.1,
            false,
        );
        let expected = [(0.0, -0.1), (4.1, -0.1), (4.1, 3.0)];
        for (p, (x, y)) in middle.iter().zip(expected) {
            assert!(
                (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9,
                "{p:?} ≠ ({x}, {y})"
            );
        }
    }

    #[test]
    fn a_bulge_is_sampled_on_its_arc_and_an_empty_drawing_says_so() {
        // A half circle from (0,0) to (2,0): every point one unit from (1,0); bulge 1 turns left (below the chord),
        // bulge -1 right (above it).
        for (bulge, side) in [(1.0, -1.0), (-1.0, 1.0)] {
            let pts = bulged(
                &[Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 2.0, y: 0.0 }],
                Some(&vec![bulge, 0.0]),
                false,
            );
            assert!(pts.len() > 3);
            for p in &pts {
                let r = (kentos_num::powi(p.x - 1.0, 2) + kentos_num::powi(p.y, 2)).sqrt();
                assert!(
                    (r - 1.0).abs() < 1e-9 && p.y * side >= -1e-9,
                    "{bulge}: {p:?}"
                );
            }
        }
        let mut doc = sample();
        doc.entities.clear();
        let none = svg(&doc, 400);
        assert_eq!(none.drawn, 0);
        assert!(none.svg.contains("Çizimde gösterilecek nesne yok"));
        assert_eq!(escape("a<b>&\"c\u{7}"), "a&lt;b&gt;&amp;&quot;c");
    }
}
