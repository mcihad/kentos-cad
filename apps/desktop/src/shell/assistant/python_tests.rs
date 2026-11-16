//! The agent's own Python, a class apart from its tool calls (docs/adr/8057 §104, PY-AI-11), against a provider on this
//! computer: a trial of its workshop's code waits for the user's Onayla unless the user let the agent run it without
//! asking; an agent set not to run it is not offered the tools and a call of one is refused; the organisation's policy
//! closes it for every agent.

use iced::futures::StreamExt as _;
use kentos_agent::PythonRuns;
use kentos_headless::agent_tools::{self, Guard, PYTHON_RUNS};
use kentos_ui::widget::ai::{AiEvent, ToolPhase};
use serde_json::{Value, json};

use super::tests::{Provider, agents, calls, chatting, says, send};
use super::{AgentsEdit, Event, msg};
use crate::app::App;
use crate::files_testing::drive;

/// The provider's tool name for an agent tool (`extension.workshop.trial` → `extension__workshop__trial`).
fn wire(name: &str) -> String {
    name.replace('.', "__")
}

/// `app` with an extension folder of its own, so Asistan has a workshop beside it (its tools are offered).
fn with_workshop(app: &mut App, name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "kentos-asistan-python-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let extensions = root.join("eklentiler");
    std::fs::create_dir_all(&extensions).expect("the folder");
    app.extensions = crate::extensions::Extensions::open(&extensions, false).0;
    root
}

/// The default agent's “Kod çalıştırma › Atölyedeki kodu” set to `runs`, through its form.
fn set(app: &mut App, runs: PythonRuns) {
    let id = app.assistant.store.default_profile().expect("an agent").id;
    agents(app, AgentsEdit::Change(id.clone()));
    let at = PythonRuns::ALL
        .iter()
        .position(|r| *r == runs)
        .expect("a choice");
    agents(app, AgentsEdit::Python(at));
    agents(app, AgentsEdit::Save);
    assert_eq!(
        app.assistant.store.get(&id).expect("kept").python_runs,
        runs
    );
}

/// A turn in which the model asks for a trial of an extension; whether its call waited for the user (refused when it
/// did) and what the model was told.
fn trial_turn(app: &mut App, provider: &Provider, first: usize) -> (bool, String) {
    let (message, task) = send(app, "Eklentini dene.");
    let mut stream = iced_runtime::task::into_stream(task).expect("the turn's events");
    let mut waited = false;
    while let Some(action) = iced::futures::executor::block_on(stream.next()) {
        let iced_runtime::Action::Output(m) = action else {
            continue;
        };
        let next = app.update(m);
        drive(app, next);
        let waiting = app.assistant.chat.message(message).and_then(|m| {
            m.tools
                .iter()
                .find(|t| t.phase == ToolPhase::Approval)
                .map(|t| t.id)
        });
        if let Some(tool) = waiting {
            waited = true;
            let card = app.assistant.chat.message(message).and_then(|m| {
                m.tools
                    .iter()
                    .find(|t| t.id == tool)
                    .map(|t| t.detail.clone())
            });
            assert!(
                card.is_some_and(|d| d.starts_with(super::PYTHON_ASKS)),
                "the card says what runs"
            );
            let next = app.update(msg(Event::Chat(AiEvent::ApproveTool {
                message,
                tool,
                approved: false,
            })));
            drive(app, next);
        }
    }
    let told = provider.bodies()[first + 1]["messages"]
        .as_array()
        .and_then(|m| m.iter().rev().find(|m| m["role"] == "tool"))
        .and_then(|m| m["content"].as_str())
        .unwrap_or_default()
        .to_owned();
    (waited, told)
}

#[test]
fn the_agents_own_python_waits_for_the_user_unless_let_run() {
    let provider = Provider::start(|n, _: &Value| {
        if n % 2 == 0 {
            calls(
                "c1",
                &wire("extension.workshop.trial"),
                &json!({ "id": "tr.ornek.yok" }),
            )
        } else {
            says("Tamam.")
        }
    });
    let mut app = chatting(&provider);
    assert_eq!(app.assistant.python_runs(), PythonRuns::Ask, "the default");
    let (waited, told) = trial_turn(&mut app, &provider, 0);
    assert!(waited, "the trial waited for the user");
    assert!(told.contains("onaylamadı"), "{told}");
    assert!(app.extensions.trial.is_none(), "nothing ran");
    // Let run without asking: the call runs at once (and the trial answers for itself: no drawing given).
    set(&mut app, PythonRuns::Allow);
    let (waited, told) = trial_turn(&mut app, &provider, 2);
    assert!(!waited, "{told}");
    assert!(
        !told.contains("onaylamadı")
            && !told.contains("python_refused")
            && told.contains("features"),
        "{told}"
    );
}

#[test]
fn an_agent_set_not_to_run_python_is_not_offered_the_tools() {
    let provider = Provider::start(|_, _: &Value| says("Tamam."));
    let mut app = chatting(&provider);
    let root = with_workshop(&mut app, "sunulan");
    let offered = |provider: &Provider, n: usize| -> Vec<String> {
        provider.bodies()[n]["tools"]
            .as_array()
            .map(|tools| {
                tools
                    .iter()
                    .filter_map(|t| t["function"]["name"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    let (_, task) = send(&mut app, "Merhaba.");
    drive(&mut app, task);
    let first = offered(&provider, 0);
    assert!(
        PYTHON_RUNS.iter().all(|t| first.contains(&wire(t))),
        "{first:?}"
    );
    set(&mut app, PythonRuns::Off);
    let (_, task) = send(&mut app, "Yeniden merhaba.");
    drive(&mut app, task);
    let later = offered(&provider, 1);
    assert!(
        PYTHON_RUNS.iter().all(|t| !later.contains(&wire(t))),
        "{later:?}"
    );
    assert!(
        later.contains(&wire("extension.workshop.write")),
        "the rest stays"
    );
    // Called anyway, refused at the tools: whatever asks.
    let refused = agent_tools::call(
        &mut super::host::AppHost(&mut app),
        &mut Guard::new(),
        "extension.workshop.trial",
        &json!({ "id": "tr.ornek.yok" }),
    )
    .expect_err("refused");
    assert_eq!(refused.code, "python_refused");
    assert!(
        refused.message.contains("Kod çalıştırma"),
        "{}",
        refused.message
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_policy_closes_every_agents_python() {
    let provider = Provider::start(|_, _: &Value| says("Tamam."));
    let mut app = chatting(&provider);
    set(&mut app, PythonRuns::Allow);
    app.assistant.policy = Some(
        kentos_extension_host::Policy::read(
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1, "allowAgentPython": false}"#,
        )
        .expect("a policy"),
    );
    assert_eq!(app.assistant.python_runs(), PythonRuns::Off);
    assert!(
        app.assistant
            .python_refusal()
            .is_some_and(|w| w.contains("Kurum ilkesi"))
    );
    // Its form says so and offers no other choice.
    let id = app.assistant.store.default_profile().expect("an agent").id;
    agents(&mut app, AgentsEdit::Change(id));
    let _ = app.ai_section();
}
