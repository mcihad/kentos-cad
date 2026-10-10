//! The labels of a window (docs/adr/0212 §3): what the store keeps for the
//! label engine (each layer's labelling, the kinds' defaults, each object's
//! texts and pins) and how a window's labels are gathered for it: the
//! visible objects' label units by their kind and class (repeated and
//! merged lines, outlines, areas cut to the window), the obstacles (point
//! symbols, the obstacle layers' objects), the contours' uphill sides and the
//! pins; the result as label records after the drawing's texts.

use std::collections::HashMap;
use std::sync::Arc;

use crate::api::json::Json;
use crate::entity::{
    Shape, entity_anchor, entity_outline, entity_vertices, label_part, polygon_holes, polygon_ring,
    tessellate_circle,
};
use crate::geometry::Bounds;
use crate::jsmath::{TAU, cos, js_max, js_min, sin};
use crate::labels::engine::{
    self, Drawn, Geo, Obstacle, Options, Pinned, Scene, Shape as Ob, Unit,
};
use crate::labels::geom::{Obb, Walk, clip_path, clip_ring, norm, pole};
use crate::labels::style::{
    AreaMode, Class, Labelling, LineMode, PointMode, read_class, read_labelling,
};
use crate::vec2::Vec2;

use super::{IdMap, Item, Store};

/// A label's frame: its block's middle, angle (degrees), width and height
/// (px), its class and its state (`engine::PINNED` …); its lines, letters and
/// callout follow it.
pub const LABEL_PLACED: f64 = 20.0;
/// A placed label's line: its middle, angle (degrees), size (px), its text
/// (`Shown::texts`) and its width (px).
pub const LABEL_PLACED_LINE: f64 = 21.0;
/// A curved label's letter: its middle, angle (degrees), size (px), its text,
/// its place in the text (letters) and its advance (px).
pub const LABEL_PLACED_LETTER: f64 = 22.0;
/// A placed label's callout: from (by the label) and to (the object), its class and state.
pub const LABEL_PLACED_CALLOUT: f64 = 23.0;

/// How far beyond the window labels are placed, px: what is placed near its
/// edge is placed as it would be inside (MapLibre's 100 px, the overlays'
/// margins are wider).
const MARGIN: f64 = 256.0;
/// How far a contour's neighbours are looked for, in font sizes.
const PROBE: f64 = 6.0;
/// A point symbol's least radius as an obstacle, px.
const SYMBOL_MIN: f64 = 2.0;

/// An object's labels as the host gives them: a text for each class that
/// labels it (its index in the layer's classes; 0 for a kind's default), and
/// its height (a contour's), NaN without one.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLabels {
    pub texts: Vec<(u16, String)>,
    pub z: f64,
}

/// A label moved, turned or hidden by hand, as the host gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct Pin {
    /// The class's name; none, the object's first label.
    pub class: Option<String>,
    /// From the object's anchor, metres.
    pub at: Option<Vec2>,
    /// Degrees.
    pub rotation: f64,
    pub hidden: bool,
}

/// A layer as the label engine sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelLayer {
    pub labelling: Option<Arc<Labelling>>,
    /// Its place in the drawing order, the top first.
    pub rank: u32,
    /// Its point symbol's size, px.
    pub point: f64,
}

/// What the store keeps for labels.
#[derive(Clone, Debug, Default)]
pub struct Labels {
    pub(super) layers: Vec<Option<LabelLayer>>,
    pub(super) defaults: [Option<Arc<Class>>; 5],
    pub(super) objects: IdMap<ObjectLabels>,
    pub(super) pins: IdMap<Vec<Pin>>,
}

/// What a window's text is: the records and their texts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    pub records: Vec<f64>,
    pub texts: Vec<String>,
}

/// What a placing is asked for besides the labels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlaceOptions {
    /// Labels with no free place, at their best one (Yerleşmeyen etiketleri göster).
    pub unplaced: bool,
    /// Labels hidden by hand (Etiketi gizle's Göster).
    pub hidden: bool,
}

/// The smallest object a class labels at `scale` (its least screen size; infinite when it is not shown
/// at this scale).
fn feature_floor(c: &Class, scale: f64) -> f64 {
    if c.shown_at(scale) {
        c.min_feature_px.unwrap_or(0.0)
    } else {
        f64::INFINITY
    }
}

/// What kind of label an object takes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Point,
    Line,
    Area,
}

fn kind_of(s: &Shape) -> Option<Kind> {
    Some(match s {
        Shape::Point { .. } | Shape::Insert { .. } => Kind::Point,
        Shape::Line { .. } | Shape::Polyline { .. } | Shape::Arc { .. } => Kind::Line,
        Shape::Spline { closed, .. } => {
            if *closed {
                Kind::Area
            } else {
                Kind::Line
            }
        }
        Shape::Ellipse { t0, t1, .. } => {
            if (t1 - t0).abs() >= TAU - 1e-9 {
                Kind::Area
            } else {
                Kind::Line
            }
        }
        Shape::Polygon { .. }
        | Shape::Circle { .. }
        | Shape::Hatch { .. }
        | Shape::Image { .. }
        | Shape::Raster { .. }
        | Shape::PointCloud { .. } => Kind::Area,
        _ => return None,
    })
}

/// A kind's place among the defaults (`set_label_defaults_json`'s order).
fn default_slot(s: &Shape) -> Option<usize> {
    match s {
        Shape::Polygon { .. } => Some(0),
        Shape::Circle { .. } => Some(1),
        Shape::Point { .. } => Some(2),
        Shape::Polyline { .. } => Some(3),
        Shape::Line { .. } => Some(4),
        _ => None,
    }
}

/// The page: the window's lower left and the scale.
#[derive(Clone, Copy)]
struct Page {
    x0: f64,
    y0: f64,
    s: f64,
}

impl Page {
    fn px(&self, p: Vec2) -> Vec2 {
        Vec2::new((p.x - self.x0) * self.s, (p.y - self.y0) * self.s)
    }

    fn world(&self, p: Vec2) -> Vec2 {
        Vec2::new(self.x0 + p.x / self.s, self.y0 + p.y / self.s)
    }
}

/// An area's rings in the label part: its outline first, then its holes (world).
fn area_rings(s: &Shape) -> Vec<Vec<Vec2>> {
    let part = label_part(s);
    match part.as_ref() {
        Shape::Polygon { pts, bulges, .. } => {
            let mut rings = vec![polygon_ring(pts, bulges.as_deref())];
            rings.extend(polygon_holes(&part));
            rings
        }
        Shape::Circle { c, r } => vec![tessellate_circle(*c, *r, 64.0)],
        Shape::Hatch { ring, holes, .. } => {
            let mut rings = vec![ring.clone()];
            rings.extend(holes.iter().flatten().cloned());
            rings
        }
        Shape::Image { .. } | Shape::Raster { .. } | Shape::PointCloud { .. } => {
            vec![entity_vertices(&part)]
        }
        other => {
            let mut ring = entity_outline(other, 64.0);
            if ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            vec![ring]
        }
    }
}

/// A ring's area, counter-clockwise positive.
fn signed_area(ring: &[Vec2]) -> f64 {
    let n = ring.len();
    let mut a = 0.0;
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        a += p.x * q.y - q.x * p.y;
    }
    a / 2.0
}

/// The stretches of a path inside `rect` along it: `(from, to)` lengths.
fn inside_stretches(walk: &Walk, rect: [f64; 4]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for i in 0..walk.pts.len().saturating_sub(1) {
        let (a, b) = (walk.pts[i], walk.pts[i + 1]);
        let pieces = clip_path(&[a, b], rect);
        let len = walk.at[i + 1] - walk.at[i];
        for piece in pieces {
            let (s, e) = (piece[0], piece[piece.len() - 1]);
            let t0 = norm(s.x - a.x, s.y - a.y) / len;
            let t1 = norm(e.x - a.x, e.y - a.y) / len;
            let (from, to) = (walk.at[i] + t0 * len, walk.at[i] + t1 * len);
            match out.last_mut() {
                Some(last) if from <= last.1 + 1e-9 => last.1 = js_max(last.1, to),
                _ => out.push((from, to)),
            }
        }
    }
    out
}

/// Line pieces meeting end to end with the same text, joined (docs/adr/0212 §3.3):
/// each chain's points (world), started by its piece first in the document.
fn merged(pieces: &[(usize, Vec<Vec2>)]) -> Pieces {
    let key = |p: Vec2| (p.x.to_bits(), p.y.to_bits());
    let mut ends: HashMap<(u64, u64), Vec<usize>> = HashMap::new();
    for (k, (_, pts)) in pieces.iter().enumerate() {
        if let (Some(&a), Some(&b)) = (pts.first(), pts.last()) {
            ends.entry(key(a)).or_default().push(k);
            ends.entry(key(b)).or_default().push(k);
        }
    }
    let mut used = vec![false; pieces.len()];
    let mut out = Vec::new();
    for k in 0..pieces.len() {
        if used[k] || pieces[k].1.len() < 2 {
            continue;
        }
        used[k] = true;
        let mut chain: Vec<Vec2> = pieces[k].1.clone();
        // On from its end, then back from its start, while exactly two pieces meet.
        for forward in [true, false] {
            loop {
                let at = if forward {
                    chain[chain.len() - 1]
                } else {
                    chain[0]
                };
                let meeting = &ends[&key(at)];
                if meeting.len() != 2 {
                    break;
                }
                let Some(&next) = meeting.iter().find(|&&m| !used[m]) else {
                    break;
                };
                used[next] = true;
                let mut pts = pieces[next].1.clone();
                if forward {
                    if key(pts[0]) != key(at) {
                        pts.reverse();
                    }
                    chain.extend(pts.into_iter().skip(1));
                } else {
                    if key(pts[pts.len() - 1]) != key(at) {
                        pts.reverse();
                    }
                    pts.pop();
                    pts.extend(chain);
                    chain = pts;
                }
            }
        }
        out.push((pieces[k].0, chain));
    }
    out
}

/// A contour's segment: its ends, its height and its object's id.
type ContourSegment = (Vec2, Vec2, f64, f64);

/// The contours' segments of a layer in grid cells, for the uphill probes.
struct Contours {
    cells: HashMap<(i64, i64), Vec<ContourSegment>>,
}

/// Pieces of lines to be merged: each its place in the list and its points.
type Pieces = Vec<(usize, Vec<Vec2>)>;

/// A line waiting to be merged with the others of its layer, class and text (docs/adr/0212 §3.3).
struct Merging<'a> {
    key: (u32, u16, &'a str),
    item: &'a Item,
    world: Vec<Vec2>,
    cls: &'a Class,
}

const PROBE_CELL: f64 = 32.0;

impl Contours {
    fn add(&mut self, a: Vec2, b: Vec2, z: f64, id: f64) {
        let (c0, c1) = (
            (js_min(a.x, b.x) / PROBE_CELL).floor() as i64,
            (js_max(a.x, b.x) / PROBE_CELL).floor() as i64,
        );
        let (r0, r1) = (
            (js_min(a.y, b.y) / PROBE_CELL).floor() as i64,
            (js_max(a.y, b.y) / PROBE_CELL).floor() as i64,
        );
        for c in c0..=c1 {
            for r in r0..=r1 {
                self.cells.entry((c, r)).or_default().push((a, b, z, id));
            }
        }
    }

    /// The height of the nearest other contour the ray from `p` along `d`
    /// crosses within `len`.
    fn nearest(&self, p: Vec2, d: Vec2, len: f64, own: f64) -> Option<f64> {
        let q = Vec2::new(p.x + d.x * len, p.y + d.y * len);
        let (c0, c1) = (
            (js_min(p.x, q.x) / PROBE_CELL).floor() as i64,
            (js_max(p.x, q.x) / PROBE_CELL).floor() as i64,
        );
        let (r0, r1) = (
            (js_min(p.y, q.y) / PROBE_CELL).floor() as i64,
            (js_max(p.y, q.y) / PROBE_CELL).floor() as i64,
        );
        let mut best: Option<(f64, f64)> = None;
        for c in c0..=c1 {
            for r in r0..=r1 {
                for &(a, b, z, id) in self.cells.get(&(c, r)).into_iter().flatten() {
                    if id == own {
                        continue;
                    }
                    // p + t·(q − p) = a + u·(b − a)
                    let (rx, ry) = (q.x - p.x, q.y - p.y);
                    let (sx, sy) = (b.x - a.x, b.y - a.y);
                    let den = rx * sy - ry * sx;
                    if den == 0.0 {
                        continue;
                    }
                    let t = ((a.x - p.x) * sy - (a.y - p.y) * sx) / den;
                    let u = ((a.x - p.x) * ry - (a.y - p.y) * rx) / den;
                    if t > 0.0
                        && t <= 1.0
                        && (0.0..=1.0).contains(&u)
                        && best.is_none_or(|(bt, _)| t < bt)
                    {
                        best = Some((t, z));
                    }
                }
            }
        }
        best.map(|(_, z)| z)
    }
}

/// Whether a contour reads along its path (true) or against it (false) so
/// its labels' tops point uphill; none when no neighbour says (docs/adr/0212 §3.3).
fn uphill(walk: &Walk, z: f64, own: f64, probe: f64, contours: &Contours) -> Option<bool> {
    let len = walk.length();
    for f in [0.5, 0.25, 0.75, 0.125, 0.375, 0.625, 0.875] {
        let s = len * f;
        let p = walk.point(s);
        let a = walk.angle(s);
        let n = Vec2::new(-sin(a), cos(a));
        let plus = contours.nearest(p, n, probe, own);
        let minus = contours.nearest(p, Vec2::new(-n.x, -n.y), probe, own);
        let says = match (plus, minus) {
            (Some(zp), Some(zm)) if zp != zm => Some(zp > zm),
            (Some(zp), None) if zp != z => Some(zp > z),
            (None, Some(zm)) if zm != z => Some(zm < z),
            _ => None,
        };
        if says.is_some() {
            return says;
        }
    }
    None
}

/// An outline as an oriented box (its first edge's direction), px.
pub fn outline_box(pts: &[Vec2]) -> Option<Obb> {
    let (&a, &b) = (pts.first()?, pts.get(1)?);
    let len = norm(b.x - a.x, b.y - a.y);
    if !(len > 0.0) {
        return None;
    }
    let u = Vec2::new((b.x - a.x) / len, (b.y - a.y) / len);
    let v = Vec2::new(-u.y, u.x);
    let (mut s0, mut s1, mut t0, mut t1) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for p in pts {
        let (s, t) = (
            (p.x - a.x) * u.x + (p.y - a.y) * u.y,
            (p.x - a.x) * v.x + (p.y - a.y) * v.y,
        );
        s0 = js_min(s0, s);
        s1 = js_max(s1, s);
        t0 = js_min(t0, t);
        t1 = js_max(t1, t);
    }
    let (ms, mt) = ((s0 + s1) / 2.0, (t0 + t1) / 2.0);
    Some(Obb {
        c: Vec2::new(a.x + u.x * ms + v.x * mt, a.y + u.y * ms + v.y * mt),
        u,
        a: (s1 - s0) / 2.0,
        b: (t1 - t0) / 2.0,
    })
}

impl Store {
    /// The label engine's view of the layers (docs/adr/0212 §3.1): `[{ id,
    /// rank, point?, label?, labels? }]`, every layer that labels or has a
    /// point symbol; `label` and `labels` its style's fields. Replaces what
    /// was sent; a table that does not read changes nothing.
    pub fn set_label_layers_json(&mut self, text: &str) -> Result<(), String> {
        let Json::Arr(list) = Json::parse(text)? else {
            return Err("katman dizisi bekleniyordu".into());
        };
        let mut rows = Vec::with_capacity(list.len());
        for (i, v) in list.iter().enumerate() {
            let Json::Str(id) = v.get("id") else {
                return Err(format!("[{i}].id: metin bekleniyordu"));
            };
            let rank = match v.get("rank") {
                Json::Num(r) if *r >= 0.0 => *r as u32,
                _ => return Err(format!("[{i}].rank: sayı bekleniyordu")),
            };
            let point = match v.get("point") {
                Json::Num(p) if p.is_finite() => *p,
                _ => 0.0,
            };
            let labelling = match (v.get("label"), v.get("labels")) {
                (Json::Null, Json::Null) => None,
                _ => Some(Arc::new(
                    read_labelling(v).map_err(|e| format!("[{i}] {id}: {e}"))?,
                )),
            };
            rows.push((
                id.clone(),
                LabelLayer {
                    labelling,
                    rank,
                    point,
                },
            ));
        }
        self.labelling.layers.clear();
        for (id, layer) in rows {
            let l = self.layer_index(&id) as usize;
            if self.labelling.layers.len() <= l {
                self.labelling.layers.resize(l + 1, None);
            }
            self.labelling.layers[l] = Some(layer);
        }
        Ok(())
    }

    /// The label styles of objects whose layer has none, by kind:
    /// `{ polygon, circle, point, polyline, line }` (`LabelStyle`s, docs/adr/0212 §2).
    pub fn set_label_defaults_json(&mut self, text: &str) -> Result<(), String> {
        let v = Json::parse(text)?;
        let mut out: [Option<Arc<Class>>; 5] = Default::default();
        for (i, kind) in ["polygon", "circle", "point", "polyline", "line"]
            .iter()
            .enumerate()
        {
            let r = v.get(kind);
            if !matches!(r, Json::Null) {
                out[i] = Some(Arc::new(
                    read_class(r, None).map_err(|e| format!("{kind}: {e}"))?,
                ));
            }
        }
        self.labelling.defaults = out;
        Ok(())
    }

    /// An object's labels' texts (docs/adr/0212 §3.1); empty forgets them.
    /// A put object loses its texts and pins: the host sends them again.
    pub fn set_object_labels(&mut self, id: f64, labels: ObjectLabels) {
        if labels.texts.is_empty() && labels.z.is_nan() {
            self.labelling.objects.remove(&id.to_bits());
        } else {
            self.labelling.objects.insert(id.to_bits(), labels);
        }
    }

    /// Objects' labels at once (the page's way): `ids[i]`'s texts are the
    /// entries `from[i]..from[i + 1]` of `classes` and `texts` (`lens` their
    /// lengths in UTF-16 units), its height `zs[i]`.
    pub fn set_object_labels_packed(
        &mut self,
        ids: &[f64],
        from: &[u32],
        classes: &[u16],
        texts: &str,
        lens: &[u32],
        zs: &[f64],
    ) -> Result<(), String> {
        if from.len() != ids.len() + 1 || zs.len() != ids.len() || classes.len() != lens.len() {
            return Err("etiket metinlerinin dizileri uyuşmuyor".into());
        }
        let units: Vec<u16> = texts.encode_utf16().collect();
        let mut at = 0usize;
        let mut strings = Vec::with_capacity(lens.len());
        for &l in lens {
            let end = at + l as usize;
            let s = String::from_utf16(units.get(at..end).ok_or("etiket metni kısa")?)
                .map_err(|_| "etiket metni bozuk")?;
            strings.push(s);
            at = end;
        }
        for (i, &id) in ids.iter().enumerate() {
            let (a, b) = (from[i] as usize, from[i + 1] as usize);
            if a > b || b > strings.len() {
                return Err("etiket metinlerinin sırası bozuk".into());
            }
            let texts = (a..b)
                .map(|k| (classes[k], std::mem::take(&mut strings[k])))
                .collect();
            self.set_object_labels(id, ObjectLabels { texts, z: zs[i] });
        }
        Ok(())
    }

    /// An object's pins (docs/adr/0212 §3.7); empty forgets them.
    pub fn set_label_pins(&mut self, id: f64, pins: Vec<Pin>) {
        if pins.is_empty() {
            self.labelling.pins.remove(&id.to_bits());
        } else {
            self.labelling.pins.insert(id.to_bits(), pins);
        }
    }

    /// Objects' pins from the contract's JSON: `[[id, [LabelPin …]] …]`.
    pub fn set_label_pins_json(&mut self, text: &str) -> Result<(), String> {
        let Json::Arr(list) = Json::parse(text)? else {
            return Err("iğne dizisi bekleniyordu".into());
        };
        for e in &list {
            let (Json::Num(id), Json::Arr(pins)) = (match e {
                Json::Arr(pair) if pair.len() == 2 => (&pair[0], &pair[1]),
                _ => return Err("[kimlik, iğneler] bekleniyordu".into()),
            }) else {
                return Err("[kimlik, iğneler] bekleniyordu".into());
            };
            let mut out = Vec::with_capacity(pins.len());
            for p in pins {
                let at = match p.get("at") {
                    Json::Null => None,
                    a => match (a.get("x"), a.get("y")) {
                        (Json::Num(x), Json::Num(y)) => Some(Vec2::new(*x, *y)),
                        _ => return Err("iğnenin yeri {x, y} olmalı".into()),
                    },
                };
                out.push(Pin {
                    class: match p.get("class") {
                        Json::Str(s) => Some(s.clone()),
                        _ => None,
                    },
                    at,
                    rotation: match p.get("rotation") {
                        Json::Num(r) => *r,
                        _ => 0.0,
                    },
                    hidden: matches!(p.get("hidden"), Json::Bool(true)),
                });
            }
            self.set_label_pins(*id, out);
        }
        Ok(())
    }

    /// Counts an object's labels that are not written at `scale` px per
    /// metre: out of their class's scale range, on an object smaller than
    /// its class's smallest feature (Etiketleri yazıya çevir's finding).
    pub(crate) fn count_unwritten(
        &self,
        id: f64,
        scale: f64,
        out: &mut crate::ops::label_text::LabelTexts,
    ) {
        let (Some(it), Some(labels)) = (self.get(id), self.labelling.objects.get(&id.to_bits()))
        else {
            return;
        };
        let b = &it.bounds;
        for (class, text) in &labels.texts {
            let Some(cls) = self.class_of(it, *class) else {
                continue;
            };
            if text.is_empty() {
                continue;
            }
            if !cls.shown_at(scale) {
                out.out_of_scale += 1;
            } else if cls
                .min_feature_px
                .is_some_and(|m| js_min(b.max_x - b.min_x, b.max_y - b.min_y) * scale < m)
            {
                out.small += 1;
            }
        }
    }

    /// Where an object's labels are pinned from (docs/adr/0212 §2): its
    /// anchor (`entity_anchor` of its label part), world; none for an
    /// unknown object or one without a place.
    pub fn label_anchor(&self, id: f64) -> Option<Vec2> {
        entity_anchor(&label_part(&self.get(id)?.shape))
    }

    /// The class an object's text `class` names: its layer's, else its kind's default.
    fn class_of<'s>(&'s self, it: &Item, class: u16) -> Option<&'s Class> {
        let layer = self
            .labelling
            .layers
            .get(it.layer as usize)
            .and_then(Option::as_ref);
        match layer.and_then(|l| l.labelling.as_deref()) {
            Some(l) if l.off => None,
            Some(l) if !l.classes.is_empty() => l.classes.get(class as usize),
            _ => default_slot(&it.shape).and_then(|k| self.labelling.defaults[k].as_deref()),
        }
    }

    /// The labels of `window` at `scale` px per metre around the drawing's
    /// texts `fixed` (their outlines, world), as label records and texts.
    pub fn place_labels(
        &self,
        window: &Bounds,
        scale: f64,
        fixed: &[Vec<Vec2>],
        options: PlaceOptions,
    ) -> Shown {
        let page = Page {
            x0: window.min_x,
            y0: window.min_y,
            s: scale,
        };
        let (width, height) = (
            (window.max_x - window.min_x) * scale,
            (window.max_y - window.min_y) * scale,
        );
        if !(scale > 0.0) || !(width >= 0.0) || !(height >= 0.0) {
            return Shown::default();
        }
        let rect = [-MARGIN, -MARGIN, width + MARGIN, height + MARGIN];
        let reach = MARGIN / scale;
        let query = super::padded(*window, reach);
        let mut units: Vec<Unit<'_>> = Vec::new();
        let mut obstacles: Vec<Obstacle> = Vec::new();
        let mut contours: HashMap<u32, Contours> = HashMap::new();
        // Lines to be merged, by layer, class and text: (unit's object, its order, its points).
        let mut to_merge: Vec<Merging<'_>> = Vec::new();
        let items = self.candidates(&query);
        // The smallest object each layer's classes label at this scale (infinite: none): one smaller is
        // passed by unread (an overview's many). None: the kinds' defaults, by the object.
        let smallest: Vec<Option<f64>> = self
            .labelling
            .layers
            .iter()
            .map(|l| {
                let l = l.as_ref()?.labelling.as_deref()?;
                if l.off {
                    return Some(f64::INFINITY);
                }
                (!l.classes.is_empty()).then(|| {
                    l.classes
                        .iter()
                        .map(|c| feature_floor(c, scale))
                        .fold(f64::INFINITY, js_min)
                })
            })
            .collect();
        for it in &items {
            let flags = self.flags(it);
            if !flags.visible {
                continue;
            }
            let layer = self
                .labelling
                .layers
                .get(it.layer as usize)
                .and_then(Option::as_ref);
            // Only a point's symbol, an obstacle layer's object or a line (a contour) matters here.
            let point = matches!(it.shape, Shape::Point { .. });
            let obstacle = layer
                .and_then(|l| l.labelling.as_ref())
                .and_then(|l| l.obstacle);
            let line = kind_of(&it.shape) == Some(Kind::Line);
            if !(point || obstacle.is_some() || line) || !self.view_shown(it.id) {
                continue;
            }
            let point_px = layer.map_or(0.0, |l| l.point);
            // Every point's symbol keeps labels off it a little.
            if let Shape::Point { p, parts, .. } = &it.shape {
                let r = js_max(point_px / 2.0, SYMBOL_MIN);
                for q in std::iter::once(*p).chain(parts.iter().flatten().map(|q| q.p)) {
                    obstacles.push(Obstacle {
                        owner: it.id,
                        weight: None,
                        shape: Ob::Circle(page.px(q), r),
                    });
                }
            }
            if let Some((w, boundary)) = obstacle {
                self.obstacle_of(it, w, boundary, page, point_px, &mut obstacles);
            }
            if line
                && let Some(labels) = self.labelling.objects.get(&it.id.to_bits())
                && labels.z.is_finite()
            {
                let path: Vec<Vec2> = entity_outline(&label_part(&it.shape), 64.0)
                    .iter()
                    .map(|&p| page.px(p))
                    .collect();
                let c = contours.entry(it.layer).or_insert_with(|| Contours {
                    cells: HashMap::new(),
                });
                for w in path.windows(2) {
                    c.add(w[0], w[1], labels.z, it.id);
                }
            }
        }
        for it in &items {
            let flags = self.flags(it);
            if !flags.visible {
                continue;
            }
            let floor = match smallest.get(it.layer as usize).copied().flatten() {
                Some(f) => f,
                None => default_slot(&it.shape)
                    .and_then(|k| self.labelling.defaults[k].as_deref())
                    .map_or(f64::INFINITY, |c| feature_floor(c, scale)),
            };
            let b = &it.bounds;
            if js_min(b.max_x - b.min_x, b.max_y - b.min_y) * scale < floor {
                continue;
            }
            if !self.view_shown(it.id)
                || (!self.text_labelled.is_empty() && self.text_labelled.contains(&it.id.to_bits()))
            {
                continue;
            }
            let Some(labels) = self.labelling.objects.get(&it.id.to_bits()) else {
                continue;
            };
            let Some(kind) = kind_of(&it.shape) else {
                continue;
            };
            let rank = self
                .labelling
                .layers
                .get(it.layer as usize)
                .and_then(Option::as_ref)
                .map_or(u32::MAX, |l| l.rank);
            let point_px = self
                .labelling
                .layers
                .get(it.layer as usize)
                .and_then(Option::as_ref)
                .map_or(0.0, |l| l.point);
            let pins = self.labelling.pins.get(&it.id.to_bits());
            for (j, (class, text)) in labels.texts.iter().enumerate() {
                let Some(cls) = self.class_of(it, *class) else {
                    continue;
                };
                if text.is_empty() || !cls.shown_at(scale) {
                    continue;
                }
                if cls
                    .min_feature_px
                    .is_some_and(|m| js_min(b.max_x - b.min_x, b.max_y - b.min_y) * scale < m)
                {
                    continue;
                }
                let pin = pins.and_then(|ps| {
                    ps.iter()
                        .find(|p| p.class == cls.name || (p.class.is_none() && j == 0))
                });
                let hidden = pin.is_some_and(|p| p.hidden);
                let pinned = pin.and_then(|p| {
                    let at = p.at?;
                    let anchor = entity_anchor(&label_part(&it.shape))?;
                    Some(Pinned {
                        at: page.px(Vec2::new(anchor.x + at.x, anchor.y + at.y)),
                        angle: p.rotation.to_radians(),
                    })
                });
                let unit = |geo: Geo, chunk: u32, target: Option<Vec2>| Unit {
                    id: it.id,
                    order: it.order,
                    rank,
                    class: *class,
                    cls,
                    text,
                    chunk,
                    geo,
                    target,
                    pin: pinned,
                    hidden,
                    uphill: None,
                };
                let corner = Geo::Corner {
                    tl: page.px(Vec2::new(b.min_x, b.max_y)),
                };
                match kind {
                    Kind::Point => {
                        let p = match &it.shape {
                            Shape::Point { p, .. } | Shape::Insert { p, .. } => page.px(*p),
                            _ => continue,
                        };
                        if cls.point == PointMode::Corner {
                            units.push(unit(corner, 0, None));
                        } else {
                            let r = if matches!(it.shape, Shape::Point { .. }) {
                                point_px / 2.0
                            } else {
                                0.0
                            };
                            units.push(unit(Geo::Point { p, r }, 0, Some(p)));
                        }
                    }
                    Kind::Line => {
                        if cls.line == LineMode::Corner {
                            units.push(unit(corner, 0, None));
                            continue;
                        }
                        let world = entity_outline(&label_part(&it.shape), 64.0);
                        if cls.merge_lines && pinned.is_none() && !hidden {
                            to_merge.push(Merging {
                                key: (it.layer, *class, text.as_str()),
                                item: it,
                                world,
                                cls,
                            });
                            continue;
                        }
                        let px: Vec<Vec2> = world.iter().map(|&p| page.px(p)).collect();
                        let start = units.len();
                        path_units(
                            &px,
                            cls,
                            rect,
                            false,
                            pinned.is_some() || hidden,
                            &unit,
                            &mut units,
                        );
                        if cls.line == LineMode::Contour {
                            let size = cls.size_at(scale);
                            let up = contours.get(&it.layer).and_then(|c| {
                                uphill(&Walk::new(&px), labels.z, it.id, PROBE * size, c)
                            });
                            for u in &mut units[start..] {
                                u.uphill = up;
                            }
                        }
                    }
                    Kind::Area => {
                        if cls.area == AreaMode::Corner {
                            units.push(unit(corner, 0, None));
                            continue;
                        }
                        let rings: Vec<Vec<Vec2>> = area_rings(&it.shape)
                            .iter()
                            .map(|r| r.iter().map(|&p| page.px(p)).collect::<Vec<Vec2>>())
                            .collect();
                        if matches!(cls.area, AreaMode::Perimeter | AreaMode::Boundary) {
                            let mut ring = rings[0].clone();
                            if signed_area(&ring) < 0.0 {
                                ring.reverse();
                            }
                            if let Some(&first) = ring.first() {
                                ring.push(first);
                            }
                            path_units(
                                &ring,
                                cls,
                                rect,
                                true,
                                pinned.is_some() || hidden,
                                &unit,
                                &mut units,
                            );
                            continue;
                        }
                        // A ring inside the window's margin is as clipping would leave it.
                        let clipped: Vec<Vec<Vec2>> = rings
                            .into_iter()
                            .map(|r| {
                                if r.iter().all(|p| {
                                    p.x >= rect[0]
                                        && p.x <= rect[2]
                                        && p.y >= rect[1]
                                        && p.y <= rect[3]
                                }) {
                                    r
                                } else {
                                    clip_ring(&r, rect)
                                }
                            })
                            .filter(|r| r.len() >= 3)
                            .collect();
                        if clipped.is_empty() {
                            continue;
                        }
                        let Some((center, _)) = pole(&clipped, 1.0) else {
                            continue;
                        };
                        let mut bbox = [
                            f64::INFINITY,
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                            f64::NEG_INFINITY,
                        ];
                        for p in &clipped[0] {
                            bbox = [
                                js_min(bbox[0], p.x),
                                js_min(bbox[1], p.y),
                                js_max(bbox[2], p.x),
                                js_max(bbox[3], p.y),
                            ];
                        }
                        units.push(unit(
                            Geo::Area {
                                rings: clipped,
                                pole: center,
                                bbox,
                            },
                            0,
                            Some(center),
                        ));
                    }
                }
            }
        }
        // Merged lines: one path a chain, labelled as its first piece (docs/adr/0212 §3.3).
        let mut groups: Vec<((u32, u16, &str), Pieces)> = Vec::new();
        for (k, m) in to_merge.iter().enumerate() {
            match groups.iter_mut().find(|(g, _)| *g == m.key) {
                Some((_, list)) => list.push((k, m.world.clone())),
                None => groups.push((m.key, vec![(k, m.world.clone())])),
            }
        }
        for (_, pieces) in groups {
            for (first, chain) in merged(&pieces) {
                let (it, cls) = (to_merge[first].item, to_merge[first].cls);
                let px: Vec<Vec2> = chain.iter().map(|&p| page.px(p)).collect();
                let labels = &self.labelling.objects[&it.id.to_bits()];
                let Some((class, text)) = labels
                    .texts
                    .iter()
                    .find(|(c, _)| *c == to_merge[first].key.1)
                    .map(|(c, t)| (*c, t.as_str()))
                else {
                    continue;
                };
                let rank = self
                    .labelling
                    .layers
                    .get(it.layer as usize)
                    .and_then(Option::as_ref)
                    .map_or(u32::MAX, |l| l.rank);
                let unit = |geo: Geo, chunk: u32, target: Option<Vec2>| Unit {
                    id: it.id,
                    order: it.order,
                    rank,
                    class,
                    cls,
                    text,
                    chunk,
                    geo,
                    target,
                    pin: None,
                    hidden: false,
                    uphill: None,
                };
                let start = units.len();
                path_units(&px, cls, rect, false, false, &unit, &mut units);
                if cls.line == LineMode::Contour {
                    let up = contours.get(&it.layer).and_then(|c| {
                        uphill(
                            &Walk::new(&px),
                            labels.z,
                            it.id,
                            PROBE * cls.size_at(scale),
                            c,
                        )
                    });
                    for u in &mut units[start..] {
                        u.uphill = up;
                    }
                }
            }
        }
        let fixed: Vec<Obb> = fixed
            .iter()
            .filter_map(|o| outline_box(&o.iter().map(|&p| page.px(p)).collect::<Vec<Vec2>>()))
            .collect();
        let scene = Scene {
            width,
            height,
            scale,
            font: self.font,
            units,
            obstacles,
            fixed,
            options: Options {
                unplaced: options.unplaced,
                hidden: options.hidden,
            },
        };
        let outcome = engine::place(&scene);
        let mut shown = Shown::default();
        for l in outcome.labels {
            let u = &scene.units[l.unit];
            let deg = |a: f64| a.to_degrees();
            let (c, angle) = match &l.drawn {
                Drawn::Straight { c, angle, .. } => (*c, *angle),
                Drawn::Curved { letters, .. } => {
                    let n = letters.len().max(1) as f64;
                    let (sx, sy) = letters
                        .iter()
                        .fold((0.0, 0.0), |(x, y), (p, _, _)| (x + p.x, y + p.y));
                    (Vec2::new(sx / n, sy / n), 0.0)
                }
            };
            let cw = page.world(c);
            shown.records.extend([
                u.id,
                LABEL_PLACED,
                cw.x,
                cw.y,
                deg(angle),
                l.w,
                l.h,
                f64::from(u.class),
                f64::from(l.state),
            ]);
            match &l.drawn {
                Drawn::Straight { c, angle, lines } => {
                    let (ux, uy) = (cos(*angle), sin(*angle));
                    for (d, text) in lines {
                        let p = Vec2::new(c.x + d.x * ux - d.y * uy, c.y + d.x * uy + d.y * ux);
                        let w = page.world(p);
                        let width =
                            crate::labels::text::line_width(text, self.font, u.cls.bold, l.size);
                        shown.records.extend([
                            u.id,
                            LABEL_PLACED_LINE,
                            w.x,
                            w.y,
                            deg(*angle),
                            l.size,
                            shown.texts.len() as f64,
                            width,
                            0.0,
                        ]);
                        shown.texts.push(text.clone());
                    }
                }
                Drawn::Curved { text, letters } => {
                    let t = shown.texts.len() as f64;
                    shown.texts.push(text.clone());
                    let chars: Vec<char> = text.chars().collect();
                    for (p, a, i) in letters {
                        let w = page.world(*p);
                        let adv = chars.get(*i as usize).map_or(0.0, |&ch| {
                            crate::labels::text::letter_width(ch, self.font, u.cls.bold, l.size)
                        });
                        shown.records.extend([
                            u.id,
                            LABEL_PLACED_LETTER,
                            w.x,
                            w.y,
                            deg(*a),
                            l.size,
                            t,
                            f64::from(*i),
                            adv,
                        ]);
                    }
                }
            }
            if let Some((from, to)) = l.callout {
                let (f, t) = (page.world(from), page.world(to));
                shown.records.extend([
                    u.id,
                    LABEL_PLACED_CALLOUT,
                    f.x,
                    f.y,
                    t.x,
                    t.y,
                    f64::from(u.class),
                    f64::from(l.state),
                    0.0,
                ]);
            }
        }
        shown
    }
}

/// A path's units: its repeated pieces, or its longest stretch in the
/// window (`single`: a pinned or hidden label is one).
fn path_units<'s>(
    px: &[Vec2],
    cls: &Class,
    rect: [f64; 4],
    outline: bool,
    single: bool,
    unit: &dyn Fn(Geo, u32, Option<Vec2>) -> Unit<'s>,
    out: &mut Vec<Unit<'s>>,
) {
    let repeat = if outline {
        cls.outline_repeat()
    } else {
        cls.repeat
    };
    match repeat {
        Some(r) if r > 0.0 && !single => {
            let walk = Walk::new(px);
            let len = walk.length();
            let mut chunks: Vec<u32> = Vec::new();
            for (from, to) in inside_stretches(&walk, rect) {
                let (k0, k1) = ((from / r).floor() as u32, (to / r).floor() as u32);
                for k in k0..=k1 {
                    if !chunks.contains(&k) {
                        chunks.push(k);
                    }
                }
            }
            chunks.sort_unstable();
            for k in chunks {
                let s0 = f64::from(k) * r;
                let s1 = js_min(s0 + r, len);
                if s1 <= s0 {
                    continue;
                }
                let target = walk.point((s0 + s1) / 2.0);
                out.push(unit(
                    Geo::Path {
                        walk: walk.clone(),
                        s0,
                        s1,
                        interior: outline,
                    },
                    k,
                    Some(target),
                ));
            }
        }
        _ => {
            let pieces = clip_path(px, rect);
            let mut best: Option<Walk> = None;
            for p in &pieces {
                let w = Walk::new(p);
                if best.as_ref().is_none_or(|b| w.length() > b.length()) {
                    best = Some(w);
                }
            }
            if let Some(walk) = best {
                let len = walk.length();
                let target = walk.point(len / 2.0);
                out.push(unit(
                    Geo::Path {
                        walk,
                        s0: 0.0,
                        s1: len,
                        interior: outline,
                    },
                    0,
                    Some(target),
                ));
            }
        }
    }
}

impl Store {
    /// An obstacle layer's object as obstacles: its points, its lines' segments, an area's edges and inside.
    fn obstacle_of(
        &self,
        it: &Item,
        weight: u8,
        boundary: bool,
        page: Page,
        point_px: f64,
        out: &mut Vec<Obstacle>,
    ) {
        let w = Some(weight);
        match kind_of(&it.shape) {
            Some(Kind::Point) => {
                let r = js_max(point_px / 2.0, SYMBOL_MIN);
                let points: Vec<Vec2> = match &it.shape {
                    Shape::Point { p, parts, .. } => std::iter::once(*p)
                        .chain(parts.iter().flatten().map(|q| q.p))
                        .collect(),
                    Shape::Insert { p, .. } => vec![*p],
                    _ => Vec::new(),
                };
                for p in points {
                    out.push(Obstacle {
                        owner: it.id,
                        weight: w,
                        shape: Ob::Circle(page.px(p), r),
                    });
                }
            }
            Some(Kind::Line) => {
                for part in crate::entity::area_parts(&it.shape).iter() {
                    let pts = entity_outline(part, 64.0);
                    for s in pts.windows(2) {
                        out.push(Obstacle {
                            owner: it.id,
                            weight: w,
                            shape: Ob::Segment(page.px(s[0]), page.px(s[1])),
                        });
                    }
                }
            }
            Some(Kind::Area) => {
                for part in crate::entity::area_parts(&it.shape).iter() {
                    let rings: Vec<Vec<Vec2>> = area_rings(part)
                        .iter()
                        .map(|r| r.iter().map(|&p| page.px(p)).collect())
                        .collect();
                    for r in &rings {
                        for i in 0..r.len() {
                            out.push(Obstacle {
                                owner: it.id,
                                weight: w,
                                shape: Ob::Segment(r[i], r[(i + 1) % r.len()]),
                            });
                        }
                    }
                    if !boundary {
                        let mut bbox = [
                            f64::INFINITY,
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                            f64::NEG_INFINITY,
                        ];
                        for p in rings.iter().flatten() {
                            bbox = [
                                js_min(bbox[0], p.x),
                                js_min(bbox[1], p.y),
                                js_max(bbox[2], p.x),
                                js_max(bbox[3], p.y),
                            ];
                        }
                        out.push(Obstacle {
                            owner: it.id,
                            weight: w,
                            shape: Ob::Area(rings, bbox),
                        });
                    }
                }
            }
            None => {}
        }
    }
}

/// A label under a point: its object, its class and its state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelHit {
    pub id: f64,
    pub class: u16,
    pub state: u32,
    /// Its frame's middle (world) and angle (degrees): where a moved or
    /// turned label starts from.
    pub at: Vec2,
    pub angle: f64,
    /// Its block's width and height, px.
    pub w: f64,
    pub h: f64,
}

/// The label under `p` (world) among a window's records (`stride` numbers
/// each, `labels` or `labels_shown`) at `scale` px per metre, within `tol`
/// px: the last drawn, the topmost. Unplaced and hidden labels count only
/// when `all`.
pub fn label_at(
    records: &[f64],
    stride: usize,
    scale: f64,
    p: Vec2,
    tol: f64,
    all: bool,
) -> Option<LabelHit> {
    if stride < crate::store::labels::LABEL_STRIDE || !(scale > 0.0) {
        return None;
    }
    let mut frame: Option<LabelHit> = None;
    let mut hit = None;
    let quiet = engine::UNPLACED | engine::HIDDEN;
    let (pad, s) = (tol / scale, scale);
    for r in records.chunks_exact(stride) {
        let what = r[1];
        if what == LABEL_PLACED {
            let f = LabelHit {
                id: r[0],
                class: r[7] as u16,
                state: r[8] as u32,
                at: Vec2::new(r[2], r[3]),
                angle: r[4],
                w: r[5],
                h: r[6],
            };
            frame = Some(f);
            if !all && f.state & quiet != 0 {
                continue;
            }
            if f.state & engine::CURVED == 0 {
                let b = Obb::new(
                    f.at,
                    r[4].to_radians(),
                    r[5] / 2.0 / s + pad,
                    r[6] / 2.0 / s + pad,
                );
                if b.contains(p) {
                    hit = Some(f);
                }
            }
        } else if what == LABEL_PLACED_LETTER
            && let Some(f) = frame
            && f.id == r[0]
            && (all || f.state & quiet == 0)
        {
            let b = Obb::new(
                Vec2::new(r[2], r[3]),
                r[4].to_radians(),
                r[8] / 2.0 / s + pad,
                r[5] / 2.0 / s + pad,
            );
            if b.contains(p) {
                hit = Some(f);
            }
        }
    }
    hit
}

/// The labels whose middle is inside the box `from`–`to` (world, either
/// corner first) among a window's records, in the order drawn, each once.
/// Unplaced and hidden labels count only when `all`.
pub fn labels_in(records: &[f64], stride: usize, from: Vec2, to: Vec2, all: bool) -> Vec<LabelHit> {
    if stride < crate::store::labels::LABEL_STRIDE {
        return Vec::new();
    }
    let (x0, x1) = (js_min(from.x, to.x), js_max(from.x, to.x));
    let (y0, y1) = (js_min(from.y, to.y), js_max(from.y, to.y));
    let quiet = engine::UNPLACED | engine::HIDDEN;
    let mut out: Vec<LabelHit> = Vec::new();
    for r in records.chunks_exact(stride) {
        if r[1] != LABEL_PLACED {
            continue;
        }
        let state = r[8] as u32;
        let (x, y) = (r[2], r[3]);
        if (!all && state & quiet != 0) || x < x0 || x > x1 || y < y0 || y > y1 {
            continue;
        }
        let hit = LabelHit {
            id: r[0],
            class: r[7] as u16,
            state,
            at: Vec2::new(x, y),
            angle: r[4],
            w: r[5],
            h: r[6],
        };
        if !out.iter().any(|h| h.id == hit.id && h.class == hit.class) {
            out.push(hit);
        }
    }
    out
}
