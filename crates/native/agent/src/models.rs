//! A provider's models and a check of an agent's connection (docs/adr/8056 §4; the research note's “Model listesi”):
//! the provider's own list where it has one (Anthropic and Gemini page through theirs, OpenRouter's is cut to the
//! models that use tools), else the preset's suggestions (GLM, Qwen). The check is the cheapest call that proves the key:
//! the list where there is one, else a streamed request for a few tokens. Both run on the agent's runtime and can be
//! awaited on any executor; dropping the future stops the call.

use std::future::Future;

use reqwest::Url;
use serde_json::Value;

use crate::conversation::Conversation;
use crate::failure::{Failure, scrub};
use crate::http;
use crate::profiles::AgentProfile;
use crate::providers::{ProviderKind, Wire};
use crate::runtime;
use crate::tools::ToolNames;
use crate::wire::{self, Call, anthropic};

/// The models `profile`'s provider offers (their ids, as the agent's model takes them).
pub fn list_models(
    profile: &AgentProfile,
    key: Option<String>,
) -> impl Future<Output = Result<Vec<String>, Failure>> + Send + 'static {
    let profile = profile.clone();
    let key = key.map(|k| k.trim().to_owned()).filter(|k| !k.is_empty());
    runtime::run(async move {
        let r = models(&profile, key.as_deref()).await;
        r.map_err(|f| Failure {
            message: scrub(&f.message, key.as_deref()),
            ..f
        })
    })
}

/// Tries `profile`'s connection and key; what to tell the user when it works.
pub fn check(
    profile: &AgentProfile,
    key: Option<String>,
) -> impl Future<Output = Result<String, Failure>> + Send + 'static {
    let profile = profile.clone();
    let key = key.map(|k| k.trim().to_owned()).filter(|k| !k.is_empty());
    runtime::run(async move {
        let r = try_connection(&profile, key.as_deref()).await;
        r.map_err(|f| Failure {
            message: scrub(&f.message, key.as_deref()),
            ..f
        })
    })
}

async fn try_connection(profile: &AgentProfile, key: Option<&str>) -> Result<String, Failure> {
    let preset = profile.preset();
    if preset.lists_models {
        let list = models(profile, key).await?;
        let model = profile.model.trim();
        let listed = list.iter().any(|m| m == model);
        return Ok(if model.is_empty() || listed || list.is_empty() {
            format!(
                "Bağlantı çalışıyor: {} {} model listeledi.",
                preset.name,
                list.len()
            )
        } else {
            format!(
                "Bağlantı çalışıyor, ancak “{model}” modeli {} listesinde yok; model listesinden birini seçin.",
                preset.name
            )
        });
    }
    // No list to ask: a streamed request for a few tokens, taken or refused.
    if preset.needs_key && key.is_none() {
        return Err(Failure::no_key(&profile.name, preset.key_env));
    }
    let endpoint = http::endpoint(profile)?;
    let mut conversation = Conversation::default();
    conversation.push_user("Merhaba");
    let names = std::sync::Arc::new(ToolNames::default());
    let call = Call {
        profile,
        preset,
        base: &endpoint.base,
        key,
        conversation: &conversation,
        tools: &[],
        names: &names,
    };
    let mut request = wire::request(&call);
    // A few tokens are enough to prove the key.
    if let Ok(mut body) = serde_json::from_str::<Value>(&request.body)
        && let Some(map) = body.as_object_mut()
    {
        map.insert("max_tokens".into(), Value::from(16));
        request.body = body.to_string();
    }
    http::probe(&endpoint, request, preset.name, key).await?;
    Ok(format!(
        "Bağlantı çalışıyor: {} anahtarı kabul etti.",
        preset.name
    ))
}

async fn models(profile: &AgentProfile, key: Option<&str>) -> Result<Vec<String>, Failure> {
    let preset = profile.preset();
    if !preset.lists_models {
        return Ok(preset.models.iter().map(|m| (*m).to_owned()).collect());
    }
    if preset.needs_key && key.is_none() {
        return Err(Failure::no_key(&profile.name, preset.key_env));
    }
    let endpoint = http::endpoint(profile)?;
    let who = preset.name;
    let url = |path: &str| {
        Url::parse(&format!("{}/{path}", endpoint.base)).map_err(|_| {
            Failure::bad_request(format!("“{}” bir sunucu adresi değil.", endpoint.base))
        })
    };
    let mut out: Vec<String> = Vec::new();
    match preset.wire {
        Wire::AnthropicMessages => {
            let headers = [("anthropic-version", anthropic::VERSION.to_owned())];
            let secrets: Vec<_> = key
                .map(|k| ("x-api-key", k.to_owned()))
                .into_iter()
                .collect();
            let mut after: Option<String> = None;
            for _ in 0..20 {
                let mut u = url("models")?;
                u.query_pairs_mut().append_pair("limit", "1000");
                if let Some(a) = &after {
                    u.query_pairs_mut().append_pair("after_id", a);
                }
                let page = http::get_json(&endpoint, u, &headers, &secrets, who, key).await?;
                out.extend(ids(&page["data"], "id"));
                after = page["last_id"].as_str().map(str::to_owned);
                if page["has_more"] != true || after.is_none() {
                    break;
                }
            }
        }
        Wire::GeminiNative => {
            let secrets: Vec<_> = key
                .map(|k| ("x-goog-api-key", k.to_owned()))
                .into_iter()
                .collect();
            let mut token: Option<String> = None;
            for _ in 0..20 {
                let mut u = url("models")?;
                u.query_pairs_mut().append_pair("pageSize", "1000");
                if let Some(t) = &token {
                    u.query_pairs_mut().append_pair("pageToken", t);
                }
                let page = http::get_json(&endpoint, u, &[], &secrets, who, key).await?;
                for m in page["models"].as_array().into_iter().flatten() {
                    let generates = m["supportedGenerationMethods"]
                        .as_array()
                        .is_none_or(|methods| methods.iter().any(|x| x == "generateContent"));
                    if let Some(name) = m["name"].as_str().filter(|_| generates) {
                        out.push(name.strip_prefix("models/").unwrap_or(name).to_owned());
                    }
                }
                token = page["nextPageToken"]
                    .as_str()
                    .filter(|t| !t.is_empty())
                    .map(str::to_owned);
                if token.is_none() {
                    break;
                }
            }
        }
        Wire::OpenAiResponses | Wire::ChatCompletions => {
            let mut u = url("models")?;
            if profile.kind == ProviderKind::OpenRouter {
                u.query_pairs_mut()
                    .append_pair("supported_parameters", "tools");
            }
            let secrets: Vec<_> = key
                .map(|k| ("authorization", format!("Bearer {k}")))
                .into_iter()
                .collect();
            let page = http::get_json(&endpoint, u, &[], &secrets, who, key).await?;
            out.extend(ids(&page["data"], "id"));
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|m| seen.insert(m.clone()));
    Ok(out)
}

/// The `field` of every object in a list.
fn ids(list: &Value, field: &str) -> Vec<String> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m[field].as_str().map(str::to_owned))
        .collect()
}
