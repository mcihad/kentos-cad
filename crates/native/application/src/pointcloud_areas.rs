//! A point cloud's statistics in an area and its profile along a line, at the source's full resolution
//! (docs/adr/0552): `cad.pointclouds.region` and `cad.pointclouds.profile`. Both read the files themselves with the
//! point cloud core, like `cad.pointclouds.query` (docs/adr/0536): a COPC reads only the nodes whose cube meets the
//! area (or a segment of the corridor), a LAS or LAZ the whole file whose box does; the answer says the points read.

use std::collections::BTreeMap;

use kentos_commands::{
    CloudRange, CloudRegionPart, CloudStats, CommandResult, CommandWarning, CloudFormat,
    PROFILE_MOST_LISTED, PROFILE_MOST_STATIONS, POINTCLOUD_MOST_COUNTED, PointCloudEntity,
    PointCloudsProfile, PointCloudsProfiled, PointCloudsRegion, PointCloudsRegioned, ProfileBin,
    ProfilePoint, Vec2,
};
use kentos_domain::Document;
use kentos_geometry_core::predicates::orient2d;
use kentos_geometry_core::vec2::Vec2 as P;
use kentos_pointcloud::copc::{Info, Key};
use kentos_pointcloud::record::Layout;
use kentos_pointcloud::source::Cloud;

use crate::checks::{self, Stop};
use crate::codes;
use crate::pointclouds::{answer, bytes_of, clouds, open, records, uids_checked};
use crate::source_bytes::Bytes;

/// A sum kept as two floats: `hi + lo` is the exact sum to about 106 bits, so the mean loses nothing before its one
/// rounding (Knuth's two-sum).
#[derive(Clone, Copy, Default)]
struct Sum {
    hi: f64,
    lo: f64,
}

impl Sum {
    fn add(&mut self, x: f64) {
        let s = self.hi + x;
        let bb = s - self.hi;
        let e = (self.hi - (s - bb)) + (x - bb);
        self.hi = s;
        self.lo += e;
    }

    fn value(self) -> f64 {
        self.hi + self.lo
    }
}

/// A value's smallest, largest and sum.
#[derive(Clone, Copy)]
struct Moments {
    min: f64,
    max: f64,
    sum: Sum,
    n: u64,
}

impl Moments {
    fn new() -> Moments {
        Moments {
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            sum: Sum::default(),
            n: 0,
        }
    }

    fn add(&mut self, v: f64) {
        self.min = self.min.min(v);
        self.max = self.max.max(v);
        self.sum.add(v);
        self.n += 1;
    }

    fn range(&self) -> Option<CloudRange> {
        (self.n > 0).then(|| CloudRange {
            min: self.min,
            max: self.max,
            mean: self.sum.value() / self.n as f64,
        })
    }
}

/// What the points seen are: count, box, heights, intensities, classes.
struct Stats {
    count: u64,
    lo: [f64; 3],
    hi: [f64; 3],
    z: Moments,
    intensity: Moments,
    classes: BTreeMap<String, u64>,
}

impl Stats {
    fn new() -> Stats {
        Stats {
            count: 0,
            lo: [f64::INFINITY; 3],
            hi: [f64::NEG_INFINITY; 3],
            z: Moments::new(),
            intensity: Moments::new(),
            classes: BTreeMap::new(),
        }
    }

    fn add(&mut self, xyz: [f64; 3], intensity: u16, class: u8) {
        self.count += 1;
        for (k, v) in xyz.iter().enumerate() {
            self.lo[k] = self.lo[k].min(*v);
            self.hi[k] = self.hi[k].max(*v);
        }
        self.z.add(xyz[2]);
        self.intensity.add(f64::from(intensity));
        *self.classes.entry(class.to_string()).or_default() += 1;
    }

    fn merge(&mut self, o: &Stats) {
        self.count += o.count;
        for k in 0..3 {
            self.lo[k] = self.lo[k].min(o.lo[k]);
            self.hi[k] = self.hi[k].max(o.hi[k]);
        }
        for (m, n) in [(&mut self.z, &o.z), (&mut self.intensity, &o.intensity)] {
            m.min = m.min.min(n.min);
            m.max = m.max.max(n.max);
            m.sum.add(n.sum.hi);
            m.sum.add(n.sum.lo);
            m.n += n.n;
        }
        for (c, n) in &o.classes {
            *self.classes.entry(c.clone()).or_default() += n;
        }
    }

    fn out(&self) -> CloudStats {
        CloudStats {
            count: self.count,
            bounds: (self.count > 0).then(|| {
                [
                    self.lo[0], self.lo[1], self.lo[2], self.hi[0], self.hi[1], self.hi[2],
                ]
            }),
            z: self.z.range(),
            intensity: self.intensity.range(),
            classes: self.classes.clone(),
        }
    }
}

/// One point as the scan hands it over.
struct Seen<'r> {
    cloud: usize,
    source: u32,
    index: u64,
    xyz: [f64; 3],
    layout: &'r Layout,
    record: &'r [u8],
}

/// Whether a COPC node's cube meets any of `boxes` (each `[x₁, y₁, x₂, y₂]`).
fn node_meets(key: Key, info: &Info, boxes: &[[f64; 4]]) -> bool {
    let (low, side) = key.cube(info);
    boxes.iter().any(|b| {
        !(b[2] < low[0] || b[0] > low[0] + side || b[3] < low[1] || b[1] > low[1] + side)
    })
}

fn box_meets(a: [f64; 4], b: [f64; 4]) -> bool {
    !(a[2] < b[0] || a[0] > b[2] || a[3] < b[1] || a[1] > b[3])
}

fn too_many(what: &str, path: &str) -> Stop {
    Stop::failed(checks::error(
        codes::TOO_MANY_CLOUD_POINTS,
        format!(
            "{what} bir çağrının okuduğundan çok nokta okutur; bir çağrı en çok {POINTCLOUD_MOST_COUNTED} nokta okur. Küçültün ya da bulutları ayrı ayrı sorun."
        ),
        Some(path.to_owned()),
    ))
}

/// What a scan looks in: the clouds, the boxes a source (a COPC's node) must meet to be read, and what the call is for
/// (the words of a refusal and its path).
struct Scan<'a> {
    doc: &'a Document,
    drawing: Option<&'a str>,
    chosen: &'a [(String, &'a PointCloudEntity)],
    boxes: &'a [[f64; 4]],
    what: &'a str,
    path: &'a str,
}

impl Scan<'_> {
    /// Every point of the sources whose box (a COPC's nodes whose cube) meets one of the boxes, in the clouds' order,
    /// each source's points in its order; `see` takes each. The points read per cloud are added to `read`.
    fn run(
        &self,
        warnings: &mut Vec<CommandWarning>,
        read: &mut [u64],
        see: &mut dyn FnMut(&Seen<'_>),
    ) -> Result<(), Stop> {
        scan(self, warnings, read, see)
    }
}

fn scan(
    this: &Scan<'_>,
    warnings: &mut Vec<CommandWarning>,
    read: &mut [u64],
    see: &mut dyn FnMut(&Seen<'_>),
) -> Result<(), Stop> {
    let Scan {
        doc,
        drawing,
        chosen,
        boxes,
        what,
        path,
    } = *this;
    // A whole LAS or LAZ file the box meets is read: the cloud's record of it is the points that call reads.
    let planned: u64 = chosen
        .iter()
        .flat_map(|(_, c)| c.cloud.sources.iter())
        .filter(|s| {
            s.format != CloudFormat::Copc
                && s.format != CloudFormat::Xyz
                && boxes
                    .iter()
                    .any(|b| box_meets(*b, [s.bounds[0], s.bounds[1], s.bounds[3], s.bounds[4]]))
        })
        .map(|s| s.count)
        .sum();
    if planned > POINTCLOUD_MOST_COUNTED {
        return Err(too_many(what, path));
    }
    let mut total = 0u64;
    let mut recs = Vec::new();
    for (k, (_, c)) in chosen.iter().enumerate() {
        for (si, s) in c.cloud.sources.iter().enumerate() {
            let b = [s.bounds[0], s.bounds[1], s.bounds[3], s.bounds[4]];
            if !boxes.iter().any(|x| box_meets(*x, b)) {
                continue;
            }
            let at = |why: String| CommandWarning {
                code: codes::CLOUD_SOURCE_UNREADABLE.into(),
                message: why,
                path: Some(format!("clouds[{k}].sources[{si}]")),
            };
            if s.format == CloudFormat::Xyz {
                warnings.push(at(format!(
                    "Bulutun {}. kaynağı metin bulutu; noktası burada aranmaz.",
                    si + 1
                )));
                continue;
            }
            let opened = (|| -> Result<(Cloud, Bytes), String> {
                let bytes = bytes_of(doc, drawing, s)?;
                Ok((open(&bytes)?, bytes))
            })();
            let (cloud, bytes) = match opened {
                Ok(x) => x,
                Err(why) => {
                    warnings.push(at(format!(
                        "Bulutun {}. kaynağı okunamadı: {why}.",
                        si + 1
                    )));
                    continue;
                }
            };
            let (layout, scale, offset) = (cloud.layout, cloud.head.scale, cloud.head.offset);
            for run in cloud.runs() {
                if let (Some(key), Some(info)) = (run.node, cloud.info.as_ref())
                    && !node_meets(key, info, boxes)
                {
                    continue;
                }
                if let Err(why) = records(&cloud, &bytes, &run, &mut recs) {
                    warnings.push(at(format!(
                        "Bulutun {}. kaynağının noktaları okunamadı: {why}.",
                        si + 1
                    )));
                    break;
                }
                let n = (recs.len() / layout.len) as u64;
                total += n;
                if total > POINTCLOUD_MOST_COUNTED {
                    return Err(too_many(what, path));
                }
                read[k] += n;
                for (i, r) in recs.chunks_exact(layout.len).enumerate() {
                    let xyz = [
                        f64::from(layout.x(r)) * scale[0] + offset[0],
                        f64::from(layout.y(r)) * scale[1] + offset[1],
                        f64::from(layout.z(r)) * scale[2] + offset[2],
                    ];
                    see(&Seen {
                        cloud: k,
                        source: si as u32,
                        index: run.first + i as u64,
                        xyz,
                        layout: &layout,
                        record: r,
                    });
                }
            }
        }
    }
    Ok(())
}

/// The clouds an area or a corridor looks in: the named, or the visible ones of the drawing.
fn chosen<'a>(
    doc: &'a Document,
    uids: &'a [String],
) -> Result<Vec<(String, &'a PointCloudEntity)>, Stop> {
    uids_checked(uids)?;
    let mut chosen = clouds(doc, uids)?;
    if uids.is_empty() {
        chosen.retain(|(_, c)| doc.layers().is_visible(&c.base.layer_id));
    }
    Ok(chosen)
}

// ── Area ───────────────────────────────────────────────────────────────

/// A ring's vertices, a repeated last vertex dropped; why not when it has fewer than three or one that is not a
/// number.
fn ring_of(r: &[Vec2], what: &str, path: &str) -> Result<Vec<P>, Stop> {
    let bad = |why: String| {
        Stop::failed(checks::error(
            codes::INVALID_CLOUD_AREA,
            why,
            Some(path.to_owned()),
        ))
    };
    let mut pts: Vec<P> = Vec::with_capacity(r.len());
    for (i, p) in r.iter().enumerate() {
        if !(p.x.is_finite() && p.y.is_finite()) {
            return Err(bad(format!(
                "{what}nin {}. köşesi sonlu bir sayı değil; köşeler planda koordinatlardır.",
                i + 1
            )));
        }
        pts.push(P { x: p.x, y: p.y });
    }
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    if pts.len() < 3 {
        return Err(bad(format!(
            "{what} en az üç köşeli olmalı; alanın köşelerini sırayla verin."
        )));
    }
    Ok(pts)
}

/// Twice a ring's signed area (the shoelace sum, taken from the first vertex so that coordinates in the millions lose
/// nothing).
fn twice_area(r: &[P]) -> f64 {
    let mut s = Sum::default();
    let o = r[0];
    for i in 0..r.len() {
        let (a, b) = (r[i], r[(i + 1) % r.len()]);
        s.add((a.x - o.x) * (b.y - o.y) - (b.x - o.x) * (a.y - o.y));
    }
    s.value()
}

#[derive(PartialEq)]
enum Place {
    Outside,
    Boundary,
    Inside,
}

/// Where a point lies against a ring: on an edge (exactly), inside (an odd number of edges cross the ray to its
/// east), or outside.
fn place_in(ring: &[P], p: P) -> Place {
    let mut inside = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        let o = orient2d(a, b, p);
        if o == 0.0
            && p.x >= a.x.min(b.x)
            && p.x <= a.x.max(b.x)
            && p.y >= a.y.min(b.y)
            && p.y <= a.y.max(b.y)
        {
            return Place::Boundary;
        }
        let up = a.y <= p.y && p.y < b.y;
        let down = b.y <= p.y && p.y < a.y;
        if (up && o > 0.0) || (down && o < 0.0) {
            inside = !inside;
        }
    }
    if inside { Place::Inside } else { Place::Outside }
}

fn ring_box(r: &[P]) -> [f64; 4] {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for p in r {
        b[0] = b[0].min(p.x);
        b[1] = b[1].min(p.y);
        b[2] = b[2].max(p.x);
        b[3] = b[3].max(p.y);
    }
    b
}

/// What `cad.pointclouds.region` says; why not.
pub(crate) fn region_in(
    doc: &Document,
    input: &PointCloudsRegion,
) -> Result<(PointCloudsRegioned, Vec<CommandWarning>), Stop> {
    let outer = ring_of(&input.ring, "Halka", "ring")?;
    let mut holes = Vec::new();
    for (h, r) in input.holes.iter().enumerate() {
        holes.push(ring_of(r, "Delik", &format!("holes[{h}]"))?);
    }
    let mut twice = twice_area(&outer).abs();
    for h in &holes {
        twice -= twice_area(h).abs();
    }
    let area = twice / 2.0;
    if area.is_nan() || area <= 0.0 {
        return Err(Stop::failed(checks::error(
            codes::INVALID_CLOUD_AREA,
            "Alanın büyüklüğü sıfırdan büyük olmalı (halka eksi delikler); köşeler bir doğru üzerinde ya da delikler halkayı kaplıyor.".into(),
            Some("ring".into()),
        )));
    }
    let chosen = chosen(doc, &input.uids)?;
    let bbox = ring_box(&outer);
    let hole_boxes: Vec<[f64; 4]> = holes.iter().map(|h| ring_box(h)).collect();
    let mut warnings = Vec::new();
    let mut read = vec![0u64; chosen.len()];
    let mut stats: Vec<Stats> = (0..chosen.len()).map(|_| Stats::new()).collect();
    Scan {
        doc,
        drawing: input.drawing.as_deref(),
        chosen: &chosen,
        boxes: &[bbox],
        what: "Bu alan",
        path: "ring",
    }
    .run(
        &mut warnings,
        &mut read,
        &mut |s| {
            let p = P {
                x: s.xyz[0],
                y: s.xyz[1],
            };
            if p.x < bbox[0] || p.x > bbox[2] || p.y < bbox[1] || p.y > bbox[3] {
                return;
            }
            if place_in(&outer, p) == Place::Outside {
                return;
            }
            for (h, hb) in holes.iter().zip(&hole_boxes) {
                if p.x >= hb[0]
                    && p.x <= hb[2]
                    && p.y >= hb[1]
                    && p.y <= hb[3]
                    && place_in(h, p) == Place::Inside
                {
                    return;
                }
            }
            stats[s.cloud].add(
                s.xyz,
                s.layout.intensity(s.record),
                s.layout.class(s.record),
            );
        },
    )?;
    let mut total = Stats::new();
    let mut parts = Vec::new();
    for (k, (uid, _)) in chosen.iter().enumerate() {
        total.merge(&stats[k]);
        parts.push(CloudRegionPart {
            uid: uid.clone(),
            read: read[k],
            stats: stats[k].out(),
        });
    }
    let total_out = total.out();
    Ok((
        PointCloudsRegioned {
            area,
            density: (total.count > 0).then(|| total.count as f64 / area),
            read: read.iter().sum(),
            total: total_out,
            clouds: parts,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

// ── Corridor ───────────────────────────────────────────────────────────

/// A path with its width: where a point lies along it.
struct Corridor {
    pts: Vec<[f64; 2]>,
    lens: Vec<f64>,
    starts: Vec<f64>,
    half2: f64,
    boxes: Vec<[f64; 4]>,
    length: f64,
}

impl Corridor {
    /// The path (at least two vertices, none repeated in a row) of width `width`.
    fn new(path: &[Vec2], width: f64) -> Result<Corridor, Stop> {
        let bad = |why: String, at: &str| {
            Stop::failed(checks::error(
                codes::INVALID_CORRIDOR,
                why,
                Some(at.to_owned()),
            ))
        };
        if path.len() < 2 {
            return Err(bad(
                "Profilin yolu en az iki köşeli olmalı; yolun köşelerini sırayla verin.".into(),
                "path",
            ));
        }
        let mut pts = Vec::with_capacity(path.len());
        for (i, p) in path.iter().enumerate() {
            if !(p.x.is_finite() && p.y.is_finite()) {
                return Err(bad(
                    format!(
                        "Yolun {}. köşesi sonlu bir sayı değil; köşeler planda koordinatlardır.",
                        i + 1
                    ),
                    &format!("path[{i}]"),
                ));
            }
            if pts.last() == Some(&[p.x, p.y]) {
                return Err(bad(
                    format!(
                        "Yolun {}. köşesi öncekiyle aynı; uzunluğu olmayan parça olmaz.",
                        i + 1
                    ),
                    &format!("path[{i}]"),
                ));
            }
            pts.push([p.x, p.y]);
        }
        if !(width.is_finite() && width > 0.0) {
            return Err(bad(
                "Koridorun genişliği sıfırdan büyük olmalı; yolun iki yanında toplam genişliği verin.".into(),
                "width",
            ));
        }
        let half = width / 2.0;
        let (mut lens, mut starts, mut boxes) = (Vec::new(), Vec::new(), Vec::new());
        let mut at = 0.0;
        for w in pts.windows(2) {
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            let len = (dx * dx + dy * dy).sqrt();
            starts.push(at);
            lens.push(len);
            at += len;
            boxes.push([
                w[0][0].min(w[1][0]) - half,
                w[0][1].min(w[1][1]) - half,
                w[0][0].max(w[1][0]) + half,
                w[0][1].max(w[1][1]) + half,
            ]);
        }
        Ok(Corridor {
            pts,
            lens,
            starts,
            half2: half * half,
            boxes,
            length: at,
        })
    }

    /// The point's station and offset, when it lies in the corridor: its distance to the path is that to the nearest
    /// segment (the earlier on a tie), the station that segment's start plus the foot's place on it, the offset
    /// positive on the path's left.
    fn place(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64, f64)> = None;
        for (i, w) in self.pts.windows(2).enumerate() {
            let (ax, ay) = (w[0][0], w[0][1]);
            let (abx, aby) = (w[1][0] - ax, w[1][1] - ay);
            let (apx, apy) = (x - ax, y - ay);
            let len2 = abx * abx + aby * aby;
            let dot = apx * abx + apy * aby;
            let cross = abx * apy - aby * apx;
            let len = self.lens[i];
            let (d2, along) = if dot <= 0.0 {
                (apx * apx + apy * apy, 0.0)
            } else if dot >= len2 {
                let (bpx, bpy) = (x - w[1][0], y - w[1][1]);
                (bpx * bpx + bpy * bpy, len)
            } else {
                (cross * cross / len2, dot / len)
            };
            if best.is_none_or(|(b, _, _)| d2 < b) {
                best = Some((d2, self.starts[i] + along, cross / len));
            }
        }
        let (d2, station, offset) = best?;
        (d2 <= self.half2).then_some((station, offset))
    }
}

/// What `cad.pointclouds.profile` says; why not.
pub(crate) fn profile_in(
    doc: &Document,
    input: &PointCloudsProfile,
) -> Result<(PointCloudsProfiled, Vec<CommandWarning>), Stop> {
    let corridor = Corridor::new(&input.path, input.width)?;
    if !(input.step.is_finite() && input.step > 0.0) {
        return Err(Stop::failed(checks::error(
            codes::INVALID_CORRIDOR,
            "İstasyon adımı sıfırdan büyük olmalı; yol boyunca adımı verin.".into(),
            Some("step".into()),
        )));
    }
    let stations = (corridor.length / input.step).ceil().max(1.0);
    if stations > PROFILE_MOST_STATIONS as f64 {
        return Err(Stop::failed(checks::error(
            codes::TOO_MANY_STATIONS,
            format!(
                "Profil {} istasyon olurdu; bir çağrı en çok {PROFILE_MOST_STATIONS} istasyon verir. Adımı büyütün ya da yolu kısaltın.",
                stations as u64
            ),
            Some("step".into()),
        )));
    }
    let n = stations as usize;
    let chosen = chosen(doc, &input.uids)?;
    let mut warnings = Vec::new();
    let mut read = vec![0u64; chosen.len()];
    let mut bins: Vec<Moments> = vec![Moments::new(); n];
    let mut listed: Vec<(f64, usize, u32, u64, ProfilePoint)> = Vec::new();
    let mut count = 0u64;
    let mut too_long = false;
    Scan {
        doc,
        drawing: input.drawing.as_deref(),
        chosen: &chosen,
        boxes: &corridor.boxes,
        what: "Bu profil",
        path: "path",
    }
    .run(
        &mut warnings,
        &mut read,
        &mut |s| {
            let Some((station, offset)) = corridor.place(s.xyz[0], s.xyz[1]) else {
                return;
            };
            count += 1;
            let k = ((station / input.step).floor().max(0.0) as usize).min(n - 1);
            bins[k].add(s.xyz[2]);
            if input.points {
                if listed.len() as u64 >= PROFILE_MOST_LISTED {
                    too_long = true;
                    return;
                }
                listed.push((
                    station,
                    s.cloud,
                    s.source,
                    s.index,
                    ProfilePoint {
                        cloud: chosen[s.cloud].0.clone(),
                        source: s.source,
                        index: s.index,
                        station,
                        offset,
                        x: s.xyz[0],
                        y: s.xyz[1],
                        z: s.xyz[2],
                        class: s.layout.class(s.record),
                    },
                ));
            }
        },
    )?;
    if too_long {
        return Err(Stop::failed(checks::error(
            codes::TOO_MANY_LISTED_POINTS,
            format!(
                "Koridorda {PROFILE_MOST_LISTED} noktadan çoğu var; bir çağrı en çok bu kadar nokta listeler. Koridoru daraltın ya da points'i kapatın (istasyonlar yine verilir)."
            ),
            Some("points".into()),
        )));
    }
    listed.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
            .then(a.3.cmp(&b.3))
    });
    let bins_out = bins
        .iter()
        .enumerate()
        .map(|(k, m)| ProfileBin {
            from: k as f64 * input.step,
            to: ((k + 1) as f64 * input.step).min(corridor.length),
            count: m.n,
            z: m.range(),
        })
        .collect();
    Ok((
        PointCloudsProfiled {
            length: corridor.length,
            count,
            read: read.iter().sum(),
            bins: bins_out,
            points: listed.into_iter().map(|t| t.4).collect(),
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

pub mod region {
    use super::*;
    use crate::ExecutionContext;

    pub fn validate(cx: &ExecutionContext<'_>, input: &PointCloudsRegion) -> CommandResult<()> {
        let checked = ring_of(&input.ring, "Halka", "ring")
            .and_then(|_| {
                input
                    .holes
                    .iter()
                    .enumerate()
                    .try_for_each(|(h, r)| ring_of(r, "Delik", &format!("holes[{h}]")).map(|_| ()))
            })
            .and_then(|()| chosen(cx.doc, &input.uids).map(|_| ()));
        match checked {
            Ok(()) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &PointCloudsRegion,
    ) -> CommandResult<PointCloudsRegioned> {
        answer(region_in(cx.doc, input))
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: PointCloudsRegion,
    ) -> CommandResult<PointCloudsRegioned> {
        plan(cx, &input)
    }
}

pub mod profile {
    use super::*;
    use crate::ExecutionContext;

    pub fn validate(cx: &ExecutionContext<'_>, input: &PointCloudsProfile) -> CommandResult<()> {
        let checked = Corridor::new(&input.path, input.width)
            .map(|_| ())
            .and_then(|()| chosen(cx.doc, &input.uids).map(|_| ()));
        match checked {
            Ok(()) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &PointCloudsProfile,
    ) -> CommandResult<PointCloudsProfiled> {
        answer(profile_in(cx.doc, input))
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: PointCloudsProfile,
    ) -> CommandResult<PointCloudsProfiled> {
        plan(cx, &input)
    }
}
