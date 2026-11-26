//! Pictures of docs/adr/9314 for the owner, a Harita project in Arial:
//! - three points written with their names (Noktalar's Satır ekle, as the Nokta tool writes them): each name a text
//!   up and right of its point, linked to it;
//! - one point moved 6 m north, its name with it; another renamed in Noktalar, its text with the new name.
//!
//! Not run by default:
//!
//! ```text
//! cargo test -p kentos-cad --test programs point_names_screens -- --ignored --nocapture --test-threads=1
//! ```
//!
//! They go to `.run/gui-evidence/kullanim-9314/`.

use iced::Size;
use kentos_commands::{CadWork, DocumentSnapshotV1, DrawingFont, Entity, Workspace};
use kentos_domain::Slot;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::points::edit::{Draft, EditColumn, write_cell, write_draft};

const PARCELS: &str = include_str!("../../../../fixtures/processing/v1/parcels.kcad");

/// The sample parcels as a Harita project drawn in Arial.
fn parcels() -> crate::document::Document {
    let mut snapshot = DocumentSnapshotV1::from_json(PARCELS).expect("the parcels read");
    snapshot.settings.workspace = Some(Workspace::Cad);
    snapshot.settings.cad_work = Some(CadWork::Map);
    snapshot.settings.drawing_font = Some(DrawingFont::Arial);
    crate::document::Document::new(snapshot, None).expect("the parcels open")
}

/// The slot of the point named `name`.
fn point_named(app: &App, name: &str) -> Slot {
    let doc = &app.document.as_ref().expect("open").model;
    doc.entities()
        .find(|e| matches!(e, Entity::Point(p) if p.base.label.as_deref() == Some(name)))
        .map(|e| Slot(e.base().id))
        .expect("the point")
}

/// The text linked to the point in `slot`.
fn name_of(app: &App, slot: Slot) -> (String, f64, f64) {
    let doc = &app.document.as_ref().expect("open").model;
    let texts = doc.linked_texts(slot);
    let Some(Entity::Text(t)) = texts.first().and_then(|s| doc.get(*s)) else {
        panic!("the point's name")
    };
    (t.text.clone(), t.p.x, t.p.y)
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    crate::drawing_fonts::load_font_folders();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-9314");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    // The glyph atlas makes a frame's new letters until a deadline and leaves the rest for the next frame, which a
    // window asks for (docs/adr/9212 §5); a picture draws its frames by hand, so a few first.
    let save = |snapshot: &mut Snapshot, app: &App, name: &str| {
        let file = out.join(format!("{name}.png"));
        for _ in 0..8 {
            let _ = snapshot.render(app.view(), &app.theme());
        }
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    };
    let settle = |snapshot: &mut Snapshot, app: &mut App| {
        snapshot.settle(app, App::view, &mut |app: &mut App, m| {
            let _ = app.update(m);
        });
    };
    let (mut app, _) = App::boot(None);
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    settle(&mut snapshot, &mut app);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(parcels())))));
    // Three points with their names, as Noktalar's Satır ekle writes them in Harita.
    let layer = app.document.as_ref().expect("open").model.layers().active().to_owned();
    for (name, east, north) in [("N1", "487000", "4420020"), ("N2", "487015", "4420020"), ("N3", "487030", "4420020")] {
        let doc = &mut app.document.as_mut().expect("open").model;
        let draft = Draft {
            name: name.to_owned(),
            east: east.to_owned(),
            north: north.to_owned(),
            ..Draft::default()
        };
        let written = write_draft(doc, &draft, &layer, None);
        assert!(written.outcome.step.is_some(), "{name}: {:?}", written.outcome.said);
    }
    let (n2, n3) = (point_named(&app, "N2"), point_named(&app, "N3"));
    let before = name_of(&app, n2);
    let _ = app.update(Message::Run("view.zoomExtents"));
    let _ = app.update(Message::BottomTab(crate::bottom::BottomTab::Points));
    settle(&mut snapshot, &mut app);
    save(&mut snapshot, &app, "nokta-adlari-yazi");
    // N2 moved 6 m north: its name goes with it. N3 renamed N30 in Noktalar: its text follows.
    {
        let doc = &mut app.document.as_mut().expect("open").model;
        let Some(Entity::Point(mut moved)) = doc.get(n2).cloned() else {
            panic!("N2")
        };
        moved.p.y += 6.0;
        assert!(doc.update(n2, Entity::Point(moved)));
        let renamed = write_cell(doc, n3, EditColumn::Name, "N30", false);
        assert!(renamed.step.is_some(), "{:?}", renamed.said);
    }
    let after = name_of(&app, n2);
    assert_eq!((after.1 - before.1, after.2 - before.2), (0.0, 6.0), "N2's name moved with it");
    assert_eq!(name_of(&app, n3).0, "N30", "N3's text took its new name");
    // The drawing area takes the edits made beside the app at its next message, as after a command.
    let _ = app.update(Message::Run("view.zoomExtents"));
    settle(&mut snapshot, &mut app);
    save(&mut snapshot, &app, "nokta-tasindi-adi-degisti");
}
