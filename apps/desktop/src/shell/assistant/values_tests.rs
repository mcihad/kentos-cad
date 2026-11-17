//! Öznitelik değerleri gönderilmesin (docs/adr/8057 §103, PY-AI-13), against a provider on this computer: what the
//! provider is sent of the drawing when the agent's own setting keeps the values, and when the organisation's policy
//! keeps them from an agent outside this computer.

use kentos_agent::{AgentProfile, KeySource, ProviderKind};
use serde_json::{Value, json};

use super::AgentsEdit;
use super::tests::{Provider, agents, calls, chatting, says, send};
use crate::app::App;
use crate::files_testing::drive;

/// A parcel with its owner's name and its island's number, drawn by an agent on the link (which reads the drawing
/// whole).
fn parcel(app: &mut App) {
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .expect("a layer");
    let pts =
        json!([{ "x": 0, "y": 0 }, { "x": 20, "y": 0 }, { "x": 20, "y": 10 }, { "x": 0, "y": 10 }]);
    app.agent_request(
        &json!({ "method": "run", "params": { "command": "cad.polygon.create", "op": "execute",
                 "input": { "layerId": layer, "pts": pts, "attrs": { "Malik": "Ayşe Yılmaz", "Ada": "4471" } } } }),
        "Ajan",
    )
    .expect("drawn");
}

/// The tool's result the provider was sent in request `n`.
fn result(provider: &Provider, n: usize) -> String {
    let bodies = provider.bodies();
    bodies[n]["messages"]
        .as_array()
        .and_then(|m| m.iter().rev().find(|m| m["role"] == "tool"))
        .and_then(|m| m["content"].as_str())
        .unwrap_or_default()
        .to_owned()
}

/// The agent reads the objects: as it is, the provider is sent the values; once the agent's form says “Gönderilmesin”,
/// the same answer carries the field names and ‹gizli› in their place, and the panel says so.
#[test]
fn an_agent_set_to_withhold_values_is_sent_none() {
    let provider = Provider::start(|n, _: &Value| {
        if n % 2 == 0 {
            calls("c1", "scene.entities", &json!({}))
        } else {
            says("Tamam.")
        }
    });
    let mut app = chatting(&provider);
    parcel(&mut app);
    let (_, task) = send(&mut app, "Nesneleri oku.");
    drive(&mut app, task);
    let sent = result(&provider, 1);
    assert!(
        sent.contains("Ayşe Yılmaz") && sent.contains("4471"),
        "{sent}"
    );
    // The agent's form: Gönderilen veri › Öznitelik değerleri › Gönderilmesin.
    let id = app.assistant.store.default_profile().expect("the agent").id;
    agents(&mut app, AgentsEdit::Change(id.clone()));
    agents(&mut app, AgentsEdit::Withhold);
    agents(&mut app, AgentsEdit::Save);
    assert!(app.assistant.store.get(&id).expect("kept").withhold_values);
    assert!(app.assistant.withholds_values());
    let _ = app.assistant_view();
    let (_, task) = send(&mut app, "Yeniden oku.");
    drive(&mut app, task);
    let sent = result(&provider, 3);
    assert!(
        !sent.contains("Ayşe Yılmaz") && !sent.contains("4471"),
        "{sent}"
    );
    assert!(sent.contains("Malik") && sent.contains("‹gizli›"), "{sent}");
    // An agent on the link is its own program: it reads the values still.
    let link = app
        .agent_request(&json!({ "method": "entities", "params": {} }), "Ajan")
        .expect("the objects");
    assert!(link.to_string().contains("Ayşe Yılmaz"));
}

/// `allowAttributeValuesToAi: false` keeps the values from an agent outside this computer, not from one on it.
#[test]
fn the_policy_keeps_the_values_from_an_agent_outside_this_computer() {
    let provider = Provider::start(|_, _: &Value| says("Merhaba."));
    let mut app = chatting(&provider);
    app.assistant.policy = Some(
        kentos_extension_host::Policy::read(
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1, "allowAttributeValuesToAi": false}"#,
        )
        .expect("a policy"),
    );
    assert!(
        !app.assistant.withholds_values(),
        "the agent on this computer"
    );
    let store = &app.assistant.store;
    let outside = store
        .add(AgentProfile {
            key: KeySource::Session,
            ..AgentProfile::new(ProviderKind::DeepSeek, "DeepSeek", "deepseek-chat")
        })
        .expect("added");
    store.set_default(&outside.id).expect("the default");
    app.assistant.refresh();
    assert!(app.assistant.withholds_values());
    // The agent's form says it and keeps the box ticked.
    agents(&mut app, AgentsEdit::Change(outside.id));
    let _ = app.ai_section();
}
