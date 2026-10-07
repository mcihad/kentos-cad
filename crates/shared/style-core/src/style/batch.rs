//! Primitives packed into GPU batches for one layer (formerly the geometry
//! side of the TypeScript `render/styledSink.ts`): equal styles share a batch,
//! geometry becomes origin-relative float32 (never absolute coordinates on
//! the GPU), and the fills are triangulated together when the layer is
//! finished. Batches come out in symbol-level order: by level, and within a
//! level fills, lines, markers, each with its box so a frame can skip what
//! is out of view. The page gives them their colours and atlas images.
//!
//! Positions are relative to their tile (docs/adr/0157): a grid of
//! [`TILE`]-metre squares centred on the drawing's anchor. A primitive takes
//! the tile of its first point, and a style's primitives in two tiles are two
//! batches; a batch outside the anchor's tile says its tile's origin
//! (`origin`, from the anchor). So float32 loses no more than a tile's size
//! allows, however far the drawing lies from its anchor; around the anchor
//! (±32 km) nothing changes.

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::jsmath::{cos, js_cmp, js_hypot, js_max, js_round, sin, stable_sort};
use kentos_geometry_core::triangulate::triangulate_many;

use super::prim::{FillPaint, Look, MarkerStyle, Sink, StrokeStyle, num};
use super::resolve::Scale;
use crate::js::number;

/// Height of a text marker's box relative to its font size (`TEXT_BOX`).
pub const TEXT_BOX: f64 = 1.25;

/// The side of the tiles positions are packed against, metres (docs/adr/0157):
/// a tile's origin is a whole multiple of it from the anchor, exact in float32.
pub const TILE: f64 = 65536.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Fill = 0,
    Stroke = 1,
    Marker = 2,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Fill => "fill",
            Kind::Stroke => "stroke",
            Kind::Marker => "marker",
        }
    }
}

/// A batch while it is being filled.
struct Entry {
    kind: Kind,
    /// Its tile, and the tile's origin (absolute) its numbers are relative to.
    tile: [f64; 2],
    base: Vec2,
    /// The first style's JSON (the page builds the batch's look from it).
    style: String,
    scale: Scale,
    level: f64,
    order: usize,
    data: Vec<f64>,
    bounds: [f64; 4],
    /// Largest marker width and height (markers), or half the stroke width (strokes).
    w: f64,
    h: f64,
}

impl Entry {
    fn grow(&mut self, x: f64, y: f64) {
        let b = &mut self.bounds;
        if x < b[0] {
            b[0] = x;
        }
        if y < b[1] {
            b[1] = y;
        }
        if x > b[2] {
            b[2] = x;
        }
        if y > b[3] {
            b[3] = y;
        }
    }
}

pub struct BatchSink {
    origin: Vec2,
    entries: Vec<Entry>,
    index: HashMap<String, usize>,
    scale: Scale,
    /// Fills waiting for the triangulation: the entry and the rings.
    fills: Vec<(usize, Vec<Vec<Vec2>>)>,
    /// The latest styles of each kind and their batches: the objects of a
    /// layer mostly share a few styles (markers along a line always share
    /// one, categories take turns), and building a key for each primitive
    /// would cost more than the primitive.
    recent_strokes: Recent<StrokeStyle>,
    recent_fills: Recent<FillPaint>,
    recent_markers: Recent<MarkerStyle>,
    /// Görünüm kipleri (docs/adr/0195): the fills (but pictures) and the
    /// strokes of the object being drawn are left out.
    hide_fills: bool,
    hide_strokes: bool,
}

/// How many styles of a kind are remembered.
const RECENT: usize = 16;

/// The latest styles seen (with their scale range and tile) and their
/// batches, newest replacing the oldest.
struct Recent<T> {
    list: Vec<(T, Scale, [f64; 2], usize)>,
    next: usize,
}

impl<T: PartialEq + Clone> Recent<T> {
    fn new() -> Recent<T> {
        Recent {
            list: Vec::with_capacity(RECENT),
            next: 0,
        }
    }

    fn find(&self, style: &T, scale: Scale, tile: [f64; 2]) -> Option<usize> {
        self.list
            .iter()
            .find(|(s, sc, t, _)| *sc == scale && *t == tile && s == style)
            .map(|&(_, _, _, e)| e)
    }

    fn remember(&mut self, style: &T, scale: Scale, tile: [f64; 2], e: usize) {
        let item = (style.clone(), scale, tile, e);
        if self.list.len() < RECENT {
            self.list.push(item);
        } else {
            self.list[self.next] = item;
            self.next = (self.next + 1) % RECENT;
        }
    }
}

/// The batches of a layer: a JSON array describing them and their numbers
/// one after another (`from`, `len` in each description).
pub struct Batches {
    pub json: String,
    pub data: Vec<f32>,
}

fn scale_key(x: Option<f64>) -> String {
    x.map_or_else(String::new, number::to_string)
}

impl BatchSink {
    pub fn new(origin: Vec2) -> BatchSink {
        BatchSink {
            origin,
            entries: Vec::new(),
            index: HashMap::new(),
            scale: Scale::default(),
            fills: Vec::new(),
            recent_strokes: Recent::new(),
            recent_fills: Recent::new(),
            recent_markers: Recent::new(),
            hide_fills: false,
            hide_strokes: false,
        }
    }

    /// What of the next object is left out (Görünüm kipleri, docs/adr/0195):
    /// its fills but pictures, its strokes; until changed.
    pub fn hide(&mut self, fills: bool, strokes: bool) {
        self.hide_fills = fills;
        self.hide_strokes = strokes;
    }

    /// Scale range of what follows (a rule's range), until changed.
    pub fn set_scale(&mut self, scale: Scale) {
        self.scale = scale;
    }

    /// The tile of a primitive whose first point is `p`: whole tiles from
    /// the anchor, the anchor's own tile reaching ±`TILE`/2.
    fn tile_of(&self, p: Vec2) -> [f64; 2] {
        let t = |v: f64| {
            let i = js_round(v / TILE);
            // A coordinate that is no number stays with the anchor.
            if i.is_finite() { i } else { 0.0 }
        };
        [t(p.x - self.origin.x), t(p.y - self.origin.y)]
    }

    fn entry(
        &mut self,
        key: &str,
        kind: Kind,
        level: f64,
        tile: [f64; 2],
        style: impl FnOnce() -> String,
    ) -> usize {
        let k = format!(
            "{key}|{}|{}|{}|{}",
            scale_key(self.scale.min),
            scale_key(self.scale.max),
            number::to_string(tile[0]),
            number::to_string(tile[1])
        );
        if let Some(&i) = self.index.get(&k) {
            return i;
        }
        self.entries.push(Entry {
            kind,
            tile,
            base: Vec2::new(
                self.origin.x + tile[0] * TILE,
                self.origin.y + tile[1] * TILE,
            ),
            style: style(),
            scale: self.scale,
            level,
            order: self.entries.len(),
            data: Vec::new(),
            bounds: [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            w: 0.0,
            h: 0.0,
        });
        self.index.insert(k, self.entries.len() - 1);
        self.entries.len() - 1
    }

    /// Triangulates the queued fills in one call, appending each fill's
    /// triangles (x, y relative to its tile) to its batch in queue order.
    fn triangulate(&mut self) {
        if self.fills.is_empty() {
            return;
        }
        let mut pts = Vec::new();
        let mut ring_sizes = Vec::new();
        let mut poly_rings = Vec::with_capacity(self.fills.len());
        let mut ends = Vec::with_capacity(self.fills.len());
        for (_, rings) in &self.fills {
            poly_rings.push(rings.len());
            for r in rings {
                ring_sizes.push(r.len());
                pts.extend_from_slice(r);
            }
            ends.push(pts.len());
        }
        let idx = triangulate_many(&pts, &ring_sizes, &poly_rings);
        let mut poly = 0;
        for t in idx.chunks_exact(3) {
            let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
            while poly < ends.len() && a >= ends[poly] {
                poly += 1;
            }
            let Some(&(entry, _)) = self.fills.get(poly) else {
                break;
            };
            let (ox, oy) = (self.entries[entry].base.x, self.entries[entry].base.y);
            self.entries[entry].data.extend([
                pts[a].x - ox,
                pts[a].y - oy,
                pts[b].x - ox,
                pts[b].y - oy,
                pts[c].x - ox,
                pts[c].y - oy,
            ]);
        }
        self.fills.clear();
    }

    /// The batches, in draw order; those without geometry are left out.
    pub fn finish(mut self) -> Batches {
        self.triangulate();
        let mut order: Vec<usize> = (0..self.entries.len()).collect();
        stable_sort(&mut order, &mut |&a, &b| {
            let (a, b) = (&self.entries[a], &self.entries[b]);
            js_cmp(a.level, b.level)
                .then(a.kind.cmp(&b.kind))
                .then(a.order.cmp(&b.order))
        });
        let mut json = String::from("[");
        let mut data: Vec<f32> = Vec::new();
        for i in order {
            let e = &self.entries[i];
            let enough = match e.kind {
                Kind::Stroke | Kind::Fill => e.data.len() >= 6,
                Kind::Marker => e.data.len() >= 5,
            };
            if !enough {
                continue;
            }
            if json.len() > 1 {
                json.push(',');
            }
            json.push_str("{\"kind\":\"");
            json.push_str(e.kind.name());
            json.push_str("\",\"style\":");
            json.push_str(&e.style);
            if let Some(min) = e.scale.min {
                json.push_str(",\"minScale\":");
                num(&mut json, min);
            }
            if let Some(max) = e.scale.max {
                json.push_str(",\"maxScale\":");
                num(&mut json, max);
            }
            json.push_str(",\"from\":");
            num(&mut json, data.len() as f64);
            json.push_str(",\"len\":");
            num(&mut json, e.data.len() as f64);
            json.push_str(",\"bounds\":[");
            for (k, b) in e.bounds.iter().enumerate() {
                if k > 0 {
                    json.push(',');
                }
                num(&mut json, *b);
            }
            json.push(']');
            if e.tile != [0.0, 0.0] {
                json.push_str(",\"origin\":[");
                num(&mut json, e.tile[0] * TILE);
                json.push(',');
                num(&mut json, e.tile[1] * TILE);
                json.push(']');
            }
            json.push_str(",\"w\":");
            num(&mut json, e.w);
            json.push_str(",\"h\":");
            num(&mut json, e.h);
            json.push('}');
            data.extend(e.data.iter().map(|&x| x as f32));
        }
        json.push(']');
        Batches { json, data }
    }
}

impl Sink for BatchSink {
    fn stroke(&mut self, style: &StrokeStyle, path: &[Vec2], closed: bool) {
        if self.hide_strokes {
            return;
        }
        // An empty path still makes its batch, as before tiles (the batches' order).
        let tile = path.first().map_or([0.0, 0.0], |&p| self.tile_of(p));
        let e = match self.recent_strokes.find(style, self.scale, tile) {
            Some(e) => e,
            None => {
                let mut key = String::from("s|");
                style.write_json(&mut key);
                let e = self.entry(&key, Kind::Stroke, style.level, tile, || {
                    key[2..].to_string()
                });
                self.recent_strokes.remember(style, self.scale, tile, e);
                e
            }
        };
        // The numbers from the tile's origin; the box from the anchor.
        let (ox, oy) = (self.origin.x, self.origin.y);
        let e = &mut self.entries[e];
        let (tx, ty) = (e.base.x, e.base.y);
        e.w = js_max(e.w, style.width / 2.0 + style.blur);
        let n = path.len();
        let count = if closed { n } else { n.saturating_sub(1) };
        let mut d = 0.0;
        for i in 0..count {
            let a = path[i];
            let b = path[(i + 1) % n];
            let len = js_hypot(b.x - a.x, b.y - a.y);
            if len < 1e-12 {
                continue;
            }
            let ends = if !closed && i == 0 { 1.0 } else { 0.0 }
                + if !closed && i + 1 == count { 2.0 } else { 0.0 };
            e.data
                .extend([a.x - tx, a.y - ty, b.x - tx, b.y - ty, d, ends]);
            e.grow(a.x - ox, a.y - oy);
            d += len;
        }
        // The last point of an open path is no segment's start.
        if n > 0 {
            let p = path[if closed { 0 } else { n - 1 }];
            e.grow(p.x - ox, p.y - oy);
        }
    }

    fn fill(&mut self, paint: &FillPaint, rings: &[Vec<Vec2>]) {
        if rings.first().is_none_or(|r| r.len() < 3)
            || (self.hide_fills && !matches!(paint, FillPaint::Image { .. }))
        {
            return;
        }
        // A gradient's frame is its area's (docs/adr/0186 §3): from the anchor, as the positions
        // are; the page moves it on to the batch's tile with the positions.
        let anchored;
        let paint = match paint {
            FillPaint::Gradient {
                color,
                color2,
                opacity,
                shape,
                inverted,
                dir,
                from,
                to,
                centre,
                radius,
                level,
            } => {
                let along = cos(*dir) * self.origin.x + sin(*dir) * self.origin.y;
                anchored = FillPaint::Gradient {
                    color: color.clone(),
                    color2: color2.clone(),
                    opacity: *opacity,
                    shape: *shape,
                    inverted: *inverted,
                    dir: *dir,
                    from: from - along,
                    to: to - along,
                    centre: [centre[0] - self.origin.x, centre[1] - self.origin.y],
                    radius: *radius,
                    level: *level,
                };
                &anchored
            }
            // A picture's frame likewise, from the anchor (docs/adr/0192 §3).
            FillPaint::Image {
                image,
                corner,
                size,
                angle,
                mirror,
                opacity,
                level,
            } => {
                anchored = FillPaint::Image {
                    image: image.clone(),
                    corner: [corner[0] - self.origin.x, corner[1] - self.origin.y],
                    size: *size,
                    angle: *angle,
                    mirror: *mirror,
                    opacity: *opacity,
                    level: *level,
                };
                &anchored
            }
            _ => paint,
        };
        let tile = self.tile_of(rings[0][0]);
        let e = match self.recent_fills.find(paint, self.scale, tile) {
            Some(e) => e,
            None => {
                let mut key = String::from("f|");
                paint.write_json(&mut key);
                let e = self.entry(&key, Kind::Fill, paint.level(), tile, || {
                    key[2..].to_string()
                });
                self.recent_fills.remember(paint, self.scale, tile, e);
                e
            }
        };
        let (ox, oy) = (self.origin.x, self.origin.y);
        for p in &rings[0] {
            self.entries[e].grow(p.x - ox, p.y - oy);
        }
        self.fills.push((e, rings.to_vec()));
    }

    fn marker(&mut self, style: &MarkerStyle, at: Vec2, angle: f64) {
        let tile = self.tile_of(at);
        let e = match self.recent_markers.find(style, self.scale, tile) {
            Some(e) => e,
            None => {
                let key = style.key();
                let e = self.entry(&key, Kind::Marker, style.common.level, tile, || {
                    let mut s = String::new();
                    style.write_json(&mut s);
                    s
                });
                self.recent_markers.remember(style, self.scale, tile, e);
                e
            }
        };
        let (w, h) = match &style.look {
            Look::Text { size, .. } => (0.0, size * TEXT_BOX),
            Look::Shape { size, height, .. } => (*size, *height),
            Look::Svg { size, .. } | Look::Raster { size, .. } => (*size, 0.0),
        };
        let (x, y) = (at.x - self.origin.x, at.y - self.origin.y);
        let e = &mut self.entries[e];
        e.data.extend([
            at.x - e.base.x,
            at.y - e.base.y,
            angle + style.common.rotation,
            w,
            h,
        ]);
        e.grow(x, y);
        if w > e.w {
            e.w = w;
        }
        if h > e.h {
            e.h = h;
        }
    }
}

#[cfg(test)]
mod tests {
    //! Tiles (docs/adr/0157): a style's primitives near the anchor and
    //! 4 400 km from it are two batches, the far one packed from its tile and
    //! saying the tile's origin; near the anchor nothing changes.
    use super::*;
    use crate::style::prim::PrimUnit;

    fn line() -> StrokeStyle {
        StrokeStyle {
            color: "#2E7D32".into(),
            opacity: 1.0,
            width: 0.35,
            unit: PrimUnit::World,
            dash: None,
            dash_offset: 0.0,
            cap: "butt".into(),
            join: "miter".into(),
            blur: 0.0,
            level: 0.0,
        }
    }

    const ANCHOR: Vec2 = Vec2 {
        x: 487_100.0,
        y: 4_420_200.0,
    };

    /// A gradient's frame (docs/adr/0186 §3) is written from the anchor, as the positions are: the
    /// share of a vertex along it is the share of its world point.
    #[test]
    fn a_gradient_s_frame_is_written_from_the_anchor() {
        let mut sink = BatchSink::new(ANCHOR);
        let ring = vec![
            Vec2::new(487_110.0, 4_420_210.0),
            Vec2::new(487_124.0, 4_420_210.0),
            Vec2::new(487_124.0, 4_420_220.0),
        ];
        let dir = 0.5_f64;
        let along = |p: Vec2| cos(dir) * p.x + sin(dir) * p.y;
        let paint = FillPaint::Gradient {
            color: "#3E63DD".into(),
            color2: "#FFFFFF".into(),
            opacity: 1.0,
            shape: 0,
            inverted: false,
            dir,
            from: along(ring[0]),
            to: along(ring[2]),
            centre: [487_117.0, 4_420_215.0],
            radius: 8.0,
            level: 0.0,
        };
        sink.fill(&paint, std::slice::from_ref(&ring));
        let out = sink.finish();
        let batches: serde_json::Value = serde_json::from_str(&out.json).expect("reads");
        let style = &batches[0]["style"];
        let (from, to) = (
            style["from"].as_f64().unwrap(),
            style["to"].as_f64().unwrap(),
        );
        let anchored = |p: Vec2| along(Vec2::new(p.x - ANCHOR.x, p.y - ANCHOR.y));
        assert!((from - anchored(ring[0])).abs() < 1e-6, "{style}");
        assert!((to - anchored(ring[2])).abs() < 1e-6, "{style}");
        let centre: Vec<f64> = style["centre"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|x| x.as_f64())
            .collect();
        assert_eq!(centre, [17.0, 15.0]);
    }

    #[test]
    fn far_primitives_make_a_batch_of_their_own_packed_from_its_tile() {
        let mut sink = BatchSink::new(ANCHOR);
        let near = [
            Vec2::new(487_110.0, 4_420_210.0),
            Vec2::new(487_130.5, 4_420_210.0),
        ];
        let far = [Vec2::new(1062.5, 2003.25), Vec2::new(1060.75, 2041.5)];
        sink.stroke(&line(), &near, false);
        sink.stroke(&line(), &far, false);
        sink.stroke(&line(), &near, false);
        let out = sink.finish();
        let batches: serde_json::Value = serde_json::from_str(&out.json).expect("reads");
        let batches = batches.as_array().expect("an array");
        assert_eq!(batches.len(), 2, "one style, two tiles");
        // The anchor's tile: no origin, numbers from the anchor as before.
        assert!(batches[0].get("origin").is_none());
        assert_eq!(&out.data[..4], &[10.0, 10.0, 30.5, 10.0]);
        assert_eq!(batches[0]["len"], 12, "both near lines");
        // The far tile: (−7, −67) tiles from the anchor; its numbers from there.
        assert_eq!(
            batches[1]["origin"],
            serde_json::json!([-458_752, -4_390_912])
        );
        let base = [ANCHOR.x - 458_752.0, ANCHOR.y - 4_390_912.0];
        let from = batches[1]["from"].as_u64().expect("from") as usize;
        assert_eq!(
            &out.data[from..from + 4],
            &[
                (1062.5 - base[0]) as f32,
                (2003.25 - base[1]) as f32,
                (1060.75 - base[0]) as f32,
                (2041.5 - base[1]) as f32,
            ]
        );
        // The box is from the anchor, as every batch's.
        assert_eq!(batches[1]["bounds"][0].as_f64(), Some(1060.75 - ANCHOR.x));
    }

    #[test]
    fn the_anchor_s_tile_reaches_half_a_tile_each_way() {
        let sink = BatchSink::new(ANCHOR);
        let at = |dx: f64, dy: f64| sink.tile_of(Vec2::new(ANCHOR.x + dx, ANCHOR.y + dy));
        assert_eq!(at(0.0, 0.0), [0.0, 0.0]);
        assert_eq!(at(32_767.9, -32_767.9), [0.0, 0.0]);
        // A half rounds up (`Math.round`).
        assert_eq!(at(32_768.0, -32_768.0), [1.0, 0.0]);
        assert_eq!(at(-98_304.1, 0.0), [-2.0, 0.0]);
        // A coordinate that is no number stays with the anchor.
        assert_eq!(sink.tile_of(Vec2::new(f64::NAN, f64::INFINITY)), [0.0, 0.0]);
    }
}
