//! A linked view whose recipe fixes the scale (docs/adr/0794): when its area does not fit the map's frame at that scale,
//! the sheet's preflight says so (`map_view_cut`), with how far it runs past; a view that fits, or one without a scale (its box
//! fitted to the frame), says nothing.

use kentos_contracts::{NamedView, Vec2, ViewRecipe};
use kentos_sheet::{
    display::RenderInputs,
    ops::{self, Op},
    *,
};
use serde_json::json;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../fixtures/sheet/v1/viewport-annotations.json")).unwrap()
}

fn view(width: f64, height: f64, scale: Option<u32>) -> NamedView {
    NamedView {
        id: "v-detay".into(),
        name: "Detay".into(),
        center: Vec2 { x: 0.0, y: 0.0 },
        width,
        height,
        twist: 0.0,
        recipe: scale.map(|s| ViewRecipe {
            version: 1,
            scale: Some(s),
            ..ViewRecipe::default()
        }),
    }
}

/// The page's map linked to `v`, and the findings.
fn findings(v: NamedView) -> Vec<kentos_sheet::preflight::Finding> {
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(fixture()["cases"][0]["sheet"].clone()).unwrap());
    let op: Op = serde_json::from_value(json!({"op": "setViewport", "id": "viewport", "locked": false,
        "source": {"type": "viewLink", "view": serde_json::to_value(&v).unwrap()}}))
    .unwrap();
    let book = ops::apply(&book, &op).unwrap().book;
    let render = RenderInputs {
        named_views: vec![v],
        ..RenderInputs::default()
    };
    kentos_sheet::preflight::preflight(&book, "page", &render).unwrap()
}

#[test]
fn a_view_larger_than_its_frame_at_its_scale_is_a_finding() {
    let found = findings(view(400.0, 300.0, Some(20)));
    let f = found.iter().find(|f| f.code == "map_view_cut").expect("the finding");
    assert!(f.message.contains("“Detay”") && f.message.contains("1:20"), "{}", f.message);
    assert!(f.message.contains("büyük"), "{}", f.message);
}

#[test]
fn a_view_that_fits_or_is_fitted_says_nothing() {
    assert!(findings(view(0.5, 0.4, Some(20))).iter().all(|f| f.code != "map_view_cut"));
    // No scale: the view's box is fitted to the frame, never cut.
    assert!(findings(view(400.0, 300.0, None)).iter().all(|f| f.code != "map_view_cut"));
}
