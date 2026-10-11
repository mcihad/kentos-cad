//! The Geometri üretimi tools' calls for the web (`model/ops/generate.ts`):
//! whole lists in, every result out, one call a run.

use super::bounding::{Bound, Kind};
use super::points::is_curved;
use super::voronoi::{Cell, delaunay_triangles, frame_of, voronoi};
use super::{bound_shape, bounding_run, concave_run, hull_area, pole};
use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::entity::{Entity, Shape};
use crate::op;
use crate::vec2::Vec2;

/// A cell as the web reads it: its ring, `"same"` for a site in an earlier one's place, null for none.
struct CellOut(Cell);

impl ToJson for CellOut {
    fn write_json(&self, out: &mut String) {
        match &self.0 {
            Cell::Ring(r) => r.write_json(out),
            Cell::Same => "same".to_owned().write_json(out),
            Cell::Empty => out.push_str("null"),
        }
    }
}

/// A run's results, or why it was refused: `{ list }` or `{ refused }`.
struct Run<T: ToJson>(Result<T, String>, &'static str);

impl<T: ToJson> ToJson for Run<T> {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(v) => field(out, &mut first, self.1, v),
            Err(why) => field(out, &mut first, "refused", why),
        }
        out.push('}');
    }
}

/// A bounding geometry as the web reads it: its shape and measures.
struct BoundOut(Bound);

impl ToJson for BoundOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "shape", &Entity::new(bound_shape(&self.0)));
        match &self.0 {
            Bound::Hull { area, .. } => field(out, &mut first, "area", area),
            Bound::Box { width, height, .. } => {
                field(out, &mut first, "width", width);
                field(out, &mut first, "height", height);
            }
            Bound::Rotated {
                length,
                width,
                bearing,
                ..
            } => {
                field(out, &mut first, "length", length);
                field(out, &mut first, "width", width);
                field(out, &mut first, "bearing", bearing);
            }
            Bound::Circle { r, .. } => field(out, &mut first, "r", r),
        }
        out.push('}');
    }
}

/// A concave hull as the web reads it: its area shape and net area.
struct HullOut(Shape, f64);

impl ToJson for HullOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "shape", &Entity::new(self.0.clone()));
        field(out, &mut first, "area", &self.1);
        out.push('}');
    }
}

/// A pole as the web reads it: `{ p, d }`.
struct PoleOut(Vec2, f64);

impl ToJson for PoleOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "p", &self.0);
        field(out, &mut first, "d", &self.1);
        out.push('}');
    }
}

/// A list of results and how many objects entered with curves taken as points.
struct Listed<T: ToJson>(Vec<Option<T>>, usize);

impl<T: ToJson> ToJson for Listed<T> {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "list", &self.0);
        field(out, &mut first, "curved", &(self.1 as f64));
        out.push('}');
    }
}

fn shapes(entities: &[Entity]) -> Vec<Shape> {
    entities.iter().map(|e| e.shape.clone()).collect()
}

pub(crate) static OPS: &[Op] = &[
    // Voronoi alanları (§2): each site's cell in the frame grown by `margin` (a fraction).
    op!("generateVoronoi", |sites: Vec<Vec2>, margin: f64| {
        let cells = match frame_of(&sites, margin) {
            Some(f) => voronoi(&sites, f).map(|c| c.into_iter().map(CellOut).collect::<Vec<_>>()),
            None => Err(super::voronoi::TOO_FEW_SITES.to_owned()),
        };
        Run(cells, "cells")
    }),
    // Delaunay üçgenleri (§3): the triangles as the sites' numbers.
    op!("generateDelaunay", |sites: Vec<Vec2>| {
        Run(
            delaunay_triangles(&sites).map(|t| {
                t.into_iter()
                    .map(|[a, b, c]| vec![a as f64, b as f64, c as f64])
                    .collect::<Vec<_>>()
            }),
            "triangles",
        )
    }),
    // Sınırlayıcı geometri (§4): each group's (objects' groups below `count`).
    op!(
        "generateBounding",
        |entities: Vec<Entity>, groups: Vec<usize>, count: usize, kind: String| {
            let Some(kind) = Kind::from_key(&kind) else {
                return Err(format!("Bilinmeyen sınırlayıcı geometri: {kind}"));
            };
            let s = shapes(&entities);
            let curved = s.iter().filter(|x| is_curved(x)).count();
            let list = bounding_run(&s, &groups, count, kind)
                .into_iter()
                .map(|b| b.map(BoundOut))
                .collect();
            Listed(list, curved)
        }
    ),
    // İçbükey zarf (§5): each group's.
    op!("generateConcave", |entities: Vec<Entity>,
                            groups: Vec<usize>,
                            count: usize,
                            ratio: f64,
                            holes: bool| {
        let s = shapes(&entities);
        let curved = s.iter().filter(|x| is_curved(x)).count();
        let list = concave_run(&s, &groups, count, ratio, holes)
            .into_iter()
            .map(|r| {
                r.and_then(|rings| hull_area(&rings))
                    .map(|(shape, a)| HullOut(shape, a))
            })
            .collect();
        Listed(list, curved)
    }),
    // Ulaşılmazlık kutbu (§6): each object's.
    op!("generatePoles", |entities: Vec<Entity>, precision: f64| {
        let curved = entities
            .iter()
            .filter(|e| !matches!(e.shape, Shape::Circle { .. }) && is_curved(&e.shape))
            .count();
        let list = entities
            .iter()
            .map(|e| pole::pole(&e.shape, precision).map(|(p, d)| PoleOut(p, d)))
            .collect();
        Listed(list, curved)
    }),
];
