//! The agent's provider layer end to end against a provider on this computer (docs/adr/8056 §4): for each wire format
//! a text answer, a tool call answered and its reasoning sent back verbatim, a refused key, a rate limit, an error
//! inside a stream and a cancel; the quirks of the Chat Completions family; the agents' file in a temporary folder; the
//! model lists and the connection check. No network, no real key, never the user's own files.

use std::future::Future;
use std::time::{Duration, Instant};

use kentos_agent::{
    AgentProfile, Conversation, Event, FailureKind, KeySource, ProviderKind, ToolSpec, Turn,
};
use serde_json::{Value, json};

#[path = "agent/mock.rs"]
mod mock;

#[path = "agent/anthropic.rs"]
mod anthropic;
#[path = "agent/chat.rs"]
mod chat;
#[path = "agent/gemini.rs"]
mod gemini;
#[path = "agent/models.rs"]
mod models;
#[path = "agent/profiles.rs"]
mod profiles;
#[path = "agent/responses.rs"]
mod responses;

/// The key the tests send; the providers here echo it back to prove it never reaches a message.
pub const KEY: &str = "sk-test-SECRET-1234";

/// An agent of `kind` reaching the mock at `base`.
pub fn profile(kind: ProviderKind, model: &str, base: String) -> AgentProfile {
    AgentProfile {
        id: "deneme".into(),
        base_url: Some(base),
        key: KeySource::Session,
        ..AgentProfile::new(kind, "Deneme", model)
    }
}

/// The app's tools as the desktop hands them over: dotted names, optional fields, a default.
pub fn tools() -> Vec<ToolSpec> {
    vec![
        ToolSpec::new(
            "cad.run",
            "Çizimin çekirdeğinin komutunu çalıştırır; yazan komut önce planlanır.",
            json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Komutun kimliği."},
                    "input": {"type": "object", "description": "Komutun girdisi."},
                    "mode": {"type": "string", "enum": ["plan", "execute", "validate"], "default": "plan"},
                    "planId": {"type": "string"}
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        ),
        ToolSpec::new(
            "catalog.search",
            "Sözcüklerle komut arar.",
            json!({
                "type": "object",
                "properties": {"query": {"type": "string"}, "limit": {"type": "integer"}},
                "required": ["query"]
            }),
        ),
    ]
}

/// The turn's events up to its last one, each tool call answered by `answer` at once.
pub fn run(turn: &mut Turn, answer: impl Fn(&str, &Value) -> Result<Value, String>) -> Vec<Event> {
    let until = Instant::now() + Duration::from_secs(20);
    let mut out = Vec::new();
    loop {
        let event = next(turn, until);
        if let Event::ToolCall { id, name, input } = &event {
            turn.answer(id, answer(name, input));
        }
        let end = matches!(event, Event::Finished { .. } | Event::Failed { .. });
        out.push(event);
        if end {
            return out;
        }
    }
}

/// The turn's next event, waited for on this thread (no runtime here, as on the desktop's executor).
pub fn next(turn: &mut Turn, until: Instant) -> Event {
    use tokio::sync::mpsc::error::TryRecvError;
    loop {
        match turn.events.try_recv() {
            Ok(e) => return e,
            Err(TryRecvError::Empty) => {
                assert!(Instant::now() < until, "the turn gave no event in time");
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(TryRecvError::Disconnected) => panic!("the turn ended without its last event"),
        }
    }
}

/// Answers nothing: for turns without tool calls.
pub fn no_tools(name: &str, _: &Value) -> Result<Value, String> {
    panic!("no tool call was expected, “{name}” came")
}

pub fn texts(events: &[Event]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect()
}

pub fn reasoning(events: &[Event]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Reasoning(t) => Some(t.as_str()),
            _ => None,
        })
        .collect()
}

pub fn usage(events: &[Event]) -> Vec<(u64, u64)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Usage {
                input_tokens,
                output_tokens,
            } => Some((*input_tokens, *output_tokens)),
            _ => None,
        })
        .collect()
}

pub fn calls(events: &[Event]) -> Vec<(String, String, Value)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::ToolCall { id, name, input } => Some((id.clone(), name.clone(), input.clone())),
            _ => None,
        })
        .collect()
}

pub fn finished(events: &[Event]) -> Conversation {
    match events.last() {
        Some(Event::Finished { conversation }) => conversation.clone(),
        other => panic!("the turn did not finish: {other:?}\n{events:#?}"),
    }
}

pub fn failed(events: &[Event]) -> (FailureKind, String, Option<Duration>) {
    match events.last() {
        Some(Event::Failed {
            kind,
            message,
            retry_after,
        }) => {
            assert!(!message.contains(KEY), "the key in a message: {message}");
            (*kind, message.clone(), *retry_after)
        }
        other => panic!("the turn did not fail: {other:?}\n{events:#?}"),
    }
}

/// Named server-sent events (`event:` + `data:`).
pub fn named(events: &[(&str, Value)]) -> Vec<String> {
    events
        .iter()
        .map(|(name, data)| format!("event: {name}\ndata: {data}\n\n"))
        .collect()
}

/// Data-only server-sent events, `[DONE]` after them when `done`.
pub fn data(values: &[Value], done: bool) -> Vec<String> {
    let mut out: Vec<String> = values.iter().map(|v| format!("data: {v}\n\n")).collect();
    if done {
        out.push("data: [DONE]\n\n".into());
    }
    out
}

/// No sampling parameter anywhere at the top of a body (the research note's traps 1, 23).
pub fn no_sampling(body: &Value) {
    for k in [
        "temperature",
        "top_p",
        "top_k",
        "topP",
        "topK",
        "presence_penalty",
        "frequency_penalty",
        "generationConfig",
    ] {
        assert!(body.get(k).is_none(), "“{k}” sent: {body}");
    }
}

/// Waits for a future on this thread, with no runtime in it (as the desktop's executor).
pub fn block_on<F: Future>(f: F) -> F::Output {
    use std::pin::pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    struct Thread(std::thread::Thread);
    impl Wake for Thread {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Thread(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut f = pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::park();
    }
}

/// Starts a turn and waits until its first text, then cancels it: it fails as cancelled, at once.
pub fn cancel_after_first_text(mut turn: Turn) {
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        match next(&mut turn, until) {
            Event::Text(_) => break,
            Event::Failed { message, .. } => panic!("failed before its first text: {message}"),
            _ => {}
        }
    }
    let asked = Instant::now();
    turn.cancel();
    let rest = run(&mut turn, no_tools);
    let (kind, message, _) = failed(&rest);
    assert_eq!(kind, FailureKind::Cancelled, "{message}");
    assert!(
        asked.elapsed() < Duration::from_secs(3),
        "the cancel took {:?}",
        asked.elapsed()
    );
    // Only the user's message was taken; nothing of the cut answer.
    let kept = turn.conversation();
    assert_eq!(kept.messages.len(), 1, "{kept:#?}");
}
