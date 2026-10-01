//! Nokta editörü's computations (docs/adr/0153 §2, §3, §5, §6), one for both
//! platforms (the web through WASM): the table's rows after its search,
//! layer and selection filters in the order a column sorts them; the groups
//! of duplicate points and what each keeps; and the vertices of a line,
//! polyline or area that follow a point moved or given a new elevation. The
//! independent reference is `scripts/fixtures/point_editor_cases.py`.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::api::Op;
use crate::jsmath::js_hypot;
use crate::op;
use crate::ops::elevation::{Elevated, ON};
use crate::text::edit::search;
use crate::text::natural::natural_cmp;
use crate::tools::point_text::js_trim;
use crate::vec2::Vec2;

/// A point as the table reads it: its name (label) and code, both as written;
/// its place (Y east, X north); its elevation; its layer's name; whether it is selected.
#[derive(Clone, Debug, PartialEq)]
pub struct TableRow {
    pub name: Option<String>,
    pub east: f64,
    pub north: f64,
    pub z: Option<f64>,
    pub code: Option<String>,
    pub layer: String,
    pub selected: bool,
}

crate::json_struct!(TableRow {
    name,
    east,
    north,
    z,
    code,
    layer,
    selected
});

/// What the table shows: the search box's text, one layer (none: all), only
/// the selected points, and the column it is sorted by (none: the drawing's
/// order) and which way.
#[derive(Clone, Debug, PartialEq)]
pub struct TableQuery {
    pub search: String,
    pub layer: Option<String>,
    pub only_selected: bool,
    pub sort: Option<String>,
    pub descending: bool,
}

crate::json_struct!(TableQuery {
    search,
    layer,
    only_selected => "onlySelected",
    sort,
    descending
});

/// A text value as the table compares it: trimmed, none when empty.
fn value(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(js_trim).filter(|v| !v.is_empty())
}

/// Two values of a column: both there, by `cmp` (reversed when descending);
/// a missing one after any present one, whichever the way.
fn present<T>(
    a: Option<T>,
    b: Option<T>,
    descending: bool,
    cmp: impl Fn(&T, &T) -> Ordering,
) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => {
            let order = cmp(&x, &y);
            if descending { order.reverse() } else { order }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// How a column sorts two rows.
type By = Box<dyn Fn(&TableRow, &TableRow) -> Ordering>;

fn numbers(a: &f64, b: &f64) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Equal)
}

/// The rows the table shows, by their index in `rows` (the drawing's order),
/// in the order it shows them (docs/adr/0153 §2). The search (trimmed) is
/// looked for in the name and the code (`text::edit::search`); a sort by a
/// column the table does not have is the drawing's order. Equal values keep
/// the drawing's order.
pub fn point_table(rows: &[TableRow], query: &TableQuery) -> Vec<u32> {
    let wanted = js_trim(&query.search);
    let mut out: Vec<u32> = (0..rows.len())
        .filter(|&i| {
            let r = &rows[i];
            (!query.only_selected || r.selected)
                && query.layer.as_ref().is_none_or(|l| *l == r.layer)
                && (wanted.is_empty()
                    || [&r.name, &r.code]
                        .into_iter()
                        .any(|v| value(v).is_some_and(|v| search(v, wanted))))
        })
        .map(|i| i as u32)
        .collect();
    let d = query.descending;
    let by: Option<By> = match query.sort.as_deref() {
        Some("name") => Some(Box::new(move |a, b| {
            present(value(&a.name), value(&b.name), d, |x, y| natural_cmp(x, y))
        })),
        Some("code") => Some(Box::new(move |a, b| {
            present(value(&a.code), value(&b.code), d, |x, y| natural_cmp(x, y))
        })),
        Some("east") => Some(Box::new(move |a, b| {
            present(Some(a.east), Some(b.east), d, numbers)
        })),
        Some("north") => Some(Box::new(move |a, b| {
            present(Some(a.north), Some(b.north), d, numbers)
        })),
        Some("z") => Some(Box::new(move |a, b| present(a.z, b.z, d, numbers))),
        Some("layer") => Some(Box::new(move |a, b| {
            present(Some(a.layer.as_str()), Some(b.layer.as_str()), d, |x, y| {
                natural_cmp(x, y)
            })
        })),
        _ => None,
    };
    if let Some(by) = by {
        out.sort_by(|&a, &b| by(&rows[a as usize], &rows[b as usize]).then(a.cmp(&b)));
    }
    out
}

/// `names` in the natural order (`text::natural`), as their indices; equal
/// names keep their order.
pub fn natural_order(names: &[String]) -> Vec<u32> {
    let mut out: Vec<u32> = (0..names.len() as u32).collect();
    out.sort_by(|&a, &b| natural_cmp(&names[a as usize], &names[b as usize]).then(a.cmp(&b)));
    out
}

/// A point as Çift noktaları ayıkla reads it, in the drawing's order.
#[derive(Clone, Debug, PartialEq)]
pub struct DupPoint {
    pub p: Vec2,
    pub z: Option<f64>,
    pub name: Option<String>,
}

crate::json_struct!(DupPoint { p, z, name });

/// One group of duplicates: its members (indices, in the drawing's order),
/// the one kept and where it stands after (its place and elevation).
#[derive(Clone, Debug, PartialEq)]
pub struct DupGroup {
    pub members: Vec<u32>,
    pub kept: u32,
    pub p: Vec2,
    pub z: Option<f64>,
}

crate::json_struct!(out DupGroup {
    members,
    kept,
    p,
    z
});

/// The groups (two members or more, by their first member) and the points
/// that go, in the drawing's order.
#[derive(Clone, Debug, PartialEq)]
pub struct Duplicates {
    pub groups: Vec<DupGroup>,
    pub removed: Vec<u32>,
}

crate::json_struct!(out Duplicates { groups, removed });

/// Which member a group keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    First,
    Last,
    /// The first, at the members' mean place and elevation.
    Average,
}

impl Keep {
    pub fn from_name(name: &str) -> Option<Keep> {
        match name {
            "first" => Some(Keep::First),
            "last" => Some(Keep::Last),
            "average" => Some(Keep::Average),
            _ => None,
        }
    }
}

/// How groups form: by name, or by place within a tolerance (metres; under
/// a nanometre it is the very same place).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DupBy {
    Name,
    Place(f64),
}

/// The smallest tolerance a grid is built for; under it, places must be the same.
const LEAST: f64 = 1e-9;

/// The mean of `values` as `first` plus the mean of their differences from it:
/// on survey coordinates no rounding error builds up.
fn mean(values: &[f64]) -> f64 {
    let first = values[0];
    let shift: f64 = values.iter().map(|v| v - first).sum::<f64>() / values.len() as f64;
    first + shift
}

/// The duplicate points of `points` (docs/adr/0153 §5): by name, the names
/// trimmed and compared exactly, unnamed points never duplicates; by place,
/// each point in turn joins the first group whose first point is within the
/// tolerance, else starts one (no chains: every member is near the first).
/// Each group keeps its first, its last, or its first moved to the members'
/// mean place and the mean of the elevations they have (none when none has one).
pub fn duplicate_points(points: &[DupPoint], by: DupBy, keep: Keep) -> Duplicates {
    let mut groups: Vec<Vec<u32>> = Vec::new();
    match by {
        DupBy::Name => {
            let mut index: HashMap<&str, usize> = HashMap::new();
            for (i, p) in points.iter().enumerate() {
                let Some(name) = value(&p.name) else {
                    continue;
                };
                match index.get(name) {
                    Some(&g) => groups[g].push(i as u32),
                    None => {
                        index.insert(name, groups.len());
                        groups.push(vec![i as u32]);
                    }
                }
            }
        }
        DupBy::Place(tolerance) => {
            let exact = tolerance.is_nan() || tolerance < LEAST;
            // The groups' first points by the cell of a grid as wide as the tolerance.
            let cell = |p: Vec2| -> (i64, i64) {
                if exact {
                    let bits = |v: f64| (if v == 0.0 { 0.0f64 } else { v }).to_bits() as i64;
                    (bits(p.x), bits(p.y))
                } else {
                    (
                        (p.x / tolerance).floor() as i64,
                        (p.y / tolerance).floor() as i64,
                    )
                }
            };
            let mut anchors: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
            for (i, p) in points.iter().enumerate() {
                let (cx, cy) = cell(p.p);
                let reach = if exact { 0 } else { 1 };
                let mut best: Option<usize> = None;
                for dx in -reach..=reach {
                    for dy in -reach..=reach {
                        for &g in anchors.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                            let a = points[groups[g][0] as usize].p;
                            let near = if exact {
                                a == p.p
                            } else {
                                js_hypot(a.x - p.p.x, a.y - p.p.y) <= tolerance
                            };
                            if near && best.is_none_or(|b| g < b) {
                                best = Some(g);
                            }
                        }
                    }
                }
                match best {
                    Some(g) => groups[g].push(i as u32),
                    None => {
                        anchors.entry((cx, cy)).or_default().push(groups.len());
                        groups.push(vec![i as u32]);
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut removed = Vec::new();
    for members in groups.into_iter().filter(|m| m.len() > 1) {
        let at = |i: u32| &points[i as usize];
        let (kept, p, z) = match keep {
            Keep::First => (members[0], at(members[0]).p, at(members[0]).z),
            Keep::Last => {
                let last = members[members.len() - 1];
                (last, at(last).p, at(last).z)
            }
            Keep::Average => {
                let xs: Vec<f64> = members.iter().map(|&i| at(i).p.x).collect();
                let ys: Vec<f64> = members.iter().map(|&i| at(i).p.y).collect();
                let zs: Vec<f64> = members.iter().filter_map(|&i| at(i).z).collect();
                let z = (!zs.is_empty()).then(|| mean(&zs));
                (members[0], Vec2::new(mean(&xs), mean(&ys)), z)
            }
        };
        removed.extend(members.iter().copied().filter(|&i| i != kept));
        out.push(DupGroup {
            members,
            kept,
            p,
            z,
        });
    }
    removed.sort_unstable();
    Duplicates {
        groups: out,
        removed,
    }
}

/// The paths of a line, polyline or area (in `elevation::paths`' order) with
/// every vertex within 1 µm of `from` moved to `to`, and when `set_z` those
/// vertices given the elevation `z` (none: they lose theirs). The bulges stay
/// (a grip's rule, docs/adr/0068). None when no vertex is there or nothing
/// would change (docs/adr/0153 §3).
pub fn follow_point(
    paths: &[Elevated],
    from: Vec2,
    to: Vec2,
    set_z: bool,
    z: Option<f64>,
) -> Option<Vec<Elevated>> {
    let mut out = paths.to_vec();
    for path in &mut out {
        if set_z && path.zs.len() < path.pts.len() {
            path.zs.resize(path.pts.len(), None);
        }
        for (k, p) in path.pts.iter_mut().enumerate() {
            if js_hypot(p.x - from.x, p.y - from.y) <= ON {
                *p = to;
                if set_z {
                    path.zs[k] = z;
                }
            }
        }
    }
    (out != paths).then_some(out)
}

pub(crate) static OPS: &[Op] = &[
    op!("pointTable", |rows: Vec<TableRow>, query: TableQuery| {
        point_table(&rows, &query)
    }),
    op!("naturalOrder", |names: Vec<String>| natural_order(&names)),
    op!(
        "duplicatePoints",
        |points: Vec<DupPoint>, by: String, tolerance: f64, keep: String| {
            let by = if by == "name" {
                DupBy::Name
            } else {
                DupBy::Place(tolerance)
            };
            duplicate_points(&points, by, Keep::from_name(&keep).unwrap_or(Keep::First))
        }
    ),
    op!("followPoint", |paths: Vec<Elevated>,
                        from: Vec2,
                        to: Vec2,
                        set_z: bool,
                        z: Option<f64>| {
        follow_point(&paths, from, to, set_z, z)
    }),
];
