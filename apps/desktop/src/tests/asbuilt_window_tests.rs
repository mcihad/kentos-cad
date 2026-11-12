//! Uygulama kontrolü (asbuilt_window.rs, docs/adr/1700) on the shared cases' drawing of `cad.asbuilt.check`: the
//! window opens on the measured layer, the design points, the kerb and the design surface guessed by their names, no
//! limit; the cases' limits typed in the form give the shared case's counts; a limit that is no number is said;
//! Dışındakileri seç selects the points not within; CSV kaydet writes the table; nothing is written to the drawing.
//! Harita, Yol and GIS offer it, Mimari does not. The returned record's field package gives the design digest and the
//! limits the form leaves empty (docs/adr/2420); paketin tasarımıyla compares the record with the package's own design
//! (docs/adr/2820). İzleme (docs/adr/3620) sets the second epoch against the first by name with the control points; the
//! form drops the design parts and the result speaks of movement. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::asbuilt_window::{Event as A, Show};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.check.json")).expect("cases")
}

fn opened(program: Option<Module>) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn limits(app: &mut App) {
    for e in [
        A::Search("0,5".into()),
        A::Horizontal("0,02".into()),
        A::Vertical("0,03".into()),
        A::QualityField("Hassasiyet".into()),
        A::Quality("0,05".into()),
    ] {
        let _ = app.update(Message::Asbuilt(e));
    }
}

#[test]
fn the_measured_points_against_the_design() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("asbuilt.check"));
    assert_eq!(app.dialog, Some(Dialog::Asbuilt));
    assert_eq!(
        (app.asbuilt.measured.as_str(), app.asbuilt.design.as_str(), app.asbuilt.lines.as_str(), app.asbuilt.surface.as_str()),
        ("olcum", "tasarim", "bordur", "t-1")
    );
    limits(&mut app);
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"];
    for key in ["within", "outside", "unmatched", "ambiguous", "lowQuality"] {
        let got = match key {
            "within" => r.within,
            "outside" => r.outside,
            "unmatched" => r.unmatched,
            "ambiguous" => r.ambiguous,
            _ => r.low_quality,
        };
        assert_eq!(Value::from(got), want[key], "{key}");
    }
    let _ = app.update(Message::Asbuilt(A::Show(Show::Faults)));
    let _ = app.update(Message::Asbuilt(A::Select));
    assert_eq!(app.dialog, None);
    let faults = (r.outside + r.unmatched + r.ambiguous) as usize;
    assert_eq!(app.selection.ids().len(), faults);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn a_limit_that_is_no_number_is_said() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("asbuilt.check"));
    let _ = app.update(Message::Asbuilt(A::Horizontal("iki".into())));
    assert_eq!(app.asbuilt.answer, Some(Err("Yatay tolerans “iki” bir sayı değil.".into())));
}

#[test]
fn csv_kaydet_writes_the_table() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("asbuilt.check"));
    limits(&mut app);
    let dir = std::env::temp_dir().join(format!("kentos-uygulama-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("kontrol.csv");
    app.asbuilt_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 11, "a head and ten points");
    assert!(lines[2].starts_with("P2;") && lines[2].contains(";Dışında;yatay dışında"), "{}", lines[2]);
    let _ = std::fs::remove_dir_all(dir);
}

/// Saha paketi yaz… (docs/adr/1920): the form's design and limits and the notes typed, a line each (empty lines left
/// out), written as the package by `cad.field.package`; its design digest is the check's; the drawing is not changed.
#[test]
fn the_field_package_is_written_from_the_form() {
    use kentos_formats::zip;
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("asbuilt.check"));
    limits(&mut app);
    app.asbuilt.notes =
        iced::widget::text_editor::Content::with_text("KN1 ve KN2'den başlanır.\n\n  Kotlar TUDKA-99'a göredir.  ");
    let dir = std::env::temp_dir().join(format!("kentos-saha-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    app.asbuilt_package(dir.join("saha"));
    let bytes = std::fs::read(dir.join("saha.zip")).expect("written, .zip added");
    let limits = zip::Limits::default();
    let listed = zip::list(&bytes, &limits).expect("a zip");
    let mut names = kentos_commands::FIELD_PACKAGE_FILES.to_vec();
    names.push(kentos_commands::FIELD_PACKAGE_SURFACE_FILE);
    assert_eq!(listed.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), names, "docs/adr/3220: the surface fourth");
    let files = zip::unpack(&bytes, &listed, &limits, |_| true).expect("unpacks");
    let manifest: Value = serde_json::from_slice(&files[0].data).expect("the manifest");
    assert_eq!(manifest["notes"], serde_json::json!(["KN1 ve KN2'den başlanır.", "Kotlar TUDKA-99'a göredir."]));
    assert_eq!(manifest["acceptance"], serde_json::json!({"horizontal": 0.02, "vertical": 0.03, "quality": 0.05}));
    assert_eq!(manifest["surface"]["name"], "Tasarım");
    assert_eq!(manifest["designDigest"], cases()["cases"][0]["steps"][0]["result"]["output"]["designDigest"]);
    let points = String::from_utf8(files[1].data.clone()).expect("text");
    assert_eq!(points.lines().count(), 9, "a head and eight design points");
    assert!(points.starts_with("Ad;Y;X;Z;Kimlik\nP1;487010;4420010;100.3;"), "{points}");
    let said = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(said.contains("Saha paketi yazıldı: 8 nokta, 2 çizgi, yüzey Tasarım (yuzey.xml, 200 üçgen), 2 not"), "{said}");
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written to the drawing");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn harita_yol_and_gis_offer_it_mimari_does_not() {
    for (module, yes) in [(Module::Map, true), (Module::Road, true), (Module::Gis, true), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("asbuilt.check"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/uygulama-kontrolu-1700/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib asbuilt_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/uygulama-kontrolu-1700");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("asbuilt.check"));
        limits(&mut app);
        app.asbuilt.notes = iced::widget::text_editor::Content::with_text(
            "Aplikasyona KN1 ve KN2'den başlanır.\nKotlar TUDKA-99'a göredir; ölçü GNSS RTK ile.",
        );
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-kontrol-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // The ribbon's place: Ölçme › Saha.
        let _ = app.update(Message::Asbuilt(A::Close));
        let _ = app.update(Message::RibbonTab("survey"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-serit-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

fn fixture(rel: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(rel)
}

/// docs/adr/2420: the record's package chosen with the form's limits empty gives the shared case's answer (its
/// horizontal and vertical limits taken, its quality limit not without the σ's attribute); the older package says the
/// design changed and its other system; Paketi kaldır takes it out.
#[test]
fn the_record_s_package_gives_the_digest_and_the_limits() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("asbuilt.check"));
    let _ = app.update(Message::Asbuilt(A::Search("0,5".into())));
    let _ = app.update(Message::Asbuilt(A::PickedPackage(Some(fixture("asbuilt/v1/files/saha-paketi.zip")))));
    let r = serde_json::to_value(app.asbuilt.answer.clone().expect("an answer").expect("checked")).expect("json");
    let case = cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "paket: σ özniteliği olmadan kalite sınırı alınmaz").cloned().expect("the case");
    let want = &case["steps"][0]["result"]["output"];
    for key in ["within", "outside", "unmatched", "ambiguous", "lowQuality"] {
        assert_eq!(r[key], want[key], "{key}");
    }
    assert_eq!(r["package"]["taken"], want["package"]["taken"]);
    assert_eq!(r["package"]["sameSystem"], true);
    assert!(r.get("designChanged").is_none_or(|v| v == false));
    let _ = app.update(Message::Asbuilt(A::PickedPackage(Some(fixture("asbuilt/v1/files/eski-paket.zip")))));
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    let p = r.package.clone().expect("the package");
    assert!(r.design_changed && !p.same_system && !p.same_epoch && !p.same_heights);
    let _ = app.update(Message::Asbuilt(A::DropPackage));
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    assert!(r.package.is_none() && !r.design_changed);
}

/// `.run/gui-evidence/saha-paketi-donus-2420/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib asbuilt_window_tests::package_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn package_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/saha-paketi-donus-2420");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name, file) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis", "asbuilt/v1/files/saha-paketi.zip"),
        ("dark", Size::new(1100.0, 760.0), "koyu-dar", "asbuilt/v1/files/eski-paket.zip"),
    ] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("asbuilt.check"));
        for e in [A::Search("0,5".into()), A::QualityField("Hassasiyet".into())] {
            let _ = app.update(Message::Asbuilt(e));
        }
        let _ = app.update(Message::Asbuilt(A::PickedPackage(Some(fixture(file)))));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-paketle-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/2820: paketin tasarımıyla, with the older design's package, gives the shared case's answer (P1 against
/// where the package had it, P7 not in it, the drawing's change said as a warning, not the record's error); without a
/// package the box cannot be ticked; Paketi kaldır takes it out.
#[test]
fn the_record_is_checked_with_its_package_s_own_design() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("asbuilt.check"));
    for e in [A::Search("0,5".into()), A::QualityField("Hassasiyet".into())] {
        let _ = app.update(Message::Asbuilt(e));
    }
    let _ = app.update(Message::Asbuilt(A::PickedPackage(Some(fixture("asbuilt/v1/files/eski-tasarim-paketi.zip")))));
    let _ = app.update(Message::Asbuilt(A::PackageDesign(true)));
    let r = serde_json::to_value(app.asbuilt.answer.clone().expect("an answer").expect("checked")).expect("json");
    let case = cases()["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"] == "paketin tasarımı: eski tasarım; kayıt sahaya verilenle, çizimdeki tasarım değişti")
        .cloned()
        .expect("the case");
    let want = &case["steps"][0]["result"]["output"];
    for key in ["designDigest", "designChanged", "within", "outside", "unmatched", "ambiguous", "lowQuality", "missing"] {
        assert_eq!(r[key], want[key], "{key}");
    }
    assert_eq!(r["package"]["design"], true);
    let _ = app.update(Message::Asbuilt(A::DropPackage));
    assert!(!app.asbuilt.record_design, "the box goes with the package");
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    assert!(r.package.is_none());
}

/// docs/adr/3220: with the package that carries its surface, paketin tasarımıyla gives the shared case's answer (the
/// surface differences against the package's plane, 5 cm under the drawing's, its north half off it) and says the
/// drawing's surface changed since as a warning; the package line names the surface.
#[test]
fn the_record_is_checked_with_its_package_s_own_surface() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("asbuilt.check"));
    for e in [A::Search("0,5".into()), A::QualityField("Hassasiyet".into())] {
        let _ = app.update(Message::Asbuilt(e));
    }
    let _ = app.update(Message::Asbuilt(A::PickedPackage(Some(fixture("asbuilt/v1/files/yuzeyli-paket.zip")))));
    let _ = app.update(Message::Asbuilt(A::PackageDesign(true)));
    let r = serde_json::to_value(app.asbuilt.answer.clone().expect("an answer").expect("checked")).expect("json");
    let case = cases()["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"] == "paketin yüzeyi: çizimdeki yüzey paketinkinden sonra değişmiş")
        .cloned()
        .expect("the case");
    let want = &case["steps"][0]["result"]["output"];
    for key in ["surfaceChanged", "within", "outside", "unmatched", "ambiguous", "lowQuality"] {
        assert_eq!(r[key], want[key], "{key}");
    }
    for (g, w) in r["points"].as_array().expect("points").iter().zip(want["points"].as_array().expect("points")) {
        assert_eq!(g["flags"], w["flags"], "{}", w["name"]);
    }
    assert_eq!(r["package"]["surface"], true);
    let checked = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    let said: Vec<&str> = crate::asbuilt_window::changes(&checked).into_iter().map(|(_, w)| w).collect();
    assert_eq!(said, ["Çizimdeki yüzey paketten sonra değişmiş; yüzey farkları sahaya verilen yüzeyle, paketin yüzeyiyle (yuzey.xml)."]);
    let package = checked.package.as_ref().expect("the package");
    assert_eq!(crate::asbuilt_window::package_design_words(package), "tasarım paketin noktaları, çizgileri ve yüzeyi");
}

/// `.run/gui-evidence/paketin-yuzeyi-3220/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib asbuilt_window_tests::package_surface_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn package_surface_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/paketin-yuzeyi-3220");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("asbuilt.check"));
        for e in [A::Search("0,5".into()), A::QualityField("Hassasiyet".into())] {
            let _ = app.update(Message::Asbuilt(e));
        }
        let _ = app.update(Message::Asbuilt(A::PickedPackage(Some(fixture("asbuilt/v1/files/yuzeyli-paket.zip")))));
        let _ = app.update(Message::Asbuilt(A::PackageDesign(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-paketin-yuzeyi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// `.run/gui-evidence/paketin-tasarimi-2820/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib asbuilt_window_tests::package_design_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn package_design_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/paketin-tasarimi-2820");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("asbuilt.check"));
        for e in [A::Search("0,5".into()), A::QualityField("Hassasiyet".into())] {
            let _ = app.update(Message::Asbuilt(e));
        }
        let _ = app.update(Message::Asbuilt(A::PickedPackage(Some(fixture("asbuilt/v1/files/eski-tasarim-paketi.zip")))));
        let _ = app.update(Message::Asbuilt(A::PackageDesign(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-paketin-tasarimi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// The monitoring job of the shared cases typed in the form (docs/adr/3620).
fn watch(app: &mut App) {
    for e in [
        A::Kind(2),
        A::Measured("donem2".into()),
        A::Design("donem1".into()),
        A::Horizontal("0,008".into()),
        A::Vertical("0,01".into()),
        A::QualityField("Hassasiyet".into()),
        A::Quality("0,01".into()),
        A::Control("K1, K2".into()),
    ] {
        let _ = app.update(Message::Asbuilt(e));
    }
}

#[test]
fn a_monitoring_job_sets_the_second_epoch_against_the_first() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("asbuilt.check"));
    watch(&mut app);
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    let all = cases();
    let case = all["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("izleme: iki dönem adla")))
        .expect("the case");
    let want = &case["steps"][0]["result"]["output"];
    for (key, got) in [("within", r.within), ("outside", r.outside), ("unmatched", r.unmatched), ("ambiguous", r.ambiguous), ("lowQuality", r.low_quality)] {
        assert_eq!(Value::from(got), want[key], "{key}");
    }
    assert_eq!(r.kind, Some(kentos_commands::AsbuiltKind::Monitoring));
    let c = r.control.clone().expect("the control points");
    assert_eq!((c.checked, c.moved, c.missing), (vec!["K1".to_owned(), "K2".to_owned()], vec!["K2".to_owned()], Vec::<String>::new()));
    // The design parts the window guessed stay out of a monitoring job (no surface difference, no line).
    assert_eq!((app.asbuilt.surface.as_str(), app.asbuilt.lines.as_str()), ("t-1", "bordur"));
    assert!(r.points.iter().all(|p| p.surface.is_none() && p.normal.is_none()));
    let table = crate::asbuilt_window::csv(&r);
    assert!(table.starts_with("Nokta;Nesne;Önceki dönemin noktası;"), "{table}");
    assert!(table.contains("Sınırı aştı") && table.contains("Yeni nokta") && table.contains("yatay sınırı aştı"), "{table}");
    // Back to the as-built job: the design check as before, no kind said.
    for e in [A::Kind(0), A::Measured("olcum".into()), A::Design("tasarim".into())] {
        let _ = app.update(Message::Asbuilt(e));
    }
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    assert_eq!((r.kind, r.control.is_none()), (None, true));
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

/// `.run/gui-evidence/izleme-3620/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib asbuilt_window_tests::monitoring_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn monitoring_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/izleme-3620");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("asbuilt.check"));
        watch(&mut app);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-izleme-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::Asbuilt(A::Show(Show::Faults)));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-hareket-edenler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/4020: with the surface alone, every point's nearest distance is the shared case's; the case's limit between
/// P3's height difference and its nearest distance flags P3 by Düşey fark and not by En yakın uzaklık.
#[test]
fn the_nearest_distance_to_the_surface_is_the_shared_case_s() {
    let case = |prefix: &str| {
        cases()["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(prefix)))
            .expect(prefix)["steps"][0]
            .clone()
    };
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("asbuilt.check"));
    for e in [A::Design(crate::asbuilt_window::NONE.into()), A::Lines(crate::asbuilt_window::NONE.into()), A::QualityField(String::new())] {
        let _ = app.update(Message::Asbuilt(e));
    }
    let want = case("yüzeye en yakın uzaklık: her noktanın");
    let r = serde_json::to_value(app.asbuilt.answer.clone().expect("an answer").expect("checked")).expect("json");
    for (g, w) in r["points"].as_array().expect("points").iter().zip(want["result"]["output"]["points"].as_array().expect("points")) {
        match (g["nearest"].as_f64(), w["nearest"].as_f64()) {
            (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "{}: {a} {b}", g["name"]),
            (a, b) => assert_eq!(a, b, "{}", g["name"]),
        }
    }
    let limit = case("yüzeye en yakın uzaklık: düşey sınır")["input"]["vertical"].as_f64().expect("a limit");
    let _ = app.update(Message::Asbuilt(A::Vertical(limit.to_string().replace('.', ","))));
    let p3 = |app: &App| {
        let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
        r.points.iter().find(|p| p.name == "P3").expect("P3").flags.clone()
    };
    assert!(p3(&app).contains(&"outsideSurface".to_owned()));
    let _ = app.update(Message::Asbuilt(A::Nearest(true)));
    assert!(!p3(&app).contains(&"outsideSurface".to_owned()));
}

/// docs/adr/4020: the window with Yüzey farkı and the En yakın column, `.run/gui-evidence/yuzeye-en-yakin-4020/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib asbuilt_window_tests::nearest_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn nearest_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yuzeye-en-yakin-4020");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("asbuilt.check"));
        limits(&mut app);
        let _ = app.update(Message::Asbuilt(A::Nearest(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("uygulama-kontrolu-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
