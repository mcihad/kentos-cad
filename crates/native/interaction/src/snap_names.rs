//! What a snap stands on, in a building's words (docs/adr/3020; Vectorworks' SmartCursor cues): a wall's face, its
//! core's face, the boundary between two of its layers, its axis or its base line; an opening's middle or its jamb.
//! The snap marker says it after the snap's kind: “Orta nokta · Duvar yüzü”, “Uç nokta · Açıklık kenarı”.
//!
//! The snap stood exactly on the line it names (the store snaps a wall by its drawn lines, `geom::wall::pieces`), so
//! the line is found again from the point: the nearest of the wall's lines within a tenth of a millimetre.

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::wall;

use crate::Vec2;

/// How near a line the snapped point is: on it, give or take rounding.
const ON: f64 = 1e-4;

/// The building's name of what `at` is on, on `shape`; none for another object or off its named lines.
pub fn name(shape: &Shape, at: Vec2) -> Option<&'static str> {
    match shape {
        Shape::Wall {
            pts,
            bulges,
            wall: spec,
        } => wall_line(pts, bulges.as_deref(), spec, at),
        Shape::Opening { place, .. } if place.len() == 4 => opening_place(place, at),
        _ => None,
    }
}

/// A wall's line: its first and last (the faces), a boundary between two components (the core's face when one of
/// them is the core, priority 1, docs/adr/0340 §1), the middle of its faces (its axis) or its path (the base line,
/// Öznitelikler's Taban çizgisi), in that order.
fn wall_line(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    spec: &wall::Spec,
    at: Vec2,
) -> Option<&'static str> {
    let lines = wall::lines(pts, bulges, spec).ok()?;
    let on = |line: &wall::Line| near_path(&line.outline(spec.closed), spec.closed, at) <= ON;
    let last = lines.len().checked_sub(1)?;
    if let Some(i) = lines.iter().position(|l| on(l)) {
        if i == 0 || i == last {
            return Some("Duvar yüzü");
        }
        let core = |k: usize| {
            spec.components
                .get(k)
                .is_some_and(|c| c.priority == Some(1))
        };
        return Some(if core(i - 1) || core(i) {
            "Çekirdek yüzü"
        } else {
            "Katman sınırı"
        });
    }
    let middle = wall::left_face(spec) - spec.width / 2.0;
    if let Ok(axis) = wall::line_at(pts, bulges, spec.closed, middle)
        && on(&axis)
    {
        return Some("Duvar ekseni");
    }
    let base = wall::line_at(pts, bulges, spec.closed, 0.0).ok()?;
    on(&base).then_some("Taban çizgisi")
}

/// An opening's middle (between its jambs' middles) or a jamb (start: corners 0 and 1, end: 2 and 3).
fn opening_place(place: &[Vec2], at: Vec2) -> Option<&'static str> {
    let mid = |a: Vec2, b: Vec2| Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let middle = mid(mid(place[0], place[1]), mid(place[2], place[3]));
    if (middle.x - at.x).hypot(middle.y - at.y) <= ON {
        return Some("Açıklık ortası");
    }
    let jamb = near_segment(place[0], place[1], at).min(near_segment(place[2], place[3], at));
    (jamb <= ON).then_some("Açıklık kenarı")
}

/// The distance from `at` to a path of straight pieces (closed: back to its start).
fn near_path(pts: &[Vec2], closed: bool, at: Vec2) -> f64 {
    let n = pts.len();
    if n == 1 {
        return (pts[0].x - at.x).hypot(pts[0].y - at.y);
    }
    let pieces = if closed { n } else { n.saturating_sub(1) };
    (0..pieces)
        .map(|i| near_segment(pts[i], pts[(i + 1) % n], at))
        .fold(f64::INFINITY, f64::min)
}

fn near_segment(a: Vec2, b: Vec2, at: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((at.x - a.x) * dx + (at.y - a.y) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a.x + dx * t - at.x).hypot(a.y + dy * t - at.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_geometry_core::geom::wall::{Component, Justify, Spec};

    fn straight(spec: Spec) -> Shape {
        Shape::Wall {
            pts: vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)],
            bulges: None,
            wall: spec,
        }
    }

    fn layer(thickness: f64, priority: u8) -> Component {
        Component {
            thickness: Some(thickness),
            priority: Some(priority),
            ..Default::default()
        }
    }

    #[test]
    fn a_wall_names_its_faces_its_axis_and_its_base_line() {
        // 0.2 thick, justified left: the base line is the left face, the axis 0.1 to its right.
        let wall = straight(Spec {
            width: 0.2,
            justify: Justify::Left,
            ..Default::default()
        });
        assert_eq!(
            name(&wall, Vec2::new(5.0, 0.0)),
            Some("Duvar yüzü"),
            "the base line on the face: the face"
        );
        assert_eq!(name(&wall, Vec2::new(5.0, -0.2)), Some("Duvar yüzü"));
        assert_eq!(name(&wall, Vec2::new(5.0, -0.1)), Some("Duvar ekseni"));
        assert_eq!(
            name(&wall, Vec2::new(5.0, -0.05)),
            None,
            "between its lines: no name"
        );
        let offset = straight(Spec {
            width: 0.2,
            justify: Justify::Center,
            offset: 0.5,
            ..Default::default()
        });
        assert_eq!(name(&offset, Vec2::new(5.0, 0.5)), Some("Duvar ekseni"));
        assert_eq!(
            name(&offset, Vec2::new(5.0, 0.0)),
            Some("Taban çizgisi"),
            "the path, off the wall"
        );
    }

    #[test]
    fn a_layered_wall_names_its_core_s_faces_apart_from_its_other_boundaries() {
        // From the left face: 2 cm plaster (5), 20 cm core (1), 5 cm insulation (3), 3 cm plaster (5).
        let wall = straight(Spec {
            width: 0.30,
            justify: Justify::Left,
            components: vec![
                layer(0.02, 5),
                layer(0.20, 1),
                layer(0.05, 3),
                layer(0.03, 5),
            ],
            ..Default::default()
        });
        assert_eq!(name(&wall, Vec2::new(5.0, -0.02)), Some("Çekirdek yüzü"));
        assert_eq!(name(&wall, Vec2::new(5.0, -0.22)), Some("Çekirdek yüzü"));
        assert_eq!(
            name(&wall, Vec2::new(5.0, -0.27)),
            Some("Katman sınırı"),
            "insulation and plaster"
        );
        assert_eq!(name(&wall, Vec2::new(5.0, -0.30)), Some("Duvar yüzü"));
    }

    #[test]
    fn an_opening_names_its_middle_and_its_jambs() {
        let opening = Shape::Opening {
            at: 1.0,
            width: 0.9,
            place: vec![
                Vec2::new(1.0, 0.1),
                Vec2::new(1.0, -0.1),
                Vec2::new(1.9, 0.1),
                Vec2::new(1.9, -0.1),
            ],
            opening: Default::default(),
        };
        assert_eq!(name(&opening, Vec2::new(1.45, 0.0)), Some("Açıklık ortası"));
        assert_eq!(name(&opening, Vec2::new(1.0, 0.0)), Some("Açıklık kenarı"));
        assert_eq!(name(&opening, Vec2::new(1.9, -0.1)), Some("Açıklık kenarı"));
        assert_eq!(name(&opening, Vec2::new(1.3, 0.1)), None, "the leaf's side");
    }
}
