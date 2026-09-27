//! Model tasarımcısı on the desktop (docs/adr/0116): a model built from the
//! parts, saved to the library and opened again; typing kept as one undo
//! step; the unsaved question; a built-in model opened as its copy; a wire
//! let go on a step and its menu; a fixed value typed with the tool
//! window's own control, and a point shown on the drawing; the keys;
//! Modeli sil. The rules and words are `kentos_processing`'s, held to the
//! web's by fixtures/processing/v1/designer.json.

use iced::keyboard::key::{Code, NativeCode, Physical};
use iced::keyboard::{Key, Modifiers, key::Named};
use kentos_processing::ValueSource;
use kentos_processing::designer::{self as plan, texts};
use kentos_processing::model::Model;
use kentos_processing::model_edit::NodeRef;
use serde_json::json;

use super::{Asking, Designer, Event};
use crate::app::{App, Dialog, Message};
use crate::files_testing::last_said;
use crate::processing::Event as Field;
use crate::processing::tests::app_with_parcels;

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::ModelDesigner(e));
}

fn designer(app: &App) -> &Designer {
    app.processing
        .designer
        .as_deref()
        .expect("the designer is open")
}

fn chord(app: &mut App, letter: &str, code: Code) {
    let _ = app.update(Message::Key(crate::keys::KeyPress {
        key: Key::Character(letter.into()),
        physical: Physical::Code(code),
        modifiers: Modifiers::CTRL,
        text: None,
        repeat: false,
    }));
}

fn key(app: &mut App, named: Named) {
    let _ = app.update(Message::Key(crate::keys::KeyPress {
        key: Key::Named(named),
        physical: Physical::Unidentified(NativeCode::Unidentified),
        modifiers: Modifiers::default(),
        text: None,
        repeat: false,
    }));
}

fn source<'a>(app: &'a App, step: &str, param: &str) -> Option<&'a ValueSource> {
    designer(app).source(step, param)
}

/// Yeni model…, an input and a step that reads it: the parts a new model is built from.
fn new_chain(app: &mut App) -> String {
    let _ = app.update(Message::Run("processing.newModel"));
    event(app, Event::AddInput(0));
    event(
        app,
        Event::AddTool {
            tool: "points.numberVertices".into(),
            at: None,
        },
    );
    designer(app).draft.steps[0].id.clone()
}

#[test]
fn a_model_is_built_from_the_parts_saved_and_opened_again() {
    let mut app = app_with_parcels();
    let _ = app.update(Message::Run("processing.newModel"));
    assert_eq!(app.dialog, Some(Dialog::ModelDesigner));
    assert_eq!(designer(&app).title(), texts::title_of("Yeni model", false));
    event(&mut app, Event::AddInput(0));
    assert_eq!(
        designer(&app).selected,
        Some(NodeRef::Input("nesneler".into())),
        "the new input is selected"
    );
    event(
        &mut app,
        Event::AddTool {
            tool: "points.numberVertices".into(),
            at: None,
        },
    );
    let step = designer(&app).draft.steps[0].id.clone();
    assert_eq!(
        source(&app, &step, "input"),
        Some(&ValueSource::Input("nesneler".into())),
        "a step added beside the selected input reads it"
    );
    assert!(designer(&app).dirty());
    assert_eq!(designer(&app).title(), texts::title_of("Yeni model", true));
    event(&mut app, Event::Label("Köşe modeli".into()));
    event(&mut app, Event::Save);
    let id = designer(&app).draft.id.clone();
    assert!(!designer(&app).dirty(), "saved: nothing left to ask about");
    assert_eq!(last_said(&app), texts::save::saved("Köşe modeli", 0));
    assert!(
        app.processing.registry.model(&id).is_some(),
        "in the library"
    );
    assert_eq!(
        app.processing
            .memory
            .models()
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        [id.as_str()],
        "and kept for the next run (islemler.json)"
    );
    // The toolbox offers it as a command of its own.
    assert!(
        app.processing
            .registry
            .get(&format!("model:{id}"))
            .is_some()
    );
    event(&mut app, Event::Close);
    assert_eq!(app.dialog, None, "nothing unsaved: it closes at once");

    app.open_model_designer(Some(&id));
    let d = designer(&app);
    assert_eq!(d.draft.label, "Köşe modeli");
    assert!(
        d.is_saved(&app.processing.registry),
        "Modeli sil is offered"
    );
    assert!(!d.dirty());
}

#[test]
fn typing_is_one_undo_step_and_ctrl_z_takes_it_back() {
    let mut app = app_with_parcels();
    let _ = app.update(Message::Run("processing.newModel"));
    for text in ["K", "Ko", "Kod"] {
        event(&mut app, Event::Label(text.into()));
    }
    assert_eq!(designer(&app).draft.label, "Kod");
    chord(&mut app, "z", Code::KeyZ);
    assert_eq!(
        designer(&app).draft.label,
        "Yeni model",
        "three letters, one step"
    );
    chord(&mut app, "y", Code::KeyY);
    assert_eq!(designer(&app).draft.label, "Kod");
    // An edit of another kind is a step of its own.
    event(&mut app, Event::AddInput(1));
    event(&mut app, Event::Undo);
    assert!(designer(&app).draft.inputs.is_empty());
    assert_eq!(designer(&app).draft.label, "Kod");
}

#[test]
fn closing_with_changes_asks_and_vazgec_stays() {
    let mut app = app_with_parcels();
    let _ = app.update(Message::Run("processing.newModel"));
    event(&mut app, Event::AddInput(2));
    event(&mut app, Event::Close);
    assert_eq!(designer(&app).asking, Some(Asking::Unsaved));
    event(&mut app, Event::Answer(None));
    assert_eq!(app.dialog, Some(Dialog::ModelDesigner), "Vazgeç stays");
    assert_eq!(designer(&app).asking, None);
    // Esc asks the same.
    key(&mut app, Named::Escape);
    assert_eq!(designer(&app).asking, Some(Asking::Unsaved));
    event(&mut app, Event::Answer(Some(false)));
    assert_eq!(app.dialog, None, "Kaydetmeden kapat");
    assert!(app.processing.designer.is_none());
    assert!(app.processing.registry.user_models().is_empty());
}

#[test]
fn saving_from_the_question_keeps_the_model_and_closes() {
    let mut app = app_with_parcels();
    let _ = app.update(Message::Run("processing.newModel"));
    event(&mut app, Event::Label("   ".into()));
    event(&mut app, Event::Close);
    event(&mut app, Event::Answer(Some(true)));
    assert_eq!(app.dialog, None);
    let saved = app.processing.registry.user_models();
    assert_eq!(saved.len(), 1);
    assert_eq!(
        saved[0].label,
        texts::save::UNNAMED,
        "a blank name is Adsız model"
    );
}

#[test]
fn a_built_in_model_opens_as_its_copy() {
    let mut app = app_with_parcels();
    app.open_model_designer(Some("builtin.parcelSheet"));
    let d = designer(&app);
    assert_ne!(d.draft.id, "builtin.parcelSheet");
    assert_eq!(d.draft.label, "Parsel ölçü yazıları (kopya)");
    assert!(d.builtin_copy);
    assert!(
        !d.is_saved(&app.processing.registry),
        "no Modeli sil for a copy"
    );
    assert!(!d.dirty(), "the copy is what it compares with");
    let id = d.draft.id.clone();
    event(&mut app, Event::Save);
    assert!(app.processing.registry.model(&id).is_some());
    assert!(
        app.processing
            .registry
            .model("builtin.parcelSheet")
            .is_some(),
        "the built-in one stays as it was"
    );
}

#[test]
fn a_wire_let_go_on_a_step_asks_what_it_feeds() {
    let mut app = app_with_parcels();
    new_chain(&mut app);
    // A second step with nothing selected: not connected.
    event(&mut app, Event::Select(None));
    event(
        &mut app,
        Event::AddTool {
            tool: "annotation.edgeLengths".into(),
            at: Some((600.0, 300.0)),
        },
    );
    let second = designer(&app).draft.steps[1].id.clone();
    assert_eq!(source(&app, &second, "input"), None);
    let from = NodeRef::Input("nesneler".into());
    event(
        &mut app,
        Event::Wire {
            from: from.clone(),
            to: second.clone(),
            at: iced::Point::new(300.0, 200.0),
        },
    );
    assert!(
        designer(&app).wire_menu.is_some(),
        "the menu opens where it was let go"
    );
    let registry = app.processing.registry.clone();
    let menu = plan::connect_choices(&designer(&app).draft, &from, &second, &|id| {
        registry.tool(id)
    })
    .expect("the step is there");
    let choice = menu
        .items
        .iter()
        .find(|c| c.param == "input")
        .expect("the objects can feed the step's input");
    event(
        &mut app,
        Event::Connect {
            step: second.clone(),
            param: choice.param.clone(),
            src: choice.src.clone(),
        },
    );
    assert_eq!(
        source(&app, &second, "input"),
        Some(&ValueSource::Input("nesneler".into()))
    );
    assert!(designer(&app).wire_menu.is_none());
    assert_eq!(designer(&app).selected, Some(NodeRef::Step(second)));
}

#[test]
fn a_fixed_value_is_typed_with_the_tool_window_s_own_control() {
    let mut app = app_with_parcels();
    let step = new_chain(&mut app);
    event(
        &mut app,
        Event::Source {
            param: "prefix".into(),
            choice: super::update::SourceChoice::Fixed,
        },
    );
    assert_eq!(
        source(&app, &step, "prefix"),
        Some(&ValueSource::Value(json!("P"))),
        "the tool's default, now fixed"
    );
    for text in ["K", "K-"] {
        event(
            &mut app,
            Event::Field(Field::Text("prefix".into(), text.into())),
        );
    }
    assert_eq!(
        source(&app, &step, "prefix"),
        Some(&ValueSource::Value(json!("K-")))
    );
    event(&mut app, Event::Undo);
    assert_eq!(
        source(&app, &step, "prefix"),
        Some(&ValueSource::Value(json!("P"))),
        "the typing was one step"
    );
    event(&mut app, Event::Undo);
    assert_eq!(
        source(&app, &step, "prefix"),
        None,
        "back to the tool's default"
    );
}

#[test]
fn a_start_point_is_shown_on_the_drawing_and_the_designer_comes_back() {
    let mut app = app_with_parcels();
    let step = new_chain(&mut app);
    event(
        &mut app,
        Event::Source {
            param: "start".into(),
            choice: super::update::SourceChoice::Fixed,
        },
    );
    event(&mut app, Event::Field(Field::PickChoice("start".into())));
    assert_eq!(app.dialog, None, "the designer steps aside");
    assert!(app.processing.designer.is_none());
    assert_eq!(
        app.session.prompt().text(),
        "Başlangıç noktası: haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]"
    );
    let _ = app.submit_line("487010,4420010");
    assert_eq!(app.dialog, Some(Dialog::ModelDesigner));
    assert_eq!(
        source(&app, &step, "startPoint"),
        Some(&ValueSource::Value(
            json!({ "x": 487010.0, "y": 4420010.0 })
        ))
    );
    assert_eq!(
        source(&app, &step, "start"),
        Some(&ValueSource::Value(json!("point"))),
        "and the choice that reads it"
    );
    assert_eq!(designer(&app).selected, Some(NodeRef::Step(step)));
}

#[test]
fn the_keys_save_and_delete_the_selected_box() {
    let mut app = app_with_parcels();
    let step = new_chain(&mut app);
    assert_eq!(designer(&app).selected, Some(NodeRef::Step(step)));
    key(&mut app, Named::Delete);
    assert!(
        designer(&app).draft.steps.is_empty(),
        "Delete removes the selected step"
    );
    assert_eq!(designer(&app).selected, None);
    chord(&mut app, "s", Code::KeyS);
    let id = designer(&app).draft.id.clone();
    assert!(app.processing.registry.model(&id).is_some(), "Ctrl+S saves");
}

#[test]
fn modeli_sil_asks_then_takes_the_model_out_of_the_library() {
    let mut app = app_with_parcels();
    new_chain(&mut app);
    event(&mut app, Event::Save);
    let id = designer(&app).draft.id.clone();
    event(&mut app, Event::Delete);
    assert_eq!(designer(&app).asking, Some(Asking::Remove));
    event(&mut app, Event::Answer(None));
    assert!(
        app.processing.registry.model(&id).is_some(),
        "Vazgeç keeps it"
    );
    event(&mut app, Event::Delete);
    event(&mut app, Event::Answer(Some(true)));
    assert_eq!(app.dialog, None);
    assert!(app.processing.registry.model(&id).is_none());
    assert!(app.processing.memory.models().is_empty());
    assert_eq!(last_said(&app), texts::remove::done("Yeni model"));
}

#[test]
fn a_number_input_keeps_what_is_typed_on_the_way() {
    let mut app = app_with_parcels();
    let _ = app.update(Message::Run("processing.newModel"));
    event(&mut app, Event::AddInput(1));
    let name = match &designer(&app).selected {
        Some(NodeRef::Input(n)) => n.clone(),
        other => panic!("the number input is selected, not {other:?}"),
    };
    for text in ["1", "1.", "1.5"] {
        event(
            &mut app,
            Event::Input(
                name.clone(),
                super::update::InputField::Number("default", text.into()),
            ),
        );
        if text == "1." {
            assert_eq!(
                designer(&app)
                    .typed
                    .get(&format!("{name}.default"))
                    .map(String::as_str),
                Some("1.")
            );
        }
    }
    let input = designer(&app)
        .draft
        .inputs
        .iter()
        .find(|i| i.name() == name)
        .expect("there");
    assert_eq!(input.0.get("default"), Some(&json!(1.5)));
    // En az emptied: taken out.
    event(
        &mut app,
        Event::Input(
            name.clone(),
            super::update::InputField::Number("min", "2".into()),
        ),
    );
    event(
        &mut app,
        Event::Input(
            name.clone(),
            super::update::InputField::Number("min", "".into()),
        ),
    );
    let input = designer(&app)
        .draft
        .inputs
        .iter()
        .find(|i| i.name() == name)
        .expect("there");
    assert_eq!(input.0.get("min"), None);
}

/// The web's picture of a model with two problems: a start point asked for
/// and not given, and a step reading an input that is gone.
fn problem_model() -> Model {
    Model::from_text(
        &json!({
        "id": "m-shot-problems",
        "label": "Sorunlu model",
        "category": "points",
        "description": "Resim için: iki sorunlu adım.",
        "inputs": [{ "type": "features", "name": "parseller", "label": "Parseller", "kinds": ["polygon"], "default": { "scope": "selection" } }],
        "steps": [
            { "id": "numara", "tool": "points.numberVertices", "values": { "input": { "kind": "input", "name": "parseller" }, "start": { "kind": "value", "value": "point" } }, "position": { "x": 330, "y": 40 } },
            { "id": "kenar", "tool": "annotation.edgeLengths", "values": { "input": { "kind": "input", "name": "yok" } }, "position": { "x": 330, "y": 160 } }
        ],
        "outputs": [],
        "inputPositions": { "parseller": { "x": 40, "y": 40 } }
        })
        .to_string(),
    )
    .expect("the model reads")
}

/// Model tasarımcısı's pictures, the web's scenes (apps/web/scripts/e2e/shots.mjs
/// `modeldesigner`), in `.run/shots/model-*`:
///
/// ```text
/// cargo test -p kentos-desktop processing::designer::tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let scenes = [
        "yeni",
        "kopya",
        "girdi",
        "adim",
        "kaynak-menusu",
        "tel-menusu",
        "sorunlar",
        "sorunlu-adim",
        "yeni-adim",
        "zincir",
        "sayi-girdisi",
        "arama",
        "kaydedilmemis",
    ];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in scenes {
                if only
                    .as_ref()
                    .is_some_and(|o| !o.split(',').any(|s| s == scene))
                {
                    continue;
                }
                let mut app = app_with_parcels();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match scene {
                    "yeni" | "yeni-adim" | "zincir" | "sayi-girdisi" | "arama"
                    | "kaydedilmemis" => {
                        let _ = app.update(Message::Run("processing.newModel"));
                    }
                    "sorunlar" | "sorunlu-adim" => {
                        let model = problem_model();
                        app.processing
                            .registry
                            .save_model(model.clone())
                            .expect("a user's model");
                        app.open_model_designer(Some(&model.id));
                    }
                    _ => app.open_model_designer(Some("builtin.parcelSheet")),
                }
                snapshot.settle(&mut app, App::view, &mut update);
                match scene {
                    "girdi" => event(
                        &mut app,
                        Event::Select(Some(NodeRef::Input("parcels".into()))),
                    ),
                    "adim" => event(&mut app, Event::Select(Some(NodeRef::Step("area".into())))),
                    "kaynak-menusu" => event(
                        &mut app,
                        Event::Select(Some(NodeRef::Step("corners".into()))),
                    ),
                    "sorunlu-adim" => event(
                        &mut app,
                        Event::Select(Some(NodeRef::Step("numara".into()))),
                    ),
                    "yeni-adim" => event(
                        &mut app,
                        Event::AddTool {
                            tool: "attributes.calculate".into(),
                            at: None,
                        },
                    ),
                    "zincir" => {
                        event(&mut app, Event::AddInput(0));
                        for tool in ["points.numberVertices", "annotation.edgeLengths"] {
                            event(
                                &mut app,
                                Event::AddTool {
                                    tool: tool.into(),
                                    at: None,
                                },
                            );
                        }
                        event(&mut app, Event::Layout);
                    }
                    "sayi-girdisi" => event(&mut app, Event::AddInput(1)),
                    "arama" => event(&mut app, Event::Search("kenar".into())),
                    "kaydedilmemis" => {
                        event(&mut app, Event::AddInput(2));
                        event(&mut app, Event::Close);
                    }
                    "tel-menusu" => {
                        // Let go over Alan hesapla's box, a little right of its middle.
                        let d = designer(&app);
                        let step = d
                            .draft
                            .steps
                            .iter()
                            .find(|s| s.id == "area")
                            .expect("the step");
                        let (x, y) = step.position.unwrap_or_default();
                        let v = d.view;
                        let at = iced::Point::new(
                            ((x + plan::canvas::STEP_W * 0.6) * v.k + v.x) as f32,
                            ((y + plan::canvas::STEP_H * 0.5) * v.k + v.y) as f32,
                        );
                        event(
                            &mut app,
                            Event::Wire {
                                from: NodeRef::Input("prefix".into()),
                                to: "area".into(),
                                at,
                            },
                        );
                    }
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if scene == "kaynak-menusu" {
                    let face = designer(&app)
                        .draft
                        .steps
                        .iter()
                        .find(|s| s.id == "corners")
                        .map(|s| {
                            let registry = app.processing.registry.clone();
                            plan::source_text(&designer(&app).draft, s.source("input"), &|id| {
                                registry.tool(id)
                            })
                        })
                        .expect("the step");
                    let caption: &'static str = Box::leak(face.into_boxed_str());
                    let at = crate::files_testing::find_text(&mut snapshot, &app, caption)
                        .expect("the source list");
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
                let file = out.join(format!("model-{scene}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
