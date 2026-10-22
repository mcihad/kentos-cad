//! One independent camera fixture shared with native, Python and the real designer.
use kentos_sheet::{
    display::{Prim, RenderInputs},
    ops::Op,
    viewport::*,
    *,
};
use serde_json::Value;
#[test]
fn independent_viewport_cases_and_inverse_keep_the_camera_and_source_exact() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewports.json"
    ))
    .unwrap();
    assert!(data["cases"].as_array().unwrap().len() >= 20);
    for case in data["cases"].as_array().unwrap() {
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(case["sheet"].clone()).unwrap());
        let before = book.clone();
        let op: Op = serde_json::from_value(case["operation"].clone()).unwrap();
        let result = kentos_sheet::ops::apply_all(&book, &[op]);
        if let Some(error) = case["expected"]["error"].as_str() {
            assert_eq!(result.unwrap_err().code, error, "{}", case["name"]);
            continue;
        }
        let done = result.unwrap_or_else(|e| panic!("{}: {e}", case["name"]));
        let item = &done.book.sheets[0].items[0];
        let ItemKind::Map(map) = &item.kind else {
            panic!()
        };
        let MapView::Fixed(view) = &map.view else {
            panic!()
        };
        let expected = &case["expected"];
        if let Some(pair) = expected.get("ratio") {
            for (got, key) in [
                (view.ratio.unwrap().paper, "paper"),
                (view.ratio.unwrap().model, "model"),
            ] {
                assert_eq!(
                    got.to_bits(),
                    pair[key].as_f64().unwrap().to_bits(),
                    "explicit ratio pair"
                );
            }
            let bytes = serde_json::to_vec(&done.book).unwrap();
            assert_eq!(
                serde_json::from_slice::<SheetBook>(&bytes).unwrap(),
                done.book
            );
        }
        assert!(
            (view.denominator() - expected["denominator"].as_f64().unwrap()).abs() < 1e-9,
            "{}",
            case["name"]
        );
        assert_eq!(view.rotation, expected["rotation"].as_i64().unwrap() as i32);
        assert_eq!(map.view_locked, expected["locked"].as_bool().unwrap());
        assert_eq!(
            map.source,
            serde_json::from_value::<Source>(expected["source"].clone()).unwrap()
        );
        let c = view.center.unwrap();
        assert!((c.x - expected["center"][0].as_f64().unwrap()).abs() < 1e-9);
        assert!((c.y - expected["center"][1].as_f64().unwrap()).abs() < 1e-9);
        let corners = display::fixed_corners(item).unwrap();
        for (a, b) in corners.iter().zip(expected["corners"].as_array().unwrap()) {
            for j in 0..2 {
                assert!((a[j] - b[j].as_f64().unwrap()).abs() < 1e-8);
            }
        }
        let list = display::display_list(&done.book, "page", &RenderInputs::default()).unwrap();
        let map = list
            .prims
            .iter()
            .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
            .unwrap();
        assert_eq!(map.clip.width, 380000);
        assert_eq!(map.clip.height, 220000);
        assert_eq!(
            ops::apply_all(&done.book, &done.inverse).unwrap().book,
            before
        );
        assert_eq!(book, before, "the source book is immutable");
    }
}
#[test]
fn a_property_patch_cannot_bypass_the_view_lock_but_can_move_the_paper_frame() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewports.json"
    ))
    .unwrap();
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap());
    let locked = ops::apply_all(
        &book,
        &[Op::SetViewport(SetViewport {
            id: "viewport".into(),
            locked: Some(true),
            ..Default::default()
        })],
    )
    .unwrap()
    .book;
    for patch in [
        serde_json::json!({"kind":{"view":{"scale":50}}}),
        serde_json::json!({"kind":{"view":{"center":{"x":3,"y":4}}}}),
        serde_json::json!({"kind":{"source":{"type":"viewSnapshot","view":{"id":"v","name":"Plan","center":{"x":0,"y":0},"width":1,"height":1,"twist":0}}}}),
    ] {
        let result = ops::apply_all(
            &locked,
            &[Op::SetItemProps(ops::SetItemProps {
                id: "viewport".into(),
                patch,
            })],
        );
        assert_eq!(result.unwrap_err().code, "item_locked");
    }
    let done = ops::apply_all(
        &locked,
        &[serde_json::from_value(
            serde_json::json!({"op":"moveItems","ids":["viewport"],"delta":[1000,2000]}),
        )
        .unwrap()],
    )
    .unwrap();
    assert_eq!(done.book.sheets[0].items[0].frame.left, 21000);
    assert_eq!(
        ops::apply_all(&done.book, &done.inverse).unwrap().book,
        locked
    );
}
