//! The Dönüştürme tools' calls for the web (`model/ops/convert.ts`): whole
//! lists in, each object's results out, so a run crosses into the core once
//! per tool. The desktop's tools call the functions with shapes.

use super::paths::{Density, NoArea, densify, densify_added, line_area, rings_as_lines, substring};
use super::points::{Which, centroid, interior_point, vertex_points};
use super::table::{RowFault, bearing_rows, ellipse_rows, xy_rows};
use super::{Elevations, Zs};
use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::entity::{Entity, Shape, area_parts};
use crate::op;
use crate::vec2::Vec2;

/// A shape and its elevations as the web reads them: `{ shape, zs? }`.
struct Out(Elevations);

impl ToJson for Out {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "shape", &Entity::new(self.0.shape.clone()));
        field(out, &mut first, "zs", &self.0.zs);
        out.push('}');
    }
}

/// A ring as a line with its part and ring.
struct RingOut(super::paths::RingLine);

impl ToJson for RingOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(
            out,
            &mut first,
            "shape",
            &Entity::new(self.0.line.shape.clone()),
        );
        field(out, &mut first, "zs", &self.0.line.zs);
        field(out, &mut first, "part", &self.0.part);
        field(out, &mut first, "ring", &self.0.ring);
        out.push('}');
    }
}

/// A line's area or why it has none (`open`, `empty`, `parts`).
struct AreaOut(Result<Elevations, NoArea>);

impl ToJson for AreaOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(e) => {
                field(out, &mut first, "shape", &Entity::new(e.shape.clone()));
                field(out, &mut first, "zs", &e.zs);
            }
            Err(why) => {
                let why = match why {
                    NoArea::Open => "open",
                    NoArea::Empty => "empty",
                    NoArea::MultiPart => "parts",
                };
                field(out, &mut first, "none", &why.to_owned());
            }
        }
        out.push('}');
    }
}

/// A densified object and the vertices added to it.
struct DenseOut(Elevations, u64);

impl ToJson for DenseOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "shape", &Entity::new(self.0.shape.clone()));
        field(out, &mut first, "zs", &self.0.zs);
        field(out, &mut first, "added", &(self.1 as f64));
        out.push('}');
    }
}

fn zs_at(zs: &[Option<Zs>], i: usize) -> Option<&Zs> {
    zs.get(i).and_then(Option::as_ref)
}

pub(crate) static OPS: &[Op] = &[
    // Nesneden nokta: each object's point (each part's with `parts`); none where it has none.
    op!("convertPoints", |entities: Vec<Entity>,
                          inside: bool,
                          parts: bool| {
        let at = |s: &crate::entity::Shape| {
            if inside {
                interior_point(s)
            } else {
                centroid(s)
            }
        };
        entities
            .iter()
            .map(|e| {
                if parts {
                    area_parts(&e.shape)
                        .iter()
                        .filter_map(at)
                        .collect::<Vec<Vec2>>()
                } else {
                    at(&e.shape).into_iter().collect()
                }
            })
            .collect::<Vec<_>>()
    }),
    op!(
        "convertVertices",
        |entities: Vec<Entity>, zs: Vec<Option<Zs>>, which: Which| {
            entities
                .iter()
                .enumerate()
                .map(|(i, e)| vertex_points(&e.shape, zs_at(&zs, i), which))
                .collect::<Vec<_>>()
        }
    ),
    // Çizginin parçası: each object's piece between its two distances (none where a distance is no number).
    op!(
        "convertSubstring",
        |entities: Vec<Entity>,
         zs: Vec<Option<Zs>>,
         starts: Vec<Option<f64>>,
         ends: Vec<Option<f64>>| {
            entities
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let (s0, s1) = (
                        starts.get(i).copied().flatten()?,
                        ends.get(i).copied().flatten()?,
                    );
                    substring(&e.shape, zs_at(&zs, i), s0, s1).map(Out)
                })
                .collect::<Vec<_>>()
        }
    ),
    op!(
        "convertRings",
        |entities: Vec<Entity>, zs: Vec<Option<Zs>>| {
            entities
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    rings_as_lines(&e.shape, zs_at(&zs, i))
                        .into_iter()
                        .map(RingOut)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        }
    ),
    op!(
        "convertLineAreas",
        |entities: Vec<Entity>, zs: Vec<Option<Zs>>, close: bool| {
            entities
                .iter()
                .enumerate()
                .map(|(i, e)| AreaOut(line_area(&e.shape, zs_at(&zs, i), close)))
                .collect::<Vec<_>>()
        }
    ),
    // Köşe ekle: how many vertices it would add, read before anything is built.
    op!(
        "convertDensifyCount",
        |entities: Vec<Entity>, by: Density| {
            entities
                .iter()
                .map(|e| densify_added(&e.shape, by))
                .sum::<u64>() as f64
        }
    ),
    op!("convertDensify", |entities: Vec<Entity>,
                           zs: Vec<Option<Zs>>,
                           by: Density| {
        entities
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let (shape, added) = densify(&e.shape, zs_at(&zs, i), by);
                DenseOut(shape, added)
            })
            .collect::<Vec<_>>()
    }),
    // Semt ve uzunluktan çizgi: rows of east, north, bearing and length cells.
    op!(
        "convertBearingRows",
        |rows: Vec<Vec<String>>, unit: String| {
            bearing_rows(&rows, &unit)
                .into_iter()
                .map(|r| RowOut(r.map(|[a, b]| Entity::new(Shape::Line { a, b }))))
                .collect::<Vec<_>>()
        }
    ),
    // XY'den çizgi: rows of the start's and the end's east and north cells.
    op!("convertXyRows", |rows: Vec<Vec<String>>| {
        xy_rows(&rows)
            .into_iter()
            .map(|r| RowOut(r.map(|[a, b]| Entity::new(Shape::Line { a, b }))))
            .collect::<Vec<_>>()
    }),
    // Tablodan elips: rows of east, north, the two semi-axes and the bearing cells.
    op!(
        "convertEllipseRows",
        |rows: Vec<Vec<String>>, unit: String| {
            ellipse_rows(&rows, &unit)
                .into_iter()
                .map(|r| RowOut(r.map(Entity::new)))
                .collect::<Vec<_>>()
        }
    ),
];

/// A row's object, or why it has none: `{ fault, column? }` (`empty`,
/// `number`, `positive` with the cell's place in the row; `same`).
struct RowOut(Result<Entity, RowFault>);

impl ToJson for RowOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(e) => field(out, &mut first, "shape", e),
            Err(fault) => {
                let (why, column) = match *fault {
                    RowFault::Empty(k) => ("empty", Some(k)),
                    RowFault::NotNumber(k) => ("number", Some(k)),
                    RowFault::NotPositive(k) => ("positive", Some(k)),
                    RowFault::SamePlace => ("same", None),
                };
                field(out, &mut first, "fault", &why.to_owned());
                field(out, &mut first, "column", &column);
            }
        }
        out.push('}');
    }
}
