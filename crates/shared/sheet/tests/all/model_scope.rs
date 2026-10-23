use kentos_sheet::{
    *,
    model_scope::Viewport,
    ops::{self, Op},
};
#[test]
fn independent_rotated_navigation_and_scope_have_one_model_and_reversible_paper_camera() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-navigation.json"
    ))
    .unwrap();
    assert_eq!(data["cases"].as_array().unwrap().len(), 24);
    for case in data["cases"].as_array().unwrap() {
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(case["sheet"].clone()).unwrap());
        let scope = Viewport::resolve(&book, "page", "viewport").unwrap();
        let p: [f64; 2] = serde_json::from_value(case["paper"].clone()).unwrap();
        let world = scope.model_point(p).unwrap();
        for j in 0..2 {
            assert!(
                (world[j] - case["world"][j].as_f64().unwrap()).abs() < 1e-8,
                "{}",
                case["name"]
            );
            assert!((scope.paper_point(world)[j] - p[j]).abs() < 1e-4);
        }
        let op: Op = serde_json::from_value(case["operation"].clone()).unwrap();
        let result = ops::apply_all(&book, std::slice::from_ref(&op)).unwrap();
        let after = Viewport::resolve(&result.book, "page", "viewport").unwrap();
        for j in 0..2 {
            assert!(
                ([after.center.x, after.center.y][j]
                    - case["expected"]["center"][j].as_f64().unwrap())
                .abs()
                    < 1e-8,
                "{}",
                case["name"]
            );
        }
        assert_eq!(
            after.denominator,
            case["expected"]["denominator"].as_f64().unwrap()
        );
        assert_eq!(
            ops::apply_all(&result.book, &result.inverse).unwrap().book,
            book
        );
        assert!(Viewport::resolve(&book, "missing", "viewport").is_err());
        assert!(Viewport::resolve(&book, "page", "missing").is_err());
        if let ItemKind::Map(m) = &mut book.sheets[0].items[0].kind {
            m.view_locked = true;
        }
        assert_eq!(
            ops::apply_all(&book, &[op]).unwrap_err().code,
            "item_locked"
        );
    }
}
#[test]
fn concave_scope_and_overlay_segments_do_not_cross_the_void() {
    let clip = clip::Clip::Polygon {
        points: vec![
            [0, 0],
            [1000000, 0],
            [1000000, 300000],
            [300000, 300000],
            [300000, 1000000],
            [0, 1000000],
        ],
    };
    let pieces = clip.segments([-1., 0.5], [2., 0.5]);
    assert_eq!(pieces.len(), 1);
    assert!((pieces[0][1][0] - 0.3).abs() < 1e-15);
    assert!(clip.segments([0.5, 0.5], [0.9, 0.9]).is_empty());
    assert_eq!(
        clip.segments([-1., 0.1], [2., 0.1]),
        vec![[[0., 0.1], [1., 0.1]]]
    );
}
