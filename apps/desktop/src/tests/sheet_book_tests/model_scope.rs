//! A sheet's model scope (sheet steps 13–16): the drawing's own tools and history inside a map, named
//! views, annotation layers and the sources' identity.

use super::*;

pub(super) fn model_scope_app(theme: &str, text: u32) -> App {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.sheets.fit.json"
    ))
    .unwrap();
    let mut snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(fixture["setup"].clone()).unwrap();
    snapshot.entities.clear();
    let mut model = kentos_domain::Document::from_snapshot(snapshot).unwrap();
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-navigation.json"
    ))
    .unwrap();
    let mut page: kentos_sheet::Sheet =
        serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap();
    page.name = "A-101 · Görünümden model düzenleme".into();
    let kentos_sheet::ItemKind::Map(map) = &mut page.items[0].kind else {
        panic!()
    };
    let kentos_sheet::MapView::Fixed(f) = &mut map.view else {
        panic!()
    };
    f.center = Some(kentos_sheet::GroundPoint { x: 2.5, y: 1.5 });
    f.rotation = 0;
    page.items[0].frame = kentos_sheet::units::RectUm::new(40000, 40000, 310000, 190000);
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets.push(page);
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let (mut app, _store) = app_with_store("sheet13-model");
    app.settings.choose(&[
        ("appearance.theme", serde_json::json!(theme)),
        ("appearance.textSize", serde_json::json!(text)),
    ]);
    app.apply_settings();
    let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    sheet(&mut app, Sheet::Tab(1));
    app
}

pub(super) fn model_click(app: &mut App, world: [f64; 2], release: bool) {
    let at = app.sheets.active_viewport().unwrap().paper_point(world);
    let stage = if release {
        kentos_sheet_ui::StageEvent::Release {
            at,
            shift: false,
            ctrl: false,
            alt: false,
        }
    } else {
        kentos_sheet_ui::StageEvent::Press {
            at,
            px: iced::Point::ORIGIN,
            size: iced::Size::new(1000., 700.),
            shift: false,
            ctrl: false,
            alt: false,
        }
    };
    sheet(app, Sheet::Stage(stage));
}

#[test]
fn model_scope_uses_real_drawing_tools_and_model_history_then_restores_camera() {
    let mut app = model_scope_app("light", 13);
    let camera = app.viewport.camera;
    let before = app.sheets.book().clone();
    sheet(&mut app, Sheet::EnterModel("viewport".into()));
    assert!(app.sheets.model_scope().is_some());
    let _ = app.run("tool.line");
    model_click(&mut app, [0., 0.], false);
    model_click(&mut app, [0., 0.], true);
    model_click(&mut app, [5., 0.], false);
    model_click(&mut app, [5., 0.], true);
    let doc = app.document.as_ref().unwrap();
    assert_eq!(doc.model.entities().count(), 1);
    let e = doc.model.entities().next().unwrap();
    let kentos_commands::Entity::Line(l) = e else {
        panic!("{e:?}")
    };
    assert!((l.a.x).abs() < 1e-6);
    assert!((l.b.x - 5.).abs() < 1e-6);
    assert_eq!(app.sheets.book(), &before);
    app.cancel();
    app.undo();
    assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 0);
    sheet(&mut app, Sheet::ReturnPaper);
    assert_eq!(app.viewport.camera, camera);
    assert!(app.sheets.is_active());
}

#[test]
#[ignore="GPU images, run alone"]
fn sheet13_model_scope_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = model_scope_app(theme, text);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::EnterModel("viewport".into()));
        let _ = app.run("tool.line");
        model_click(&mut app, [0., 0.], false);
        model_click(&mut app, [0., 0.], true);
        model_click(&mut app, [5., 0.], false);
        model_click(&mut app, [5., 0.], true);
        let at = app.sheets.active_viewport().unwrap().paper_point([5., 3.]);
        sheet(
            &mut app,
            Sheet::Stage(kentos_sheet_ui::StageEvent::Move {
                at,
                size: iced::Size::new(1000., 700.),
                shift: false,
                ctrl: false,
                alt: false,
            }),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet13-model-scope-{theme}-{text}.png")))
            .unwrap();
        assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 1);
        assert!(app.session.preview(&app.format()).is_some());
        app.cancel();
        app.session.exit();
        model_click(&mut app, [2.5, 0.], false);
        model_click(&mut app, [2.5, 0.], true);
        assert_eq!(
            app.selection.len(),
            1,
            "a press through the viewport chooses the original model object"
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet13-model-properties-{theme}-{text}.png")))
            .unwrap();
        if text == 18 {
            snap.input(
                &mut app,
                App::view,
                &mut update,
                kentos_ui::snapshot::Input::Scroll(iced::Point::new(w - 100., h - 170.), -7.),
            );
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme())
                .save(out.join("sheet13-model-properties-scrolled.png"))
                .unwrap();
            let task = app.update(Message::Run("tool.text"));
            drive(&mut app, task);
            model_click(&mut app, [1., 1.5], false);
            assert!(app.text_field.is_some());
            let task = app.update(Message::TextField(crate::text_field::Event::Input(
                "Model yazısı".into(),
            )));
            drive(&mut app, task);
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme())
                .save(out.join("sheet13-model-text-field.png"))
                .unwrap();
            let task = app.update(Message::TextField(crate::text_field::Event::Keep));
            drive(&mut app, task);
            assert!(app.document.as_ref().unwrap().model.entities().any(
                |e| matches!(e,kentos_commands::Entity::Text(t) if t.text=="Model yazısı")
            ));
        }
    }
}

#[test]
#[ignore="GPU images and independent PDF, run alone"]
fn sheet14_named_view_screens() {
    use kentos_sheet::{ItemKind, MapView, viewport::Source};
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-links.json"
    ))
    .unwrap();
    let mut page: kentos_sheet::Sheet =
        serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap();
    page.name = "A-101 · Canlı görünüm ve kopya".into();
    let mut items = Vec::new();
    let captured = kentos_commands::NamedView {
        id: "model-plan".into(),
        name: "Plan ilk kayıt".into(),
        center: kentos_commands::Vec2 { x: 2.5, y: 1.5 },
        width: 14.,
        height: 6.,
        twist: 0.,
        recipe: None,
    };
    let current = kentos_commands::NamedView {
        name: "Plan güncel".into(),
        width: 7.,
        height: 3.,
        ..captured.clone()
    };
    for (id, left, source) in [
        (
            "live",
            35000,
            Source::ViewLink {
                view: captured.clone(),
            },
        ),
        (
            "copy",
            235000,
            Source::ViewSnapshot {
                view: captured.clone(),
            },
        ),
    ] {
        let mut item = page.items[0].clone();
        item.id = id.into();
        item.name = if id == "live" {
            "Canlı · Plan güncel"
        } else {
            "Kopya · İlk kayıt"
        }
        .into();
        item.frame = kentos_sheet::units::RectUm::new(left, 60000, 150000, 120000);
        let ItemKind::Map(map) = &mut item.kind else {
            panic!()
        };
        map.source = source;
        let MapView::Fixed(view) = &mut map.view else {
            panic!()
        };
        view.center = Some(kentos_sheet::GroundPoint { x: 2.5, y: 1.5 });
        view.scale = 100;
        view.ratio = None;
        view.rotation = 0;
        items.push(item);
        let mut kind = kentos_sheet::kinds::default_kind("text").unwrap();
        let ItemKind::Text(t) = &mut kind else {
            panic!()
        };
        t.content = if id == "live" {
            "Canlı görünüm · [% @olcek %]"
        } else {
            "Kopya görünüm · [% @olcek %]"
        }
        .into();
        t.map = Some(id.into());
        t.style.size = 3000;
        items.push(kentos_sheet::Item::new(
            &format!("title-{id}"),
            &format!("Başlık {id}"),
            kentos_sheet::units::RectUm::new(left, 190000, 150000, 15000),
            kind,
        ));
    }
    page.items = items;
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets.push(page);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.sheets.fit.json"
    ))
    .unwrap();
    let mut snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(fixture["setup"].clone()).unwrap();
    snapshot.settings.named_views = vec![current];
    snapshot.entities.clear();
    snapshot.entities.push(serde_json::from_value(serde_json::json!({"kind":"polyline","id":1,"layerId":"cizim","attrs":{},"closed":true,"color":"#1747aa","pts":[{"x":0.,"y":0.},{"x":5.,"y":0.},{"x":5.,"y":3.},{"x":0.,"y":3.}]})).unwrap());
    let mut model = kentos_domain::Document::from_snapshot(snapshot).unwrap();
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let answer=kentos_native_application::registry::run(&mut kentos_native_application::ExecutionContext::new(&mut model),"cad.viewmarks.create",kentos_commands::CallOp::Execute,serde_json::json!({"layerId":"cizim","marks":[{"form":"elevation","points":[{"x":2.5,"y":1.5}],"mark":"42","size":2.5}]})).unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    let pdf = kentos_plot::book::book_pdf(&model, &[], "Test", "2026-10-22").unwrap();
    std::fs::write(out.join("sheet14-view-links.pdf"), pdf.bytes).unwrap();
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store("sheet14-links");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        sheet(&mut app, Sheet::Tab(1));
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::Select(vec!["live".into()]));
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet14-view-links-{theme}-{text}.png")))
            .unwrap();
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Scroll(
                iced::Point::new(w - 100., h - 140.),
                if text == 18 { -15. } else { -8. },
            ),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet14-view-source-{theme}-{text}.png")))
            .unwrap();
        let maps: Vec<_> = app
            .sheets
            .display_list()
            .unwrap()
            .prims
            .iter()
            .filter_map(|p| {
                if let kentos_sheet::display::Prim::Map(m) = p {
                    Some(m)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(maps.len(), 2);
        assert!((maps[0].view.scale - 50.).abs() < 1e-10);
        assert_eq!(maps[1].view.scale, 100.);
    }
}

#[test]
#[ignore = "GPU images and PDF, run alone"]
fn sheet15_annotation_layer_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-annotations.json"
    ))
    .unwrap();
    let cases = data["cases"].as_array().unwrap();
    let mut page: kentos_sheet::Sheet =
        serde_json::from_value(cases[12]["sheet"].clone()).unwrap();
    page.name = "A-101 · Katman ve açıklama temsilleri".into();
    page.items.clear();
    for (index, c) in cases[12..16].iter().enumerate() {
        let mut item: kentos_sheet::Item =
            serde_json::from_value(c["sheet"]["items"][0].clone()).unwrap();
        let patch: kentos_sheet::ops::Op =
            serde_json::from_value(c["operation"].clone()).unwrap();
        let mut one = kentos_sheet::SheetBook::default();
        let mut single = page.clone();
        single.items.clear();
        single.items.push(item);
        one.sheets.push(single);
        item = kentos_sheet::ops::apply(&one, &patch).unwrap().book.sheets[0]
            .items
            .remove(0);
        item.id = format!("v{index}");
        item.name = c["name"].as_str().unwrap().into();
        item.padding = 0;
        item.frame = kentos_sheet::units::RectUm::new(
            20000 + (index as i32 % 2) * 200000,
            30000 + (index as i32 / 2) * 125000,
            170000,
            85000,
        );
        if let kentos_sheet::ItemKind::Map(m) = &mut item.kind {
            m.label_band = 0;
        }
        let left = item.frame.left;
        let top = item.frame.top;
        page.items.push(item);
        let mut kind = kentos_sheet::kinds::default_kind("text").unwrap();
        if let kentos_sheet::ItemKind::Text(t) = &mut kind {
            t.content = c["name"].as_str().unwrap().into();
            t.style.size = 3000;
        }
        page.items.push(kentos_sheet::Item::new(
            &format!("title{index}"),
            &format!("Başlık {index}"),
            kentos_sheet::units::RectUm::new(left, top + 90000, 170000, 15000),
            kind,
        ));
    }
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets.push(page);
    let snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(cases[12]["model"].clone()).unwrap();
    let mut model = kentos_domain::Document::from_snapshot(snapshot).unwrap();
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        out.join("sheet15-annotations.pdf"),
        kentos_plot::book::book_pdf(&model, &[], "Test", "2026-10-22")
            .unwrap()
            .bytes,
    )
    .unwrap();
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store("sheet15");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        sheet(&mut app, Sheet::Tab(1));
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::Select(vec!["v1".into()]));
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet15-annotations-{theme}-{text}.png")))
            .unwrap();
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Scroll(iced::Point::new(w - 100., h - 140.), -12.),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet15-controls-{theme}-{text}.png")))
            .unwrap();
    }
}

pub(super) fn identity_scope_app(index: usize, theme: &str, text: u32) -> App {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-identities.json"
    ))
    .unwrap();
    let c = &data["cases"][index];
    let snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(c["model"].clone()).unwrap();
    let mut model = kentos_domain::Document::from_snapshot(snapshot).unwrap();
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut model),
        "cad.sheets.edit",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"ops":[{"op":"addSheet","sheet":c["sheet"]},c["operation"]]}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed");
    let (mut app, _store) = app_with_store("sheet16-identities");
    app.settings.choose(&[
        ("appearance.theme", serde_json::json!(theme)),
        ("appearance.textSize", serde_json::json!(text)),
    ]);
    app.apply_settings();
    let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    sheet(&mut app, Sheet::Tab(1));
    app
}

#[test]
fn sheet_model_scope_uses_visible_source_ids_and_new_text_takes_the_physical_paper_scale() {
    let mut app = identity_scope_app(8, "light", 13);
    sheet(&mut app, Sheet::EnterModel("viewport".into()));
    assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 5);
    let at = app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(kentos_domain::Slot(5))
        .unwrap();
    let kentos_commands::Entity::Point(p) = at else {
        panic!()
    };
    let p = [p.p.x, p.p.y];
    model_click(&mut app, p, false);
    model_click(&mut app, p, true);
    assert!(app.selection.contains(kentos_domain::Slot(5)));
    app.cancel();
    app.draft.snap = false;
    let task = app.update(Message::Run("tool.text"));
    drive(&mut app, task);
    model_click(&mut app, [-1., 0.], false);
    assert!(app.text_field.is_some());
    let task = app.update(Message::TextField(crate::text_field::Event::Input(
        "PAPER37".into(),
    )));
    drive(&mut app, task);
    let task = app.update(Message::TextField(crate::text_field::Event::Keep));
    drive(&mut app, task);
    let doc = app.document.as_ref().unwrap();
    let t = doc
        .model
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Text(t) if t.text == "PAPER37" => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(
        (t.height
            - 37.5
                * doc
                    .model
                    .settings()
                    .annotation_mm(kentos_commands::AnnotationKind::Text)
                / 1000.)
            .abs()
            < 1e-12
    );
    app.undo();
    assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 5);
    sheet(&mut app, Sheet::ReturnPaper);
    app.sync_sheet_model_index();
    assert!(
        app.spatial
            .store()
            .shown(app.spatial.store().get(4.).unwrap())
    );
}

#[test]
#[ignore="GPU images, run alone"]
fn sheet16_original_identity_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = identity_scope_app(12, theme, text);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::EnterModel("viewport".into()));
        let point = app
            .document
            .as_ref()
            .unwrap()
            .model
            .get(kentos_domain::Slot(5))
            .unwrap();
        let kentos_commands::Entity::Point(p) = point else {
            panic!()
        };
        let at = [p.p.x, p.p.y];
        model_click(&mut app, at, false);
        model_click(&mut app, at, true);
        snap.settle(&mut app, App::view, &mut update);
        assert!(app.selection.contains(kentos_domain::Slot(5)));
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet16-source-selection-{theme}-{text}.png")))
            .unwrap();
        assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 5);
    }
}

#[test]
fn an_active_viewport_index_cannot_hide_source_objects_in_another_map_or_export() {
    use kentos_sheet_ui::MapPainter;
    let mut app = identity_scope_app(12, "light", 13);
    let mut map = app
        .sheets
        .export_list()
        .unwrap()
        .prims
        .into_iter()
        .find_map(|p| match p {
            kentos_sheet::display::Prim::Map(m) => Some(m),
            _ => None,
        })
        .unwrap();
    map.item = "other-unclipped-source-view".into();
    map.clip_shape = kentos_sheet::clip::Clip::Frame;
    map.layer_overrides.clear();
    map.layers = Default::default();
    let content = |app: &App| {
        let request =
            kentos_sheet_ui::MapRequest::of(&map, iced::Size::new(1000., 600.), 0.003, true);
        app.sheet_painter()
            .vector(&request)
            .expect("source vector content")
    };
    let before = content(&app);
    sheet(&mut app, Sheet::EnterModel("viewport".into()));
    assert!(!app.spatial.store().view_shown(4.));
    assert_eq!(
        content(&app),
        before,
        "the model interaction scope must not become another viewport's data source"
    );
    assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 5);
}

#[test]
fn a_lost_active_source_returns_to_paper_instead_of_using_an_unscoped_model_camera() {
    let mut app=identity_scope_app(12,"light",13);let camera=app.viewport.camera;
    let view=kentos_commands::NamedView{id:"lost-view".into(),name:"Plan".into(),center:kentos_commands::Vec2{x:0.,y:0.},width:19.5,height:11.5,twist:0.,recipe:None};
    let mut ctx=app.sheets.context().clone();ctx.named_views=vec![view.clone()];app.sheets.set_context(ctx.clone());let mut book=app.sheets.book().clone();if let kentos_sheet::ItemKind::Map(m)=&mut book.sheets[0].items[0].kind {m.source=kentos_sheet::viewport::Source::ViewLink{view};}
    app.sheets.load(book);app.sheets.update(Sheet::Tab(1));app.sheets.update(Sheet::EnterModel("viewport".into()));app.sync_sheet_model_camera();assert!(app.spatial.has_sheet_view());
    ctx.named_views.clear();app.sheets.set_context(ctx);app.sync_sheet_model_camera();assert!(app.sheets.model_scope().is_none());assert!(!app.spatial.has_sheet_view());assert_eq!(app.viewport.camera,camera);
}
