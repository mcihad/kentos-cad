//! Tables bound on a thread of their own (docs/adr/0757): the same layer, objects, link and base as a bind at once,
//! one undo step; Durdur writes nothing; meanwhile the drawing takes no edit and Esc stops; the bar moves on and never
//! back.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::{Ask, Started, Table};
use crate::app::{App, Message};
use crate::exchange::postgis_tests::finish_binding;
use crate::exchange::tests::{count, said};
use crate::exchange::{Event, binding};
use crate::files_testing::{app_with_drawing, scratch};

/// QGIS's GeoPackage of the shared cases, copied: a bind keeps the file's SHA-256 in its link.
fn kadastro(name: &str) -> PathBuf {
    let dir = scratch(name);
    let file = dir.join("kadastro.gpkg");
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geopackage/v1/qgis.gpkg"),
        &file,
    )
    .expect("copied");
    file
}

/// The parcels table of `file`, at once (`rows` small) or on its thread (`rows` unknown).
fn ask(file: &std::path::Path, rows: Option<u64>) -> Ask {
    Ask {
        source: file.display().to_string(),
        shown: "kadastro.gpkg".into(),
        tables: vec![Table {
            name: "parsel".into(),
            geometry: true,
            rows,
        }],
        reproject: false,
        area: None,
        key: None,
        file: true,
        then: "Değişiklikleri Kaynağa yaz ile dosyaya yazın.".into(),
    }
}

fn frame(app: &mut App) {
    let _ = app.update(Message::Exchange(Box::new(Event::Binding(
        binding::Event::Frame,
    ))));
}

/// What a bind left in the drawing: the bound layer's objects (ids and contents), the link and the base, the step.
fn bound(app: &App) -> (Vec<String>, String, String, Option<String>) {
    let doc = app.document.as_ref().expect("a drawing");
    let link = doc.settings().sources.first().cloned().expect("a link");
    let mut objects: Vec<String> = doc
        .model
        .by_layer_with_uids(&link.layer)
        .map(|(u, e)| format!("{u} {}", serde_json::to_string(e).unwrap_or_default()))
        .collect();
    objects.sort();
    let base = doc
        .settings()
        .source_bases
        .iter()
        .find(|b| b.source == link.id)
        .map(|b| format!("{}|{}|{}|{:?}", b.revision, b.digest, b.rows, b.fields));
    let link = format!(
        "{} {} {} {} {}",
        link.table, link.layer, link.digest, link.revision, link.path
    );
    (objects, link, doc.model.undo_labels().join(" / "), base)
}

#[test]
fn a_table_bound_on_its_thread_leaves_what_a_bind_at_once_leaves() {
    let file = kadastro("bind-thread-same");
    let mut once = app_with_drawing();
    assert_eq!(once.start_binding(ask(&file, Some(1))), Started::Done);
    assert!(!once.binding_now());
    let mut apart = app_with_drawing();
    let before = count(&apart);
    let revision = apart.document.as_ref().map(|d| d.model.revision());
    assert_eq!(apart.start_binding(ask(&file, None)), Started::Going);
    assert!(apart.binding_now());
    // Nothing is written before the job is over.
    assert_eq!(count(&apart), before);
    assert_eq!(
        apart.document.as_ref().map(|d| d.model.revision()),
        revision
    );
    finish_binding(&mut apart);
    let (a, b) = (bound(&once), bound(&apart));
    assert!(!a.0.is_empty(), "the parcels came");
    assert_eq!(a, b, "the same objects, link, step and base");
    // The next drawn object comes after the parcels, and the view went to them (a job's patch kept the slot behind once).
    let next = |app: &App| app.document.as_ref().map(|d| d.model.next_slot());
    assert_eq!(next(&once), next(&apart));
    let seen = |app: &App| {
        let v = app.viewport.camera.visible_bounds();
        [v.min_x, v.min_y, v.max_x, v.max_y].map(|x| (x * 1e3).round())
    };
    assert_eq!(seen(&once), seen(&apart), "the view on the parcels");
    assert!(
        said(&apart, "“kadastro.gpkg” bağlandı"),
        "{:?}",
        apart.log.lines().last().map(|l| l.text.clone())
    );
    // One undo step takes the layer and its objects back, as after a bind at once (the link is a project setting,
    // outside the undo history, either way).
    let _ = once.update(Message::Run("edit.undo"));
    let _ = apart.update(Message::Run("edit.undo"));
    assert_eq!(count(&apart), before);
    assert_eq!(count(&once), before);
    let links = |app: &App| {
        app.document.as_ref().map(|d| {
            d.settings()
                .sources
                .iter()
                .map(|s| (s.table.clone(), s.layer.clone()))
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(links(&once), links(&apart));
}

#[test]
fn a_database_table_goes_on_its_thread_whatever_its_estimate_and_its_panel_waits() {
    let file = kadastro("bind-thread-estimate");
    let mut app = app_with_drawing();
    // A database's count is the catalogue's estimate: one row may be fifty thousand after a bulk load.
    let mut small = ask(&file, Some(1));
    small.file = false;
    assert_eq!(app.start_binding(small), Started::Going);
    assert!(app.binding_view().is_none(), "no panel flashes by");
    std::thread::sleep(binding::SHOWN_AFTER);
    assert!(
        app.binding_view().is_some(),
        "the panel of a bind that goes on"
    );
    finish_binding(&mut app);
    assert!(
        app.document
            .as_ref()
            .is_some_and(|d| d.settings().sources.len() == 1)
    );
}

#[test]
fn durdur_ends_a_bind_straight_away_and_writes_nothing() {
    let file = kadastro("bind-thread-stop");
    let mut app = app_with_drawing();
    let before = count(&app);
    let revision = app.document.as_ref().map(|d| d.model.revision());
    assert_eq!(app.start_binding(ask(&file, None)), Started::Going);
    let _ = app.update(Message::Exchange(Box::new(Event::Binding(
        binding::Event::Stop,
    ))));
    assert!(!app.binding_now(), "Durdur ends it at once");
    assert!(
        app.log.lines().any(|l| l
            .text
            .starts_with("“parsel” bağlanması durduruldu; bir şey yazılmadı.")),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    // Whatever the job still says is not heard.
    for _ in 0..20 {
        frame(&mut app);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(count(&app), before);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision);
    assert!(
        app.document
            .as_ref()
            .is_some_and(|d| d.settings().sources.is_empty())
    );
}

#[test]
fn while_a_table_binds_the_drawing_takes_no_edit_and_esc_stops() {
    let file = kadastro("bind-thread-gate");
    let mut app = app_with_drawing();
    let before = count(&app);
    assert_eq!(app.start_binding(ask(&file, None)), Started::Going);
    // An edit is not taken: its step would make the job's write refused.
    let _ = app.update(Message::Run("edit.selectAll"));
    let _ = app.update(Message::Run("edit.delete"));
    assert_eq!(count(&app), before, "no edit while binding");
    // A second bind waits for the first.
    assert!(matches!(
        app.start_binding(ask(&file, None)),
        Started::Failed(_)
    ));
    let esc = crate::keys::KeyPress {
        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
        physical: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        modifiers: iced::keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    };
    let _ = app.update(Message::Key(esc));
    assert!(!app.binding_now(), "Esc stops the bind");
    assert_eq!(count(&app), before);
}

#[test]
fn the_bar_moves_on_through_a_step_and_never_back() {
    let file = kadastro("bind-thread-bar");
    let mut app = app_with_drawing();
    assert_eq!(
        app.start_binding(ask(&file, Some(50_000))),
        Started::Going,
        "a table known to be large goes on its thread"
    );
    // Its bar followed frame by frame, the job's words aside.
    let b = app.binding.as_mut().expect("binding");
    let began = Instant::now();
    let mut last = 0.0;
    for k in 1..=30 {
        b.frame(began + Duration::from_millis(16 * k));
        let shown = b.shown();
        assert!(shown >= last, "never back: {shown} after {last}");
        last = shown;
    }
    assert!(last > 0.0, "the bar moved on before the job said anything");
    assert!(last < 0.3, "and not past its step: {last}");
    b.heard(0.55);
    for k in 31..=60 {
        b.frame(began + Duration::from_millis(16 * k));
        let shown = b.shown();
        assert!(shown >= last, "never back: {shown} after {last}");
        last = shown;
    }
    assert!(last > 0.3, "towards what the job said: {last}");
    assert!(last < 0.7, "not past the next step: {last}");
    app.stop_binding();
}

/// The bind's panel as it goes (reading the table, making its rows objects, the second of two tables with the first
/// drawn) and the drawing after; `.run/shots/baglaniyor-*` (docs/adr/0757). With `KENTOS_PG_URL` and `KENTOS_PG_BIND`
/// (and `KENTOS_PG_FIRST`, a table bound before it) the database's tables, else the shared GeoPackage's.
#[test]
#[ignore]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let db = std::env::var("KENTOS_PG_URL")
        .ok()
        .zip(std::env::var("KENTOS_PG_BIND").ok());
    if db.is_some() {
        kentos_postgis::register();
    }
    let file = kadastro("bind-screens");
    let source = |two: bool| -> Ask {
        let Some((url, table)) = &db else {
            let mut a = ask(&file, Some(50_000));
            if two {
                a.tables.insert(
                    0,
                    Table {
                        name: "yol".into(),
                        geometry: true,
                        rows: Some(2_500),
                    },
                );
            }
            return a;
        };
        let mut tables = vec![Table {
            name: table.clone(),
            geometry: true,
            rows: Some(50_000),
        }];
        if two {
            let first = std::env::var("KENTOS_PG_FIRST").unwrap_or_else(|_| "kbs.adalar".into());
            tables.insert(
                0,
                Table {
                    name: first,
                    geometry: false,
                    rows: Some(2_500),
                },
            );
        }
        Ask {
            source: url.clone(),
            shown: "Belediye".into(),
            tables,
            reproject: false,
            area: None,
            key: None,
            file: false,
            then: "Tabloların değişikliklerini Kaynağa yaz ile veritabanına yazın; view ve sorgu salt okunurdur.".into(),
        }
    };
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["1-okunuyor", "2-cevriliyor", "3-iki-tablo", "4-baglandi"] {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| name.ends_with(x)))
                {
                    continue;
                }
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let _ = app.update(Message::Run("view.zoomExtents"));
                assert_eq!(
                    app.start_binding(source(name == "3-iki-tablo")),
                    Started::Going
                );
                // Past the moment its panel shows.
                if let Some(b) = app.binding.as_mut() {
                    b.began = b
                        .began
                        .checked_sub(Duration::from_secs(1))
                        .unwrap_or(b.began);
                }
                match name {
                    "1-okunuyor" => {
                        let b = app.binding.as_mut().expect("binding");
                        b.shown = 0.12;
                    }
                    "2-cevriliyor" => {
                        let b = app.binding.as_mut().expect("binding");
                        (b.said, b.shown) = (0.3, 0.42);
                    }
                    "3-iki-tablo" => {
                        // The first table bound and drawn, the second going.
                        let began = Instant::now();
                        while app.binding.as_ref().is_some_and(|b| b.place < 2) {
                            assert!(
                                began.elapsed().as_secs() < 120,
                                "the first table did not end"
                            );
                            frame(&mut app);
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        let b = app.binding.as_mut().expect("the second table binds");
                        (b.said, b.shown) = (0.3, 0.37);
                    }
                    _ => finish_binding(&mut app),
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let shot = out.join(format!("baglaniyor-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&shot)
                    .expect("writes the picture");
                println!("{}", shot.display());
                app.stop_binding();
            }
        }
    }
}
