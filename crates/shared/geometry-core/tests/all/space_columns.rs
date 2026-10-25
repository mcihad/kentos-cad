//! Column deductions are physical analytic area differences, not display fills.
use kentos_geometry_core::geom::{
    arch::{ColumnSpec, column_area},
    arrangement::{Area, Ring},
    region,
};
use kentos_geometry_core::vec2::Vec2;

fn room(x: f64, y: f64) -> Area {
    Area {
        outer: Ring {
            pts: vec![
                Vec2::new(x, y),
                Vec2::new(x + 10., y),
                Vec2::new(x + 10., y + 8.),
                Vec2::new(x, y + 8.),
            ],
            bulges: None,
        },
        holes: vec![],
    }
}
fn column(x: f64, y: f64, width: f64, depth: f64) -> Area {
    column_area(
        Vec2::new(x, y),
        &ColumnSpec {
            width,
            depth: Some(depth),
            ..Default::default()
        },
    )
    .unwrap()
}
fn equal(parts: &[Area], expected: f64) {
    let got: f64 = parts.iter().map(region::net_area).sum();
    assert!((got - expected).abs() < 1e-6, "{got} != {expected}");
}

#[test]
fn rotated_rectangles_and_round_columns_keep_physical_areas_and_analytic_holes() {
    for (x, y) in [(0., 0.), (500000., 4400000.)] {
        let rect = column_area(
            Vec2::new(x + 2., y + 2.),
            &ColumnSpec {
                width: 2.,
                depth: Some(3.),
                rotation: 0.37,
                fill: false,
                ..Default::default()
            },
        )
        .unwrap();
        equal(std::slice::from_ref(&rect), 6.);
        let round = column_area(
            Vec2::new(x + 7., y + 5.),
            &ColumnSpec {
                round: true,
                width: 2.,
                depth: Some(7.),
                rotation: 1.1,
                fill: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(round.outer.pts.len(), 2);
        equal(std::slice::from_ref(&round), std::f64::consts::PI);
        let cut = region::subtract_areas(&[room(x, y)], &[rect, round]);
        assert_eq!(cut.len(), 1);
        assert_eq!(cut[0].holes.len(), 2);
        assert!(cut[0].holes.iter().any(|h| {
            h.pts.len() == 2
                && h.bulges
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|v| *v != 0.))
        }));
        equal(&cut, 80. - 6. - std::f64::consts::PI);
    }
}
#[test]
fn overlapping_columns_are_deducted_once_and_boundary_columns_are_clipped() {
    let base = room(0., 0.);
    let cut = region::subtract_areas(
        std::slice::from_ref(&base),
        &[column(3., 4., 2., 2.), column(4., 4., 2., 2.)],
    );
    equal(&cut, 74.);
    let cut = region::subtract_areas(
        std::slice::from_ref(&base),
        &[column(0.5, 4., 2., 4.), column(20., 20., 2., 2.)],
    );
    equal(&cut, 74.);
    assert_eq!(cut.len(), 1);
    assert!(cut[0].holes.is_empty());
}
#[test]
fn a_column_bar_can_leave_two_parts_and_full_coverage_has_no_free_area() {
    let base = room(0., 0.);
    let cut = region::subtract_areas(std::slice::from_ref(&base), &[column(5., 4., 2., 10.)]);
    assert_eq!(cut.len(), 2);
    equal(&cut, 64.);
    for part in &cut {
        equal(std::slice::from_ref(part), 32.);
    }
    assert!(region::subtract_areas(&[base], &[column(5., 4., 12., 10.)]).is_empty());
}
#[test]
fn invalid_or_unrepresentable_column_footprints_are_explicitly_absent() {
    for width in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(
            column_area(
                Vec2::new(0., 0.),
                &ColumnSpec {
                    width,
                    ..Default::default()
                }
            )
            .is_none()
        );
    }
    let valid = ColumnSpec {
        width: 0.4,
        ..Default::default()
    };
    assert!(column_area(Vec2::new(f64::INFINITY, 0.), &valid).is_none());
    assert!(column_area(Vec2::new(1e20, 1e20), &valid).is_none());
    assert!(
        column_area(
            Vec2::new(0., 0.),
            &ColumnSpec {
                rotation: f64::NAN,
                ..valid
            }
        )
        .is_none()
    );
}
