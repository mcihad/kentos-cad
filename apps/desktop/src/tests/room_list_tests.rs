//! Mahal listesi (M38, docs/adr/8038): the acceptance apartment's room list, storey by storey under the storey's
//! heading; each room's finishes as Öznitelikler wrote them, the doors and windows serving it by their joinery codes
//! (the room Doğrama kodları numbers them by, docs/adr/8028); the list's own cells write the rooms; one undo.

use serde_json::{Value, json};

use crate::app::{App, Message};

fn run(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// The drawing's last table's rows.
fn rows(app: &App) -> Vec<Vec<String>> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Table(t) => Some(t.cells.clone()),
            _ => None,
        })
        .last()
        .unwrap_or_default()
}

/// The room called `name`'s persistent id.
fn room(app: &App, name: &str) -> String {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Space(s) if s.space.name.as_deref() == Some(name) => doc
                .uid(kentos_domain::Slot(e.base().id))
                .map(|u| u.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a room called {name}"))
}

#[cfg(unix)]
#[test]
fn the_apartment_s_room_list_says_each_room_s_finishes_doors_and_windows() {
    let (mut app, _dir) = crate::acceptance_flat_tests::opened("mahal-listesi");
    let _ = crate::acceptance_flat_tests::drawn(&mut app);
    // Doğrama kodları first (docs/adr/8028): K01 the inner doors, GK01 the entrance, P01 and P02 the windows.
    let _ = app.update(Message::Run("tool.openingCodes"));
    let _ = app.submit_line("");
    // The salon's finishes, as Öznitelikler writes them (one change each).
    let salon = room(&app, "Salon");
    for (field, words) in [
        ("floor", "Lamine parke"),
        ("base", "Ahşap süpürgelik"),
        ("wall", "Saten boya"),
        ("ceiling", "Saten boya"),
        ("sill", "Mermer denizlik, 3 cm"),
    ] {
        run(
            &mut app,
            "cad.spaces.edit",
            json!({"uids": [salon], "operation": {"kind": "set", field: words}}),
        );
    }
    let _ = app.update(Message::Run("tool.scheduleRoomList"));
    assert_eq!(app.session.tool_id(), "scheduleRoomList");
    let _ = app.submit_line("30,0");
    let got = rows(&app);
    let row = |k: usize| got[k].iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(
        row(0),
        [
            "Kat",
            "No",
            "Ad",
            "Döşeme",
            "Süpürgelik",
            "Duvar kaplaması",
            "Tavan",
            "Kapı",
            "Pencere",
            "Denizlik ve parapet",
            "Basamak ve rıht",
            "Küpeşte ve korkuluk"
        ]
    );
    // The storey's heading row, then its rooms by number; no total row (the list adds nothing up).
    assert_eq!(row(1)[0], "Zemin kat");
    assert!(row(1)[1..].iter().all(|c| c.is_empty()), "{:?}", row(1));
    assert_eq!(
        row(2),
        [
            "Zemin kat",
            "01",
            "Salon",
            "Lamine parke",
            "Ahşap süpürgelik",
            "Saten boya",
            "Saten boya",
            "K01",
            "P01 (2)",
            "Mermer denizlik, 3 cm",
            "",
            ""
        ]
    );
    let by_name = |name: &str| {
        got.iter()
            .find(|r| r[2] == name)
            .map(|r| (r[7].clone(), r[8].clone()))
            .unwrap_or_else(|| panic!("{name} in the list: {got:?}"))
    };
    // Each door and window in the room it serves (docs/adr/8028's rule): the entrance the corridor's, the bathroom
    // both doors (the corridor's and the second bedroom's, which opens into it), its 60 window P02.
    assert_eq!(by_name("Yatak odası 1"), ("K01".into(), "P01".into()));
    assert_eq!(by_name("Yatak odası 2"), (String::new(), "P01".into()));
    assert_eq!(by_name("Koridor"), ("GK01".into(), String::new()));
    assert_eq!(by_name("Banyo"), ("K01 (2)".into(), "P02".into()));
    assert_eq!(
        got.len(),
        7,
        "a heading, the storey's heading and five rooms"
    );
    // The list's finish cells write the rooms: the bathroom's floor and sills.
    let table = {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities()
            .filter(|e| matches!(e, kentos_commands::Entity::Table(_)))
            .last()
            .and_then(|e| doc.uid(kentos_domain::Slot(e.base().id)))
            .map(|u| u.to_string())
            .expect("the list")
    };
    let bath = got
        .iter()
        .position(|r| r[2] == "Banyo")
        .expect("the bathroom's row");
    for (column, words) in [(3, "Seramik"), (9, "Mermer denizlik")] {
        run(
            &mut app,
            "cad.schedules.edit",
            json!({"uid": table, "operation": {"kind": "cell", "row": bath, "column": column, "value": words}}),
        );
    }
    let doc = &app.document.as_ref().expect("a drawing").model;
    let finishes = doc
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Space(s) if s.space.name.as_deref() == Some("Banyo") => {
                Some(s.space.finishes.clone())
            }
            _ => None,
        })
        .expect("the bathroom");
    assert_eq!(finishes.floor.as_deref(), Some("Seramik"));
    assert_eq!(finishes.sill.as_deref(), Some("Mermer denizlik"));
    assert_eq!(
        rows(&app)[bath][9],
        "Mermer denizlik",
        "the list follows its rooms"
    );
    // A door's type's code changes: the list says the new one in the same step.
    run(
        &mut app,
        "cad.openingtypes.edit",
        json!({"operation": {"kind": "code", "type": "İç kapı 90", "code": "K10"}}),
    );
    let now = rows(&app);
    assert_eq!(now[2][7], "K10", "{:?}", now[2]);
    assert_eq!(now[bath][7], "K10 (2)", "{:?}", now[bath]);
    // The room lists in the drawing: the one placed, until undone.
    let lists = |app: &App| {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .filter(|e| {
                matches!(e, kentos_commands::Entity::Table(t)
                    if matches!(&t.source, Some(kentos_commands::TableSource::Schedule { columns, .. })
                        if columns.iter().any(|c| c.field.is_room_list())))
            })
            .count()
    };
    assert_eq!(lists(&app), 1);
    // The code, the two cells, then the list itself: one step each.
    for _ in 0..3 {
        let _ = app.update(Message::Run("edit.undo"));
    }
    assert_eq!(lists(&app), 1);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(lists(&app), 0, "one undo takes the list away");
}
