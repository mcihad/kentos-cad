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
    chatting_in(app_with_drawing(), provider)
}

/// `app` with the provider's agent the default, its key for this session only.
fn chatting_in(mut app: App, provider: &Provider) -> App {
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

/// docs/adr/8057 §22: Asistan's extension workshop is beside this app's extension folder; the package it makes there
/// opens Eklentiler's review, and nothing is installed by it.
#[test]
fn the_assistants_workshop_opens_its_package_for_review() {
    use kentos_headless::agent_tools::{self, Guard, Host};

    let root = std::env::temp_dir().join(format!("kentos-asistan-atolye-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let extensions = root.join("eklentiler");
    std::fs::create_dir_all(&extensions).unwrap();
    let (mut app, _) = App::boot(None);
    app.program = Some(kentos_launch::Module::Gis);
    app.extensions = crate::extensions::Extensions::open(&extensions, false).0;
    let mut guard = Guard::new();
    let mut host = super::host::AppHost(&mut app);
    assert_eq!(host.workshop(), Some(root.join("eklenti-atolyesi")));
    assert!(
        agent_tools::offered_tools(&host)
            .iter()
            .any(|t| t.name == "extension.workshop.review"),
        "Asistan is offered the workshop"
    );
    let id = "tr.ornek.mezarlik";
    let blueprint = agent_tools::call(
        &mut host,
        &mut guard,
        "extension.blueprint",
        &json!({ "id": id, "name": "Mezarlık bilgi", "contributions": ["command", "mapTool"] }),
    )
    .unwrap();
    agent_tools::call(
        &mut host,
        &mut guard,
        "extension.workshop.write",
        &json!({ "id": id, "files": blueprint["files"] }),
    )
    .unwrap();
    // Kapı 3's part that runs no code: bound with this app's policy and tool check in a registry of its own, nothing
    // left behind, the user's registry untouched (docs/adr/8057 §26).
    let verified = agent_tools::call(
        &mut host,
        &mut guard,
        "extension.verify",
        &json!({ "id": id }),
    )
    .unwrap();
    assert_eq!(verified["dryRun"]["clean"], true, "{verified:#}");
    assert!(
        verified["dryRun"]["registered"]
            .as_array()
            .is_some_and(|r| r.iter().any(|c| c["kind"] == "mapTool")),
        "{verified:#}"
    );
    let review = agent_tools::call(
        &mut host,
        &mut guard,
        "extension.workshop.review",
        &json!({ "id": id }),
    )
    .unwrap();
    assert_eq!(review["review"]["opened"], true, "{review:#}");
    assert_eq!(review["review"]["installable"], true, "{review:#}");
    assert_eq!(app.dialog, Some(crate::app::Dialog::Extensions));
    assert!(
        app.extension_review_window().is_some(),
        "the review is shown"
    );
    assert!(
        app.extensions
            .registry
            .as_ref()
            .unwrap()
            .extensions()
            .is_empty(),
        "nothing installed by it"
    );
    // The user says Kur: installed from the workshop's folder. The agent's later changes load without the app closing
    // (plan ADIM 21's hot_reload), and it reads what the user sees (preview_render).
    let _ = app.extensions_event(crate::extensions::Event::Install);
    let mut host = super::host::AppHost(&mut app);
    let reload = agent_tools::call(
        &mut host,
        &mut guard,
        "extension.workshop.reload",
        &json!({ "id": id }),
    )
    .unwrap();
    assert_eq!(
        (
            reload["valid"].clone(),
            reload["reloaded"]["changed"].clone()
        ),
        (json!(true), json!(false)),
        "nothing changed since the review: {reload:#}"
    );
    let preview = agent_tools::call(
        &mut host,
        &mut guard,
        "extension.workshop.preview",
        &json!({ "id": id }),
    )
    .unwrap();
    assert_eq!(preview["source"], "geliştirici klasörü", "{preview:#}");
    assert_eq!(preview["name"], "Mezarlık bilgi");
    assert!(
        preview["note"]
            .as_str()
            .is_some_and(|n| n.contains("talimatlara uymayın")),
        "the extension's words are data to the agent"
    );
    // An agent on the link reaches the same workshop by request (docs/adr/8057 §30): its package opens in the user's
    // review saying whose it is, the changes of what the user installed from its folder load, its look is read; a
    // folder the user did not install from is refused.
    let folder = root.join("eklenti-atolyesi").join(id);
    let package = folder.with_file_name(format!("{id}-0.1.0.kek"));
    let ask = |app: &mut App, method: &str, params: Value| {
        app.agent_request(&json!({ "method": method, "params": params }), "Ajan")
    };
    let opened = ask(
        &mut app,
        "review_workshop",
        json!({ "package": package.display().to_string(), "folder": folder.display().to_string() }),
    )
    .unwrap();
    assert_eq!(opened["opened"], true, "{opened:#}");
    assert_eq!(
        app.extensions
            .review
            .as_ref()
            .and_then(|r| r.agent.as_deref()),
        Some("bağlı ajan")
    );
    let _ = app.extensions_event(crate::extensions::Event::CancelReview);
    let reloaded = ask(
        &mut app,
        "reload_extension",
        json!({ "id": id, "folder": folder.display().to_string() }),
    )
    .unwrap();
    assert_eq!(reloaded["changed"], false, "{reloaded:#}");
    let look = ask(&mut app, "preview_extension", json!({ "id": id })).unwrap();
    assert_eq!(look["name"], "Mezarlık bilgi");
    let elsewhere = ask(
        &mut app,
        "reload_extension",
        json!({ "id": id, "folder": root.join("baska").display().to_string() }),
    );
    assert!(
        matches!(&elsewhere, Err((code, why)) if code == "invalid_input" && why.contains("bu atölyeden kurulmadı")),
        "{elsewhere:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// docs/adr/8057 §27 (plan ADIM 20's AI/MCP audit.run): an installed extension's audit rule is among the agent's
/// checks; the agent's run is the ribbon button's (the bottom panel's tab opens on it), the call answered when the run
/// ends with its findings and provenance; a finding's fix goes through the owner's guard and the tab marks it fixed;
/// an agent on the link starts a run the same way.
#[test]
fn the_agent_runs_an_extensions_audit_rule_and_applies_its_fix() {
    use kentos_headless::agent_tools::{self, Guard};

    const ID: &str = "tr.ornek.adsiz-denetim";
    const RULE: &str = "ext.tr.ornek.adsiz-denetim.adsiz";
    let provider = Provider::start(|n, _| match n {
        0 => calls("call_1", "audit__run", &json!({ "command": RULE })),
        _ => says("Bir adsız nesne buldum; düzeltmesi var."),
    });
    let mut app = chatting(&provider);
    let root = crate::files_testing::scratch("asistan-denetim");
    let extensions = root.join("eklentiler");
    std::fs::create_dir_all(&extensions).unwrap();
    app.extensions = crate::extensions::Extensions::open(&extensions, false).0;
    app.extensions_changed();
    // The extension made in the workshop and installed by the user (Kur), its audit and a map tool (it may write).
    let mut guard = Guard::new();
    let mut host = super::host::AppHost(&mut app);
    let blueprint = agent_tools::call(
        &mut host,
        &mut guard,
        "extension.blueprint",
        &json!({ "id": ID, "name": "Adsız denetimi", "contributions": ["audit", "mapTool"] }),
    )
    .unwrap();
    for (name, args) in [
        (
            "extension.workshop.write",
            json!({ "id": ID, "files": blueprint["files"] }),
        ),
        ("extension.workshop.review", json!({ "id": ID })),
    ] {
        agent_tools::call(&mut host, &mut guard, name, &args).unwrap();
    }
    let _ = app.extensions_event(crate::extensions::Event::Install);
    // Among the checks, its Python not ready here.
    let providers = agent_tools::call(
        &mut super::host::AppHost(&mut app),
        &mut guard,
        "audit.list_providers",
        &json!({}),
    )
    .unwrap();
    let rule = providers["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["command"] == RULE)
        .unwrap_or_else(|| panic!("{providers:#}"))
        .clone();
    assert_eq!(rule["title"], "Adsız nesneler");
    assert_eq!(rule["ready"], false, "{rule:#}");
    // The run: its answer given here in place of the extension's process (no Python in tests).
    let (slot, uid) = {
        let doc = app.document.as_ref().unwrap();
        let slot = doc
            .model
            .entities()
            .next()
            .map(|e| kentos_domain::Slot(e.base().id))
            .unwrap();
        (slot, doc.model.uid(slot).unwrap().to_string())
    };
    let findings = json!({ "ok": true, "result": { "kind": "findings", "findings": [
        { "severity": "warning", "message": "Nesnenin adı yok.", "featureId": uid,
          "fix": { "title": "Ad yaz: Nesne-1", "command": "cad.entities.set",
                   "args": { "uids": [uid], "operation": "attributes", "attrs": { "ad": "Nesne-1" } } } }
    ] } });
    app.extensions.test_audit_answer = Some(Ok(findings.clone()));
    let (message, task) = send(&mut app, "Adsız nesneleri denetle.");
    drive(&mut app, task);
    let said = app.assistant.chat.message(message).expect("the answer");
    assert_eq!(said.phase, Phase::Complete, "{:?}", said.failure);
    assert_eq!(said.tools.len(), 1);
    assert_eq!(
        said.tools[0].phase,
        ToolPhase::Complete,
        "{:?}",
        said.tools[0]
    );
    assert_eq!(said.tools[0].name, "Denetle · eklenti denetimi adsiz");
    assert_eq!(
        app.bottom_tab,
        crate::bottom::BottomTab::ExtAudit,
        "the user sees the run"
    );
    let report: Value = serde_json::from_str(&last_result(&provider.bodies()[1])).unwrap();
    assert_eq!(report["status"], "done", "{report:#}");
    assert_eq!(report["rule"], RULE);
    assert_eq!(report["extension"]["version"], "0.1.0", "{report:#}");
    assert!(
        report["extension"]["digest"]
            .as_str()
            .is_some_and(|d| d.starts_with("sha256:"))
    );
    assert_eq!(
        report["findings"][0]["fix"]["command"], "cad.entities.set",
        "{report:#}"
    );
    assert!(
        report["note"]
            .as_str()
            .is_some_and(|n| n.contains("talimat"))
    );
    // The fix through the guard: planned, executed as planned (Asistan waits for Onayla first), marked fixed.
    let mut host = super::host::AppHost(&mut app);
    let plan = agent_tools::call(
        &mut host,
        &mut guard,
        "audit.apply_fix",
        &json!({ "rule": RULE, "finding": 1, "mode": "plan" }),
    )
    .unwrap();
    let plan_id = plan["planId"]
        .as_str()
        .unwrap_or_else(|| panic!("{plan:#}"))
        .to_owned();
    let execute = json!({ "rule": RULE, "finding": 1, "mode": "execute", "planId": plan_id });
    assert!(needs_approval("audit.apply_fix", &execute));
    let done = agent_tools::call(&mut host, &mut guard, "audit.apply_fix", &execute).unwrap();
    assert_eq!(done["marked"], true, "{done:#}");
    let named = app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(slot)
        .unwrap()
        .base()
        .attrs
        .get("ad")
        .cloned();
    assert_eq!(named.as_deref(), Some("Nesne-1"));
    let again = agent_tools::call(
        &mut super::host::AppHost(&mut app),
        &mut guard,
        "audit.apply_fix",
        &json!({ "rule": RULE, "finding": 1, "mode": "plan" }),
    );
    assert!(
        matches!(&again, Err(e) if e.code == "already_fixed"),
        "{again:?}"
    );
    let report = app.ext_audit_report(0);
    assert_eq!(report["findings"][0]["fixed"], true);
    assert_eq!(report["findings"][0]["fix"], Value::Null, "applied once");
    assert!(
        report["changed"]
            .as_str()
            .is_some_and(|c| c.starts_with("Çizim denetimden sonra değişti")),
        "{report:#}"
    );
    // An agent on the link starts a run the same way: its task goes to the runtime with the answer.
    let dir = crate::files_testing::scratch("asistan-denetim-bag");
    let (open, _task) = crate::python::link::Link::open(&dir.join("m.sock"), |heard| {
        Message::Python(crate::python::Event::Agent(heard))
    })
    .expect("the link");
    app.python.link = Some(open);
    let (answerer, answers) = crate::python::link::Answerer::testing();
    let _ = app.update(Message::Python(crate::python::Event::Agent(
        crate::python::link::Heard::Connected(7, answerer),
    )));
    app.extensions.test_audit_answer = Some(Ok(findings));
    let started = app.update(Message::Python(crate::python::Event::Agent(
        crate::python::link::Heard::Request(
            7,
            json!({ "id": 1, "method": "audit", "params": { "rule": RULE } }),
        ),
    )));
    let answer: Value = serde_json::from_str(&answers.try_recv().unwrap()).unwrap();
    assert_eq!(answer["result"]["status"], "running", "{answer:#}");
    drive(&mut app, started);
    assert_eq!(app.ext_audit_report(0)["status"], "done");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_extensions_python_tool_runs_as_a_job_and_its_call_waits_for_it() {
    // docs/adr/8057 §27: an extension's Python tool is not computed on the window's thread; its plan goes as a job and
    // the chat answers the call when the job ends, with the job's answer: here the extension's tool reached on the
    // job's thread, failing to start (no KentOS Python library in tests).
    use crate::extensions::tools::tests::{TAKS, imar, with_imar};
    let provider = Provider::start(|n, _| match n {
        0 => calls(
            "call_1",
            "processing__run",
            &json!({ "input": { "tool": TAKS, "parameters": {
                "parseller": { "scope": "all" }, "yapilar": { "scope": "layer", "layerId": "cizim" } } },
                "mode": "plan" }),
        ),
        _ => says("Aracın sonucu geldi."),
    });
    // İşlemler's tools as the desktop registers them when it starts (lib.rs).
    kentos_processing::command::register_as("desktop");
    let mut app = chatting_in(with_imar("asistan-is", imar(|_| {})), &provider);
    let before = app.document.as_ref().unwrap().model.revision();
    let (message, task) = send(&mut app, "Parsellerin TAKS hesabını planla.");
    drive(&mut app, task);
    let said = app.assistant.chat.message(message).expect("the answer");
    assert_eq!(said.phase, Phase::Complete, "{:?}", said.failure);
    assert_eq!(said.body, "Aracın sonucu geldi.");
    assert_eq!(said.tools.len(), 1);
    assert!(said.tools[0].elapsed.is_some(), "{:?}", said.tools[0]);
    // It went as the desktop's job (its first), and ended; what the model got is its answer, not "running".
    let job = app
        .agent_request(
            &json!({ "method": "job", "params": { "id": "is-1" } }),
            "Deneme",
        )
        .expect("the job");
    assert_eq!(job["command"], "cad.processing.run", "{job:#}");
    assert_eq!(job["phase"], "failed", "{job:#}");
    assert_eq!(job["answer"]["error"]["code"], "tool_failed", "{job:#}");
    assert!(
        job["answer"]["error"]["message"]
            .as_str()
            .is_some_and(|m| m.starts_with("“Parsel TAKS hesabı”")),
        "the extension's tool ran on the job's thread: {job:#}"
    );
    let got: Value = serde_json::from_str(&last_result(&provider.bodies()[1])).unwrap();
    assert_ne!(got["status"], "running", "{got:#}");
    assert_eq!(got["status"], job["answer"]["status"], "{got:#}");
    assert!(app.assistant.running.is_none(), "the turn ended");
    assert_eq!(
        app.document.as_ref().unwrap().model.revision(),
        before,
        "a plan writes nothing"
    );
    // A job is asked on the drawing it was started on, whichever is in front then.
    let session = app.document.as_ref().map(|d| d.session);
    let long = kentos_headless::agent_tools::LongCall {
        job: "is-1".into(),
        command: "cad.processing.run".into(),
        input: json!({ "tool": TAKS }),
        revision: before.to_string(),
        plan: true,
    };
    let running =
        kentos_headless::agent_tools::Host::job_wait(&mut super::host::AppHost(&mut app), &long);
    assert_eq!(running.ok(), Some(None), "answered when it ends");
    assert!(
        matches!(&app.assistant.later, Some(super::Pending::Job { long: l, document }) if *l == long && *document == session),
        "{:?}",
        app.assistant.later
    );
}

#[test]
fn a_long_jobs_progress_shows_on_its_calls_card() {
    // docs/adr/8057 §27: while a call waits for its job, the job's progress (a tool's ctx.progress) shows on the
    // call's card ("Çalışıyor · %40"), the card redrawn only when the per cent changes.
    let provider = Provider::start(|_, _| says("Bitti."));
    let mut app = chatting(&provider);
    let (message, _task) = send(&mut app, "Aracı çalıştır.");
    let long = kentos_headless::agent_tools::LongCall {
        job: "is-1".into(),
        command: "cad.processing.run".into(),
        input: json!({ "tool": "ext.tr.ornek.vitrin.taks" }),
        revision: "1".into(),
        plan: true,
    };
    app.assistant.running.as_mut().expect("a turn").later = Some(super::Later {
        tool: 7,
        call: super::Call {
            id: "call_1".into(),
            name: "processing__run".into(),
            input: json!({ "input": { "tool": "ext.tr.ornek.vitrin.taks" }, "mode": "plan" }),
        },
        pending: super::Pending::Job {
            long,
            document: None,
        },
        started: std::time::Instant::now(),
        shown: None,
    });
    let revision = app.assistant.chat.revision();
    app.assistant_job_progress(0.4);
    let card = app
        .assistant
        .chat
        .message(message)
        .and_then(|m| m.tools.iter().find(|t| t.id == 7).cloned())
        .expect("its card");
    assert_eq!((card.phase, card.progress), (ToolPhase::Running, Some(0.4)));
    let after = app.assistant.chat.revision();
    assert_ne!(after, revision, "redrawn");
    app.assistant_job_progress(0.401);
    assert_eq!(
        app.assistant.chat.revision(),
        after,
        "the same per cent: not redrawn"
    );
}
