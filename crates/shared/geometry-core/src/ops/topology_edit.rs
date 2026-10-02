//! Topological editing (docs/adr/0160), one for both platforms (the web
//! through WASM): the neighbours of an edited vertex or edge put right with
//! it, so that shared corners and edges stay shared. Nothing is computed:
//! vertices take the changes' points, edges their bulges, so both platforms
//! give the same bits. The independent reference is
//! `scripts/fixtures/topology_edit_cases.py`.

use crate::api::Op;
use crate::api::json::{FromJson, Json, ToJson, field, read_field};
use crate::entity::{Part, Shape};
use crate::geom::arrangement::Ring;
use crate::geometry::dist;
use crate::op;
use crate::vec2::Vec2;

/// Two places this close are one (the elevations' “at a vertex”, docs/adr/0142).
pub const SAME: f64 = 1e-6;

/// Two bulges this close are the same arc's.
const BULGE: f64 = 1e-9;

/// A bulge this small is a straight edge (`geom::bulge`).
const STRAIGHT: f64 = 1e-12;

/// An object near the edited one: its shape and whether its layer is locked.
#[derive(Clone, Debug, PartialEq)]
pub struct Neighbour {
    pub shape: Shape,
    pub locked: bool,
}

impl FromJson for Neighbour {
    fn from_json(v: &Json) -> Result<Neighbour, String> {
        Ok(Neighbour {
            shape: read_field(v, "shape")?,
            locked: read_field::<Option<bool>>(v, "locked")?.unwrap_or(false),
        })
    }
}

/// What the edit did at shared places: the moves happen together, the other
/// changes one after the other.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// A vertex moved from `at` to `to`.
    Move { at: Vec2, to: Vec2 },
    /// `p` added on the straight edge from `a` to `b`.
    Insert { a: Vec2, b: Vec2, p: Vec2 },
    /// The edge from `a` to `b` reshaped: its bulge `from` became `to`.
    Bulge {
        a: Vec2,
        b: Vec2,
        from: f64,
        to: f64,
    },
    /// The vertex at `at` removed, its path neighbours `prev` and `next`.
    Remove { at: Vec2, prev: Vec2, next: Vec2 },
}

crate::json_tagged!(Change, "kind",
    Move => "move" { at, to },
    Insert => "insert" { a, b, p },
    Bulge => "bulge" { a, b, from, to },
    Remove => "remove" { at, prev, next },
);

/// The neighbours put right (their indices and shapes), how many would have
/// changed but lie on a locked layer, and how many would have been left
/// invalid (too few vertices).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answer {
    pub edited: Vec<(usize, Shape)>,
    pub locked: usize,
    pub invalid: usize,
}

impl ToJson for Answer {
    fn write_json(&self, out: &mut String) {
        out.push_str("{\"edited\":[");
        for (k, (i, shape)) in self.edited.iter().enumerate() {
            if k > 0 {
                out.push(',');
            }
            out.push('{');
            let mut first = true;
            field(out, &mut first, "index", &(*i as f64));
            field(out, &mut first, "shape", shape);
            out.push('}');
        }
        out.push(']');
        let mut first = false;
        field(out, &mut first, "locked", &(self.locked as f64));
        field(out, &mut first, "invalid", &(self.invalid as f64));
        out.push('}');
    }
}

/// A path of a neighbour: a line's two ends, a polyline, or one of an
/// area's rings and holes.
pub(crate) struct Path {
    pub(crate) pts: Vec<Vec2>,
    pub(crate) bulges: Vec<f64>,
    pub(crate) closed: bool,
}

/// How the paths go back into a shape: a line, a polyline (with its holes
/// as they were), or an area's parts with how many holes each has.
pub(crate) enum Plan {
    Line,
    Polyline(Option<Vec<Ring>>),
    Polygon(Vec<usize>),
}

pub(crate) fn padded(bulges: &Option<Vec<f64>>, n: usize) -> Vec<f64> {
    let mut b = bulges.clone().unwrap_or_default();
    b.resize(n, 0.0);
    b
}

/// A neighbour's paths in the elevations' order: an area's outer ring, its
/// holes, then each further part's ring and holes.
pub(crate) fn paths_of(shape: &Shape) -> Option<(Vec<Path>, Plan)> {
    match shape {
        Shape::Line { a, b } => Some((
            vec![Path {
                pts: vec![*a, *b],
                bulges: vec![0.0],
                closed: false,
            }],
            Plan::Line,
        )),
        Shape::Polyline { pts, bulges, holes } => Some((
            vec![Path {
                pts: pts.clone(),
                bulges: padded(bulges, pts.len().saturating_sub(1)),
                closed: false,
            }],
            Plan::Polyline(holes.clone()),
        )),
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let mut out = Vec::new();
            let mut plan = Vec::new();
            let mut ring =
                |pts: &Vec<Vec2>, bulges: &Option<Vec<f64>>, holes: &Option<Vec<Ring>>| {
                    out.push(Path {
                        pts: pts.clone(),
                        bulges: padded(bulges, pts.len()),
                        closed: true,
                    });
                    let holes = holes.as_deref().unwrap_or_default();
                    for h in holes {
                        out.push(Path {
                            pts: h.pts.clone(),
                            bulges: padded(&h.bulges, h.pts.len()),
                            closed: true,
                        });
                    }
                    plan.push(holes.len());
                };
            ring(pts, bulges, holes);
            for part in parts.iter().flatten() {
                ring(&part.pts, &part.bulges, &part.holes);
            }
            Some((out, Plan::Polygon(plan)))
        }
        _ => None,
    }
}

pub(crate) fn shape_of(paths: Vec<Path>, plan: Plan) -> Shape {
    let mut paths = paths.into_iter();
    match plan {
        Plan::Line | Plan::Polyline(_) => {
            let Path { pts, bulges, .. } = paths.next().unwrap_or(Path {
                pts: Vec::new(),
                bulges: Vec::new(),
                closed: false,
            });
            match plan {
                Plan::Line if pts.len() == 2 && bulges.iter().all(|&b| b == 0.0) => Shape::Line {
                    a: pts[0],
                    b: pts[1],
                },
                Plan::Polyline(holes) => Shape::Polyline {
                    pts,
                    bulges: Some(bulges),
                    holes,
                },
                _ => Shape::Polyline {
                    pts,
                    bulges: Some(bulges),
                    holes: None,
                },
            }
        }
        Plan::Polygon(holes_per) => {
            let mut rings = Vec::with_capacity(holes_per.len());
            for count in holes_per {
                let ring = paths.next();
                let holes: Vec<Ring> = (0..count)
                    .filter_map(|_| paths.next())
                    .map(|h| Ring {
                        pts: h.pts,
                        bulges: Some(h.bulges),
                    })
                    .collect();
                if let Some(ring) = ring {
                    rings.push((ring, (!holes.is_empty()).then_some(holes)));
                }
            }
            let mut rings = rings.into_iter();
            let Some((first, holes)) = rings.next() else {
                return Shape::Polygon {
                    pts: Vec::new(),
                    bulges: None,
                    holes: None,
                    parts: None,
                };
            };
            let parts: Vec<Part> = rings
                .map(|(r, h)| Part {
                    pts: r.pts,
                    bulges: Some(r.bulges),
                    holes: h,
                })
                .collect();
            Shape::Polygon {
                pts: first.pts,
                bulges: Some(first.bulges),
                holes,
                parts: (!parts.is_empty()).then_some(parts),
            }
        }
    }
}

fn same(p: Vec2, q: Vec2) -> bool {
    dist(p, q) <= SAME
}

/// The neighbours put right by `changes` (docs/adr/0160 §2–§3); points
/// take part only with `points`.
pub fn apply(neighbours: &[Neighbour], changes: &[Change], points: bool) -> Answer {
    let moves: Vec<(Vec2, Vec2)> = changes
        .iter()
        .filter_map(|c| match c {
            Change::Move { at, to } => Some((*at, *to)),
            _ => None,
        })
        .collect();
    let mut answer = Answer::default();
    for (i, n) in neighbours.iter().enumerate() {
        if let Shape::Point { p, z } = &n.shape {
            if !points {
                continue;
            }
            let Some(&(_, to)) = moves.iter().find(|(at, _)| same(*p, *at)) else {
                continue;
            };
            if n.locked {
                answer.locked += 1;
            } else {
                answer.edited.push((i, Shape::Point { p: to, z: *z }));
            }
            continue;
        }
        let Some((mut paths, plan)) = paths_of(&n.shape) else {
            continue;
        };
        let (mut changed, mut bad) = (false, false);
        // The moves together, against the places as they were.
        for path in &mut paths {
            for v in &mut path.pts {
                if let Some(&(_, to)) = moves.iter().find(|(at, _)| same(*v, *at)) {
                    *v = to;
                    changed = true;
                }
            }
        }
        for c in changes {
            for path in &mut paths {
                let len = path.pts.len();
                let count = if path.closed {
                    len
                } else {
                    len.saturating_sub(1)
                };
                match *c {
                    Change::Move { .. } => {}
                    Change::Insert { a, b, p } => {
                        for e in (0..count).rev() {
                            let (u, w) = (path.pts[e], path.pts[(e + 1) % len]);
                            let on = (same(u, a) && same(w, b)) || (same(u, b) && same(w, a));
                            if on && path.bulges[e].abs() <= STRAIGHT {
                                path.pts.insert(e + 1, p);
                                path.bulges.splice(e..=e, [0.0, 0.0]);
                                changed = true;
                            }
                        }
                    }
                    Change::Bulge { a, b, from, to } => {
                        for e in 0..count {
                            let (u, w) = (path.pts[e], path.pts[(e + 1) % len]);
                            if same(u, a) && same(w, b) && (path.bulges[e] - from).abs() <= BULGE {
                                path.bulges[e] = to;
                                changed = true;
                            } else if same(u, b)
                                && same(w, a)
                                && (path.bulges[e] + from).abs() <= BULGE
                            {
                                path.bulges[e] = if to == 0.0 { 0.0 } else { -to };
                                changed = true;
                            }
                        }
                    }
                    Change::Remove { at, prev, next } => {
                        for k in 0..len {
                            if !same(path.pts[k], at) {
                                continue;
                            }
                            let (before, after) = if path.closed {
                                (path.pts[(k + len - 1) % len], path.pts[(k + 1) % len])
                            } else if k == 0 || k == len - 1 {
                                continue;
                            } else {
                                (path.pts[k - 1], path.pts[k + 1])
                            };
                            let chain = (same(before, prev) && same(after, next))
                                || (same(before, next) && same(after, prev));
                            if !chain {
                                continue;
                            }
                            if len - 1 < if path.closed { 3 } else { 2 } {
                                bad = true;
                                break;
                            }
                            let prev_edge = if path.closed {
                                (k + len - 1) % len
                            } else {
                                k - 1
                            };
                            path.bulges[prev_edge] = 0.0;
                            path.pts.remove(k);
                            let gone = if k < path.bulges.len() {
                                k
                            } else {
                                path.bulges.len() - 1
                            };
                            path.bulges.remove(gone);
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }
        if !changed && !bad {
            continue;
        }
        if n.locked {
            answer.locked += 1;
        } else if bad {
            answer.invalid += 1;
        } else {
            answer.edited.push((i, shape_of(paths, plan)));
        }
    }
    answer
}

/// The changes an edit made, from the edited object before and after it
/// (docs/adr/0160 §7), path by path in the elevations' order: with as many
/// vertices, every vertex that changed place is a move and every edge whose
/// ends stayed and whose bulge changed a bulge; with one vertex more, the
/// first that differs is an insert on the edge it splits; with one fewer, a
/// remove with its two path neighbours (neither at an open path's end). The
/// moves come first. A point is a corner of its own (Noktalar da): moved, it
/// is a move of its place.
pub fn changes(before: &Shape, after: &Shape) -> Vec<Change> {
    if let (Shape::Point { p: at, .. }, Shape::Point { p: to, .. }) = (before, after) {
        return if at == to {
            Vec::new()
        } else {
            vec![Change::Move { at: *at, to: *to }]
        };
    }
    let (Some((pb, _)), Some((pa, _))) = (paths_of(before), paths_of(after)) else {
        return Vec::new();
    };
    if pb.len() != pa.len() {
        return Vec::new();
    }
    let (mut moves, mut others) = (Vec::new(), Vec::new());
    for (b, a) in pb.iter().zip(&pa) {
        let (n, m) = (b.pts.len(), a.pts.len());
        if m == n {
            for k in 0..n {
                if b.pts[k] != a.pts[k] {
                    moves.push(Change::Move {
                        at: b.pts[k],
                        to: a.pts[k],
                    });
                }
            }
            let count = if b.closed { n } else { n.saturating_sub(1) };
            for e in 0..count {
                let (u, w) = (b.pts[e], b.pts[(e + 1) % n]);
                if u == a.pts[e] && w == a.pts[(e + 1) % n] && b.bulges[e] != a.bulges[e] {
                    others.push(Change::Bulge {
                        a: u,
                        b: w,
                        from: b.bulges[e],
                        to: a.bulges[e],
                    });
                }
            }
        } else if m == n + 1 {
            let j = (0..n).find(|&k| b.pts[k] != a.pts[k]).unwrap_or(n);
            if b.closed {
                others.push(Change::Insert {
                    a: b.pts[(j + n - 1) % n],
                    b: b.pts[j % n],
                    p: a.pts[j],
                });
            } else if j > 0 && j < n {
                others.push(Change::Insert {
                    a: b.pts[j - 1],
                    b: b.pts[j],
                    p: a.pts[j],
                });
            }
        } else if m + 1 == n {
            let j = (0..m).find(|&k| b.pts[k] != a.pts[k]).unwrap_or(m);
            if b.closed {
                others.push(Change::Remove {
                    at: b.pts[j],
                    prev: b.pts[(j + n - 1) % n],
                    next: b.pts[(j + 1) % n],
                });
            } else if j > 0 && j + 1 < n {
                others.push(Change::Remove {
                    at: b.pts[j],
                    prev: b.pts[j - 1],
                    next: b.pts[j + 1],
                });
            }
        }
    }
    moves.extend(others);
    moves
}

pub(crate) static OPS: &[Op] = &[
    op!("topologyEdit", |neighbours: Vec<Neighbour>,
                         changes: Vec<Change>,
                         points: bool| {
        apply(&neighbours, &changes, points)
    }),
    op!("topologyChanges", |before: Shape, after: Shape| changes(
        &before, &after
    )),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference's cases (scripts/fixtures/topology_edit_cases.py): the
    /// neighbours put right, bit for bit, and the counts.
    #[test]
    fn every_case_is_put_right_as_the_reference_puts_it() {
        let file = Json::parse(include_str!(
            "../../../../../fixtures/topology/v1/edit.json"
        ))
        .expect("edit.json reads");
        let Json::Arr(cases) = file.get("cases") else {
            panic!("cases")
        };
        assert!(cases.len() >= 11, "{} cases", cases.len());
        let mut off = Vec::new();
        for case in cases {
            let name = String::from_json(case.get("name")).unwrap_or_default();
            let neighbours =
                Vec::<Neighbour>::from_json(case.get("neighbours")).expect("neighbours");
            let changes = Vec::<Change>::from_json(case.get("changes")).expect("changes");
            let points = bool::from_json(case.get("points")).expect("points");
            let got = apply(&neighbours, &changes, points);
            let expected = case.get("expected");
            let Json::Arr(edited) = expected.get("edited") else {
                panic!("edited")
            };
            let want: Vec<(usize, Shape)> = edited
                .iter()
                .map(|e| {
                    (
                        usize::from_json(e.get("index")).expect("index"),
                        Shape::from_json(e.get("shape")).expect("shape"),
                    )
                })
                .collect();
            if got.edited != want {
                off.push(format!("{name}: {:?} ≠ {want:?}", got.edited));
            }
            for (what, n) in [("locked", got.locked), ("invalid", got.invalid)] {
                let w = usize::from_json(expected.get(what)).expect(what);
                if n != w {
                    off.push(format!("{name}: {what} {n} ≠ {w}"));
                }
            }
        }
        assert!(off.is_empty(), "{}", off.join("\n"));
    }

    /// The reference's edits (its `diffs`): the changes found from before and after.
    #[test]
    fn every_edit_gives_the_references_changes() {
        let file = Json::parse(include_str!(
            "../../../../../fixtures/topology/v1/edit.json"
        ))
        .expect("edit.json reads");
        let Json::Arr(diffs) = file.get("diffs") else {
            panic!("diffs")
        };
        assert!(diffs.len() >= 9, "{} diffs", diffs.len());
        let mut off = Vec::new();
        for d in diffs {
            let name = String::from_json(d.get("name")).unwrap_or_default();
            let before = Shape::from_json(d.get("before")).expect("before");
            let after = Shape::from_json(d.get("after")).expect("after");
            let want = Vec::<Change>::from_json(d.get("expected")).expect("expected");
            let got = changes(&before, &after);
            if got != want {
                off.push(format!("{name}: {got:?} ≠ {want:?}"));
            }
        }
        assert!(off.is_empty(), "{}", off.join("\n"));
    }
}
