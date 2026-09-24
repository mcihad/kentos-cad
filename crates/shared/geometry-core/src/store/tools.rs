//! Tool previews and totals on the store (docs/adr/0008, S1c): trim and
//! extend against the visible edges (or chosen boundaries) in one call,
//! the outlines of moved, copied, stretched or pasted objects, and the
//! selection's total length and area. The move, copy and paste themselves
//! come from here too, packed (`transform_packed`).
//!
//! Trim and extend are the ported `trim_entity` / `extend_entity`; only the
//! boundaries they get are fewer. Their result depends on which boundary
//! edges meet the target, never on the order or on edges that miss it
//! (cuts are sorted and merged, reaches are minimised), so the store hands
//! them just the edges whose boxes come near: the target's own edges for a
//! trim, the ray or circle an end would grow along for an extend. With a
//! million contour segments in view the TypeScript intersected every one
//! with every edge of the target on every frame.

use super::rtree::{PackedTree, overlaps};
use super::{Item, Packer, Store};
use crate::entity::{
    Entity, Shape, dimension_geom, ellipse_geom, entity_area, entity_length, entity_outline,
    polygon_ring,
};
use crate::geom::affine::Affine;
use crate::geom::bulge::{bulge_arc, bulge_at};
use crate::geom::dimension::layout_dimension;
use crate::geom::ellipse::minor_axis;
use crate::geom::intersect::Edge;
use crate::geometry::{Bounds, empty_bounds, extend_bounds};
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::ops::curve_cuts::{Cut, Geometry};
use crate::ops::edges::entity_edges;
use crate::ops::stretch::stretch_entity;
use crate::ops::transform::transform_shape;
use crate::ops::trim::{extend_entity, trim_entity};
use crate::vec2::Vec2;

/// A box widened for the intersection tests' tolerances (1e-9 of an edge's
/// parameter, a micrometre-scale margin besides): a boundary that could
/// still count as touching is never left out.
fn grow(b: Bounds) -> Bounds {
    let size = (b.max_x - b.min_x).abs() + (b.max_y - b.min_y).abs();
    let m = 1e-6
        + 1e-8 * size
        + 1e-12 * (b.min_x.abs() + b.min_y.abs() + b.max_x.abs() + b.max_y.abs());
    Bounds {
        min_x: b.min_x - m,
        min_y: b.min_y - m,
        max_x: b.max_x + m,
        max_y: b.max_y + m,
    }
}

/// An edge's box; an arc's whole circle (enough to rule it out).
fn edge_box(e: &Edge) -> Bounds {
    let mut b = empty_bounds();
    match *e {
        Edge::Seg { a, b: q } => {
            extend_bounds(&mut b, a, 0.0);
            extend_bounds(&mut b, q, 0.0);
        }
        Edge::Arc { c, r, .. } => extend_bounds(&mut b, c, r.abs()),
    }
    grow(b)
}

/// Whether a box can be hit by the ray `o + t·dir`, t ≥ 0 (slab test).
fn ray_meets(o: Vec2, dir: Vec2, b: &Bounds) -> bool {
    let mut lo = 0.0_f64;
    let mut hi = f64::INFINITY;
    for (p, d, min, max) in [
        (o.x, dir.x, b.min_x, b.max_x),
        (o.y, dir.y, b.min_y, b.max_y),
    ] {
        if d.abs() < 1e-300 {
            if p < min || p > max {
                return false;
            }
            continue;
        }
        let t1 = (min - p) / d;
        let t2 = (max - p) / d;
        lo = js_max(lo, js_min(t1, t2));
        hi = js_min(hi, js_max(t1, t2));
    }
    // NaN (a degenerate box or ray) keeps the edge: the exact test decides.
    !(lo > hi)
}

/// Where an extended end can go: along a ray, within a box (a circle or an
/// ellipse it grows along), or nowhere (the target cannot be extended).
enum Reach {
    Ray(Vec2, Vec2),
    Area(Bounds),
    Nowhere,
}

impl Reach {
    fn may_meet(&self, b: &Bounds) -> bool {
        match self {
            Reach::Ray(o, dir) => ray_meets(*o, *dir, b),
            Reach::Area(a) => overlaps(a, b),
            Reach::Nowhere => false,
        }
    }
}

fn circle_box(c: Vec2, r: f64) -> Bounds {
    let mut b = empty_bounds();
    extend_bounds(&mut b, c, r.abs());
    grow(b)
}

/// The region `extend_entity` searches for boundaries, taking the same
/// branches: the end nearest the pick, an arc end along its circle, a
/// straight end along its direction.
fn reach_of(target: &Shape, pick: Vec2) -> Reach {
    match target {
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let minor = minor_axis(&ellipse_geom(*c, *major, *ratio, *t0, *t1));
            Reach::Area(circle_box(
                *c,
                js_hypot(major.x, major.y) + js_hypot(minor.x, minor.y),
            ))
        }
        Shape::Line { .. } | Shape::Polyline { .. } => {
            let (pts, bulges): (Vec<Vec2>, Option<&[f64]>) = match target {
                Shape::Line { a, b } => (vec![*a, *b], None),
                Shape::Polyline { pts, bulges, .. } => (pts.clone(), bulges.as_deref()),
                _ => return Reach::Nowhere,
            };
            let n = pts.len();
            if n < 2 {
                return Reach::Nowhere;
            }
            let at_end = js_hypot(pick.x - pts[n - 1].x, pick.y - pts[n - 1].y)
                <= js_hypot(pick.x - pts[0].x, pick.y - pts[0].y);
            let seg = if at_end { n - 2 } else { 0 };
            if let Some(arc) = bulge_arc(pts[seg], pts[seg + 1], bulge_at(bulges, seg)) {
                return Reach::Area(circle_box(arc.c, arc.r));
            }
            let (tip, prev) = if at_end {
                (pts[n - 1], pts[n - 2])
            } else {
                (pts[0], pts[1])
            };
            let len = js_hypot(tip.x - prev.x, tip.y - prev.y);
            if len < 1e-12 {
                return Reach::Nowhere;
            }
            Reach::Ray(
                tip,
                Vec2::new((tip.x - prev.x) / len, (tip.y - prev.y) / len),
            )
        }
        Shape::Arc { c, r, .. } => Reach::Area(circle_box(*c, *r)),
        _ => Reach::Nowhere,
    }
}

/// An object whose own box says nothing about its edges: infinite lines and
/// boxes the tree cannot hold. Their edges are always looked at.
fn loose_box(it: &Item) -> bool {
    it.special()
}

/// Paths to stroke for an object's outline, as `strokeGeometry` draws it
/// (`apps/web/src/tools/preview.ts`): `flags, n, x0, y0, …` per path, flags 0 open,
/// 1 closed, 2 a marker (points and text: one point, drawn as a square).
pub fn outline_paths(s: &Shape, out: &mut Vec<f64>) {
    let path = |out: &mut Vec<f64>, flags: f64, pts: &[Vec2]| {
        out.push(flags);
        out.push(pts.len() as f64);
        for p in pts {
            out.extend([p.x, p.y]);
        }
    };
    match s {
        Shape::Point { p, .. } | Shape::Text { p, .. } => path(out, 2.0, &[*p]),
        Shape::Dimension { .. } => {
            if let Some(l) = dimension_geom(s).and_then(|g| layout_dimension(&g)) {
                for [a, b] in l.lines {
                    path(out, 0.0, &[a, b]);
                }
            }
        }
        _ => {
            let closed = matches!(s, Shape::Polygon { .. } | Shape::Circle { .. });
            path(
                out,
                if closed { 1.0 } else { 0.0 },
                &entity_outline(s, 64.0),
            );
            if let Shape::Polygon {
                holes: Some(holes), ..
            } = s
            {
                for h in holes {
                    path(out, 1.0, &polygon_ring(&h.pts, h.bulges.as_deref()));
                }
            }
        }
    }
}

impl Store {
    /// Boundaries as the TypeScript gathered them: the chosen objects
    /// (hidden ones too), or every visible object overlapping `view`; the
    /// target left out.
    fn boundary_items(
        &self,
        except: Option<f64>,
        chosen: Option<&[f64]>,
        view: &Bounds,
    ) -> Vec<&Item> {
        match chosen {
            Some(ids) => ids
                .iter()
                .filter(|&&id| Some(id) != except)
                .filter_map(|&id| self.get(id))
                .collect(),
            None => self.overlapping(view, except),
        }
    }

    /// `trimEntity(target, pick, boundaries)` with the boundaries the trim
    /// tool would pass (see `boundary_items`), cut down to the edges near
    /// the target's own.
    pub fn trim_preview(
        &self,
        target: &Entity,
        pick: Vec2,
        except: Option<f64>,
        chosen: Option<&[f64]>,
        view: &Bounds,
    ) -> Cut {
        let own: Vec<(u32, Bounds)> = match &target.shape {
            // Crossings are found on the true curve: take the whole ellipse's box.
            Shape::Ellipse { .. } => match reach_of(&target.shape, pick) {
                Reach::Area(b) => vec![(0, b)],
                _ => Vec::new(),
            },
            s => entity_edges(s)
                .iter()
                .enumerate()
                .map(|(i, e)| (i as u32, edge_box(e)))
                .collect(),
        };
        let mut whole = empty_bounds();
        for (_, b) in &own {
            extend_bounds(&mut whole, Vec2::new(b.min_x, b.min_y), 0.0);
            extend_bounds(&mut whole, Vec2::new(b.max_x, b.max_y), 0.0);
        }
        let near = PackedTree::build(&own);
        let mut hits = Vec::new();
        let mut edges = Vec::new();
        for it in self.boundary_items(except, chosen, view) {
            if !loose_box(it) && !overlaps(&it.bounds, &whole) {
                continue;
            }
            for ed in entity_edges(&it.shape) {
                hits.clear();
                near.search(&edge_box(&ed), &mut hits);
                if !hits.is_empty() {
                    edges.push(ed);
                }
            }
        }
        trim_entity(target, pick, &edges)
    }

    /// `extendEntity(target, pick, boundaries)` with the boundaries the
    /// extend tool would pass, cut down to the edges the end can reach.
    pub fn extend_preview(
        &self,
        target: &Entity,
        pick: Vec2,
        except: Option<f64>,
        chosen: Option<&[f64]>,
        view: &Bounds,
    ) -> Geometry {
        let reach = reach_of(&target.shape, pick);
        let mut edges = Vec::new();
        for it in self.boundary_items(except, chosen, view) {
            if !loose_box(it) && !reach.may_meet(&grow(it.bounds)) {
                continue;
            }
            for ed in entity_edges(&it.shape) {
                if reach.may_meet(&edge_box(&ed)) {
                    edges.push(ed);
                }
            }
        }
        extend_entity(target, pick, &edges)
    }

    /// Outlines of objects moved by each affine in turn (ghosts of move,
    /// copy, rotate, scale, mirror, arrays, paste): at most `limit` + 1
    /// objects, counted as `SelectionFirstTool.draw` counts them. Unknown
    /// ids are skipped. See `outline_paths` for the layout.
    pub fn transform_outlines(&self, ids: &[f64], affines: &[Affine], limit: usize) -> Vec<f64> {
        let mut out = Vec::new();
        let mut drawn = 0usize;
        for m in affines {
            for &id in ids {
                let Some(it) = self.get(id) else { continue };
                let n = drawn;
                drawn += 1;
                if n > limit {
                    break;
                }
                outline_paths(&transform_shape(&it.shape, m), &mut out);
            }
        }
        out
    }

    /// Objects moved by each affine in turn, as `transformEntities` gives
    /// them, packed as `put_packed` reads them (`Packer`): move, copy,
    /// rotate, scale, mirror, arrays and paste transform the store's own
    /// copies, and nothing crosses as JSON. Records come affine after
    /// affine, in the order of `ids`; unknown ids are skipped.
    pub fn transform_packed(&self, ids: &[f64], affines: &[Affine]) -> Packer {
        let names = self.layer_names();
        let mut out = Packer::default();
        for m in affines {
            for &id in ids {
                let Some(it) = self.get(id) else { continue };
                let layer = names.get(it.layer as usize).copied().unwrap_or("");
                out.object(it.id, layer, it.label, &transform_shape(&it.shape, m));
            }
        }
        out
    }

    /// Outlines of objects stretched by a window and a displacement
    /// (`stretchEntity`); objects the window leaves alone are skipped.
    pub fn stretch_outlines(&self, ids: &[f64], window: &Bounds, dx: f64, dy: f64) -> Vec<f64> {
        let mut out = Vec::new();
        for &id in ids {
            let Some(it) = self.get(id) else { continue };
            if let Some(g) = stretch_entity(&Entity::new(it.shape.clone()), window, dx, dy) {
                outline_paths(&g.shape, &mut out);
            }
        }
        out
    }

    /// Total length (polygons' perimeters left out) and total area of these
    /// objects, summed in this order as the properties panel summed them.
    pub fn measure(&self, ids: &[f64]) -> (f64, f64) {
        let mut length = 0.0;
        let mut area = 0.0;
        for &id in ids {
            let Some(it) = self.get(id) else { continue };
            length += if matches!(it.shape, Shape::Polygon { .. }) {
                0.0
            } else {
                entity_length(&it.shape).unwrap_or(0.0)
            };
            area += entity_area(&it.shape).unwrap_or(0.0);
        }
        (length, area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::FromJson;
    use crate::api::json::Json;

    fn entity(json: &str) -> Entity {
        Entity::from_json(&Json::parse(json).unwrap()).unwrap()
    }

    fn view() -> Bounds {
        Bounds {
            min_x: -100.0,
            min_y: -100.0,
            max_x: 100.0,
            max_y: 100.0,
        }
    }

    #[test]
    fn trims_and_extends_with_only_the_edges_near() {
        let mut s = Store::new();
        let mut lines = Vec::new();
        // Vertical walls at x = 0, 10, …, 90 and a horizontal target line.
        for i in 0..10 {
            let x = f64::from(i) * 10.0;
            lines.push(format!(
                r#"{{"id":{},"layerId":"a","kind":"line","a":{{"x":{x},"y":-5}},"b":{{"x":{x},"y":5}}}}"#,
                i + 1
            ));
        }
        lines.push(
            r#"{"id":99,"layerId":"a","kind":"line","a":{"x":-20,"y":0},"b":{"x":55,"y":0}}"#
                .into(),
        );
        s.put_json(&format!("[{}]", lines.join(","))).unwrap();
        let target = entity(
            r#"{"id":99,"layerId":"a","kind":"line","a":{"x":-20,"y":0},"b":{"x":55,"y":0}}"#,
        );
        let all: Vec<Edge> = s.edges_in(&view(), Some(99.0));
        let json = |v: &dyn crate::api::json::ToJson| {
            let mut out = String::new();
            v.write_json(&mut out);
            out
        };
        // Trim between the walls at 20 and 30.
        let want = trim_entity(&target, Vec2::new(25.0, 0.0), &all);
        let got = s.trim_preview(&target, Vec2::new(25.0, 0.0), Some(99.0), None, &view());
        assert_eq!(json(&got), json(&want));
        assert!(matches!(want, Cut::Pieces(ref p) if p.len() == 2));
        // Extend the right end to the wall at 60, or only to chosen walls.
        let want = extend_entity(&target, Vec2::new(54.0, 0.0), &all);
        let got = s.extend_preview(&target, Vec2::new(54.0, 0.0), Some(99.0), None, &view());
        assert_eq!(json(&got), json(&want));
        let chosen = [9.0, 10.0];
        let got = s.extend_preview(
            &target,
            Vec2::new(54.0, 0.0),
            Some(99.0),
            Some(&chosen),
            &view(),
        );
        match got {
            Geometry::Ok(e) => assert!(matches!(e.shape, Shape::Line { b, .. } if b.x == 80.0)),
            Geometry::Error(e) => panic!("{e}"),
        }
    }

    #[test]
    fn outlines_ghosts_and_totals() {
        let mut s = Store::new();
        s.put_json(
            r#"[{"id":1,"layerId":"a","kind":"line","a":{"x":0,"y":0},"b":{"x":3,"y":4}},
                {"id":2,"layerId":"a","kind":"polygon","pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4},{"x":0,"y":4}],"holes":[{"pts":[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]}]},
                {"id":3,"layerId":"a","kind":"point","p":{"x":7,"y":7}}]"#,
        )
        .unwrap();
        let shift: Affine = [1.0, 0.0, 0.0, 1.0, 10.0, 0.0];
        let g = s.transform_outlines(&[1.0, 3.0, 42.0], &[shift], 400);
        assert_eq!(g, [0.0, 2.0, 10.0, 0.0, 13.0, 4.0, 2.0, 1.0, 17.0, 7.0]);
        // The limit counts as the tool counted: one more than the limit is drawn.
        let two = s.transform_outlines(&[1.0, 1.0, 1.0], &[shift, shift], 1);
        assert_eq!(two.len(), 2 * 6);
        let mut poly = Vec::new();
        outline_paths(&s.get(2.0).unwrap().shape, &mut poly);
        assert_eq!((poly[0], poly[1], poly[10], poly[11]), (1.0, 4.0, 1.0, 3.0));
        let w = Bounds {
            min_x: 2.0,
            min_y: 3.0,
            max_x: 5.0,
            max_y: 5.0,
        };
        assert_eq!(
            s.stretch_outlines(&[1.0], &w, 1.0, 0.0),
            [0.0, 2.0, 0.0, 0.0, 4.0, 4.0]
        );
        assert_eq!(s.measure(&[1.0, 2.0, 3.0, 5.0]), (5.0, 16.0 - 0.5));
    }

    /// Objects of every kind in a TM zone, with the fields the core does not
    /// read (attributes, colour, symbol, label) and optional geometry given
    /// and left out.
    const TM_OBJECTS: &str = r##"[
        {"id":1,"layerId":"parsel","attrs":{"Ada":"104","Parsel":"7"},"label":"7","kind":"polygon","pts":[{"x":486512.34,"y":4420187.52},{"x":486535.757,"y":4420188.723},{"x":486538.221,"y":4420218.986},{"x":486514.344,"y":4420220.532}],"bulges":[0,0.25,0,-0.1],"holes":[{"pts":[{"x":486520,"y":4420195},{"x":486525,"y":4420195},{"x":486522,"y":4420200}],"bulges":[0,0,0.3]}]},
        {"id":2,"layerId":"parsel","attrs":{},"kind":"polygon","pts":[{"x":486500.1,"y":4420100.2},{"x":486540.35,"y":4420101.9},{"x":486538.8,"y":4420141.15}]},
        {"id":3,"layerId":"yol","attrs":{},"color":"#aa3322","kind":"polyline","pts":[{"x":486500.1,"y":4420100.2},{"x":486540.35,"y":4420101.9},{"x":486538.8,"y":4420141.15}],"bulges":[0.5,0,0]},
        {"id":4,"layerId":"yol","attrs":{},"kind":"polyline","pts":[{"x":486501,"y":4420102},{"x":486503,"y":4420109}]},
        {"id":5,"layerId":"kot","attrs":{"Ad":"P1"},"label":"P1","kind":"point","p":{"x":486512.345,"y":4420187.521},"z":850.25},
        {"id":6,"layerId":"kot","attrs":{},"kind":"point","p":{"x":486513,"y":4420188}},
        {"id":7,"layerId":"taslak","attrs":{},"kind":"line","a":{"x":486500.1,"y":4420100.2},"b":{"x":486512.34,"y":4420187.52}},
        {"id":8,"layerId":"taslak","attrs":{},"kind":"circle","c":{"x":486520,"y":4420200},"r":12.5},
        {"id":9,"layerId":"taslak","attrs":{},"kind":"arc","c":{"x":486520,"y":4420200},"r":12.5,"a0":0.3,"a1":2.1},
        {"id":10,"layerId":"taslak","attrs":{},"kind":"ellipse","c":{"x":486520,"y":4420200},"major":{"x":10,"y":-3},"ratio":0.4,"t0":0.5,"t1":2.5},
        {"id":11,"layerId":"taslak","attrs":{},"kind":"ellipse","c":{"x":486520,"y":4420200},"major":{"x":10,"y":-3},"ratio":0.4,"t0":1,"t1":1},
        {"id":12,"layerId":"taslak","attrs":{},"kind":"xline","p":{"x":486520,"y":4420200},"dir":{"x":0.6,"y":0.8}},
        {"id":13,"layerId":"taslak","attrs":{},"kind":"ray","p":{"x":486520,"y":4420200},"dir":{"x":-1,"y":0}},
        {"id":14,"layerId":"taslak","attrs":{},"symbol":"mpyy:konut","kind":"spline","pts":[{"x":486520,"y":4420200},{"x":486530,"y":4420210},{"x":486540,"y":4420205}],"closed":false},
        {"id":15,"layerId":"yazi","attrs":{},"kind":"text","p":{"x":486520,"y":4420200},"text":"Ada 104 😀","height":2,"rotation":-30},
        {"id":16,"layerId":"olcu","attrs":{},"kind":"dimension","a":{"x":486520,"y":4420200},"b":{"x":486530,"y":4420207},"offset":2,"height":0.5},
        {"id":17,"layerId":"olcu","attrs":{},"kind":"dimension","a":{"x":486520,"y":4420200},"b":{"x":486530,"y":4420207},"offset":-2,"height":0.5,"style":"linear"},
        {"id":18,"layerId":"olcu","attrs":{},"kind":"dimension","a":{"x":486520,"y":4420200},"b":{"x":486530,"y":4420207},"offset":-2,"height":0.5,"text":"12,5 m","style":"linear","angle":90},
        {"id":19,"layerId":"olcu","attrs":{},"kind":"dimension","a":{"x":486530,"y":4420200},"b":{"x":486520,"y":4420210},"offset":5,"height":1,"style":"angular","c":{"x":486520,"y":4420200}},
        {"id":20,"layerId":"olcu","attrs":{},"kind":"dimension","a":{"x":486520,"y":4420200},"b":{"x":486530,"y":4420200},"offset":1,"height":1,"style":"radius"},
        {"id":21,"layerId":"tarama","attrs":{},"kind":"hatch","ring":[{"x":486520,"y":4420200},{"x":486530,"y":4420200},{"x":486525,"y":4420210}],"holes":[[{"x":486524,"y":4420202},{"x":486526,"y":4420202},{"x":486525,"y":4420204}]],"pattern":{"type":"lines","angle":45,"spacing":1}},
        {"id":22,"layerId":"tarama","attrs":{},"kind":"hatch","ring":[{"x":486520,"y":4420200},{"x":486530,"y":4420200},{"x":486525,"y":4420210}],"pattern":{"type":"solid","angle":0,"spacing":1}}
    ]"##;

    #[test]
    fn transforms_packed_as_the_json_call_does() {
        use super::super::pack::unpack;
        use crate::api::json::{self, FromJson, Json};
        use crate::api::run_named;
        use crate::geom::affine::{IDENTITY, compose, mirror, rotation, scaling, translation};

        let mut s = Store::new();
        s.put_json(TM_OBJECTS).unwrap();
        let all = Vec::<Entity>::from_json(&Json::parse(TM_OBJECTS).unwrap()).unwrap();
        let o = Vec2::new(486520.0, 4420200.0);
        let affines: Vec<Affine> = vec![
            translation(12.5, -7.25),
            rotation(0.3, o),
            rotation(3.0, o),
            scaling(2.5, o),
            // Non-uniform: the tools never make one, the transform still must not differ.
            [1.5, 0.0, 0.0, 0.5, o.x * (1.0 - 1.5), o.y * (1.0 - 0.5)],
            mirror(o, Vec2::new(486530.0, 4420210.0)),
            mirror(o, Vec2::new(486520.0, 4420260.0)),
            compose(
                &rotation(-1.1, o),
                &mirror(o, Vec2::new(486525.0, 4420200.0)),
            ),
            IDENTITY,
        ];
        // Any order the caller asks in; an id the store does not hold is left out.
        let ids: Vec<f64> = (1..=all.len())
            .rev()
            .map(|i| i as f64)
            .chain([999.0])
            .collect();
        let out = s.transform_packed(&ids, &affines);
        let got = unpack(&out.nums, &out.strings).unwrap();

        let list: Vec<Entity> = ids
            .iter()
            .filter_map(|&id| all.get(id as usize - 1).cloned())
            .collect();
        let args = format!("[{},{}]", json::to_string(&list), json::to_string(&affines));
        let want = run_named("transformEntities", &args).unwrap();
        let want = Vec::<Entity>::from_json(&Json::parse(&want).unwrap()).unwrap();
        assert_eq!(got.len(), list.len() * affines.len());
        assert_eq!(want.len(), got.len());
        for (k, ((id, layer, label, shape), w)) in got.iter().zip(&want).enumerate() {
            let e = &list[k % list.len()];
            let field = |name: &str| e.rest.iter().find(|(n, _)| n == name).map(|(_, v)| v);
            assert_eq!(Some(&Json::Num(*id)), field("id"), "{k}");
            assert_eq!(Some(&Json::Str(layer.clone())), field("layerId"), "{k}");
            assert_eq!(*label, field("label").is_some(), "{k}");
            // The same geometry, number for number: the JSON writer tells −0, NaN and every bit apart.
            assert_eq!(json::to_string(shape), json::to_string(&w.shape), "{k}");
            assert_eq!(*shape, transform_shape(&e.shape, &affines[k / list.len()]));
            // The JSON call hands every other field back as it came.
            assert_eq!(w.rest, e.rest, "{k}");
        }
        assert!(s.transform_packed(&[], &affines).nums.is_empty());
        assert!(s.transform_packed(&ids, &[]).nums.is_empty());
    }
}
