//! Pictures of docs/adr/9316 for the owner:
//! - a CBS project's points with Nokta etiketleri and Nokta kodları on (Görünüm › Görünüş);
//! - a Harita project's point written without a name and named later in Noktalar: its name written as a text;
//! - a Harita drawing layer's and a CBS data layer's Görünüş ▸ (no labels, no style files in the map).
//!
//! Not run by default:
//!
//! ```text
//! cargo test -p kentos-cad --test programs label_rule_screens -- --ignored --nocapture --test-threads=1
//! ```
//!
//! They go to `.run/gui-evidence/kullanim-9316/`.

use iced::Size;
use kentos_commands::{CadWork, DocumentSnapshotV1, DrawingFont, Entity, Workspace};
use kentos_domain::Slot;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::points::edit::{Draft, EditColumn, write_cell, write_draft};

const POINTS: &str = include_str!("../../../../fixtures/interaction/v1/point-editor.kcad");

/// The sample points as a project of `workspace` (and `cad_work`), drawn in Arial.
fn points(workspace: Workspace, cad_work: Option<CadWork>) -> crate::document::Document {
    let mut snapshot = DocumentSnapshotV1::from_json(POINTS).expect("the points read");
    snapshot.settings.workspace = Some(workspace);
    snapshot.settings.cad_work = cad_work;
    snapshot.settings.drawing_font = Some(DrawingFont::Arial);
    crate::document::Document::new(snapshot, None).expect("the points open")
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    crate::drawing_fonts::load_font_folders();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/kullanim-9316");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    // The glyph atlas makes a frame's new letters until a deadline (docs/adr/9212 §5): a few frames first.
    let save = |snapshot: &mut Snapshot, app: &App, name: &str| {
        let file = out.join(format!("{name}.png"));
        for _ in 0..8 {
            let _ = snapshot.render(app.view(), &app.theme());
        }
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    };
    let settle = |snapshot: &mut Snapshot, app: &mut App| {
        snapshot.settle(app, App::view, &mut |app: &mut App, m| {
            let _ = app.update(m);
        });
    };

    // CBS: Nokta etiketleri and Nokta kodları on its Görünüm tab's Görünüş panel.
    let (mut app, _) = App::boot(None);
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    settle(&mut snapshot, &mut app);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(points(
        Workspace::Gis,
        None,
    ))))));
    let _ = app.update(Message::Run("view.pointLabels"));
    let _ = app.update(Message::Run("view.pointCodes"));
    assert!(app.settings.bool("graphics.pointLabels"));
    let _ = app.update(Message::Run("view.zoomExtents"));
    let _ = app.update(Message::RibbonTab("view"));
    settle(&mut snapshot, &mut app);
    save(&mut snapshot, &app, "cbs-nokta-etiketleri");

    // Harita: a point written without a name, then named in Noktalar.
    let (mut app, _) = App::boot(None);
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    settle(&mut snapshot, &mut app);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(points(
        Workspace::Cad,
        Some(CadWork::Map),
    ))))));
    let slot = {
        let doc = &mut app.document.as_mut().expect("open").model;
        let layer = doc.layers().active().to_owned();
        let at = doc
            .entities()
            .find_map(|e| match e {
                Entity::Point(p) => Some(p.p),
                _ => None,
            })
            .expect("a point");
        let draft = Draft {
            east: format!("{}", at.x + 6.0),
            north: format!("{}", at.y + 4.0),
            ..Draft::default()
        };
        assert!(
            write_draft(doc, &draft, &layer, None)
                .outcome
                .step
                .is_some()
        );
        let slot = doc
            .entities()
            .filter(|e| matches!(e, Entity::Point(_)))
            .map(|e| Slot(e.base().id))
            .max()
            .expect("the new point");
        let named = write_cell(doc, slot, EditColumn::Name, "N7", false);
        assert!(named.step.is_some(), "{:?}", named.said);
        assert_eq!(doc.linked_texts(slot).len(), 1, "its name is a text");
        slot
    };
    app.selection.set([slot]);
    let _ = app.update(Message::Run("view.zoomExtents"));
    let _ = app.update(Message::BottomTab(crate::bottom::BottomTab::Points));
    settle(&mut snapshot, &mut app);
    save(&mut snapshot, &app, "harita-sonradan-verilen-ad");
}
