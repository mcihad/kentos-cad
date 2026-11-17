//! The limits on Asistan (docs/adr/8056 §6, §7), against a provider on this computer: an agent calling tools past its
//! conversation's limit stops there and the next turn is refused until a new conversation; a day that spent its limit
//! refuses the agent until tomorrow; the agent's form keeps the limits and says what is not one. The organisation's
//! policy keeps the drawing from an agent outside this computer, before a key is asked.

use kentos_agent::{AgentProfile, KeySource, ProviderKind};
use kentos_ui::widget::ai::{AiEvent, Phase, PromptState};
use serde_json::json;

use super::tests::{Provider, agents, calls, chatting, says, send};
use super::{AgentsEdit, Event, msg, today};
use crate::app::App;
use crate::files_testing::{app_with_drawing, drive};

/// The default agent with `edit` made to it.
fn limited(app: &mut App, edit: impl FnOnce(&mut kentos_agent::AgentProfile)) -> String {
    let store = &app.assistant.store;
    let mut agent = store.default_profile().expect("the default agent");
    edit(&mut agent);
    store.update(agent.clone()).expect("kept");
    agent.id
}

fn failure(app: &App, message: u64) -> String {
    let said = app.assistant.chat.message(message).expect("the answer");
    assert_eq!(said.phase, Phase::Error, "{said:?}");
    said.failure.clone().unwrap_or_default()
}

/// What the chat says to `words` refused before a turn starts.
fn refused(app: &mut App, words: &str) -> String {
    app.assistant.chat.prompt = PromptState::with_text(words);
    let task = app.update(msg(Event::Chat(AiEvent::Submit)));
    drive(app, task);
    assert!(!app.assistant.busy(), "no turn started");
    let said = app.assistant.chat.messages().last().expect("the answer");
    assert_eq!(said.phase, Phase::Error, "{said:?}");
    said.failure.clone().unwrap_or_default()
}

#[test]
fn an_agent_past_its_conversations_limit_stops_and_a_new_conversation_starts_again() {
    // The first answer calls a tool and costs 150 tokens; any later one answers in words.
    let provider = Provider::start(|n, _| match n {
        0 => calls("call_1", "drawing__summary", &json!({})),
        _ => says("Çizim hazır."),
    });
    let mut app = chatting(&provider);
    let id = limited(&mut app, |a| a.conversation_tokens = Some(100));
    let (message, task) = send(&mut app, "Çizimi özetle.");
    drive(&mut app, task);
    let why = failure(&app, message);
    assert!(
        why.starts_with("Ajan durduruldu") && why.contains("150 / 100 token"),
        "{why}"
    );
    assert_eq!(
        provider.bodies().len(),
        1,
        "nothing more asked of the provider"
    );
    assert!(!app.assistant.busy());
    assert_eq!(app.assistant.spent, 150);
    assert_eq!(
        app.assistant.ledger.spent(&id, &today()),
        150,
        "the day counts it too"
    );
    // The next turn of the conversation is refused before it starts.
    let why = refused(&mut app, "Devam et.");
    assert!(why.contains("Yeni sohbet"), "{why}");
    assert_eq!(provider.bodies().len(), 1);
    // A new conversation counts from nothing; the day keeps what was spent.
    let _ = app.update(msg(Event::New));
    assert_eq!(app.assistant.spent, 0);
    let (message, task) = send(&mut app, "Merhaba");
    drive(&mut app, task);
    let said = app.assistant.chat.message(message).expect("the answer");
    assert_eq!(said.phase, Phase::Complete, "{said:?}");
    assert_eq!(
        said.usage.and_then(|u| u.limit),
        Some(100),
        "the panel shows the limit"
    );
    assert_eq!(app.assistant.ledger.spent(&id, &today()), 150);
}

#[test]
fn a_day_that_spent_its_limit_refuses_the_agent() {
    let provider = Provider::start(|_, _| says("Çizim hazır."));
    let mut app = chatting(&provider);
    let id = limited(&mut app, |a| a.daily_tokens = Some(1_000));
    app.assistant.ledger.add(&id, &today(), 1_000);
    let why = refused(&mut app, "Merhaba");
    assert!(
        why.contains("bugün") && why.contains("1.000 / 1.000 token"),
        "{why}"
    );
    assert!(provider.bodies().is_empty());
}

#[test]
fn the_agents_form_keeps_its_limits_and_says_what_is_not_one() {
    let mut app = app_with_drawing();
    agents(&mut app, AgentsEdit::Add);
    agents(&mut app, AgentsEdit::Key("sk-deneme".into()));
    agents(&mut app, AgentsEdit::ConversationTokens("1,5".into()));
    agents(&mut app, AgentsEdit::Save);
    assert!(
        app.assistant.store.list().is_empty(),
        "not a limit: not kept"
    );
    agents(&mut app, AgentsEdit::ConversationTokens("200.000".into()));
    agents(&mut app, AgentsEdit::DailyTokens("1 000 000".into()));
    agents(&mut app, AgentsEdit::Save);
    let list = app.assistant.store.list();
    assert_eq!(list.len(), 1);
    assert_eq!(
        (list[0].conversation_tokens, list[0].daily_tokens),
        (Some(200_000), Some(1_000_000))
    );
    // Opened again, the form writes them with dots; emptied, there is no limit.
    let _ = app.update(crate::app::Message::Run("tools.options"));
    agents(&mut app, AgentsEdit::Change(list[0].id.clone()));
    assert!(app.agent_window().is_some());
    agents(&mut app, AgentsEdit::ConversationTokens(String::new()));
    agents(&mut app, AgentsEdit::Save);
    let kept = app.assistant.store.get(&list[0].id).expect("kept");
    assert_eq!(
        (kept.conversation_tokens, kept.daily_tokens),
        (None, Some(1_000_000))
    );
}

#[test]
fn the_organisations_policy_keeps_the_drawing_from_an_agent_outside_this_computer() {
    let provider = Provider::start(|_, _| says("Merhaba."));
    let mut app = chatting(&provider);
    app.assistant.policy = Some(
        kentos_extension_host::Policy::read(
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1, "allowDataToAi": false}"#,
        )
        .expect("a policy"),
    );
    // The agent on this computer still answers.
    let (message, task) = send(&mut app, "Merhaba");
    drive(&mut app, task);
    let said = app.assistant.chat.message(message).expect("the answer");
    assert_eq!(said.phase, Phase::Complete, "{said:?}");
    // One outside is refused before its key is asked and before any request.
    let store = &app.assistant.store;
    let outside = store
        .add(AgentProfile {
            key: KeySource::Session,
            ..AgentProfile::new(ProviderKind::DeepSeek, "DeepSeek", "deepseek-chat")
        })
        .expect("added");
    store.set_default(&outside.id).expect("the default");
    app.assistant.refresh();
    let why = refused(&mut app, "Çizimi özetle.");
    assert!(
        why.contains("bu bilgisayarın dışındaki")
            && why.contains("“DeepSeek” https://api.deepseek.com"),
        "{why}"
    );
    assert!(
        !why.contains("API anahtarı"),
        "the policy, not the key: {why}"
    );
    assert_eq!(provider.bodies().len(), 1);
    // The settings say it on the agent's row.
    let _ = app.update(crate::app::Message::Run("tools.options"));
    assert!(app.assistant.refusal(&outside).is_some());
    let _ = app.ai_section();
}

/// The workshop refuses a file holding the user's own agent key, whatever its shape, saying whose and never the value;
/// the same file without it is written (docs/adr/8059 §15).
#[test]
fn the_workshop_refuses_the_users_own_key() {
    use kentos_headless::agent_tools::{self, Guard};

    let root = std::env::temp_dir().join(format!("kentos-asistan-anahtar-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let extensions = root.join("eklentiler");
    std::fs::create_dir_all(&extensions).unwrap();
    let provider = Provider::start(|_, _| says("Tamam."));
    let mut app = chatting(&provider);
    app.extensions = crate::extensions::Extensions::open(&extensions, false).0;
    let mut guard = Guard::new();
    let mut host = super::host::AppHost(&mut app);
    let mut write = |host: &mut super::host::AppHost, code: &str| {
        agent_tools::call(
            host,
            &mut guard,
            "extension.workshop.write",
            &json!({ "id": "tr.ornek.anahtar", "files": { "src/anahtar.py": code } }),
        )
    };
    // chatting() gave the agent the key “deneme-anahtari” for this session.
    let refused = write(&mut host, "ANAHTAR = 'deneme-anahtari'\n").expect_err("refused");
    assert_eq!(refused.code, "secret_in_text");
    assert!(
        refused
            .message
            .contains("“Deneme ajanı” ajanının API anahtarı")
            && !refused.message.contains("deneme-anahtari"),
        "{}",
        refused.message
    );
    assert!(
        !root
            .join("eklenti-atolyesi/tr.ornek.anahtar/src/anahtar.py")
            .exists()
    );
    write(
        &mut host,
        "import os\nANAHTAR = os.environ.get('SERVIS_ANAHTARI')\n",
    )
    .expect("written");
    // A service connection's password too, said by its connection.
    crate::services::secrets::secrets()
        .put(
            "https://harita.ornek.bel.tr",
            kentos_commands::ConnectionSecret {
                id: "atolye-sinamasi".into(),
                user: Some("harita".into()),
                password: Some("Parola-2026-Ornek!".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let refused = write(&mut host, "PAROLA = 'Parola-2026-Ornek!'\n").expect_err("refused");
    assert!(
        refused
            .message
            .contains("https://harita.ornek.bel.tr bağlantısının parolası")
            && !refused.message.contains("Parola-2026"),
        "{}",
        refused.message
    );
    let _ = std::fs::remove_dir_all(&root);
}
