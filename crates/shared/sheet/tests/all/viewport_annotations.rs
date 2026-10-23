use kentos_sheet::{
    *,
    ops::{self, Op},
    display::{self, Prim, RenderInputs},
    viewport::{SetViewport, AnnotationScale},
};
#[test]
fn twenty_annotation_and_layer_cases_are_one_locked_camera_safe_change() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-annotations.json"
    ))
    .unwrap();
    assert_eq!(data["cases"].as_array().unwrap().len(), 20);
    for c in data["cases"].as_array().unwrap() {
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(c["sheet"].clone()).unwrap());
        let before = book.clone();
        let op: Op = serde_json::from_value(c["operation"].clone()).unwrap();
        let done = ops::apply(&book, &op).unwrap_or_else(|e| panic!("{} {e}", c["name"]));
        let ItemKind::Map(old) = &book.sheets[0].items[0].kind else {
            panic!()
        };
        let ItemKind::Map(map) = &done.book.sheets[0].items[0].kind else {
            panic!()
        };
        assert_eq!(map.view, old.view);
        assert!(map.view_locked);
        assert_eq!(
            map.layer_overrides[1].frozen,
            c["expected"]["annoFrozen"].as_bool().unwrap()
        );
        let list = display::display_list(&done.book, "page", &RenderInputs::default()).unwrap();
        let map = list
            .prims
            .iter()
            .find_map(|p| match p {
                Prim::Map(m) => Some(m),
                _ => None,
            })
            .unwrap();
        assert_eq!(map.view.scale, c["expected"]["physical"].as_f64().unwrap());
        assert_eq!(
            map.view.annotation_denominator(),
            c["expected"]["annotation"].as_f64().unwrap()
        );
        assert_eq!(
            ops::apply_all(&done.book, &done.inverse).unwrap().book,
            before
        );
        let reset = Op::SetViewport(SetViewport {
            id: "viewport".into(),
            annotation: Some(AnnotationScale::Auto),
            layer_overrides: Some(Vec::new()),
            ..Default::default()
        });
        let reset = ops::apply(&done.book, &reset).unwrap();
        let ItemKind::Map(map) = &reset.book.sheets[0].items[0].kind else {
            panic!()
        };
        assert!(map.annotation_scale.is_none());
        assert!(map.layer_overrides.is_empty());
        assert_eq!(map.view, old.view);
        for invalid in [0., -1., 1000001., f64::INFINITY, f64::NAN] {
            let op = Op::SetViewport(SetViewport {
                id: "viewport".into(),
                annotation: Some(AnnotationScale::Fixed { scale: invalid }),
                ..Default::default()
            });
            assert_eq!(
                ops::apply(&book, &op).unwrap_err().code,
                "bad_annotation_scale"
            );
        }
    }
}
