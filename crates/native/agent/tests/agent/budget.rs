//! The user's token limits in a turn (docs/adr/8056 §6): a model that calls tools again after its answer crossed the
//! conversation's or the day's limit is stopped there, its calls told they did not run; an answer that ends the turn
//! is whole even past the limit.

use kentos_agent::{Role, Spent, ToolResult, start_turn, start_turn_counting};

use super::chat::{recording, text_answer};
use super::mock::{Mock, Reply};
use super::*;

/// The recorded DeepSeek answer (346 + 135 tokens, two calls) crosses a 400-token conversation: no call reaches the
/// desktop, both are told why, nothing more is asked of the provider.
#[test]
fn a_model_calling_tools_past_the_conversations_limit_is_stopped() {
    let mock = Mock::start(vec![Reply::stream(recording())]);
    let a = AgentProfile {
        conversation_tokens: Some(400),
        ..profile(ProviderKind::DeepSeek, "deepseek-flash", mock.base.clone())
    };
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::new("CAD"),
        tools(),
        "Çizimde kapalı alan oluşturan komutu bul.".into(),
    );
    let events = run(&mut turn, no_tools);
    assert!(calls(&events).is_empty(), "no call reaches the desktop");
    assert_eq!(usage(&events), [(346, 135)], "the tokens are still said");
    let (kind, message, _) = failed(&events);
    assert_eq!(kind, FailureKind::Budget);
    assert!(
        message.starts_with("Ajan durduruldu") && message.contains("481 / 400 token"),
        "{message}"
    );
    assert_eq!(mock.requests().len(), 1);
    let kept = turn.conversation();
    let roles: Vec<Role> = kept.messages.iter().map(|m| m.role).collect();
    assert_eq!(
        roles,
        [Role::User, Role::Assistant, Role::Tool],
        "never a call without its result"
    );
    let results: Vec<&ToolResult> = kept.messages[2].tool_results_iter().collect();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| {
        r.is_error
            && r.output
                .as_str()
                .is_some_and(|o| o.contains("token sınırına"))
    }));
}

/// The day's limit counts on from what the day spent before the turn: an answer that ends the turn is whole past it,
/// a model calling tools past it is stopped.
#[test]
fn the_days_limit_counts_on_from_what_the_day_spent() {
    let a = |base: String| AgentProfile {
        daily_tokens: Some(1_000),
        ..profile(ProviderKind::DeepSeek, "deepseek-flash", base)
    };
    let mock = Mock::start(vec![Reply::stream(text_answer("Tamam."))]);
    let mut turn = start_turn_counting(
        &a(mock.base.clone()),
        Some(KEY.into()),
        Conversation::new("CAD"),
        tools(),
        "Merhaba".into(),
        Spent {
            conversation: 0,
            today: 990,
        },
    );
    let events = run(&mut turn, no_tools);
    assert_eq!(
        finished(&events).last_answer().as_deref(),
        Some("Tamam."),
        "an answer that ends the turn is never cut"
    );
    let mock = Mock::start(vec![Reply::stream(recording())]);
    let mut turn = start_turn_counting(
        &a(mock.base.clone()),
        Some(KEY.into()),
        Conversation::new("CAD"),
        tools(),
        "Bul.".into(),
        Spent {
            conversation: 0,
            today: 600,
        },
    );
    let events = run(&mut turn, no_tools);
    let (kind, message, _) = failed(&events);
    assert_eq!(kind, FailureKind::Budget);
    assert!(
        message.contains("bugün") && message.contains("1.081 / 1.000 token"),
        "{message}"
    );
}
