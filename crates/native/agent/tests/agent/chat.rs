//! The Chat Completions family against a provider on this computer: a real DeepSeek stream replayed (recorded 13
//! November 2026 from `deepseek-flash`, no key in it), the quirk profiles of GLM, Kimi, Mistral, Ollama and LM Studio,
//! OpenRouter's errors, and a conversation that moves from DeepSeek to Anthropic.

use std::time::Duration;

use kentos_agent::{
    Conversation, FailureKind, Message, Part, ProviderKind, Role, ToolCall, ToolResult, start_turn,
};
use serde_json::json;

use super::mock::{Mock, Reply};
use super::*;

const RECORDING: &str = include_str!("../data/deepseek-flash-tool-call.sse");

fn chunk(delta: Value, finish: Option<&str>) -> Value {
    json!({"id": "c1", "object": "chat.completion.chunk", "created": 1, "model": "m",
        "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]})
}

fn text_answer(text: &str) -> Vec<String> {
    let mut pieces = data(
        &[chunk(json!({"role": "assistant", "content": ""}), None)],
        false,
    );
    pieces.push(": keep-alive\n\n".into());
    pieces.extend(data(
        &[
            chunk(json!({"reasoning_content": "Kısa yanıt."}), None),
            chunk(json!({"content": text}), None),
            chunk(json!({"content": ""}), Some("stop")),
            json!({"id": "c1", "object": "chat.completion.chunk", "choices": [],
                "usage": {"prompt_tokens": 20, "completion_tokens": 9, "total_tokens": 29}}),
        ],
        true,
    ));
    pieces
}

/// The recording's events, one piece each, as the provider sent them.
fn recording() -> Vec<String> {
    RECORDING
        .split_inclusive("\n\n")
        .map(str::to_owned)
        .collect()
}

/// The recording's reasoning, put together.
fn recorded_reasoning() -> String {
    RECORDING
        .lines()
        .filter_map(|l| l.strip_prefix("data: "))
        .filter_map(|d| serde_json::from_str::<Value>(d).ok())
        .filter_map(|v| {
            v["choices"][0]["delta"]["reasoning_content"]
                .as_str()
                .map(str::to_owned)
        })
        .collect()
}

#[test]
fn a_text_answer_streams_past_keep_alives_and_a_usage_chunk() {
    let mock = Mock::start(vec![Reply::stream(text_answer("Merhaba, çizim hazır."))]);
    let a = profile(ProviderKind::DeepSeek, "deepseek-flash", mock.base.clone());
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::new("CAD"),
        tools(),
        "Merhaba".into(),
    );
    let events = run(&mut turn, no_tools);
    assert_eq!(texts(&events), "Merhaba, çizim hazır.");
    assert_eq!(reasoning(&events), "Kısa yanıt.");
    assert_eq!(usage(&events), [(20, 9)]);
    finished(&events);
    let r = &mock.requests()[0];
    assert_eq!(r.path, "/chat/completions");
    assert_eq!(
        r.header("authorization"),
        Some(format!("Bearer {KEY}").as_str())
    );
    let body = r.json();
    no_sampling(&body);
    assert_eq!(body["model"], "deepseek-flash");
    assert_eq!(
        body["messages"],
        json!([{"role": "system", "content": "CAD"}, {"role": "user", "content": "Merhaba"}])
    );
    assert_eq!(body["stream_options"], json!({"include_usage": true}));
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(
        body["tools"][1],
        json!({"type": "function", "function": {"name": "catalog__search",
        "description": "Sözcüklerle komut arar.", "parameters": tools()[1].input_schema}})
    );
}

/// The recorded DeepSeek answer: two calls in parallel; their answers; the reasoning sent back with the calls.
fn deepseek_turn() -> (Mock, Conversation) {
    let mock = Mock::start(vec![
        Reply::stream(recording()),
        Reply::stream(text_answer("İki komut buldum.")),
    ]);
    let a = profile(ProviderKind::DeepSeek, "deepseek-flash", mock.base.clone());
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::new("CAD"),
        tools(),
        "Çizimde kapalı alan oluşturan komutu bul.".into(),
    );
    let events = run(&mut turn, |name, input| {
        assert_eq!(name, "catalog.search");
        Ok(json!({"commands": [format!("{} için sonuç", input["query"].as_str().unwrap())]}))
    });
    let c = calls(&events);
    assert_eq!(
        c,
        [
            (
                "call_00_U6LlnerzaOh30ePShR1A1483".to_owned(),
                "catalog.search".to_owned(),
                json!({"query": "kapalı alan oluştur"})
            ),
            (
                "call_01_cypEjJxLuLWbre48CSbM2738".to_owned(),
                "catalog.search".to_owned(),
                json!({"query": "boundary region"})
            ),
        ]
    );
    let first_reasoning: String = {
        let until = events
            .iter()
            .position(|e| matches!(e, Event::ToolCall { .. }))
            .unwrap();
        reasoning(&events[..until])
    };
    assert_eq!(first_reasoning, recorded_reasoning());
    assert_eq!(usage(&events)[0], (346, 135));
    (mock, finished(&events))
}

#[test]
fn a_recorded_deepseek_tool_round_sends_its_reasoning_back_with_the_calls() {
    let (mock, conversation) = deepseek_turn();
    assert_eq!(
        conversation.last_answer().as_deref(),
        Some("İki komut buldum.")
    );
    let second = mock.requests()[1].json();
    no_sampling(&second);
    let m = second["messages"].as_array().unwrap();
    assert_eq!(m.len(), 5, "{second}");
    assert_eq!(m[2]["role"], "assistant");
    assert_eq!(
        m[2]["reasoning_content"],
        recorded_reasoning(),
        "DeepSeek refuses a tool request without it"
    );
    assert_eq!(m[2]["content"], "");
    assert_eq!(
        m[2]["tool_calls"],
        json!([
            {"id": "call_00_U6LlnerzaOh30ePShR1A1483", "type": "function",
                "function": {"name": "catalog__search", "arguments": "{\"query\": \"kapalı alan oluştur\"}"}},
            {"id": "call_01_cypEjJxLuLWbre48CSbM2738", "type": "function",
                "function": {"name": "catalog__search", "arguments": "{\"query\": \"boundary region\"}"}}
        ]),
        "the arguments' text as the model wrote it"
    );
    assert_eq!(m[3]["role"], "tool");
    assert_eq!(m[3]["tool_call_id"], "call_00_U6LlnerzaOh30ePShR1A1483");
    assert!(
        m[3].get("name").is_none(),
        "DeepSeek's results name no tool"
    );
    assert_eq!(m[4]["tool_call_id"], "call_01_cypEjJxLuLWbre48CSbM2738");
    let out: Value = serde_json::from_str(m[4]["content"].as_str().unwrap()).unwrap();
    assert_eq!(out, json!({"commands": ["boundary region için sonuç"]}));
}

#[test]
fn a_conversation_moved_from_deepseek_to_anthropic_goes_without_deepseeks_reasoning() {
    let (_, conversation) = deepseek_turn();
    let mock = Mock::start(vec![Reply::stream(super::anthropic::text_answer(&[
        "Tamam.",
    ]))]);
    let claude = profile(
        ProviderKind::Anthropic,
        "claude-opus-5-5",
        format!("{}/v1", mock.base),
    );
    let moved = conversation.for_profile(&claude);
    assert!(
        moved
            .messages
            .iter()
            .all(|m| m.echo().is_none() && m.reasoning_text().is_empty())
    );
    let mut turn = start_turn(
        &claude,
        Some(KEY.into()),
        moved,
        tools(),
        "Hangisini önerirsin?".into(),
    );
    finished(&run(&mut turn, no_tools));
    let r = &mock.requests()[0];
    assert!(
        !r.body.contains("reasoning_content") && !r.body.contains("thinking"),
        "{}",
        r.body
    );
    let m = r.json()["messages"].clone();
    assert_eq!(m[1]["role"], "assistant");
    assert_eq!(m[1]["content"][0]["type"], "tool_use");
    assert_eq!(m[1]["content"][0]["id"], "call_00_U6LlnerzaOh30ePShR1A1483");
    assert_eq!(
        m[1]["content"][1]["input"],
        json!({"query": "boundary region"})
    );
    // The results and the next words in one user message, the results first.
    assert_eq!(m[2]["role"], "user");
    assert_eq!(
        m[2]["content"][0]["tool_use_id"],
        "call_00_U6LlnerzaOh30ePShR1A1483"
    );
    assert_eq!(
        m[2]["content"][1]["tool_use_id"],
        "call_01_cypEjJxLuLWbre48CSbM2738"
    );
    assert_eq!(m[3]["role"], "assistant");
    assert_eq!(
        m[4],
        json!({"role": "user", "content": [{"type": "text", "text": "Hangisini önerirsin?"}]})
    );
}

/// A conversation another provider wrote: a call with its id, its result, an answer.
fn foreign() -> Conversation {
    let mut c = Conversation::new("CAD");
    c.push_user("Katmanları say.");
    let call = ToolCall {
        id: "toolu_01A09q90qw90lq917835lq9".into(),
        name: "cad.run".into(),
        input: json!({"command": "cad.layers.list"}),
    };
    c.messages.push(Message::assistant(vec![
        Part::Text("Bakıyorum.".into()),
        Part::ToolCall(call.clone()),
    ]));
    c.messages
        .push(Message::tool_results(vec![ToolResult::from_answer(
            &call,
            Ok(json!({"count": 3})),
        )]));
    c.messages
        .push(Message::assistant(vec![Part::Text("3 katman var.".into())]));
    c
}

fn first_body(
    kind: ProviderKind,
    model: &str,
    conversation: Conversation,
    key: Option<&str>,
) -> (Value, super::mock::Recorded) {
    let mock = Mock::start(vec![Reply::stream(text_answer("Tamam."))]);
    let a = profile(kind, model, format!("{}/v1", mock.base));
    let mut turn = start_turn(
        &a,
        key.map(str::to_owned),
        conversation,
        tools(),
        "Teşekkürler.".into(),
    );
    finished(&run(&mut turn, no_tools));
    let r = mock.requests()[0].clone();
    (r.json(), r)
}

#[test]
fn mistral_gets_nine_character_ids_and_named_results() {
    let (body, _) = first_body(
        ProviderKind::Mistral,
        "mistral-large-4",
        foreign(),
        Some(KEY),
    );
    no_sampling(&body);
    let m = body["messages"].as_array().unwrap();
    let id = m[2]["tool_calls"][0]["id"].as_str().unwrap();
    assert_eq!(id.len(), 9);
    assert!(id.chars().all(|c| c.is_ascii_alphanumeric()), "{id}");
    assert_eq!(m[2]["content"], "Bakıyorum.");
    assert_eq!(
        m[2]["tool_calls"][0]["function"],
        json!({"name": "cad__run", "arguments": "{\"command\":\"cad.layers.list\"}"})
    );
    assert_eq!(
        m[3],
        json!({"role": "tool", "tool_call_id": id, "name": "cad__run", "content": "{\"count\":3}"})
    );
    assert!(
        body.get("stream_options").is_none(),
        "Mistral refuses fields it does not know"
    );
    assert_eq!(body["tool_choice"], "auto");
}

#[test]
fn kimi_names_its_results_keeps_ids_and_gets_room_to_think() {
    let (body, _) = first_body(ProviderKind::Kimi, "kimi-k3.1-pro", foreign(), Some(KEY));
    let m = body["messages"].as_array().unwrap();
    assert_eq!(m[2]["tool_calls"][0]["id"], "toolu_01A09q90qw90lq917835lq9");
    assert_eq!(m[3]["name"], "cad__run");
    assert_eq!(body["max_tokens"], 32_000);
    assert_eq!(body["stream_options"], json!({"include_usage": true}));
}

#[test]
fn glm_streams_its_calls_and_local_servers_take_no_tool_choice() {
    let (glm, _) = first_body(
        ProviderKind::Glm,
        "glm-5.3",
        Conversation::default(),
        Some(KEY),
    );
    assert_eq!(
        (glm["tool_stream"].clone(), glm["tool_choice"].clone()),
        (json!(true), json!("auto"))
    );
    assert!(glm.get("stream_options").is_none());
    let (ollama, r) = first_body(
        ProviderKind::Ollama,
        "llama4",
        Conversation::default(),
        None,
    );
    assert!(
        ollama.get("tool_choice").is_none(),
        "Ollama has no tool_choice"
    );
    assert_eq!(ollama["stream_options"], json!({"include_usage": true}));
    assert!(r.header("authorization").is_none(), "no key, no header");
    let (lmstudio, _) = first_body(
        ProviderKind::LmStudio,
        "qwen3-coder",
        Conversation::default(),
        None,
    );
    assert!(lmstudio.get("tool_choice").is_none() && lmstudio.get("stream_options").is_none());
    let (openrouter, r) = first_body(
        ProviderKind::OpenRouter,
        "anthropic/claude-opus-5-5",
        Conversation::default(),
        Some(KEY),
    );
    assert_eq!(openrouter["tool_choice"], "auto");
    assert_eq!(r.header("x-openrouter-title"), Some("KentOS CAD"));
}

#[test]
fn mistral_thinking_chunks_and_whole_arguments_go_back_as_they_came() {
    let first = data(
        &[
            chunk(
                json!({"role": "assistant", "content": [{"type": "thinking", "thinking": [{"type": "text", "text": "Katmanlar "}]}]}),
                None,
            ),
            chunk(
                json!({"content": [{"type": "thinking", "thinking": [{"type": "text", "text": "gerekli."}]}]}),
                None,
            ),
            chunk(
                json!({"tool_calls": [{"id": "D681PevKs", "type": "function", "index": 0,
                    "function": {"name": "cad__run", "arguments": {"command": "cad.layers.list"}}}]}),
                Some("tool_calls"),
            ),
            json!({"choices": [], "usage": {"prompt_tokens": 50, "completion_tokens": 12}}),
        ],
        true,
    );
    let mock = Mock::start(vec![
        Reply::stream(first),
        Reply::stream(text_answer("3 katman var.")),
    ]);
    let a = profile(
        ProviderKind::Mistral,
        "magistral-medium",
        format!("{}/v1", mock.base),
    );
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Katmanları say.".into(),
    );
    let events = run(&mut turn, |_, input| {
        assert_eq!(input, &json!({"command": "cad.layers.list"}));
        Ok(json!({"count": 3}))
    });
    assert_eq!(
        reasoning(&events).split("Kısa").next(),
        Some("Katmanlar gerekli.")
    );
    finished(&events);
    let m = mock.requests()[1].json()["messages"].clone();
    assert_eq!(
        m[1]["content"],
        json!([{"type": "thinking", "thinking": [{"type": "text", "text": "Katmanlar gerekli."}]}])
    );
    assert_eq!(m[1]["tool_calls"][0]["id"], "D681PevKs");
    assert_eq!(
        m[1]["tool_calls"][0]["function"]["arguments"],
        "{\"command\":\"cad.layers.list\"}"
    );
    assert_eq!(m[2]["tool_call_id"], "D681PevKs");
    assert!(
        m[1].get("reasoning_content").is_none(),
        "Mistral's thinking goes back as chunks only"
    );
}

#[test]
fn a_refused_key_is_an_auth_failure_that_never_shows_the_key() {
    let mock = Mock::start(vec![Reply::json(
        401,
        &[],
        json!({"error": {"code": 401, "message": format!("No auth credentials found for {KEY}")}}),
    )]);
    let a = profile(
        ProviderKind::OpenRouter,
        "openai/gpt-6-astra",
        format!("{}/api/v1", mock.base),
    );
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    );
    assert_eq!(failed(&run(&mut turn, no_tools)).0, FailureKind::Auth);
    assert_eq!(mock.requests()[0].path, "/api/v1/chat/completions");
}

#[test]
fn a_rate_limit_says_how_long_to_wait() {
    let mock = Mock::start(vec![Reply::json(
        429,
        &[("retry-after", "7")],
        json!({"error": {"message": "rate limit reached", "type": "rate_limit_reached_error"}}),
    )]);
    let a = profile(
        ProviderKind::Kimi,
        "kimi-k3.1-pro",
        format!("{}/v1", mock.base),
    );
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
}

#[test]
fn an_error_inside_the_stream_is_a_stream_failure() {
    let pieces = data(
        &[
            chunk(json!({"role": "assistant", "content": "Yar"}), None),
            json!({"id": "c1", "object": "chat.completion.chunk", "error": {"code": 502, "message": "Provider returned error"},
                "choices": [{"index": 0, "delta": {"content": ""}, "finish_reason": "error"}]}),
        ],
        false,
    );
    let mock = Mock::start(vec![Reply::stream(pieces)]);
    let a = profile(
        ProviderKind::OpenRouter,
        "openai/gpt-6-astra",
        format!("{}/api/v1", mock.base),
    );
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
    assert!(message.contains("Provider returned error"), "{message}");
}

#[test]
fn cancelling_mid_stream_stops_the_turn() {
    let pieces = data(
        &[chunk(
            json!({"role": "assistant", "content": "Başlıyorum"}),
            None,
        )],
        false,
    );
    let mock = Mock::start(vec![Reply::hang(pieces)]);
    let a = profile(ProviderKind::Ollama, "llama4", format!("{}/v1", mock.base));
    cancel_after_first_text(start_turn(
        &a,
        None,
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    ));
}

#[test]
fn an_unanswered_call_is_told_it_did_not_run_when_the_turn_is_cancelled() {
    let mock = Mock::start(vec![Reply::stream(recording())]);
    let a = profile(ProviderKind::DeepSeek, "deepseek-flash", mock.base.clone());
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Bul.".into(),
    );
    let until = std::time::Instant::now() + Duration::from_secs(20);
    // The first call answered, the second not: then stopped.
    let mut first = None;
    while first.is_none() {
        if let Event::ToolCall { id, .. } = next(&mut turn, until) {
            first = Some(id);
        }
    }
    turn.answer(&first.unwrap(), Ok(json!("bir")));
    std::thread::sleep(Duration::from_millis(50));
    turn.cancel();
    let rest = run(&mut turn, |_, _| Ok(json!("geç")));
    assert_eq!(failed(&rest).0, FailureKind::Cancelled);
    let kept = turn.conversation();
    let roles: Vec<Role> = kept.messages.iter().map(|m| m.role).collect();
    assert_eq!(
        roles,
        [Role::User, Role::Assistant, Role::Tool],
        "never a call without its result"
    );
    let results: Vec<&ToolResult> = kept.messages[2].tool_results_iter().collect();
    assert_eq!(
        (results[0].is_error, &results[0].output),
        (false, &json!("bir"))
    );
    assert!(results[1].is_error);
    assert_eq!(mock.requests().len(), 1);
}

#[test]
fn the_events_can_be_taken_to_wait_on_apart_from_the_turn() {
    use tokio::sync::mpsc::error::TryRecvError;
    let mock = Mock::start(vec![Reply::stream(text_answer("Tamam."))]);
    let a = profile(ProviderKind::DeepSeek, "deepseek-flash", mock.base.clone());
    let mut turn = start_turn(
        &a,
        Some(KEY.into()),
        Conversation::default(),
        tools(),
        "Merhaba".into(),
    );
    let mut events = turn.take_events();
    assert!(
        matches!(turn.events.try_recv(), Err(TryRecvError::Disconnected)),
        "a second take gets a closed channel"
    );
    let until = std::time::Instant::now() + Duration::from_secs(20);
    let mut got = Vec::new();
    while !matches!(got.last(), Some(Event::Finished { .. })) {
        match events.try_recv() {
            Ok(e) => got.push(e),
            Err(_) => {
                assert!(std::time::Instant::now() < until, "{got:#?}");
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }
    assert_eq!(texts(&got), "Tamam.");
    // The turn kept for answering and stopping still holds its conversation.
    assert_eq!(turn.conversation().last_answer().as_deref(), Some("Tamam."));
}
