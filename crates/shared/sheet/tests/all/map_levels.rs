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

/// docs/adr/0718: one sheet per level from one template, every map on the same ground (the box's centre) at one
/// scale, its level the sheet's; numbers count on past a number a sheet has; a level a map shows is left out unless
/// asked again, a sheet id taken gets `-2`.
#[test]
fn level_sheets_share_the_ground_and_number_on() {
    use kentos_sheet::levels::{LevelSheetsOptions, level_sheets};
    use kentos_sheet::template::{InstanceOptions, system_template};
    let t = system_template("sys:cad-teknik").expect("a system template").clone();
    let levels = vec![
        ("zemin".to_owned(), "Zemin kat".to_owned()),
        ("kat-1".to_owned(), "1. kat".to_owned()),
    ];
    let how = LevelSheetsOptions {
        bounds: Some([0.0, 0.0, 10.0, 8.0]),
        ..Default::default()
    };
    let ids = |sheet: &str| kentos_sheet::create::ids_for(&t, sheet);
    let made = level_sheets(&SheetBook::default(), &t, &InstanceOptions::default(), &levels, &how, ids)
        .expect("made");
    let numbers: Vec<&str> = made.made.iter().map(|m| m.number.as_str()).collect();
    assert_eq!(numbers, ["A-101", "A-102"]);
    let book = ops::apply_all(&SheetBook::default(), &made.ops).expect("applies").book;
    // Each map's centre, scale and level.
    type Shown = (Option<(f64, f64)>, u32, Option<String>);
    let views: Vec<Shown> = book
        .sheets
        .iter()
        .map(|s| {
            let m = s
                .items
                .iter()
                .find_map(|i| match &i.kind {
                    ItemKind::Map(m) => Some(m),
                    _ => None,
                })
                .expect("a map");
            let MapView::Fixed(v) = &m.view else {
                panic!("a fixed map")
            };
            (v.center.map(|c| (c.x, c.y)), v.scale, m.level.as_ref().map(|l| l.id.clone()))
        })
        .collect();
    assert_eq!(views[0], (Some((5.0, 4.0)), 100, Some("zemin".into())));
    assert_eq!(views[1], (Some((5.0, 4.0)), 100, Some("kat-1".into())), "the same ground and scale");
    assert_eq!(book.sheets[0].name, "A-101 Zemin kat planı");
    assert_eq!(
        book.sheets[1].drawing_info.as_ref().map(|i| (i.number.as_str(), i.title.as_str())),
        Some(("A-102", "1. kat planı"))
    );
    // Again: Zemin kat is shown, 1. kat too; with `again` a second sheet of Zemin kat, numbered on, its id `-2`.
    let none = level_sheets(&book, &t, &InstanceOptions::default(), &levels, &how, ids);
    assert_eq!(none.err().map(|e| e.code), Some("levels_have_sheets".to_owned()));
    let again = level_sheets(
        &book,
        &t,
        &InstanceOptions::default(),
        &levels[..1],
        &LevelSheetsOptions { again: true, ..how.clone() },
        ids,
    )
    .expect("made again");
    assert_eq!(again.made[0].sheet, "kat-plani-zemin-2");
    assert_eq!(again.made[0].number, "A-103");
}
