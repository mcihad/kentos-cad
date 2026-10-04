//! Köşe tablosu (docs/adr/0172), one for both platforms (the web through
//! WASM): an object's vertices as table rows, and the table's writes on its
//! paths with their elevations (`elevation::Elevated`, in the order of the
//! elevations' `paths`: a line's two ends, a polyline, an area's outer ring,
//! its holes, then each other part's ring and holes). A write gives the new
//! paths or why it is refused; the words are the table's, on each platform.
//! The independent reference is `scripts/fixtures/vertex_table_cases.py`.
//!
//! An edge's arc is its bulge (tan θ/4, counter-clockwise positive; ADR
//! 0069); the table shows it as a signed radius, plus where the arc turns
//! left. A radius is taken back to a bulge with square roots only, so both
//! platforms give the same bits.

use crate::api::Op;
use crate::api::json::{FromJson, Json, ToJson, field};
use crate::geom::bulge::{bulge_at, has_bulges, is_arc_bulge};
use crate::jsmath::js_hypot;
use crate::op;
use crate::ops::elevation::Elevated;
use crate::vec2::Vec2;

/// Two vertices this close are one place (`clean_bulge_path`'s tolerance).
pub const SAME: f64 = 1e-9;

/// What the object is: its paths say the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Line,
    Polyline,
    Polygon,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Line => "line",
            Kind::Polyline => "polyline",
            Kind::Polygon => "polygon",
        }
    }
}

impl FromJson for Kind {
    fn from_json(v: &Json) -> Result<Kind, String> {
        match String::from_json(v)?.as_str() {
            "line" => Ok(Kind::Line),
            "polyline" => Ok(Kind::Polyline),
            "polygon" => Ok(Kind::Polygon),
            other => Err(format!("köşe tablosunda “{other}” türü yok")),
        }
    }
}

impl ToJson for Kind {
    fn write_json(&self, out: &mut String) {
        self.name().write_json(out);
    }
}

/// A vertex as the table shows it: its path and its place in it, where it
/// is, its elevation, and the edge leaving it (to the next vertex; a closed
/// path's last to its first): the chord and, on an arc, the signed radius.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub path: usize,
    pub index: usize,
    pub p: Vec2,
    pub z: Option<f64>,
    pub chord: Option<f64>,
    pub radius: Option<f64>,
}

crate::json_struct!(Row {
    path,
    index,
    p,
    z,
    chord,
    radius
});

/// Why a write is refused.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// The table names a vertex the paths do not have.
    Missing,
    /// A vertex moved, or a new one put, where a neighbouring vertex is.
    OntoNeighbour,
    /// A radius for the last vertex of an open path: no edge leaves it.
    NoEdge,
    /// A line's edge made an arc: a line has none.
    LineArc,
    /// An arc on an edge whose two ends are one place.
    NoChord,
    /// A radius shorter than half the chord, by more than the slack.
    RadiusBelow { least: f64 },
    /// A line's end removed.
    LineEnds,
    /// A polyline left with fewer than two vertices.
    PathMin,
    /// A ring left with fewer than three vertices.
    RingMin,
}

crate::json_tagged!(Refusal, "why",
    Missing => "missing" {},
    OntoNeighbour => "ontoNeighbour" {},
    NoEdge => "noEdge" {},
    LineArc => "lineArc" {},
    NoChord => "noChord" {},
    RadiusBelow => "radiusBelow" { least },
    LineEnds => "lineEnds" {},
    PathMin => "pathMin" {},
    RingMin => "ringMin" {},
);

/// The object after a write: its kind (a line given a vertex is a polyline)
/// and its paths.
#[derive(Clone, Debug, PartialEq)]
pub struct Edited {
    pub kind: Kind,
    pub paths: Vec<Elevated>,
}

crate::json_struct!(Edited { kind, paths });

/// A write's answer for the web: the object, or why not.
pub struct Answer(pub Result<Edited, Refusal>);

impl ToJson for Answer {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(e) => field(out, &mut first, "edited", e),
            Err(r) => field(out, &mut first, "refusal", r),
        }
        out.push('}');
    }
}

fn same(a: Vec2, b: Vec2) -> bool {
    js_hypot(a.x - b.x, a.y - b.y) <= SAME
}

/// The vertex after `i` along the path: its next, a closed path's first
/// after its last; none past an open path's end.
fn next_of(p: &Elevated, i: usize) -> Option<usize> {
    let n = p.pts.len();
    if i + 1 < n {
        Some(i + 1)
    } else if p.closed && n > 1 {
        Some(0)
    } else {
        None
    }
}

/// The vertex before `i` along the path.
fn prev_of(p: &Elevated, i: usize) -> Option<usize> {
    let n = p.pts.len();
    if i > 0 {
        Some(i - 1)
    } else if p.closed && n > 1 {
        Some(n - 1)
    } else {
        None
    }
}

/// The signed radius of an edge with this chord and bulge; none when it is
/// straight (or has no length).
pub fn radius_of(chord: f64, bulge: f64) -> Option<f64> {
    if !is_arc_bulge(bulge) || chord <= 0.0 {
        return None;
    }
    let r = chord * (1.0 + bulge * bulge) / (4.0 * bulge.abs());
    Some(if bulge < 0.0 { -r } else { r })
}

/// The bulge of an arc of signed radius `radius` on a chord: `s` the sine
/// of half its angle (chord over the diameter), the larger arc when `major`.
fn bulge_of(s: f64, major: bool, radius: f64) -> f64 {
    let root = ((1.0 - s) * (1.0 + s)).sqrt();
    let t = if major {
        (1.0 + root) / s
    } else {
        s / (1.0 + root)
    };
    if radius < 0.0 { -t } else { t }
}

/// The table's rows: every vertex of every path, in order.
pub fn rows(paths: &[Elevated]) -> Vec<Row> {
    let mut out = Vec::new();
    for (k, p) in paths.iter().enumerate() {
        for (i, &at) in p.pts.iter().enumerate() {
            let (chord, radius) = match next_of(p, i) {
                Some(j) => {
                    let b = p.pts[j];
                    let chord = js_hypot(b.x - at.x, b.y - at.y);
                    (
                        Some(chord),
                        radius_of(chord, bulge_at(p.bulges.as_deref(), i)),
                    )
                }
                None => (None, None),
            };
            out.push(Row {
                path: k,
                index: i,
                p: at,
                z: p.zs.get(i).copied().flatten(),
                chord,
                radius,
            });
        }
    }
    out
}

fn vertex(paths: &[Elevated], path: usize, index: usize) -> Result<&Elevated, Refusal> {
    match paths.get(path) {
        Some(p) if index < p.pts.len() => Ok(p),
        _ => Err(Refusal::Missing),
    }
}

/// Bulges as many as the vertices, an open path's last straight; none when
/// every edge is.
fn tidy(mut bulges: Vec<f64>, closed: bool) -> Option<Vec<f64>> {
    if !closed && let Some(last) = bulges.last_mut() {
        *last = 0.0;
    }
    has_bulges(Some(&bulges)).then_some(bulges)
}

fn full_bulges(p: &Elevated) -> Vec<f64> {
    (0..p.pts.len())
        .map(|i| bulge_at(p.bulges.as_deref(), i))
        .collect()
}

fn full_zs(p: &Elevated) -> Vec<Option<f64>> {
    (0..p.pts.len())
        .map(|i| p.zs.get(i).copied().flatten())
        .collect()
}

/// Vertex `index` of path `path` moved to `to`; its edges keep their bulges
/// (the grip's rule, ADR 0068). Not onto a neighbouring vertex.
pub fn moved(
    paths: &[Elevated],
    path: usize,
    index: usize,
    to: Vec2,
) -> Result<Vec<Elevated>, Refusal> {
    let p = vertex(paths, path, index)?;
    for n in [prev_of(p, index), next_of(p, index)].into_iter().flatten() {
        if n != index && same(p.pts[n], to) {
            return Err(Refusal::OntoNeighbour);
        }
    }
    let mut out = paths.to_vec();
    out[path].pts[index] = to;
    Ok(out)
}

/// Vertex `index` of path `path` given the elevation `z` (none removes it).
pub fn with_z(
    paths: &[Elevated],
    path: usize,
    index: usize,
    z: Option<f64>,
) -> Result<Vec<Elevated>, Refusal> {
    let p = vertex(paths, path, index)?;
    let mut zs = full_zs(p);
    zs[index] = z;
    let mut out = paths.to_vec();
    out[path].zs = zs;
    Ok(out)
}

/// The edge leaving vertex `index` of path `path` given the signed radius
/// `radius`: none or 0 makes it straight; plus turns left. An arc keeps its
/// size (more or less than half a circle; a straight edge takes the
/// smaller). Shorter than half the chord is refused, unless by at most
/// `slack` (half a unit of what the table shows): then it is half a circle.
pub fn with_radius(
    kind: Kind,
    paths: &[Elevated],
    path: usize,
    index: usize,
    radius: Option<f64>,
    slack: f64,
) -> Result<Vec<Elevated>, Refusal> {
    let p = vertex(paths, path, index)?;
    if kind == Kind::Line {
        return Err(Refusal::LineArc);
    }
    let Some(j) = next_of(p, index) else {
        return Err(Refusal::NoEdge);
    };
    let bulge = match radius {
        Some(r) if !r.is_finite() => return Err(Refusal::Missing),
        Some(r) if r != 0.0 => {
            let (a, b) = (p.pts[index], p.pts[j]);
            let chord = js_hypot(b.x - a.x, b.y - a.y);
            if chord <= SAME {
                return Err(Refusal::NoChord);
            }
            let half = chord / 2.0;
            let s = if r.abs() >= half {
                half / r.abs()
            } else if half - r.abs() <= slack {
                1.0
            } else {
                return Err(Refusal::RadiusBelow { least: half });
            };
            let major = bulge_at(p.bulges.as_deref(), index).abs() > 1.0;
            bulge_of(s, major, r)
        }
        _ => 0.0,
    };
    let mut bulges = full_bulges(p);
    bulges[index] = bulge;
    let mut out = paths.to_vec();
    out[path].bulges = tidy(bulges, p.closed);
    Ok(out)
}

/// A vertex at `at` with elevation `z` put after vertex `after` of path
/// `path`: between it and the next (a closed path's last and first), or at
/// an open path's end. The edge it splits becomes two straight edges; a
/// line given a vertex is a polyline. Not onto a neighbouring vertex.
pub fn inserted(
    kind: Kind,
    paths: &[Elevated],
    path: usize,
    after: usize,
    at: Vec2,
    z: Option<f64>,
) -> Result<Edited, Refusal> {
    let p = vertex(paths, path, after)?;
    let next = next_of(p, after);
    if same(p.pts[after], at) || next.is_some_and(|j| same(p.pts[j], at)) {
        return Err(Refusal::OntoNeighbour);
    }
    let mut pts = p.pts.clone();
    let mut zs = full_zs(p);
    let mut bulges = full_bulges(p);
    pts.insert(after + 1, at);
    zs.insert(after + 1, z);
    bulges[after] = 0.0;
    bulges.insert(after + 1, 0.0);
    let mut out = paths.to_vec();
    out[path] = Elevated {
        pts,
        bulges: tidy(bulges, p.closed),
        closed: p.closed,
        zs,
    };
    Ok(Edited {
        kind: if kind == Kind::Line {
            Kind::Polyline
        } else {
            kind
        },
        paths: out,
    })
}

/// The vertices `at` (each its path and index) removed in one write. A
/// vertex left keeps its edge's bulge while the edge's far vertex stays;
/// otherwise its new edge, to the next vertex left, is straight (Köşe sil's
/// rule). A polyline keeps two vertices, a ring three; a line both ends.
pub fn removed(
    kind: Kind,
    paths: &[Elevated],
    at: &[[usize; 2]],
) -> Result<Vec<Elevated>, Refusal> {
    let mut gone: Vec<Vec<bool>> = paths.iter().map(|p| vec![false; p.pts.len()]).collect();
    for &[path, index] in at {
        vertex(paths, path, index)?;
        gone[path][index] = true;
    }
    if kind == Kind::Line && gone.iter().flatten().any(|&g| g) {
        return Err(Refusal::LineEnds);
    }
    let mut out = paths.to_vec();
    for (k, p) in paths.iter().enumerate() {
        let off = &gone[k];
        if !off.iter().any(|&g| g) {
            continue;
        }
        let left = off.iter().filter(|&&g| !g).count();
        if p.closed && left < 3 {
            return Err(Refusal::RingMin);
        }
        if !p.closed && left < 2 {
            return Err(Refusal::PathMin);
        }
        let (bulges, zs) = (full_bulges(p), full_zs(p));
        let mut kept = Elevated {
            pts: Vec::with_capacity(left),
            bulges: None,
            closed: p.closed,
            zs: Vec::with_capacity(left),
        };
        let mut kept_bulges = Vec::with_capacity(left);
        for i in (0..p.pts.len()).filter(|&i| !off[i]) {
            kept.pts.push(p.pts[i]);
            kept.zs.push(zs[i]);
            let far_stays = next_of(p, i).is_some_and(|j| !off[j]);
            kept_bulges.push(if far_stays { bulges[i] } else { 0.0 });
        }
        kept.bulges = tidy(kept_bulges, p.closed);
        out[k] = kept;
    }
    Ok(out)
}

pub(crate) static OPS: &[Op] = &[
    op!("vertexTableRows", |paths: Vec<Elevated>| rows(&paths)),
    op!("vertexTableMove", |kind: Kind,
                            paths: Vec<Elevated>,
                            path: usize,
                            index: usize,
                            to: Vec2| {
        Answer(moved(&paths, path, index, to).map(|paths| Edited { kind, paths }))
    }),
    op!("vertexTableZ", |kind: Kind,
                         paths: Vec<Elevated>,
                         path: usize,
                         index: usize,
                         z: Option<f64>| {
        Answer(with_z(&paths, path, index, z).map(|paths| Edited { kind, paths }))
    }),
    op!(
        "vertexTableRadius",
        |kind: Kind,
         paths: Vec<Elevated>,
         path: usize,
         index: usize,
         radius: Option<f64>,
         slack: f64| {
            Answer(
                with_radius(kind, &paths, path, index, radius, slack)
                    .map(|paths| Edited { kind, paths }),
            )
        }
    ),
    op!(
        "vertexTableInsert",
        |kind: Kind, paths: Vec<Elevated>, path: usize, after: usize, at: Vec2, z: Option<f64>| {
            Answer(inserted(kind, &paths, path, after, at, z))
        }
    ),
    op!(
        "vertexTableRemove",
        |kind: Kind, paths: Vec<Elevated>, at: Vec<[usize; 2]>| {
            Answer(removed(kind, &paths, &at).map(|paths| Edited { kind, paths }))
        }
    ),
];
