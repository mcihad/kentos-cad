//! Asistan in a narrow panel (the CBS review's ninth item, docs/adr/9337): the box is as tall as its placeholder wraps
//! in the panel's width, not cut in the middle of a line; the chat scrolls above it, the box and Gönder stay in reach.

use kentos_agent::{AgentProfile, KeySource, ProviderKind};

use crate::app::{App, Message};

/// The sample drawing with an agent (no provider is asked), Asistan open.
fn with_agent() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    let mut agent = AgentProfile::new(ProviderKind::OpenAiCompatible, "Deneme ajanı", "deneme-modeli");
    agent.base_url = Some("http://localhost:8000/v1".to_owned());
    agent.key = KeySource::Session;
    let store = &app.assistant.store;
    let agent = store.add(agent).expect("added");
    store.set_default(&agent.id).expect("the default");
    app.assistant.refresh();
    let _ = app.update(Message::Run("view.assistant"));
    app
}

/// The pictures (`.run/gui-evidence/kullanim-9337/`): Asistan at 1440 and 1100 px, the placeholder whole.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib assistant::narrow_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9337", "03-asistan-dar", with_agent);
}
