//! An opening's temporary distances (docs/adr/0701): along the face of its wall's edge, its jambs to the nearest
//! reference before and after it (the face's corner or end, another opening's jamb, another wall's face crossing it),
//! and its middle to the nearest grid crossing the face on each side (docs/adr/0728).
//! `cad.openings.distances` answers them; `cad.openings.edit`'s `distance` moves an opening to one, the reference
//! fixed; the desktop's selection shows them. The shared cases are `fixtures/commands/v1/cad.openings.distances.json`
//! and `cad.openings.edit.distance.json`.

use kentos_commands::{
    CommandResult, DimensionSetFace, Entity, OpeningDistance, OpeningDistances, OpeningFields,
    OpeningReference, OpeningsDistances, OpeningsDistancesInput, Vec2,
};
use kentos_domain::{Document, Slot, Uuid, opening_follow};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::dimension_set::on_edge;
use kentos_geometry_core::ops::edges::edge_length;
use kentos_geometry_core::ops::opening_distance::{self as core, Candidate, Kind};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::openings::fail;

/// One side's reference: what it is, its object, where it stands on the face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reference {
    pub kind: Kind,
    pub uid: Option<Uuid>,
    /// Metres from the face segment's start.
    pub s: f64,
    pub at: Vec2,
}

/// An opening measured on one face of its edge.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    /// The edge of the wall's path it is on.
    pub edge: usize,
    pub face: DimensionSetFace,
    /// The face segment of the edge, from the path's direction.
    pub line: [Vec2; 2],
    /// Its jambs on the face, the start's then the end's, with where they stand on it.
    pub jambs: [(Vec2, f64); 2],
    pub start: Option<Reference>,
    pub end: Option<Reference>,
    /// The nearest grids crossing the face before and after its middle (docs/adr/0728 §1).
    pub start_grid: Option<Reference>,
    pub end_grid: Option<Reference>,
}

impl Measured {
    /// The distance on `start`'s side (true) or the end's; none without a reference there.
    pub fn distance(&self, start: bool) -> Option<f64> {
        if start {
            self.start.map(|r| self.jambs[0].1 - r.s)
        } else {
            self.end.map(|r| r.s - self.jambs[1].1)
        }
    }

    /// Where its middle stands on the face: half way between its jambs.
    pub fn middle(&self) -> f64 {
        (self.jambs[0].1 + self.jambs[1].1) / 2.0
    }

    /// The distance from its middle to the grid on `start`'s side (true) or the end's (docs/adr/0728 §1).
    pub fn grid_distance(&self, start: bool) -> Option<f64> {
        if start {
            self.start_grid.map(|r| (self.middle() - r.s).max(0.0))
        } else {
            self.end_grid.map(|r| (r.s - self.middle()).max(0.0))
        }
    }

    /// Its middle's point on the face.
    pub fn middle_at(&self) -> Vec2 {
        let [a, b] = self.line;
        let length = (b.x - a.x).hypot(b.y - a.y);
        let t = if length > 0.0 { self.middle() / length } else { 0.0 };
        Vec2 {
            x: a.x + (b.x - a.x) * t,
            y: a.y + (b.y - a.y) * t,
        }
    }
}

fn core_point(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

fn key(uid: Uuid) -> u128 {
    u128::from_be_bytes(uid.into_bytes())
}

/// The opening `o` measured on its wall's `face`, for a tool's overlay; none where the command refuses it.
pub fn measured(doc: &Document, slot: Slot, o: &OpeningFields, face: DimensionSetFace) -> Option<Measured> {
    measure(doc, slot, o, face, "uids[0]").ok()
}

/// The opening `o` measured on its wall's `face`; `opening_host_gone`, or `invalid_opening` for an arc edge or a face
/// whose corners the path's vertices do not name. `path` is where the input names it.
pub(crate) fn measure(
    doc: &Document,
    slot: Slot,
    o: &OpeningFields,
    face: DimensionSetFace,
    path: &str,
) -> Result<Measured, Stop> {
    let gone = || {
        fail(
            codes::OPENING_HOST_GONE,
            "Açıklığın duvarı yok; geçici ölçü duvarın yüzünde ölçülür. Önce bir duvara taşıyın (Başka duvara).".into(),
            path,
        )
    };
    if o.broken.is_some() || o.place.len() != 4 {
        return Err(gone());
    }
    let host = Uuid::parse_str(&o.host).map_err(|_| gone())?;
    let line = doc.wall_line_of(&o.host).ok_or_else(gone)?;
    let (s0, s1) = opening_follow::span(o);
    let p = &line.path;
    let tol = 1e-9 * p.length.max(1.0);
    let edge = (0..p.edges.len())
        .find(|&i| s0 >= p.cum[i] - tol && s1 <= p.cum[i] + edge_length(&p.edges[i]) + tol)
        .ok_or_else(gone)?;
    if matches!(p.edges[edge], Edge::Arc { .. }) {
        return Err(fail(
            codes::INVALID_OPENING,
            "Açıklık duvarın yay kenarında; geçici ölçü yalnız düz kenarda ölçülür.".into(),
            path,
        ));
    }
    let [a, b] = doc
        .dimension_set_wall_face(host, face, Some(edge as u32))
        .map_err(|_| {
            fail(
                codes::INVALID_OPENING,
                "Duvarın bu yüzü köşesinde pahlandığı için yüz boyunca ölçülemiyor. Öbür yüzü seçin.".into(),
                path,
            )
        })?;
    let length = (b.x - a.x).hypot(b.y - a.y);
    let u = ((b.x - a.x) / length, (b.y - a.y) / length);
    let along = |q: Vec2| (q.x - a.x) * u.0 + (q.y - a.y) * u.1;
    // The jambs on the face: its place's points 0 and 2 on the left face, 1 and 3 on the right.
    let pick = match face {
        DimensionSetFace::Left => [0, 2],
        DimensionSetFace::Right => [1, 3],
    };
    let mut jambs = pick.map(|i| (o.place[i], along(o.place[i])));
    if jambs[0].1 > jambs[1].1 {
        jambs.swap(0, 1);
    }
    let mut candidates = vec![
        Candidate {
            kind: Kind::End,
            key: 0,
            s: 0.0,
        },
        Candidate {
            kind: Kind::End,
            key: 0,
            s: length,
        },
    ];
    let width = line.spec.width;
    let face_line = [core_point(a), core_point(b)];
    for (other, f) in doc.openings_of(&o.host) {
        if other == slot || f.broken.is_some() || f.place.len() != 4 {
            continue;
        }
        let place = [f.place[0], f.place[1], f.place[2], f.place[3]].map(core_point);
        if !on_edge(face_line, &place, width) {
            continue;
        }
        let Some(uid) = doc.uid(other) else { continue };
        for i in pick {
            candidates.push(Candidate {
                kind: Kind::Opening,
                key: key(uid),
                s: along(f.place[i]),
            });
        }
    }
    // Other walls' faces crossing the face segment: a tee's, a wall drawn on its own at a corner.
    let (lo, hi) = (
        (a.x.min(b.x), a.y.min(b.y)),
        (a.x.max(b.x), a.y.max(b.y)),
    );
    for (uid, e) in doc.entities_with_uids() {
        let Entity::Wall(w) = e else { continue };
        if uid == host {
            continue;
        }
        let reach = w.wall.width + w.wall.offset.abs();
        let near = w.wall.pts.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |r, q| {
            (r.0.min(q.x), r.1.min(q.y), r.2.max(q.x), r.3.max(q.y))
        });
        if near.0 - reach > hi.0 || near.2 + reach < lo.0 || near.1 - reach > hi.1 || near.3 + reach < lo.1 {
            continue;
        }
        let Shape::Wall { pts, bulges, wall } = crate::geometry::shape(e) else {
            continue;
        };
        let Ok(lines) = kentos_geometry_core::geom::wall::lines(&pts, bulges.as_deref(), &wall) else {
            continue;
        };
        for l in [lines.first(), lines.last()].into_iter().flatten() {
            let n = l.pts.len();
            let count = if wall.closed { n } else { n.saturating_sub(1) };
            for k in 0..count {
                if l.bulges.get(k).is_some_and(|b| *b != 0.0) {
                    continue;
                }
                if let Some(t) = core::crossing(face_line[0], face_line[1], l.pts[k], l.pts[(k + 1) % n]) {
                    candidates.push(Candidate {
                        kind: Kind::Wall,
                        key: key(uid),
                        s: t * length,
                    });
                }
            }
        }
    }
    let reference = |c: Candidate| Reference {
        kind: c.kind,
        uid: match c.kind {
            Kind::End => None,
            _ => Some(Uuid::from_bytes(c.key.to_be_bytes())),
        },
        s: c.s,
        at: Vec2 {
            x: a.x + u.0 * c.s,
            y: a.y + u.1 * c.s,
        },
    };
    // The grids crossing the face, from the opening's middle (docs/adr/0728 §1), on its wall's level.
    let level = doc
        .slot_of(host)
        .and_then(|s| match doc.get(s) {
            Some(Entity::Wall(w)) => w.wall.arch.level.clone(),
            _ => None,
        });
    let grids = crate::grid_refs::Grids::of(doc);
    let middle = (jambs[0].1 + jambs[1].1) / 2.0;
    let [before, after] = kentos_geometry_core::ops::grid_ref::crossing_at(
        &grids.ins(),
        level.as_deref(),
        core_point(a),
        core_point(b),
        middle,
    );
    let grid_reference = |g: kentos_geometry_core::ops::grid_ref::GridGap| {
        let uid = grids.uid_of(doc, g.grid)?;
        let s = (g.to.x - a.x) * u.0 + (g.to.y - a.y) * u.1;
        Some(Reference {
            kind: Kind::Grid,
            uid: Some(uid),
            s,
            at: Vec2 { x: g.to.x, y: g.to.y },
        })
    };
    Ok(Measured {
        edge,
        face,
        line: [a, b],
        start: core::nearest(&candidates, jambs[0].1, true).map(reference),
        end: core::nearest(&candidates, jambs[1].1, false).map(reference),
        start_grid: before.and_then(grid_reference),
        end_grid: after.and_then(grid_reference),
        jambs,
    })
}

/// The answer's side: the distance, the jamb, the reference's point and what it is.
fn described(m: &Measured, start: bool) -> Option<OpeningDistance> {
    let r = if start { m.start } else { m.end }?;
    Some(OpeningDistance {
        distance: m.distance(start)?,
        from: m.jambs[usize::from(!start)].0,
        to: r.at,
        reference: reference_of(r),
    })
}

/// The answer's grid on a side: from the opening's middle (docs/adr/0728 §1).
fn described_grid(m: &Measured, start: bool) -> Option<OpeningDistance> {
    let r = if start { m.start_grid } else { m.end_grid }?;
    Some(OpeningDistance {
        distance: m.grid_distance(start)?,
        from: m.middle_at(),
        to: r.at,
        reference: reference_of(r),
    })
}

fn reference_of(r: Reference) -> OpeningReference {
    match (r.kind, r.uid) {
        (Kind::Opening, Some(uid)) => OpeningReference::Opening {
            uid: uid.to_string(),
        },
        (Kind::Wall, Some(uid)) => OpeningReference::Wall {
            uid: uid.to_string(),
        },
        (Kind::Grid, Some(uid)) => OpeningReference::Grid {
            uid: uid.to_string(),
        },
        _ => OpeningReference::End,
    }
}

fn read(doc: &Document, input: &OpeningsDistancesInput) -> Result<OpeningsDistances, Stop> {
    checks::uids(&input.uids, "Ölçülecek açıklık verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    let mut openings = Vec::with_capacity(found.len());
    for (at, slot, e, uid) in found {
        let path = format!("uids[{at}]");
        let Entity::Opening(o) = e else {
            return Err(fail(
                codes::NOT_AN_OPENING,
                "Nesne kapı, pencere ya da boşluk değil; geçici ölçü yalnız açıklıkların mesafesini verir.".into(),
                &path,
            ));
        };
        let m = measure(doc, slot, &o.opening, input.face, &path)?;
        openings.push(OpeningDistances {
            uid: uid.clone(),
            edge: m.edge as u32,
            face: input.face,
            start: described(&m, true),
            end: described(&m, false),
            start_grid: described_grid(&m, true),
            end_grid: described_grid(&m, false),
        });
    }
    Ok(OpeningsDistances {
        openings,
        revision: doc.revision().to_string(),
    })
}

pub mod distances {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &OpeningsDistancesInput) -> CommandResult<()> {
        match read(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(e) => e.into(),
        }
    }
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &OpeningsDistancesInput,
    ) -> CommandResult<OpeningsDistances> {
        plan_in(cx.doc, input)
    }
    /// The answer without a context, for a tool's overlay.
    pub fn plan_in(doc: &Document, input: &OpeningsDistancesInput) -> CommandResult<OpeningsDistances> {
        match read(doc, input) {
            Ok(output) => CommandResult::Completed {
                output,
                warnings: Vec::new(),
            },
            Err(e) => e.into(),
        }
    }
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: OpeningsDistancesInput,
    ) -> CommandResult<OpeningsDistances> {
        plan_in(cx.doc, &input)
    }
}
