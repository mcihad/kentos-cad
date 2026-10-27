//! A map's level (docs/adr/0716): `setViewport`'s `level` sets it whole (a level without `below` is not faint), `all`
//! takes it off, an empty or overlong id is `bad_level`, the inverse puts the old one back; the display primitive and the
//! model scope carry it.

use kentos_sheet::{
    display::{self, Prim, RenderInputs},
    kinds::{ItemKind, MapLevel},
    model_scope::Viewport,
    ops::{self, Op},
    *,
};
use serde_json::{Value, json};

fn book() -> SheetBook {
    let data: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/sheet/v1/viewport-annotations.json")).unwrap();
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap());
    book
}

fn set(level: Value) -> Op {
    serde_json::from_value(json!({"op": "setViewport", "id": "viewport", "level": level})).unwrap()
}

fn level_of(book: &SheetBook) -> Option<MapLevel> {
    match &book.sheets[0].items[0].kind {
        ItemKind::Map(m) => m.level.clone(),
        _ => panic!("a map"),
    }
}

#[test]
fn a_level_is_set_whole_taken_off_and_undone() {
    let b0 = book();
    let a = ops::apply(&b0, &set(json!({"type": "level", "id": "kat-1", "below": true}))).unwrap();
    assert_eq!(level_of(&a.book), Some(MapLevel { id: "kat-1".into(), below: true }));
    // Another level without `below`: not faint (the patch writes the level whole).
    let b = ops::apply(&a.book, &set(json!({"type": "level", "id": "zemin"}))).unwrap();
    assert_eq!(level_of(&b.book), Some(MapLevel { id: "zemin".into(), below: false }));
    // Its inverse puts kat-1 and its faint level below back.
    let back = b.inverse.iter().fold(b.book.clone(), |book, op| ops::apply(&book, op).unwrap().book);
    assert_eq!(level_of(&back), level_of(&a.book));
    let all = ops::apply(&b.book, &set(json!({"type": "all"}))).unwrap();
    assert_eq!(level_of(&all.book), None);
    // Carried to the display primitive and the paper's model scope.
    let list = display::display_list(&a.book, "page", &RenderInputs::default()).unwrap();
    let prim = list
        .prims
        .iter()
        .find_map(|p| match p {
            Prim::Map(m) => Some(m),
            _ => None,
        })
        .unwrap();
    assert_eq!(prim.level, level_of(&a.book));
    let scope = Viewport::resolve(&a.book, "page", "viewport").unwrap();
    assert_eq!(scope.level, level_of(&a.book));
}

#[test]
fn an_empty_or_overlong_level_is_refused() {
    let b0 = book();
    for id in [" ".to_owned(), "k".repeat(129)] {
        let e = ops::apply(&b0, &set(json!({"type": "level", "id": id}))).unwrap_err();
        assert_eq!(e.code, "bad_level", "{id}");
    }
    assert!(ops::apply(&b0, &set(json!({"type": "level", "id": "k".repeat(128)}))).is_ok());
    // The designer's patch is checked by the book's validation.
    let e = ops::apply(
        &b0,
        &serde_json::from_value(json!({"op": "setItemProps", "id": "viewport",
            "patch": {"kind": {"level": {"id": ""}}}}))
        .unwrap(),
    )
    .unwrap_err();
    assert_eq!(e.code, "bad_level");
}
