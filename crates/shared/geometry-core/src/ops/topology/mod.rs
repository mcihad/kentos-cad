//! Topolojik temizlik (docs/adr/0148): line work and area outlines put
//! right within a tolerance the user gives. Ends that almost meet meet,
//! vertices that almost coincide coincide (Uçlar, Köşeler; `join`), and a
//! free end that stops short of a line or runs past one is extended or
//! trimmed, or else moved onto the nearest line (Uzat, Buda, Uçlar; `ends`).
//! A vertex never moves further than the tolerance and joins on a vertex
//! that is there, never an average; fixed objects and points never move;
//! elevations are carried by the ADR's rules (§6).
//!
//! The objects come in the drawing's order as paths with their elevations
//! (a line, an open polyline, an arc as two vertices and a bulge, an
//! area's rings, a point, or a boundary that has edges only: a circle, an
//! ellipse's or a curve's chords); the answer is each changed object's new
//! paths, every change for the preview, the counts and the largest move.
//! The independent reference is `scripts/fixtures/topology_cases.py`
//! (`fixtures/topology/v1/clean.json`).

use crate::api::Op;
use crate::op;
use crate::vec2::Vec2;

mod ends;
mod join;

/// "The same place" and "on" (m): the elevation rules' (`ops::elevation::ON`).
pub const TOUCH: f64 = 1e-6;

/// A path of an object with its bulges and its vertices' elevations.
#[derive(Clone, Debug, PartialEq)]
pub struct TopoPath {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
    pub closed: bool,
    pub zs: Vec<Option<f64>>,
}

crate::json_struct!(TopoPath {
    pts,
    bulges,
    closed,
    zs
});

/// An object as the cleanup takes it.
#[derive(Clone, Debug, PartialEq)]
pub struct TopoObject {
    /// "line", "polyline", "arc", "area", "point" or "edges".
    pub kind: String,
    /// A fixed object never moves; its vertices and edges are where others go.
    pub fixed: bool,
    pub paths: Vec<TopoPath>,
}

crate::json_struct!(TopoObject { kind, fixed, paths });

/// The four works (docs/adr/0148 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopoWorks {
    pub ends: bool,
    pub vertices: bool,
    pub extend: bool,
    pub trim: bool,
}

crate::json_struct!(TopoWorks {
    ends,
    vertices,
    extend,
    trim
});

/// A changed object: its index in the input, its new paths.
#[derive(Clone, Debug, PartialEq)]
pub struct TopoChanged {
    pub object: usize,
    pub paths: Vec<TopoPath>,
}

crate::json_struct!(TopoChanged { object, paths });

/// One change, for the preview: "end", "vertex", "extended", "trimmed" or "edge".
#[derive(Clone, Debug, PartialEq)]
pub struct TopoChange {
    pub kind: String,
    pub from: Vec2,
    pub to: Vec2,
    pub object: usize,
}

crate::json_struct!(TopoChange {
    kind,
    from,
    to,
    object
});

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TopoCounts {
    pub ends: usize,
    pub vertices: usize,
    pub extended: usize,
    pub trimmed: usize,
    pub edges: usize,
}

crate::json_struct!(TopoCounts {
    ends,
    vertices,
    extended,
    trimmed,
    edges
});

#[derive(Clone, Debug, PartialEq)]
pub struct TopoResult {
    pub changed: Vec<TopoChanged>,
    pub changes: Vec<TopoChange>,
    pub counts: TopoCounts,
    pub max_shift: f64,
}

crate::json_struct!(TopoResult {
    changed,
    changes,
    counts,
    max_shift => "maxShift"
});

/// An object's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Line,
    Polyline,
    Arc,
    Area,
    Point,
    Edges,
}

impl Kind {
    fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "line" => Kind::Line,
            "polyline" => Kind::Polyline,
            "arc" => Kind::Arc,
            "area" => Kind::Area,
            "point" => Kind::Point,
            "edges" => Kind::Edges,
            _ => return None,
        })
    }

    /// Line work with ends: a line, an open polyline, an arc.
    fn open(self) -> bool {
        matches!(self, Kind::Line | Kind::Polyline | Kind::Arc)
    }
}

/// An object while it is worked on.
struct Work {
    kind: Kind,
    fixed: bool,
    paths: Vec<TopoPath>,
    changed: bool,
}

/// The answer as it grows.
struct Out {
    changes: Vec<TopoChange>,
    counts: TopoCounts,
    shift: f64,
}

impl Out {
    fn note(&mut self, kind: &str, from: Vec2, to: Vec2, object: usize) {
        self.changes.push(TopoChange {
            kind: kind.to_owned(),
            from,
            to,
            object,
        });
        self.shift = crate::jsmath::js_max(
            self.shift,
            crate::jsmath::js_hypot(to.x - from.x, to.y - from.y),
        );
    }
}

/// Topolojik temizlik over `objects` (docs/adr/0148). Refused: a tolerance
/// below 1 µm or not finite, an unknown kind, a path whose elevations or
/// bulges do not go with its vertices.
pub fn topology_clean(
    objects: &[TopoObject],
    tol: f64,
    works: TopoWorks,
) -> Result<TopoResult, String> {
    if !(tol.is_finite() && tol >= TOUCH) {
        return Err("Tolerans en az 0,000001 m olmalı.".into());
    }
    let mut work = Vec::with_capacity(objects.len());
    for (i, o) in objects.iter().enumerate() {
        let kind = Kind::parse(&o.kind)
            .ok_or_else(|| format!("{i}. nesnenin türü bilinmiyor: “{}”", o.kind))?;
        for p in &o.paths {
            if p.zs.len() != p.pts.len()
                || p.bulges.as_ref().is_some_and(|b| b.len() != p.pts.len())
            {
                return Err(format!(
                    "{i}. nesnenin yolunda köşe, kot ve kabarıklık sayıları tutmuyor."
                ));
            }
            if p.pts.iter().any(|q| !q.x.is_finite() || !q.y.is_finite()) {
                return Err(format!("{i}. nesnenin bir köşesi sonlu değil."));
            }
        }
        work.push(Work {
            kind,
            fixed: o.fixed,
            paths: o.paths.clone(),
            changed: false,
        });
    }
    let mut out = Out {
        changes: Vec::new(),
        counts: TopoCounts::default(),
        shift: 0.0,
    };
    join::join(&mut work, tol, works, &mut out);
    ends::ends(&mut work, tol, works, &mut out);
    let changed = work
        .into_iter()
        .enumerate()
        .filter(|(_, w)| w.changed)
        .map(|(object, w)| TopoChanged {
            object,
            paths: w.paths,
        })
        .collect();
    Ok(TopoResult {
        changed,
        changes: out.changes,
        counts: out.counts,
        max_shift: out.shift,
    })
}

pub(crate) static OPS: &[Op] = &[op!(
    "topologyClean",
    |objects: Vec<TopoObject>, tolerance: f64, works: TopoWorks| topology_clean(
        &objects, tolerance, works
    )
)];
