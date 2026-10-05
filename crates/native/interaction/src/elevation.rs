//! What the interface does with the elevations of vertices (docs/adr/0142):
//! Kot ver, Öznitelikler, the grip's tag, Koordinat oku and the hover card
//! all read and write them through here, so the desktop says and does one
//! thing. The web's is `apps/web/src/product/elevationValues.ts`.
//!
//! A vertex without an elevation is `None`, not 0. A point's is its `z`, a
//! line's its two ends, a polyline's one for each vertex, an area's the outer
//! ring's and then each hole's, in that order.
//!
//! Nothing is computed here beyond sums of what the shared core answers: the
//! length in space is the core's ([`length_3d`]), and Artır's arithmetic is a
//! plain addition, the value written the sum and not a rounded one
//! (CLAUDE.md §23.2).

use kentos_contracts::{Entity, EntityGeometry};
use kentos_geometry_core::ops::elevation::{ON, length_3d};
use kentos_geometry_core::ops::grips::{grip_part, hole_grip};
use kentos_geometry_core::tools::point_text::{is_js_space, js_trim, parse_number};
use kentos_native_application::elevation::paths;
use kentos_native_application::geometry::{edit_geometry, shape};

use crate::Vec2;
use crate::format::Format;

/// Whether an object's kind takes elevations: a point (its `z`), a line, a
/// polyline and an area. The others are left out of Kot ver.
pub fn takes(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Point(_) | Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    )
}

/// Whether an object is a line, a polyline or an area: what has vertex
/// elevations beside a point's `z`.
pub fn is_path(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    )
}

fn any(zs: &Option<Vec<Option<f64>>>) -> bool {
    zs.iter().flatten().any(Option::is_some)
}

/// Whether any vertex has an elevation (nothing is allocated).
pub fn has_any(e: &Entity) -> bool {
    match e {
        // Every point and every part too (docs/adr/0174).
        Entity::Point(p) => p.z.is_some() || p.parts.iter().flatten().any(|q| q.z.is_some()),
        Entity::Line(l) => l.za.is_some() || l.zb.is_some(),
        Entity::Polyline(p) => any(&p.zs) || p.parts.iter().flatten().any(|q| any(&q.zs)),
        Entity::Polygon(p) => {
            let holes = |hs: &Option<Vec<kentos_contracts::RingGeometry>>| {
                hs.iter().flatten().any(|h| any(&h.zs))
            };
            // Every part's too (docs/adr/0143).
            any(&p.zs)
                || holes(&p.holes)
                || p.parts.iter().flatten().any(|q| any(&q.zs) || holes(&q.holes))
        }
        _ => false,
    }
}

/// Every vertex's elevation, in the order above; empty for a kind without any.
pub fn vertex_elevations(e: &Entity) -> Vec<Option<f64>> {
    // Every point of a multi-point object (docs/adr/0174).
    if let Entity::Point(p) = e {
        return std::iter::once(p.z)
            .chain(p.parts.iter().flatten().map(|q| q.z))
            .collect();
    }
    paths(e).into_iter().flat_map(|p| p.zs).collect()
}

/// What a list of elevations comes to, as the Kot rows say it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Summary {
    /// No vertex has one.
    None,
    /// Every vertex has this one.
    Value(f64),
    /// Every vertex has one and they differ: the lowest and the highest.
    Range(f64, f64),
    /// Some vertices have none: the lowest and the highest of those that
    /// have one (equal when they agree).
    Partial(f64, f64),
}

/// What a row says when no vertex has an elevation.
pub const NO_ELEVATION: &str = "kot yok";
/// What it adds to the range when some vertices have none.
const SOME_WITHOUT: &str = "(bazı köşeler kotsuz)";

/// What a run of elevations adds up to, without keeping them.
#[derive(Default)]
struct Tally {
    min: f64,
    max: f64,
    have: usize,
    missing: usize,
}

impl Tally {
    fn add(&mut self, z: Option<f64>) {
        match z {
            Some(z) if self.have == 0 => {
                (self.min, self.max) = (z, z);
                self.have = 1;
            }
            Some(z) => {
                (self.min, self.max) = (self.min.min(z), self.max.max(z));
                self.have += 1;
            }
            None => self.missing += 1,
        }
    }

    /// A ring of `n` vertices with its elevations, absent when none has one.
    fn ring(&mut self, zs: &Option<Vec<Option<f64>>>, n: usize) {
        match zs {
            Some(list) => {
                list.iter().for_each(|z| self.add(*z));
                self.missing += n.saturating_sub(list.len());
            }
            None => self.missing += n,
        }
    }

    /// Every vertex of an object that takes an elevation.
    fn object(&mut self, e: &Entity) {
        match e {
            Entity::Point(p) => {
                self.add(p.z);
                // Every point of a multi-point object (docs/adr/0174).
                for q in p.parts.iter().flatten() {
                    self.add(q.z);
                }
            }
            Entity::Line(l) => {
                self.add(l.za);
                self.add(l.zb);
            }
            Entity::Polyline(p) => {
                self.ring(&p.zs, p.pts.len());
                // Every other part (docs/adr/0174).
                for part in p.parts.iter().flatten() {
                    self.ring(&part.zs, part.pts.len());
                }
            }
            Entity::Polygon(p) => {
                self.ring(&p.zs, p.pts.len());
                for hole in p.holes.iter().flatten() {
                    self.ring(&hole.zs, hole.pts.len());
                }
                // Every other part, its ring and holes (docs/adr/0143).
                for part in p.parts.iter().flatten() {
                    self.ring(&part.zs, part.pts.len());
                    for hole in part.holes.iter().flatten() {
                        self.ring(&hole.zs, hole.pts.len());
                    }
                }
            }
            _ => {}
        }
    }

    fn summary(self) -> Summary {
        if self.have == 0 {
            Summary::None
        } else if self.missing > 0 {
            Summary::Partial(self.min, self.max)
        } else if self.min == self.max {
            Summary::Value(self.min)
        } else {
            Summary::Range(self.min, self.max)
        }
    }
}

impl Summary {
    /// What a list of elevations comes to.
    pub fn of(zs: &[Option<f64>]) -> Self {
        let mut tally = Tally::default();
        zs.iter().for_each(|z| tally.add(*z));
        tally.summary()
    }

    /// What an object's vertices come to, all its rings together: a line's
    /// two ends, a polyline's vertices, an area's outer ring and its holes;
    /// a point's `z`. Nothing is copied. Two objects say the same when their
    /// elevations come to the same.
    pub fn of_object(e: &Entity) -> Self {
        let mut tally = Tally::default();
        tally.object(e);
        tally.summary()
    }

    /// What the vertices of several objects come to together, as one list:
    /// the value when every one has the same, none when none has any,
    /// otherwise a range or a partial one. Objects that take no elevation
    /// add nothing.
    pub fn of_objects<'a>(objects: impl IntoIterator<Item = &'a Entity>) -> Self {
        let mut tally = Tally::default();
        objects.into_iter().for_each(|e| tally.object(e));
        tally.summary()
    }

    /// How the row shows it, and the unit that follows it: `kot yok`;
    /// `100.000` and `m`; `98.500–105.250` and `m` (an en dash without spaces, as ranges are set: four-digit elevations fit the cell at 1100 px). With vertices missing it
    /// is the range of those that have one; [`Summary::note`] says the rest.
    pub fn shown(&self, f: &Format) -> (String, Option<&'static str>) {
        let range = |lo: f64, hi: f64| {
            if lo == hi {
                f.length_bare(lo)
            } else {
                format!("{}–{}", f.length_bare(lo), f.length_bare(hi))
            }
        };
        match *self {
            Summary::None => (NO_ELEVATION.to_owned(), None),
            Summary::Value(z) => (f.length_bare(z), Some(f.length_unit_label())),
            Summary::Range(lo, hi) | Summary::Partial(lo, hi) => {
                (range(lo, hi), Some(f.length_unit_label()))
            }
        }
    }

    /// What follows the range when some vertices have no elevation:
    /// `(bazı köşeler kotsuz)`. The panel puts it on a line under the value,
    /// which is too narrow to hold both.
    pub fn note(&self) -> Option<&'static str> {
        matches!(self, Summary::Partial(..)).then_some(SOME_WITHOUT)
    }

    /// The same as one text, as a message says it: `98.500–105.250 m
    /// (bazı köşeler kotsuz)`.
    pub fn text(&self, f: &Format) -> String {
        let (text, unit) = self.shown(f);
        let mut out = match unit {
            Some(unit) => format!("{text} {unit}"),
            None => text,
        };
        if let Some(note) = self.note() {
            out.push(' ');
            out.push_str(note);
        }
        out
    }
}

/// What was typed into a Kot cell (docs/adr/0142), as the web's
/// `parseElevation` reads it: a number, a decimal comma allowed and a trailing
/// “m”, is `Some(Some(z))`; nothing (or an “m” alone) is `Some(None)`, the
/// elevation cleared; anything else is `None`, not taken.
pub fn parse_typed(text: &str) -> Option<Option<f64>> {
    parse_typed_in(text, "m")
}

/// [`parse_typed`] in a project whose unit is `unit` (docs/adr/0165 §2): its
/// mark may follow the number; the number is in the unit it was typed in.
pub fn parse_typed_in(text: &str, unit: &str) -> Option<Option<f64>> {
    let typed = js_trim(text);
    let typed = match typed.strip_suffix(unit) {
        Some(rest) => rest.trim_end_matches(is_js_space),
        None => typed,
    };
    if typed.is_empty() {
        return Some(None);
    }
    parse_number(typed).map(Some)
}

/// The length in space beside the plan one, when every vertex has an
/// elevation: `3B uzunluk` of a line or a polyline, `3B çevre` of an area
/// (its outer ring and every hole, all closed, as the plan perimeter counts
/// them). `None` for anything else, or when a vertex has none.
pub fn space_length(e: &Entity) -> Option<(&'static str, f64)> {
    let label = match e {
        Entity::Line(_) | Entity::Polyline(_) => "3B uzunluk",
        Entity::Polygon(_) => "3B çevre",
        _ => return None,
    };
    if !has_any(e) {
        return None;
    }
    let mut sum = 0.0;
    for path in paths(e) {
        sum += length_3d(&path.pts, path.bulges.as_deref(), path.closed, &path.zs)?;
    }
    Some((label, sum))
}

/// The elevation of the vertex a grip stands on, or none: a mid grip is no
/// vertex, and a vertex may have none. `index` counts as the core lists the
/// grips (`entity_grips`): a path's vertices, then one mid grip for each
/// edge, then the vertices of each hole; a multi-part area's and polyline's
/// part after part (docs/adr/0143, 0174); a multi-point object's points.
pub fn grip_elevation(e: &Entity, index: usize) -> Option<f64> {
    if let Entity::Polygon(p) | Entity::Polyline(p) = e
        && p.parts.as_ref().is_some_and(|ps| !ps.is_empty())
    {
        let (k, local) = grip_part(&shape(e), index)?;
        let one = if k == 0 {
            let mut first = p.clone();
            first.parts = None;
            first
        } else {
            let part = p.parts.as_ref()?.get(k - 1)?;
            kentos_contracts::PathEntity {
                base: p.base.clone(),
                pts: part.pts.clone(),
                bulges: part.bulges.clone(),
                holes: part.holes.clone(),
                zs: part.zs.clone(),
                parts: None,
            }
        };
        return grip_elevation(
            &match e {
                Entity::Polygon(_) => Entity::Polygon(one),
                _ => Entity::Polyline(one),
            },
            local,
        );
    }
    match e {
        Entity::Point(p) if index == 0 => p.z,
        // A multi-point object's other points, each a grip (docs/adr/0174).
        Entity::Point(p) => p.parts.as_ref()?.get(index - 1)?.z,
        Entity::Line(l) => match index {
            0 => l.za,
            1 => l.zb,
            _ => None,
        },
        Entity::Polyline(p) => p.zs.as_ref()?.get(index).copied().flatten(),
        Entity::Polygon(p) => {
            if index < p.pts.len() {
                return p.zs.as_ref()?.get(index).copied().flatten();
            }
            let hole = hole_grip(&shape(e), index)?;
            p.holes
                .as_ref()?
                .get(hole.hole)?
                .zs
                .as_ref()?
                .get(hole.vertex)
                .copied()
                .flatten()
        }
        _ => None,
    }
}

/// The elevation of the object's vertex at `at`, or none: no vertex there,
/// or it has none. A line's end counts, and a point is a vertex.
pub fn elevation_at(e: &Entity, at: Vec2) -> Option<f64> {
    let near = |q: &kentos_contracts::Vec2| (q.x - at.x).abs() <= ON && (q.y - at.y).abs() <= ON;
    match e {
        // Any point of a multi-point object (docs/adr/0174).
        Entity::Point(p) => p.z.filter(|_| near(&p.p)).or_else(|| {
            p.parts
                .iter()
                .flatten()
                .find(|q| near(&q.p))
                .and_then(|q| q.z)
        }),
        Entity::Line(l) => {
            if near(&l.a) {
                l.za
            } else if near(&l.b) {
                l.zb
            } else {
                None
            }
        }
        Entity::Polyline(_) | Entity::Polygon(_) => paths(e).into_iter().find_map(|path| {
            path.pts
                .iter()
                .zip(&path.zs)
                .find(|(q, z)| z.is_some() && (q.x - at.x).abs() <= ON && (q.y - at.y).abs() <= ON)
                .and_then(|(_, z)| *z)
        }),
        _ => None,
    }
}

/// What a write does to each vertex's elevation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    /// Every vertex gets this elevation, or none: Sabit and Sıfırla, and an
    /// Öznitelikler cell that takes them all.
    Set(Option<f64>),
    /// Every vertex that has an elevation gets it plus the difference; one
    /// without stays without (Artır).
    Raise(f64),
    /// One end of a line, by its place (0 the start, 1 the end): its
    /// elevation, or none; the other end stays as it is.
    End(usize, Option<f64>),
}

impl Change {
    /// The vertex at `index` of the object's order, with `z` now, after the change.
    fn apply(self, z: Option<f64>, index: usize) -> Option<f64> {
        match self {
            Change::Set(z) => z,
            Change::Raise(by) => z.map(|z| z + by),
            Change::End(end, to) if end == index => to,
            Change::End(..) => z,
        }
    }
}

/// Whether `change` would leave every vertex's elevation of the object as it is.
pub fn leaves(e: &Entity, change: Change) -> bool {
    vertex_elevations(e)
        .iter()
        .enumerate()
        .all(|(i, z)| change.apply(*z, i) == *z)
}

/// The object's own geometry with each vertex's elevation as `change` says,
/// as `cad.entities.edit` takes it in the operation `elevation`: `zs` for a
/// line (its two ends), a polyline and an area (each hole's in its own `zs`),
/// `z` for a point. Every elevation is explicit, so the command writes them
/// as they are. `None` for a kind without elevations.
pub fn geometry_with(e: &Entity, change: Change) -> Option<EntityGeometry> {
    let mut geometry = edit_geometry(shape(e))?;
    let mut next = 0;
    let mut run = |zs: Vec<Option<f64>>| -> Vec<Option<f64>> {
        zs.into_iter()
            .map(|z| {
                let out = change.apply(z, next);
                next += 1;
                out
            })
            .collect()
    };
    match (&mut geometry, e) {
        (EntityGeometry::Point { z, parts, .. }, Entity::Point(p)) => {
            *z = change.apply(p.z, 0);
            // Every point of a multi-point object, in turn (docs/adr/0174).
            for (k, (part, was)) in parts
                .iter_mut()
                .flatten()
                .zip(p.parts.iter().flatten())
                .enumerate()
            {
                part.z = change.apply(was.z, k + 1);
            }
        }
        (EntityGeometry::Line { zs, .. }, Entity::Line(l)) => *zs = Some(run(vec![l.za, l.zb])),
        (EntityGeometry::Polyline { zs, parts, .. }, Entity::Polyline(_)) => {
            // `paths`' order: the polyline, then each other part (docs/adr/0174).
            let mut lines = paths(e).into_iter();
            *zs = lines.next().map(|p| run(p.zs));
            for part in parts.iter_mut().flatten() {
                part.zs = lines.next().map(|p| run(p.zs));
            }
        }
        (
            EntityGeometry::Polygon {
                zs, holes, parts, ..
            },
            Entity::Polygon(_),
        ) => {
            // `paths`' order: the ring, its holes, then each other part's ring and holes.
            let mut rings = paths(e).into_iter();
            *zs = rings.next().map(|p| run(p.zs));
            for hole in holes.iter_mut().flatten() {
                hole.zs = rings.next().map(|p| run(p.zs));
            }
            for part in parts.iter_mut().flatten() {
                part.zs = rings.next().map(|p| run(p.zs));
                for hole in part.holes.iter_mut().flatten() {
                    hole.zs = rings.next().map(|p| run(p.zs));
                }
            }
        }
        _ => return None,
    }
    Some(geometry)
}

#[cfg(test)]
mod tests {
    use kentos_contracts::{EntityBase, LineEntity, PathEntity, PointEntity, RingGeometry};

    use super::*;

    fn base() -> EntityBase {
        EntityBase {
            id: 1,
            layer_id: "cizim".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
        }
    }

    fn pt(x: f64, y: f64) -> kentos_contracts::Vec2 {
        kentos_contracts::Vec2 { x, y }
    }

    fn line(za: Option<f64>, zb: Option<f64>) -> Entity {
        Entity::Line(LineEntity {
            base: base(),
            a: pt(0.0, 0.0),
            b: pt(30.0, 0.0),
            za,
            zb,
        })
    }

    /// A 10 m square with a 2 m square hole.
    fn area(zs: Option<Vec<Option<f64>>>, hole: Option<Vec<Option<f64>>>) -> Entity {
        Entity::Polygon(PathEntity {
            base: base(),
            pts: vec![pt(0.0, 0.0), pt(10.0, 0.0), pt(10.0, 10.0), pt(0.0, 10.0)],
            bulges: None,
            holes: Some(vec![RingGeometry {
                pts: vec![pt(2.0, 2.0), pt(4.0, 2.0), pt(4.0, 4.0), pt(2.0, 4.0)],
                bulges: None,
                zs: hole,
            }]),
            zs,
            parts: None,
        })
    }

    fn polyline(zs: Option<Vec<Option<f64>>>) -> Entity {
        Entity::Polyline(PathEntity {
            base: base(),
            pts: vec![pt(0.0, 0.0), pt(10.0, 0.0), pt(10.0, 10.0)],
            bulges: None,
            holes: None,
            zs,
            parts: None,
        })
    }

    fn some(v: &[f64]) -> Option<Vec<Option<f64>>> {
        Some(v.iter().copied().map(Some).collect())
    }

    #[test]
    fn a_list_comes_to_a_value_a_range_or_nothing() {
        assert_eq!(Summary::of(&[]), Summary::None);
        assert_eq!(Summary::of(&[None, None]), Summary::None);
        assert_eq!(Summary::of(&[Some(4.0), Some(4.0)]), Summary::Value(4.0));
        assert_eq!(
            Summary::of(&[Some(105.25), Some(98.5), Some(101.0)]),
            Summary::Range(98.5, 105.25)
        );
        assert_eq!(
            Summary::of(&[Some(105.25), None, Some(98.5)]),
            Summary::Partial(98.5, 105.25)
        );
        // Partial with the ones that have an elevation agreeing: the range is one value.
        assert_eq!(Summary::of(&[Some(2.0), None]), Summary::Partial(2.0, 2.0));
    }

    #[test]
    fn the_rows_say_it_in_the_project_s_digits() {
        let f = Format::default();
        assert_eq!(Summary::None.text(&f), "kot yok");
        assert_eq!(Summary::None.shown(&f), ("kot yok".into(), None));
        assert_eq!(
            Summary::Value(-4.25).shown(&f),
            ("-4.250".into(), Some("m"))
        );
        assert_eq!(Summary::Range(98.5, 105.25).text(&f), "98.500–105.250 m");
        assert_eq!(
            Summary::Partial(98.5, 105.25).text(&f),
            "98.500–105.250 m (bazı köşeler kotsuz)"
        );
        // The row shows the range with its unit and puts the note on a line under it.
        assert_eq!(
            Summary::Partial(98.5, 105.25).shown(&f),
            ("98.500–105.250".into(), Some("m"))
        );
        assert_eq!(
            Summary::Partial(98.5, 105.25).note(),
            Some("(bazı köşeler kotsuz)")
        );
        assert_eq!(Summary::Range(98.5, 105.25).note(), None);
        assert_eq!(Summary::None.note(), None);
        assert_eq!(
            Summary::Partial(2.0, 2.0).text(&f),
            "2.000 m (bazı köşeler kotsuz)"
        );
    }

    /// 30 m in plan, rising 40 m: 50 m in space (a 3-4-5 triangle scaled by 10).
    #[test]
    fn a_line_rising_forty_metres_over_thirty_is_fifty_long() {
        let (label, len) = space_length(&line(Some(0.0), Some(40.0))).expect("both ends");
        assert_eq!(label, "3B uzunluk");
        assert!((len - 50.0).abs() < 1e-12, "{len}");
        // A vertex without an elevation: no length in space; none at all: none either.
        assert_eq!(space_length(&line(Some(0.0), None)), None);
        assert_eq!(space_length(&line(None, None)), None);
        // A point has none.
        let point = Entity::Point(PointEntity {
            base: base(),
            p: pt(0.0, 0.0),
            z: Some(3.0),
            parts: None,
        });
        assert_eq!(space_length(&point), None);
    }

    #[test]
    fn a_polyline_sums_its_edges_and_an_area_closes_every_ring() {
        // 10 m east rising 0, then 10 m north rising 10: 10 + √200.
        let path = polyline(some(&[5.0, 5.0, 15.0]));
        let (label, len) = space_length(&path).expect("all three");
        assert_eq!(label, "3B uzunluk");
        assert!((len - (10.0 + 200f64.sqrt())).abs() < 1e-12, "{len}");
        // A flat area at 7 m: the plan perimeter, 40 m, plus the hole's 8 m.
        let flat = area(some(&[7.0; 4]), some(&[7.0; 4]));
        let (label, len) = space_length(&flat).expect("outer ring and hole");
        assert_eq!(label, "3B çevre");
        assert!((len - 48.0).abs() < 1e-12, "{len}");
        // One hole vertex without an elevation: none of it counts.
        let partial = area(
            some(&[7.0; 4]),
            Some(vec![Some(7.0), None, Some(7.0), Some(7.0)]),
        );
        assert_eq!(space_length(&partial), None);
    }

    #[test]
    fn a_grip_stands_on_a_vertex_of_the_ring_or_of_a_hole() {
        let a = area(
            Some(vec![Some(1.0), None, Some(3.0), Some(4.0)]),
            some(&[5.0, 6.0, 7.0, 8.0]),
        );
        assert_eq!(grip_elevation(&a, 0), Some(1.0));
        assert_eq!(grip_elevation(&a, 1), None, "a vertex without one");
        // Grips 4 to 7 are the ring's mid grips: no vertex.
        assert_eq!(grip_elevation(&a, 4), None);
        // The hole's vertices follow the mid grips: 4 + 4 = 8.
        assert_eq!(grip_elevation(&a, 8), Some(5.0));
        assert_eq!(grip_elevation(&a, 11), Some(8.0));
        assert_eq!(grip_elevation(&a, 12), None);
        let p = polyline(some(&[1.0, 2.0, 3.0]));
        assert_eq!(grip_elevation(&p, 2), Some(3.0));
        assert_eq!(grip_elevation(&p, 3), None, "the first mid grip");
        assert_eq!(grip_elevation(&line(Some(9.0), None), 0), Some(9.0));
        assert_eq!(grip_elevation(&line(Some(9.0), None), 1), None);
    }

    #[test]
    fn a_vertex_is_found_by_its_place() {
        let p = polyline(some(&[1.0, 2.0, 3.0]));
        assert_eq!(elevation_at(&p, Vec2::new(10.0, 0.0)), Some(2.0));
        assert_eq!(elevation_at(&p, Vec2::new(10.0 + 5e-7, 0.0)), Some(2.0));
        assert_eq!(
            elevation_at(&p, Vec2::new(10.0, 5.0)),
            None,
            "an edge's middle"
        );
        let l = line(Some(1.5), Some(2.5));
        assert_eq!(elevation_at(&l, Vec2::new(30.0, 0.0)), Some(2.5));
        assert_eq!(elevation_at(&l, Vec2::new(0.0, 0.0)), Some(1.5));
        let a = area(None, some(&[5.0; 4]));
        assert_eq!(
            elevation_at(&a, Vec2::new(4.0, 4.0)),
            Some(5.0),
            "a hole's vertex"
        );
    }

    fn zs_of(g: &EntityGeometry) -> Vec<Option<f64>> {
        match g {
            EntityGeometry::Line { zs, .. }
            | EntityGeometry::Polyline { zs, .. }
            | EntityGeometry::Polygon { zs, .. } => zs.clone().expect("elevations written"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_change_writes_every_vertex_explicitly_holes_too() {
        let a = area(
            Some(vec![Some(1.0), None, Some(3.0), Some(4.0)]),
            some(&[5.0, 6.0, 7.0, 8.0]),
        );
        let set = geometry_with(&a, Change::Set(Some(100.5))).expect("an area");
        assert_eq!(zs_of(&set), vec![Some(100.5); 4]);
        let EntityGeometry::Polygon { holes, pts, .. } = &set else {
            panic!("{set:?}");
        };
        assert_eq!(pts.len(), 4, "the geometry is its own");
        assert_eq!(
            holes.as_ref().expect("the hole")[0].zs,
            Some(vec![Some(100.5); 4])
        );
        // Artır: those that have one are raised, one without stays without.
        let raised = geometry_with(&a, Change::Raise(2.5)).expect("an area");
        assert_eq!(zs_of(&raised), vec![Some(3.5), None, Some(5.5), Some(6.5)]);
        let EntityGeometry::Polygon { holes, .. } = &raised else {
            panic!("{raised:?}");
        };
        assert_eq!(
            holes.as_ref().expect("the hole")[0].zs,
            Some(vec![Some(7.5), Some(8.5), Some(9.5), Some(10.5)])
        );
        // Sıfırla: all of them written as none (not 0).
        let cleared = geometry_with(&a, Change::Set(None)).expect("an area");
        assert_eq!(zs_of(&cleared), vec![None; 4]);
    }

    #[test]
    fn a_line_s_end_changes_alone_and_a_point_takes_its_z() {
        let l = line(Some(10.0), Some(20.0));
        let g = geometry_with(&l, Change::End(1, Some(25.0))).expect("a line");
        assert_eq!(zs_of(&g), vec![Some(10.0), Some(25.0)]);
        let g = geometry_with(&l, Change::End(0, None)).expect("a line");
        assert_eq!(zs_of(&g), vec![None, Some(20.0)]);
        let point = |z| {
            Entity::Point(PointEntity {
                base: base(),
                p: pt(1.0, 2.0),
                z,
                parts: None,
            })
        };
        let raised = geometry_with(&point(Some(12.5)), Change::Raise(2.5)).expect("a point");
        assert_eq!(
            raised,
            EntityGeometry::Point {
                p: pt(1.0, 2.0),
                z: Some(15.0),
                parts: None
            }
        );
        let stays = geometry_with(&point(None), Change::Raise(2.5)).expect("a point");
        assert_eq!(
            stays,
            EntityGeometry::Point {
                p: pt(1.0, 2.0),
                z: None,
                parts: None
            }
        );
        // A circle takes none.
        let circle = Entity::Circle(kentos_contracts::CircleEntity {
            base: base(),
            c: pt(0.0, 0.0),
            r: 1.0,
        });
        assert!(geometry_with(&circle, Change::Set(Some(1.0))).is_none());
        assert!(!takes(&circle));
    }

    #[test]
    fn an_object_comes_to_its_rings_together_without_copying_them() {
        assert_eq!(Summary::of_object(&line(None, None)), Summary::None);
        assert_eq!(
            Summary::of_object(&line(Some(10.0), Some(20.0))),
            Summary::Range(10.0, 20.0)
        );
        assert_eq!(
            Summary::of_object(&line(Some(5.0), Some(5.0))),
            Summary::Value(5.0)
        );
        assert_eq!(
            Summary::of_object(&line(Some(5.0), None)),
            Summary::Partial(5.0, 5.0)
        );
        // No elevations at all: every vertex is missing.
        assert_eq!(Summary::of_object(&polyline(None)), Summary::None);
        assert_eq!(
            Summary::of_object(&polyline(Some(vec![Some(3.0), None, Some(1.0)]))),
            Summary::Partial(1.0, 3.0)
        );
        // An area: the ring and its hole together, the hole without any making it partial.
        let with_hole = area(some(&[1.0, 2.0, 3.0, 4.0]), some(&[5.0; 4]));
        assert_eq!(Summary::of_object(&with_hole), Summary::Range(1.0, 5.0));
        let bare_hole = area(some(&[1.0, 2.0, 3.0, 4.0]), None);
        assert_eq!(
            Summary::of_object(&bare_hole),
            Summary::Partial(1.0, 4.0),
            "a hole without elevations has vertices without one"
        );
        // The same list by the two ways.
        assert_eq!(
            Summary::of_object(&with_hole),
            Summary::of(&vertex_elevations(&with_hole))
        );
        let point = Entity::Point(PointEntity {
            base: base(),
            p: pt(0.0, 0.0),
            z: Some(2.0),
            parts: None,
        });
        assert_eq!(Summary::of_object(&point), Summary::Value(2.0));
    }

    #[test]
    fn several_objects_come_to_one_list_and_what_takes_none_adds_nothing() {
        let a = polyline(some(&[5.0; 3]));
        let b = line(Some(5.0), Some(5.0));
        let spot = Entity::Point(PointEntity {
            base: base(),
            p: pt(0.0, 0.0),
            z: Some(5.0),
            parts: None,
        });
        let circle = Entity::Circle(kentos_contracts::CircleEntity {
            base: base(),
            c: pt(0.0, 0.0),
            r: 1.0,
        });
        // The one value, though three objects say it: and a circle has no vertex.
        assert_eq!(
            Summary::of_objects([&a, &b, &spot, &circle]),
            Summary::Value(5.0)
        );
        // Two objects with the same range are not the one value: the list has a range.
        let r1 = polyline(some(&[1.0, 2.0, 3.0]));
        assert_eq!(Summary::of_objects([&r1, &r1]), Summary::Range(1.0, 3.0));
        // A point without a z is a vertex without an elevation.
        let bare = Entity::Point(PointEntity {
            base: base(),
            p: pt(0.0, 0.0),
            z: None,
            parts: None,
        });
        assert_eq!(Summary::of_objects([&a, &bare]), Summary::Partial(5.0, 5.0));
        assert_eq!(Summary::of_objects([&polyline(None), &bare]), Summary::None);
        assert_eq!(Summary::of_objects([&circle]), Summary::None);
    }

    /// The web's `parseElevation`: a number, a decimal comma and a trailing m allowed; nothing clears.
    #[test]
    fn a_typed_kot_is_a_number_nothing_or_not_taken() {
        assert_eq!(parse_typed("100.5"), Some(Some(100.5)));
        assert_eq!(parse_typed(" 112,5 "), Some(Some(112.5)));
        assert_eq!(parse_typed("-4.25 m"), Some(Some(-4.25)));
        assert_eq!(parse_typed("0"), Some(Some(0.0)));
        assert_eq!(parse_typed("12m"), Some(Some(12.0)));
        assert_eq!(parse_typed(""), Some(None));
        assert_eq!(parse_typed("   "), Some(None));
        assert_eq!(parse_typed("m"), Some(None));
        // Neither: the cell shows its value again.
        for text in [
            "kot yok", "yüz", "12abc", "1,2,3", "NaN", "Infinity", "@5,3",
        ] {
            assert_eq!(parse_typed(text), None, "{text}");
        }
    }

    #[test]
    fn a_change_that_leaves_everything_as_it_is_is_known() {
        let l = line(Some(10.0), Some(20.0));
        assert!(leaves(&l, Change::Raise(0.0)));
        assert!(!leaves(&l, Change::Raise(0.5)));
        assert!(leaves(&l, Change::End(0, Some(10.0))));
        assert!(!leaves(&l, Change::End(1, None)));
        assert!(!leaves(&l, Change::Set(Some(10.0))), "the far end changes");
        let bare = polyline(None);
        assert!(leaves(&bare, Change::Set(None)), "nothing to clear");
        assert!(leaves(&bare, Change::Raise(3.0)), "nothing to raise");
        assert!(!leaves(&bare, Change::Set(Some(0.0))), "0 is an elevation");
    }

    #[test]
    fn what_takes_elevations_and_what_has_any() {
        assert!(has_any(&line(None, Some(0.0))), "0 is an elevation");
        assert!(!has_any(&line(None, None)));
        assert!(has_any(&area(None, some(&[1.0; 4]))), "a hole's counts");
        assert!(!has_any(&polyline(None)));
        assert_eq!(
            vertex_elevations(&area(some(&[1.0; 4]), None)),
            vec![
                Some(1.0),
                Some(1.0),
                Some(1.0),
                Some(1.0),
                None,
                None,
                None,
                None
            ],
            "the ring, then the hole"
        );
        assert!(is_path(&polyline(None)) && takes(&polyline(None)));
    }
}
