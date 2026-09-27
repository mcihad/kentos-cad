//! İşlemler on the desktop: the window over a drawing, a run and its undo
//! step, what shows when nothing is selected, the point shown on the
//! drawing, and a tool's alias on the command line. The drawing is the
//! shared cases' (fixtures/processing/v1/parcels.kcad), whose answers
//! `kentos-processing` holds to the web's.

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Slot;
use serde_json::json;

use super::{Event, RunStatus};
use crate::app::{App, Dialog, Message};
use crate::document::Document;
use crate::files_testing::last_said;

const PARCELS: &str = include_str!("../../../../fixtures/processing/v1/parcels.kcad");

/// The app with the parcels open.
pub(crate) fn app_with_parcels() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(PARCELS).expect("the parcels read");
    let doc = Document::new(snapshot, None).expect("the parcels open");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Processing(e));
}

fn status(app: &App) -> RunStatus {
    app.processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

#[test]
fn a_run_writes_one_undo_step_named_after_the_tool_and_the_window_stays() {
    let mut app = app_with_parcels();
    app.selection.set([Slot(1), Slot(2), Slot(7)]);
    let _ = app.update(Message::Run("processing.run.annotation.edgeLengths"));
    assert_eq!(app.dialog, Some(Dialog::Processing));
    let before = app.document.as_ref().map_or(0, |d| d.model.len());
    event(&mut app, Event::Run);
    let summary = "3 nesneye 8 kenar uzunluğu yazıldı; 1 ortak kenar bir kez yazıldı.";
    assert_eq!(
        status(&app),
        RunStatus::Ok {
            text: summary.into(),
            pick: (11..=18).map(Slot).collect(),
            selected: false,
            undo: true,
        }
    );
    assert_eq!(
        app.dialog,
        Some(Dialog::Processing),
        "the window stays open"
    );
    assert_eq!(
        last_said(&app),
        format!("Kenar uzunluklarını yaz: {summary}")
    );
    let doc = &app.document.as_ref().expect("open").model;
    assert_eq!(doc.len(), before + 8);
    assert!(doc.layers().get("islem-kenar-olculeri").is_some());
    // Geri al takes the whole run back, the new layer too.
    event(&mut app, Event::Undo);
    let doc = &app.document.as_ref().expect("open").model;
    assert_eq!(doc.len(), before);
    assert!(doc.layers().get("islem-kenar-olculeri").is_none());
    assert_eq!(status(&app), RunStatus::Idle);
}

#[test]
fn with_nothing_selected_the_problem_shows_under_the_field() {
    let mut app = app_with_parcels();
    let _ = app.update(Message::Run("processing.run.points.numberVertices"));
    let window = app.processing.dialog.as_ref().expect("open");
    assert!(
        window.issue_of("input").is_none(),
        "nothing shows before Çalıştır"
    );
    event(&mut app, Event::Run);
    let window = app.processing.dialog.as_ref().expect("still open");
    assert_eq!(
        window.issue_of("input").map(|i| i.message.as_str()),
        Some(
            "“Alanlar”: seçili nesneler arasında uygun nesne yok. Önce nesneleri seçin ya da kapsamı değiştirin."
        )
    );
    assert_eq!(
        window.warning().as_deref(),
        Some("Çalıştırmadan önce 1 alanı düzeltin.")
    );
    // A change clears the run's problem; a number out of range shows while typing.
    event(&mut app, Event::Number("length".into(), "31".into()));
    let window = app.processing.dialog.as_ref().expect("open");
    assert!(window.issue_of("input").is_none());
    assert_eq!(
        window.issue_of("length").map(|i| i.message.as_str()),
        Some("“Toplam uzunluk” en çok 30 olmalı.")
    );
    assert_eq!(
        window.preview().as_deref(),
        Some("Önizleme için alanları düzeltin.")
    );
}

#[test]
fn a_start_point_is_shown_on_the_drawing_and_the_window_comes_back_with_it() {
    let mut app = app_with_parcels();
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("processing.run.points.numberVertices"));
    event(&mut app, Event::Value("start".into(), json!("point")));
    event(&mut app, Event::Pick("startPoint".into()));
    assert_eq!(app.dialog, None, "the window steps aside");
    assert_eq!(
        app.session.prompt().text(),
        "Başlangıç noktası: haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]"
    );
    let _ = app.submit_line("487010,4420010");
    assert_eq!(app.dialog, Some(Dialog::Processing));
    let window = app.processing.dialog.as_ref().expect("back");
    assert_eq!(
        window.values.get("startPoint"),
        Some(&json!({ "x": 487010.0, "y": 4420010.0 }))
    );
    assert_eq!(
        window.values.get("start"),
        Some(&json!("point")),
        "as it was"
    );
}

fn key(app: &mut App, named: iced::keyboard::key::Named) {
    use iced::keyboard::key::{NativeCode, Physical};
    let _ = app.update(Message::Key(crate::keys::KeyPress {
        key: iced::keyboard::Key::Named(named),
        physical: Physical::Unidentified(NativeCode::Unidentified),
        modifiers: iced::keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    }));
}

#[test]
fn the_start_vertex_is_picked_on_the_drawing_from_beside_its_choice() {
    let mut app = app_with_parcels();
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("processing.run.points.numberVertices"));
    // Sahneden seç beside Başlangıç köşesi: the point, and the choice it gives.
    event(&mut app, Event::PickChoice("start".into()));
    assert_eq!(app.dialog, None, "the window steps aside");
    assert_eq!(
        app.session.prompt().text(),
        "Başlangıç noktası: haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]"
    );
    let _ = app.submit_line("487010,4420010");
    let window = app.processing.dialog.as_ref().expect("back");
    assert_eq!(window.values.get("start"), Some(&json!("point")));
    assert_eq!(
        window.values.get("startPoint"),
        Some(&json!({ "x": 487010.0, "y": 4420010.0 }))
    );
    // Esc: nothing changes.
    event(&mut app, Event::Value("start".into(), json!("north")));
    event(&mut app, Event::PickChoice("start".into()));
    key(&mut app, iced::keyboard::key::Named::Escape);
    let window = app.processing.dialog.as_ref().expect("back");
    assert_eq!(window.values.get("start"), Some(&json!("north")));
}

#[test]
fn input_objects_are_picked_on_the_drawing_of_the_fields_kinds() {
    use crate::selecting::tests::{click, objects};
    use iced::keyboard::key::Named;

    let mut app = objects();
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("processing.run.points.numberVertices"));
    event(&mut app, Event::PickObjects("input".into()));
    assert_eq!(app.dialog, None, "the window steps aside");
    assert!(app.selection.is_empty(), "a fresh pick");
    assert_eq!(
        app.session.prompt().text(),
        "Alanlar: nesneleri tıklayın ya da pencereyle seçin (0 seçili) [Bitti (Enter) / Vazgeç (Esc)]"
    );
    // A point is not an area: taken neither by a click nor by a box.
    click(&mut app, 20.0, -8.0);
    assert!(app.selection.is_empty());
    // The parcel inside.
    click(&mut app, -18.0, 9.0);
    assert_eq!(app.selection.ids(), &[Slot(4)]);
    assert!(app.session.prompt().text().contains("(1 seçili)"));
    key(&mut app, Named::Enter);
    let window = app.processing.dialog.as_ref().expect("back");
    assert_eq!(
        window.values.get("input"),
        Some(&json!({ "scope": "selection" }))
    );
    assert_eq!(app.selection.ids(), &[Slot(4)]);
    // Esc: the selection as it was.
    event(&mut app, Event::PickObjects("input".into()));
    click(&mut app, -18.0, 9.0);
    click(&mut app, -18.0, 9.0);
    key(&mut app, Named::Escape);
    assert!(app.processing.dialog.is_some());
    assert_eq!(app.selection.ids(), &[Slot(4)], "back as it was");
}

#[test]
fn an_alias_on_the_command_line_opens_the_tool_and_last_values_come_back() {
    let mut app = app_with_parcels();
    let _ = app.submit_line("kenaryaz");
    assert_eq!(app.dialog, Some(Dialog::Processing));
    let window = app.processing.dialog.as_ref().expect("open");
    assert_eq!(window.tool.id, "annotation.edgeLengths");
    // What ran last is what the window opens with next time.
    app.selection.set([Slot(6)]);
    event(&mut app, Event::Text("suffix".into(), " m".into()));
    event(&mut app, Event::Run);
    event(&mut app, Event::Close);
    let _ = app.update(Message::Run("map.edgeLengths"));
    let window = app.processing.dialog.as_ref().expect("open again");
    assert_eq!(window.values.get("suffix"), Some(&json!(" m")));
}

/// The processing window for the owner, on the parcels: Kenar uzunluklarını
/// yaz, the numbering with Gelişmiş ayarlar and a start point, Öznitelik
/// hesapla, İfadeyle seç, the Parsel ölçü yazıları model, a finished run
/// and a refused one. Not run by default:
/// `cargo test -p kentos-desktop processing::tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let views: [(&str, &'static str); 9] = [
        ("kenar", "processing.run.annotation.edgeLengths"),
        ("numara", "processing.run.points.numberVertices"),
        ("oznitelik", "processing.run.attributes.calculate"),
        ("ifade", "processing.run.selection.byExpression"),
        ("model", "processing.model.builtin.parcelSheet"),
        ("sonuc", "processing.run.points.numberVertices"),
        ("gecersiz", "processing.run.points.numberVertices"),
        // The expression field's menus, opened by a click: long ones scroll.
        ("ifade-islevler", "processing.run.selection.byExpression"),
        ("ifade-degiskenler", "processing.run.selection.byExpression"),
    ];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for (name, command) in views {
                let mut app = app_with_parcels();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                if name != "gecersiz" {
                    app.selection.set([Slot(1), Slot(2), Slot(7)]);
                }
                let _ = app.update(Message::Run(command));
                match name {
                    "numara" => {
                        event(&mut app, Event::Value("start".into(), json!("point")));
                        event(&mut app, Event::Advanced);
                    }
                    "sonuc" | "gecersiz" => event(&mut app, Event::Run),
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if let Some(menu) = name.strip_prefix("ifade-") {
                    let caption = if menu == "islevler" {
                        "İşlevler"
                    } else {
                        "Değişkenler"
                    };
                    let at = crate::files_testing::find_text(&mut snapshot, &app, caption)
                        .expect("the menu's button");
                    let center = at.center();
                    snapshot.step(
                        &mut app,
                        App::view,
                        &mut update,
                        &[
                            iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                                position: center,
                            }),
                            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                                iced::mouse::Button::Left,
                            )),
                            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                                iced::mouse::Button::Left,
                            )),
                        ],
                    );
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("islem-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

#[test]
fn the_toolbox_is_a_tab_beside_the_layers_with_search_and_this_session_s_runs() {
    use super::panel::{Event as PanelEvent, Tab};
    use crate::app::Panel;

    let mut app = app_with_parcels();
    let slot = app.docks.slot(Panel::Processing).expect("docked");
    assert_eq!(
        app.docks.slot(Panel::Layers),
        Some(slot),
        "the tab is beside Katmanlar"
    );
    let _ = app.update(Message::Run("processing.toolbox"));
    assert!(app.docks.is_shown(Panel::Processing));
    assert_eq!(app.processing.panel.tab, Tab::Tools);
    assert_eq!(app.processing_meta(), "4 araç");
    // Turkish letters folded: “kose” finds Köşe noktalarını numarala only.
    event(&mut app, Event::Panel(PanelEvent::Search("kose".into())));
    let hits: Vec<String> = app
        .processing
        .registry
        .search(&app.processing.panel.search)
        .iter()
        .map(|t| t.id.clone())
        .collect();
    assert_eq!(hits, ["points.numberVertices"]);
    // A run goes to Geçmiş; Yeniden aç brings its values back, n nesneyi seç what it made.
    app.selection.set([Slot(3)]);
    let _ = app.update(Message::Run("processing.run.points.numberVertices"));
    event(&mut app, Event::Value("prefix".into(), json!("K")));
    event(&mut app, Event::Run);
    event(&mut app, Event::Close);
    let _ = app.update(Message::Run("processing.history"));
    assert_eq!(app.processing.panel.tab, Tab::History);
    assert_eq!(app.processing_meta(), "1 kayıt");
    let seq = app.processing.runner.history()[0].seq;
    app.selection.clear();
    event(&mut app, Event::Panel(PanelEvent::SelectRun(seq)));
    assert_eq!(app.selection.len(), 4, "the four new corner points");
    event(&mut app, Event::Panel(PanelEvent::Reopen(seq)));
    let window = app.processing.dialog.as_ref().expect("open again");
    assert_eq!(window.values.get("prefix"), Some(&json!("K")));
}

/// The dock's İşlemler tab for the owner: the tools, a search and the
/// history with three runs. Not run by default:
/// `cargo test -p kentos-desktop processing::tests::panel_screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn panel_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    use super::panel::Event as PanelEvent;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["araclar", "arama", "gecmis"] {
                let mut app = app_with_parcels();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "gecmis" {
                    for (ids, command) in [
                        (
                            vec![Slot(1), Slot(2)],
                            "processing.run.points.numberVertices",
                        ),
                        (vec![Slot(6)], "processing.run.annotation.edgeLengths"),
                        (
                            vec![Slot(1), Slot(2)],
                            "processing.model.builtin.parcelSheet",
                        ),
                    ] {
                        app.selection.set(ids);
                        let _ = app.update(Message::Run(command));
                        event(&mut app, Event::Run);
                        event(&mut app, Event::Close);
                    }
                    let _ = app.update(Message::Run("processing.history"));
                } else {
                    let _ = app.update(Message::Run("processing.toolbox"));
                }
                if name == "arama" {
                    event(&mut app, Event::Panel(PanelEvent::Search("kose".into())));
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("islemler-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
