use kentos_sheet::{
    *,
    display::{RenderInputs, Prim},
    viewport::*,
    ops::{self, Op},
};
#[test]
fn twenty_independent_link_cameras_follow_stable_ids_but_snapshots_do_not() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-links.json"
    ))
    .unwrap();
    assert_eq!(data["cases"].as_array().unwrap().len(), 20);
    for c in data["cases"].as_array().unwrap() {
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(c["sheet"].clone()).unwrap());
        let op: Op = serde_json::from_value(c["operation"].clone()).unwrap();
        let result = ops::apply(&book, &op).unwrap();
        let book = result.book;
        let saved = book.clone();
        let views: Vec<NamedModelView> = serde_json::from_value(c["views"].clone()).unwrap();
        let inputs = RenderInputs {
            named_views: views.clone(),
            ..Default::default()
        };
        let list = display::display_list(&book, "page", &inputs).unwrap();
        let map = list
            .prims
            .iter()
            .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
            .unwrap();
        let expected = &c["expected"];
        assert!(
            (map.view.scale - expected["scale"].as_f64().unwrap()).abs() < 1e-10,
            "{}",
            c["name"]
        );
        assert_eq!(
            map.view.rotation,
            expected["rotation"].as_i64().unwrap() as i32
        );
        assert_eq!(
            serde_json::to_value(map.view.center).unwrap(),
            expected["center"]
        );
        let scope =
            model_scope::Viewport::resolve_with_views(&book, "page", "viewport", &views).unwrap();
        assert_eq!(scope.center, map.view.center.unwrap());
        assert_eq!(scope.denominator, map.view.scale);
        assert_eq!(
            book, saved,
            "rendering never rewrites the captured source or model"
        );
        assert!(
            preflight::preflight(&book, "page", &inputs)
                .unwrap()
                .iter()
                .all(|f| f.code != "missing_named_view")
        );
        assert!(
            preflight::preflight(&book, "page", &RenderInputs::default())
                .unwrap()
                .iter()
                .any(|f| f.code == "missing_named_view")
        );
        let missing = display::display_list(&book, "page", &RenderInputs::default()).unwrap();
        assert!(!missing.prims.iter().any(|p| matches!(p, Prim::Map(_))));
        let detach = SetViewport {
            id: "viewport".into(),
            source: Some(Source::Drawing),
            ..Default::default()
        };
        let detach = prepare_source(&book, &detach, &views).unwrap();
        let copy = ops::apply(&book, &Op::SetViewport(detach)).unwrap();
        let camera = effective_fixed_view(copy.book.item("viewport").unwrap(), &[]).unwrap();
        assert_eq!(camera.center, map.view.center);
        assert_eq!(camera.denominator(), map.view.scale);
        assert_eq!(
            ops::apply_all(&copy.book, &copy.inverse).unwrap().book,
            book
        );
        let mut snapshot = c["operation"].clone();
        snapshot["source"]["type"] = serde_json::json!("viewSnapshot");
        let snap = ops::apply(&saved, &serde_json::from_value(snapshot).unwrap()).unwrap();
        assert_ne!(
            effective_fixed_view(snap.book.item("viewport").unwrap(), &views)
                .unwrap()
                .center,
            map.view.center
        );
        let raw=Op::SetItemProps(ops::SetItemProps{id:"viewport".into(),patch:serde_json::json!({"kind":{"view":{"center":{"x":0.,"y":0.}}}})});
        assert_eq!(ops::apply(&book,&raw).unwrap_err().code,"linked_view");
        let edit = SetViewport {
            id: "viewport".into(),
            center: Some(GroundPoint { x: 0., y: 0. }),
            ..Default::default()
        };
        assert_eq!(
            configure(book.item("viewport").unwrap(), &edit)
                .unwrap_err()
                .code,
            "linked_view"
        );
    }
}
