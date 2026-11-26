//! Pictures of docs/adr/9310 and 9311 for the owner, every module's parts registered (a module's surfaces are its
//! profile's): İşlem araç kutusu in a Harita and in a CBS project, each with its
//! own tools; Köşe noktalarını numarala's names written at the corners as text objects in Arial (the operating system's
//! own, given here as the program's start gives it), the layer's labels off. Not run by default:
//!
//! ```text
//! cargo test -p kentos-cad --test programs tool_modules_screens -- --ignored --nocapture --test-threads=1
//! ```
//!
//! They go to `.run/gui-evidence/kullanim-9310/`.

use iced::Size;
use kentos_commands::{CadWork, DocumentSnapshotV1, DrawingFont, Workspace};
use kentos_domain::Slot;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::gis;

const PARCELS: &str = include_str!("../../../../fixtures/processing/v1/parcels.kcad");

/// The sample parcels as a project of this type and work, drawn in Arial.
fn parcels(workspace: Workspace, work: Option<CadWork>) -> crate::document::Document {
    let mut snapshot = DocumentSnapshotV1::from_json(PARCELS).expect("the parcels read");
    snapshot.settings.workspace = Some(workspace);
    snapshot.settings.cad_work = work;
    snapshot.settings.drawing_font = Some(DrawingFont::Arial);
    crate::document::Document::new(snapshot, None).expect("the parcels open")
}

fn processing(app: &mut App, e: crate::processing::Event) {
    let _ = app.update(Message::Gis(gis::Event::Processing(e)));
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let found = crate::drawing_fonts::give_system_faces();
    println!("the system's own faces: {found:?}");
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-9310");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let save = |snapshot: &mut Snapshot, app: &App, name: &str| {
        let file = out.join(format!("{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    };
    // The area settles at its size before the drawing opens, as the tools' pictures do, so the drawing is fitted to it.
    let open = |workspace: Workspace, work: Option<CadWork>| -> (App, Snapshot) {
        let (mut app, _) = App::boot(None);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut |app: &mut App, m| {
            let _ = app.update(m);
        });
        let _ = app.update(Message::Opened(Some(Ok(Box::new(parcels(workspace, work))))));
        let _ = app.update(Message::Run("view.zoomExtents"));
        (app, snapshot)
    };
    let settle = |snapshot: &mut Snapshot, app: &mut App| {
        snapshot.settle(app, App::view, &mut |app: &mut App, m| {
            let _ = app.update(m);
        });
    };
    // The toolbox of each module: Harita's and CBS's trees differ (docs/adr/9310).
    for (name, workspace, work) in [
        ("arac-kutusu-harita", Workspace::Cad, Some(CadWork::Map)),
        ("arac-kutusu-cbs", Workspace::Gis, None),
    ] {
        let (mut app, mut snapshot) = open(workspace, work);
        let _ = app.update(Message::Run("processing.toolbox"));
        settle(&mut snapshot, &mut app);
        save(&mut snapshot, &app, name);
    }
    // Köşe noktalarını numarala in Harita, its defaults: the names are texts at the corners, the labels off.
    let (mut app, mut snapshot) = open(Workspace::Cad, Some(CadWork::Map));
    settle(&mut snapshot, &mut app);
    app.selection.set([Slot(1), Slot(2)]);
    let _ = app.update(Message::Run("processing.run.points.numberVertices"));
    settle(&mut snapshot, &mut app);
    save(&mut snapshot, &app, "kose-numarala-pencere");
    processing(&mut app, crate::processing::Event::Run);
    processing(&mut app, crate::processing::Event::Close);
    app.selection.clear();
    settle(&mut snapshot, &mut app);
    save(&mut snapshot, &app, "kose-adlari-yazi");
}
