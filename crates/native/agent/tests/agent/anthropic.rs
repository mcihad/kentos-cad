//! Anthropic's Messages API against a provider on this computer: the events as its streaming documentation shows them.

use std::time::Duration;

use kentos_agent::{Conversation, FailureKind, Part, ProviderKind, Role, start_turn};
use serde_json::json;

use super::mock::{Mock, Reply};
use super::*;

fn start() -> Value {
    json!({"type": "message_start", "message": {"id": "msg_01", "type": "message", "role": "assistant", "content": [],
        "model": "claude-opus-5-5", "stop_reason": null, "usage": {"input_tokens": 25, "output_tokens": 1}}})
}

/// A whole text answer in pieces.
pub fn text_answer(text: &[&str]) -> Vec<String> {
    let mut events = vec![
        ("message_start", start()),
        (
            "content_block_start",
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
        ),
        ("ping", json!({"type": "ping"})),
    ];
    for t in text {
        events.push((
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": t}}),
        ));
    }
    events.extend([
        ("content_block_stop", json!({"type": "content_block_stop", "index": 0})),
        ("message_delta", json!({"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": null}, "usage": {"output_tokens": 15}})),
        ("message_stop", json!({"type": "message_stop"})),
    ]);
    named(&events)
}

const SIGNATURE: &str = "EqQBCgIYAhIM1gbcDa9GJwZA2b3hGgxBdjrkzLoky3dl1pkiMOYds";

fn tool_call_answer() -> Vec<String> {
    named(&[
        ("message_start", start()),
        (
            "content_block_start",
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "thinking", "thinking": "", "signature": ""}}),
        ),
        (
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "thinking_delta", "thinking": "Önce katmanlara "}}),
        ),
        (
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "thinking_delta", "thinking": "bakmalıyım."}}),
        ),
        (
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "signature_delta", "signature": SIGNATURE}}),
        ),
        (
            "content_block_stop",
            json!({"type": "content_block_stop", "index": 0}),
        ),
        (
            "content_block_start",
            json!({"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "toolu_01T1x1fJ34qAmk2tNTrN7Up6", "name": "cad__run", "input": {}}}),
        ),
        (
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": ""}}),
        ),
        (
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"mode\": \"plan\", \"comm"}}),
        ),
        (
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "and\": \"cad.lines.create\"}"}}),
        ),
        (
            "content_block_stop",
            json!({"type": "content_block_stop", "index": 1}),
        ),
        (
            "message_delta",
            json!({"type": "message_delta", "delta": {"stop_reason": "tool_use", "stop_sequence": null}, "usage": {"output_tokens": 89}}),
        ),
        ("message_stop", json!({"type": "message_stop"})),
    ])
}

fn agent(mock: &Mock) -> kentos_agent::AgentProfile {
    profile(
        ProviderKind::Anthropic,
        "claude-opus-5-5",
        format!("{}/v1", mock.base),
    )
}

#[test]
fn a_text_answer_streams_and_the_request_is_as_the_api_wants_it() {
    let mock = Mock::start(vec![Reply::stream(text_answer(&[
        "Merhaba",
        ", çizim ",
        "hazır.",
    ]))]);
    let mut turn = start_turn(
        &agent(&mock),
        Some(KEY.into()),
        Conversation::new("Sen bir CAD asistanısın."),
        tools(),
        "Merhaba".into(),
    );
    let events = run(&mut turn, no_tools);
    assert_eq!(texts(&events), "Merhaba, çizim hazır.");
    assert_eq!(usage(&events), [(25, 15)]);
    let c = finished(&events);
    assert_eq!(c.messages.len(), 2);
    assert_eq!(c.messages[1].role, Role::Assistant);
    assert_eq!(c.messages[1].text(), "Merhaba, çizim hazır.");
    assert_eq!(turn.conversation(), c);
    let r = &mock.requests()[0];
    assert_eq!(
        (r.method.as_str(), r.path.as_str()),
        ("POST", "/v1/messages")
    );
    assert_eq!(r.header("x-api-key"), Some(KEY));
    assert_eq!(r.header("anthropic-version"), Some("2026-10-21"));
    let body = r.json();
    no_sampling(&body);
    assert_eq!(body["model"], "claude-opus-5-5");
    assert_eq!(body["stream"], true);
    assert!(body["max_tokens"].as_u64().is_some());
    assert_eq!(body["system"], "Sen bir CAD asistanısın.");
    assert_eq!(
        body["messages"],
        json!([{"role": "user", "content": [{"type": "text", "text": "Merhaba"}]}])
    );
    assert_eq!(body["tool_choice"], json!({"type": "auto"}));
    assert_eq!(body["tools"][0]["name"], "cad__run");
    assert_eq!(body["tools"][1]["name"], "catalog__search");
    assert_eq!(
        body["tools"][0]["input_schema"]["required"],
        json!(["command"])
    );
}

#[test]
fn a_tool_call_in_fragments_is_answered_and_its_thinking_and_input_go_back_verbatim() {
    let mock = Mock::start(vec![
        Reply::stream(tool_call_answer()),
        Reply::stream(text_answer(&["Plan hazır: bir çizgi eklenecek."])),
    ]);
    let mut turn = start_turn(
        &agent(&mock),
        Some(KEY.into()),
        Conversation::new("CAD"),
        tools(),
        "Bir çizgi çiz.".into(),
    );
    let events = run(&mut turn, |name, input| {
        assert_eq!(name, "cad.run");
        assert_eq!(
            input,
            &json!({"mode": "plan", "command": "cad.lines.create"})
        );
        Ok(json!({"planId": "p1", "adds": 1}))
    });
    assert_eq!(reasoning(&events), "Önce katmanlara bakmalıyım.");
    assert_eq!(calls(&events)[0].0, "toolu_01T1x1fJ34qAmk2tNTrN7Up6");
    assert_eq!(texts(&events), "Plan hazır: bir çizgi eklenecek.");
    assert_eq!(usage(&events).len(), 2, "one per request");
    let c = finished(&events);
    let roles: Vec<Role> = c.messages.iter().map(|m| m.role).collect();
    assert_eq!(
        roles,
        [Role::User, Role::Assistant, Role::Tool, Role::Assistant]
    );
    assert!(
        matches!(&c.messages[1].parts[0], Part::Reasoning(r) if r.text == "Önce katmanlara bakmalıyım.")
    );
    // The second request: the assistant's turn as it came, the result first in the next user message.
    let second = &mock.requests()[1];
    assert!(
        second
            .body
            .contains(r#""input":{"mode": "plan", "command": "cad.lines.create"}"#),
        "the input as the model wrote it, in its order: {}",
        second.body
    );
    let body = second.json();
    no_sampling(&body);
    let m = &body["messages"];
    assert_eq!(m[1]["role"], "assistant");
    assert_eq!(
        m[1]["content"][0],
        json!({"type": "thinking", "thinking": "Önce katmanlara bakmalıyım.", "signature": SIGNATURE})
    );
    assert_eq!(
        m[1]["content"][1],
        json!({"type": "tool_use", "id": "toolu_01T1x1fJ34qAmk2tNTrN7Up6", "name": "cad__run",
            "input": {"mode": "plan", "command": "cad.lines.create"}})
    );
    assert_eq!(m[2]["role"], "user");
    assert_eq!(m[2]["content"][0]["type"], "tool_result");
    assert_eq!(
        m[2]["content"][0]["tool_use_id"],
        "toolu_01T1x1fJ34qAmk2tNTrN7Up6"
    );
    let result: Value =
        serde_json::from_str(m[2]["content"][0]["content"].as_str().unwrap()).unwrap();
    assert_eq!(result, json!({"planId": "p1", "adds": 1}));
    assert!(m[2]["content"][0].get("is_error").is_none());
    assert_eq!(m.as_array().unwrap().len(), 3, "ends with the results");
}

#[test]
fn a_refused_key_is_an_auth_failure_that_never_shows_the_key() {
    let mock = Mock::start(vec![Reply::json(
        401,
        &[],
        json!({"type": "error", "error": {"type": "authentication_error", "message": format!("invalid x-api-key {KEY}")},
            "request_id": "req_1"}),
    )]);
    let mut turn = start_turn(
        &agent(&mock),
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    );
    let (kind, message, _) = failed(&run(&mut turn, no_tools));
    assert_eq!(kind, FailureKind::Auth);
    assert!(message.contains("anahtar"), "{message}");
}

#[test]
fn a_rate_limit_says_how_long_to_wait() {
    let mock = Mock::start(vec![Reply::json(
        429,
        &[("retry-after", "7")],
        json!({"type": "error", "error": {"type": "rate_limit_error", "message": "Number of requests has exceeded your rate limit"}}),
    )]);
    let mut turn = start_turn(
        &agent(&mock),
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    );
    let (kind, message, wait) = failed(&run(&mut turn, no_tools));
    assert_eq!(
        (kind, wait),
        (FailureKind::RateLimit, Some(Duration::from_secs(7))),
        "{message}"
    );
}

#[test]
fn an_error_inside_the_stream_is_a_stream_failure() {
    let mut pieces = named(&[
        ("message_start", start()),
        (
            "content_block_start",
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
        ),
        (
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Yar"}}),
        ),
    ]);
    pieces.extend(named(&[(
        "error",
        json!({"type": "error", "error": {"type": "api_error", "message": "Internal server error"}}),
    )]));
    let mock = Mock::start(vec![Reply::stream(pieces)]);
    let mut turn = start_turn(
        &agent(&mock),
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    );
    let events = run(&mut turn, no_tools);
    assert_eq!(texts(&events), "Yar");
    let (kind, message, _) = failed(&events);
    assert_eq!(kind, FailureKind::Stream);
    assert!(message.contains("Internal server error"), "{message}");
}

#[test]
fn a_stream_cut_before_its_end_is_a_stream_failure() {
    let mut pieces = text_answer(&["Yarım"]);
    pieces.truncate(4);
    let mock = Mock::start(vec![Reply::stream(pieces)]);
    let mut turn = start_turn(
        &agent(&mock),
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    );
    assert_eq!(failed(&run(&mut turn, no_tools)).0, FailureKind::Stream);
}

#[test]
fn cancelling_mid_stream_stops_the_turn() {
    let mut pieces = text_answer(&["Başlıyorum"]);
    pieces.truncate(4);
    let mock = Mock::start(vec![Reply::hang(pieces)]);
    let turn = start_turn(
        &agent(&mock),
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    );
    cancel_after_first_text(turn);
}

#[test]
fn no_key_for_a_provider_that_needs_one_is_said_before_anything_is_sent() {
    let mock = Mock::start(vec![]);
    let mut turn = start_turn(
        &agent(&mock),
        None,
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    );
    let (kind, message, _) = failed(&run(&mut turn, no_tools));
    assert_eq!(kind, FailureKind::Auth);
    assert!(message.contains("ANTHROPIC_API_KEY"), "{message}");
    assert!(mock.requests().is_empty());
}
