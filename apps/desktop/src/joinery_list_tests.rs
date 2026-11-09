//! Grafik doğrama listesi (M38, docs/adr/3038 §6): the acceptance apartment's door and window types side by side, each
//! with its code, its elevation with Türkiye's opening lines, its rough size, sill, opening, frame and leaves, U value
//! and count; the list is a block the document writes again when a type or an opening changes; in the permit set it is
//! a sheet of its own at 1/50 (EK-2 LS/MMC).

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

/// The list's block's words, in its order.
fn words(app: &App) -> Vec<String> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.block(kentos_domain::joinery_list::JOINERY_LIST_ID)
        .map(|b| {
            b.entities
                .iter()
                .filter_map(|e| match e {
                    kentos_commands::Entity::Text(t) => Some(t.text.clone()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// How many inserts of the list the drawing has.
fn inserts(app: &App) -> usize {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.block_uses(kentos_domain::joinery_list::JOINERY_LIST_ID)
}

#[cfg(unix)]
#[test]
fn the_apartment_s_joinery_list_draws_each_type_and_follows_the_drawing() {
    let (mut app, _dir) = crate::acceptance_flat_tests::opened("dograma-listesi");
    let _ = crate::acceptance_flat_tests::drawn(&mut app);
    let _ = app.update(Message::Run("tool.openingCodes"));
    let _ = app.submit_line("");
    // The windows' frame, glass and U value (docs/adr/3038 §5).
    run(
        &mut app,
        "cad.openingtypes.edit",
        json!({"operation": {"kind": "update", "type": "Pencere 120", "change": {
            "material": "PVC, 5 odacıklı; çift camlı Low-E 4+16+4", "uValue": 1.6}}}),
    );
    let _ = app.update(Message::Run("tool.joineryList"));
    assert_eq!(app.session.tool_id(), "joineryList");
    let _ = app.submit_line("30,0");
    let said = crate::files_testing::last_said(&app);
    assert_eq!(
        said,
        "Doğrama listesi: 4 tip (GK01, K01, P01, P02), 10 açıklık; tipler değişince liste kendini yeniden yazar.",
    );
    assert_eq!(inserts(&app), 1);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let block = doc
        .block(kentos_domain::joinery_list::JOINERY_LIST_ID)
        .expect("the list's block");
    assert_eq!(block.name, "MM-XXXX-DOGRAMALISTESI");
    let w = words(&app);
    // The labels, then each type's column: its code, its size's dimensions, its cells, its count.
    for label in kentos_domain::joinery_list::ROWS {
        assert!(w.iter().any(|x| x == label), "{label}: {w:?}");
    }
    for want in [
        "GK01",
        "K01",
        "P01",
        "P02",
        "100/210",
        "90/210",
        "120/130",
        "60/70",
        "Kanatlı",
        "PVC, 5 odacıklı; çift",
        "camlı Low-E 4+16+4",
        "1,6",
        "4",
        "1",
    ] {
        assert!(w.iter().any(|x| x == want), "{want}: {w:?}");
    }
    // The order: GK01, K01, P01, P02.
    let at = |c: &str| w.iter().position(|x| x == c).unwrap();
    assert!(
        at("GK01") < at("K01") && at("K01") < at("P01") && at("P01") < at("P02"),
        "{w:?}"
    );
    // A window's U value changes: the list writes it in the same step.
    run(
        &mut app,
        "cad.openingtypes.edit",
        json!({"operation": {"kind": "update", "type": "Pencere 120", "change": {"uValue": 1.4}}}),
    );
    let w = words(&app);
    assert!(
        w.iter().any(|x| x == "1,4") && !w.iter().any(|x| x == "1,6"),
        "{w:?}"
    );
    // One undo takes the U value back, list and all.
    let _ = app.update(Message::Run("edit.undo"));
    assert!(words(&app).iter().any(|x| x == "1,6"));
    // The permit set: the list a sheet of its own at 1/50 after the room list (EK-2 LS/MMC).
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let tables = kentos_interaction::calc_tables::ensure(doc).expect("the tables");
    let joinery = tables
        .iter()
        .find(|t| t.label == "Doğrama listesi")
        .expect("the joinery list");
    assert_eq!((joinery.list, joinery.scale), (Some("MMC"), Some(50)));
    assert_eq!(inserts(&app), 1, "found, not placed again");
    let labels: Vec<&str> = tables.iter().map(|t| t.label).collect();
    assert_eq!(
        &labels[..2],
        ["Mahal listesi", "Doğrama listesi"],
        "{labels:?}"
    );
    // Its sheet: after the room list's, on its own paper, at 1/50 (its elevations are drawn to scale), EK-2 LS/MMC.
    let plot = 100;
    let input = kentos_native_application::permit_set::PermitSet {
        phase: kentos_sheet::permit::Phase::Permit,
        values: crate::permit_set_tests::particulars(),
        tables: tables
            .iter()
            .filter(|t| t.list.is_none())
            .map(|t| (t.label.to_owned(), t.bounds))
            .collect(),
        lists: tables
            .iter()
            .filter_map(|t| {
                t.list
                    .map(|kind| kentos_native_application::permit_set::ListSheet {
                        title: t.label.to_owned(),
                        kind,
                        bounds: t.bounds,
                        scale: t.scale.unwrap_or(plot),
                        scaled: t.scale.is_some(),
                    })
            })
            .collect(),
    };
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let (_, made, notes) = kentos_native_application::permit_set::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        &input,
    )
    .expect("made");
    assert!(notes.is_empty(), "{notes:?}");
    let titles: Vec<&str> = made.iter().map(|m| m.title.as_str()).collect();
    let at = titles
        .iter()
        .position(|t| *t == "Doğrama listesi")
        .expect("its sheet");
    assert_eq!(titles[at - 1], "Mahal listesi", "{titles:?}");
    let sheet = crate::permit_set_tests::texts(&app, &made[at].sheet);
    for want in ["Doğrama listesi", "1/50", "_LS_MMC_NNN_01_000"] {
        assert!(sheet.iter().any(|x| x.contains(want)), "{want}: {sheet:?}");
    }
    let rooms = crate::permit_set_tests::texts(&app, &made[at - 1].sheet);
    assert!(
        !rooms.iter().any(|x| x.contains("1/100")),
        "the room list has no scale: {rooms:?}"
    );
    assert_eq!(inserts(&app), 1);
}
