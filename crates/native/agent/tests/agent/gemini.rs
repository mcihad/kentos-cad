//! Gemini's `streamGenerateContent` against a provider on this computer: thought signatures and the parts they ride on
//! sent back exactly as they came, in their own key order.

use std::time::Duration;

use kentos_agent::{
    Conversation, FailureKind, Message, Part, ProviderKind, Role, ToolCall, ToolResult, start_turn,
};
use serde_json::json;

use super::mock::{Mock, Reply};
use super::*;

const SIGNED_CALL: &str = r#"{"functionCall":{"id":"fc-7","name":"cad__run","args":{"mode":"plan","command":"cad.lines.create"}},"thoughtSignature":"R2VtaW5pU2lnbmF0dXJl"}"#;
const THOUGHT: &str = r#"{"text":"Katmanlara bakılacak.","thought":true}"#;

fn text_answer(pieces: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, p) in pieces.iter().enumerate() {
        let last = i + 1 == pieces.len();
        let mut part = json!({"text": p});
        if last {
            part["thoughtSignature"] = json!("c2lnMQ==");
        }
        let mut candidate = json!({"content": {"parts": [part], "role": "model"}, "index": 0});
        if last {
            candidate["finishReason"] = json!("STOP");
        }
        out.extend(data(
            &[json!({"candidates": [candidate],
                "usageMetadata": {"promptTokenCount": 12, "candidatesTokenCount": 2 + 6 * i, "totalTokenCount": 14 + 6 * i},
                "modelVersion": "gemini-4-carbon"})],
            false,
        ));
    }
    out
}

fn tool_call_answer() -> Vec<String> {
    vec![format!(
        "data: {{\"candidates\":[{{\"content\":{{\"parts\":[{THOUGHT},{SIGNED_CALL}],\"role\":\"model\"}},\"finishReason\":\"STOP\",\"index\":0}}],\"usageMetadata\":{{\"promptTokenCount\":40,\"candidatesTokenCount\":10,\"thoughtsTokenCount\":5,\"totalTokenCount\":55}}}}\r\n\r\n"
    )]
}

fn agent(mock: &Mock) -> kentos_agent::AgentProfile {
    profile(
        ProviderKind::Gemini,
        "gemini-4-carbon",
        format!("{}/v1beta", mock.base),
    )
}

#[test]
fn a_text_answer_streams_and_the_request_is_gemini_s_own() {
    let mock = Mock::start(vec![Reply::stream(text_answer(&[
        "Merhaba",
        ", çizim hazır.",
    ]))]);
    let mut turn = start_turn(
        &agent(&mock),
        Some(KEY.into()),
        Conversation::new("CAD asistanı"),
        tools(),
        "Merhaba".into(),
    );
    let events = run(&mut turn, no_tools);
    assert_eq!(texts(&events), "Merhaba, çizim hazır.");
    assert_eq!(usage(&events), [(12, 8)]);
    let c = finished(&events);
    assert_eq!(c.messages[1].text(), "Merhaba, çizim hazır.");
    let r = &mock.requests()[0];
    assert_eq!(
        r.path,
        "/v1beta/models/gemini-4-carbon:streamGenerateContent?alt=sse"
    );
    assert_eq!(r.header("x-goog-api-key"), Some(KEY));
    assert!(!r.path.contains(KEY), "never the key in the address");
    let body = r.json();
    no_sampling(&body);
    assert_eq!(
        body["contents"],
        json!([{"role": "user", "parts": [{"text": "Merhaba"}]}])
    );
    assert_eq!(
        body["systemInstruction"],
        json!({"parts": [{"text": "CAD asistanı"}]})
    );
    assert_eq!(
        body["toolConfig"],
        json!({"functionCallingConfig": {"mode": "AUTO"}})
    );
    let f = &body["tools"][0]["functionDeclarations"][0];
    assert_eq!(f["name"], "cad__run");
    assert!(f.get("parameters").is_none());
    assert_eq!(
        f["parametersJsonSchema"]["properties"]["mode"],
        json!({"type": "string", "enum": ["plan", "execute", "validate"]})
    );
}

#[test]
fn a_signed_call_is_answered_and_its_parts_go_back_exactly() {
    let mock = Mock::start(vec![
        Reply::stream(tool_call_answer()),
        Reply::stream(text_answer(&["Plan hazır."])),
    ]);
    let mut turn = start_turn(
        &agent(&mock),
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Bir çizgi çiz.".into(),
    );
    let events = run(&mut turn, |name, input| {
        assert_eq!(name, "cad.run");
        assert_eq!(
            input,
            &json!({"mode": "plan", "command": "cad.lines.create"})
        );
        Ok(json!({"planId": "p1"}))
    });
    assert_eq!(reasoning(&events), "Katmanlara bakılacak.");
    assert_eq!(calls(&events)[0].0, "fc-7");
    assert_eq!(usage(&events)[0], (40, 15));
    let c = finished(&events);
    assert_eq!(
        c.messages.iter().map(|m| m.role).collect::<Vec<_>>(),
        [Role::User, Role::Assistant, Role::Tool, Role::Assistant]
    );
    let second = &mock.requests()[1];
    let model_turn = format!(r#"{{"role":"model","parts":[{THOUGHT},{SIGNED_CALL}]}}"#);
    assert!(
        second.body.contains(&model_turn),
        "the model's parts as they came:\n{}",
        second.body
    );
    let body = second.json();
    assert_eq!(
        body["contents"][2],
        json!({"role": "user", "parts": [{"functionResponse": {"id": "fc-7", "name": "cad__run", "response": {"planId": "p1"}}}]})
    );
}

#[test]
fn a_call_without_an_id_and_another_providers_calls_go_as_gemini_takes_them() {
    let unnamed = r#"data: {"candidates":[{"content":{"parts":[{"functionCall":{"name":"catalog__search","args":{"query":"alan"}}}],"role":"model"},"finishReason":"STOP","index":0}]}"#;
    let mock = Mock::start(vec![
        Reply::stream(vec![format!("{unnamed}\n\n")]),
        Reply::stream(text_answer(&["Buldum."])),
    ]);
    // A conversation another provider began: its call goes with the placeholder signature.
    let mut c = Conversation::new("CAD");
    c.push_user("Katmanları say.");
    let call = ToolCall {
        id: "call_00_x".into(),
        name: "cad.run".into(),
        input: json!({"command": "cad.layers.list"}),
    };
    c.messages
        .push(Message::assistant(vec![Part::ToolCall(call.clone())]));
    c.messages
        .push(Message::tool_results(vec![ToolResult::from_answer(
            &call,
            Err("Yok.".into()),
        )]));
    let mut turn = start_turn(
        &agent(&mock),
        Some(KEY.into()),
        c,
        tools(),
        "Alanı bul.".into(),
    );
    let events = run(&mut turn, |_, _| Ok(json!(["cad.regions.create"])));
    let (id, name, _) = calls(&events)[0].clone();
    assert!(!id.is_empty());
    assert_eq!(name, "catalog.search");
    let first = mock.requests()[0].json();
    assert_eq!(
        first["contents"][1],
        json!({"role": "model", "parts": [{"functionCall": {"id": "call_00_x", "name": "cad__run", "args": {"command": "cad.layers.list"}},
            "thoughtSignature": "skip_thought_signature_validator"}]})
    );
    // The error result, and the user's words after it, in one user turn.
    assert_eq!(
        first["contents"][2],
        json!({"role": "user", "parts": [
            {"functionResponse": {"id": "call_00_x", "name": "cad__run", "response": {"error": "Yok."}}},
            {"text": "Alanı bul."}
        ]})
    );
    let second = mock.requests()[1].json();
    let response = &second["contents"][4]["parts"][0]["functionResponse"];
    assert!(
        response.get("id").is_none(),
        "no id was given, none goes back: {response}"
    );
    assert_eq!(
        response["response"],
        json!({"output": ["cad.regions.create"]})
    );
}

#[test]
fn a_refused_key_is_an_auth_failure_that_never_shows_the_key() {
    let mock = Mock::start(vec![Reply::json(
        401,
        &[],
        json!({"error": {"code": 401, "message": format!("Request had invalid authentication credentials: {KEY}"), "status": "UNAUTHENTICATED"}}),
    )]);
    let mut turn = start_turn(
        &agent(&mock),
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
        json!({"error": {"code": 429, "message": "Resource has been exhausted.", "status": "RESOURCE_EXHAUSTED"}}),
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
    let mut pieces = text_answer(&["Yar", "ım"]);
    pieces.truncate(1);
    pieces.extend(data(
        &[json!({"error": {"code": 500, "message": "Internal error encountered.", "status": "INTERNAL"}})],
        false,
    ));
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
    assert!(message.contains("Internal error encountered."), "{message}");
}

#[test]
fn an_answer_without_its_finish_reason_is_cut() {
    let mut pieces = text_answer(&["Yar", "ım"]);
    pieces.truncate(1);
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
    let mut pieces = text_answer(&["Başlıyorum", "…"]);
    pieces.truncate(1);
    let mock = Mock::start(vec![Reply::hang(pieces)]);
    cancel_after_first_text(start_turn(
        &agent(&mock),
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    ));
}
