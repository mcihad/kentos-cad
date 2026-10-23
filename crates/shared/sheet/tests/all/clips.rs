//! Independent known regions, coverage, immutable camera and inverse operations.
use kentos_sheet::{
    display::{Prim, RenderInputs, RenderMode},
    ops::Op,
    *,
};
use serde_json::Value;
#[test]
fn viewport_boundaries_preserve_camera_and_match_the_independent_regions_and_coverage() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-clips.json"
    ))
    .unwrap();
    assert!(data["cases"].as_array().unwrap().len() >= 20);
    for case in data["cases"].as_array().unwrap() {
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(case["sheet"].clone()).unwrap());
        let original = book.clone();
        let operation: Op = serde_json::from_value(case["operation"].clone()).unwrap();
        let result = ops::apply_all(&book, &[operation]);
        if let Some(error) = case["expected"]["error"].as_str() {
            assert_eq!(result.unwrap_err().code, error, "{}", case["name"]);
            continue;
        }
        let done = result.unwrap_or_else(|e| panic!("{}: {e}", case["name"]));
        let item = &done.book.sheets[0].items[0];
        let ItemKind::Map(map) = &item.kind else {
            panic!()
        };
        let ItemKind::Map(old) = &original.sheets[0].items[0].kind else {
            panic!()
        };
        assert_eq!(map.view, old.view, "camera must not change");
        assert_eq!(map.view_locked, old.view_locked);
        assert_eq!(
            serde_json::to_value(&map.clip).unwrap(),
            case["expected"]["clip"]
        );
        assert_eq!(
            map.frame_plot,
            case["expected"]["framePlot"].as_bool().unwrap()
        );
        for probe in case["expected"]["probes"].as_array().unwrap() {
            assert_eq!(
                map.clip
                    .contains(probe[0].as_f64().unwrap(), probe[1].as_f64().unwrap()),
                probe[2].as_bool().unwrap(),
                "{}: {probe}",
                case["name"]
            );
        }
        let mut rgba = vec![255; 40 * 30 * 4];
        assert!(map.clip.mask_rgba(&mut rgba, 40, 30));
        let area = rgba
            .chunks_exact(4)
            .map(|p| f64::from(p[3]) / 255.)
            .sum::<f64>();
        assert!(
            (area - case["expected"]["areaFraction"].as_f64().unwrap() * 1200.).abs() < 2.,
            "{}: coverage {area}",
            case["name"]
        );
        let list = display::display_list(
            &done.book,
            "page",
            &RenderInputs {
                mode: RenderMode::Export,
                ..Default::default()
            },
        )
        .unwrap();
        let prim = list
            .prims
            .iter()
            .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
            .unwrap();
        assert_eq!(prim.clip_shape, map.clip);
        let border_count = list
            .prims
            .iter()
            .filter(|p| {
                matches!(p,Prim::Rect(r)if r.stroke.is_some())
                    || matches!(p,Prim::Path(p)if p.stroke.is_some())
            })
            .count();
        assert_eq!(border_count, usize::from(map.frame_plot));
        let json = serde_json::to_vec(&done.book).unwrap();
        assert_eq!(
            serde_json::from_slice::<SheetBook>(&json).unwrap(),
            done.book
        );
        assert_eq!(
            ops::apply_all(&done.book, &done.inverse).unwrap().book,
            original
        );
        assert_eq!(book, original);
    }
}
#[test]
fn custom_clip_hit_testing_keeps_the_void_click_through_and_the_actual_boundary_selectable() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-clips.json"
    ))
    .unwrap();
    let case = data["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Triangle")
        .unwrap();
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(case["sheet"].clone()).unwrap());
    let done = ops::apply_all(
        &book,
        &[serde_json::from_value(case["operation"].clone()).unwrap()],
    )
    .unwrap();
    let item = &done.book.sheets[0].items[0];
    let ItemKind::Map(map) = &item.kind else {
        panic!()
    };
    let r = display::map_content_rect(item, map);
    assert!(hit::point_on(
        item,
        [
            f64::from(r.left) + f64::from(r.width) * 0.1,
            f64::from(r.top) + f64::from(r.height) * 0.1
        ],
        1.
    ));
    assert!(!hit::point_on(
        item,
        [
            f64::from(r.left) + f64::from(r.width) * 0.9,
            f64::from(r.top) + f64::from(r.height) * 0.9
        ],
        1.
    ));
}
