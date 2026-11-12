//! Sets and output (sheet steps 17–26): page setup, plot profiles and ink, printing and calibration,
//! publishing, packages, preflight, snapshots and detail sets.

use super::*;

pub(super) fn multipage_set_app(theme: &str, text: u32) -> App {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/multipage-sets.json"
    ))
    .unwrap();
    let c = &data["cases"][8];
    let mut model = kentos_domain::Document::from_snapshot(
        serde_json::from_value(c["model"].clone()).unwrap(),
    )
    .unwrap();
    let book: kentos_sheet::SheetBook = serde_json::from_value(c["book"].clone()).unwrap();
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let (mut app, _store) = app_with_store("sheet17-sets");
    app.settings.choose(&[
        ("appearance.theme", serde_json::json!(theme)),
        ("appearance.textSize", serde_json::json!(text)),
    ]);
    app.apply_settings();
    let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    sheet(&mut app, Sheet::Tab(1));
    sheet(
        &mut app,
        Sheet::Pdf(kentos_sheet_ui::message::PdfMessage::Scope(
            kentos_sheet_ui::message::PdfScope::All,
        )),
    );
    app
}

#[test]
fn desktop_set_pdf_resolves_each_master_map_and_the_four_named_architectural_views() {
    let app = multipage_set_app("light", 13);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let painter = app.sheet_painter();
    let (bytes, _) = app
        .sheets
        .pdf_and_findings(&painter, app.sheet_pdf_host())
        .unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-sheet17-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("set.pdf");
    std::fs::write(&path, bytes).unwrap();
    let read = std::process::Command::new("pdftotext")
        .args(["-layout"])
        .arg(&path)
        .arg("-")
        .output()
        .unwrap();
    assert!(read.status.success());
    let words = String::from_utf8(read.stdout).unwrap();
    let pages = words.split('\u{c}').collect::<Vec<_>>();
    assert!(!pages[0].contains("FAR"));
    for (n, word) in [
        (0, "CENTER"),
        (1, "FAR"),
        (2, "PLAN"),
        (3, "ELEV"),
        (4, "SECT"),
        (5, "DETAIL"),
    ] {
        assert!(pages[n].contains(word), "page {} {words}", n + 1);
    }
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore="GPU images and PDF, run alone"]
fn sheet17_multipage_set_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = multipage_set_app(theme, text);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        for (tab, label) in [(1, "small"), (2, "large"), (3, "plan"), (5, "section")] {
            sheet(&mut app, Sheet::Tab(tab));
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme())
                .save(out.join(format!("sheet17-{label}-{theme}-{text}.png")))
                .unwrap();
        }
        if theme == "light" {
            let painter = app.sheet_painter();
            std::fs::write(
                out.join("sheet17-set.pdf"),
                app.sheets.pdf(&painter, app.sheet_pdf_host()).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
#[ignore="GPU page setup window, run alone"]
fn sheet18_page_setup_screens() {
    use kentos_sheet_ui::page_setup::{SetupMessage as P, Area};
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = multipage_set_app(theme, text);
        sheet(&mut app, Sheet::Tab(3));
        for m in [
            P::Open,
            P::Name("Ofis · A3 baskı".into()),
            P::Area(Area::Window),
            P::Window(0, 10.),
            P::Window(1, 10.),
            P::Window(2, 390.),
            P::Window(3, 250.),
            P::Fit(true),
            P::Center(true),
        ] {
            sheet(&mut app, Sheet::PageSetup(m));
        }
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet18-page-setup-{theme}-{text}.png")))
            .unwrap();
        sheet(&mut app, Sheet::PageSetup(P::Save));
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        assert!(app.sheets.open_sheet().unwrap().plot.is_some());
    }
}

#[test]
#[ignore="GPU profile choice window, run alone"]
fn sheet19_plot_profile_screens() {
    use kentos_sheet_ui::page_setup::SetupMessage as P;
    use kentos_sheet::{
        ops::Op,
        plot_profile::{SavePlotProfiles, PlotProfile},
    };
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/plot-profiles.json"
    ))
    .unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = multipage_set_app(theme, text);
        sheet(&mut app, Sheet::Tab(3));
        let profiles: Vec<PlotProfile> = [0, 10]
            .into_iter()
            .map(|i| {
                serde_json::from_value(data["cases"][i]["exchange"]["profile"].clone()).unwrap()
            })
            .collect();
        let id = profiles[1].id.clone();
        assert!(app.sheets.apply(
            vec![Op::SavePlotProfiles(SavePlotProfiles { profiles })],
            "Baskı profilleri"
        ));
        for m in [P::Open, P::Name("Profil ile baskı".into()), P::Profile(id)] {
            sheet(&mut app, Sheet::PageSetup(m));
        }
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet19-profiles-{theme}-{text}.png")))
            .unwrap();
        if theme=="light" {
            use kentos_ui::snapshot::Input;
            snap.input(&mut app,App::view,&mut update,Input::Click(iced::Point::new(700.,566.)));
            snap.render(app.view(),&app.theme()).save(out.join("sheet19-profiles-menu.png")).unwrap();
            snap.input(&mut app,App::view,&mut update,Input::Key(iced::keyboard::key::Named::ArrowUp));
            snap.input(&mut app,App::view,&mut update,Input::Key(iced::keyboard::key::Named::Enter));
        }
        sheet(&mut app, Sheet::PageSetup(P::Save));
        if theme=="light" {assert_eq!(app.sheets.open_sheet().unwrap().plot.as_ref().unwrap().settings.profile.as_deref(),Some("profile-0"));}
        assert!(app.sheets.error().is_none());
        assert!(
            app.sheets
                .open_sheet()
                .unwrap()
                .plot
                .as_ref()
                .unwrap()
                .settings.profile_snapshot
                .is_some()
        );
    }
}

#[test]
#[ignore="GPU plot ink controls and real PDF, run alone"]
fn sheet20_plot_ink_screens() {
    use kentos_sheet_ui::plot_profiles::{PlotMessage as P, ColorMode};
    use kentos_sheet::{plot_profile::TableKind, LineCap, LineJoin};
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = multipage_set_app(theme, text);
        sheet(&mut app, Sheet::Tab(3));
        for m in [
            P::Open,
            P::New(TableKind::Named),
            P::Name("Mimari · gri ve ince".into()),
            P::ColorMode(ColorMode::Gray),
            P::Screening(70.),
            P::Weight(true),
            P::WeightMm(0.35),
            P::Dash(true),
            P::DashText("2 1".into()),
            P::Opacity(true),
            P::Alpha(80.),
            P::Cap(Some(LineCap::Round)),
            P::Join(Some(LineJoin::Miter)),
        ] {
            sheet(&mut app, Sheet::PlotProfiles(m));
        }
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet20-ink-top-{theme}-{text}.png")))
            .unwrap();
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Scroll(iced::Point::new(w / 2., h / 2.), -25.),
        );
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet20-ink-bottom-{theme}-{text}.png")))
            .unwrap();
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Click(iced::Point::new(
                if text == 18 { 930. } else { 990. },
                h - 32.,
            )),
        );
        assert!(
            app.sheets.book().sheets[2]
                .plot
                .as_ref()
                .is_some_and(|p| p.settings.profile_snapshot.is_some())
        );
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        if theme == "light" {
            let painter = app.sheet_painter();
            std::fs::write(
                out.join("sheet20-ink.pdf"),
                app.sheets.pdf(&painter, app.sheet_pdf_host()).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn desktop_pdf_refuses_an_impossible_model_curve_without_a_gray_raster_fallback() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/plot-document.json"
    ))
    .unwrap();
    let mut source = data["model"].clone();
    source["entities"][1]["r"] = serde_json::json!(1e10);
    let mut model =
        kentos_domain::Document::from_snapshot(serde_json::from_value(source).unwrap())
            .unwrap();
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets
        .push(serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap());
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let (mut app, _store) = app_with_store("sheet22-quality");
    let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    sheet(&mut app, Sheet::Tab(1));
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let painter = app.sheet_painter();
    let error = app
        .sheets
        .pdf_and_findings(&painter, app.sheet_pdf_host())
        .unwrap_err();
    assert!(error.contains("plot_curve_too_complex"), "{error}");
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}

/// plan/cad-advanced.md CAD-ADV-11: “Ekran temasını değiştirmek çizimin atanmış veya basılacak rengini
/// değiştirmesin.” The same set printed under each of the four themes is the same PDF, byte for byte, and the drawing
/// (its colours with it) is the same; the screen's palette never reaches the paper (kentos-plot's paper palette).
#[test]
fn the_screen_theme_changes_neither_the_drawing_nor_the_printed_pdf() {
    use kentos_ui::theme::Mode;
    let mut app = multipage_set_app("light", 13);
    sheet(&mut app, Sheet::Tab(3));
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let printed = |app: &App| app.sheet_native_pdf().expect("prints").bytes;
    let light = printed(&app);
    for mode in [Mode::Dark, Mode::Night, Mode::HighContrast, Mode::Light] {
        app.set_theme(mode);
        assert_eq!(app.mode, mode);
        assert!(printed(&app) == light, "{mode:?}: the PDF is the light theme's");
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before, "{mode:?}: the drawing is untouched");
    }
}

#[test]
fn sheet23_real_save_and_print_use_the_same_native_pdf() {
    let mut app = multipage_set_app("light", 13);
    sheet(&mut app, Sheet::Tab(3));
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let doc = app.sheet_output_snapshot().unwrap();
    let options = app.sheets.output_pdf_options();
    let mut captured=kentos_plot::provenance::Capture::new(&doc,&kentos_plot::book::current_user(),&kentos_plot::book::today(),None).unwrap();
    captured.dpi_override(Some(app.sheets.output_dpi())).unwrap();
    let expected=kentos_plot::book::book_pdf_captured(&captured,&options.sheets.clone(),Some(options),None).unwrap();
    let path = scratch("sheet23-real-save").join("native.pdf");
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(path.clone()));
    assert_eq!(std::fs::read(path).unwrap(), expected.bytes);
    let _ = app.sheet_print();
    let path = crate::sheet_pdf::OPENED
        .with(|p| p.borrow().last().cloned())
        .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), expected.bytes);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}

#[test]
#[ignore = "GPU print preview controls, run alone"]
fn sheet23_print_preview_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 720.),
    ] {
        let mut app = multipage_set_app(theme, text);
        sheet(&mut app, Sheet::Tab(3));
        sheet(&mut app, Sheet::Print);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet23-options-{theme}-{text}.png")))
            .unwrap();
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Click(iced::Point::new(
                if text == 18 { 910. } else { 975. },
                if text == 18 { 570. } else { 626. },
            )),
        );
        snap.settle(&mut app, App::view, &mut update);
        let shot = snap.render(app.view(), &app.theme());
        let white = shot
            .rgba
            .chunks_exact(4)
            .filter(|p| p[0] > 248 && p[1] > 248 && p[2] > 248)
            .count();
        shot.save(out.join(format!("sheet23-preview-{theme}-{text}.png")))
            .unwrap();
        assert!(
            white > 20_000,
            "A printed white page must be visible: {white}"
        );
        assert!(app.sheets.error().is_none());
    }
}

#[test]
#[ignore = "GPU calibration sheet and common native output, run alone"]
fn sheet24_calibration_screens() {
    use kentos_sheet::template::*;
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/calibration.json"
    ))
    .unwrap();
    let t = system_template("sys:baski-kalibrasyonu").unwrap();
    let ids = InstanceIds {
        sheet: "cal".into(),
        master: None,
        items: t.sheet.items.iter().map(|i| i.id.clone()).collect(),
        master_items: Vec::new(),
    };
    let inst = instantiate(
        t,
        &ids,
        &InstanceOptions {
            center: Some(kentos_sheet::GroundPoint { x: 0., y: 0. }),
            scale: Some(200),
            ..Default::default()
        },
    )
    .unwrap();
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets.push(inst.sheet);
    let mut model = kentos_domain::Document::from_snapshot(
        serde_json::from_value(data["cases"][6]["model"].clone()).unwrap(),
    )
    .unwrap();
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [("light", 13, 1440., 900.), ("dark", 18, 1100., 720.)] {
        let (mut app, _) = app_with_store("sheet24-calibration");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let task = app.update(Message::Opened(Some(Ok(Box::new(
            Document::from_v2(model.to_snapshot_v2(), None).unwrap(),
        )))));
        drive(&mut app, task);
        sheet(&mut app, Sheet::Tab(1));
        sheet(&mut app, Sheet::ZoomPage);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet24-calibration-{theme}-{text}.png")))
            .unwrap();
        sheet(&mut app, Sheet::Print);
        sheet(
            &mut app,
            Sheet::Pdf(kentos_sheet_ui::message::PdfMessage::Preview),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet24-preview-{theme}-{text}.png")))
            .unwrap();
        if theme == "light" {
            app.sheet_export_to(
                kentos_sheet_ui::ExportKind::Pdf,
                Some(out.join("sheet24-calibration.pdf")),
            );
        }
    }
}

pub(super) fn wait_sheet_publication(app: &mut App) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while app.sheet_publish_running() {
        let _ = app.sheet_publish_event(crate::exchange::sheet_publish::Event::Tick);
        assert!(std::time::Instant::now() < deadline, "publication did not finish");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn sheet25_desktop_plans_then_publishes_the_selected_order_without_editing_the_source() {
    use crate::exchange::{Window, sheet_publish::Event as E};
    let mut app = multipage_set_app("light", 13);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let path = scratch("sheet25-desktop").join("delivery");
    let _ = app.sheet_publish_open();
    for e in [E::Path(path.display().to_string()), E::All(false), E::Selected(2,true), E::Selected(5,true), E::Move(5,-1), E::Move(4,-1), E::Move(3,-1), E::Separate(true), E::Plan] {
        let _ = app.sheet_publish_event(e);
    }
    wait_sheet_publication(&mut app);
    let Some(Window::SheetPublish(s)) = &app.exchange else { panic!("publisher closed") };
    assert!(s.report.as_ref().is_some_and(|r| r.ready), "{} {:?}", s.status, s.report);
    assert!(!path.exists(), "a plan must never write files");
    let report = s.report.clone().unwrap();
    let _ = app.sheet_publish_event(E::Publish);
    wait_sheet_publication(&mut app);
    let Some(Window::SheetPublish(s)) = &app.exchange else { panic!("publisher closed") };
    assert!(path.join("yayin.json").exists(), "{}", s.status);
    assert_eq!(s.report.as_ref().unwrap().sheets, report.sheets);
    assert_eq!(report.files.len(), 2);
    let expected_words = ["DETAIL", "PLAN"];
    for (f, word) in report.files.iter().zip(expected_words) {
        let output = std::process::Command::new("pdftotext").arg(path.join(&f.name)).arg("-").output().unwrap();
        assert!(output.status.success());
        let words = String::from_utf8(output.stdout).unwrap();
        assert!(words.contains(word), "{} {words}", f.name);
    }
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
}

#[test]
#[ignore = "GPU publication controls and asynchronous job, run alone"]
fn sheet25_publish_screens() {
    use crate::exchange::{Window, sheet_publish::Event as E};
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text) in [("light",13),("dark",13),("dark",18)] {
        let mut app = multipage_set_app(theme, text);
        let path = scratch(&format!("sheet25-{theme}-{text}")).join("delivery");
        let _ = app.sheet_publish_open();
        for e in [E::Path(path.display().to_string()), E::All(true), E::Separate(true)] { let _ = app.sheet_publish_event(e); }
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(1100.,720.)).unwrap();
        let mut update = |app: &mut App, m: Message| { let _ = app.update(m); };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme()).save(out.join(format!("sheet25-options-{theme}-{text}.png"))).unwrap();
        snap.input(&mut app, App::view, &mut update,
            kentos_ui::snapshot::Input::Click(iced::Point::new(if text == 18 { 862. } else { 758. }, if text == 18 { 612. } else { 589. })));
        assert!(app.sheet_publish_running(), "Denetle button must start the job");
        wait_sheet_publication(&mut app);
        let Some(Window::SheetPublish(s)) = &app.exchange else { panic!("publisher closed") };
        assert!(s.report.as_ref().is_some_and(|r| r.ready), "{}", s.status);
        assert!(!path.exists());
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme()).save(out.join(format!("sheet25-ready-{theme}-{text}.png"))).unwrap();
    }
}

#[test]
fn sheet26_desktop_drawing_set_fields_and_live_index_have_one_paper_undo() {
    use kentos_sheet_ui::drawing_set::{SetMessage as E,Field};
    let mut app=multipage_set_app("light",13);let before=kept(&app).unwrap();
    for event in [E::Open,E::Edit(Field::Number,"A-001".into()),E::Edit(Field::Title,"Merkez plan".into()),E::Edit(Field::Revision,"B".into()),E::Edit(Field::Status,"Kontrol".into()),E::Edit(Field::Subset,"Mimari / Planlar".into()),E::InsertIndex]{sheet(&mut app,Sheet::DrawingSet(event));}
    let after=kept(&app).unwrap();let info=after.sheets[0].drawing_info.as_ref().unwrap();
    assert_eq!(info.number,"A-001");assert_eq!(info.subset,["Mimari","Planlar"]);
    assert!(after.sheets[0].items.iter().any(|it|matches!(&it.kind,kentos_sheet::ItemKind::Table(t) if matches!(t.source,kentos_sheet::TableSource::DrawingSet(_)))));
    history_key(&mut app,true);assert_eq!(kept(&app).unwrap(),before);
    history_key(&mut app,false);assert_eq!(kept(&app).unwrap(),after);
}

#[test]
#[ignore = "GPU drawing set editor and live output, run alone"]
fn sheet26_drawing_set_screens() {
    use kentos_sheet_ui::drawing_set::{SetMessage as E,Field};
    crate::drawing_fonts::load();kentos_ui::theme::motion::set_reduced(true);
    let out=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme,text) in [("light",13),("dark",13),("dark",18)] {
        let mut app=multipage_set_app(theme,text);
        let _=app.run("sheet.drawingSet");
        for event in [E::Edit(Field::Number,"A-001".into()),E::Edit(Field::Title,"Zemin kat planı".into()),E::Edit(Field::Revision,"R02".into()),E::Edit(Field::Status,"Kontrol".into()),E::Edit(Field::Subset,"Mimari / Kat planları".into()),E::Select(2),E::Edit(Field::Number,"A-101".into()),E::Edit(Field::Title,"Kat planı".into()),E::Select(0)]{sheet(&mut app,Sheet::DrawingSet(event));}
        let mut snap=kentos_ui::snapshot::Snapshot::new(iced::Size::new(1100.,720.)).unwrap();
        let mut update=|app:&mut App,m:Message|{let _=app.update(m);};
        snap.settle(&mut app,App::view,&mut update);
        use kentos_ui::snapshot::Input;
        snap.input(&mut app,App::view,&mut update,Input::Click(iced::Point::new(900.,if text==18{320.}else{309.})));
        snap.input(&mut app,App::view,&mut update,Input::Key(iced::keyboard::key::Named::End));
        snap.input(&mut app,App::view,&mut update,Input::Type("-K".into()));
        snap.render(app.view(),&app.theme()).save(out.join(format!("sheet26-set-{theme}-{text}.png"))).unwrap();
        snap.input(&mut app,App::view,&mut update,Input::Click(iced::Point::new(if text==18{850.}else{800.},if text==18{614.}else{580.})));
        assert_eq!(kept(&app).unwrap().sheets[0].drawing_info.as_ref().unwrap().revision,"R02-K","the real field and insert button must work");
        assert!(app.sheets.error().is_none(),"{:?}",app.sheets.error());
        snap.settle(&mut app,App::view,&mut update);
        snap.render(app.view(),&app.theme()).save(out.join(format!("sheet26-index-{theme}-{text}.png"))).unwrap();
        if theme=="light" {app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf,Some(out.join("sheet26-index.pdf")));}
    }
}

#[test]
fn sheet26_drawing_set_command_opens_from_model_without_a_hidden_dialog() {
    let mut app=multipage_set_app("light",13);sheet(&mut app,Sheet::Model);assert!(!app.sheets.is_active());
    let task=app.run("sheet.drawingSet");drive(&mut app,task);assert!(app.sheets.is_active());
    sheet(&mut app,Sheet::DrawingSet(kentos_sheet_ui::drawing_set::SetMessage::Edit(kentos_sheet_ui::drawing_set::Field::Number,"A-001".into())));
    sheet(&mut app,Sheet::DrawingSet(kentos_sheet_ui::drawing_set::SetMessage::Save));
    assert_eq!(kept(&app).unwrap().sheets[0].drawing_info.as_ref().unwrap().number,"A-001");
}

#[test]
fn sheet_package_desktop_publishes_the_complete_source_without_editing_model_or_paper(){
    use crate::exchange::{Window,sheet_publish::Event as E};
    let mut app=multipage_set_app("light",13);let before=app.document.as_ref().unwrap().model.to_snapshot_v2();let path=scratch("sheet-package").join("delivery");let _=app.sheet_publish_open();
    for event in [E::Path(path.display().to_string()),E::All(true),E::Resources(true),E::Plan]{let _=app.sheet_publish_event(event);}
    wait_sheet_publication(&mut app);let Some(Window::SheetPublish(s))=&app.exchange else{panic!("window")};let plan=s.report.as_ref().unwrap();assert!(plan.ready,"{:?}",plan.issues);assert_eq!(plan.resources.as_ref().unwrap().files.iter().filter(|f|matches!(f.kind,kentos_commands::SheetResourceKind::Font)).count(),22);assert!(!path.exists());
    let _=app.sheet_publish_event(E::Publish);wait_sheet_publication(&mut app);assert!(path.join("kaynaklar/cizim.kcad").is_file());let after=app.document.as_ref().unwrap().model.to_snapshot_v2();assert_eq!(after.entities,before.entities);assert_eq!(after.layers,before.layers);assert_eq!(after.settings,before.settings);assert_eq!(after.sheets,before.sheets);
}

#[test]
#[ignore="GPU complete delivery publication controls, run alone"]
fn sheet_package_publish_screens(){
    use crate::exchange::{Window,sheet_publish::Event as E};
    crate::drawing_fonts::load();kentos_ui::theme::motion::set_reduced(true);let out=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme,text) in [("light",13),("dark",13),("dark",18)]{
        let mut app=multipage_set_app(theme,text);let path=scratch(&format!("sheet-package-{theme}-{text}")).join("delivery");let _=app.sheet_publish_open();
        for event in [E::Path(path.display().to_string()),E::All(true),E::Separate(true)]{let _=app.sheet_publish_event(event);}
        let mut snap=kentos_ui::snapshot::Snapshot::new(iced::Size::new(1100.,720.)).unwrap();let mut update=|app:&mut App,m:Message|{let _=app.update(m);};snap.settle(&mut app,App::view,&mut update);
        snap.render(app.view(),&app.theme()).save(out.join(format!("sheet27-before-{theme}-{text}.png"))).unwrap();
        snap.input(&mut app,App::view,&mut update,kentos_ui::snapshot::Input::Click(iced::Point::new(if text==18{104.}else{238.},421.)));
        assert!(matches!(&app.exchange,Some(Window::SheetPublish(s)) if s.input.resources),"real resource switch must enable the source package");
        snap.render(app.view(),&app.theme()).save(out.join(format!("sheet27-package-options-{theme}-{text}.png"))).unwrap();
        snap.input(&mut app,App::view,&mut update,kentos_ui::snapshot::Input::Click(iced::Point::new(if text==18{862.}else{758.},if text==18{612.}else{589.})));assert!(app.sheet_publish_running(),"real Denetle button must start the job");wait_sheet_publication(&mut app);let Some(Window::SheetPublish(s))=&app.exchange else{panic!("window")};assert!(s.report.as_ref().unwrap().ready,"{}",s.status);assert!(s.report.as_ref().unwrap().resources.is_some());snap.settle(&mut app,App::view,&mut update);snap.render(app.view(),&app.theme()).save(out.join(format!("sheet27-package-ready-{theme}-{text}.png"))).unwrap();
    }
}

pub(super) fn sheet_preflight_app(theme: &str, text: u32) -> App {
    let mut app = multipage_set_app(theme, text);
    let image:kentos_commands::Entity=serde_json::from_value(serde_json::json!({"kind":"image","id":20,"layerId":"cizim","attrs":{},"file":"/missing-preflight/source.png","p":{"x":0,"y":0},"width":1,"height":1,"rotation":0})).unwrap();
    app.document.as_mut().unwrap().model.add(image).unwrap();app
}

#[test]
fn sheet_preflight_desktop_uses_the_native_query_and_cannot_publish_its_report(){
    use crate::exchange::{Window,sheet_publish::Event as E};
    let mut app=sheet_preflight_app("light",13);let before=app.document.as_ref().unwrap().model.to_snapshot_v2();let task=app.update(Message::Sheet(kentos_sheet_ui::Message::NativePreflight));drive(&mut app,task);wait_sheet_publication(&mut app);
    let Some(Window::SheetPublish(s))=&app.exchange else{panic!("preflight window")};assert!(s.check_only);let report=s.report.as_ref().unwrap();assert!(!report.ready);assert!(report.issues.iter().any(|i|i.code=="missing_reference"&&i.error));assert!(report.files.is_empty());assert_eq!(before.entities,app.document.as_ref().unwrap().model.to_snapshot_v2().entities);
    let _=app.sheet_publish_event(E::Publish);assert!(!app.sheet_publish_running());
}

#[test]
#[ignore="GPU native preflight report, run alone"]
fn sheet_preflight_native_screens(){
    use crate::exchange::Window;
    crate::drawing_fonts::load();kentos_ui::theme::motion::set_reduced(true);let out=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for(theme,text)in[("light",13),("dark",13),("dark",18)]{
        let mut app=sheet_preflight_app(theme,text);let task=app.run("file.preflight.sheets");drive(&mut app,task);wait_sheet_publication(&mut app);let Some(Window::SheetPublish(s))=&app.exchange else{panic!("preflight")};assert!(s.check_only);assert!(s.report.as_ref().is_some_and(|r|r.issues.iter().any(|i|i.code=="missing_reference")));
        let mut snap=kentos_ui::snapshot::Snapshot::new(iced::Size::new(1100.,720.)).unwrap();let mut update=|app:&mut App,m:Message|{let _=app.update(m);};snap.settle(&mut app,App::view,&mut update);
        snap.input(&mut app,App::view,&mut update,kentos_ui::snapshot::Input::Click(iced::Point::new(if text==18{960.}else{840.},if text==18{685.}else{675.})));assert!(app.sheet_publish_running(),"real Denetle button starts the read-only native check");wait_sheet_publication(&mut app);snap.settle(&mut app,App::view,&mut update);
        snap.render(app.view(),&app.theme()).save(out.join(format!("sheet28-preflight-{theme}-{text}.png"))).unwrap();
    }
}

pub(super) fn sheet_snapshot_changed_while_planning(theme: &str, text: u32) -> App {
    use crate::exchange::{Window, sheet_publish::Event as E};
    let mut app = multipage_set_app(theme, text);
    let path = scratch(&format!("sheet29-{theme}-{text}")).join("delivery");
    let _ = app.sheet_publish_open();
    for event in [E::Path(path.display().to_string()), E::All(true), E::Plan] {
        let _ = app.sheet_publish_event(event);
    }
    assert!(app.sheet_publish_running());
    let doc=&mut app.document.as_mut().unwrap().model;
    let(mut book,pictures)=kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    book.sheets[0].name="Sonraki düzenleme".into();
    doc.set_sheets(kentos_sheet::project::project_sheets(&book,|h|pictures.iter().find(|(sha,_)|sha==h).map(|(_,b)|b.clone())).unwrap()).unwrap();
    let after=doc.to_snapshot_v2();wait_sheet_publication(&mut app);
    let Some(Window::SheetPublish(s))=&app.exchange else{panic!("publisher")};
    assert!(s.status.contains("Çizim bu sırada değişti"),"{}",s.status);
    assert!(s.report.as_ref().unwrap().ready);assert!(!path.exists());
    let _=app.sheet_publish_event(E::Publish);assert!(!app.sheet_publish_running());assert!(!path.exists());
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),after);app
}

#[test]
fn sheet_snapshot_desktop_marks_old_results_and_disables_stale_publication(){let _=sheet_snapshot_changed_while_planning("light",13);}

#[test]
#[ignore="GPU source revision report, run alone"]
fn sheet_snapshot_source_report_screens(){
    crate::drawing_fonts::load();kentos_ui::theme::motion::set_reduced(true);
    let out=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for(theme,text)in[("light",13),("dark",13),("dark",18)]{
        let mut app=sheet_snapshot_changed_while_planning(theme,text);
        let mut snap=kentos_ui::snapshot::Snapshot::new(iced::Size::new(1100.,720.)).unwrap();
        let mut update=|app:&mut App,m:Message|{let _=app.update(m);};
        snap.settle(&mut app,App::view,&mut update);
        snap.render(app.view(),&app.theme()).save(out.join(format!("sheet29-source-{theme}-{text}.png"))).unwrap();
    }
}

#[test]
fn a_host_pdf_refusal_reopens_output_options_and_never_creates_the_chosen_file() {
    use kentos_sheet_ui::{DemoMaps, ExportKind, message::PdfMessage, painter::lend};
    let mut app = sheet_preflight_app("light", 13);
    sheet(&mut app, Sheet::Print);
    let options = app.sheets.output_pdf_options();
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    // The native file chooser already returned; no OS dialog is needed here.
    app.sheets.update(Sheet::Pdf(PdfMessage::Save));
    let path = scratch("sheet-host-refusal").join("must-not-exist.pdf");
    assert!(!path.exists());
    app.sheet_export_to(ExportKind::Pdf, Some(path.clone()));
    assert!(!path.exists());
    assert!(app.sheets.error().is_some_and(|e| e.starts_with("Dışa aktarılamadı:")));
    assert!(app.sheets.window(lend(&DemoMaps)).is_some());
    assert_eq!(app.sheets.output_pdf_options(), options);
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
}

/// CAD-ADV-16 (docs/adr/1090): the general 2D detail set from `cad.sheets.details`, opened in the desktop.
pub(super) fn detail_set_app(theme: &str, text: u32) -> App {
    let fx: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.sheets.details.json"
    ))
    .unwrap();
    let mut model = kentos_domain::Document::from_snapshot(
        serde_json::from_value(fx["setup"].clone()).unwrap(),
    )
    .unwrap();
    let r = |m: f64| serde_json::json!({"paper": 1.0, "model": m});
    let table = model.uid(kentos_domain::Slot(4)).unwrap().to_string();
    for details in [
        serde_json::json!([
            {"name": "Bulon", "bounds": [-0.1, -0.05, 0.2, 0.1], "ratio": r(1.)},
            {"name": "Levha", "bounds": [-0.5, -0.25, 1.0, 0.5], "ratio": r(5.)},
            {"name": "Bağlantı", "bounds": [-2.0, -1.0, 4.0, 2.0], "ratio": r(20.)},
            {"name": "Genel", "bounds": [-15.0, -7.5, 15.0, 7.5], "ratio": r(100.)}
        ]),
        serde_json::json!([
            {"name": "Ölçülü", "bounds": [-0.05, -0.3, 0.45, 0.1], "ratio": r(5.), "annotation": {"type": "fixed", "scale": 5.0}},
            {"name": "Ölçüsüz", "bounds": [-0.05, -0.3, 0.45, 0.1], "ratio": r(5.), "layerOverrides": [{"layer": "olcu", "frozen": true}]},
            {"name": "Poz listesi 1", "table": {"table": table, "part": 0}, "ratio": r(20.)},
            {"name": "Poz listesi 2", "table": {"table": table, "part": 1}, "ratio": r(20.)}
        ]),
    ] {
        let answer = kentos_native_application::call::call(&mut model, None, envelope(kentos_commands::CAD_SHEETS_DETAILS, serde_json::json!({ "options": {"values": [{"name": "kurum", "value": "KentOS"}, {"name": "birim", "value": "Proje"}, {"name": "referans", "value": "ADR 1090"}, {"name": "onaylayan", "value": "Denetçi"}, {"name": "cizim_no", "value": "D-1"}]}, "details": details })));
        assert!(matches!(answer.result, kentos_commands::CommandResult::Completed { .. }), "{answer:?}");
    }
    let (mut app, _store) = app_with_store("adv16-details");
    app.settings.choose(&[
        ("appearance.theme", serde_json::json!(theme)),
        ("appearance.textSize", serde_json::json!(text)),
    ]);
    app.apply_settings();
    let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    app
}

pub(super) fn envelope(command: &str, input: serde_json::Value) -> kentos_commands::CallEnvelope {
    kentos_commands::CallEnvelope {
        command: command.into(),
        version: Some(1),
        op: kentos_commands::CallOp::Execute,
        request_id: None,
        expected: None,
        relax_constraints: false,
        input,
    }
}

#[test]
fn a_detail_set_is_published_and_its_delivery_read_back_as_current_from_the_window() {
    use crate::exchange::{Window, sheet_publish::Event as E};
    let mut app = detail_set_app("light", 13);
    let path = scratch("adv16-publish").join("teslim");
    let _ = app.sheet_publish_open();
    for e in [E::Path(path.display().to_string()), E::All(true), E::Separate(true), E::Plan] {
        let _ = app.sheet_publish_event(e);
    }
    wait_sheet_publication(&mut app);
    let Some(Window::SheetPublish(s)) = &app.exchange else { panic!("window") };
    let plan = s.report.as_ref().unwrap();
    assert!(plan.ready, "{:?}", plan.issues);
    // Four detail sheets, then the twin 1:5 sheet and the table's two parts, which run on to further sheets.
    assert_eq!(plan.scope.len(), 7);
    assert_eq!(plan.pages.len(), 7);
    let _ = app.sheet_publish_event(E::Publish);
    wait_sheet_publication(&mut app);
    assert!(path.join("yayin.json").is_file());
    let _ = app.sheet_publish_event(E::Status);
    wait_sheet_publication(&mut app);
    let Some(Window::SheetPublish(s)) = &app.exchange else { panic!("window") };
    assert!(s.folder.as_ref().is_some_and(|f| f.stale.is_empty() && !f.sheets.is_empty()), "{}", s.status);
}

#[test]
#[ignore = "GPU detail set, publication scope, page audit, staleness and resume, run alone"]
fn adv16_detail_set_screens() {
    use crate::exchange::{Window, sheet_publish::Event as E};
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    for (theme, text) in [("light", 13), ("dark", 13), ("dark", 18)] {
        let mut app = detail_set_app(theme, text);
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        // The 1:5 sheet and the sheet of the two representations with the table's parts.
        for (tab, name) in [(2, "scale-5"), (5, "pair"), (6, "table-part")] {
            sheet(&mut app, Sheet::Tab(tab));
            let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(1440., 900.)).unwrap();
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme()).save(out.join(format!("adv16-sheet-{name}-{theme}-{text}.png"))).unwrap();
        }
        // Publication: the scope and page audit, then the delivery's state after a change, then a stopped run.
        let path = scratch(&format!("adv16-shots-{theme}-{text}")).join("teslim");
        let _ = app.sheet_publish_open();
        for e in [E::Path(path.display().to_string()), E::All(true), E::Separate(true), E::Plan] {
            let _ = app.sheet_publish_event(e);
        }
        wait_sheet_publication(&mut app);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(1100., 720.)).unwrap();
        snap.settle(&mut app, App::view, &mut update);
        snap.input(&mut app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(iced::Point::new(550., 420.), -40.));
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme()).save(out.join(format!("adv16-publish-scope-{theme}-{text}.png"))).unwrap();
        let _ = app.sheet_publish_event(E::Publish);
        wait_sheet_publication(&mut app);
        {
            let model = &mut app.document.as_mut().unwrap().model;
            let line = model.uid(kentos_domain::Slot(3)).unwrap().to_string();
            let answer = kentos_native_application::call::call(
                model,
                None,
                envelope("cad.entities.transform", serde_json::json!({"uids": [line], "transform": {"kind": "move", "dx": 0.01, "dy": 0.0}})),
            );
            assert!(matches!(answer.result, kentos_commands::CommandResult::Completed { .. }), "{answer:?}");
        }
        let _ = app.sheet_publish_event(E::Status);
        wait_sheet_publication(&mut app);
        let Some(Window::SheetPublish(s)) = &app.exchange else { panic!("window") };
        assert!(s.folder.as_ref().is_some_and(|f| !f.stale.is_empty()), "{}", s.status);
        snap.settle(&mut app, App::view, &mut update);
        snap.input(&mut app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(iced::Point::new(550., 420.), -40.));
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme()).save(out.join(format!("adv16-publish-stale-{theme}-{text}.png"))).unwrap();
        // A stopped run: stopped once its journal holds a finished page.
        let other = scratch(&format!("adv16-stop-{theme}-{text}")).join("teslim");
        let journal = other.parent().unwrap().join(".teslim.kentos-devam/devam.json");
        let _ = app.sheet_publish_event(E::Path(other.display().to_string()));
        let _ = app.sheet_publish_event(E::Plan);
        wait_sheet_publication(&mut app);
        let _ = app.sheet_publish_event(E::Publish);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while app.sheet_publish_running() {
            if journal.exists() {
                let _ = app.sheet_publish_event(E::Stop);
            }
            let _ = app.sheet_publish_event(E::Tick);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let _ = app.sheet_publish_event(E::Status);
        wait_sheet_publication(&mut app);
        let Some(Window::SheetPublish(s)) = &app.exchange else { panic!("window") };
        assert!(s.folder.as_ref().and_then(|f| f.stopped.as_ref()).is_some(), "{}", s.status);
        snap.settle(&mut app, App::view, &mut update);
        snap.input(&mut app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(iced::Point::new(550., 420.), -40.));
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme()).save(out.join(format!("adv16-publish-stopped-{theme}-{text}.png"))).unwrap();
        let _ = app.sheet_publish_event(E::Resume);
        wait_sheet_publication(&mut app);
        assert!(other.join("yayin.json").is_file());
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme()).save(out.join(format!("adv16-publish-resumed-{theme}-{text}.png"))).unwrap();
    }
}
