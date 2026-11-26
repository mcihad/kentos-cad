//! Pictures of docs/adr/9313 for the owner, the fonts folders read as the program's start reads them:
//! - Köşe noktalarını numarala's names upright, in Arial and in Roboto (a family of the fonts folder);
//! - Yazı tipleri with the folders' files and what KentOS does with each;
//! - Proje ayarları with Roboto as the project's typeface.
//!
//! Not run by default:
//!
//! ```text
//! cargo test -p kentos-cad --test programs font_folders_screens -- --ignored --nocapture --test-threads=1
//! ```
//!
//! They go to `.run/gui-evidence/kullanim-9313/`.

use iced::Size;
use kentos_commands::{CadWork, DocumentSnapshotV1, DrawingFont, Workspace};
use kentos_domain::Slot;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::gis;

const PARCELS: &str = include_str!("../../../../fixtures/processing/v1/parcels.kcad");

/// The sample parcels as a Harita project drawn in `font`.
fn parcels(font: DrawingFont) -> crate::document::Document {
    let mut snapshot = DocumentSnapshotV1::from_json(PARCELS).expect("the parcels read");
    snapshot.settings.workspace = Some(Workspace::Cad);
    snapshot.settings.cad_work = Some(CadWork::Map);
    snapshot.settings.drawing_font = Some(font);
    crate::document::Document::new(snapshot, None).expect("the parcels open")
}

fn processing(app: &mut App, e: crate::processing::Event) {
    let _ = app.update(Message::Gis(gis::Event::Processing(e)));
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let given = crate::drawing_fonts::load_font_folders();
    println!("the system's own faces: {given:?}");
    let roboto = DrawingFont::from_name("Roboto").expect("a family");
    // Roboto is in paket/fonts/arayuz: the text system draws it in its own face, not Arial's.
    assert_eq!(crate::drawing_fonts::family(roboto), "Roboto");
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-9313");
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
    // The area settles at its size before the drawing opens, so the drawing is fitted to it.
    let open = |font: DrawingFont| -> (App, Snapshot) {
        let (mut app, _) = App::boot(None);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut |app: &mut App, m| {
            let _ = app.update(m);
        });
        let _ = app.update(Message::Opened(Some(Ok(Box::new(parcels(font))))));
        let _ = app.update(Message::Run("view.zoomExtents"));
        (app, snapshot)
    };
    // The corners named as texts, upright: the project's Arial, then the folder's Roboto.
    for (name, font) in [("kose-adlari-arial", DrawingFont::Arial), ("kose-adlari-roboto", roboto)] {
        let (mut app, mut snapshot) = open(font);
        settle(&mut snapshot, &mut app);
        app.selection.set([Slot(1), Slot(2)]);
        let _ = app.update(Message::Run("processing.run.points.numberVertices"));
        settle(&mut snapshot, &mut app);
        processing(&mut app, crate::processing::Event::Run);
        processing(&mut app, crate::processing::Event::Close);
        app.selection.clear();
        settle(&mut snapshot, &mut app);
        save(&mut snapshot, &app, name);
        if font == roboto {
            // Yazı tipleri: the drawing's faces and the fonts folders' files.
            let _ = app.update(Message::Run("text.fonts"));
            settle(&mut snapshot, &mut app);
            save(&mut snapshot, &app, "yazi-tipleri-penceresi");
        }
    }
    // Proje ayarları: Roboto is the project's typeface, one of the list's.
    let (mut app, mut snapshot) = open(roboto);
    settle(&mut snapshot, &mut app);
    let _ = app.update(Message::Run("file.settings"));
    settle(&mut snapshot, &mut app);
    save(&mut snapshot, &app, "proje-ayarlari-yazi-tipi");
}
