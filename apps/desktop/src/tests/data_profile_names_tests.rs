//! Veri profili's Kurum adları page (docs/adr/5180): the fields' names in the institution by a form of names, a mapping's
//! own names first, the problems said row by row, and the form written into the drawing's mapping, which Bulgular then
//! reads the institution's names by.
//!
//! Test code only.

use crate::app::{App, Message};
use crate::data_profile::Event;
use crate::data_profile_names::{NameForm, Page};
use kentos_commands::{MappingNames, NameCase, NameLetters, NameProblem, NameSource};

const PARK: &str = include_str!("../../../../fixtures/profiles/v1/park-r1.json");
const SIVAS: &str = include_str!("../../../../fixtures/profiles/v1/files/sivas-bel.eslem.json");
const CASES: &str = include_str!("../../../../fixtures/commands/v1/cad.profiles.names.json");

fn park() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(PARK).expect("the project reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("profile.window"));
    app
}

fn file(name: &str, text: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-adlar-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the folder");
    let path = dir.join(name);
    std::fs::write(&path, text).expect("the file");
    path
}

/// The shared cases' profile whose names the forms put on one name and cannot spell.
fn clashing_profile() -> String {
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("the cases read");
    let p = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|c| &c["steps"][0]["input"]["profile"])
        .find(|p| p["name"] == "cakisan")
        .expect("the profile")
        .clone();
    p.to_string()
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::DataProfile(e));
}

const SNAKE_ASCII: MappingNames = MappingNames {
    case: NameCase::UpperSnakeCase,
    letters: NameLetters::Ascii,
};

#[test]
fn the_names_page_spells_the_fields_and_writes_the_form_into_the_mapping() {
    let mut app = park();
    ev(&mut app, Event::Page(Page::Names));
    ev(&mut app, Event::NamesForm(NameForm::Snake));
    ev(&mut app, Event::NamesAscii(true));
    let w = app.data_profile.as_ref().expect("the window");
    let a = w.names.answer.as_ref().expect("the names");
    let kapi = a.rows.iter().find(|r| r.field == "KapiNo").expect("KapiNo");
    assert_eq!((kapi.name.as_str(), kapi.source), ("KAPI_NO", NameSource::Form));
    assert!(a.holds, "{a:?}");
    // Without a mapping the form cannot be written.
    assert!(!app.data_profile_form_writable());
    // The institution's mapping, inside the drawing: its own names come first, the form spells the rest.
    ev(&mut app, Event::Imported(Some(file("sivas.json", SIVAS))));
    let w = app.data_profile.as_ref().expect("the window");
    assert_eq!(w.mapping.as_deref(), Some("sivas-bel"));
    ev(&mut app, Event::Page(Page::Names));
    ev(&mut app, Event::NamesForm(NameForm::Snake));
    ev(&mut app, Event::NamesAscii(true));
    let a = app.data_profile.as_ref().and_then(|w| w.names.answer.as_ref().ok()).expect("the names").clone();
    let bina = a.rows.iter().find(|r| r.field == "BinaKimlik").expect("BinaKimlik");
    assert_eq!((bina.name.as_str(), bina.source), ("YAPI_ID", NameSource::Mapping));
    let mahalle = a.rows.iter().find(|r| r.field == "MahalleKodu").expect("MahalleKodu");
    assert_eq!(mahalle.name, "MAHALLE_KODU");
    ev(&mut app, Event::NamesWrite);
    let w = app.data_profile.as_ref().expect("the window");
    assert_eq!(w.names.form, NameForm::Mapping, "the mapping's own form is shown again");
    let a = w.names.answer.as_ref().expect("the names");
    assert_eq!(a.names, Some(SNAKE_ASCII), "the mapping carries the form");
    let carried = app
        .document
        .as_ref()
        .map(|d| d.model.settings().profile_files.iter().find(|f| f.name == "sivas-bel").map(|f| f.text.clone()))
        .flatten()
        .expect("the drawing's mapping");
    assert!(carried.contains("upperSnakeCase"), "{carried}");
    // Uygula reads the institution's names by it: the layers it adds carry the form's names, and Bulgular finds them.
    ev(&mut app, Event::Page(Page::Findings));
    ev(&mut app, Event::Apply);
    fn names(nodes: &[kentos_commands::LayerNode], out: &mut Vec<String>) {
        for n in nodes {
            out.extend(n.fields.iter().map(|f| f.name.clone()));
            names(&n.children, out);
        }
    }
    let mut fields = Vec::new();
    names(app.document.as_ref().expect("the drawing").model.layers().nodes(), &mut fields);
    assert!(fields.iter().any(|f| f == "MAHALLE_KODU"), "{fields:?}");
    assert!(!fields.iter().any(|f| f == "MahalleKodu"), "{fields:?}");
    let w = app.data_profile.as_ref().expect("the window");
    let r = w.report_for_test().expect("the findings");
    assert!(
        !r.findings.iter().any(|f| f.kind == kentos_commands::ProfileFindingKind::MissingField),
        "{:?}",
        r.findings.iter().map(|f| f.subject.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn a_clash_and_a_name_the_form_cannot_spell_are_said_and_nothing_is_written() {
    let mut app = park();
    ev(&mut app, Event::Imported(Some(file("cakisan.json", &clashing_profile()))));
    ev(&mut app, Event::Page(Page::Names));
    ev(&mut app, Event::NamesForm(NameForm::Snake));
    ev(&mut app, Event::NamesAscii(true));
    let w = app.data_profile.as_ref().expect("the window");
    assert_eq!(w.profile, "cakisan");
    let a = w.names.answer.as_ref().expect("the names");
    assert!(!a.holds);
    assert_eq!((a.unfit, a.clashes), (2, 2));
    let problems: Vec<_> = a.rows.iter().map(|r| (r.field.as_str(), r.problem)).collect();
    assert!(problems.contains(&("Café", Some(NameProblem::Unfit))), "{problems:?}");
    assert!(problems.contains(&("Ada no", Some(NameProblem::Clash))), "{problems:?}");
    assert!(!app.data_profile_form_writable(), "no mapping, and it does not hold");
}

#[test]
#[ignore = "writes the pictures"]
/// `.run/gui-evidence/kurum-adlari-5180/`:
///
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib data_profile_names_tests::screens -- --ignored --nocapture
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kurum-adlari-5180");
    std::fs::create_dir_all(&out).expect("a folder");
    let shoot = |app: &mut App, size: Size, file: &str| {
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(file);
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    };
    let set = |app: &mut App, theme: &str, size: Size| {
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
    };
    let (wide, narrow) = (Size::new(1440.0, 900.0), Size::new(1100.0, 760.0));
    let mut app = park();
    set(&mut app, "light", wide);
    ev(&mut app, Event::Imported(Some(file("sivas.json", SIVAS))));
    ev(&mut app, Event::Page(Page::Names));
    ev(&mut app, Event::NamesForm(NameForm::Snake));
    ev(&mut app, Event::NamesAscii(true));
    shoot(&mut app, wide, "1-buyuk-harf-esleme-acik-genis.png");
    ev(&mut app, Event::NamesWrite);
    ev(&mut app, Event::Page(Page::Findings));
    shoot(&mut app, wide, "2-bulgular-bicimle-acik-genis.png");
    let mut app = park();
    set(&mut app, "dark", narrow);
    ev(&mut app, Event::Imported(Some(file("cakisan.json", &clashing_profile()))));
    ev(&mut app, Event::Page(Page::Names));
    ev(&mut app, Event::NamesForm(NameForm::Camel));
    ev(&mut app, Event::NamesAscii(true));
    ev(&mut app, Event::NamesPick(1));
    shoot(&mut app, narrow, "3-cakisma-koyu-dar.png");
}
