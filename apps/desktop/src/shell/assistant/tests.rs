//! Asistan (docs/adr/8056 §4) against a provider on this computer: no network, no key of anyone's, no file of the
//! user's (the agents stay in memory). A model that plans a closed area and executes the plan draws it, one undo step
//! said in the log as the assistant's; a write that takes something away waits for the user and, refused, is not run;
//! a refused key is said in the chat; Uygulama ayarları › Yapay zekâ keeps the agents and their default.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex, PoisonError};

use iced::futures::StreamExt as _;
use kentos_agent::{AgentProfile, KeySource, ProviderKind};
use kentos_ui::widget::ai::{AiEvent, Phase, PromptState, ToolPhase};
use serde_json::{Value, json};

use super::{AgentsEdit, Event, msg, needs_approval};
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive};
use crate::settings_view::Edit;

/// A provider speaking Chat Completions on 127.0.0.1: each request is answered as `answer` says for it (its place and
/// body); the bodies are kept.
struct Provider {
    url: String,
    bodies: Arc<Mutex<Vec<Value>>>,
}

impl Provider {
    fn start(answer: impl Fn(usize, &Value) -> (u16, String) + Send + 'static) -> Provider {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let url = format!("http://{}/v1", listener.local_addr().expect("its address"));
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&bodies);
        std::thread::spawn(move || {
            for (n, stream) in listener.incoming().enumerate() {
                let Ok(mut stream) = stream else { break };
                let mut reader = BufReader::new(stream.try_clone().expect("the stream"));
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim_end().is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':')
                        && name.eq_ignore_ascii_case("content-length")
                    {
                        length = value.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0; length];
                let _ = reader.read_exact(&mut body);
                let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                let (status, text) = answer(n, &body);
                kept.lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(body);
                let kind = if status == 200 {
                    "text/event-stream"
                } else {
                    "application/json"
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} Durum\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                    text.len()
                );
                let _ = stream.flush();
            }
        });
        Provider { url, bodies }
    }

    fn bodies(&self) -> Vec<Value> {
        self.bodies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

fn chunk(delta: Value, finish: Option<&str>) -> String {
    let chunk = json!({ "id": "t", "object": "chat.completion.chunk", "created": 1, "model": "deneme-modeli",
        "choices": [{ "index": 0, "delta": delta, "finish_reason": finish }] });
    format!("data: {chunk}\n\n")
}

/// An answer calling tool `name` (as the wire names it) with `input`, its arguments in two pieces.
fn calls(id: &str, name: &str, input: &Value) -> (u16, String) {
    let arguments = input.to_string();
    let mut half = arguments.len() / 2;
    while !arguments.is_char_boundary(half) {
        half -= 1;
    }
    let (first, second) = arguments.split_at(half);
    let open = json!({ "role": "assistant", "content": null, "tool_calls": [{ "index": 0, "id": id, "type": "function",
        "function": { "name": name, "arguments": "" } }] });
    let piece = |p: &str| json!({ "tool_calls": [{ "index": 0, "function": { "arguments": p } }] });
    let usage = json!({ "id": "t", "object": "chat.completion.chunk", "created": 1, "model": "deneme-modeli",
        "choices": [], "usage": { "prompt_tokens": 120, "completion_tokens": 30, "total_tokens": 150 } });
    let sse = [
        chunk(open, None),
        chunk(piece(first), None),
        chunk(piece(second), None),
        chunk(json!({}), Some("tool_calls")),
        format!("data: {usage}\n\n"),
        "data: [DONE]\n\n".to_owned(),
    ];
    (200, sse.concat())
}

/// An answer in words.
fn says(words: &str) -> (u16, String) {
    let sse = [
        chunk(json!({ "role": "assistant", "content": words }), None),
        chunk(json!({}), Some("stop")),
        "data: [DONE]\n\n".to_owned(),
    ];
    (200, sse.concat())
}

/// What the last tool result in a request said.
fn last_result(body: &Value) -> String {
    body["messages"]
        .as_array()
        .and_then(|m| m.iter().rev().find(|m| m["role"] == "tool"))
        .and_then(|m| m["content"].as_str())
        .unwrap_or_default()
        .to_owned()
}

/// The sample drawing, and the provider's agent the default with a key for this session only.
fn chatting(provider: &Provider) -> App {
    let mut app = app_with_drawing();
    let mut agent = AgentProfile::new(
        ProviderKind::OpenAiCompatible,
        "Deneme ajanı",
        "deneme-modeli",
    );
    agent.base_url = Some(provider.url.clone());
    agent.key = KeySource::Session;
    let store = &app.assistant.store;
    let agent = store.add(agent).expect("added");
    store.set_default(&agent.id).expect("the default");
    store.remember_session_key(&agent.id, "deneme-anahtari");
    app.assistant.refresh();
    app
}

fn send(app: &mut App, words: &str) -> (u64, iced::Task<Message>) {
    app.assistant.chat.prompt = PromptState::with_text(words);
    let task = app.update(msg(Event::Chat(AiEvent::Submit)));
    let message = app.assistant.chat.pending().expect("a turn under way");
    (message, task)
}

fn objects(app: &App) -> usize {
    app.document.as_ref().expect("a drawing").model.len()
}

/// The drawing's active layer, as the agent reads it.
fn layer(app: &mut App) -> String {
    let summary = kentos_headless::agent_tools::call(
        &mut super::host::AppHost(app),
        &mut kentos_headless::agent_tools::Guard::new(),
        "drawing.summary",
        &json!({}),
    )
    .expect("the summary");
    summary["activeLayer"]
        .as_str()
        .unwrap_or("cizim")
        .to_owned()
}

#[test]
fn the_agent_plans_then_draws_through_the_guard() {
    let layer_id = std::sync::Arc::new(Mutex::new(String::new()));
    let seen = std::sync::Arc::clone(&layer_id);
    let provider = Provider::start(move |n, body| {
        let input = json!({
            "layerId": *seen.lock().unwrap_or_else(PoisonError::into_inner),
            "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 6.0, "y": 0.0 }, { "x": 6.0, "y": 4.0 }, { "x": 0.0, "y": 4.0 }],
        });
        match n {
            0 => calls(
                "call_1",
                "cad__run",
                &json!({ "command": "cad.polygon.create", "input": input, "mode": "plan" }),
            ),
            1 => {
                let plan: Value = serde_json::from_str(&last_result(body)).unwrap_or_default();
                let plan = plan["planId"].as_str().unwrap_or("yok");
                calls(
                    "call_2",
                    "cad__run",
                    &json!({ "command": "cad.polygon.create", "input": input, "mode": "execute", "planId": plan }),
                )
            }
            _ => says("6 × 4 m alan çizildi."),
        }
    });
    let mut app = chatting(&provider);
    *layer_id.lock().unwrap_or_else(PoisonError::into_inner) = layer(&mut app);
    let before = objects(&app);
    let (message, task) = send(&mut app, "Etkin katmana 6 × 4 m bir alan çiz.");
    drive(&mut app, task);

    assert_eq!(objects(&app), before + 1, "the plan was executed: one area");
    let said = app.assistant.chat.message(message).expect("the answer");
    assert_eq!(said.phase, Phase::Complete, "{:?}", said.failure);
    assert_eq!(said.body, "6 × 4 m alan çizildi.");
    assert_eq!(said.tools.len(), 2);
    assert!(
        said.tools.iter().all(|t| t.phase == ToolPhase::Complete),
        "{:?}",
        said.tools
    );
    assert_eq!(said.tools[0].name, "Kapalı alan oluştur · plan");
    assert_eq!(
        said.usage.map(|u| u.input_tokens),
        Some(240),
        "two rounds' tokens"
    );
    assert!(
        app.log
            .lines()
            .any(|l| l.text == "Asistan: cad.polygon.create"),
        "the write is the assistant's in the log"
    );
    // What the model was sent: the house rules, the tools by their wire names, the plan's result back.
    let bodies = provider.bodies();
    assert_eq!(bodies.len(), 3);
    let system = bodies[0]["messages"][0]["content"]
        .as_str()
        .unwrap_or_default();
    assert!(system.contains("mode: \"plan\""), "{system}");
    let tools: Vec<&str> = bodies[0]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|t| t["function"]["name"].as_str())
        .collect();
    assert!(
        tools.contains(&"cad__run") && tools.contains(&"catalog__search"),
        "{tools:?}"
    );
    assert!(
        last_result(&bodies[2]).contains("completed"),
        "{}",
        last_result(&bodies[2])
    );
    // The next turn goes on from what was said and done.
    let history = app.assistant.history.as_ref().expect("the history");
    assert_eq!(
        history.messages.len(),
        6,
        "user, call, result, call, result, answer"
    );
    // One undo step takes the area away.
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(objects(&app), before);
}

#[test]
fn a_write_that_takes_away_waits_for_the_user_and_refused_is_not_run() {
    let provider = Provider::start(|n, _| match n {
        0 => calls(
            "call_1",
            "cad__run",
            &json!({ "command": "cad.entities.delete", "input": { "ids": [1] }, "mode": "execute", "planId": "p" }),
        ),
        _ => says("Tamam, silmedim."),
    });
    let mut app = chatting(&provider);
    let before = objects(&app);
    let (message, task) = send(&mut app, "Her şeyi sil.");
    let mut stream = iced_runtime::task::into_stream(task).expect("the turn's events");
    let mut refused = false;
    while let Some(action) = iced::futures::executor::block_on(stream.next()) {
        let iced_runtime::Action::Output(m) = action else {
            continue;
        };
        let next = app.update(m);
        drive(&mut app, next);
        let waiting = app.assistant.chat.message(message).and_then(|m| {
            m.tools
                .iter()
                .find(|t| t.phase == ToolPhase::Approval)
                .map(|t| t.id)
        });
        if let Some(tool) = waiting {
            assert_eq!(app.assistant.chat.phase(), Phase::Waiting);
            let next = app.update(msg(Event::Chat(AiEvent::ApproveTool {
                message,
                tool,
                approved: false,
            })));
            drive(&mut app, next);
            refused = true;
        }
    }
    assert!(refused, "the delete waited for the user");
    assert_eq!(objects(&app), before, "nothing was deleted");
    let said = app.assistant.chat.message(message).expect("the answer");
    assert_eq!(said.body, "Tamam, silmedim.");
    assert_eq!(said.tools[0].phase, ToolPhase::Error);
    assert!(last_result(&provider.bodies()[1]).contains("onaylamadı"));
}

#[test]
fn a_refused_key_is_said_in_the_chat() {
    let provider = Provider::start(|_, _| {
        (
            401,
            json!({ "error": { "message": "Invalid API key", "type": "invalid_request_error" } })
                .to_string(),
        )
    });
    let mut app = chatting(&provider);
    let (message, task) = send(&mut app, "Merhaba");
    drive(&mut app, task);
    let said = app.assistant.chat.message(message).expect("the answer");
    assert_eq!(said.phase, Phase::Error);
    let why = said.failure.clone().unwrap_or_default();
    assert!(
        !why.contains("deneme-anahtari"),
        "the key never shows: {why}"
    );
    assert!(!app.assistant.busy());
}

#[test]
fn without_an_agent_the_chat_says_where_to_add_one() {
    let mut app = app_with_drawing();
    app.assistant.chat.prompt = PromptState::with_text("Merhaba");
    let _ = app.update(msg(Event::Chat(AiEvent::Submit)));
    let said = app.assistant.chat.messages().last().expect("the answer");
    assert!(
        said.failure
            .as_deref()
            .is_some_and(|f| f.contains("Yapay zekâ")),
        "{:?}",
        said.failure
    );
    // Ajan ekle opens Uygulama ayarları on Yapay zekâ with the form.
    let _ = app.update(msg(Event::Settings));
    assert_eq!(app.dialog, Some(crate::app::Dialog::Settings));
    assert!(app.assistant.agents.editor.is_some());
}

fn agents(app: &mut App, edit: AgentsEdit) {
    let task = app.update(Message::Settings(Edit::Agents(edit)));
    drive(app, task);
}

#[test]
fn the_settings_keep_the_agents_and_their_default() {
    let mut app = app_with_drawing();
    let deepseek = ProviderKind::ALL
        .iter()
        .position(|k| *k == ProviderKind::DeepSeek)
        .expect("DeepSeek");
    agents(&mut app, AgentsEdit::Add);
    agents(&mut app, AgentsEdit::Kind(deepseek));
    agents(&mut app, AgentsEdit::Name("İş ajanım".into()));
    // A provider that needs a key is not kept without one.
    agents(&mut app, AgentsEdit::Save);
    assert!(app.assistant.store.list().is_empty());
    agents(&mut app, AgentsEdit::Key("sk-deneme".into()));
    agents(&mut app, AgentsEdit::Keep(1));
    agents(&mut app, AgentsEdit::Save);
    let list = app.assistant.store.list();
    assert_eq!(list.len(), 1);
    assert!(list[0].default, "the first agent is the default");
    assert_eq!(list[0].key, KeySource::Session);
    assert_eq!(
        app.assistant.store.key_for(&list[0]).as_deref(),
        Some("sk-deneme")
    );
    assert!(
        app.assistant.store.path().is_none(),
        "tests never write the user's file"
    );
    assert_eq!(app.assistant.shown.0, "İş ajanım");
    // A second agent, then the default moved to it.
    agents(&mut app, AgentsEdit::Add);
    agents(&mut app, AgentsEdit::Name("Yedek".into()));
    agents(&mut app, AgentsEdit::Key("sk-yedek".into()));
    agents(&mut app, AgentsEdit::Save);
    let second = app
        .assistant
        .store
        .list()
        .into_iter()
        .find(|p| p.name == "Yedek")
        .expect("kept");
    assert!(!second.default);
    agents(&mut app, AgentsEdit::MakeDefault(second.id.clone()));
    assert_eq!(
        app.assistant.store.default_profile().map(|p| p.name),
        Some("Yedek".to_owned())
    );
    assert_eq!(app.assistant.shown.0, "Yedek");
    agents(&mut app, AgentsEdit::Remove(second.id));
    assert_eq!(
        app.assistant.shown.0, "İş ajanım",
        "the one left is the default"
    );
    // The agent's window is over the settings window: Esc closes it, the settings window stays.
    let _ = app.update(Message::Run("tools.options"));
    agents(&mut app, AgentsEdit::Add);
    assert!(app.agent_window().is_some());
    let _ = app.update(Message::DialogClosed);
    assert!(app.assistant.agents.editor.is_none());
    assert_eq!(app.dialog, Some(crate::app::Dialog::Settings));
    let _ = app.update(Message::DialogClosed);
    assert_eq!(app.dialog, None);
}

/// The model field's suggestions: the provider's, then its list's, each once; all of them while the field holds one,
/// else those holding what is typed.
#[test]
fn the_model_field_suggests_the_providers_models() {
    let mut app = app_with_drawing();
    let deepseek = ProviderKind::ALL
        .iter()
        .position(|k| *k == ProviderKind::DeepSeek)
        .expect("DeepSeek");
    agents(&mut app, AgentsEdit::Add);
    agents(&mut app, AgentsEdit::Kind(deepseek));
    agents(
        &mut app,
        AgentsEdit::Models(Ok(vec!["deepseek-flash".into(), "deepseek-yeni".into()])),
    );
    let names = |app: &App| -> Vec<String> {
        let e = app.assistant.agents.editor.as_ref().expect("the form");
        super::settings::model_choices(e)
            .into_iter()
            .map(|(n, _)| n)
            .collect()
    };
    let all = names(&app);
    assert!(all.contains(&"deepseek-yeni".to_owned()), "{all:?}");
    assert_eq!(
        all.iter().filter(|n| *n == "deepseek-flash").count(),
        1,
        "each once"
    );
    agents(&mut app, AgentsEdit::Model("yen".into()));
    assert_eq!(names(&app), ["deepseek-yeni"]);
}

#[test]
fn a_write_waits_for_the_user_only_when_it_takes_away() {
    let run = |command: &str, mode: &str| json!({ "command": command, "input": {}, "mode": mode });
    assert!(needs_approval(
        "cad.run",
        &run("cad.entities.delete", "execute")
    ));
    assert!(!needs_approval(
        "cad.run",
        &run("cad.entities.delete", "plan")
    ));
    assert!(!needs_approval(
        "cad.run",
        &run("cad.polygon.create", "execute")
    ));
    assert!(needs_approval(
        "arch.run",
        &run("cad.entities.edit", "execute")
    ));
    let steps = |command: &str| {
        json!({ "steps": [{ "command": "cad.polygon.create", "input": {} }, { "command": command, "input": {} }],
            "mode": "execute" })
    };
    assert!(needs_approval(
        "command.batch",
        &steps("cad.entities.delete")
    ));
    assert!(!needs_approval("command.batch", &steps("cad.text.create")));
    assert!(needs_approval(
        "audit.apply_fix",
        &json!({ "command": "cad.topology.repair", "mode": "execute" })
    ));
    assert!(!needs_approval(
        "catalog.search",
        &json!({ "query": "sil" })
    ));
}

/// A door the program never opens is not offered (measured: KentOS Mimari and GIS run no road command; Harita and
/// Yol reach the others through their interfaces, Görünümler's `cad.views.edit` being architecture's, so they keep
/// every door).
#[test]
fn a_module_program_offers_its_own_doors() {
    let doors = |program| {
        let mut app = app_with_drawing();
        app.program = program;
        super::tool_specs(&mut app)
            .into_iter()
            .map(|t| t.name)
            .filter(|n| {
                n.ends_with(".run") && !n.starts_with("audit.") && !n.starts_with("processing.")
            })
            .collect::<Vec<_>>()
    };
    let every = ["cad.run", "arch.run", "map.run", "gis.run", "road.run"];
    assert_eq!(doors(None), every);
    for module in [
        kentos_launch::Module::Architecture,
        kentos_launch::Module::Gis,
    ] {
        assert_eq!(doors(Some(module)), every[..4], "{module:?}");
    }
    assert_eq!(doors(Some(kentos_launch::Module::Road)), every);
}
