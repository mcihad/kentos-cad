//! Eleman tipleri (docs/adr/8106; plan/bim-plan.md BIM-12's first acceptance) worked by hand: the section building's
//! first column made a type (Tipe çevir), its sixteen columns tied to it through Öznitelikler's Tip list, one given its
//! own width there (an exception). The type's width typed in the window, the plan says before Uygula what follows;
//! after it fifteen columns follow and the one with its own width keeps it; Öznitelikler says where each value comes
//! from (Tipten, Örnekten). One undo step gives all back.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib element_types_tests::screens -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use crate::arch::ArchState;

use kentos_commands::{ArchChange, ArchOperation, Entity, type_link};
use kentos_domain::Slot;
use kentos_launch::Module;

use crate::app::{App, Message};
use crate::element_types_window::Event;
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

fn send(app: &mut App, e: Event) {
    let task = app.update(Message::from(crate::arch::Event::ElementTypes(e)));
    drive(app, task);
}

fn opened() -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let snapshot =
        kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/section-building.kcad")).expect("decodes");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn columns(app: &App) -> Vec<Slot> {
    let doc = app.document.as_ref().expect("open");
    doc.model.entities().filter(|e| matches!(e, Entity::Column(_))).map(|e| Slot(e.base().id)).collect()
}

fn width(app: &App, slot: Slot) -> f64 {
    match app.document.as_ref().and_then(|d| d.model.get(slot)) {
        Some(Entity::Column(c)) => c.column.width,
        other => panic!("{other:?}"),
    }
}

/// The value, the source and the section of the row `label` of the selection's Öznitelikler.
fn row(app: &App, label: &str) -> (String, Option<&'static str>) {
    let doc = app.document.as_ref().expect("open");
    let panel = app.properties_panel(doc);
    let r = panel
        .sections
        .into_iter()
        .flat_map(|s| s.rows)
        .find(|r| !r.note && r.label == label)
        .unwrap_or_else(|| panic!("{label}"));
    (r.value, r.source)
}

/// The section building's columns: the first made S1, all tied to it, the last with its own width.
fn typed() -> (App, Vec<Slot>, String) {
    let mut app = opened();
    let cols = columns(&app);
    assert_eq!(cols.len(), 16, "the section building's columns");
    app.selection.set([cols[0]]);
    let _ = app.update(Message::Run("arch.elementTypeFrom"));
    let id = app.modules.arch().element_types.as_ref().and_then(|w| w.chosen.clone()).expect("the new type chosen");
    app.selection.set(cols.clone());
    send(&mut app, Event::Assign(id.clone()));
    let last = cols[15];
    let own = width(&app, last) + 0.15;
    app.selection.set([last]);
    let op = ArchOperation::Set(Box::new(ArchChange { width: Some(own), ..Default::default() }));
    let task = app.update(Message::Properties(crate::properties::Event::Arch(last, op)));
    drive(&mut app, task);
    (app, cols, id)
}

#[test]
fn the_type_s_width_changed_fifteen_columns_follow_and_the_one_with_its_own_keeps_it() {
    let (mut app, cols, id) = typed();
    let last = cols[15];
    let own = width(&app, last);
    // Öznitelikler: the last column's width its own, its depth its type's.
    assert_eq!(row(&app, "Tip").0, "Kolon 1");
    assert_eq!(row(&app, "En").1, Some("Örnekten"));
    assert_eq!(row(&app, "Boy").1, Some("Tipten"));
    assert_eq!(row(&app, "İstisnalar").0, "En");
    app.selection.set([cols[3]]);
    assert_eq!(row(&app, "En").1, Some("Tipten"));
    assert_eq!(row(&app, "İstisnalar").0, "Yok");
    // The window: the type's width typed, the plan before Uygula.
    let _ = app.update(Message::Run("arch.elementTypes"));
    send(&mut app, Event::Choose(id.clone()));
    let was = width(&app, cols[0]);
    let next = was + 0.05;
    let f = kentos_interaction::Format::of(app.document.as_ref().expect("open").model.settings());
    send(&mut app, Event::Value("width", f.length_bare(next)));
    let preview = app.modules.arch().element_types.as_ref().and_then(|w| w.preview.clone()).expect("a plan");
    let plan = preview.expect("it holds");
    assert_eq!(plan.report.applied.len(), 16);
    assert_eq!(plan.report.kept.len(), 1);
    assert_eq!(
        crate::element_types_window::report_words(&plan.report),
        "15 eleman yeni değerleri alır; 1 eleman değişen alanda kendi değerini korur"
    );
    assert!((width(&app, cols[0]) - was).abs() < 1e-12, "the plan writes nothing");
    send(&mut app, Event::Apply);
    let followed = cols.iter().filter(|&&c| (width(&app, c) - next).abs() < 1e-9).count();
    assert_eq!(followed, 15, "fifteen follow");
    assert!((width(&app, last) - own).abs() < 1e-12, "the one with its own width keeps it");
    let doc = app.document.as_ref().expect("open");
    assert!(cols.iter().all(|&c| doc.model.get(c).and_then(type_link).is_some_and(|l| l.version == 2)));
    // One undo step: the type and its columns back.
    let _ = app.update(Message::Run("edit.undo"));
    assert!((width(&app, cols[0]) - was).abs() < 1e-12);
    assert_eq!(app.document.as_ref().expect("open").model.element_types()[0].version, 1);
}

#[test]
fn tipten_ayir_keeps_the_values_and_tipe_dondur_gives_the_type_s_back() {
    let (mut app, cols, _) = typed();
    let last = cols[15];
    let typed_width = width(&app, cols[0]);
    // Tipe döndür: its own width given up.
    app.selection.set([last]);
    send(&mut app, Event::Reset);
    assert!((width(&app, last) - typed_width).abs() < 1e-12);
    assert_eq!(row(&app, "İstisnalar").0, "Yok");
    // Tipten ayır: its values stay, it follows no type.
    send(&mut app, Event::Detach);
    assert!((width(&app, last) - typed_width).abs() < 1e-12);
    assert_eq!(row(&app, "Tip").0, "Tipsiz");
    assert_eq!(row(&app, "En").1, None, "no source for an element of its own");
    // Aynı tiptekileri seç: the fifteen still following.
    app.selection.set([cols[0]]);
    let _ = app.update(Message::Run("arch.elementSameType"));
    assert_eq!(app.selection.ids().len(), 15);
}

/// The window's and Öznitelikler's pictures (`.run/gui-evidence/eleman-tipleri-8106/`), light and dark.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/eleman-tipleri-8106");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let size = Size::new(1440.0, 900.0);
        let (mut app, cols, id) = typed();
        // Tipe çevir opened the window; Öznitelikler first.
        send(&mut app, Event::Close);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        fn update(app: &mut App, message: Message) {
            let _ = app.update(message);
        }
        let shot = |app: &mut App, snapshot: &mut Snapshot, file: &str| {
            app.toasts.clear();
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        // Öznitelikler, in a tall window so its type rows are in sight: a column following its type, then the one
        // with its own width.
        let tall = Size::new(1440.0, 1700.0);
        let _ = app.update(Message::WindowResized(tall));
        let mut long = Snapshot::new(tall).expect("a renderer");
        app.selection.set([cols[3]]);
        let _ = app.update(Message::Properties(crate::properties::Event::Toggle("general")));
        shot(&mut app, &mut long, "1-oznitelikler-tipten");
        app.selection.set([cols[15]]);
        shot(&mut app, &mut long, "2-oznitelikler-ornekten");
        // Its Tip section in sight: the look and the values' origins folded.
        for id in ["geometry", "modelAppearance", "valueOrigin"] {
            let _ = app.update(Message::Properties(crate::properties::Event::Toggle(id)));
        }
        shot(&mut app, &mut long, "2b-oznitelikler-tip-bolumu");
        for id in ["geometry", "modelAppearance", "valueOrigin"] {
            let _ = app.update(Message::Properties(crate::properties::Event::Toggle(id)));
        }
        let _ = app.update(Message::WindowResized(size));
        // The window: the type's width typed, the plan said before Uygula.
        let _ = app.update(Message::Run("arch.elementTypes"));
        send(&mut app, Event::Choose(id.clone()));
        let f = kentos_interaction::Format::of(app.document.as_ref().expect("open").model.settings());
        let next = width(&app, cols[0]) + 0.05;
        send(&mut app, Event::Value("width", f.length_bare(next)));
        shot(&mut app, &mut snapshot, "3-pencere-onizleme");
        send(&mut app, Event::Apply);
        shot(&mut app, &mut snapshot, "4-pencere-uygulandi");
        send(&mut app, Event::Close);
        // The contextual tab's Tip panel with a column selected.
        app.selection.set([cols[0]]);
        shot(&mut app, &mut snapshot, "5-baglamsal-tip");
        // Its place in Mimari › Proje ▾.
        app.selection.set(Vec::<Slot>::new());
        let _ = app.update(Message::RibbonSearch("Eleman tipleri".into()));
        let _ = app.update(Message::RibbonSearchReveal(0));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(&mut app, App::view, &mut update, Input::Click(iced::Point::new(1278.0, 133.0)));
        shot(&mut app, &mut snapshot, "6-serit-yeri");
    }
}
