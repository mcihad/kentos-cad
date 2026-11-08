//! Düzenleme dağıtım taslağı (readjustment_window.rs, docs/adr/1460): the rule package typed, a CSV of synthetic
//! records read, the shared cases' first answer reproduced (totals kept, the remainder's receiver said), a refusal
//! shown with its place, nothing written; Harita and GIS offer it, Yol and Mimari do not. The scope (docs/adr/2060)
//! on the shared cases' drawing of `cad.readjustment.scope`: the selected boundary gives the case's statuses and bases,
//! Esas alanları kayıtlara yaz writes the bases into the records and says what did not match, the rule package's
//! places and the piece tolerance run it again; without a selected area there is no scope. Ayırma çapı (docs/adr/2280)
//! lists the partial parcels' parts corner by corner as the case gives them, and Çapı çizime yaz writes them on their
//! own layer in one undo step. And its pictures.

use kentos_launch::Module;

use crate::app::{App, Dialog, Message};
use crate::readjustment_window::{Event as R, View, records};

fn opened(program: Option<Module>) -> App {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.readjustment.distribute.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

const CSV: &str = "parsel;hak sahibi;tapu alanı;pay\n101/1;H1;1000;1/3\n101/1;H2;1000;2/3\n101/2;H2;500.50;1/2\n101/2;H3;500.50;1/2\nP;K1;100.01;1/3\nP;K2;100.01;1/3\nP;K3;100.01;1/3\n";

fn filled(app: &mut App) {
    filled_with(app, CSV);
}

fn filled_with(app: &mut App, csv: &str) {
    for e in [
        R::Id("ornek-paket".into()),
        R::Jurisdiction("Örnek kurum (sentetik)".into()),
        R::Source("Sentetik deneme belgesi".into()),
        R::Effective("2026-01-01".into()),
        R::Deduction("0.40".into()),
    ] {
        let _ = app.update(Message::Readjustment(e));
    }
    let dir = std::env::temp_dir().join(format!("kentos-duzenleme-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("kayitlar.csv");
    std::fs::write(&file, csv).expect("written");
    app.readjustment_file(file);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_records_are_read_and_the_totals_kept() {
    assert_eq!(records(CSV).expect("reads").len(), 7);
    assert!(records("a;b;c").is_err());
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("readjustment.distribute"));
    assert_eq!(app.dialog, Some(Dialog::Readjustment));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    filled(&mut app);
    let a = app.readjustment.answer.clone().expect("an answer").expect("distributed");
    assert!(a.draft && a.conserved);
    assert_eq!((a.registered.as_str(), a.deduction.as_str(), a.net.as_str()), ("1600.51", "640.20", "960.31"));
    let p = a.parcels.iter().find(|p| p.parcel == "P").expect("P");
    assert_eq!((p.remainder.as_str(), p.remainder_to.as_slice()), ("0.01", &["K1".to_owned()][..]));
    // A rule that refuses a remainder says so, with its place.
    let _ = app.update(Message::Readjustment(R::Remainder("reject".into())));
    let why = app.readjustment.answer.clone().expect("an answer").expect_err("refused");
    assert!(why.contains("reject") && why.contains("entries[4].share"), "{why}");
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

const BOUNDARY: &str = "0192f800-2060-7000-8000-000000000001";

fn scope_cases() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.readjustment.scope.json")).expect("cases")
}

/// The scope cases' drawing in Harita, its boundary selected or nothing.
fn bounded(select: bool) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(scope_cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    if select {
        let uid = kentos_domain::Uuid::parse_str(BOUNDARY).expect("a uid");
        let slot = app.document.as_ref().and_then(|d| d.model.slot_of(uid)).expect("the boundary");
        app.selection.set([slot]);
    }
    app
}

const SCOPE_CSV: &str = "parsel;hak sahibi;tapu alanı;pay\n101/1;H1;600.00;1/2\n101/1;H2;600.00;1/2\n101/3;H3;598.40;1/1\n101/4;H4;590;1/1\nK9;H5;100;1/1\n";

#[test]
fn the_selected_boundary_gives_the_case_s_scope_and_its_bases_go_into_the_records() {
    let mut app = bounded(true);
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("readjustment.distribute"));
    assert_eq!(app.dialog, Some(Dialog::Readjustment));
    let k = app.readjustment.scope.clone().expect("a scope").expect("read");
    let want = &scope_cases()["cases"][0]["steps"][0]["result"]["output"];
    assert_eq!(serde_json::Value::from(k.basis.as_str()), want["basis"]);
    assert_eq!(
        [k.whole, k.partial, k.ready, k.review].map(serde_json::Value::from),
        [&want["whole"], &want["partial"], &want["ready"], &want["review"]].map(Clone::clone)
    );
    let statuses: Vec<&str> = k.parcels.iter().map(|p| p.status.as_str()).collect();
    let wanted: Vec<&str> = want["parcels"].as_array().expect("parcels").iter().filter_map(|p| p["status"].as_str()).collect();
    assert_eq!(statuses, wanted);
    let _ = app.update(Message::Readjustment(R::View(View::Scope)));
    filled_with(&mut app, SCOPE_CSV);
    let _ = app.update(Message::Readjustment(R::Apply));
    let areas: Vec<&str> = app.readjustment.entries.iter().map(|e| e.area.as_str()).collect();
    assert_eq!(areas, ["600.00", "600.00", "333.33", "590", "100"], "101/3 takes the part inside; 101/4 has no basis");
    let lines: Vec<&str> = app.readjustment.applied.iter().map(|(_, l)| l.as_str()).collect();
    assert!(lines[0].contains("101/3 598.40 → 333.33"), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("101/4 (yanılma sınırı dışında)")), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("Kapsamda olmayan kayıt parseli: K9")), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("kaydı olmayan parsel: 101/2")), "{lines:?}");
    let a = app.readjustment.answer.clone().expect("an answer").expect("distributed");
    assert_eq!(a.parcels.iter().find(|p| p.parcel == "101/3").map(|p| p.area.as_str()), Some("333.33"));
    assert!(a.draft && a.conserved);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn the_policy_and_the_tolerance_run_the_scope_again() {
    let mut app = bounded(true);
    let _ = app.update(Message::Run("readjustment.distribute"));
    let status = |app: &App, name: &str| {
        let k = app.readjustment.scope.clone().expect("a scope").expect("read");
        k.parcels.iter().find(|p| p.parcel == name).map(|p| (p.status.clone(), p.basis.clone())).expect("the parcel")
    };
    assert_eq!(status(&app, "101/9"), ("sliver".to_owned(), None));
    let _ = app.update(Message::Readjustment(R::Scale(0.0)));
    assert_eq!(status(&app, "101/1"), ("finerThanPolicy".to_owned(), None));
    assert_eq!(status(&app, "101/3"), ("computed".to_owned(), Some("333".to_owned())));
    let _ = app.update(Message::Readjustment(R::Scale(2.0)));
    let _ = app.update(Message::Readjustment(R::Tolerance(0.003)));
    assert_eq!(status(&app, "101/9"), ("computed".to_owned(), Some("30.00".to_owned())), "the 4 mm piece counts");
}

/// docs/adr/2280: Ayırma çapı's rows are the case's parts' corners; Çapı çizime yaz writes each part as an area on
/// “Ayırma çapı” with its parcel, name and area; one undo step takes the parts and the new layer away.
#[test]
fn the_separation_sheet_lists_the_parts_and_writes_them() {
    let mut app = bounded(true);
    let _ = app.update(Message::Run("readjustment.distribute"));
    let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
    let k = app.readjustment.scope.clone().expect("a scope").expect("read");
    let parts: Vec<(&str, usize)> =
        k.parcels.iter().flat_map(|p| p.parts.iter().map(move |q| (p.parcel.as_str(), q.rings.iter().map(|r| r.pts.len()).sum()))).collect();
    let corners: usize = parts.iter().map(|(_, n)| n).sum();
    let rows = crate::readjustment_sheet::rows(&k, &app.format());
    assert_eq!(rows.len(), corners);
    assert_eq!(parts.len(), 9, "101/3, 101/6, 11 two each, 101/10 three");
    assert_eq!((rows[0][0].as_str(), rows[0][1].as_str(), rows[0][2].as_str()), ("101/3", "Giren", "1"));
    let ten: Vec<&str> = rows.iter().filter(|r| r[0] == "101/10" && r[2] == "1").map(|r| r[1].as_str()).collect();
    assert_eq!(ten, ["Giren", "Girmeyen 1", "Girmeyen 2"]);
    let _ = app.update(Message::Readjustment(R::WriteSheet));
    let doc = &mut app.document.as_mut().expect("open").model;
    let layer = doc.layers().leaves().into_iter().find(|l| l.name == "Ayırma çapı").map(|l| l.id.clone()).expect("the layer");
    let written: Vec<_> = doc.entities().filter(|e| e.base().layer_id == layer).cloned().collect();
    assert_eq!(written.len(), 9);
    let attrs = &written[0].base().attrs;
    assert_eq!((attrs["Parsel"].as_str(), attrs["Kısım"].as_str()), ("101/3", "Giren"));
    assert_eq!(doc.undo().as_deref(), Some("Ayırma çapı"));
    assert!(doc.layers().leaves().iter().all(|l| l.name != "Ayırma çapı"), "one step");
}

#[test]
fn without_a_selected_area_there_is_no_scope() {
    let mut app = bounded(false);
    let _ = app.update(Message::Run("readjustment.distribute"));
    assert!(app.readjustment.scope.is_none());
    filled(&mut app);
    let before = app.readjustment.entries.clone();
    let _ = app.update(Message::Readjustment(R::Apply));
    assert_eq!(app.readjustment.entries, before);
    assert!(app.readjustment.applied.is_empty());
}

#[test]
fn harita_and_gis_offer_it_yol_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, true), (Module::Road, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("readjustment.distribute"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/ayirma-capi-2280/`: Ayırma çapı's corners, and the drawing after Çapı çizime yaz.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib readjustment_window_tests::sheet_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn sheet_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ayirma-capi-2280");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (write, stem) in [(false, "1-capin-koseleri"), (true, "2-cizimde")] {
            let mut app = bounded(true);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("readjustment.distribute"));
            let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
            if write {
                let _ = app.update(Message::Readjustment(R::WriteSheet));
                let _ = app.update(Message::Readjustment(R::Close));
                // The other layers hidden: the sheet's parts alone.
                let doc = &mut app.document.as_mut().expect("open").model;
                let others: Vec<String> =
                    doc.layers().leaves().into_iter().filter(|l| l.name != "Ayırma çapı").map(|l| l.id.clone()).collect();
                for id in others {
                    doc.set_layer_visible(&id, false);
                }
                app.selection.clear();
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}

/// `.run/gui-evidence/duzenleme-taslagi-1460/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib readjustment_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/duzenleme-taslagi-1460");
    std::fs::create_dir_all(&out).expect("a folder");
    for (view, stem) in [(View::Holders, "1-hak-sahipleri"), (View::Parcels, "2-parseller")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")]
        {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("readjustment.distribute"));
            filled(&mut app);
            let _ = app.update(Message::Readjustment(R::View(view)));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
    // The scope (docs/adr/2060): Kapsam, then the parcels after the bases went into the records.
    for (view, stem) in [(View::Scope, "3-kapsam"), (View::Parcels, "4-esas-alanla-parseller")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")]
        {
            let mut app = bounded(true);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("readjustment.distribute"));
            filled_with(&mut app, SCOPE_CSV);
            let _ = app.update(Message::Readjustment(R::Apply));
            let _ = app.update(Message::Readjustment(R::View(view)));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
