//! The Responses API of OpenAI and xAI against a provider on this computer: typed events as the streaming reference
//! names them; the stateless request (`store: false`) and its encrypted reasoning items sent back verbatim.

use std::time::Duration;

use kentos_agent::{Conversation, FailureKind, ProviderKind, Role, start_turn};
use serde_json::json;

use super::mock::{Mock, Reply};
use super::*;

fn ev(kind: &str, mut v: Value) -> (String, Value) {
    v["type"] = json!(kind);
    (kind.to_owned(), v)
}

fn stream(events: Vec<(String, Value)>) -> Vec<String> {
    let refs: Vec<(&str, Value)> = events
        .iter()
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    named(&refs)
}

fn message_item(text: &str) -> Value {
    json!({"id": "msg_1", "type": "message", "status": "completed", "role": "assistant",
        "content": [{"type": "output_text", "text": text, "annotations": []}]})
}

fn text_answer(pieces: &[&str]) -> Vec<String> {
    let whole: String = pieces.concat();
    let mut events = vec![
        ev(
            "response.created",
            json!({"response": {"id": "resp_1", "status": "in_progress", "output": []}}),
        ),
        ev(
            "response.output_item.added",
            json!({"output_index": 0, "item": {"id": "msg_1", "type": "message", "status": "in_progress", "role": "assistant", "content": []}}),
        ),
    ];
    for p in pieces {
        events.push(ev(
            "response.output_text.delta",
            json!({"item_id": "msg_1", "output_index": 0, "content_index": 0, "delta": p}),
        ));
    }
    events.push(ev(
        "response.output_item.done",
        json!({"output_index": 0, "item": message_item(&whole)}),
    ));
    events.push(ev(
        "response.completed",
        json!({"response": {"id": "resp_1", "status": "completed", "output": [message_item(&whole)],
            "usage": {"input_tokens": 30, "output_tokens": 8, "total_tokens": 38}}}),
    ));
    stream(events)
}

fn reasoning_item() -> Value {
    json!({"id": "rs_1", "type": "reasoning", "summary": [{"type": "summary_text", "text": "Katmanlara bakılacak."}],
        "encrypted_content": "gAAAAABpENCRYPTED-REASONING-STATE=="})
}

fn call_item() -> Value {
    json!({"id": "fc_1", "type": "function_call", "status": "completed",
        "arguments": "{\"mode\":\"plan\",\"command\":\"cad.lines.create\"}", "call_id": "call_abc", "name": "cad__run"})
}

fn tool_call_answer() -> Vec<String> {
    stream(vec![
        ev(
            "response.created",
            json!({"response": {"id": "resp_2", "status": "in_progress", "output": []}}),
        ),
        ev(
            "response.output_item.added",
            json!({"output_index": 0, "item": {"id": "rs_1", "type": "reasoning", "summary": []}}),
        ),
        ev(
            "response.reasoning_summary_text.delta",
            json!({"item_id": "rs_1", "output_index": 0, "summary_index": 0, "delta": "Katmanlara bakılacak."}),
        ),
        ev(
            "response.output_item.done",
            json!({"output_index": 0, "item": reasoning_item()}),
        ),
        ev(
            "response.output_item.added",
            json!({"output_index": 1, "item": {"id": "fc_1", "type": "function_call", "status": "in_progress", "arguments": "", "call_id": "call_abc", "name": "cad__run"}}),
        ),
        ev(
            "response.function_call_arguments.delta",
            json!({"item_id": "fc_1", "output_index": 1, "delta": "{\"mode\":\"plan\","}),
        ),
        ev(
            "response.function_call_arguments.delta",
            json!({"item_id": "fc_1", "output_index": 1, "delta": "\"command\":\"cad.lines.create\"}"}),
        ),
        ev(
            "response.function_call_arguments.done",
            json!({"item_id": "fc_1", "output_index": 1, "arguments": "{\"mode\":\"plan\",\"command\":\"cad.lines.create\"}"}),
        ),
        ev(
            "response.output_item.done",
            json!({"output_index": 1, "item": call_item()}),
        ),
        ev(
            "response.completed",
            json!({"response": {"id": "resp_2", "status": "completed", "output": [reasoning_item(), call_item()],
                "usage": {"input_tokens": 120, "output_tokens": 40}}}),
        ),
    ])
}

fn agent(kind: ProviderKind, model: &str, mock: &Mock) -> kentos_agent::AgentProfile {
    profile(kind, model, format!("{}/v1", mock.base))
}

#[test]
fn a_text_answer_streams_and_the_request_is_stateless() {
    let mock = Mock::start(vec![Reply::stream(text_answer(&[
        "Merhaba",
        ", çizim hazır.",
    ]))]);
    let a = agent(ProviderKind::OpenAi, "gpt-6-astra", &mock);
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::new("CAD asistanı"),
        tools(),
        "Merhaba".into(),
    );
    let events = run(&mut turn, no_tools);
    assert_eq!(texts(&events), "Merhaba, çizim hazır.");
    assert_eq!(usage(&events), [(30, 8)]);
    let c = finished(&events);
    assert_eq!(c.messages[1].text(), "Merhaba, çizim hazır.");
    let r = &mock.requests()[0];
    assert_eq!(r.path, "/v1/responses");
    assert_eq!(
        r.header("authorization"),
        Some(format!("Bearer {KEY}").as_str())
    );
    let body = r.json();
    no_sampling(&body);
    assert_eq!(body["store"], false);
    assert_eq!(body["include"], json!(["reasoning.encrypted_content"]));
    assert_eq!(body["instructions"], "CAD asistanı");
    assert_eq!(
        body["input"],
        json!([{"role": "user", "content": "Merhaba"}])
    );
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["name"], "cad__run");
    assert_eq!(
        body["tools"][0]["strict"], false,
        "never strict: the app's schemas have optional fields"
    );
    assert_eq!(
        body["tools"][0]["parameters"]["properties"]["mode"]["default"],
        "plan"
    );
}

#[test]
fn a_tool_call_is_answered_and_the_encrypted_reasoning_goes_back_verbatim() {
    let mock = Mock::start(vec![
        Reply::stream(tool_call_answer()),
        Reply::stream(text_answer(&["Plan hazır."])),
    ]);
    let a = agent(ProviderKind::OpenAi, "gpt-6-astra", &mock);
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Bir çizgi çiz.".into(),
    );
    let events = run(&mut turn, |name, input| {
        assert_eq!(name, "cad.run");
        assert_eq!(input["command"], "cad.lines.create");
        Err("Katman kilitli.".into())
    });
    assert_eq!(reasoning(&events), "Katmanlara bakılacak.");
    assert_eq!(calls(&events)[0].0, "call_abc");
    let c = finished(&events);
    assert_eq!(
        c.messages.iter().map(|m| m.role).collect::<Vec<_>>(),
        [Role::User, Role::Assistant, Role::Tool, Role::Assistant]
    );
    let body = mock.requests()[1].json();
    no_sampling(&body);
    let input = body["input"].as_array().unwrap();
    assert_eq!(
        input[0],
        json!({"role": "user", "content": "Bir çizgi çiz."})
    );
    assert_eq!(
        input[1],
        reasoning_item(),
        "the reasoning item with its encrypted content, as it came"
    );
    assert_eq!(
        input[2],
        call_item(),
        "the call with its id and its arguments' text"
    );
    assert_eq!(input[3]["type"], "function_call_output");
    assert_eq!(input[3]["call_id"], "call_abc");
    let output: Value = serde_json::from_str(input[3]["output"].as_str().unwrap()).unwrap();
    assert_eq!(
        output,
        json!({"error": "Katman kilitli."}),
        "an error the model can read"
    );
    assert_eq!(input.len(), 4);
}

#[test]
fn a_refused_key_is_an_auth_failure_that_never_shows_the_key() {
    let mock = Mock::start(vec![Reply::json(
        401,
        &[],
        json!({"error": {"message": format!("Incorrect API key provided: {KEY}."), "type": "invalid_request_error", "code": "invalid_api_key"}}),
    )]);
    let a = agent(ProviderKind::OpenAi, "gpt-6-astra", &mock);
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    );
    assert_eq!(failed(&run(&mut turn, no_tools)).0, FailureKind::Auth);
}

#[test]
fn a_rate_limit_says_how_long_to_wait() {
    let mock = Mock::start(vec![Reply::json(
        429,
        &[("retry-after", "7")],
        json!({"error": {"message": "Rate limit reached for requests", "type": "requests", "code": "rate_limit_exceeded"}}),
    )]);
    // xAI speaks the same format.
    let a = agent(ProviderKind::Xai, "grok-4.7", &mock);
    let mut turn = start_turn(
        &a,
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
    assert_eq!(mock.requests()[0].path, "/v1/responses");
}

#[test]
fn an_error_inside_the_stream_is_a_stream_failure() {
    let mut events = text_answer(&["Yar"]);
    events.truncate(3);
    events.extend(stream(vec![ev(
        "response.failed",
        json!({"response": {"id": "resp_1", "status": "failed", "error": {"code": "server_error", "message": "The model failed to generate a response."}}}),
    )]));
    let mock = Mock::start(vec![Reply::stream(events)]);
    let a = agent(ProviderKind::OpenAi, "gpt-6-astra", &mock);
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    );
    let events = run(&mut turn, no_tools);
    assert_eq!(texts(&events), "Yar");
    let (kind, message, _) = failed(&events);
    assert_eq!(kind, FailureKind::Stream);
    assert!(message.contains("server_error"), "{message}");
}

#[test]
fn cancelling_mid_stream_stops_the_turn() {
    let mut events = text_answer(&["Başlıyorum"]);
    events.truncate(3);
    let mock = Mock::start(vec![Reply::hang(events)]);
    let a = agent(ProviderKind::OpenAi, "gpt-6-astra", &mock);
    cancel_after_first_text(start_turn(
        &a,
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    ));
}
