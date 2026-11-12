//! Enkesit (section_window.rs, plan/road.md Y5): the window builds sections as it is typed and writes through
//! `road.templates.edit`, `road.sections.edit` and `road.sections.draw`, each write one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::road;
use crate::section_window::{Chosen, Event as W};

/// A road project with `road.sections.query`'s drawing: 120 points with heights, a surface, the main and the side
/// alignment, their profiles, two templates and two sample line sets; or without the sets and templates.
fn app(sets: bool) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.sections.query.json")).expect("cases");
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
    let _ = app.update(Message::Road(road::Event::Sections(e)));
}

fn model(app: &mut App) -> &mut kentos_domain::Document {
    &mut app.document.as_mut().expect("a drawing").model
}

/// Tip kesit oluştur: two lanes and a shoulder, a ditch in the cut; then a set on the main alignment with it.
fn made(app: &mut App) {
    let _ = app.update(Message::Run("section.template"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Sections))
    );
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
    let w = app.road.section_window.as_ref().expect("open");
    assert_eq!(w.chosen, Some(Chosen::Set(0)));
    assert!(w.preview.why.is_none(), "builds: {:?}", w.preview.why);
    assert!(
        w.preview.rows.len() > 20,
        "every 20 m and the curve's points"
    );
    assert!(
        w.preview.rows.iter().any(|r| r.left.is_some()),
        "the platform is built on the profile"
    );
    send(&mut app, W::Next);
    assert_eq!(app.road.section_window.as_ref().expect("open").at, 1);
    send(&mut app, W::Save);
    assert!(
        app.road.section_window.is_none(),
        "Kaydet closes the window"
    );
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
    assert_eq!(
        app.road
            .section_window
            .as_ref()
            .expect("open")
            .templates
            .len(),
        2
    );
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
    assert!(
        app.road.section_window.is_some(),
        "a blank width leaves the window open"
    );
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
    assert!(
        app.road.section_window.is_none(),
        "the window closes on the sheets"
    );
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

/// Üstyapı (docs/adr/0975 §3): four layers typed on a template, its edge on the fill slope; Kaydet writes them with
/// `road.templates.edit` and the set's sections carry the pavement.
fn paved(app: &mut App) {
    let _ = app.update(Message::Run("section.editor"));
    send(app, W::Choose(Chosen::Template(0)));
    send(app, W::Pavement);
    for (j, (name, t, m)) in [("Aşınma", "0,05", "KGM-a"), ("Binder", "0,08", ""), ("Plentmiks temel", "0,20", ""), ("Alttemel", "0,25", "")]
        .into_iter()
        .enumerate()
    {
        if j > 0 {
            send(app, W::AddLayer);
        }
        send(app, W::Layer(j, 0, name.into()));
        send(app, W::Layer(j, 1, t.into()));
        send(app, W::Layer(j, 2, m.into()));
    }
    send(app, W::PavementSloped(true));
    send(app, W::PavementSlope("1,5".into()));
}

#[test]
fn the_template_takes_a_pavement_and_the_sections_show_it() {
    let mut app = app(true);
    paved(&mut app);
    send(&mut app, W::Save);
    let t = model(&mut app).road_templates()[0].clone();
    let p = t.pavement.expect("written");
    assert_eq!(p.layers.len(), 4);
    assert_eq!(p.layers[0].material.as_deref(), Some("KGM-a"));
    assert_eq!(p.edge, kentos_commands::PavementEdge::Slope { h: 1.5 });
    let made = kentos_native_application::road_sections::made_sections(model(&mut app));
    let rows = made[0].1.as_ref().expect("sections");
    let r = rows.iter().find(|r| r.pavement.is_some()).expect("a section with its pavement");
    let pv = r.pavement.as_ref().expect("pavement");
    assert_eq!(pv.layers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Aşınma", "Binder", "Plentmiks temel", "Alttemel"]);
    assert!(pv.formation_cut >= r.cut.unwrap_or(0.0) && pv.formation_fill <= r.fill.unwrap_or(0.0));
    // Taken off again: removePavement.
    let _ = app.update(Message::Run("section.editor"));
    send(&mut app, W::Choose(Chosen::Template(0)));
    send(&mut app, W::Pavement);
    send(&mut app, W::Save);
    assert!(model(&mut app).road_templates()[0].pavement.is_none());
}

/// Üstyapı in the template editor and on the sections, wide and narrow, light and dark; `.run/gui-evidence/yol-y26-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib section_window_tests::screens_pavement -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_pavement() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y26");
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
            let file = out.join(format!("y26-3-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        paved(&mut app);
        shot(&mut app, "tip-kesit-ustyapi");
        send(&mut app, W::Save);
        let _ = app.update(Message::Run("section.editor"));
        send(&mut app, W::Choose(Chosen::Set(0)));
        send(&mut app, W::Next);
        send(&mut app, W::Next);
        shot(&mut app, "enkesit-ustyapi");
        send(&mut app, W::Cancel);
    }
}

/// Y29.3 (docs/adr/0933 §3): a set at the terrain's breaks, then the main set's left arms widened past the curve's
/// radius: the window says the lines meet, Kesişen kolları kırp ends them where they meet, Kaydet writes the clip.
#[test]
fn breaks_and_crossing_arms_are_typed_said_and_clipped() {
    let mut app = app(true);
    let _ = app.update(Message::Run("section.editor"));
    send(&mut app, W::Choose(Chosen::Set(0)));
    send(&mut app, W::Method(3));
    send(&mut app, W::SetField(2, "0".into()));
    send(&mut app, W::SetField(3, "600".into()));
    send(&mut app, W::SetField(7, "0,1".into()));
    send(&mut app, W::SetField(8, "20".into()));
    let w = app.road.section_window.as_ref().expect("open");
    assert!(w.preview.why.is_none(), "{:?}", w.preview.why);
    // The case drawing's 0–600 m breaks within 0,1 m, at most 20 m apart (32, the query's case), and the set keeps
    // its Kurp noktaları ve PVI'ler: 7 more.
    assert_eq!(w.preview.rows.len(), 39);
    // Back to a range over the curve, the left arms 180 m: the lines meet on the inside (R 150 m).
    send(&mut app, W::Method(0));
    send(&mut app, W::SetField(2, "200".into()));
    send(&mut app, W::SetField(3, "420".into()));
    send(&mut app, W::SetField(4, "20".into()));
    send(&mut app, W::SetField(0, "180".into()));
    let w = app.road.section_window.as_ref().unwrap();
    let said = w.preview.crossing.clone().expect("said");
    // Every 20 m and, with Kurp noktaları ve PVI'ler, the curve's points: 37 pairs (23 without them, the query's case).
    assert!(said.contains("37 çift"), "{said}");
    send(&mut app, W::Clip);
    let w = app.road.section_window.as_ref().unwrap();
    assert!(w.preview.crossing.is_none());
    // The right arms stay 25 m; on the circle the left ones end at the centre, R 150 m out.
    let lengths: Vec<f64> = w
        .preview
        .rows
        .iter()
        .map(|r| (r.ends[0][0] - r.ends[1][0]).hypot(r.ends[0][1] - r.ends[1][1]))
        .collect();
    assert!(lengths.iter().all(|l| *l <= 205.0 + 1e-6), "{lengths:?}");
    assert!(lengths.iter().any(|l| (l - 175.0).abs() < 1e-6), "150 + 25 on the circle: {lengths:?}");
    send(&mut app, W::Save);
    let doc = model(&mut app);
    assert!(doc.road_sections()[0].clip);
}

/// The Y29.3 pictures, `.run/gui-evidence/yol-y26/y29-3-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib section_window_tests::screens_breaks -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_breaks() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y26");
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
            let file = out.join(format!("y29-3-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("section.editor"));
        send(&mut app, W::Choose(Chosen::Set(0)));
        send(&mut app, W::Method(3));
        send(&mut app, W::SetField(2, "0".into()));
        send(&mut app, W::SetField(3, "600".into()));
        send(&mut app, W::SetField(7, "0,1".into()));
        send(&mut app, W::SetField(8, "20".into()));
        shot(&mut app, "kiriklar");
        send(&mut app, W::Method(0));
        send(&mut app, W::SetField(2, "200".into()));
        send(&mut app, W::SetField(3, "420".into()));
        send(&mut app, W::SetField(4, "20".into()));
        send(&mut app, W::SetField(0, "180".into()));
        for _ in 0..4 {
            send(&mut app, W::Next);
        }
        shot(&mut app, "kesisen-kollar");
        send(&mut app, W::Clip);
        shot(&mut app, "kirpilan-kollar");
        send(&mut app, W::Cancel);
    }
}

/// Basamaklı şev (Y52, Y53; docs/adr/0958): cut and fill benches typed on the left of a template (the right its
/// mirror); Kaydet writes them with `road.templates.edit`, and on the window's sample ground the cut slope on the left
/// and the fill slope on the right walk over benches before they meet it.
fn benched(app: &mut App) {
    made(app);
    send(app, W::Choose(Chosen::Template(0)));
    send(app, W::Bench);
    send(app, W::End(5, "3".into()));
    send(app, W::End(6, "2".into()));
    send(app, W::End(7, "4".into()));
    send(app, W::FillBench);
    send(app, W::End(8, "3".into()));
    send(app, W::End(9, "2".into()));
}

#[test]
fn the_template_takes_a_cut_bench_and_its_slope_walks_over_it() {
    let mut app = app(false);
    benched(&mut app);
    send(&mut app, W::Save);
    let t = model(&mut app).road_templates()[0].clone();
    let b = t.left.bench.expect("written");
    assert_eq!((b.height, b.width, b.fall), (3.0, 2.0, 0.04), "the window's 4 % is the grade 0.04");
    let f = t.left.fill_bench.expect("written");
    assert_eq!((f.height, f.width, f.fall), (3.0, 2.0, 0.0), "no fall typed: level");
    assert!(t.right.is_none(), "the right is the left's mirror");
    let core = kentos_native_application::road_sections::core_template(&t);
    let s = kentos_road::section::template::build(&core, 100.0, &crate::section_window::sample_ground_benched()).expect("builds");
    assert_eq!(s.left.kind, kentos_road::section::template::Kind::Cut);
    // Past the ditch, faces of 3 m rise at 1:1 and benches 2 m wide fall 4 % outward: each bench is a level pair.
    let flats = |p: &[[f64; 2]]| p.windows(2).filter(|p| (p[1][0] - p[0][0]).abs() > 1.99 && (p[1][1] - p[0][1]).abs() < 0.1).count();
    assert!(flats(&s.left.points) >= 2, "at least two benches on the 10 m cut: {:?}", s.left.points);
    // On the right, faces of 3 m fall at 1:1.5 and level benches 2 m wide: the fill is benched too.
    assert_eq!(s.right.kind, kentos_road::section::template::Kind::Fill);
    assert!(flats(&s.right.points) >= 2, "at least two benches on the deep fill: {:?}", s.right.points);
    // Taken off again.
    let _ = app.update(Message::Run("section.editor"));
    send(&mut app, W::Choose(Chosen::Template(0)));
    send(&mut app, W::Bench);
    send(&mut app, W::FillBench);
    send(&mut app, W::Save);
    let left = &model(&mut app).road_templates()[0].left;
    assert!(left.bench.is_none() && left.fill_bench.is_none());
}

/// The benched template on its sample cut and fill, wide and narrow, light and dark; `.run/gui-evidence/yol-y53/*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib section_window_tests::screens_benches -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_benches() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y53");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name, full) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis", false),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar", false),
        ("light", Size::new(1920.0, 1200.0), "tam-ekran", true),
    ] {
        use kentos_ui::widget::sizable::sizes;
        if full {
            sizes::set(crate::section_window::TITLE, sizes::Kept { width: 800.0, height: 600.0, maximized: true });
        } else {
            sizes::forget(crate::section_window::TITLE);
        }
        let mut app = app(false);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y53-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        benched(&mut app);
        shot(&mut app, "tip-kesit-basamak");
        send(&mut app, W::Save);
        let _ = app.update(Message::Run("section.editor"));
        send(&mut app, W::Choose(Chosen::Set(0)));
        send(&mut app, W::Next);
        send(&mut app, W::Next);
        shot(&mut app, "enkesit-basamak");
        send(&mut app, W::Cancel);
    }
    kentos_ui::widget::sizable::sizes::forget(crate::section_window::TITLE);
}

/// Yer left empty (docs/adr/0997): the sections, drawn and laid on sheets, go under everything drawn; nothing written
/// reaches into the drawing's box (the km over the first row's frames included).
#[test]
fn an_empty_place_puts_the_sections_under_the_drawing() {
    for sheets in [false, true] {
        let mut app = app(true);
        let _ = app.update(Message::Run("section.editor"));
        assert!(
            app.road
                .section_window
                .as_ref()
                .is_some_and(|w| w.place.is_empty()),
            "Yer starts empty"
        );
        send(&mut app, W::Choose(Chosen::Set(0)));
        send(&mut app, W::DrawLayer("0".into()));
        let before_ids: std::collections::HashSet<u32> =
            model(&mut app).entities().map(|e| e.base().id).collect();
        let doc = &app.document.as_ref().expect("doc").model;
        app.spatial.sync(doc);
        let before = app.spatial.extent().expect("a drawing");
        if sheets {
            send(&mut app, W::SheetScale("500".into()));
            send(&mut app, W::Sheets);
        } else {
            send(&mut app, W::Draw);
        }
        let doc = &app.document.as_ref().expect("doc").model;
        let new: Vec<f64> = doc.entities().filter(|e| !before_ids.contains(&e.base().id)).map(|e| f64::from(e.base().id)).collect();
        assert!(!new.is_empty(), "the sections are drawn (sheets: {sheets})");
        app.spatial.sync(doc);
        let drawn = app.spatial.store().extent(Some(&new)).expect("their box");
        assert!(drawn.max_y < before.min_y, "sheets {sheets}: the sections ({}) stay under the drawing ({})", drawn.max_y, before.min_y);
    }
}

/// Y91's picture (docs/adr/0997): the sections drawn with Yer left empty, the model after; `.run/gui-evidence/yol-y91/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib section_window_tests::screens_place_under -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_place_under() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y91");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut app = app(true);
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    let _ = app.update(Message::Run("section.editor"));
    send(&mut app, W::Choose(Chosen::Set(0)));
    send(&mut app, W::DrawLayer("0".into()));
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("y91-pencere.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
    println!("{}", file.display());
    send(&mut app, W::Draw);
    send(&mut app, W::Cancel);
    let _ = app.update(Message::Run("view.zoomExtents"));
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("y91-model.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
    println!("{}", file.display());
}

/// ROAD-SEC-08 (docs/adr/1034): a set whose design lies over ground the surface does not give says so in Enkesit, as
/// the query's warning; its sections give the swath's stretches off the surface.
#[test]
fn the_window_says_where_the_design_has_no_ground() {
    let mut app = app(true);
    let _ = app.update(Message::Run("section.editor"));
    send(&mut app, W::Choose(Chosen::Set(1)));
    let w = app.road.section_window.as_ref().expect("open");
    let missing = w.preview.missing.clone().expect("the warning");
    assert!(
        missing.contains("km 0+050.000") && missing.contains("alana katılmadı"),
        "{missing}"
    );
    assert!(
        w.preview.rows.iter().any(|r| !r.missing.is_empty()),
        "a section gives its stretches off the surface"
    );
    send(&mut app, W::Choose(Chosen::Set(0)));
    assert!(
        app.road
            .section_window
            .as_ref()
            .expect("open")
            .preview
            .missing
            .is_none(),
        "the main set has ground under its design"
    );
}

/// Enkesit on the side set, its first section over a stretch off the surface, light and dark; `.run/gui-evidence/yol-y99/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib section_window_tests::screens_missing -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_missing() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y99");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 760.0), "koyu")] {
        let mut app = app(true);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("section.editor"));
        send(&mut app, W::Choose(Chosen::Set(1)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y99-enkesit-arazisiz-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// ROAD-SEC-04 (docs/adr/1039): Enkesit says what a set's sections are computed from, as the query does.
#[test]
fn the_window_says_what_the_sections_are_computed_from() {
    let mut app = app(true);
    let _ = app.update(Message::Run("section.editor"));
    send(&mut app, W::Choose(Chosen::Set(0)));
    let said = app
        .road
        .section_window
        .as_ref()
        .expect("open")
        .preview
        .sources
        .clone()
        .expect("the sources");
    assert!(
        said.starts_with(
            "Kaynaklar: takım k-1 (1), aks g-1 (1), boykesit b-1 (1), tip kesit tk-1 (1)"
        ),
        "{said}"
    );
    // Docs/adr/1040: the ground by its digest of now, to eight digits, and its acceptance.
    let ground = said.split(", arazi t-1 (").nth(1).expect("the ground");
    assert!(ground.len() > 8 && ground[..8].bytes().all(|b| b.is_ascii_hexdigit()), "{said}");
    assert!(ground.ends_with("…, kabul edilmedi)"), "{said}");
}
