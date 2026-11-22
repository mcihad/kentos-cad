//! One name for a project (docs/adr/9138), the municipal report's case: a new project accepted as “Yeni proje” and saved
//! as Ankara-Mahalle-Hizmetleri.kcad is called so in the file; reopened, the inspector's Proje adı and the opening's
//! words say the same name as the title. A file of an older KentOS whose project kept the new project's name is shown
//! by its file's name, its own name left as it is.

use crate::app::{App, Message};
use crate::files_testing::{drive, last_said, scratch};

/// A new project as the wizard makes it, its name the new project's.
fn new_project() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
        name: kentos_project::new_project::NEW_PROJECT_NAME.into(),
        srid: 5256,
        plot_scale: 1000.0,
        workspace: kentos_commands::Workspace::Gis,
        drawing_font: kentos_commands::DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
        annotation: None,
        cad_work: None,
        heights: None,
    })
    .expect("a project");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn inspector_name(app: &App) -> String {
    let doc = app.document.as_ref().expect("open");
    doc.shown_name().into_owned()
}

#[test]
fn a_new_project_saved_under_a_file_name_is_called_by_it_everywhere() {
    let dir = scratch("proje-adi");
    let path = dir.join("Ankara-Mahalle-Hizmetleri.kcad");
    let mut app = new_project();
    assert_eq!(app.document.as_ref().expect("open").name(), "Yeni proje");
    let task = app.start_saving(path.clone());
    drive(&mut app, task);
    let doc = app.document.as_ref().expect("open");
    assert_eq!(doc.name(), "Ankara-Mahalle-Hizmetleri", "named after its file");
    assert!(!doc.dirty(), "the name went in with the save");
    // Reopened: the opening's words, the inspector and the title say one name.
    let task = app.start_opening(path.clone(), crate::opening::Purpose::File);
    drive(&mut app, task);
    assert!(last_said(&app).starts_with("Ankara-Mahalle-Hizmetleri açıldı:"), "{}", last_said(&app));
    assert_eq!(inspector_name(&app), "Ankara-Mahalle-Hizmetleri");
    assert!(app.title().contains("Ankara-Mahalle-Hizmetleri"), "{}", app.title());
    // A name given by the person is theirs: a later save under another file keeps it.
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_name("Mahalle hizmetleri 2026");
    }
    let task = app.start_saving(dir.join("kopya.kcad"));
    drive(&mut app, task);
    assert_eq!(app.document.as_ref().expect("open").name(), "Mahalle hizmetleri 2026");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_older_file_that_kept_the_new_projects_name_is_shown_by_its_file() {
    let dir = scratch("eski-proje-adi");
    let path = dir.join("Kadastro-2025.kcad");
    // Written as an older KentOS wrote it: the project kept “Yeni proje”.
    let app = new_project();
    let snapshot = app.document.as_ref().expect("open").model.to_snapshot_v2();
    std::fs::write(&path, kentos_kcad::encode_verified(&snapshot).expect("written")).expect("on disk");
    let mut app = new_project();
    let task = app.start_opening(path.clone(), crate::opening::Purpose::File);
    drive(&mut app, task);
    let doc = app.document.as_ref().expect("open");
    assert_eq!(doc.name(), "Yeni proje", "its own name is left as it is");
    assert_eq!(inspector_name(&app), "Kadastro-2025");
    assert!(last_said(&app).starts_with("Kadastro-2025 açıldı:"), "{}", last_said(&app));
    let _ = std::fs::remove_dir_all(&dir);
}
