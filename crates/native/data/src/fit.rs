//! A geometry fitted to the column it is written to (docs/adr/9100 §5): a single part made a multi of one for a multi
//! column, a multi of one made its part for a single one; anything else is left for the source to accept or refuse.

use crate::geom::Geometry;

/// `g` as the column `type_name` (OGC's name, any case) takes it.
pub fn fit(g: Geometry, type_name: &str) -> Geometry {
    let t = type_name.trim().to_ascii_uppercase();
    match (g, t.as_str()) {
        (Geometry::Point(Some(p)), "MULTIPOINT") => Geometry::MultiPoint(vec![p]),
        (Geometry::Curve(c), "MULTILINESTRING" | "MULTICURVE") => Geometry::MultiCurve(vec![c]),
        (Geometry::Polygon(r), "MULTIPOLYGON" | "MULTISURFACE") => Geometry::MultiPolygon(vec![r]),
        (Geometry::MultiPoint(mut p), "POINT") if p.len() == 1 => Geometry::Point(p.pop()),
        (
            Geometry::MultiCurve(mut c),
            "LINESTRING" | "CURVE" | "COMPOUNDCURVE" | "CIRCULARSTRING",
        ) if c.len() == 1 => match c.pop() {
            Some(one) => Geometry::Curve(one),
            None => Geometry::MultiCurve(c),
        },
        (Geometry::MultiPolygon(mut p), "POLYGON" | "CURVEPOLYGON") if p.len() == 1 => {
            match p.pop() {
                Some(one) => Geometry::Polygon(one),
                None => Geometry::MultiPolygon(p),
            }
        }
        (g, _) => g,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Coord, Curve};

    #[test]
    fn parts_become_multis_and_back() {
        let ring = vec![Curve::Line(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.0),
            Coord::xy(1.0, 1.0),
            Coord::xy(0.0, 0.0),
        ])];
        let p = Geometry::Polygon(ring.clone());
        assert_eq!(
            fit(p.clone(), "MultiPolygon"),
            Geometry::MultiPolygon(vec![ring.clone()])
        );
        assert_eq!(fit(Geometry::MultiPolygon(vec![ring]), "POLYGON"), p);
        assert_eq!(fit(p.clone(), "GEOMETRY"), p);
    }
}
