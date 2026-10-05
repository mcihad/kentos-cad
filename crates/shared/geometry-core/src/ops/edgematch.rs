//! Kenar eşleme (docs/adr/0159), one for both platforms (the web through
//! WASM): the line ends of two sheets that meet across their common edge,
//! found as links (the best continuation within a search distance and an
//! angle) and put together: an end moved, a segment added, or the vertices
//! adjusted by a shift that fades along the line; at the adjacent end, in
//! the middle or on a border. The independent reference is
//! `scripts/fixtures/edgematch_cases.py` (mpmath, 50 digits).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::api::Op;
use crate::api::json::{FromJson, Json, ToJson, field};
use crate::entity::{Shape, entity_vertices, is_multi_part};
use crate::geom::intersect::{Edge, closest_on_edge};
use crate::geometry::dist;
use crate::jsmath::{PI, atan, atan2, cos, js_hypot, sin};
use crate::op;
use crate::ops::edges::entity_edges;
use crate::vec2::Vec2;

/// Two points this close are one (the elevations' “at a vertex”, docs/adr/0142).
pub const SAME: f64 = 1e-6;

/// A bulge this small is a straight edge (`geom::bulge`).
const STRAIGHT: f64 = 1e-12;

/// A shape's paths' elevations: a path's per vertex (a line's two ends).
pub type Elevations = Vec<Vec<Option<f64>>>;

/// A member put right: its new shape and elevations.
pub type Put = (Shape, Elevations);

/// An end's place in the order links are taken in: its member, then 0 for
/// its first end and 1 for its last.
type Place = (usize, usize);

/// Which end of a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum End {
    First,
    Last,
}

impl End {
    fn code(self) -> &'static str {
        match self {
            End::First => "first",
            End::Last => "last",
        }
    }

    fn index(self) -> usize {
        match self {
            End::First => 0,
            End::Last => 1,
        }
    }
}

impl ToJson for End {
    fn write_json(&self, out: &mut String) {
        self.code().write_json(out);
    }
}

impl FromJson for End {
    fn from_json(v: &Json) -> Result<End, String> {
        match String::from_json(v)?.as_str() {
            "first" => Ok(End::First),
            "last" => Ok(End::Last),
            other => Err(format!("“{other}” bir uç değil: first ya da last")),
        }
    }
}

/// An object of a set: its shape, its paths' elevations, and its key for
/// the criterion (a layer's name or an attribute's value; none without one).
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub shape: Shape,
    pub zs: Elevations,
    pub key: Option<String>,
}

crate::json_struct!(Member { shape, zs, key });

/// What the links are found with: the search distance (m), the angle
/// tolerance (degrees), the sheet's border, and whether keys must match.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub distance: f64,
    pub angle: f64,
    pub border: Option<Shape>,
    pub keyed: bool,
}

crate::json_struct!(Settings {
    distance,
    angle,
    border,
    keyed
});

/// A link: a source end and the adjacent end it meets; the gap (m), the
/// angle between the one's direction and the other's turned round
/// (degrees), and its score.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub source: usize,
    pub source_end: End,
    pub adjacent: usize,
    pub adjacent_end: End,
    pub from: Vec2,
    pub to: Vec2,
    pub gap: f64,
    pub angle: f64,
    pub score: f64,
}

impl ToJson for Link {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "source", &(self.source as f64));
        field(out, &mut first, "sourceEnd", &self.source_end);
        field(out, &mut first, "adjacent", &(self.adjacent as f64));
        field(out, &mut first, "adjacentEnd", &self.adjacent_end);
        field(out, &mut first, "from", &self.from);
        field(out, &mut first, "to", &self.to);
        field(out, &mut first, "gap", &self.gap);
        field(out, &mut first, "angle", &self.angle);
        field(out, &mut first, "score", &self.score);
        out.push('}');
    }
}

impl FromJson for Link {
    fn from_json(v: &Json) -> Result<Link, String> {
        use crate::api::json::read_field;
        Ok(Link {
            source: read_field(v, "source")?,
            source_end: read_field(v, "sourceEnd")?,
            adjacent: read_field(v, "adjacent")?,
            adjacent_end: read_field(v, "adjacentEnd")?,
            from: read_field(v, "from")?,
            to: read_field(v, "to")?,
            gap: read_field(v, "gap")?,
            angle: read_field(v, "angle")?,
            score: read_field(v, "score")?,
        })
    }
}

/// What a search finds: the links; the free source ends near the edge left
/// unmatched; the junction ends there; the source objects that take no
/// part (other kinds, closed or empty paths).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Found {
    pub links: Vec<Link>,
    pub unmatched: Vec<(usize, End)>,
    pub junctions: usize,
    pub others: usize,
}

impl ToJson for Found {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "links", &self.links);
        out.push_str(",\"unmatched\":[");
        for (k, (i, end)) in self.unmatched.iter().enumerate() {
            if k > 0 {
                out.push(',');
            }
            out.push('{');
            let mut f = true;
            field(out, &mut f, "source", &(*i as f64));
            field(out, &mut f, "end", end);
            out.push('}');
        }
        out.push(']');
        first = false;
        field(out, &mut first, "junctions", &(self.junctions as f64));
        field(out, &mut first, "others", &(self.others as f64));
        out.push('}');
    }
}

// ── Paths and ends ──

/// A line's or an open polyline's vertices and bulges; none for any other
/// kind, an empty line, a closed polyline or a multi-part one (which part
/// would be matched is not known; docs/adr/0174).
fn path(shape: &Shape) -> Option<(Vec<Vec2>, Vec<f64>)> {
    if is_multi_part(shape) {
        return None;
    }
    match shape {
        Shape::Line { a, b } => (dist(*a, *b) > SAME).then(|| (vec![*a, *b], vec![0.0])),
        Shape::Polyline { pts, bulges, .. } => {
            let n = pts.len();
            if n < 2 || dist(pts[0], pts[n - 1]) <= SAME {
                return None;
            }
            let mut b = bulges.clone().unwrap_or_default();
            b.resize(n - 1, 0.0);
            Some((pts.clone(), b))
        }
        _ => None,
    }
}

fn unit(x: f64, y: f64) -> Vec2 {
    let n = js_hypot(x, y);
    Vec2::new(x / n, y / n)
}

fn turn(v: Vec2, phi: f64) -> Vec2 {
    let (c, s) = (cos(phi), sin(phi));
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// An end's outward direction: along its edge towards the end; an arc
/// edge's tangent there (the chord turned by half the sweep).
fn outward(pts: &[Vec2], bulges: &[f64], end: End) -> Option<Vec2> {
    let n = pts.len();
    match end {
        End::First => {
            let k = (1..n).find(|&k| dist(pts[k], pts[0]) > SAME)?;
            if k == 1 && bulges[0].abs() > STRAIGHT {
                let chord = unit(pts[1].x - pts[0].x, pts[1].y - pts[0].y);
                let t = turn(chord, -2.0 * atan(bulges[0]));
                Some(Vec2::new(-t.x, -t.y))
            } else {
                Some(unit(pts[0].x - pts[k].x, pts[0].y - pts[k].y))
            }
        }
        End::Last => {
            let k = (0..n - 1)
                .rev()
                .find(|&k| dist(pts[k], pts[n - 1]) > SAME)?;
            if k == n - 2 && bulges[n - 2].abs() > STRAIGHT {
                let chord = unit(pts[n - 1].x - pts[n - 2].x, pts[n - 1].y - pts[n - 2].y);
                Some(turn(chord, 2.0 * atan(bulges[n - 2])))
            } else {
                Some(unit(pts[n - 1].x - pts[k].x, pts[n - 1].y - pts[k].y))
            }
        }
    }
}

/// An end of a member that takes part: whose, which, where, its outward direction.
#[derive(Clone, Copy, Debug)]
struct EndAt {
    member: usize,
    end: End,
    at: Vec2,
    out: Vec2,
}

fn ends_of(members: &[Member]) -> Vec<EndAt> {
    let mut out = Vec::new();
    for (i, m) in members.iter().enumerate() {
        let Some((pts, bulges)) = path(&m.shape) else {
            continue;
        };
        for end in [End::First, End::Last] {
            if let Some(u) = outward(&pts, &bulges, end) {
                let at = if end == End::First {
                    pts[0]
                } else {
                    pts[pts.len() - 1]
                };
                out.push(EndAt {
                    member: i,
                    end,
                    at,
                    out: u,
                });
            }
        }
    }
    out
}

/// A key as the criterion compares it: trimmed, Turkish lower case
/// (`text::edit::fold`, as Bul ve değiştir compares).
fn key_of(m: &Member) -> Option<String> {
    m.key
        .as_deref()
        .map(|k| k.trim().chars().map(crate::text::edit::fold).collect())
}

// ── A grid of points ──

/// Points by the square of a grid they fall in, with what they belong to;
/// asked only for what lies within one square's side.
struct Grid<T> {
    cell: f64,
    cells: HashMap<(i64, i64), Vec<(Vec2, T)>>,
}

impl<T: Copy> Grid<T> {
    fn new(cell: f64) -> Self {
        Self {
            cell,
            cells: HashMap::new(),
        }
    }

    fn key(&self, p: Vec2) -> (i64, i64) {
        (
            (p.x / self.cell).floor() as i64,
            (p.y / self.cell).floor() as i64,
        )
    }

    fn put(&mut self, p: Vec2, v: T) {
        let k = self.key(p);
        self.cells.entry(k).or_default().push((p, v));
    }

    /// What lies within `r` (at most the cell) of `p`.
    fn near(&self, p: Vec2, r: f64) -> Vec<(Vec2, T)> {
        let (cx, cy) = self.key(p);
        let mut out = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(items) = self.cells.get(&(cx + dx, cy + dy)) {
                    out.extend(items.iter().filter(|(q, _)| dist(*q, p) <= r).copied());
                }
            }
        }
        out
    }
}

/// Every vertex of the lines, polylines (open or closed) and areas (rings,
/// holes and parts) of a set, by member.
fn vertex_grid(members: &[Member]) -> Grid<usize> {
    let mut g = Grid::new(1.0);
    for (i, m) in members.iter().enumerate() {
        if matches!(
            m.shape,
            Shape::Line { .. } | Shape::Polyline { .. } | Shape::Polygon { .. }
        ) {
            for p in entity_vertices(&m.shape) {
                g.put(p, i);
            }
        }
    }
    g
}

fn end_grid(ends: &[EndAt], cell: f64) -> Grid<usize> {
    let mut g = Grid::new(cell);
    for (k, e) in ends.iter().enumerate() {
        g.put(e.at, k);
    }
    g
}

// ── The border ──

/// The point of the border nearest to `p` and its distance; the first
/// edge's on a tie.
fn to_border(edges: &[Edge], p: Vec2) -> Option<(Vec2, f64)> {
    let mut best: Option<(Vec2, f64)> = None;
    for e in edges {
        let c = closest_on_edge(e, p);
        if best.is_none_or(|(_, d)| c.d < d) {
            best = Some((c.p, c.d));
        }
    }
    best
}

// ── Links ──

/// The links between `sources` and `adjacent` (docs/adr/0159 §3–§4).
pub fn links(sources: &[Member], adjacent: &[Member], s: &Settings) -> Found {
    let d = s.distance;
    let alpha = s.angle * PI / 180.0;
    // A search distance or an angle that is not a positive number finds nothing.
    if !(d.is_finite() && d > 0.0 && alpha.is_finite() && alpha > 0.0) {
        return Found {
            others: sources.iter().filter(|m| path(&m.shape).is_none()).count(),
            ..Found::default()
        };
    }
    let edges = s.border.as_ref().map(entity_edges).unwrap_or_default();
    let src = ends_of(sources);
    let adj = ends_of(adjacent);
    let (src_vertices, adj_vertices) = (vertex_grid(sources), vertex_grid(adjacent));
    let (src_at, adj_at) = (end_grid(&src, 1.0), end_grid(&adj, 1.0));
    let junction = |grid: &Grid<usize>, e: &EndAt| {
        grid.near(e.at, SAME)
            .iter()
            .any(|&(_, owner)| owner != e.member)
    };
    let connected = |grid: &Grid<usize>, e: &EndAt| !grid.near(e.at, SAME).is_empty();
    let free = |ends: &[EndAt], own: &Grid<usize>, other: &Grid<usize>| -> Vec<bool> {
        ends.iter()
            .map(|e| !junction(own, e) && !connected(other, e))
            .collect()
    };
    let free_src = free(&src, &src_vertices, &adj_at);
    let free_adj = free(&adj, &adj_vertices, &src_at);
    let reach = end_grid(&adj, d);
    let near = |p: Vec2| -> bool {
        if !edges.is_empty() {
            return to_border(&edges, p).is_some_and(|(_, dd)| dd <= d);
        }
        !reach.near(p, d).is_empty()
    };
    let keys_src: Vec<Option<String>> = sources.iter().map(key_of).collect();
    let keys_adj: Vec<Option<String>> = adjacent.iter().map(key_of).collect();
    let mut free_adj_grid = Grid::new(d);
    for (k, e) in adj.iter().enumerate() {
        if free_adj[k] {
            free_adj_grid.put(e.at, k);
        }
    }
    // The candidates with their ends' places.
    let mut candidates: Vec<(Place, Place, Link)> = Vec::new();
    for (i, e) in src.iter().enumerate() {
        if !free_src[i] {
            continue;
        }
        for (_, k) in free_adj_grid.near(e.at, d) {
            let t = &adj[k];
            let gap = dist(e.at, t.at);
            if gap <= SAME || gap > d {
                continue;
            }
            let w = Vec2::new(-t.out.x, -t.out.y);
            let theta = atan2(
                (e.out.x * w.y - e.out.y * w.x).abs(),
                e.out.x * w.x + e.out.y * w.y,
            );
            if theta > alpha {
                continue;
            }
            if s.keyed {
                let (a, b) = (&keys_src[e.member], &keys_adj[t.member]);
                if a.is_none() || a != b {
                    continue;
                }
            }
            // With a border, both ends lie near it.
            let by_border = edges.is_empty() || (near(e.at) && near(t.at));
            if !by_border {
                continue;
            }
            let score = theta / alpha + gap / d;
            candidates.push((
                (e.member, e.end.index()),
                (t.member, t.end.index()),
                Link {
                    source: e.member,
                    source_end: e.end,
                    adjacent: t.member,
                    adjacent_end: t.end,
                    from: e.at,
                    to: t.at,
                    gap,
                    angle: theta * 180.0 / PI,
                    score,
                },
            ));
        }
    }
    candidates.sort_by(|a, b| {
        a.2.score
            .total_cmp(&b.2.score)
            .then(a.0.cmp(&b.0))
            .then(a.1.cmp(&b.1))
    });
    let (mut taken_src, mut taken_adj) = (BTreeSet::new(), BTreeSet::new());
    let mut found = Found::default();
    for (ks, ka, link) in candidates {
        if taken_src.contains(&ks) || taken_adj.contains(&ka) {
            continue;
        }
        taken_src.insert(ks);
        taken_adj.insert(ka);
        found.links.push(link);
    }
    for (i, e) in src.iter().enumerate() {
        if free_src[i] && !taken_src.contains(&(e.member, e.end.index())) && near(e.at) {
            found.unmatched.push((e.member, e.end));
        }
    }
    found.junctions = src
        .iter()
        .filter(|e| junction(&src_vertices, e) && near(e.at))
        .count();
    found.others = sources.iter().filter(|m| path(&m.shape).is_none()).count();
    found
}

// ── Putting them together ──

/// Where the two ends meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Meet {
    /// At the adjacent end; the adjacent line stays.
    Adjacent,
    /// Halfway; both lines are put right.
    Middle,
    /// On the border, nearest to halfway; both lines are put right.
    Border,
}

impl FromJson for Meet {
    fn from_json(v: &Json) -> Result<Meet, String> {
        match String::from_json(v)?.as_str() {
            "adjacent" => Ok(Meet::Adjacent),
            "middle" => Ok(Meet::Middle),
            "border" => Ok(Meet::Border),
            other => Err(format!("“{other}” bir buluşma yeri değil")),
        }
    }
}

/// How a line is put right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// Ucu taşı: the end goes there.
    Move,
    /// Parça ekle: a straight edge from the end to there.
    Segment,
    /// Köşeleri ayarla: the shift fades along the line to its other end.
    Adjust,
}

impl FromJson for Method {
    fn from_json(v: &Json) -> Result<Method, String> {
        match String::from_json(v)?.as_str() {
            "move" => Ok(Method::Move),
            "segment" => Ok(Method::Segment),
            "adjust" => Ok(Method::Adjust),
            other => Err(format!("“{other}” bir yöntem değil")),
        }
    }
}

/// An edge's length: its chord, or its arc's (r·|sweep|).
fn edge_length(a: Vec2, b: Vec2, bulge: f64) -> f64 {
    let chord = dist(a, b);
    if bulge.abs() <= STRAIGHT || chord < STRAIGHT {
        return chord;
    }
    let r = chord * (1.0 + bulge * bulge) / (4.0 * bulge.abs());
    r * 4.0 * atan(bulge).abs()
}

/// A member with its ends' targets: its new shape and elevations, or none
/// when two of its vertices in a row come within [`SAME`].
fn put(member: &Member, targets: [Option<Vec2>; 2], method: Method) -> Option<Put> {
    let (mut pts, mut bulges) = path(&member.shape)?;
    let mut zs = member.zs.clone();
    let n = pts.len();
    let path_z = zs.first().is_some_and(|z| z.len() == n);
    match method {
        Method::Adjust => {
            let lengths: Vec<f64> = (0..n - 1)
                .map(|k| edge_length(pts[k], pts[k + 1], bulges[k]))
                .collect();
            let total: f64 = lengths.iter().sum();
            let shift = |m: Option<Vec2>, p: Vec2| m.map_or((0.0, 0.0), |m| (m.x - p.x, m.y - p.y));
            let (d1, d2) = (shift(targets[0], pts[0]), shift(targets[1], pts[n - 1]));
            let mut along = 0.0;
            for (k, p) in pts.iter_mut().enumerate() {
                let f = along / total;
                *p = Vec2::new(
                    p.x + d1.0 * (1.0 - f) + d2.0 * f,
                    p.y + d1.1 * (1.0 - f) + d2.1 * f,
                );
                if k < n - 1 {
                    along += lengths[k];
                }
            }
            // The ends land exactly where they meet.
            if let Some(m) = targets[0] {
                pts[0] = m;
            }
            if let Some(m) = targets[1] {
                pts[n - 1] = m;
            }
        }
        Method::Move | Method::Segment => {
            for end in [End::First, End::Last] {
                let Some(m) = targets[end.index()] else {
                    continue;
                };
                let at = if end == End::First { 0 } else { pts.len() - 1 };
                if method == Method::Segment && dist(pts[at], m) > SAME {
                    if end == End::First {
                        pts.insert(0, m);
                        bulges.insert(0, 0.0);
                        if path_z {
                            let z = zs[0][0];
                            zs[0].insert(0, z);
                        }
                    } else {
                        pts.push(m);
                        bulges.push(0.0);
                        if path_z {
                            let z = zs[0][zs[0].len() - 1];
                            zs[0].push(z);
                        }
                    }
                } else {
                    pts[at] = m;
                }
            }
        }
    }
    if pts.windows(2).any(|w| dist(w[0], w[1]) <= SAME) {
        return None;
    }
    let shape = if matches!(member.shape, Shape::Line { .. }) && pts.len() == 2 {
        Shape::Line {
            a: pts[0],
            b: pts[1],
        }
    } else {
        Shape::Polyline {
            pts,
            bulges: Some(bulges),
            holes: None,
            parts: None,
        }
    };
    Some((shape, zs))
}

/// The lines put together: each changed member's shape and elevations (none
/// for an unchanged one), and the links not written (their indices).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Applied {
    pub sources: Vec<Option<Put>>,
    pub adjacent: Vec<Option<Put>>,
    pub refused: Vec<usize>,
}

/// Why nothing can be put together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyError {
    /// Sınırda without a border.
    NoBorder,
}

/// `links` written with `meet` and `method` (docs/adr/0159 §5–§6): a
/// member that would keep two vertices in a row within [`SAME`] has none of
/// its links written, and the rest is worked out again without them.
pub fn apply(
    sources: &[Member],
    adjacent: &[Member],
    links: &[Link],
    meet: Meet,
    method: Method,
    border: Option<&Shape>,
) -> Result<Applied, ApplyError> {
    let edges = border.map(entity_edges).unwrap_or_default();
    if meet == Meet::Border && edges.is_empty() {
        return Err(ApplyError::NoBorder);
    }
    let meet_at = |l: &Link| -> Vec2 {
        let (s, t) = (l.from, l.to);
        let mid = Vec2::new((s.x + t.x) / 2.0, (s.y + t.y) / 2.0);
        match meet {
            Meet::Adjacent => t,
            Meet::Middle => mid,
            Meet::Border => to_border(&edges, mid).map_or(mid, |(p, _)| p),
        }
    };
    // Members as (0 source | 1 adjacent, index).
    let mut live: Vec<usize> = (0..links.len()).collect();
    loop {
        let mut targets: BTreeMap<(u8, usize), [Option<Vec2>; 2]> = BTreeMap::new();
        for &li in &live {
            let l = &links[li];
            let m = meet_at(l);
            targets.entry((0, l.source)).or_default()[l.source_end.index()] = Some(m);
            if meet != Meet::Adjacent {
                targets.entry((1, l.adjacent)).or_default()[l.adjacent_end.index()] = Some(m);
            }
        }
        let mut out = Applied {
            sources: vec![None; sources.len()],
            adjacent: vec![None; adjacent.len()],
            refused: Vec::new(),
        };
        let mut collapsed = BTreeSet::new();
        for (key, ends) in targets {
            let member = if key.0 == 0 {
                sources.get(key.1)
            } else {
                adjacent.get(key.1)
            };
            match member.and_then(|m| put(m, ends, method)) {
                Some(r) => {
                    let slot = if key.0 == 0 {
                        &mut out.sources[key.1]
                    } else {
                        &mut out.adjacent[key.1]
                    };
                    *slot = Some(r);
                }
                None => {
                    collapsed.insert(key);
                }
            }
        }
        if collapsed.is_empty() {
            out.refused = (0..links.len()).filter(|i| !live.contains(i)).collect();
            return Ok(out);
        }
        live.retain(|&li| {
            !collapsed.contains(&(0, links[li].source))
                && !collapsed.contains(&(1, links[li].adjacent))
        });
    }
}

/// What both platforms read from `edgematchApply`.
pub struct ApplyAnswer(pub Result<Applied, ApplyError>);

impl ToJson for ApplyAnswer {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(a) => {
                let shapes = |list: &[Option<Put>]| {
                    list.iter()
                        .map(|r| r.as_ref().map(|(s, _)| s.clone()))
                        .collect::<Vec<_>>()
                };
                let zs = |list: &[Option<Put>]| {
                    list.iter()
                        .map(|r| r.as_ref().map(|(_, z)| z.clone()))
                        .collect::<Vec<_>>()
                };
                field(out, &mut first, "sources", &shapes(&a.sources));
                field(out, &mut first, "sourceZs", &zs(&a.sources));
                field(out, &mut first, "adjacent", &shapes(&a.adjacent));
                field(out, &mut first, "adjacentZs", &zs(&a.adjacent));
                let refused: Vec<f64> = a.refused.iter().map(|&i| i as f64).collect();
                field(out, &mut first, "refused", &refused);
            }
            Err(ApplyError::NoBorder) => field(out, &mut first, "error", "no_border"),
        }
        out.push('}');
    }
}

pub(crate) static OPS: &[Op] = &[
    op!(
        "edgematchLinks",
        |sources: Vec<Member>, adjacent: Vec<Member>, settings: Settings| {
            links(&sources, &adjacent, &settings)
        }
    ),
    op!(
        "edgematchApply",
        |sources: Vec<Member>,
         adjacent: Vec<Member>,
         links: Vec<Link>,
         meet: Meet,
         method: Method,
         border: Option<Shape>| {
            ApplyAnswer(apply(
                &sources,
                &adjacent,
                &links,
                meet,
                method,
                border.as_ref(),
            ))
        }
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::Json;

    fn near(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// Two shapes the same within `metres`: their kind, their points, their
    /// bulges (within 1e-12).
    fn same_shape(a: &Shape, b: &Shape, metres: f64) -> bool {
        let close = |p: &Vec2, q: &Vec2| dist(*p, *q) <= metres;
        match (a, b) {
            (Shape::Line { a: a1, b: b1 }, Shape::Line { a: a2, b: b2 }) => {
                close(a1, a2) && close(b1, b2)
            }
            (
                Shape::Polyline {
                    pts: p1,
                    bulges: g1,
                    ..
                },
                Shape::Polyline {
                    pts: p2,
                    bulges: g2,
                    ..
                },
            ) => {
                let (g1, g2) = (
                    g1.clone().unwrap_or_default(),
                    g2.clone().unwrap_or_default(),
                );
                p1.len() == p2.len()
                    && p1.iter().zip(p2).all(|(p, q)| close(p, q))
                    && g1.len() == g2.len()
                    && g1.iter().zip(&g2).all(|(x, y)| near(*x, *y, 1e-12))
            }
            _ => false,
        }
    }

    /// The reference's cases (scripts/fixtures/edgematch_cases.py): the
    /// links, the unmatched ends, the junctions and the others; every
    /// writing's shapes, elevations and refused links.
    #[test]
    fn every_case_is_matched_as_the_reference_matches_it() {
        let file = Json::parse(include_str!(
            "../../../../../fixtures/fit/v1/edgematch.json"
        ))
        .expect("edgematch.json reads");
        let tolerance = file.get("tolerance");
        let metres = f64::from_json(tolerance.get("metres")).expect("metres");
        let degrees = f64::from_json(tolerance.get("degrees")).expect("degrees");
        let score = f64::from_json(tolerance.get("score")).expect("score");
        let Json::Arr(cases) = file.get("cases") else {
            panic!("cases")
        };
        assert!(cases.len() >= 10, "{} cases", cases.len());
        let mut off = Vec::new();
        for case in cases {
            let name = String::from_json(case.get("name")).unwrap_or_default();
            let sources = Vec::<Member>::from_json(case.get("sources")).expect("sources");
            let adjacent = Vec::<Member>::from_json(case.get("adjacent")).expect("adjacent");
            let settings = Settings::from_json(case.get("settings")).expect("settings");
            let expected = case.get("expected");
            let found = links(&sources, &adjacent, &settings);
            let Json::Arr(want) = expected.get("links") else {
                panic!("links")
            };
            if want.len() != found.links.len() {
                off.push(format!(
                    "{name}: {} links ≠ {}",
                    found.links.len(),
                    want.len()
                ));
                continue;
            }
            for (k, (got, w)) in found.links.iter().zip(want).enumerate() {
                let same = got.source == usize::from_json(w.get("source")).expect("source")
                    && got.source_end == End::from_json(w.get("sourceEnd")).expect("end")
                    && got.adjacent == usize::from_json(w.get("adjacent")).expect("adjacent")
                    && got.adjacent_end == End::from_json(w.get("adjacentEnd")).expect("end")
                    && near(got.gap, f64::from_json(w.get("gap")).expect("gap"), metres)
                    && near(
                        got.angle,
                        f64::from_json(w.get("angle")).expect("angle"),
                        degrees,
                    )
                    && near(
                        got.score,
                        f64::from_json(w.get("score")).expect("score"),
                        score,
                    );
                if !same {
                    off.push(format!("{name}: link {k}: {got:?} ≠ {w:?}"));
                }
            }
            let Json::Arr(unmatched) = expected.get("unmatched") else {
                panic!("unmatched")
            };
            let want_unmatched: Vec<(usize, End)> = unmatched
                .iter()
                .map(|u| {
                    (
                        usize::from_json(u.get("source")).expect("source"),
                        End::from_json(u.get("end")).expect("end"),
                    )
                })
                .collect();
            if found.unmatched != want_unmatched {
                off.push(format!(
                    "{name}: unmatched {:?} ≠ {want_unmatched:?}",
                    found.unmatched
                ));
            }
            for (what, got) in [("junctions", found.junctions), ("others", found.others)] {
                let want = usize::from_json(expected.get(what)).expect(what);
                if got != want {
                    off.push(format!("{name}: {what} {got} ≠ {want}"));
                }
            }
            let Json::Arr(applies) = case.get("apply") else {
                panic!("apply")
            };
            for a in applies {
                let meet = Meet::from_json(a.get("meet")).expect("meet");
                let method = Method::from_json(a.get("method")).expect("method");
                let used = Vec::<usize>::from_json(a.get("use")).expect("use");
                let chosen: Vec<Link> = used.iter().map(|&i| found.links[i].clone()).collect();
                let label = format!("{name} ({meet:?}, {method:?})");
                let Ok(got) = apply(
                    &sources,
                    &adjacent,
                    &chosen,
                    meet,
                    method,
                    settings.border.as_ref(),
                ) else {
                    off.push(format!("{label}: no border"));
                    continue;
                };
                let want = a.get("expected");
                for (side, results, shapes, zs) in [
                    ("source", &got.sources, "sources", "sourceZs"),
                    ("adjacent", &got.adjacent, "adjacent", "adjacentZs"),
                ] {
                    let want_shapes =
                        Vec::<Option<Shape>>::from_json(want.get(shapes)).expect("shapes");
                    let want_zs = Vec::<Option<Elevations>>::from_json(want.get(zs)).expect("zs");
                    for (i, r) in results.iter().enumerate() {
                        match (r, &want_shapes[i]) {
                            (None, None) => {}
                            (Some((s, z)), Some(w)) => {
                                if !same_shape(s, w, metres) {
                                    off.push(format!("{label}: {side} {i}: {s:?} ≠ {w:?}"));
                                }
                                if Some(z) != want_zs[i].as_ref() {
                                    off.push(format!(
                                        "{label}: {side} {i} elevations {z:?} ≠ {:?}",
                                        want_zs[i]
                                    ));
                                }
                            }
                            (r, w) => off.push(format!("{label}: {side} {i}: {r:?} ≠ {w:?}")),
                        }
                    }
                }
                let refused: Vec<usize> = got.refused.iter().map(|&k| used[k]).collect();
                let want_refused = Vec::<usize>::from_json(want.get("refused")).expect("refused");
                if refused != want_refused {
                    off.push(format!("{label}: refused {refused:?} ≠ {want_refused:?}"));
                }
            }
        }
        assert!(off.is_empty(), "{}", off.join("\n"));
    }

    /// Sınırda asks for a border.
    #[test]
    fn meeting_on_the_border_needs_one() {
        let m = |a: Vec2, b: Vec2| Member {
            shape: Shape::Line { a, b },
            zs: vec![vec![None, None]],
            key: None,
        };
        let src = [m(Vec2::new(0.0, 0.0), Vec2::new(9.9, 0.0))];
        let adj = [m(Vec2::new(10.1, 0.0), Vec2::new(20.0, 0.0))];
        let s = Settings {
            distance: 0.5,
            angle: 30.0,
            border: None,
            keyed: false,
        };
        let found = links(&src, &adj, &s);
        assert_eq!(found.links.len(), 1);
        assert_eq!(
            apply(&src, &adj, &found.links, Meet::Border, Method::Move, None),
            Err(ApplyError::NoBorder)
        );
    }
}
