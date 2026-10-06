//! Veri karşılaştır (docs/adr/0179): two sets of objects, old and new,
//! paired by location or by a key attribute and compared, geometry and
//! attributes. One rule for both platforms (the web calls `dataCompare`
//! through WASM); the independent reference is
//! scripts/fixtures/compare_cases.py, whose cases
//! (fixtures/compare/v1/cases.json) both run.
//!
//! - An object is taken apart into its defining points, lengths and other
//!   values. Points come as fixed lists (a point's and its parts', a circle's
//!   centre, an arc's centre and ends, a text's or an insert's place), as
//!   paths (a line, a polyline and its parts, a leader, an open spline:
//!   direction free) and as rings (an area's ring, holes and parts, a
//!   hatch's, a closed spline's: start and direction free). A path or ring
//!   has its vertices and the middle of each edge: the arc's middle on a
//!   bulged edge, the chord's on a straight one.
//! - The location difference of two objects of the same kind and structure
//!   is the largest distance between their matching points at each path's
//!   and ring's best alignment; with another structure, the distance between
//!   the centres of the boxes around their points. Objects of other kinds
//!   have none.
//! - The geometry is the same when the kind and structure are, the location
//!   difference is within the tolerance, the lengths differ by no more than
//!   the tolerance and the other values are equal.
//! - By location: candidates of the same kind within the search distance,
//!   taken from the smallest difference up (ties: the old object's order,
//!   then the new one's), each object once. By key: the same trimmed value of
//!   the key attribute; one that is empty, or repeated on either side, makes
//!   its objects key problems.
//! - The attributes compared are both objects' fields in the order of their
//!   characters' codes, less the ignored ones and, by key, the key field; a
//!   missing field is empty.

use std::collections::{HashMap, HashSet};

use crate::api::Op;
use crate::api::json::{FromJson, Json, ToJson, field};
use crate::entity::Shape;
use crate::jsmath::{cos, js_max, js_min, sin};
use crate::op;
use crate::vec2::Vec2;

/// An object's attributes, in their order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attrs(pub Vec<(String, String)>);

impl Attrs {
    fn get(&self, name: &str) -> &str {
        self.0
            .iter()
            .find(|(k, _)| k == name)
            .map_or("", |(_, v)| v.as_str())
    }

    fn has(&self, name: &str) -> bool {
        self.0.iter().any(|(k, _)| k == name)
    }
}

impl FromJson for Attrs {
    fn from_json(v: &Json) -> Result<Attrs, String> {
        match v {
            Json::Obj(pairs) => pairs
                .iter()
                .map(|(k, v)| match v {
                    Json::Str(s) => Ok((k.clone(), s.clone())),
                    _ => Err(format!("“{k}” özniteliği metin olmalı")),
                })
                .collect::<Result<Vec<_>, String>>()
                .map(Attrs),
            Json::Null => Ok(Attrs::default()),
            _ => Err("öznitelikler nesne olmalı".into()),
        }
    }
}

impl ToJson for Attrs {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        for (k, v) in &self.0 {
            field(out, &mut first, k, v);
        }
        out.push('}');
    }
}

/// An object of a set: its shape and its attributes.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub shape: Shape,
    pub attrs: Attrs,
}

crate::json_struct!(Member { shape, attrs });

/// How the objects are paired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Matching {
    Location,
    Key,
}

impl FromJson for Matching {
    fn from_json(v: &Json) -> Result<Matching, String> {
        match v {
            Json::Str(s) if s == "location" => Ok(Matching::Location),
            Json::Str(s) if s == "key" => Ok(Matching::Key),
            _ => Err("eşleme “location” ya da “key” olmalı".into()),
        }
    }
}

impl ToJson for Matching {
    fn write_json(&self, out: &mut String) {
        match self {
            Matching::Location => "location",
            Matching::Key => "key",
        }
        .write_json(out);
    }
}

/// What the comparison is made with: the pairing, the key field (by key),
/// the search distance and the tolerance (metres), the fields left out.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub matching: Matching,
    pub key: Option<String>,
    pub search: f64,
    pub tolerance: f64,
    pub ignore: Option<Vec<String>>,
}

crate::json_struct!(Settings {
    matching => "match",
    key,
    search,
    tolerance,
    ignore
});

/// A row's finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Same,
    Geometry,
    Attributes,
    Both,
    Added,
    Removed,
    Key,
}

impl ToJson for Status {
    fn write_json(&self, out: &mut String) {
        match self {
            Status::Same => "same",
            Status::Geometry => "geometry",
            Status::Attributes => "attributes",
            Status::Both => "both",
            Status::Added => "added",
            Status::Removed => "removed",
            Status::Key => "key",
        }
        .write_json(out);
    }
}

/// A row: the finding, the old and the new object (indices), the location
/// difference (m) of a pair whose kinds agree, the changed fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub status: Status,
    pub old: Option<usize>,
    pub new: Option<usize>,
    pub distance: Option<f64>,
    pub fields: Vec<String>,
}

impl ToJson for Row {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "status", &self.status);
        field(out, &mut first, "old", &self.old.map(|i| i as f64));
        field(out, &mut first, "new", &self.new.map(|i| i as f64));
        field(out, &mut first, "distance", &self.distance);
        field(out, &mut first, "fields", &self.fields);
        out.push('}');
    }
}

/// A component of an object's points.
enum Comp {
    /// Points matched in their order.
    Fixed(Vec<Vec2>),
    /// An open path's vertices and edge middles: either direction.
    Path(Vec<Vec2>, Vec<Vec2>),
    /// A ring's vertices and edge middles: any start, either direction.
    Ring(Vec<Vec2>, Vec<Vec2>),
}

/// A value compared exactly.
#[derive(Debug, PartialEq)]
enum Other {
    Num(f64),
    Text(String),
    Flag(bool),
    Nothing,
}

fn num(v: Option<f64>) -> Other {
    v.map_or(Other::Nothing, Other::Num)
}

fn text(v: Option<&str>) -> Other {
    v.map_or(Other::Nothing, |s| Other::Text(s.to_owned()))
}

/// An object taken apart.
struct Defs {
    comps: Vec<Comp>,
    lengths: Vec<f64>,
    others: Vec<Other>,
}

/// An edge's middle: the chord's, moved by the sagitta to the right of a→b
/// for a counter-clockwise (positive) bulge.
fn middle(a: Vec2, b: Vec2, bulge: f64) -> Vec2 {
    let m = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    if bulge == 0.0 {
        return m;
    }
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let k = bulge / 2.0;
    Vec2::new(m.x + dy * k, m.y - dx * k)
}

fn middles(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> Vec<Vec2> {
    let n = pts.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    (0..count)
        .map(|i| {
            let b = bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0);
            middle(pts[i], pts[(i + 1) % n], b)
        })
        .collect()
}

fn path(pts: &[Vec2], bulges: Option<&[f64]>) -> Comp {
    Comp::Path(pts.to_vec(), middles(pts, bulges, false))
}

fn ring(pts: &[Vec2], bulges: Option<&[f64]>) -> Comp {
    Comp::Ring(pts.to_vec(), middles(pts, bulges, true))
}

fn defs(shape: &Shape) -> Defs {
    let mut comps = Vec::new();
    let mut lengths = Vec::new();
    let mut others = Vec::new();
    match shape {
        Shape::Point { p, parts, .. } => {
            let mut pts = vec![*p];
            pts.extend(parts.iter().flatten().map(|q| q.p));
            comps.push(Comp::Fixed(pts));
        }
        Shape::Line { a, b } => comps.push(path(&[*a, *b], None)),
        Shape::Polyline {
            pts, bulges, parts, ..
        } => {
            comps.push(path(pts, bulges.as_deref()));
            for q in parts.iter().flatten() {
                comps.push(path(&q.pts, q.bulges.as_deref()));
            }
        }
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            comps.push(ring(pts, bulges.as_deref()));
            for h in holes.iter().flatten() {
                comps.push(ring(&h.pts, h.bulges.as_deref()));
            }
            for q in parts.iter().flatten() {
                comps.push(ring(&q.pts, q.bulges.as_deref()));
                for h in q.holes.iter().flatten() {
                    comps.push(ring(&h.pts, h.bulges.as_deref()));
                }
            }
        }
        Shape::Circle { c, r } => {
            comps.push(Comp::Fixed(vec![*c]));
            lengths.push(*r);
        }
        Shape::Arc { c, r, a0, a1 } => {
            let at = |a: f64| Vec2::new(c.x + r * cos(a), c.y + r * sin(a));
            comps.push(Comp::Fixed(vec![*c, at(*a0), at(*a1)]));
            lengths.push(*r);
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            comps.push(Comp::Fixed(vec![
                *c,
                Vec2::new(c.x + major.x, c.y + major.y),
            ]));
            others.extend([Other::Num(*ratio), Other::Num(*t0), Other::Num(*t1)]);
        }
        Shape::Spline { pts, closed } => {
            comps.push(if *closed {
                ring(pts, None)
            } else {
                path(pts, None)
            });
            others.push(Other::Flag(*closed));
        }
        Shape::Xline { p, dir } | Shape::Ray { p, dir } => {
            comps.push(Comp::Fixed(vec![*p]));
            others.extend([Other::Num(dir.x), Other::Num(dir.y)]);
        }
        Shape::Text {
            p,
            text: words,
            height,
            rotation,
            align,
            width_factor,
            ..
        } => {
            comps.push(Comp::Fixed(vec![*p]));
            lengths.push(*height);
            others.extend([
                Other::Text(words.clone()),
                Other::Num(*rotation),
                text(align.map(|a| a.name())),
                // A width factor of 1 is no factor, as a file writes it.
                num(width_factor.filter(|w| *w != 1.0)),
            ]);
        }
        Shape::Dimension {
            a,
            b,
            offset,
            height,
            text: words,
            style,
            angle,
            c,
            ..
        } => {
            let mut pts = vec![*a, *b];
            pts.extend(*c);
            comps.push(Comp::Fixed(pts));
            lengths.extend([*offset, *height]);
            others.extend([text(words.as_deref()), text(style.as_deref()), num(*angle)]);
        }
        // What its region follows is not its geometry (docs/adr/0186 §6).
        Shape::Hatch {
            ring: outer,
            holes,
            pattern,
            ..
        } => {
            comps.push(ring(outer, None));
            for h in holes.iter().flatten() {
                comps.push(ring(h, None));
            }
            others.extend([
                Other::Text(pattern.kind.clone()),
                Other::Num(pattern.angle),
                Other::Num(pattern.spacing),
                text(pattern.name.as_deref()),
                num(pattern.scale),
            ]);
            if let Some(lines) = &pattern.lines {
                others.push(Other::Text(crate::api::json::to_string(lines)));
            }
            if let Some(g) = &pattern.gradient {
                others.push(Other::Text(crate::api::json::to_string(g)));
            }
        }
        Shape::Insert {
            block,
            p,
            scale,
            rotation,
            mirror,
            ..
        } => {
            comps.push(Comp::Fixed(vec![*p]));
            others.extend([
                Other::Text(block.clone()),
                Other::Num(*scale),
                Other::Num(*rotation),
                // Not mirrored is no `mirror`, as a file writes it.
                if *mirror == Some(true) {
                    Other::Flag(true)
                } else {
                    Other::Nothing
                },
            ]);
        }
        Shape::Leader {
            pts,
            text: words,
            height,
            rotation,
            arrow,
            ..
        } => {
            comps.push(path(pts, None));
            lengths.push(*height);
            others.extend([
                text(words.as_deref()),
                Other::Num(*rotation),
                text(arrow.as_deref()),
            ]);
        }
        // Its corner, sizes, words and look (docs/adr/0184).
        Shape::Table {
            p,
            rotation,
            height,
            rows,
            columns,
            cells,
            merges,
            aligns,
            header,
            grid,
            ..
        } => {
            comps.push(Comp::Fixed(vec![*p]));
            lengths.push(*height);
            lengths.extend(rows.iter().chain(columns));
            others.push(Other::Num(*rotation));
            others.extend(cells.iter().flatten().map(|w| Other::Text(w.clone())));
            for m in merges.iter().flatten() {
                others.extend([m.row, m.col, m.rows, m.cols].map(|x| Other::Num(x as f64)));
            }
            others.extend(aligns.iter().flatten().map(|a| Other::Text(a.clone())));
            others.extend([Other::Flag(*header == Some(true)), text(grid.as_deref())]);
        }
    }
    Defs {
        comps,
        lengths,
        others,
    }
}

fn d2(a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (a.x - b.x, a.y - b.y);
    dx * dx + dy * dy
}

/// A component pair's largest squared distance at its best alignment, or
/// none when their shapes differ.
fn best(a: &Comp, b: &Comp) -> Option<f64> {
    let worst = |pa: &[Vec2],
                 ma: &[Vec2],
                 pb: &[Vec2],
                 mb: &[Vec2],
                 v: &dyn Fn(usize) -> usize,
                 e: &dyn Fn(usize) -> usize,
                 bound: f64| {
        let mut m: f64 = 0.0;
        for i in 0..pa.len() {
            m = js_max(m, d2(pa[i], pb[v(i)]));
            if m >= bound {
                return m;
            }
        }
        for i in 0..ma.len() {
            m = js_max(m, d2(ma[i], mb[e(i)]));
            if m >= bound {
                return m;
            }
        }
        m
    };
    match (a, b) {
        (Comp::Fixed(pa), Comp::Fixed(pb)) if pa.len() == pb.len() => {
            Some(pa.iter().zip(pb).map(|(p, q)| d2(*p, *q)).fold(0.0, js_max))
        }
        (Comp::Path(pa, ma), Comp::Path(pb, mb))
            if pa.len() == pb.len() && ma.len() == mb.len() =>
        {
            let (n, k) = (pa.len(), ma.len());
            let forward = worst(pa, ma, pb, mb, &|i| i, &|i| i, f64::INFINITY);
            let back = worst(pa, ma, pb, mb, &|i| n - 1 - i, &|i| k - 1 - i, forward);
            Some(js_min(forward, back))
        }
        (Comp::Ring(pa, ma), Comp::Ring(pb, mb))
            if pa.len() == pb.len() && ma.len() == mb.len() =>
        {
            let n = pa.len();
            let mut least = f64::INFINITY;
            for s in 0..n {
                let f = worst(pa, ma, pb, mb, &|i| (s + i) % n, &|i| (s + i) % n, least);
                least = js_min(least, f);
                // Backwards: vertex i to s − i, edge i (i → i + 1) to the edge from s − i − 1 to s − i.
                let r = worst(
                    pa,
                    ma,
                    pb,
                    mb,
                    &|i| (s + n - i % n) % n,
                    &|i| (s + 2 * n - i - 1) % n,
                    least,
                );
                least = js_min(least, r);
            }
            Some(if n == 0 { 0.0 } else { least })
        }
        _ => None,
    }
}

/// Every point of an object: its vertices and edge middles.
fn points(d: &Defs) -> impl Iterator<Item = Vec2> + '_ {
    const NONE: &[Vec2] = &[];
    d.comps
        .iter()
        .flat_map(|c| match c {
            Comp::Fixed(p) => p.iter().chain(NONE.iter()),
            Comp::Path(p, m) | Comp::Ring(p, m) => p.iter().chain(m.iter()),
        })
        .copied()
}

/// The centre of the box around an object's points.
fn centre(d: &Defs) -> Vec2 {
    let (mut lo, mut hi) = (
        Vec2::new(f64::INFINITY, f64::INFINITY),
        Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in points(d) {
        lo = Vec2::new(js_min(lo.x, p.x), js_min(lo.y, p.y));
        hi = Vec2::new(js_max(hi.x, p.x), js_max(hi.y, p.y));
    }
    Vec2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0)
}

fn same_kind(a: &Shape, b: &Shape) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

/// The squared location difference of two objects and whether their
/// structure is the same; none for objects of other kinds.
fn difference(a: &Shape, da: &Defs, b: &Shape, db: &Defs) -> Option<(f64, bool)> {
    if !same_kind(a, b) {
        return None;
    }
    if da.comps.len() == db.comps.len() {
        let mut most: f64 = 0.0;
        let mut whole = true;
        for (x, y) in da.comps.iter().zip(&db.comps) {
            match best(x, y) {
                Some(v) => most = js_max(most, v),
                None => {
                    whole = false;
                    break;
                }
            }
        }
        if whole {
            return Some((most, true));
        }
    }
    Some((d2(centre(da), centre(db)), false))
}

fn same_geometry(a: &Shape, da: &Defs, b: &Shape, db: &Defs, tol: f64) -> bool {
    match difference(a, da, b, db) {
        Some((d, true)) if d <= tol * tol => {
            da.lengths.len() == db.lengths.len()
                && da
                    .lengths
                    .iter()
                    .zip(&db.lengths)
                    .all(|(x, y)| (x - y).abs() <= tol)
                && da.others == db.others
        }
        _ => false,
    }
}

/// The fields whose values differ, in the order of their characters' codes:
/// a drawing keeps no order of its own (the desktop's are sorted, a file's
/// may be).
fn changed_fields(a: &Attrs, b: &Attrs, ignore: &HashSet<&str>) -> Vec<String> {
    let mut names: Vec<&str> =
        a.0.iter()
            .map(|(k, _)| k.as_str())
            .chain(b.0.iter().map(|(k, _)| k.as_str()).filter(|k| !a.has(k)))
            .collect();
    names.sort_unstable();
    names
        .into_iter()
        .filter(|k| !ignore.contains(k) && a.get(k) != b.get(k))
        .map(str::to_owned)
        .collect()
}

/// The box around an object's points, grown by `by`.
fn grown(d: &Defs, by: f64) -> (f64, f64, f64, f64) {
    let mut b = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in points(d) {
        b = (
            js_min(b.0, p.x),
            js_min(b.1, p.y),
            js_max(b.2, p.x),
            js_max(b.3, p.y),
        );
    }
    (b.0 - by, b.1 - by, b.2 + by, b.3 + by)
}

/// Pairs by location: new index → old index.
fn by_location(
    old: &[Member],
    od: &[Defs],
    new: &[Member],
    nd: &[Defs],
    search: f64,
) -> HashMap<usize, usize> {
    // Candidates only where the grown boxes overlap (a difference within the search distance needs it).
    let boxes: Vec<_> = od.iter().map(|d| grown(d, search)).collect();
    let mut order: Vec<usize> = (0..old.len()).collect();
    order.sort_by(|&i, &j| boxes[i].0.total_cmp(&boxes[j].0));
    let mut cands: Vec<(f64, usize, usize)> = Vec::new();
    for (j, b) in new.iter().enumerate() {
        let nb = grown(&nd[j], 0.0);
        for &i in &order {
            let ob = boxes[i];
            if ob.0 > nb.2 {
                break;
            }
            if ob.2 < nb.0 || ob.1 > nb.3 || ob.3 < nb.1 {
                continue;
            }
            if let Some((d, _)) = difference(&old[i].shape, &od[i], &b.shape, &nd[j])
                && d <= search * search
            {
                cands.push((d, i, j));
            }
        }
    }
    cands.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut pairs = HashMap::new();
    let mut taken = HashSet::new();
    for (_, i, j) in cands {
        if !taken.contains(&i) && !pairs.contains_key(&j) {
            pairs.insert(j, i);
            taken.insert(i);
        }
    }
    pairs
}

/// The comparison's rows: the new set's order (its pair, added or a key
/// problem), then the old set's unpaired objects in theirs.
pub fn compare(old: &[Member], new: &[Member], s: &Settings) -> Vec<Row> {
    let od: Vec<Defs> = old.iter().map(|m| defs(&m.shape)).collect();
    let nd: Vec<Defs> = new.iter().map(|m| defs(&m.shape)).collect();
    let mut problems_old = HashSet::new();
    let mut problems_new = HashSet::new();
    let key = s.key.as_deref().unwrap_or("");
    let pairs = match s.matching {
        Matching::Location => by_location(old, &od, new, &nd, s.search),
        Matching::Key => {
            let keys = |side: &[Member]| {
                let mut out: HashMap<String, Vec<usize>> = HashMap::new();
                for (i, m) in side.iter().enumerate() {
                    out.entry(m.attrs.get(key).trim().to_owned())
                        .or_default()
                        .push(i);
                }
                out
            };
            let (ko, kn) = (keys(old), keys(new));
            let bad = |k: &str| {
                k.is_empty()
                    || ko.get(k).is_some_and(|v| v.len() > 1)
                    || kn.get(k).is_some_and(|v| v.len() > 1)
            };
            let mut pairs = HashMap::new();
            for (k, idx) in &ko {
                if bad(k) {
                    problems_old.extend(idx.iter().copied());
                }
            }
            for (k, idx) in &kn {
                if bad(k) {
                    problems_new.extend(idx.iter().copied());
                } else if let Some(o) = ko.get(k) {
                    pairs.insert(idx[0], o[0]);
                }
            }
            pairs
        }
    };
    let mut ignore: HashSet<&str> = s.ignore.iter().flatten().map(String::as_str).collect();
    if s.matching == Matching::Key {
        ignore.insert(key);
    }
    let paired_old: HashSet<usize> = pairs.values().copied().collect();
    let mut rows = Vec::new();
    for (j, b) in new.iter().enumerate() {
        if problems_new.contains(&j) {
            rows.push(Row {
                status: Status::Key,
                old: None,
                new: Some(j),
                distance: None,
                fields: Vec::new(),
            });
        } else if let Some(&i) = pairs.get(&j) {
            let a = &old[i];
            let diff = difference(&a.shape, &od[i], &b.shape, &nd[j]);
            let geometry = !same_geometry(&a.shape, &od[i], &b.shape, &nd[j], s.tolerance);
            let fields = changed_fields(&a.attrs, &b.attrs, &ignore);
            let status = match (geometry, !fields.is_empty()) {
                (false, false) => Status::Same,
                (true, false) => Status::Geometry,
                (false, true) => Status::Attributes,
                (true, true) => Status::Both,
            };
            rows.push(Row {
                status,
                old: Some(i),
                new: Some(j),
                distance: diff.map(|(d, _)| d.sqrt()),
                fields,
            });
        } else {
            rows.push(Row {
                status: Status::Added,
                old: None,
                new: Some(j),
                distance: None,
                fields: Vec::new(),
            });
        }
    }
    for i in 0..old.len() {
        if problems_old.contains(&i) {
            rows.push(Row {
                status: Status::Key,
                old: Some(i),
                new: None,
                distance: None,
                fields: Vec::new(),
            });
        } else if !paired_old.contains(&i) {
            rows.push(Row {
                status: Status::Removed,
                old: Some(i),
                new: None,
                distance: None,
                fields: Vec::new(),
            });
        }
    }
    rows
}

pub(crate) static OPS: &[Op] = &[op!(
    "dataCompare",
    |old: Vec<Member>, new: Vec<Member>, settings: Settings| compare(&old, &new, &settings)
)];

#[cfg(test)]
mod tests {
    use super::*;

    /// The shared cases (fixtures/compare/v1/cases.json), written by
    /// scripts/fixtures/compare_cases.py from docs/adr/0179, not from this
    /// code; the web runs them through WASM (model/ops/compare.test.ts).
    #[test]
    fn compares_as_the_shared_cases_say() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/compare/v1/cases.json"
        );
        let text = std::fs::read_to_string(path).expect("the cases");
        let file = Json::parse(&text).expect("JSON");
        let Json::Arr(cases) = file.get("cases") else {
            panic!("cases")
        };
        assert!(cases.len() >= 5);
        for c in cases {
            let name = match c.get("name") {
                Json::Str(s) => s.clone(),
                _ => String::new(),
            };
            let old = Vec::<Member>::from_json(c.get("old")).expect("old members");
            let new = Vec::<Member>::from_json(c.get("new")).expect("new members");
            let settings = Settings::from_json(c.get("settings")).expect("settings");
            let rows = compare(&old, &new, &settings);
            let Json::Arr(want) = c.get("rows") else {
                panic!("rows")
            };
            assert_eq!(rows.len(), want.len(), "{name}");
            for (got, w) in rows.iter().zip(want) {
                let mut status = String::new();
                got.status.write_json(&mut status);
                assert_eq!(
                    &Json::Str(status.trim_matches('"').to_owned()),
                    w.get("status"),
                    "{name}"
                );
                let index = |v: &Json| match v {
                    Json::Num(n) => Some(*n as usize),
                    _ => None,
                };
                assert_eq!(got.old, index(w.get("old")), "{name}");
                assert_eq!(got.new, index(w.get("new")), "{name}");
                match (got.distance, w.get("distance")) {
                    (Some(d), Json::Num(e)) => assert!((d - e).abs() <= 1e-9, "{name}: {d} {e}"),
                    (None, Json::Null) => {}
                    other => panic!("{name}: {other:?}"),
                }
                let fields = Vec::<String>::from_json(w.get("fields")).expect("fields");
                assert_eq!(got.fields, fields, "{name}");
            }
        }
    }
}
