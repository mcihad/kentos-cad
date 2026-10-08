//! A coordinate list's source and headings (docs/adr/0206 §2, §3), worked
//! out by hand: the headings each given or their default (a local project's
//! east is X), what the source gives said in a line, the objects taken by
//! their ids and the headings checked when written, and the drawn list's
//! header and area row in the words given.

use crate::common;

use common::*;
use kentos_sheet::display::{CoordPoint, CoordinateInput, coordinate_headings, coordinate_summary};
use kentos_sheet::kinds::{
    CoordColumns, CoordLayer, CoordObjects, CoordSource, CoordinateListItem, Empty, ItemKind,
    default_kind,
};
use kentos_sheet::ops::{Op, apply};
use kentos_sheet::validate::read_book;
use kentos_sheet::*;
use serde_json::json;

fn list() -> CoordinateListItem {
    match default_kind("coordinateList").expect("a coordinate list") {
        ItemKind::CoordinateList(c) => c,
        other => panic!("{other:?}"),
    }
}

fn input(
    points: usize,
    closed: bool,
    area: Option<f64>,
    objects: u32,
    missing: Option<u32>,
) -> CoordinateInput {
    CoordinateInput {
        item: "koordinatlar".into(),
        points: (0..points)
            .map(|i| CoordPoint {
                name: None,
                x: i as f64,
                y: 0.0,
                z: None,
            })
            .collect(),
        closed,
        area,
        objects: Some(objects),
        missing,
    }
}

#[test]
fn the_headings_are_given_or_their_defaults() {
    let mut c = list();
    assert_eq!(
        coordinate_headings(&c, true),
        ["Nokta", "Y (m)", "X (m)", "Z (m)", "Alan"].map(String::from)
    );
    // A local project: east is X.
    assert_eq!(
        coordinate_headings(&c, false)[1..3],
        ["X (m)".to_owned(), "Y (m)".to_owned()]
    );
    c.columns = Some(CoordColumns {
        point: Some("No".into()),
        east: Some("Sağa (Y)".into()),
        north: Some("  ".into()),
        z: None,
        area: Some("Yüzölçümü".into()),
    });
    // A blank one is its default.
    assert_eq!(
        coordinate_headings(&c, true),
        ["No", "Sağa (Y)", "X (m)", "Z (m)", "Yüzölçümü"].map(String::from)
    );
}

#[test]
fn what_the_source_gives_is_said() {
    let objects = CoordSource::Objects(CoordObjects {
        uids: vec!["a".into(), "b".into(), "c".into()],
    });
    assert_eq!(
        coordinate_summary(&objects, Some(&input(12, false, None, 3, None)), None),
        "Seçilen 3 nesne: 12 nokta."
    );
    assert_eq!(
        coordinate_summary(&objects, Some(&input(8, false, None, 2, Some(1))), None),
        "Seçilen 3 nesne (çizimde olmayan: 1): 8 nokta."
    );
    let one = CoordSource::Objects(CoordObjects {
        uids: vec!["a".into()],
    });
    assert_eq!(
        coordinate_summary(&one, Some(&input(4, true, Some(812.4), 1, None)), None),
        "Seçilen 1 nesne: kapalı şeklin 4 köşesi, alanı 812.40 m²."
    );
    assert_eq!(
        coordinate_summary(
            &CoordSource::Objects(CoordObjects { uids: vec![] }),
            None,
            None
        ),
        "Nesne alınmadı: çizimde nesneleri seçip “Seçimi al”a basın."
    );
    let layer = CoordSource::Layer(CoordLayer {
        layer: "parsel".into(),
    });
    assert_eq!(
        coordinate_summary(
            &layer,
            Some(&input(48, false, None, 12, None)),
            Some("Parsel")
        ),
        "“Parsel” katmanı: 12 nesne, 48 nokta."
    );
    assert_eq!(
        coordinate_summary(
            &layer,
            Some(&input(0, false, None, 0, None)),
            Some("Parsel")
        ),
        "“Parsel” katmanında nesne yok: liste boş."
    );
    let live = CoordSource::Selection(Empty {});
    assert_eq!(
        coordinate_summary(&live, Some(&input(0, false, None, 0, None)), None),
        "Çizimde seçili nesne yok: liste boş. Seçim değiştikçe liste de değişir; sabitlemek için “Seçimi al”."
    );
    assert!(
        coordinate_summary(&live, Some(&input(5, false, None, 2, None)), None)
            .starts_with("Çizimde şu an seçili 2 nesne: 5 nokta.")
    );
}

fn book() -> SheetBook {
    let path = fixtures().join("display/books/every-kind.json");
    read_book(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn patch(kind: serde_json::Value) -> Op {
    serde_json::from_value(
        json!({ "op": "setItemProps", "id": "koordinatlar", "patch": { "kind": kind } }),
    )
    .expect("an operation")
}

#[test]
fn the_objects_and_headings_are_checked_when_written() {
    let b = book();
    // The objects by their ids, and headings: written; a heading set to null is the default again.
    let applied = apply(&b, &patch(json!({ "type": "coordinateList", "source": { "type": "objects", "uids": ["0192f6a0-0000-7000-8000-000000000001"] }, "columns": { "point": "No", "area": "Yüzölçümü" } })))
        .expect("written");
    let back = apply(
        &applied.book,
        &patch(json!({ "type": "coordinateList", "columns": { "point": null } })),
    )
    .expect("taken away");
    let kind = |b: &SheetBook| {
        b.sheets[0]
            .items
            .iter()
            .find(|i| i.id == "koordinatlar")
            .map(|i| i.kind.clone())
    };
    match kind(&back.book) {
        Some(ItemKind::CoordinateList(c)) => {
            assert_eq!(c.columns.and_then(|c| c.area).as_deref(), Some("Yüzölçümü"));
            assert!(matches!(c.source, CoordSource::Objects(o) if o.uids.len() == 1));
        }
        other => panic!("{other:?}"),
    }
    // A heading over 40 characters or of two lines, an empty id, too many objects: refused.
    let refused = |kind: serde_json::Value| apply(&b, &patch(kind)).err().map(|e| e.code);
    assert_eq!(
        refused(json!({ "type": "coordinateList", "columns": { "east": "x".repeat(41) } }))
            .as_deref(),
        Some("invalid_column_name")
    );
    assert_eq!(
        refused(json!({ "type": "coordinateList", "columns": { "z": "Kot\n(m)" } })).as_deref(),
        Some("invalid_column_name")
    );
    assert_eq!(
        refused(json!({ "type": "coordinateList", "source": { "type": "objects", "uids": [""] } }))
            .as_deref(),
        Some("bad_id")
    );
    let many: Vec<String> = (0..10_001).map(|i| format!("u{i}")).collect();
    assert_eq!(
        refused(json!({ "type": "coordinateList", "source": { "type": "objects", "uids": many } }))
            .as_deref(),
        Some("out_of_range")
    );
}

#[test]
fn the_drawn_list_says_the_headings_given() {
    let b = book();
    let applied = apply(&b, &patch(json!({ "type": "coordinateList", "columns": { "point": "No", "east": "Sağa", "area": "Yüzölçümü" } }))).expect("written");
    // Tall enough for its rows and the area row (the book's own frame cuts them).
    let taller: Op = serde_json::from_value(json!({ "op": "setItemProps", "id": "koordinatlar", "patch": { "frame": { "height": 80000 } } }))
        .expect("an operation");
    let applied = apply(&applied.book, &taller).expect("taller");
    let sheet = applied.book.sheets[0].id.clone();
    let inputs = sample_inputs(&applied.book, &sheet, true);
    let list = kentos_sheet::display::display_list(&applied.book, &sheet, &inputs).expect("drawn");
    let text = serde_json::to_string(&list).unwrap();
    for word in ["\"No\"", "\"Sağa\"", "\"X (m)\"", "Yüzölçümü = "] {
        assert!(text.contains(word), "{word} yok");
    }
    assert!(!text.contains("\"Nokta\""), "the default no longer");
}
