//! The designer's way to a release and its preview (the CBS review's 6th item, docs/adr/9325): Alanları yerleştir, Örnek
//! kayıtla dene, Taslağı kaydet and Formu yayınla said done one by one; the preview says which record it shows and,
//! through Okuyan's eyes, changes nothing.

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive};
use crate::forms::designer::{Event, PreviewRole};
use crate::gis;

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Forms(e)));
    drive(app, task);
}

/// A layer of the test's own and the designer on a new form of it, its field `durum` placed.
fn opened() -> App {
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("open");
    let done = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.layers.edit",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "operation": "add", "name": "Saha" }),
    )
    .expect("runs");
    let layer = done["output"]["layer"].as_str().expect("the layer").to_owned();
    let task = app.update(Message::Run("data.forms"));
    drive(&mut app, task);
    ev(&mut app, Event::NewForm);
    ev(&mut app, Event::NewLayer(layer));
    ev(&mut app, Event::NewName("Saha formu".into()));
    ev(&mut app, Event::Create);
    ev(&mut app, Event::NewFieldName("durum".into()));
    ev(&mut app, Event::CreateField);
    app
}

#[test]
fn the_steps_are_said_done_as_the_form_goes_to_its_release() {
    let mut app = opened();
    let w = app.gis.forms_window.as_ref().expect("the designer");
    assert!(w.dirty && !w.tried, "placed, not tried, not saved");
    ev(&mut app, Event::Preview(true));
    assert!(app.gis.forms_window.as_ref().is_some_and(|w| w.tried), "Örnek kayıtla dene done");
    ev(&mut app, Event::Preview(false));
    ev(&mut app, Event::Save);
    assert!(app.gis.forms_window.as_ref().is_some_and(|w| !w.dirty), "Taslağı kaydet done");
    let form = app.project_forms().first().cloned().expect("the form");
    assert!(form.releases.is_empty(), "not published yet");
    ev(&mut app, Event::Publish);
    let form = app.project_forms().first().cloned().expect("the form");
    let release = form.releases.last().expect("Formu yayınla done");
    assert!(crate::forms::designer::same_form(&release.body, &form.draft));
}

#[test]
fn okuyan_sees_the_preview_read_only() {
    let mut app = opened();
    ev(&mut app, Event::Preview(true));
    assert!(app.gis.form_preview.as_ref().is_some_and(|e| !e.read_only), "Düzenleyen edits");
    ev(&mut app, Event::PreviewRole(PreviewRole::Viewer));
    let e = app.gis.form_preview.as_ref().expect("the preview");
    assert!(e.read_only);
    assert_eq!(e.read_why.as_deref(), Some("Okuyan rolü kaydı okur; formda hiçbir şeyi değiştiremez."));
    ev(&mut app, Event::PreviewRole(PreviewRole::Entry));
    assert!(app.gis.form_preview.as_ref().is_some_and(|e| !e.read_only), "Veri girişi edits the attributes");
}

/// The pictures (`.run/gui-evidence/kullanim-9325/`): the steps over a form placed and not saved, with Taslağı kaydet
/// and Formu yayınla; the preview through Okuyan's eyes.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib design_steps_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9325", "01-adimlar", opened);
    crate::files_testing::usage_shots("9325", "02-onizleme-okuyan", || {
        let mut app = opened();
        ev(&mut app, Event::Preview(true));
        ev(&mut app, Event::PreviewRole(PreviewRole::Viewer));
        app
    });
}
