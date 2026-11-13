//! Model lists and the connection check against a provider on this computer, awaited on a plain executor.

use kentos_agent::{FailureKind, ProviderKind, check, list_models};
use serde_json::json;

use super::mock::{Mock, Reply};
use super::*;

#[test]
fn anthropic_pages_through_its_list() {
    let mock = Mock::start(vec![
        Reply::json(
            200,
            &[],
            json!({"data": [{"id": "claude-opus-5-5"}, {"id": "claude-haiku-5-5"}], "has_more": true,
                "first_id": "claude-opus-5-5", "last_id": "claude-haiku-5-5"}),
        ),
        Reply::json(
            200,
            &[],
            json!({"data": [{"id": "claude-fable-5-2"}], "has_more": false, "last_id": "claude-fable-5-2"}),
        ),
    ]);
    let a = profile(
        ProviderKind::Anthropic,
        "claude-opus-5-5",
        format!("{}/v1", mock.base),
    );
    let list = block_on(list_models(&a, Some(KEY.into()))).unwrap();
    assert_eq!(
        list,
        ["claude-opus-5-5", "claude-haiku-5-5", "claude-fable-5-2"]
    );
    let r = mock.requests();
    assert_eq!(
        (r[0].method.as_str(), r[0].path.as_str()),
        ("GET", "/v1/models?limit=1000")
    );
    assert_eq!(r[1].path, "/v1/models?limit=1000&after_id=claude-haiku-5-5");
    assert_eq!(r[0].header("x-api-key"), Some(KEY));
    assert_eq!(r[0].header("anthropic-version"), Some("2026-10-21"));
}

#[test]
fn gemini_lists_only_what_generates_content_and_openrouter_only_what_uses_tools() {
    let mock = Mock::start(vec![Reply::json(
        200,
        &[],
        json!({"models": [
            {"name": "models/gemini-4-carbon", "supportedGenerationMethods": ["generateContent", "countTokens"]},
            {"name": "models/text-embedding-5", "supportedGenerationMethods": ["embedContent"]}
        ]}),
    )]);
    let g = profile(
        ProviderKind::Gemini,
        "gemini-4-carbon",
        format!("{}/v1beta", mock.base),
    );
    assert_eq!(
        block_on(list_models(&g, Some(KEY.into()))).unwrap(),
        ["gemini-4-carbon"]
    );
    assert_eq!(mock.requests()[0].path, "/v1beta/models?pageSize=1000");
    assert_eq!(mock.requests()[0].header("x-goog-api-key"), Some(KEY));
    let mock = Mock::start(vec![Reply::json(
        200,
        &[],
        json!({"data": [{"id": "openai/gpt-6-astra"}, {"id": "openai/gpt-6-astra"}]}),
    )]);
    let o = profile(
        ProviderKind::OpenRouter,
        "openai/gpt-6-astra",
        format!("{}/api/v1", mock.base),
    );
    assert_eq!(
        block_on(list_models(&o, Some(KEY.into()))).unwrap(),
        ["openai/gpt-6-astra"]
    );
    assert_eq!(
        mock.requests()[0].path,
        "/api/v1/models?supported_parameters=tools"
    );
    assert_eq!(
        mock.requests()[0].header("authorization"),
        Some(format!("Bearer {KEY}").as_str())
    );
}

#[test]
fn providers_without_a_list_give_their_suggestions_without_asking() {
    let a = profile(
        ProviderKind::Glm,
        "glm-5.3",
        "http://127.0.0.1:9/yok".into(),
    );
    assert_eq!(
        block_on(list_models(&a, None)).unwrap(),
        ["glm-5.3", "glm-5.3-flash", "glm-5.3-flashx"]
    );
}

#[test]
fn the_check_proves_the_key_with_the_list_or_a_few_tokens() {
    let mock = Mock::start(vec![Reply::json(
        200,
        &[],
        json!({"object": "list", "data": [{"id": "deepseek-flash"}, {"id": "deepseek-v4-pro"}]}),
    )]);
    let d = profile(ProviderKind::DeepSeek, "deepseek-flash", mock.base.clone());
    let said = block_on(check(&d, Some(KEY.into()))).unwrap();
    assert!(
        said.contains("Bağlantı çalışıyor") && said.contains('2'),
        "{said}"
    );
    let mock = Mock::start(vec![Reply::json(
        200,
        &[],
        json!({"data": [{"id": "deepseek-v4-pro"}]}),
    )]);
    let d = profile(ProviderKind::DeepSeek, "deepseek-flash", mock.base.clone());
    let said = block_on(check(&d, Some(KEY.into()))).unwrap();
    assert!(
        said.contains("deepseek-flash") && said.contains("yok"),
        "a model the provider does not list is said: {said}"
    );
    // GLM lists nothing: a streamed request for a few tokens.
    let mock = Mock::start(vec![Reply::stream(data(
        &[json!({"choices": [{"index": 0, "delta": {"content": "Mer"}}]})],
        true,
    ))]);
    let g = profile(
        ProviderKind::Glm,
        "glm-5.3",
        format!("{}/api/paas/v4", mock.base),
    );
    assert!(
        block_on(check(&g, Some(KEY.into())))
            .unwrap()
            .contains("kabul")
    );
    let body = mock.requests()[0].json();
    assert_eq!(body["max_tokens"], 16);
    assert_eq!(body["stream"], true);
    assert_eq!(mock.requests()[0].path, "/api/paas/v4/chat/completions");
    // A refused key.
    let mock = Mock::start(vec![Reply::json(
        401,
        &[],
        json!({"error": {"message": format!("bad key {KEY}")}}),
    )]);
    let d = profile(ProviderKind::DeepSeek, "deepseek-flash", mock.base.clone());
    let f = block_on(check(&d, Some(KEY.into()))).unwrap_err();
    assert_eq!(f.kind, FailureKind::Auth);
    assert!(!f.message.contains(KEY), "{}", f.message);
    // No key for a provider that needs one: said without a request.
    let f = block_on(check(&d, None)).unwrap_err();
    assert_eq!(f.kind, FailureKind::Auth);
    assert!(f.message.contains("DEEPSEEK_API_KEY"), "{}", f.message);
}

#[test]
fn a_local_server_that_is_not_running_is_said_so() {
    // A port nothing listens on.
    let free = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let a = profile(
        ProviderKind::LmStudio,
        "qwen3-coder",
        format!("http://127.0.0.1:{free}/v1"),
    );
    let f = block_on(list_models(&a, None)).unwrap_err();
    assert_eq!(f.kind, FailureKind::Network);
    assert!(f.message.contains("yanıt vermiyor"), "{}", f.message);
}
