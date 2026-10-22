//! Independent physical units and large-coordinate checks; no golden output is generated here.
use kentos_sheet::{
    GroundPoint,
    projection::{Scale, Transform},
    units::RectUm,
};
use serde_json::Value;
#[test]
fn physical_projection_and_its_inverse_match_all_independent_unit_and_rotation_cases() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-projection.json"
    ))
    .unwrap();
    assert_eq!(data["cases"].as_array().unwrap().len(), 40);
    for case in data["cases"].as_array().unwrap() {
        let frame: RectUm = serde_json::from_value(case["frame"].clone()).unwrap();
        let denominator = case["denominator"].as_f64().unwrap();
        let scale = Scale::new(denominator).unwrap();
        let center = GroundPoint {
            x: case["center"][0].as_f64().unwrap(),
            y: case["center"][1].as_f64().unwrap(),
        };
        let rotation = case["rotation"].as_i64().unwrap() as i32;
        let camera = Transform::paper(&frame, center, denominator, rotation);
        let pixels_per_um = case["pixelsPerUm"].as_f64().unwrap();
        let pixels = Transform::at_resolution(
            center,
            [500., 250.],
            scale.pixels_per_metre(pixels_per_um),
            rotation,
        );
        let origin = [center.x - 10., center.y - 20.];
        let (m, t) = pixels.relative_affine(origin);
        for (index, world) in case["world"].as_array().unwrap().iter().enumerate() {
            let point = [world[0].as_f64().unwrap(), world[1].as_f64().unwrap()];
            let paper = camera.point(point);
            let inverse = camera.inverse(paper);
            let px = pixels.point(point);
            let local = [
                ((point[0] - origin[0]) as f32) as f64,
                ((point[1] - origin[1]) as f32) as f64,
            ];
            let shader = [
                m[0] * local[0] + m[2] * local[1] + t[0],
                m[1] * local[0] + m[3] * local[1] + t[1],
            ];
            for j in 0..2 {
                assert!(
                    (paper[j] - case["expected"]["paper"][index][j].as_f64().unwrap()).abs() < 0.01,
                    "{}",
                    case["name"]
                );
                assert!((inverse[j] - point[j]).abs() < 1e-8);
                assert!(
                    (px[j] - case["expected"]["pixels"][index][j].as_f64().unwrap()).abs() < 1e-5
                );
                assert!(
                    (shader[j] - px[j]).abs() < 0.02,
                    "{}: relative FP32 detail",
                    case["name"]
                );
            }
        }
        for (a, b) in camera
            .ground_corners(&frame)
            .iter()
            .zip(case["expected"]["corners"].as_array().unwrap())
        {
            for j in 0..2 {
                assert!((a[j] - b[j].as_f64().unwrap()).abs() < 1e-8);
            }
        }
        for (got, want) in camera
            .ground_bounds(&frame)
            .iter()
            .zip(case["expected"]["bounds"].as_array().unwrap())
        {
            assert!(
                (got - want.as_f64().unwrap()).abs() < 1e-8,
                "{}: ground bounds",
                case["name"]
            );
        }
        let length = case["modelLength"].as_f64().unwrap();
        let mm = case["expected"]["lengthMm"].as_f64().unwrap();
        assert!((scale.paper_mm(length) - mm).abs() < 1e-10);
        assert!((scale.model_metres_from_mm(mm) - length).abs() < 1e-12);
        assert!(
            (Scale::from_lengths(length, mm).unwrap().denominator() - denominator).abs() < 1e-10
        );
    }
}
#[test]
fn invalid_physical_scales_are_rejected_at_the_command_boundary() {
    for n in [
        0.,
        -1.,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1e-7,
        1e9,
    ] {
        assert!(Scale::new(n).is_none());
    }
    for (a, b) in [(0., 1.), (1., 0.), (-1., 1.), (1., -1.), (f64::NAN, 1.)] {
        assert!(Scale::from_lengths(a, b).is_none());
    }
    assert!(Scale::new(0.5).is_some());
}
