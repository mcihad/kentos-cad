//! The pictures of docs/adr/0192 (Resim nesnesi) in a CAD project at 1:500,
//! the shared traces' ground (`images.kcad`): an embedded site picture and a
//! parcel. Resim ekle with its frame following the cursor, the picture placed
//! beside the site and Öznitelikler's rows; Resmi kırp with its rectangle
//! following the cursor, and the site clipped. The web's are `shots.mjs
//! images`. `tools_screens` takes them in the dark and the light theme at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=resim-ekle,resim-ekle-yazildi,resim-kirp,resim-kirpildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::document::Document;
use crate::tools_scenes::{E, N, click, hover, run, typed};
use crate::tools_screens::Scene;

const DRAWING: &str = include_str!("../../../fixtures/interaction/v1/images.kcad");
const LOGO: &[u8] = include_bytes!("../../../fixtures/interaction/v1/logo.png");

/// The traces' drawing, the view around the site and the parcel.
fn opened(app: &mut App) {
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "insert";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 26.0,
            min_y: N - 20.0,
            max_x: E + 46.0,
            max_y: N + 26.0,
        },
        24.0,
    );
}

/// Resim ekle running with the logo given, as a file beside the drawing.
fn inserting(app: &mut App) {
    opened(app);
    run(app, "tool.imageInsert");
    app.image_file_wanted = false;
    app.image_file_given(Some((
        "logo.png".to_owned(),
        Some("logo.png".to_owned()),
        LOGO.to_vec(),
    )));
}

/// Its corner clicked, the frame and its diagonals following the cursor.
fn insert(app: &mut App) {
    inserting(app);
    click(app, [24.0, 8.0]);
    hover(app, [41.0, 18.0]);
}

/// The logo placed 16 m wide and selected: Öznitelikler's rows, the layers folded away.
fn inserted(app: &mut App) {
    inserting(app);
    click(app, [24.0, 8.0]);
    typed(app, "16");
    app.selection.set([Slot(3)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    hover(app, [44.0, -14.0]);
}

/// Resmi kırp with the site picked and the rectangle's first corner given.
fn clip(app: &mut App) {
    opened(app);
    app.tab = "home";
    run(app, "tool.imageClip");
    click(app, [0.0, -15.0]);
    click(app, [-12.0, -6.0]);
    hover(app, [10.0, 10.0]);
}

/// The site clipped to its middle, a third see-through, selected.
fn clipped(app: &mut App) {
    opened(app);
    run(app, "tool.imageClip");
    click(app, [0.0, -15.0]);
    click(app, [-12.0, -6.0]);
    click(app, [10.0, 10.0]);
    let _ = app.update(Message::Run("tool.cancel"));
    if let Some(doc) = app.document.as_mut()
        && let Some(kentos_contracts::Entity::Image(mut i)) = doc.model.get(Slot(1)).cloned()
    {
        i.image.opacity = Some(0.7);
        let _ = kentos_interaction::properties::set_geometry(
            &mut doc.model,
            Slot(1),
            &kentos_contracts::Entity::Image(i),
        );
    }
    app.selection.set([Slot(1)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    hover(app, [44.0, -14.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("resim-ekle", insert),
        ("resim-ekle-yazildi", inserted),
        ("resim-kirp", clip),
        ("resim-kirpildi", clipped),
    ]
}
