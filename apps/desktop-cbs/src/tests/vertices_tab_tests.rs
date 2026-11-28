//! vertices › tests's tests that need the CBS module's surfaces (its project's bottom tabs and panels; docs/adr/9199
//! §4.14): moved from the core's cad/vertices/tests.rs with the module's package; the work they test is the core's.

use crate::vertices::edit::{At, Column};
use kentos_domain::Slot;

fn app_with_vertices() -> crate::app::App {
    let (mut app, _) = crate::app::App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
        "../../../../fixtures/interaction/v1/vertex-table.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(crate::app::Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn ev(app: &mut crate::app::App, e: crate::vertices::Event) {
    let _ = app.update(crate::app::Message::Vertices(e));
}

fn parcel_x(app: &crate::app::App, i: usize) -> f64 {
    match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_commands::Entity::Polygon(a)) => a.pts[i].x,
        _ => panic!("the parcel"),
    }
}

/// The table in the app: the parcel's rows, a row selected and ringed, a Y
/// typed in place (Enter writes it and opens the row below), a radius
/// refused with the cell open and said; two objects or a locked layer show
/// the table without writing it.
#[test]
fn the_table_selects_rows_and_edits_cells_in_place() {
    use crate::app::Message;
    use crate::vertices::{Cell, Event, Walk, texts};
    let mut app = app_with_vertices();
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("view.coords"));
    let target = app.vertex_target().expect("the parcel in the table");
    assert!(target.writes);
    assert_eq!(app.vertex_rows(Slot(1)).len(), 13);
    // A press selects a row and rings its vertex.
    ev(&mut app, Event::Press(2));
    assert_eq!(app.vertex_marks().len(), 1);
    // Y of the second row: its whole value, then written with Enter, the row below open.
    ev(&mut app, Event::Edit(1, Column::East));
    assert_eq!(app.vertices.text, "486760");
    ev(&mut app, Event::Input("486761.5".into()));
    ev(&mut app, Event::Finish(Some(Walk::Down)));
    assert_eq!(parcel_x(&app, 1), 486761.5);
    assert_eq!(
        app.vertices.editing,
        Some((Cell::Vertex(At { path: 0, index: 2 }), Column::East))
    );
    assert_eq!(app.vertices.text, "486780");
    // The row selection survives the write: the vertex count is the same.
    assert_eq!(app.vertex_marks().len(), 1);
    ev(&mut app, Event::Cancel);
    // A radius shorter than half the chord (the first edge, its end moved above): refused, said,
    // the cell kept open with what was typed.
    ev(&mut app, Event::Edit(0, Column::Radius));
    ev(&mut app, Event::Input("10".into()));
    ev(&mut app, Event::Finish(Some(Walk::Down)));
    assert_eq!(
        app.vertices.editing,
        Some((Cell::Vertex(At { path: 0, index: 0 }), Column::Radius))
    );
    assert_eq!(app.vertices.text, "10");
    let said = app.log.last().map(|l| l.text.clone()).unwrap_or_default();
    assert_eq!(
        said,
        "Köşe tablosu: Yarıçap kirişin yarısından küçük olamaz: en az 30.851 m."
    );
    ev(&mut app, Event::Cancel);
    // Tab walks right, over the cells that are edited: Z to Yarıçap, Yarıçap to the next row's Y.
    ev(&mut app, Event::Edit(0, Column::Z));
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    assert_eq!(
        app.vertices.editing,
        Some((Cell::Vertex(At { path: 0, index: 0 }), Column::Radius))
    );
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    assert_eq!(
        app.vertices.editing,
        Some((Cell::Vertex(At { path: 0, index: 1 }), Column::East))
    );
    ev(&mut app, Event::Cancel);
    // Satır ekle under the first row: Y, Tab, X, Enter adds the vertex after the first and opens
    // the next draft under it; Esc leaves it.
    ev(&mut app, Event::Press(0));
    assert!(app.vertices.keyboard);
    ev(&mut app, Event::AddRow);
    assert_eq!(app.vertices.editing, Some((Cell::Draft, Column::East)));
    ev(&mut app, Event::Input("486730".into()));
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    assert_eq!(app.vertices.editing, Some((Cell::Draft, Column::North)));
    ev(&mut app, Event::Input("4420190".into()));
    ev(&mut app, Event::Finish(Some(Walk::Down)));
    let outer =
        |app: &crate::app::App| match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
            Some(kentos_commands::Entity::Polygon(a)) => a.pts.clone(),
            _ => panic!("the parcel"),
        };
    assert_eq!(outer(&app).len(), 6);
    assert_eq!((outer(&app)[1].x, outer(&app)[1].y), (486730.0, 4420190.0));
    assert_eq!(app.vertices.editing, Some((Cell::Draft, Column::East)));
    assert_eq!(app.vertex_rows(Slot(1)).len(), 14);
    ev(&mut app, Event::Cancel);
    assert!(app.vertices.draft.is_none());
    // Sil: the new vertex's row removed in one step.
    ev(&mut app, Event::Press(1));
    ev(&mut app, Event::Remove);
    assert_eq!(outer(&app).len(), 5);
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Köşe sil"));
    // Two objects: the first shown, not written.
    app.selection.set([Slot(1), Slot(2)]);
    let target = app.vertex_target().expect("the first in the table");
    assert_eq!(
        (target.slot, target.writes, target.note),
        (Slot(1), false, texts::MANY_NOTE)
    );
    ev(&mut app, Event::Edit(0, Column::East));
    assert!(!app.vertices.editing());
    // A locked layer's polyline: shown, not written.
    app.selection.set([Slot(4)]);
    let target = app.vertex_target().expect("the locked polyline");
    assert_eq!((target.writes, target.note), (false, texts::LOCKED_NOTE));
    // A point is the point list's.
    app.selection.set(Vec::<Slot>::new());
    assert!(app.vertex_target().is_none());
}
