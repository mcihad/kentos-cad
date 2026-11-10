//! Düzenleme dağıtım taslağı (readjustment_window.rs, docs/adr/1460): the rule package typed, a CSV of synthetic
//! records read, the shared cases' first answer reproduced (totals kept, the remainder's receiver said), a refusal
//! shown with its place, nothing written; Harita and GIS offer it, Yol and Mimari do not. The scope (docs/adr/2060)
//! on the shared cases' drawing of `cad.readjustment.scope`: the selected boundary gives the case's statuses and bases,
//! Esas alanları kayıtlara yaz writes the bases into the records and says what did not match, the rule package's
//! places and the piece tolerance run it again; without a selected area there is no scope. Ayırma çapı (docs/adr/2280)
//! lists the partial parcels' parts corner by corner as the case gives them, and Çapı çizime yaz writes them on their
//! own layer in one undo step. Çapın paftaları (docs/adr/2680) makes a separation sheet for each parcel partly inside,
//! its points on the parcel's own layer, in one undo step. And its pictures.

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
    // A folder of its own for each call: tests run side by side in one process, and a shared one let one test read
    // another's records or lose its file to the other's clean-up.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("kentos-duzenleme-{}-{k}", std::process::id()));
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
    bounded_on(scope_cases()["setup"].clone(), select)
}

/// The scope cases' drawing with parcel 11 given Ada 101: every parcel partly inside has an identity (docs/adr/2680).
fn named() -> App {
    let mut setup = scope_cases()["setup"].clone();
    for e in setup["entities"].as_array_mut().expect("entities") {
        if e["attrs"]["Parsel"] == "11" && e["attrs"].get("Ada").is_none() {
            e["attrs"]["Ada"] = "101".into();
        }
    }
    bounded_on(setup, true)
}

fn bounded_on(setup: serde_json::Value, select: bool) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
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
    // A part's first row carries its area; the corners are the sheet's point numbers (docs/adr/2680).
    let ten: Vec<&str> = rows.iter().filter(|r| r[0] == "101/10" && !r[5].is_empty()).map(|r| r[1].as_str()).collect();
    assert_eq!(ten, ["Giren", "Girmeyen 1", "Girmeyen 2"]);
    let girmeyen: Vec<&str> =
        rows.iter().filter(|r| r[0] == "101/3" && r[1] == "Girmeyen").map(|r| r[2].as_str()).collect();
    assert_eq!(girmeyen, ["3", "2", "5", "6"], "the corners shared with Giren keep its numbers");
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

/// docs/adr/3080: the parcel numbers of 101/10's two parts outside typed in Dış parça no: read by the parcel's name
/// into its id; a parcel not partial and a line that does not read are said; Çapın paftaları writes them (the command's
/// parts table names each outside part with its number); a number short is the command's refusal, nothing written.
#[test]
fn the_parts_outside_take_the_typed_parcel_numbers() {
    let mut app = named();
    let _ = app.update(Message::Run("readjustment.distribute"));
    let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
    let k = app.readjustment.scope.clone().expect("a scope").expect("read");
    let ten = k.parcels.iter().find(|p| p.parcel == "101/10").expect("101/10").uid.clone();
    let typed = crate::readjustment_sheet::typed_numbers("101/10: 101/24, 101/25", &k).expect("reads").expect("some");
    assert_eq!(typed.get(&ten).map(Vec::as_slice), Some(&["101/24".to_owned(), "101/25".to_owned()][..]));
    // Why a parcel takes no number (docs/adr/3880): wholly inside, wholly outside, not of the scope.
    let why = |text: &str| crate::readjustment_sheet::typed_numbers(text, &k).expect_err("not partial");
    let kind = |name: &str| k.parcels.iter().find(|p| p.parcel == name).map(|p| p.kind.clone());
    assert_eq!(kind("101/1").as_deref(), Some("whole"));
    assert_eq!(why("101/1: 5"), "Dış parça no: “101/1” parseli düzenleme sınırının tamamen içinde, dışarıda parçası yok.");
    if kind("101/8").as_deref() == Some("outside") {
        assert_eq!(why("101/8: 5"), "Dış parça no: “101/8” parseli düzenleme sınırının tamamen dışında, düzenlemeye girmiyor.");
    }
    assert_eq!(why("999/9: 5"), "Dış parça no: “999/9” parseli kapsamda kısmen giren bir parsel değil.");
    assert!(crate::readjustment_sheet::typed_numbers("101/10 101/24", &k).expect_err("no colon").contains("okunmadı"));
    assert_eq!(crate::readjustment_sheet::typed_numbers("  ", &k), Ok(None));
    // One number short: the command refuses, nothing written, the window stays.
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Readjustment(R::Numbers("101/10: 101/24".into())));
    let _ = app.update(Message::Readjustment(R::MakeSheets));
    assert!(app.log.last().expect("said").text.contains("2 parçası var; 1 parsel numarası verildi"));
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision);
    // Both: the sheets are made.
    let _ = app.update(Message::Readjustment(R::Numbers("101/10: 101/24, 101/25".into())));
    let _ = app.update(Message::Readjustment(R::MakeSheets));
    assert_eq!(app.dialog, None, "the sheets are made");
    // docs/adr/3480: on 101/10's layer of numbered pieces each part as an area with its number, and a point inside it
    // labelled with the number; the points' layer keeps the corners only (the coordinate list reads it).
    let doc = &app.document.as_ref().expect("open").model;
    let named_layer = |name: &str| doc.layers().leaves().into_iter().find(|l| l.name == name).map(|l| l.id.clone());
    let corners = named_layer("101/10").expect("101/10's points");
    assert!(doc.entities().filter(|e| e.base().layer_id == corners).all(|e| matches!(e, kentos_commands::Entity::Point(_)) && e.base().attrs.contains_key("Nokta")));
    let layer = named_layer("101/10 dış parçalar").expect("101/10's pieces");
    let on: Vec<_> = doc.entities().filter(|e| e.base().layer_id == layer).cloned().collect();
    let parts: Vec<_> = on.iter().filter(|e| matches!(e, kentos_commands::Entity::Polygon(_))).collect();
    let numbers: Vec<&str> = parts.iter().map(|e| e.base().attrs["ParselNo"].as_str()).collect();
    assert_eq!(numbers, ["101/24", "101/25"]);
    for part in &parts {
        let kentos_commands::Entity::Polygon(area) = part else { unreachable!() };
        let number = &part.base().attrs["ParselNo"];
        let point = on
            .iter()
            .find_map(|e| match e {
                kentos_commands::Entity::Point(q) if e.base().label.as_deref() == Some(number.as_str()) => Some(q.p),
                _ => None,
            })
            .expect("its label");
        let (x0, x1) = area.pts.iter().fold((f64::MAX, f64::MIN), |(a, b), q| (a.min(q.x), b.max(q.x)));
        let (y0, y1) = area.pts.iter().fold((f64::MAX, f64::MIN), |(a, b), q| (a.min(q.y), b.max(q.y)));
        assert!(point.x > x0 && point.x < x1 && point.y > y0 && point.y < y1, "{number} inside its part");
    }
}

/// docs/adr/2680: Çapın paftaları over a drawing where a parcel partly inside has no Ada says so and writes nothing;
/// with every such parcel named, one sheet each in the scope's order, the points numbered on the parcel's own layer
/// under “Ayırma çapı noktaları”, each map freezing the other parcels' point layers and its coordinate list reading its
/// own; the book goes to PDF (the tables' — and − are in the drawing font); one undo step takes it all.
#[test]
fn a_separation_sheet_is_made_for_each_partial_parcel() {
    use kentos_sheet::{CoordSource, ItemKind};
    let mut app = bounded(true);
    let _ = app.update(Message::Run("readjustment.distribute"));
    let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Readjustment(R::MakeSheets));
    let said = app.log.last().expect("said");
    assert_eq!(said.level, kentos_interaction::Level::Warn);
    assert!(said.text.contains("parselinin Ada'sı ya da Parsel'i yok"), "{}", said.text);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
    assert_eq!(app.dialog, Some(Dialog::Readjustment), "the window stays");

    let mut app = named();
    let _ = app.update(Message::Run("readjustment.distribute"));
    let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
    let _ = app.update(Message::Readjustment(R::MakeSheets));
    assert_eq!(app.dialog, None, "the window closes on the sheets");
    let doc = &app.document.as_ref().expect("open").model;
    let book = kentos_sheet::project::read_project_sheets(doc.sheets().expect("a book")).expect("reads").0;
    let names: Vec<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Ayırma çapı 101/3", "Ayırma çapı 101/6", "Ayırma çapı 101/10", "Ayırma çapı 101/11"]);
    let group = doc.layers().nodes().iter().find(|n| n.name == "Ayırma çapı noktaları").expect("the group");
    let layers: Vec<(String, String)> = group.children.iter().map(|c| (c.name.clone(), c.id.clone())).collect();
    assert_eq!(layers.iter().map(|l| l.0.as_str()).collect::<Vec<_>>(), ["101/3", "101/6", "101/10", "101/11"]);
    let labels = |layer: &str| -> Vec<String> {
        doc.by_layer(layer).map(|e| e.base().label.clone().unwrap_or_default()).collect()
    };
    assert_eq!(labels(&layers[0].1), ["1", "2", "3", "4", "5", "6"]);
    assert_eq!(labels(&layers[2].1).len(), 8, "101/10: two pieces outside, eight points");
    let first = &book.sheets[0];
    let map = first.items.iter().find_map(|i| match &i.kind {
        ItemKind::Map(m) => Some(m),
        _ => None,
    });
    let frozen: Vec<&str> =
        map.expect("a map").layer_overrides.iter().filter(|o| o.frozen).map(|o| o.layer.as_str()).collect();
    assert_eq!(frozen, [layers[1].1.as_str(), layers[2].1.as_str(), layers[3].1.as_str()]);
    let list = first.items.iter().find_map(|i| match &i.kind {
        ItemKind::CoordinateList(c) => Some(c),
        _ => None,
    });
    assert!(matches!(&list.expect("a list").source, CoordSource::Layer(l) if l.layer == layers[0].1));
    let pdf = kentos_plot::book::book_pdf(doc, &[], "test", "2026-11-08");
    assert!(pdf.is_ok(), "{:?}", pdf.err());
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Model));
    let _ = app.update(Message::Run("edit.undo"));
    let doc = &app.document.as_ref().expect("open").model;
    assert!(doc.layers().nodes().iter().all(|n| n.name != "Ayırma çapı noktaları"), "one step: the layers");
    let left = doc.sheets().map(|s| kentos_sheet::project::read_project_sheets(s).expect("reads").0.sheets.len());
    assert_eq!(left.unwrap_or(0), 0, "one step: the sheets");
}

/// `.run/gui-evidence/ayirma-capi-paftasi-2680/`: the Ayırma çapı view with its two buttons, and the sheets made
/// (101/3's and 101/10's, two pieces outside), light and wide, dark and narrow.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib readjustment_window_tests::sheets_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn sheets_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ayirma-capi-paftasi-2680");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = named();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("readjustment.distribute"));
        let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, stem: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        shot(&mut app, "1-pencere");
        let _ = app.update(Message::Readjustment(R::MakeSheets));
        shot(&mut app, "2-pafta-101-3");
        let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::InspectorTab(kentos_sheet_ui::InspectorTab::Preflight)));
        shot(&mut app, "3-on-denetim");
        let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(3)));
        shot(&mut app, "4-pafta-101-10");
        if name == "acik-genis" {
            // The book as it prints, to read the tables at full size.
            let doc = &app.document.as_ref().expect("open").model;
            let pdf = kentos_plot::book::book_pdf(doc, &[], "KentOS", "2026-11-08").expect("a PDF");
            std::fs::write(out.join("paftalar.pdf"), &pdf.bytes).expect("writes");
            for (sheet, f) in &pdf.sheet_findings {
                println!("ön denetim {sheet}: {:?} {} {}", f.severity, f.code, f.message);
            }
        }
    }
}

/// `.run/gui-evidence/dis-parca-3080/`: the Ayırma çapı view with Dış parça no typed for 101/10's two pieces outside,
/// and 101/10's sheet whose table of parts names them, light and wide, dark and narrow; the book as PDF.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib readjustment_window_tests::numbers_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn numbers_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dis-parca-3080");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = named();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("readjustment.distribute"));
        let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
        let _ = app.update(Message::Readjustment(R::Numbers("101/10: 101/24, 101/25".into())));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, stem: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        shot(&mut app, "1-pencere");
        let _ = app.update(Message::Readjustment(R::MakeSheets));
        let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(3)));
        shot(&mut app, "2-pafta-101-10");
        if name == "acik-genis" {
            let doc = &app.document.as_ref().expect("open").model;
            let pdf = kentos_plot::book::book_pdf(doc, &[], "KentOS", "2026-11-09").expect("a PDF");
            std::fs::write(out.join("paftalar.pdf"), &pdf.bytes).expect("writes");
        }
    }
}

/// docs/adr/3880: Dış parça no typed for 101/1, a parcel wholly inside the boundary: the window says why it takes no
/// number; light wide, `.run/gui-evidence/dis-parca-ret-3880/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib readjustment_window_tests::refusal_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn refusal_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dis-parca-ret-3880");
    std::fs::create_dir_all(&out).expect("a folder");
    let size = Size::new(1440.0, 900.0);
    let mut app = named();
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("view.zoomExtents"));
    let _ = app.update(Message::Run("readjustment.distribute"));
    let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
    let _ = app.update(Message::Readjustment(R::Numbers("101/1: 5".into())));
    // Çapın paftaları: the typed numbers read, the parcel's reason said.
    let _ = app.update(Message::Readjustment(R::MakeSheets));
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("pencere-acik-genis.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}

/// docs/adr/3480: 101/10's sheet after Dış parça no, its two pieces outside drawn with their numbers inside; and the
/// drawing on 101/10; light wide and dark narrow, `.run/gui-evidence/dis-parca-no-3480/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib readjustment_window_tests::label_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn label_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dis-parca-no-3480");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = named();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("readjustment.distribute"));
        let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
        let _ = app.update(Message::Readjustment(R::Numbers("101/10: 101/24, 101/25".into())));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let _ = app.update(Message::Readjustment(R::MakeSheets));
        let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(3)));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pafta-101-10-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
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

/// docs/adr/4280: Numara yazısı 2 mm with 101/10's numbers: the numbers are texts, not labelled points, the sheet's
/// scale times 2 mm tall and turned along the 4 m wide pieces' long sides; a height that does not read is said and
/// nothing is written.
#[test]
fn the_numbers_typed_with_a_height_are_texts_fitted_in_their_parts() {
    let mut app = named();
    let _ = app.update(Message::Run("readjustment.distribute"));
    let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
    let _ = app.update(Message::Readjustment(R::Numbers("101/10: 101/24, 101/25".into())));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Readjustment(R::NumberHeight("iki".into())));
    let _ = app.update(Message::Readjustment(R::MakeSheets));
    assert!(app.log.last().expect("said").text.contains("Numara yazısı: “iki” bir sayı değil"));
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision);
    let _ = app.update(Message::Readjustment(R::NumberHeight("2".into())));
    let _ = app.update(Message::Readjustment(R::MakeSheets));
    assert_eq!(app.dialog, None, "the sheets are made");
    let doc = &app.document.as_ref().expect("open").model;
    let layer = doc.layers().leaves().into_iter().find(|l| l.name == "101/10 dış parçalar").map(|l| l.id.clone()).expect("101/10's pieces");
    let on: Vec<_> = doc.entities().filter(|e| e.base().layer_id == layer).cloned().collect();
    assert!(!on.iter().any(|e| matches!(e, kentos_commands::Entity::Point(_))), "no labelled point");
    let texts: Vec<_> = on
        .iter()
        .filter_map(|e| match e {
            kentos_commands::Entity::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(texts.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["101/24", "101/25"]);
    // 101/10's sheet is at 1:1000 (the shared case's): 2 mm on paper is 2 m.
    let scale = 1000.0;
    for t in &texts {
        assert!((t.height - 2.0 * scale / 1000.0).abs() < 1e-9, "{}: {}", t.text, t.height);
        assert!((t.rotation.abs() - 90.0).abs() < 1e-9, "{}: {}", t.text, t.rotation);
        assert_eq!(t.base.attrs["ParselNo"], t.text);
    }
}

/// docs/adr/4680: with Sığmayan numara on, at 6 mm the numbers of 101/10's 4 m pieces do not fit at the full height: they
/// are leaders on its pieces' layer, 6 m tall (the sheet's 1:1000), their tips inside the pieces and their lines running
/// east past them; no text is stepped down.
#[test]
fn the_numbers_too_big_go_outside_on_leaders() {
    let mut app = named();
    let _ = app.update(Message::Run("readjustment.distribute"));
    let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
    let _ = app.update(Message::Readjustment(R::Numbers("101/10: 101/24, 101/25".into())));
    let _ = app.update(Message::Readjustment(R::NumberHeight("6".into())));
    let _ = app.update(Message::Readjustment(R::NumberLeader(true)));
    let _ = app.update(Message::Readjustment(R::MakeSheets));
    assert_eq!(app.dialog, None, "the sheets are made");
    let doc = &app.document.as_ref().expect("open").model;
    let layer = doc.layers().leaves().into_iter().find(|l| l.name == "101/10 dış parçalar").map(|l| l.id.clone()).expect("101/10's pieces");
    let on: Vec<_> = doc.entities().filter(|e| e.base().layer_id == layer).cloned().collect();
    assert!(!on.iter().any(|e| matches!(e, kentos_commands::Entity::Text(_))), "no text stepped down");
    let leaders: Vec<_> = on
        .iter()
        .filter_map(|e| match e {
            kentos_commands::Entity::Leader(l) => Some(l.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(leaders.iter().map(|l| l.text.clone().unwrap_or_default()).collect::<Vec<_>>(), ["101/24", "101/25"]);
    for l in &leaders {
        assert!((l.height - 6.0).abs() < 1e-9, "{:?}", l.height);
        assert_eq!(l.rotation, 0.0);
        let (tip, start) = (l.pts[0], l.pts[l.pts.len() - 1]);
        assert!(start.x > tip.x && (start.y - tip.y).abs() < 1e-9, "east of the tip: {tip:?} → {start:?}");
        assert_eq!(l.base.attrs["ParselNo"], l.text.clone().unwrap_or_default());
    }
}

/// docs/adr/4680: the window with Sığmayan numara on and the model on 101/10's pieces with their numbers on leaders;
/// `.run/gui-evidence/numara-kilavuz-4680/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib readjustment_window_tests::leader_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn leader_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/numara-kilavuz-4680");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (made, file) in [(false, "1-pencere"), (true, "2-model-kilavuzlar")] {
            let mut app = named();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("readjustment.distribute"));
            let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
            let _ = app.update(Message::Readjustment(R::Numbers("101/10: 101/24, 101/25".into())));
            let _ = app.update(Message::Readjustment(R::NumberHeight("6".into())));
            let _ = app.update(Message::Readjustment(R::NumberLeader(true)));
            if made {
                let _ = app.update(Message::Readjustment(R::MakeSheets));
                let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(0)));
                // 101/10's two 4 m pieces outside and their leaders running east.
                let b = kentos_commands::Bounds { min_x: 486980.0, min_y: 4419970.0, max_x: 487045.0, max_y: 4420070.0 };
                app.zoom_to_bounds(&b);
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}

/// docs/adr/4280: 101/10's sheet with the numbers fitted at 2 mm, turned along the narrow pieces; light wide and dark
/// narrow, `.run/gui-evidence/numara-sigdir-4280/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib readjustment_window_tests::fit_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn fit_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/numara-sigdir-4280");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (made, file) in [(0, "1-pencere"), (1, "2-pafta-101-10"), (2, "3-model-dar-parca")] {
            let mut app = named();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("readjustment.distribute"));
            let _ = app.update(Message::Readjustment(R::View(View::Sheet)));
            let _ = app.update(Message::Readjustment(R::Numbers("101/10: 101/24, 101/25".into())));
            let _ = app.update(Message::Readjustment(R::NumberHeight("2".into())));
            if made > 0 {
                let _ = app.update(Message::Readjustment(R::MakeSheets));
                let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(3)));
            }
            if made == 2 {
                // The model on 101/10's upper piece outside, 4 m wide: its number turned along it.
                let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(0)));
                let b = kentos_commands::Bounds { min_x: 486984.0, min_y: 4420046.0, max_x: 487000.0, max_y: 4420064.0 };
                app.zoom_to_bounds(&b);
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}
