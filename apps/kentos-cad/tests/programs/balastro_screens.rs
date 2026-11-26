//! Pictures of docs/adr/9315 for the owner: a new Harita project, its Nokta layer on top of the tree, three named
//! points written on it (Noktalar's Satır ekle) and lines on Çizim meeting at them: each point a balastro, 0.75 mm
//! across at the plot scale, the lines not showing inside it. Not run by default:
//!
//! ```text
//! cargo test -p kentos-cad --test programs balastro_screens -- --ignored --nocapture --test-threads=1
//! ```
//!
//! They go to `.run/gui-evidence/kullanim-9315/`.

use iced::Size;
use kentos_commands::{CadWork, DrawingFont, Entity, Workspace};
use kentos_project::new_project::{NewProject, new_project};
use kentos_ui::snapshot::Snapshot;
use serde_json::json;

use crate::app::{App, Message};
use crate::points::edit::{Draft, write_draft};

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    crate::drawing_fonts::load_font_folders();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-9315");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    // The glyph atlas makes a frame's new letters until a deadline; a picture draws its frames by hand, so a few first.
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
    let snapshot_v1 = new_project(&NewProject {
        name: "Balastro".to_owned(),
        srid: 5256,
        plot_scale: 1000.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Arial,
        province: None,
        drawing_unit: None,
        annotation: None,
        cad_work: Some(CadWork::Map),
        heights: None,
    })
    .expect("a new Harita project");
    assert_eq!(snapshot_v1.layers[0].id, "nokta", "Nokta on top");
    let (mut app, _) = App::boot(None);
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    settle(&mut snapshot, &mut app);
    let origin = snapshot_v1.origin;
    let document = crate::document::Document::new(snapshot_v1, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(document)))));
    // Three named points on Nokta, and the lines between them on Çizim.
    let corners = [(0.0, 0.0), (20.0, 0.0), (20.0, 15.0)];
    {
        let doc = &mut app.document.as_mut().expect("open").model;
        for (i, (dx, dy)) in corners.iter().enumerate() {
            let draft = Draft {
                name: format!("{}", 101 + i),
                east: format!("{}", origin.x + dx),
                north: format!("{}", origin.y + dy),
                ..Draft::default()
            };
            let written = write_draft(doc, &draft, "nokta", None);
            assert!(written.outcome.step.is_some(), "{:?}", written.outcome.said);
        }
        for w in [(0, 1), (1, 2), (2, 0)] {
            let (a, b) = (corners[w.0], corners[w.1]);
            let line: Entity = serde_json::from_value(json!({
                "kind": "line", "id": 0, "layerId": "cizim", "attrs": {},
                "a": {"x": origin.x + a.0, "y": origin.y + a.1},
                "b": {"x": origin.x + b.0, "y": origin.y + b.1},
            }))
            .expect("a line");
            assert!(doc.add(line).is_ok());
        }
    }
    // The drawing area takes the edits made beside the app at its next message: the first fit reads them, the second
    // fits them.
    let _ = app.update(Message::Run("view.zoomExtents"));
    settle(&mut snapshot, &mut app);
    let _ = app.update(Message::Run("view.zoomExtents"));
    settle(&mut snapshot, &mut app);
    save(&mut snapshot, &app, "nokta-katmani-balastro");
}
