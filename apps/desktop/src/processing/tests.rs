//! İşlemler on the desktop: the window over a drawing, a run and its undo
//! step, what shows when nothing is selected, the point shown on the
//! drawing, and a tool's alias on the command line. The drawing is the
//! shared cases' (fixtures/processing/v1/parcels.kcad), whose answers
//! `kentos-processing` holds to the web's.

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Slot;
use serde_json::json;

use kentos_processing::{Status, Target};

use super::plan::{AUTO_HINT, Choice, LineKind, RUNNING_BACKGROUND, footer_of, targets_view};
use super::{Event, RunStatus};
use crate::app::{App, Dialog, Message};
use crate::document::Document;
use crate::files_testing::{drive, last_said};

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
            table: None,
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

/// Nerede çalışır: Arka planda. The run goes to another thread and the
/// window waits; driven to its end, it does what it does here, in one
/// undo step, recorded as run in the background.
#[test]
fn a_run_in_the_background_does_what_it_does_here_in_one_undo_step() {
    let mut app = app_with_parcels();
    app.selection.set([Slot(1), Slot(2), Slot(7)]);
    let _ = app.update(Message::Run("processing.run.annotation.edgeLengths"));
    event(&mut app, Event::Target("worker".into()));
    assert_eq!(
        app.processing.memory.target("annotation.edgeLengths"),
        "worker"
    );
    let before = app.document.as_ref().map_or(0, |d| d.model.len());
    let task = app.update(Message::Processing(Event::Run));
    assert!(
        matches!(
            status(&app),
            RunStatus::Running {
                background: true,
                ..
            }
        ),
        "{:?}",
        status(&app)
    );
    let window = app.processing.dialog.as_ref().expect("open");
    let buttons = footer_of(&window.status);
    assert_eq!((buttons.run, buttons.close), ("Çalışıyor…", "Durdur"));
    assert!(buttons.run_disabled && buttons.reset_disabled);
    assert_eq!(window.line().text, RUNNING_BACKGROUND);
    assert_eq!(
        app.document.as_ref().map_or(0, |d| d.model.len()),
        before,
        "nothing changes before the answer"
    );
    // Çalıştır again waits for it.
    event(&mut app, Event::Run);
    assert_eq!(app.processing.running.len(), 1);
    drive(&mut app, task);
    let summary = "3 nesneye 8 kenar uzunluğu yazıldı; 1 ortak kenar bir kez yazıldı.";
    assert_eq!(
        status(&app),
        RunStatus::Ok {
            text: summary.into(),
            pick: (11..=18).map(Slot).collect(),
            selected: false,
            undo: true,
            table: None,
        }
    );
    assert_eq!(
        last_said(&app),
        format!("Kenar uzunluklarını yaz: {summary}")
    );
    assert_eq!(
        app.processing.runner.history()[0].target,
        Some(Target::Worker)
    );
    assert!(app.processing.running.is_empty());
    let doc = &app.document.as_ref().expect("open").model;
    assert_eq!(doc.len(), before + 8);
    assert!(doc.layers().get("islem-kenar-olculeri").is_some());
    event(&mut app, Event::Undo);
    let doc = &app.document.as_ref().expect("open").model;
    assert_eq!(doc.len(), before);
    assert!(doc.layers().get("islem-kenar-olculeri").is_none());
}

/// Durdur ends a background run at once, as the web's ends its worker's
/// job: the drawing stays as it is, and the thread's late answer is not heard.
#[test]
fn durdur_ends_a_background_run_at_once_and_its_late_answer_is_dropped() {
    let mut app = app_with_parcels();
    app.selection.set([Slot(1), Slot(2), Slot(7)]);
    let _ = app.update(Message::Run("processing.run.annotation.edgeLengths"));
    event(&mut app, Event::Target("worker".into()));
    let before = app.document.as_ref().map_or(0, |d| d.model.len());
    let task = app.update(Message::Processing(Event::Run));
    event(&mut app, Event::Stop);
    let stopped = "İşlem iptal edildi; çizim değişmedi.";
    assert_eq!(status(&app), RunStatus::Error(stopped.into()));
    assert_eq!(last_said(&app), stopped);
    assert_eq!(app.dialog, Some(Dialog::Processing), "the window stays");
    drive(&mut app, task);
    assert_eq!(status(&app), RunStatus::Error(stopped.into()));
    assert_eq!(app.document.as_ref().map_or(0, |d| d.model.len()), before);
    let record = &app.processing.runner.history()[0];
    assert_eq!(
        (record.status, record.target),
        (Status::Canceled, Some(Target::Worker))
    );
}

/// Otomatik (the web's rule): inputs of 2 000 objects or more run in the
/// background, and the answer is the one a run here gives.
#[test]
fn otomatik_sends_two_thousand_objects_to_the_background_with_the_same_answer() {
    let (mut app, _) = App::boot(None);
    let drawing = crate::files_testing::drawing(2100);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing)))));
    let _ = app.update(Message::Run("processing.run.selection.byExpression"));
    // The points are on Çizim, a hidden layer: the layer's objects, not the shown ones.
    event(
        &mut app,
        Event::Value(
            "input".into(),
            json!({ "scope": "layer", "layerId": "cizim" }),
        ),
    );
    event(
        &mut app,
        Event::Text("condition".into(), "$x >= 486500".into()),
    );
    let window = app.processing.dialog.as_ref().expect("open");
    let (options, hint, choice) =
        targets_view(&window.targets(&app.processing.registry), Choice::Auto);
    assert_eq!(options[0].note.as_deref(), Some("şimdi: arka planda"));
    assert_eq!((hint, choice), (Some(AUTO_HINT), Some(Choice::Auto)));
    // Here first, for the answer.
    event(&mut app, Event::Target("client".into()));
    event(&mut app, Event::Run);
    let here = status(&app);
    assert!(
        matches!(&here, RunStatus::Ok { selected: true, .. }),
        "{here:?}"
    );
    let chosen = app.selection.ids().to_vec();
    assert!(chosen.len() >= 2100, "{}", chosen.len());
    app.selection.clear();
    event(&mut app, Event::Target("auto".into()));
    let task = app.update(Message::Processing(Event::Run));
    assert!(matches!(status(&app), RunStatus::Running { .. }));
    drive(&mut app, task);
    assert_eq!(status(&app), here);
    assert_eq!(app.selection.ids(), chosen.as_slice());
    assert_eq!(
        app.processing.runner.history()[0].target,
        Some(Target::Worker)
    );
}

/// Parsel ölçü yazıları on two parcels: `worker`, whole on another thread
/// (driven to its end); else here. `meanwhile` changes the drawing after
/// Çalıştır, before the answer.
fn parcel_sheet(worker: bool, meanwhile: impl FnOnce(&mut App)) -> App {
    let mut app = app_with_parcels();
    app.selection.set([Slot(1), Slot(2), Slot(7)]);
    let _ = app.update(Message::Run("processing.model.builtin.parcelSheet"));
    let place = if worker { "worker" } else { "client" };
    event(&mut app, Event::Target(place.into()));
    let task = app.update(Message::Processing(Event::Run));
    if worker {
        assert!(
            matches!(status(&app), RunStatus::Running { .. }),
            "{:?}",
            status(&app)
        );
        meanwhile(&mut app);
        drive(&mut app, task);
    }
    app
}

/// The drawing's objects and layers, to compare two runs.
fn drawn(app: &App) -> (Vec<kentos_contracts::Entity>, Vec<(String, String)>) {
    let doc = &app.document.as_ref().expect("open").model;
    let layers = doc
        .layers()
        .leaves()
        .into_iter()
        .map(|l| (l.id.clone(), l.name.clone()))
        .collect();
    (doc.entities().cloned().collect(), layers)
}

/// A model runs whole on another thread, over the drawing's copy, and its
/// steps are applied on the drawing as they went there: it ends as a run
/// here does, in one undo step.
#[test]
fn a_model_in_the_background_ends_as_it_does_here_in_one_undo_step() {
    let before = drawn(&app_with_parcels());
    let here = parcel_sheet(false, |_| {});
    assert!(
        matches!(status(&here), RunStatus::Ok { undo: true, .. }),
        "{:?}",
        status(&here)
    );
    let mut apart = parcel_sheet(true, |_| {});
    assert_eq!(status(&apart), status(&here));
    assert_eq!(drawn(&apart), drawn(&here));
    assert_eq!(last_said(&apart), last_said(&here));
    assert!(apart.processing.running.is_empty());
    event(&mut apart, Event::Undo);
    assert_eq!(
        drawn(&apart),
        before,
        "one undo step takes the whole model back"
    );
}

/// Durdur ends a model at once, in the model's words; its late answer is dropped.
#[test]
fn durdur_ends_a_model_at_once() {
    let before = drawn(&app_with_parcels());
    let app = parcel_sheet(true, |app| event(app, Event::Stop));
    let stopped = "Model durduruldu; çizim değişmedi.";
    assert_eq!(status(&app), RunStatus::Error(stopped.into()));
    assert_eq!(last_said(&app), stopped);
    assert_eq!(drawn(&app), before);
    let record = &app.processing.runner.history()[0];
    assert_eq!(
        (record.tool_id.as_str(), record.status),
        ("model:builtin.parcelSheet", Status::Canceled)
    );
}

/// The drawing changed while the model ran on its copy (another editor's
/// work): the copy's steps would not fit it, so the model runs again here,
/// on the drawing as it is, and says so.
#[test]
fn a_model_whose_drawing_changed_meanwhile_runs_again_on_it() {
    let change = |app: &mut App| {
        let doc = &mut app.document.as_mut().expect("open").model;
        doc.remove(&[Slot(7)]);
    };
    let apart = parcel_sheet(true, change);
    let mut here = app_with_parcels();
    change(&mut here);
    here.selection.set([Slot(1), Slot(2)]);
    let _ = here.update(Message::Run("processing.model.builtin.parcelSheet"));
    event(&mut here, Event::Target("client".into()));
    event(&mut here, Event::Run);
    assert!(
        apart
            .log
            .lines()
            .any(|l| l.text == super::CHANGED_MEANWHILE),
        "it says why it ran again"
    );
    assert_eq!(status(&apart), status(&here));
    assert_eq!(drawn(&apart), drawn(&here));
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
    let line = window.line();
    assert_eq!(
        (line.kind, line.text.as_str()),
        (LineKind::Warn, "Çalıştırmadan önce 1 alanı düzeltin.")
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
    let views: [(&str, &'static str); 12] = [
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
        // The output layer's list: the new layer, then the existing ones by their place.
        ("katman-listesi", "processing.run.annotation.edgeLengths"),
        // Running on another thread (Nerede çalışır: Arka planda), and stopped with Durdur.
        ("arka-plan", "processing.run.annotation.edgeLengths"),
        ("durduruldu", "processing.run.annotation.edgeLengths"),
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
                    "arka-plan" | "durduruldu" => {
                        event(&mut app, Event::Target("worker".into()));
                        event(&mut app, Event::Run);
                        let id = app.processing.runs;
                        let step = super::background::Reply::Progress(0.42, String::new());
                        event(&mut app, Event::Background(id, step));
                        if name == "durduruldu" {
                            event(&mut app, Event::Stop);
                        }
                    }
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let opens = match name {
                    "ifade-islevler" => Some("İşlevler"),
                    "ifade-degiskenler" => Some("Değişkenler"),
                    "katman-listesi" => Some("Kenar ölçüleri (yeni)"),
                    _ => None,
                };
                if let Some(caption) = opens {
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
    // The web's 23 (Ağ analizi's three among them, docs/adr/0209 §9) and the nine point cloud tools, the desktop's
    // own for now (docs/adr/0207 §7).
    assert_eq!(app.processing_meta(), "32 araç");
    // Turkish letters folded: “kose numara” finds Köşe noktalarını numarala only (as the web's test).
    event(
        &mut app,
        Event::Panel(PanelEvent::Search("kose numara".into())),
    );
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
