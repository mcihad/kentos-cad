//! Baskı stili tablosu (plot_styles_window.rs, docs/adr/1130): the window previews `cad.plotstyles.import`'s plan
//! without writing, gives a pen to a layer by hand, applies the profile to a sheet and writes it all in one step that
//! one undo takes back; Kapat writes nothing; a file that is no table is said in the window.

use crate::app::{App, Dialog, Message};
use crate::plot_styles_window::Event as P;

/// The drawing of `cad.plotstyles.import`'s shared cases (two layers, a book of two sheets), opened.
pub(crate) fn app() -> App {
    let (mut app, _) = App::boot(None);
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.plotstyles.import.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(fx["setup"].clone()).expect("a drawing");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

pub(crate) fn table(name: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/commands/v1/plotstyles").join(name);
    p.to_string_lossy().into_owned()
}

fn send(app: &mut App, e: P) {
    let _ = app.update(Message::PlotStyles(e));
}

fn book(app: &App) -> Option<kentos_sheet::SheetBook> {
    let doc = app.document.as_ref().expect("a drawing");
    doc.model.sheets().map(|s| kentos_sheet::project::read_project_sheets(s).expect("reads").0)
}

#[test]
fn the_window_previews_gives_a_pen_applies_and_one_undo_takes_it_back() {
    let mut app = app();
    let _ = app.update(Message::Run("sheet.plotStyles"));
    assert_eq!(app.dialog, Some(Dialog::PlotStyles));
    send(&mut app, P::Picked(Some(table("siyah.ctb").into())));
    let w = app.plot_styles.as_ref().expect("open");
    let out = w.result.clone().expect("previewed").expect("plans");
    assert_eq!((out.styles, out.exact, out.format.as_str()), (255, 255, "ctb"));
    // Pen 1 to Duvar (the second layer row), the profile to A-101.
    let duvar = w.pens.iter().position(|p| p.0 == "duvar").expect("Duvar");
    send(&mut app, P::Pen(duvar, 1));
    send(&mut app, P::Sheet(0, true));
    // The preview wrote nothing.
    assert!(book(&app).is_some_and(|b| b.plot_profiles.is_empty()));
    assert!(!app.document.as_ref().expect("a drawing").model.can_undo());
    send(&mut app, P::Import);
    assert!(app.dialog.is_none(), "the window closes");
    let b = book(&app).expect("a book");
    assert_eq!(b.plot_profiles.len(), 1);
    let p = &b.plot_profiles[0];
    assert_eq!(p.name, "siyah");
    assert_eq!(p.entries[0].style.line_weight, Some(500), "pen 1 is 0,50 mm in the table");
    assert_eq!(serde_json::to_value(&p.assignments).expect("json")[0]["target"]["id"], "duvar");
    let a101 = b.sheet("a-101").expect("A-101");
    assert_eq!(a101.plot.as_ref().and_then(|x| x.settings.profile.clone()), Some(p.id.clone()));
    assert!(b.sheet("a-102").expect("A-102").plot.is_none(), "A-102 was not chosen");
    let _ = app.update(Message::Run("edit.undo"));
    let after = book(&app).expect("a book");
    assert!(after.plot_profiles.is_empty(), "one undo takes the profile back");
    assert!(after.sheet("a-101").expect("A-101").plot.is_none(), "and the sheet's plot settings");
}

#[test]
fn closing_writes_nothing_and_a_file_that_is_no_table_is_said() {
    let mut app = app();
    let _ = app.update(Message::Run("sheet.plotStyles"));
    send(&mut app, P::Path(table("metin.ctb")));
    let said = app.plot_styles.as_ref().and_then(|w| w.result.clone()).expect("previewed");
    assert!(said.as_ref().is_err_and(|e| e.contains(".ctb/.stb")), "{said:?}");
    send(&mut app, P::Picked(Some(table("stiller.stb").into())));
    let w = app.plot_styles.as_ref().expect("open");
    let out = w.result.clone().expect("previewed").expect("plans");
    assert_eq!(out.format, "stb");
    assert_eq!(w.keys().len(), 3, "Normal is the object's look, not a style of the profile");
    send(&mut app, P::Close);
    assert!(app.dialog.is_none());
    assert!(book(&app).is_some_and(|b| b.plot_profiles.is_empty()), "Kapat wrote nothing");
}
