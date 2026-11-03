//! Enkesit (section_window.rs, plan/road.md Y5): the window builds sections as it is typed and writes through
//! `road.templates.edit`, `road.sections.edit` and `road.sections.draw`, each write one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::section_window::{Chosen, Event as W};

/// A road project with `road.sections.query`'s drawing: 120 points with heights, a surface, the main and the side
/// alignment, their profiles, two templates and two sample line sets; or without the sets and templates.
fn app(sets: bool) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.sections.query.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    if !sets {
        let road = setup["settings"]["road"].as_object_mut().expect("road");
        road.remove("sections");
        road.remove("templates");
    }
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Sections(e));
}

fn model(app: &mut App) -> &mut kentos_domain::Document {
    &mut app.document.as_mut().expect("a drawing").model
}

/// Tip kesit oluştur: two lanes and a shoulder, a ditch in the cut; then a set on the main alignment with it.
fn made(app: &mut App) {
    let _ = app.update(Message::Run("section.template"));
    assert_eq!(app.dialog, Some(Dialog::Sections));
    send(app, W::Name("İki şerit".into()));
    send(app, W::Part(0, 0, "3,5".into()));
    send(app, W::Part(0, 1, "-2,5".into()));
    send(app, W::Add);
    send(app, W::Part(1, 0, "1".into()));
    send(app, W::Part(1, 1, "-4".into()));
    send(app, W::End(0, "1".into()));
    send(app, W::End(1, "1,5".into()));
    send(app, W::Ditch);
    send(app, W::End(2, "2".into()));
    send(app, W::End(3, "0,5".into()));
    send(app, W::NewSet);
    send(app, W::Template(Some("tk-1".into())));
}

#[test]
fn the_window_makes_a_template_and_a_set_and_shows_their_sections() {
    let mut app = app(false);
    made(&mut app);
    let w = app.section_window.as_ref().expect("open");
    assert_eq!(w.chosen, Some(Chosen::Set(0)));
    assert!(w.preview.why.is_none(), "builds: {:?}", w.preview.why);
    assert!(w.preview.rows.len() > 20, "every 20 m and the curve's points");
    assert!(w.preview.rows.iter().any(|r| r.left.is_some()), "the platform is built on the profile");
    send(&mut app, W::Next);
    assert_eq!(app.section_window.as_ref().expect("open").at, 1);
    send(&mut app, W::Save);
    assert!(app.section_window.is_none(), "Kaydet closes the window");
    let doc = model(&mut app);
    assert_eq!(doc.road_templates().len(), 1);
    assert_eq!(doc.road_templates()[0].left.components.len(), 2);
    assert!(doc.road_templates()[0].right.is_none(), "the right side mirrors the left");
    let sets = doc.road_sections().to_vec();
    assert_eq!(sets.len(), 1);
    assert_eq!((sets[0].id.as_str(), sets[0].template.as_deref(), sets[0].profile.as_deref()), ("k-1", Some("tk-1"), Some("b-1")));
    assert_eq!(doc.undo().as_deref(), Some("Kesit takımı"));
    assert_eq!(doc.undo().as_deref(), Some("Tip kesit"));
}

#[test]
fn a_saved_set_is_drawn_and_a_used_template_is_kept() {
    let mut app = app(true);
    let before = model(&mut app).entities().count();
    let _ = app.update(Message::Run("section.editor"));
    send(&mut app, W::Choose(Chosen::Set(0)));
    send(&mut app, W::DrawLayer("0".into()));
    send(&mut app, W::Place("0; -300".into()));
    send(&mut app, W::Draw);
    let doc = model(&mut app);
    assert!(doc.entities().count() > before + 100, "eight views with their grids, lines and words");
    assert_eq!(doc.undo().as_deref(), Some("Enkesit çiz"));
    // A template a set uses stays.
    send(&mut app, W::Choose(Chosen::Template(0)));
    send(&mut app, W::Remove);
    assert_eq!(app.section_window.as_ref().expect("open").templates.len(), 2);
    // The stale side set is rebound, one undo step.
    send(&mut app, W::Choose(Chosen::Set(1)));
    send(&mut app, W::Rebind);
    let doc = model(&mut app);
    assert_eq!(doc.road_sections()[1].alignment_revision, 3);
    assert_eq!(doc.undo().as_deref(), Some("Kesit takımı"));
}

#[test]
fn a_template_with_a_side_left_blank_is_refused_and_the_window_stays() {
    let mut app = app(false);
    let _ = app.update(Message::Run("section.template"));
    send(&mut app, W::Save);
    assert!(app.section_window.is_some(), "a blank width leaves the window open");
    assert!(model(&mut app).road_templates().is_empty());
}

#[test]
fn a_saved_set_is_laid_on_sheets_as_one_step() {
    let mut app = app(true);
    let before = model(&mut app).entities().count();
    let _ = app.update(Message::Run("section.editor"));
    send(&mut app, W::Choose(Chosen::Set(0)));
    send(&mut app, W::DrawLayer("0".into()));
    send(&mut app, W::Place("0; -500".into()));
    send(&mut app, W::SheetScale("500".into()));
    send(&mut app, W::Sheets);
    assert!(app.section_window.is_none(), "the window closes on the sheets");
    let doc = model(&mut app);
    let sets = doc.road_sheet_sets().to_vec();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].sections.as_deref(), Some("k-1"));
    assert_eq!(app.sheets.book().sheets.len(), sets[0].sheets.len());
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Model));
    let doc = model(&mut app);
    assert_eq!(doc.undo().as_deref(), Some("Enkesit paftaları"), "one undo step");
    assert!(doc.road_sheet_sets().is_empty());
    assert_eq!(doc.entities().count(), before);
}

/// Paftalara diz's sheets; `.run/gui-evidence/yol-y10-enkesit-pafta-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib section_window_tests::sheet_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn sheet_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = app(true);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y10-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("section.editor"));
        send(&mut app, W::Choose(Chosen::Set(0)));
        send(&mut app, W::DrawLayer("0".into()));
        send(&mut app, W::Place("0; -500".into()));
        send(&mut app, W::Exaggeration("2".into()));
        send(&mut app, W::SheetScale("500".into()));
        shot(&mut app, "enkesit-pafta-pencere");
        send(&mut app, W::Sheets);
        shot(&mut app, "enkesit-pafta");
    }
}

/// The window over the drawing, wide and narrow, light and dark; `.run/gui-evidence/yol-y5-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib section_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app(true);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y5-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("section.editor"));
        send(&mut app, W::Choose(Chosen::Set(0)));
        send(&mut app, W::Next);
        send(&mut app, W::Next);
        shot(&mut app, "enkesit");
        // The template on its sample ground.
        send(&mut app, W::Choose(Chosen::Template(0)));
        shot(&mut app, "tip-kesit");
        // The stale side set with its asymmetric template and an open side.
        send(&mut app, W::Choose(Chosen::Set(1)));
        shot(&mut app, "enkesit-eski");
        if name == "acik-genis" {
            // Çizime yaz: the sheet under the alignments, the window closed over it.
            send(&mut app, W::Choose(Chosen::Set(0)));
            send(&mut app, W::DrawLayer("0".into()));
            send(&mut app, W::Place("0; -100".into()));
            send(&mut app, W::Exaggeration("5".into()));
            send(&mut app, W::Columns("4".into()));
            send(&mut app, W::Draw);
            send(&mut app, W::Cancel);
            let _ = app.update(Message::Run("view.zoomExtents"));
            shot(&mut app, "enkesit-cizimde");
            // Closer: the first view, its grids, numbers and areas.
            app.zoom_to_bounds(&kentos_commands::Bounds { min_x: -5.0, min_y: -160.0, max_x: 80.0, max_y: -95.0 });
            shot(&mut app, "enkesit-gorunus");
        }
    }
}
