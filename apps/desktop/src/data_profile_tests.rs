//! Veri profili keeping the institution's profile inside the drawing (docs/adr/0735): İçe aktar… with Nereye, the
//! drawing's copy chosen and read, and taken out again only after its question.
//!
//! Test code only.

use crate::app::{App, Message};
use crate::data_profile::{Event, Keep};
use kentos_commands::ProfileFileKind;

const PARK: &str = include_str!("../../../fixtures/profiles/v1/park-r1.json");
const PARK_PROFILE: &str =
    include_str!("../../../fixtures/profiles/v1/files/park-bahce.profil.json");
const PARK_MAPPING: &str =
    include_str!("../../../fixtures/profiles/v1/files/park-bahce-kurum.eslem.json");

/// The park project opened, never saved: no file, so no folder beside it.
fn park() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(PARK).expect("the project reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("profile.window"));
    app
}

/// A file of the institution's to bring in, in a folder of the test's own.
fn file(name: &str, text: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-profil-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the folder");
    let path = dir.join(name);
    std::fs::write(&path, text).expect("the file");
    path
}

fn carried(app: &App) -> Vec<(ProfileFileKind, String)> {
    app.document
        .as_ref()
        .map(|d| {
            d.model
                .settings()
                .profile_files
                .iter()
                .map(|f| (f.kind, f.name.clone()))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_profile_is_kept_inside_a_drawing_without_a_file_and_taken_out_after_its_question() {
    let mut app = park();
    assert!(app.data_profile.is_some(), "the window opens");
    // Nereye starts at the drawing: no file is needed.
    let _ = app.update(Message::DataProfile(Event::Imported(Some(file(
        "park-bahce.json",
        PARK_PROFILE,
    )))));
    let _ = app.update(Message::DataProfile(Event::Imported(Some(file(
        "park-esleme.json",
        PARK_MAPPING,
    )))));
    assert_eq!(
        carried(&app),
        [
            (ProfileFileKind::Profile, "park-bahce".to_owned()),
            (ProfileFileKind::Mapping, "park-bahce-kurum".to_owned()),
        ]
    );
    assert!(app.document.as_ref().is_some_and(|d| d.model.is_dirty()), "the drawing is to be saved");
    // What came in is chosen: the mapping, of the profile it names.
    let w = app.data_profile.as_ref().expect("the window");
    assert_eq!(w.profile_name(), "park-bahce");
    assert_eq!(w.mapping_name(), Some("park-bahce-kurum"));
    // Taking the profile out asks first; no leaves it.
    let _ = app.update(Message::DataProfile(Event::Remove(ProfileFileKind::Profile)));
    assert!(app.data_profile.as_ref().is_some_and(|w| w.asking()), "the question is open");
    let _ = app.update(Message::DataProfile(Event::Removed(false)));
    assert!(app.data_profile.as_ref().is_some_and(|w| !w.asking()));
    assert_eq!(carried(&app).len(), 2, "no took nothing out");
    // Yes takes the profile out, and the drawing's mapping of it with it: no other copy of the profile is left, so the
    // mapping could be chosen no more.
    let _ = app.update(Message::DataProfile(Event::Remove(ProfileFileKind::Profile)));
    let _ = app.update(Message::DataProfile(Event::Removed(true)));
    assert!(carried(&app).is_empty(), "{:?}", carried(&app));
    let w = app.data_profile.as_ref().expect("the window");
    assert_eq!(w.profile_name(), "tucbs-temel", "KentOS's profile is chosen again");
    assert_eq!(w.mapping_name(), None);
}

#[test]
fn a_mapping_alone_is_taken_out_and_its_profile_stays() {
    let mut app = park();
    for (name, text) in [("p.json", PARK_PROFILE), ("m.json", PARK_MAPPING)] {
        let _ = app.update(Message::DataProfile(Event::Imported(Some(file(name, text)))));
    }
    let _ = app.update(Message::DataProfile(Event::Remove(ProfileFileKind::Mapping)));
    let _ = app.update(Message::DataProfile(Event::Removed(true)));
    assert_eq!(
        carried(&app),
        [(ProfileFileKind::Profile, "park-bahce".to_owned())]
    );
    let w = app.data_profile.as_ref().expect("the window");
    assert_eq!(w.profile_name(), "park-bahce", "the profile stays chosen");
    assert_eq!(w.mapping_name(), None);
}

#[test]
fn the_folder_needs_the_drawings_file() {
    let mut app = park();
    let _ = app.update(Message::DataProfile(Event::Keep(Keep::Folder)));
    let _ = app.update(Message::DataProfile(Event::Imported(Some(file(
        "park-bahce-2.json",
        PARK_PROFILE,
    )))));
    assert!(carried(&app).is_empty(), "nothing is kept: the drawing has no folder");
    assert!(
        !app.document.as_ref().is_some_and(|d| d.model.is_dirty()),
        "the drawing is as it was"
    );
}
