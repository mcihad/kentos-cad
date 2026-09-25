//! Primitives packed into GPU batches for one layer (formerly the geometry
//! side of the TypeScript `render/styledSink.ts`): equal styles share a batch,
//! geometry becomes origin-relative float32 (never absolute coordinates on
//! the GPU), and the fills are triangulated together when the layer is
//! finished. Batches come out in symbol-level order: by level, and within a
//! level fills, lines, markers, each with its box so a frame can skip what
//! is out of view. The page gives them their colours and atlas images.

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::jsmath::{js_cmp, js_hypot, js_max, stable_sort};
use kentos_geometry_core::triangulate::triangulate_many;

use super::prim::{FillPaint, Look, MarkerStyle, Sink, StrokeStyle, num};
use super::resolve::Scale;
use crate::js::number;

/// Height of a text marker's box relative to its font size (`TEXT_BOX`).
pub const TEXT_BOX: f64 = 1.25;

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
}

/// How many styles of a kind are remembered.
const RECENT: usize = 16;

/// The latest styles seen (with their scale range) and their batches, newest replacing the oldest.
struct Recent<T> {
    list: Vec<(T, Scale, usize)>,
    next: usize,
}

impl<T: PartialEq + Clone> Recent<T> {
    fn new() -> Recent<T> {
        Recent {
            list: Vec::with_capacity(RECENT),
            next: 0,
        }
    }

    fn find(&self, style: &T, scale: Scale) -> Option<usize> {
        self.list
            .iter()
            .find(|(s, sc, _)| *sc == scale && s == style)
            .map(|&(_, _, e)| e)
    }

    fn remember(&mut self, style: &T, scale: Scale, e: usize) {
        let item = (style.clone(), scale, e);
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
        }
    }

    /// Scale range of what follows (a rule's range), until changed.
    pub fn set_scale(&mut self, scale: Scale) {
        self.scale = scale;
    }

    fn entry(
        &mut self,
        key: &str,
        kind: Kind,
        level: f64,
        style: impl FnOnce() -> String,
    ) -> usize {
        let k = format!(
            "{key}|{}|{}",
            scale_key(self.scale.min),
            scale_key(self.scale.max)
        );
        if let Some(&i) = self.index.get(&k) {
            return i;
        }
        self.entries.push(Entry {
            kind,
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
    /// triangles (x, y relative to the origin) to its batch in queue order.
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
        let (ox, oy) = (self.origin.x, self.origin.y);
        let mut poly = 0;
        for t in idx.chunks_exact(3) {
            let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
            while poly < ends.len() && a >= ends[poly] {
                poly += 1;
            }
            let Some(&(entry, _)) = self.fills.get(poly) else {
                break;
            };
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
            json.push_str("],\"w\":");
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
        let e = match self.recent_strokes.find(style, self.scale) {
            Some(e) => e,
            None => {
                let mut key = String::from("s|");
                style.write_json(&mut key);
                let e = self.entry(&key, Kind::Stroke, style.level, || key[2..].to_string());
                self.recent_strokes.remember(style, self.scale, e);
                e
            }
        };
        let (ox, oy) = (self.origin.x, self.origin.y);
        let e = &mut self.entries[e];
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
            let (ax, ay) = (a.x - ox, a.y - oy);
            e.data.extend([ax, ay, b.x - ox, b.y - oy, d, ends]);
            e.grow(ax, ay);
            d += len;
        }
        // The last point of an open path is no segment's start.
        if n > 0 {
            let p = path[if closed { 0 } else { n - 1 }];
            e.grow(p.x - ox, p.y - oy);
        }
    }

    fn fill(&mut self, paint: &FillPaint, rings: &[Vec<Vec2>]) {
        if rings.first().is_none_or(|r| r.len() < 3) {
            return;
        }
        let e = match self.recent_fills.find(paint, self.scale) {
            Some(e) => e,
            None => {
                let mut key = String::from("f|");
                paint.write_json(&mut key);
                let e = self.entry(&key, Kind::Fill, paint.level(), || key[2..].to_string());
                self.recent_fills.remember(paint, self.scale, e);
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
        let e = match self.recent_markers.find(style, self.scale) {
            Some(e) => e,
            None => {
                let key = style.key();
                let e = self.entry(&key, Kind::Marker, style.common.level, || {
                    let mut s = String::new();
                    style.write_json(&mut s);
                    s
                });
                self.recent_markers.remember(style, self.scale, e);
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
        e.data.extend([x, y, angle + style.common.rotation, w, h]);
        e.grow(x, y);
        if w > e.w {
            e.w = w;
        }
        if h > e.h {
            e.h = h;
        }
    }
}
