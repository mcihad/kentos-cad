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
use std::sync::Arc;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::signed_area;
use kentos_geometry_core::jsmath::{cos, js_cmp, js_hypot, js_max, js_round, sin, stable_sort};
use kentos_geometry_core::triangulate::triangulate_many;

use super::prim::{EffectSpec, FillPaint, Look, MarkerStyle, Sink, StrokeStyle, group_key, num};
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
    /// The draw order band of its objects (docs/adr/0260 §3): bands draw one
    /// after another, each with its own fills under its own strokes.
    band: i32,
    order: usize,
    data: Vec<f64>,
    bounds: [f64; 4],
    /// Largest marker width and height (markers), or half the stroke width (strokes).
    w: f64,
    h: f64,
    /// Its effect group (from 1; 0 none) and the group's effects (docs/adr/0226 §2).
    group: u32,
    effect: Option<String>,
    /// Its width measured on the ground (docs/adr/0793 §1).
    ground: bool,
    /// The key it was asked for (before its scale, tile, group and band), so an instanced batch finds its sibling.
    raw: String,
    /// 0, or 1 for an instanced batch: it draws right after its sibling, the batch of the same style (docs/adr/1063 §2).
    sub: u8,
    /// An instanced batch: the definition and which of its batches; its `data` are then the instances, each
    /// [`INSTANCE_FLOATS`] numbers (docs/adr/1063 §2).
    inst: Option<(Arc<Def>, usize)>,
    /// Largest uniform scale of its instances (an instanced batch's), for the curves' detail level.
    kmax: f64,
    /// A curve's strokes by detail level (docs/adr/1063 §3): level 0 the fine chords, then one per
    /// [`CURVE_LEVELS`]; empty when no curve came with levels.
    lods: Vec<Vec<f64>>,
}

/// Numbers per instance of an instanced batch: the placement's linear part by columns (x axis, y axis), its
/// translation from the batch's tile, its uniform scale, 0 (docs/adr/1063 §2).
pub const INSTANCE_FLOATS: usize = 8;

/// The curves' coarser detail levels (docs/adr/1063 §3): level `k + 1` is drawn where the view's logical px per metre
/// (times an instanced batch's largest scale) is at most `CURVE_LEVELS[k]`; above the first, level 0 (the fine chords,
/// as before). Each level's chords keep within [`CURVE_TOLERANCE_PX`] of the curve at its ceiling.
pub const CURVE_LEVELS: [f64; 3] = [4.0, 1.0, 0.25];
/// Largest distance between a curve and a coarse level's chords at the level's ceiling, logical pixels (the drawing
/// area's own tolerance, `RenderSettings::DEFAULT_CURVE_TOLERANCE_PX`).
pub const CURVE_TOLERANCE_PX: f64 = 0.25;

/// A block definition drawn once (docs/adr/1063 §2): its batches as a layer's would be, from its base point.
#[derive(Debug)]
pub struct Def {
    pub id: usize,
    batches: Vec<DefBatch>,
}

#[derive(Debug)]
struct DefBatch {
    kind: Kind,
    raw: String,
    style: String,
    level: f64,
    ground: bool,
    data: Vec<f64>,
    lods: Vec<Vec<f64>>,
    bounds: [f64; 4],
    w: f64,
    h: f64,
    /// Its far proxy (docs/adr/1065 §2): the box round its strokes as four segments, drawn instead of them where the
    /// largest insert shows the definition at most [`BLOCK_PROXY_PX`]; empty for areas and for a batch of few
    /// strokes. `extent` is the box's larger side, in the definition's units.
    proxy: Vec<f64>,
    extent: f64,
}

/// The most logical pixels a block definition's strokes span on the screen (at the largest insert's scale) for its
/// far proxy to be drawn instead of them (docs/adr/1065 §2).
pub const BLOCK_PROXY_PX: f64 = 3.0;
/// The fewest segments a definition's batch draws at its coarsest level before it comes with a proxy: the proxy's own
/// four would save little below this.
const PROXY_LEAST_SEGMENTS: usize = 12;

/// The box `b` (a batch's, from the definition's base) as the four segments of a stroke batch, start lengths 0 and no
/// end caps (a closed path's); a box that is a point is one segment of no length (a dot).
fn proxy_box(b: &[f64; 4]) -> Vec<f64> {
    let [x0, y0, x1, y1] = *b;
    let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    let mut out = Vec::with_capacity(4 * 6);
    for i in 0..4 {
        let (a, c) = (corners[i], corners[(i + 1) % 4]);
        if js_hypot(c.0 - a.0, c.1 - a.1) >= 1e-12 {
            out.extend([a.0, a.1, c.0, c.1, 0.0, 0.0]);
        }
    }
    if out.is_empty() {
        out.extend([x0, y0, x0, y0, 0.0, 0.0]);
    }
    out
}

impl Entry {
    fn grow(&mut self, x: f64, y: f64) {
        grow_box(&mut self.bounds, x, y);
    }
}

fn grow_box(b: &mut [f64; 4], x: f64, y: f64) {
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

pub struct BatchSink {
    /// The map's system (docs/adr/0583 §3): what comes in is moved there first.
    map: Option<&'static kentos_geometry_core::crs::map::MapProjection>,
    origin: Vec2,
    position_low: bool,
    entries: Vec<Entry>,
    index: HashMap<String, usize>,
    scale: Scale,
    /// Fills waiting for the triangulation: the entry, the rings and whether a fan (`fan`).
    fills: Vec<(usize, Vec<Vec<Vec2>>, bool)>,
    /// Aggregate CAD glyph workload across the layer, before triangulation.
    cad_points: usize,
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
    pictures: Vec<Picture>,
    dropped: u64,
    /// The effect group what follows belongs to (0 none) and its effects;
    /// the groups made, by their key (docs/adr/0226 §2).
    group: u32,
    group_json: String,
    groups: HashMap<String, u32>,
    /// The draw order band of what follows (0 unless an object has its own order).
    band: i32,
    /// A definition being drawn once (`recorder`): what an instance could not draw the same refuses it.
    recording: bool,
    refused: bool,
    /// The detail level the strokes that follow go to (docs/adr/1063 §3); above 0 nothing else is kept.
    lod: Option<usize>,
    /// A long stroke drawn by itself (no curve level asked) comes with its far levels too (docs/adr/1064 §2).
    line_lod: bool,
    /// A definition being recorded comes with its far proxy (docs/adr/1065 §2).
    block_proxy: bool,
}

/// How many styles of a kind are remembered.
const RECENT: usize = 16;

/// The latest styles seen (with their scale range, tile and effect group)
/// and their batches, newest replacing the oldest.
struct Recent<T> {
    list: Vec<(T, Scale, [f64; 2], u32, usize)>,
    next: usize,
}

impl<T: PartialEq + Clone> Recent<T> {
    fn new() -> Recent<T> {
        Recent {
            list: Vec::with_capacity(RECENT),
            next: 0,
        }
    }

    fn find(&self, style: &T, scale: Scale, tile: [f64; 2], group: u32) -> Option<usize> {
        self.list
            .iter()
            .find(|(s, sc, t, g, _)| *sc == scale && *t == tile && *g == group && s == style)
            .map(|&(_, _, _, _, e)| e)
    }

    fn remember(&mut self, style: &T, scale: Scale, tile: [f64; 2], group: u32, e: usize) {
        let item = (style.clone(), scale, tile, group, e);
        if self.list.len() < RECENT {
            self.list.push(item);
        } else {
            self.list[self.next] = item;
            self.next = (self.next + 1) % RECENT;
        }
    }
}

/// The batches of a layer: a JSON array describing them and their numbers
/// one after another (`from`, `len` in each description); the pictures the
/// build made (a heat map's, docs/adr/0213 §2.6), and the dots left out
/// over the layer's limit (§2.4).
pub struct Batches {
    pub json: String,
    pub data: Vec<f32>,
    /// Residual of each packed value after its float32 high part. Legacy data/layout is unchanged.
    pub low: Vec<f32>,
    pub pictures: Vec<Picture>,
    pub dropped: u64,
    /// The batches (by their place in `json`) whose stroke or hatch width is measured on the ground, not paper mm
    /// (docs/adr/0793 §1): the model screen never holds it to its paper width.
    pub ground: Vec<u32>,
}

/// A picture a build made: the key a batch draws it by, its size, its
/// pixels (RGBA, straight alpha, rows from the top).
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    pub key: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

fn scale_key(x: Option<f64>) -> String {
    x.map_or_else(String::new, number::to_string)
}

impl BatchSink {
    pub fn new(origin: Vec2) -> BatchSink {
        BatchSink {
            origin,
            position_low: false,
            entries: Vec::new(),
            index: HashMap::new(),
            scale: Scale::default(),
            fills: Vec::new(),
            cad_points: 0,
            recent_strokes: Recent::new(),
            recent_fills: Recent::new(),
            recent_markers: Recent::new(),
            hide_fills: false,
            hide_strokes: false,
            pictures: Vec::new(),
            dropped: 0,
            group: 0,
            group_json: String::new(),
            groups: HashMap::new(),
            band: 0,
            map: None,
            recording: false,
            refused: false,
            lod: None,
            line_lod: false,
            block_proxy: false,
        }
    }

    /// A sink a block definition is drawn into once, from its base point (docs/adr/1063 §2): [`Self::into_def`] gives
    /// its batches unless something of it would not draw the same placed by any similarity (a dash or line type, a
    /// pattern, a marker, an effect, a scale range).
    pub fn recorder() -> BatchSink {
        let mut s = BatchSink::new(Vec2::new(0.0, 0.0));
        s.recording = true;
        s
    }

    /// Refuses the definition being recorded.
    pub fn refuse(&mut self) {
        self.refused = true;
    }

    /// The detail level of the strokes that follow (docs/adr/1063 §3): `Some(0)` the fine chords of a curve that
    /// also comes coarser, `Some(k)` its level `k` (nothing but strokes kept), none what has no levels.
    pub fn set_lod(&mut self, lod: Option<usize>) {
        self.lod = lod;
    }

    /// Whether a long stroke comes with its far levels (docs/adr/1064 §2): the screen's drawing area only, never paper.
    pub fn set_line_lod(&mut self, on: bool) {
        self.line_lod = on;
    }

    /// Whether the definition being recorded comes with its far proxy (docs/adr/1065 §2): the drawing area only.
    pub fn set_block_proxy(&mut self, on: bool) {
        self.block_proxy = on;
    }

    /// The recorded definition's batches, in the order they were met; none when refused.
    pub fn into_def(mut self, id: usize) -> Option<Def> {
        if self.refused || self.group != 0 {
            return None;
        }
        self.triangulate();
        let proxies = self.block_proxy;
        let mut batches = Vec::with_capacity(self.entries.len());
        for e in self.entries {
            let lods: usize = e.lods.iter().map(Vec::len).sum();
            if e.data.len() + lods < 6 {
                continue;
            }
            if e.tile != [0.0, 0.0] || e.kind == Kind::Marker || e.group != 0 || e.scale != Scale::default() {
                return None;
            }
            // A far proxy for strokes that draw many segments even at their coarsest level (docs/adr/1065 §2).
            let coarsest = e.data.len() + e.lods.last().map_or(0, Vec::len);
            let boxed = e.bounds.iter().all(|v| v.is_finite()) && e.bounds[0] <= e.bounds[2] && e.bounds[1] <= e.bounds[3];
            let (proxy, extent) = if proxies && e.kind == Kind::Stroke && boxed && coarsest / 6 >= PROXY_LEAST_SEGMENTS {
                (proxy_box(&e.bounds), js_max(e.bounds[2] - e.bounds[0], e.bounds[3] - e.bounds[1]))
            } else {
                (Vec::new(), 0.0)
            };
            batches.push(DefBatch {
                kind: e.kind,
                raw: e.raw,
                style: e.style,
                level: e.level,
                ground: e.ground,
                data: e.data,
                lods: e.lods,
                bounds: e.bounds,
                w: e.w,
                h: e.h,
                proxy,
                extent,
            });
        }
        Some(Def { id, batches })
    }

    /// An insert of a recorded definition, placed by the similarity `m` of uniform scale `k` (docs/adr/1063 §2):
    /// each of the definition's batches gets an instance in the batch instanced from it, which draws right after the
    /// batch of its style (where the insert's pieces would have gone).
    /// False, and nothing drawn, inside an effect group or on a map of another system (the insert is then drawn
    /// piece by piece).
    pub fn instance(&mut self, def: &Arc<Def>, m: &[f64; 6], k: f64) -> bool {
        if self.group != 0 || self.map.is_some() || self.recording {
            return false;
        }
        // Its batches have no scale range, as the definition's had none.
        let scale = std::mem::take(&mut self.scale);
        self.place(def, m, k);
        self.scale = scale;
        true
    }

    fn place(&mut self, def: &Arc<Def>, m: &[f64; 6], k: f64) {
        let t = Vec2::new(m[4], m[5]);
        let tile = self.tile_of(t);
        let (ox, oy) = (self.origin.x, self.origin.y);
        for (j, b) in def.batches.iter().enumerate() {
            let sibling = self.entry(&b.raw, b.kind, b.level, tile, b.ground, || b.style.clone());
            let order = self.entries[sibling].order;
            let key = format!(
                "i{}#{j}|{}|{}|{}",
                def.id,
                number::to_string(tile[0]),
                number::to_string(tile[1]),
                self.band
            );
            let e = match self.index.get(&key) {
                Some(&e) => e,
                None => {
                    let base = self.entries[sibling].base;
                    self.entries.push(Entry {
                        kind: b.kind,
                        tile,
                        base,
                        style: b.style.clone(),
                        scale: Scale::default(),
                        level: b.level,
                        band: self.band,
                        order,
                        data: Vec::new(),
                        bounds: [
                            f64::INFINITY,
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                            f64::NEG_INFINITY,
                        ],
                        w: b.w,
                        h: b.h,
                        group: 0,
                        effect: None,
                        ground: b.ground,
                        raw: b.raw.clone(),
                        sub: 1,
                        inst: Some((def.clone(), j)),
                        kmax: 0.0,
                        lods: Vec::new(),
                    });
                    self.index.insert(key, self.entries.len() - 1);
                    self.entries.len() - 1
                }
            };
            let e = &mut self.entries[e];
            e.data.extend([m[0], m[1], m[2], m[3], t.x - e.base.x, t.y - e.base.y, k, 0.0]);
            e.kmax = js_max(e.kmax, k);
            let [x0, y0, x1, y1] = b.bounds;
            for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
                e.grow(m[0] * x + m[2] * y + m[4] - ox, m[1] * x + m[3] * y + m[5] - oy);
            }
        }
    }

    /// The map's system what follows is drawn in (docs/adr/0583 §3); none: the project's.
    pub fn set_map(&mut self, map: Option<&'static kentos_geometry_core::crs::map::MapProjection>) {
        self.map = map;
    }

    /// The draw order band of what follows (docs/adr/0260 §3), until changed:
    /// an object brought to the front draws over the strokes of those behind
    /// it, its fill too. The styles remembered are forgotten on a change, so
    /// no batch takes primitives of two bands.
    pub fn set_band(&mut self, band: i32) {
        if band != self.band {
            self.band = band;
            self.recent_strokes = Recent::new();
            self.recent_fills = Recent::new();
            self.recent_markers = Recent::new();
        }
    }

    /// Emit a residual companion stream for backends which bind precise positions.
    pub fn set_position_low(&mut self, enabled: bool) {
        self.position_low = enabled;
    }

    /// A picture the build made, drawn by its key.
    pub fn picture(&mut self, picture: Picture) {
        self.pictures.push(picture);
    }

    /// Dots left out over the layer's limit.
    pub fn drop_dots(&mut self, n: u64) {
        self.dropped += n;
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
        ground: bool,
        style: impl FnOnce() -> String,
    ) -> usize {
        let raw = key.to_owned();
        // A ground width and a paper one never share a batch, though their styles read alike (docs/adr/0793 §1).
        let key = if ground { format!("{key}|g") } else { key.to_owned() };
        let k = format!(
            "{key}|{}|{}|{}|{}|{}",
            scale_key(self.scale.min),
            scale_key(self.scale.max),
            number::to_string(tile[0]),
            number::to_string(tile[1]),
            self.group
        );
        let k = if self.band == 0 {
            k
        } else {
            format!("{k}|b{}", self.band)
        };
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
            band: self.band,
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
            group: self.group,
            effect: (self.group > 0).then(|| self.group_json.clone()),
            ground,
            raw,
            sub: 0,
            inst: None,
            kmax: 0.0,
            lods: Vec::new(),
        });
        self.index.insert(k, self.entries.len() - 1);
        self.entries.len() - 1
    }

    /// Triangulates the queued fills in one call, appending each fill's
    /// triangles (x, y relative to its tile) to its batch in queue order; a
    /// fan's triangles are its first point's, counter-clockwise as the ear
    /// clipper's are.
    fn triangulate(&mut self) {
        if self.fills.is_empty() {
            return;
        }
        let mut pts = Vec::new();
        let mut ring_sizes = Vec::new();
        let mut poly_rings = Vec::with_capacity(self.fills.len());
        let mut ends = Vec::with_capacity(self.fills.len());
        for (_, rings, fan) in &self.fills {
            if *fan {
                continue;
            }
            poly_rings.push(rings.len());
            for r in rings {
                ring_sizes.push(r.len());
                pts.extend_from_slice(r);
            }
            ends.push(pts.len());
        }
        let idx = if poly_rings.is_empty() {
            Vec::new()
        } else {
            triangulate_many(&pts, &ring_sizes, &poly_rings)
        };
        let fills = std::mem::take(&mut self.fills);
        let (mut t, mut poly) = (0, 0);
        for (entry, rings, fan) in &fills {
            let (ox, oy) = (self.entries[*entry].base.x, self.entries[*entry].base.y);
            let data = &mut self.entries[*entry].data;
            if *fan {
                let r = &rings[0];
                let ccw = signed_area(r) >= 0.0;
                for k in 1..r.len() - 1 {
                    let (b, c) = if ccw {
                        (r[k], r[k + 1])
                    } else {
                        (r[k + 1], r[k])
                    };
                    data.extend([
                        r[0].x - ox,
                        r[0].y - oy,
                        b.x - ox,
                        b.y - oy,
                        c.x - ox,
                        c.y - oy,
                    ]);
                }
                continue;
            }
            // This polygon's triangles: the next ones whose corners are its points.
            let end = ends.get(poly).copied().unwrap_or(0);
            poly += 1;
            while t + 3 <= idx.len() && (idx[t] as usize) < end {
                let [a, b, c] = [idx[t] as usize, idx[t + 1] as usize, idx[t + 2] as usize];
                data.extend([
                    pts[a].x - ox,
                    pts[a].y - oy,
                    pts[b].x - ox,
                    pts[b].y - oy,
                    pts[c].x - ox,
                    pts[c].y - oy,
                ]);
                t += 3;
            }
        }
    }

    /// The batches, in draw order; those without geometry are left out.
    pub fn finish(mut self) -> Batches {
        self.triangulate();
        let mut order: Vec<usize> = (0..self.entries.len()).collect();
        stable_sort(&mut order, &mut |&a, &b| {
            let (a, b) = (&self.entries[a], &self.entries[b]);
            // An effect group's batches stay together (docs/adr/0226 §2); 0 is none.
            a.band
                .cmp(&b.band)
                .then(js_cmp(a.level, b.level))
                .then(a.group.cmp(&b.group))
                .then(a.kind.cmp(&b.kind))
                .then(a.order.cmp(&b.order))
                .then(a.sub.cmp(&b.sub))
        });
        let mut json = String::from("[");
        let mut data: Vec<f32> = Vec::new();
        let mut low = Vec::new();
        let mut ground = Vec::new();
        let mut placed = 0u32;
        for i in order {
            let e = &self.entries[i];
            let lods: usize = e.lods.iter().map(Vec::len).sum();
            let enough = match e.kind {
                _ if e.inst.is_some() => !e.data.is_empty(),
                Kind::Stroke | Kind::Fill => e.data.len() + lods >= 6,
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
            // An instanced batch's numbers are its definition's (its own levels with them), then its instances.
            // Its definition's far proxy after the levels (docs/adr/1065 §2), and its extent.
            let empty: &[Vec<f64>] = &[];
            let (main, levels, proxy, own): (&[f64], &[Vec<f64>], &[f64], Option<&[f64]>) = match &e.inst {
                Some((def, j)) => (
                    def.batches[*j].data.as_slice(),
                    def.batches[*j].lods.as_slice(),
                    def.batches[*j].proxy.as_slice(),
                    Some(e.data.as_slice()),
                ),
                None => (e.data.as_slice(), if e.lods.is_empty() { empty } else { e.lods.as_slice() }, &[], None),
            };
            let total = main.len()
                + levels.iter().map(Vec::len).sum::<usize>()
                + proxy.len()
                + own.map_or(0, <[f64]>::len);
            json.push_str(",\"from\":");
            num(&mut json, data.len() as f64);
            json.push_str(",\"len\":");
            num(&mut json, total as f64);
            if !levels.is_empty() {
                json.push_str(",\"lod\":[");
                num(&mut json, main.len() as f64);
                for l in levels {
                    json.push(',');
                    num(&mut json, l.len() as f64);
                }
                json.push(']');
            }
            if let Some(own) = own {
                json.push_str(",\"inst\":{\"count\":");
                num(&mut json, (own.len() / INSTANCE_FLOATS) as f64);
                json.push_str(",\"k\":");
                num(&mut json, e.kmax);
                if let Some((def, j)) = &e.inst
                    && !proxy.is_empty()
                {
                    json.push_str(",\"proxy\":[");
                    num(&mut json, proxy.len() as f64);
                    json.push(',');
                    num(&mut json, def.batches[*j].extent);
                    json.push(']');
                }
                json.push('}');
            }
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
            // Its effect group and effects (docs/adr/0226 §2).
            if let Some(effect) = &e.effect {
                json.push_str(",\"group\":");
                num(&mut json, f64::from(e.group));
                json.push_str(",\"effect\":");
                json.push_str(effect);
            }
            json.push('}');
            let all = || {
                main.iter()
                    .chain(levels.iter().flatten())
                    .chain(proxy.iter())
                    .chain(own.into_iter().flatten())
            };
            data.extend(all().map(|&x| x as f32));
            if self.position_low {
                low.extend(all().map(|&x| {
                    let hi = x as f32;
                    if x.is_finite() && hi.is_finite() {
                        (x - f64::from(hi)) as f32
                    } else {
                        0.0
                    }
                }));
            }
            if e.ground {
                ground.push(placed);
            }
            placed += 1;
        }
        json.push(']');
        Batches {
            json,
            data,
            low,
            pictures: self.pictures,
            dropped: self.dropped,
            ground,
        }
    }
}

/// A path's segments through its `kept` vertices (indices into `pts`, in order) as a stroke batch takes them: each
/// from the tile's base `t`, its start's length along the path (`along`), its end caps when the path is `open`.
fn push_segments(out: &mut Vec<f64>, pts: &[Vec2], along: &[f64], kept: &[usize], open: bool, [tx, ty]: [f64; 2]) {
    let count = kept.len().saturating_sub(1);
    for (j, w) in kept.windows(2).enumerate() {
        let (a, b) = (pts[w[0]], pts[w[1]]);
        if js_hypot(b.x - a.x, b.y - a.y) < 1e-12 {
            continue;
        }
        let ends = if open && j == 0 { 1.0 } else { 0.0 } + if open && j + 1 == count { 2.0 } else { 0.0 };
        out.extend([a.x - tx, a.y - ty, b.x - tx, b.y - ty, along[w[0]], ends]);
    }
}

impl Sink for BatchSink {
    fn stroke(&mut self, style: &StrokeStyle, path: &[Vec2], closed: bool) {
        if self.hide_strokes {
            return;
        }
        if self.recording
            && (style.dash.is_some()
                || style.line_type_loss.is_some()
                || self.scale != Scale::default())
        {
            self.refused = true;
        }
        // On a map of another system the path is moved there, its edges split as they bend (docs/adr/0583 §3).
        if let Some(m) = self.map {
            let Some(moved) = m.path(path, closed) else {
                return;
            };
            self.map = None;
            self.stroke(style, &moved, closed);
            self.map = Some(m);
            return;
        }
        // An empty path still makes its batch, as before tiles (the batches' order).
        let tile = path.first().map_or([0.0, 0.0], |&p| self.tile_of(p));
        let e = match self
            .recent_strokes
            .find(style, self.scale, tile, self.group)
        {
            Some(e) => e,
            None => {
                let mut key = String::from("s|");
                style.write_json(&mut key);
                let e = self.entry(&key, Kind::Stroke, style.level, tile, style.ground && style.width > 0.0, || {
                    key[2..].to_string()
                });
                self.recent_strokes
                    .remember(style, self.scale, tile, self.group, e);
                e
            }
        };
        // The numbers from the tile's origin; the box from the anchor.
        let (ox, oy) = (self.origin.x, self.origin.y);
        let lod = self.lod;
        let e = &mut self.entries[e];
        let (tx, ty) = (e.base.x, e.base.y);
        e.w = js_max(e.w, style.width / 2.0 + style.blur);
        let n = path.len();
        let count = if closed { n } else { n.saturating_sub(1) };
        let mut d = 0.0;
        // A long path by itself: as it is at level 0, simplified at the far ones (docs/adr/1064 §2).
        if self.line_lod && lod.is_none() && n >= super::line_lod::LEAST_POINTS {
            let mut pts = path.to_vec();
            if closed {
                pts.push(path[0]);
            }
            let tolerances = CURVE_LEVELS.map(|ceiling| CURVE_TOLERANCE_PX / ceiling);
            if let Some(levels) = super::line_lod::levels(&pts, &tolerances) {
                let Entry { lods, bounds, .. } = e;
                if lods.len() <= CURVE_LEVELS.len() {
                    lods.resize_with(CURVE_LEVELS.len() + 1, Vec::new);
                }
                // Each vertex's length along the path as level 0 measures it: a level's dash starts where it did.
                let mut along = Vec::with_capacity(pts.len());
                for (i, a) in pts.iter().enumerate() {
                    along.push(d);
                    if let Some(b) = pts.get(i + 1) {
                        let len = js_hypot(b.x - a.x, b.y - a.y);
                        if len >= 1e-12 {
                            d += len;
                        }
                    }
                }
                let all: Vec<usize> = (0..pts.len()).collect();
                for (l, kept) in std::iter::once(&all).chain(levels.iter()).enumerate() {
                    push_segments(&mut lods[l], &pts, &along, kept, !closed, [tx, ty]);
                }
                for p in &pts {
                    grow_box(bounds, p.x - ox, p.y - oy);
                }
                return;
            }
        }
        // A curve's level goes to its own list (docs/adr/1063 §3); the box grows the same.
        let Entry {
            data, lods, bounds, ..
        } = e;
        let out = match lod {
            Some(l) => {
                if lods.len() <= l {
                    lods.resize_with(l + 1, Vec::new);
                }
                &mut lods[l]
            }
            None => data,
        };
        for i in 0..count {
            let a = path[i];
            let b = path[(i + 1) % n];
            let len = js_hypot(b.x - a.x, b.y - a.y);
            if len < 1e-12 {
                continue;
            }
            let ends = if !closed && i == 0 { 1.0 } else { 0.0 }
                + if !closed && i + 1 == count { 2.0 } else { 0.0 };
            out.extend([a.x - tx, a.y - ty, b.x - tx, b.y - ty, d, ends]);
            grow_box(bounds, a.x - ox, a.y - oy);
            d += len;
        }
        // The last point of an open path is no segment's start.
        if n > 0 {
            let p = path[if closed { 0 } else { n - 1 }];
            e.grow(p.x - ox, p.y - oy);
        }
    }

    fn fill(&mut self, paint: &FillPaint, rings: &[Vec<Vec2>]) {
        if self.lod.is_some_and(|l| l > 0) {
            return;
        }
        if self.recording
            && (!matches!(paint, FillPaint::Solid { .. }) || self.scale != Scale::default())
        {
            self.refused = true;
        }
        if let Some(m) = self.map {
            // A picture on a map of another system is a mesh moved node by node (docs/adr/0630 §2).
            if matches!(paint, FillPaint::Image { .. }) {
                self.warp_picture(m, paint, rings);
                return;
            }
            let Some(moved) = rings
                .iter()
                .map(|r| m.path(r, true))
                .collect::<Option<Vec<_>>>()
            else {
                return;
            };
            // A raster's frame only bounds its batch, its affine stays the project's: the raster pass meshes its
            // tiles onto the map (docs/adr/0630 §1). A cloud's plan is an area as any other, its picture the
            // points pass draws on the map (§3).
            self.map = None;
            self.fill(paint, &moved);
            self.map = Some(m);
            return;
        }
        if let Some(e) = rings.first().and_then(|outer| self.queue(paint, outer)) {
            self.fills.push((e, rings.to_vec(), false));
        }
    }

    fn reserve_cad_points(&mut self, points: usize) -> bool {
        if self.recording {
            self.refused = true;
        }
        let count = self.cad_points.saturating_add(points);
        if count > 2_000_000 { return false; }
        self.cad_points = count;
        true
    }

    fn cad_mark(&mut self, paint: &FillPaint, rings: &[Vec<Vec2>]) {
        if self.hide_strokes || self.lod.is_some_and(|l| l > 0) { return; }
        if self.recording {
            self.refused = true;
        }
        let hidden = self.hide_fills;
        self.hide_fills = false;
        self.fill(paint, rings);
        self.hide_fills = hidden;
    }

    fn cad_mark_mesh(&mut self, paint: &FillPaint, rings: &[Vec<Vec2>], indices: &[u32]) {
        if self.hide_strokes || self.lod.is_some_and(|l| l > 0) {
            return;
        }
        if self.recording {
            self.refused = true;
        }
        // A projection can densify/nonlinearly warp the boundary. Keep its
        // existing mapped-ring triangulation, not the source mesh connectivity.
        if self.map.is_some() || !matches!(paint, FillPaint::Solid { .. }) {
            self.cad_mark(paint, rings);
            return;
        }
        let points: Vec<_> = rings.iter().flatten().copied().collect();
        if !indices.len().is_multiple_of(3) || indices.iter().any(|&i| i as usize >= points.len()) {
            self.cad_mark(paint, rings);
            return;
        }
        let hidden = self.hide_fills;
        self.hide_fills = false;
        let entry = rings.first().and_then(|outer| self.queue(paint, outer));
        self.hide_fills = hidden;
        if let Some(entry) = entry {
            let e = &mut self.entries[entry];
            for &i in indices {
                let p = points[i as usize];
                e.data.extend([p.x - e.base.x, p.y - e.base.y]);
            }
        }
    }

    fn marker(&mut self, style: &MarkerStyle, at: Vec2, angle: f64) {
        if self.lod.is_some_and(|l| l > 0) {
            return;
        }
        if self.recording {
            self.refused = true;
        }
        let at = match self.map {
            None => at,
            Some(m) => match m.forward(at) {
                Some(p) => p,
                None => return,
            },
        };
        self.place_marker(style, at, angle);
    }

    fn effects(&mut self, spec: Option<&EffectSpec>, level: f64) {
        let Some(spec) = spec else {
            self.group = 0;
            return;
        };
        if self.recording {
            self.refused = true;
        }
        let json = spec.json();
        let next = self.groups.len() as u32 + 1;
        self.group = *self.groups.entry(group_key(&json, level)).or_insert(next);
        self.group_json = json;
    }
}

impl BatchSink {
    /// An area whose ring is star-shaped from its first point (a pie's slice, docs/adr/0213 §2.5): its
    /// triangles are the fan from that point, made without the ear clipper's scans.
    pub fn fan(&mut self, paint: &FillPaint, ring: &[Vec2]) {
        if self.lod.is_some_and(|l| l > 0) {
            return;
        }
        if self.recording {
            self.refused = true;
        }
        if let Some(m) = self.map {
            let Some(moved) = m.path(ring, true) else {
                return;
            };
            self.map = None;
            self.fan(paint, &moved);
            self.map = Some(m);
            return;
        }
        if let Some(e) = self.queue(paint, ring) {
            self.fills.push((e, vec![ring.to_vec()], true));
        }
    }

    /// A picture on a map of another system (docs/adr/0630 §2): its frame (clipped as it is) triangulated in the
    /// project's system, each triangle halved as the map's bend asks and moved node by node; its batch holds the
    /// small triangles' corners on the map with their picture coordinates, x, y, u, v (`"warped":true`), drawn as a
    /// raster's tiles are.
    fn warp_picture(
        &mut self,
        m: &kentos_geometry_core::crs::map::MapProjection,
        paint: &FillPaint,
        rings: &[Vec<Vec2>],
    ) {
        /// The small triangles a picture is drawn with at most.
        const MOST: usize = 200_000;
        let FillPaint::Image {
            corner,
            size,
            angle,
            mirror,
            ..
        } = paint
        else {
            return;
        };
        if rings.first().is_none_or(|r| r.len() < 3) || !(size[0] > 0.0 && size[1] > 0.0) {
            return;
        }
        let mut pts = Vec::new();
        let mut sizes = Vec::with_capacity(rings.len());
        for r in rings {
            sizes.push(r.len());
            pts.extend_from_slice(r);
        }
        let idx = triangulate_many(&pts, &sizes, &[rings.len()]);
        let tris: Vec<[Vec2; 3]> = idx
            .chunks_exact(3)
            .map(|t| [pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]])
            .collect();
        if tris.is_empty() {
            return;
        }
        // As many halvings as the bend asks, while the triangles stay under the bound.
        let mut most = 0;
        while most < 8 && tris.len() << (2 * (most + 1)) <= MOST {
            most += 1;
        }
        let level = m.mesh_level(&tris, m.tolerance, most);
        let mut nodes = Vec::with_capacity((tris.len() * 3) << (2 * level));
        for t in &tris {
            m.mesh(*t, level, &mut nodes);
        }
        let Some(first) = nodes.first() else {
            return;
        };
        // The picture's coordinates of a point of the project's system, as `imageFs` takes them.
        let (c, s) = (cos(*angle), sin(*angle));
        let uv = |p: Vec2| {
            let (dx, dy) = (p.x - corner[0], p.y - corner[1]);
            let u = (dx * c + dy * s) / size[0];
            let t = (dy * c - dx * s) / size[1];
            let t = if *mirror { 1.0 - t } else { t };
            [u, 1.0 - t]
        };
        let mut key = String::from("w|");
        paint.write_json(&mut key);
        let tile = self.tile_of(first[1]);
        let e = self.entry(&key, Kind::Fill, paint.level(), tile, false, || {
            let mut style = key[2..].to_string();
            style.pop();
            style.push_str(",\"warped\":true}");
            style
        });
        let (ox, oy) = (self.origin.x, self.origin.y);
        let e = &mut self.entries[e];
        let (bx, by) = (e.base.x, e.base.y);
        for [p, q] in nodes {
            let [u, v] = uv(p);
            e.data.extend([q.x - bx, q.y - by, u, v]);
            e.grow(q.x - ox, q.y - oy);
        }
    }

    /// The batch of a fill whose outer ring is `outer`, grown by it; none where nothing is drawn.
    fn queue(&mut self, paint: &FillPaint, outer: &[Vec2]) -> Option<usize> {
        if outer.len() < 3
            || (self.hide_fills
                && !matches!(
                    paint,
                    FillPaint::Image { .. }
                        | FillPaint::Raster { .. }
                        | FillPaint::PointCloud { .. }
                ))
        {
            return None;
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
            // A raster's affine likewise: its first pixel's corner from the anchor (docs/adr/0204 §5).
            FillPaint::Raster {
                raster,
                look,
                affine,
                size,
                nearest,
                opacity,
                level,
            } => {
                let [x0, a, b, y0, c, d] = *affine;
                anchored = FillPaint::Raster {
                    raster: raster.clone(),
                    look: look.clone(),
                    affine: [x0 - self.origin.x, a, b, y0 - self.origin.y, c, d],
                    size: *size,
                    nearest: *nearest,
                    opacity: *opacity,
                    level: *level,
                };
                &anchored
            }
            _ => paint,
        };
        let tile = self.tile_of(outer[0]);
        let e = match self.recent_fills.find(paint, self.scale, tile, self.group) {
            Some(e) => e,
            None => {
                let mut key = String::from("f|");
                paint.write_json(&mut key);
                let e = self.entry(&key, Kind::Fill, paint.level(), tile, paint.ground(), || {
                    key[2..].to_string()
                });
                self.recent_fills
                    .remember(paint, self.scale, tile, self.group, e);
                e
            }
        };
        let (ox, oy) = (self.origin.x, self.origin.y);
        for p in outer {
            self.entries[e].grow(p.x - ox, p.y - oy);
        }
        Some(e)
    }

    fn place_marker(&mut self, style: &MarkerStyle, at: Vec2, angle: f64) {
        let tile = self.tile_of(at);
        let e = match self
            .recent_markers
            .find(style, self.scale, tile, self.group)
        {
            Some(e) => e,
            None => {
                let key = style.key();
                let e = self.entry(&key, Kind::Marker, style.common.level, tile, false, || {
                    let mut s = String::new();
                    style.write_json(&mut s);
                    s
                });
                self.recent_markers
                    .remember(style, self.scale, tile, self.group, e);
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
            line_type_loss: None,
            cap: "butt".into(),
            join: "miter".into(),
            blur: 0.0,
            level: 0.0,
            ground: false,
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

    /// docs/adr/0793 §1: a width measured on the ground keeps a batch of its own beside a paper width whose style
    /// reads alike, and only it is listed; the style's JSON is the page's as before.
    #[test]
    fn a_ground_width_keeps_a_batch_of_its_own_and_is_listed() {
        let mut sink = BatchSink::new(ANCHOR);
        let path = [
            Vec2::new(487_110.0, 4_420_210.0),
            Vec2::new(487_130.5, 4_420_210.0),
        ];
        let ground = StrokeStyle {
            ground: true,
            ..line()
        };
        let hair = StrokeStyle {
            ground: true,
            width: 0.0,
            ..line()
        };
        sink.stroke(&line(), &path, false);
        sink.stroke(&ground, &path, false);
        sink.stroke(&line(), &path, false);
        sink.stroke(&hair, &path, false);
        let out = sink.finish();
        let batches: serde_json::Value = serde_json::from_str(&out.json).expect("reads");
        let batches = batches.as_array().expect("an array");
        assert_eq!(batches.len(), 3, "paper, ground, a hairline");
        assert_eq!(batches[0]["style"], batches[1]["style"], "the page reads them alike");
        assert!(batches[1]["style"].get("ground").is_none());
        // A zero width is a hairline either way: never listed.
        assert_eq!(out.ground, [1]);
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

    #[test]
    fn cad_mark_mesh_uses_mapped_contours_for_nonlinear_projection() {
        use kentos_geometry_core::crs::map::{MapProjection, interned};
        use kentos_geometry_core::crs::{Datum, System};
        let source = System::Tm { datum: Datum::Turef, central_meridian: 36., scale_factor: 1., false_easting: 500_000., false_northing: 0., latitude_of_origin: None };
        let projection = interned(MapProjection::new(source, System::Mercator {}, Vec::new()));
        let anchor = projection.forward(ANCHOR).unwrap();
        let rings = vec![vec![ANCHOR, Vec2::new(ANCHOR.x + 6000., ANCHOR.y), Vec2::new(ANCHOR.x + 6000., ANCHOR.y + 4000.), Vec2::new(ANCHOR.x, ANCHOR.y + 4000.)]];
        let paint = FillPaint::Solid { color: "#884422".into(), opacity: 0.5, level: 0. };
        let mut expected = BatchSink::new(anchor); expected.set_map(Some(projection)); expected.cad_mark(&paint, &rings);
        let mut actual = BatchSink::new(anchor); actual.set_map(Some(projection)); actual.cad_mark_mesh(&paint, &rings, &[0, 1, 2, 0, 2, 3]);
        let (expected, actual) = (expected.finish(), actual.finish());
        assert!(!actual.data.is_empty());
        assert_eq!(actual.json, expected.json);
        assert_eq!(actual.data, expected.data);
        assert_eq!(actual.low, expected.low);
    }

    /// A picture on a map of another system (docs/adr/0630 §2): its batch holds its small triangles on the map with
    /// the picture's coordinates, x, y, u, v; the frame's corners are the picture's own corners on the map.
    #[test]
    fn a_picture_on_another_map_is_a_mesh_with_its_coordinates() {
        use kentos_geometry_core::crs::map::{MapProjection, interned};
        use kentos_geometry_core::crs::{Datum, System};
        let tm = |cm: f64| System::Tm {
            datum: Datum::Turef,
            central_meridian: cm,
            scale_factor: 1.0,
            false_easting: 500_000.0,
            false_northing: 0.0,
            latitude_of_origin: None,
        };
        let m = interned(MapProjection::new(
            tm(36.0),
            System::Mercator {},
            Vec::new(),
        ));
        let anchor = m.forward(ANCHOR).expect("on the map");
        let mut sink = BatchSink::new(anchor);
        sink.set_map(Some(m));
        let (x0, y0, w, h) = (ANCHOR.x, ANCHOR.y, 6_000.0, 4_000.0);
        let paint = FillPaint::Image {
            image: "asset:pafta".into(),
            corner: [x0, y0],
            size: [w, h],
            angle: 0.0,
            mirror: false,
            opacity: 1.0,
            level: 0.0,
        };
        let ring = vec![
            Vec2::new(x0, y0),
            Vec2::new(x0 + w, y0),
            Vec2::new(x0 + w, y0 + h),
            Vec2::new(x0, y0 + h),
        ];
        sink.fill(&paint, &[ring]);
        let out = sink.finish();
        let batches: serde_json::Value = serde_json::from_str(&out.json).expect("JSON");
        assert_eq!(batches[0]["style"]["warped"], serde_json::json!(true));
        let data = &out.data;
        assert_eq!(data.len() % 12, 0);
        // A 6 km sheet in Web Mercator is halved (map_warp_cases.py: level 3, 128 triangles).
        assert_eq!(data.len() / 12, 128);
        // Each vertex: the picture's coordinates of its project point land on the map where PROJ puts it.
        for v in data.chunks_exact(4) {
            let (u, t) = (f64::from(v[2]), 1.0 - f64::from(v[3]));
            assert!((-1e-6..=1.0 + 1e-6).contains(&u) && (-1e-6..=1.0 + 1e-6).contains(&t));
            let q = m
                .forward(Vec2::new(x0 + u * w, y0 + t * h))
                .expect("on the map");
            let got = (f64::from(v[0]) + anchor.x, f64::from(v[1]) + anchor.y);
            assert!(
                (got.0 - q.x).abs() < 0.01 && (got.1 - q.y).abs() < 0.01,
                "{got:?} {q:?}"
            );
        }
    }
}
