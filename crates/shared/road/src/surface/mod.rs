//! The terrain surface (docs/adr/0900): spot points and breaklines triangulated as a constrained Delaunay TIN
//! (`kentos.cdt/1`), clipped by a draped boundary and draped voids, its outer slivers trimmed by the longest edge,
//! and the height of a place read off its triangle. Every decision is the geometry core's exact predicates; no
//! number is constructed (a draped ring's corner takes the height the surface has there).

mod analysis;
mod contours;
mod coverage;
mod figures;
mod mesh;
mod quality;
mod tiled;
mod query;

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::delaunay::{self, NONE};
use kentos_geometry_core::predicates::orient2d;
use serde::{Deserialize, Serialize};

pub use analysis::{AnalysisError, Band, BandParts, Bands, GridSlope, MAX_CELLS as MAX_GRID_CELLS, Measure, Roughness};
pub use coverage::{Coverage, Region, clipped_area, ring_area};
pub use contours::{ContourError, ContourLine, MAX_LEVELS, Slope};
pub use figures::{Figures, SectionPoint};
pub use quality::{Gaps, NearPair, Spike};
pub use tiled::{Tiled, TiledError, build_tiled};
pub use query::{Height, Place};

use mesh::{Mesh, Walk};

/// The algorithm's id, kept with a surface so a later one is told apart (ROAD-TIN-02).
pub const ALGORITHM: &str = "kentos.cdt/1";

/// The heights two records at one place may differ by and still be one point, in metres (ADR 0900 §3).
pub const DEFAULT_TOLERANCE: f64 = 0.001;

/// A surveyed point: where it is and its height, in metres in the project's plane.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// What it came from (an object's id, a file's row), said back where it is used or refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// A breakline (ADR 0900 §4): its corners with their heights; each of its pieces is an edge of the surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakline {
    pub points: Vec<[f64; 3]>,
    /// Closed: the last corner joins the first.
    #[serde(default)]
    pub closed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// A boundary or a void (ADR 0900 §5): a closed ring without heights, draped on the surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ring {
    pub points: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// What is done with the long, thin triangles of a surface's outer edge (ADR 0900 §6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EdgeMethod {
    /// Every triangle of the points' hull is kept.
    #[default]
    None,
    /// An outer triangle whose outer edge is longer than `length` metres goes, until none is.
    MaxEdge { length: f64 },
}

/// What a surface is made of.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceInput {
    #[serde(default)]
    pub points: Vec<SpotPoint>,
    #[serde(default)]
    pub breaklines: Vec<Breakline>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary: Option<Ring>,
    #[serde(default)]
    pub voids: Vec<Ring>,
    /// [`DEFAULT_TOLERANCE`] when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
    #[serde(default)]
    pub edge: EdgeMethod,
}

/// Why no surface is made: a stable code, the reason in Turkish and, where there is one, the place.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceError {
    /// `invalid_input`, `too_few_points`, `height_ambiguous`, `height_conflict`, `constraints_cross`,
    /// `boundary_outside`, `internal`.
    pub code: &'static str,
    pub message: String,
    pub at: Option<[f64; 2]>,
}

impl SurfaceError {
    fn new(code: &'static str, message: impl Into<String>, at: Option<[f64; 2]>) -> Self {
        Self {
            code,
            message: message.into(),
            at,
        }
    }
}

impl std::fmt::Display for SurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// What a triangle of the whole triangulation is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Kept,
    /// Outside the boundary.
    Outside,
    /// In a void.
    Void,
    /// Taken off the outer edge by the edge method.
    Trimmed,
}

/// What was done to make the surface, said back to the user.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// Records at the place of an earlier one, with a height within the tolerance: one point each.
    pub merged: usize,
    /// Breakline pieces split at a point that lies on them.
    pub split: usize,
    /// Triangles the edge method took off.
    pub trimmed: usize,
}

/// A terrain surface: its corners (x, y, z), the triangles kept (counter-clockwise), its constrained edges and
/// what was done; the whole triangulation stays for the heights.
#[derive(Clone, Debug)]
pub struct Surface {
    pub vertices: Vec<[f64; 3]>,
    /// Each corner's source, the first record's at its place.
    pub sources: Vec<Option<String>>,
    pub triangles: Vec<[u32; 3]>,
    /// The breaklines' and rings' pieces as corner pairs, the lower index first, sorted.
    pub constrained: Vec<[u32; 2]>,
    pub report: Report,
    mesh: Mesh,
    /// The corners' places and heights as the walks read them.
    xy: Vec<Vec2>,
    z: Vec<f64>,
    kinds: Vec<Kind>,
    /// The kept triangle's index of each triangle of the whole triangulation.
    kept_index: Vec<u32>,
}

/// One constraint to put in: two corners and, for a breakline, which one (its heights are checked).
#[derive(Clone, Copy, Debug)]
struct Piece {
    a: u32,
    b: u32,
    breakline: bool,
}

/// The corners gathered so far, one per place.
#[derive(Default)]
struct Corners {
    xy: Vec<Vec2>,
    z: Vec<f64>,
    sources: Vec<Option<String>>,
    at: HashMap<(u64, u64), u32>,
    merged: usize,
}

fn key(x: f64, y: f64) -> (u64, u64) {
    // -0.0 and 0.0 are one place.
    ((x + 0.0).to_bits(), (y + 0.0).to_bits())
}

fn named(source: &Option<String>) -> String {
    source
        .as_deref()
        .map_or_else(|| "adsız kayıt".to_owned(), |s| format!("“{s}”"))
}

impl Corners {
    /// The corner at (x, y), added when new; one already there with another height is refused.
    fn add(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        source: &Option<String>,
        tolerance: f64,
    ) -> Result<u32, SurfaceError> {
        if let Some(&i) = self.at.get(&key(x, y)) {
            let had = self.z[i as usize];
            if (had - z).abs() > tolerance {
                return Err(SurfaceError::new(
                    "height_ambiguous",
                    format!(
                        "({x}, {y}) noktasında iki ayrı kot var: {} {had} m, {} {z} m. Tek değerli yüzeyde bir yerin \
                         bir kotu olur; birini düzeltin ya da çıkarın.",
                        named(&self.sources[i as usize]),
                        named(source)
                    ),
                    Some([x, y]),
                ));
            }
            self.merged += 1;
            return Ok(i);
        }
        let i = self.xy.len() as u32;
        self.xy.push(Vec2 { x, y });
        self.z.push(z);
        self.sources.push(source.clone());
        self.at.insert(key(x, y), i);
        Ok(i)
    }
}

fn finite(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite())
}

fn check(input: &SurfaceInput) -> Result<f64, SurfaceError> {
    let tolerance = input.tolerance.unwrap_or(DEFAULT_TOLERANCE);
    if !(tolerance.is_finite() && tolerance >= 0.0) {
        return Err(SurfaceError::new(
            "invalid_input",
            "tolerance 0 ya da pozitif bir sayı olmalı (metre).",
            None,
        ));
    }
    if let EdgeMethod::MaxEdge { length } = input.edge
        && !(length.is_finite() && length > 0.0)
    {
        return Err(SurfaceError::new(
            "invalid_input",
            "En uzun kenar pozitif bir uzunluk olmalı (metre).",
            None,
        ));
    }
    for (i, p) in input.points.iter().enumerate() {
        if !finite(&[p.x, p.y, p.z]) {
            return Err(SurfaceError::new(
                "invalid_input",
                format!("points[{i}]: x, y ve z sayı olmalı."),
                None,
            ));
        }
    }
    for (i, b) in input.breaklines.iter().enumerate() {
        let least = if b.closed { 3 } else { 2 };
        if b.points.len() < least {
            return Err(SurfaceError::new(
                "invalid_input",
                format!("breaklines[{i}]: kırık hatta en az {least} köşe olmalı."),
                None,
            ));
        }
        for p in &b.points {
            if !finite(p) {
                return Err(SurfaceError::new(
                    "invalid_input",
                    format!("breaklines[{i}]: köşelerin x, y ve z'si sayı olmalı."),
                    None,
                ));
            }
        }
        for w in pairs(b.points.len(), b.closed) {
            let (p, q) = (b.points[w.0], b.points[w.1]);
            if p[0] == q[0] && p[1] == q[1] {
                return Err(SurfaceError::new(
                    "invalid_input",
                    format!("breaklines[{i}]: ardışık iki köşe aynı yerde; sıfır boylu parça olmaz."),
                    Some([p[0], p[1]]),
                ));
            }
        }
    }
    let rings = input
        .boundary
        .iter()
        .map(|r| ("boundary".to_owned(), r))
        .chain(
            input
                .voids
                .iter()
                .enumerate()
                .map(|(i, r)| (format!("voids[{i}]"), r)),
        );
    for (name, r) in rings {
        if r.points.len() < 3 {
            return Err(SurfaceError::new(
                "invalid_input",
                format!("{name}: halkada en az üç köşe olmalı."),
                None,
            ));
        }
        for (k, p) in r.points.iter().enumerate() {
            let q = r.points[(k + 1) % r.points.len()];
            if !finite(p) {
                return Err(SurfaceError::new(
                    "invalid_input",
                    format!("{name}: köşelerin x ve y'si sayı olmalı."),
                    None,
                ));
            }
            if p[0] == q[0] && p[1] == q[1] {
                return Err(SurfaceError::new(
                    "invalid_input",
                    format!("{name}: ardışık iki köşe aynı yerde."),
                    Some(*p),
                ));
            }
        }
    }
    Ok(tolerance)
}

/// A path's pieces as corner index pairs.
fn pairs(n: usize, closed: bool) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = (0..n.saturating_sub(1)).map(|i| (i, i + 1)).collect();
    if closed && n > 2 {
        out.push((n - 1, 0));
    }
    out
}

/// The surface of `input` (ADR 0900).
pub fn build(input: &SurfaceInput) -> Result<Surface, SurfaceError> {
    let tolerance = check(input)?;
    let mut corners = Corners::default();
    for p in &input.points {
        corners.add(p.x, p.y, p.z, &p.source, tolerance)?;
    }
    let mut pieces = Vec::new();
    for b in &input.breaklines {
        let ids = b
            .points
            .iter()
            .map(|p| corners.add(p[0], p[1], p[2], &b.source, tolerance))
            .collect::<Result<Vec<_>, _>>()?;
        for (i, j) in pairs(ids.len(), b.closed) {
            pieces.push(Piece {
                a: ids[i],
                b: ids[j],
                breakline: true,
            });
        }
    }
    let rings: Vec<&Ring> = input.boundary.iter().chain(input.voids.iter()).collect();
    if !rings.is_empty() {
        // Drape (ADR 0900 §5): the rings' corners take the heights of the surface of the points and breaklines.
        let data = triangulate(&corners, &pieces, tolerance)?;
        let mut first = 0;
        for ring in &rings {
            let mut ids = Vec::with_capacity(ring.points.len());
            for p in &ring.points {
                let i = match corners.at.get(&key(p[0], p[1])) {
                    Some(&i) => i,
                    None => {
                        let (z, t) = data.0.height_in(&corners.xy, &corners.z, *p, first);
                        let Some(z) = z else {
                            return Err(SurfaceError::new(
                                "boundary_outside",
                                format!(
                                    "({}, {}) köşesi noktaların ve kırık hatların yüzeyinin dışında; sınır ve \
                                     boşluk köşeleri yüzeyin kotunu alır, dışarıda kot yoktur.",
                                    p[0], p[1]
                                ),
                                Some(*p),
                            ));
                        };
                        first = t;
                        corners.add(p[0], p[1], z, &ring.source, tolerance)?
                    }
                };
                ids.push(i);
            }
            for (i, j) in pairs(ids.len(), true) {
                pieces.push(Piece {
                    a: ids[i],
                    b: ids[j],
                    breakline: false,
                });
            }
        }
    }
    let (mesh, split) = triangulate(&corners, &pieces, tolerance)?;
    let mut kinds = classify(&mesh, &corners.xy, input);
    let trimmed = match input.edge {
        EdgeMethod::None => 0,
        EdgeMethod::MaxEdge { length } => trim(&mesh, &corners.xy, &mut kinds, length),
    };
    let mut kept_index = vec![NONE; kinds.len()];
    let mut triangles = Vec::new();
    for (t, kind) in kinds.iter().enumerate() {
        if *kind == Kind::Kept {
            kept_index[t] = triangles.len() as u32;
            triangles.push(mesh.corners(t as u32));
        }
    }
    let mut constrained: Vec<[u32; 2]> = mesh.constrained_pairs();
    constrained.sort_unstable();
    Ok(Surface {
        vertices: corners
            .xy
            .iter()
            .zip(&corners.z)
            .map(|(p, z)| [p.x, p.y, *z])
            .collect(),
        sources: corners.sources,
        xy: corners.xy,
        z: corners.z,
        triangles,
        constrained,
        report: Report {
            merged: corners.merged,
            split,
            trimmed,
        },
        mesh,
        kinds,
        kept_index,
    })
}

/// The constrained Delaunay triangulation of the corners with the pieces, and how many pieces were split.
fn triangulate(
    corners: &Corners,
    pieces: &[Piece],
    tolerance: f64,
) -> Result<(Mesh, usize), SurfaceError> {
    let d = delaunay::triangulate(&corners.xy).map_err(|why| {
        let code = if why == delaunay::TOO_FEW {
            "too_few_points"
        } else {
            "invalid_input"
        };
        SurfaceError::new(code, why, None)
    })?;
    if !d.skipped.is_empty() {
        return Err(SurfaceError::new(
            "internal",
            "Üçgenleme bazı noktaları dışarıda bıraktı; bu bir yazılım hatasıdır, durumu bildirin.",
            None,
        ));
    }
    let mut mesh = Mesh::new(d, corners.xy.len());
    let mut split = 0;
    let mut todo: Vec<Piece> = pieces.to_vec();
    todo.reverse();
    while let Some(p) = todo.pop() {
        match mesh.walk(&corners.xy, p.a, p.b) {
            Walk::Crossed(at) => {
                return Err(SurfaceError::new(
                    "constraints_cross",
                    format!(
                        "Kırık hatlar ya da sınır halkaları ({:.3}, {:.3}) yakınında kesişiyor; yalnız ortak \
                         köşelerinde buluşabilirler. Kesişme yerine iki hatta da ortak bir köşe koyun.",
                        at[0], at[1]
                    ),
                    Some(at),
                ));
            }
            Walk::Through(w) => {
                if p.breakline {
                    let (a, b, c) = (
                        corners.xy[p.a as usize],
                        corners.xy[p.b as usize],
                        corners.xy[w as usize],
                    );
                    let along = ((c.x - a.x).hypot(c.y - a.y)) / ((b.x - a.x).hypot(b.y - a.y));
                    let (za, zb) = (corners.z[p.a as usize], corners.z[p.b as usize]);
                    let line = za + along * (zb - za);
                    let had = corners.z[w as usize];
                    if (had - line).abs() > tolerance {
                        return Err(SurfaceError::new(
                            "height_conflict",
                            format!(
                                "({}, {}) noktası kırık hattın üstünde ama kotu {had} m, kırık hattın orada kotu \
                                 {line:.3} m. Noktayı ya da kırık hattı düzeltin.",
                                c.x, c.y
                            ),
                            Some([c.x, c.y]),
                        ));
                    }
                }
                split += 1;
                todo.push(Piece { a: w, ..p });
                todo.push(Piece { b: w, ..p });
            }
            Walk::Edges(crossed) => mesh
                .recover(&corners.xy, p.a, p.b, crossed)
                .map_err(|why| SurfaceError::new("internal", why, None))?,
        }
    }
    mesh.restore_delaunay(&corners.xy);
    Ok((mesh, split))
}

/// Whether `q` is inside the ring (even-odd, the edges' straddles decided exactly).
fn in_ring(q: Vec2, ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for k in 0..ring.len() {
        let a = Vec2 {
            x: ring[k][0],
            y: ring[k][1],
        };
        let b = Vec2 {
            x: ring[(k + 1) % ring.len()][0],
            y: ring[(k + 1) % ring.len()][1],
        };
        let (lo, hi) = if a.y <= b.y { (a, b) } else { (b, a) };
        if lo.y <= q.y && q.y < hi.y && orient2d(lo, hi, q) > 0.0 {
            inside = !inside;
        }
    }
    inside
}

/// Each triangle kept, outside the boundary or in a void: one centroid tested per region the constraints bound.
fn classify(mesh: &Mesh, xy: &[Vec2], input: &SurfaceInput) -> Vec<Kind> {
    let n = mesh.len();
    let mut kinds = vec![Kind::Kept; n];
    if input.boundary.is_none() && input.voids.is_empty() {
        return kinds;
    }
    let mut seen = vec![false; n];
    for start in 0..n {
        if seen[start] {
            continue;
        }
        let [a, b, c] = mesh.corners(start as u32).map(|i| xy[i as usize]);
        let centre = Vec2 {
            x: (a.x + b.x + c.x) / 3.0,
            y: (a.y + b.y + c.y) / 3.0,
        };
        let kind = if input
            .boundary
            .as_ref()
            .is_some_and(|r| !in_ring(centre, &r.points))
        {
            Kind::Outside
        } else if input.voids.iter().any(|r| in_ring(centre, &r.points)) {
            Kind::Void
        } else {
            Kind::Kept
        };
        let mut stack = vec![start as u32];
        seen[start] = true;
        while let Some(t) = stack.pop() {
            kinds[t as usize] = kind;
            for e in 3 * t..3 * t + 3 {
                if mesh.is_constrained(e) {
                    continue;
                }
                let tw = mesh.twin(e);
                if tw != NONE && !seen[(tw / 3) as usize] {
                    seen[(tw / 3) as usize] = true;
                    stack.push(tw / 3);
                }
            }
        }
    }
    kinds
}

/// The edge method's longest outer edge (ADR 0900 §6): outer triangles with a longer outer edge go, until none is.
fn trim(mesh: &Mesh, xy: &[Vec2], kinds: &mut [Kind], length: f64) -> usize {
    let mut trimmed = 0;
    loop {
        let mut changed = false;
        for t in 0..kinds.len() {
            if kinds[t] != Kind::Kept {
                continue;
            }
            for e in 3 * t as u32..3 * t as u32 + 3 {
                if mesh.is_constrained(e) {
                    continue;
                }
                let tw = mesh.twin(e);
                let outer = tw == NONE || kinds[(tw / 3) as usize] == Kind::Trimmed;
                if outer && mesh.edge_length(xy, e) > length {
                    kinds[t] = Kind::Trimmed;
                    trimmed += 1;
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            return trimmed;
        }
    }
}
