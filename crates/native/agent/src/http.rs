//! Sending a request to a provider and reading its answer (docs/adr/8056 §4). The address is checked first: `https://`,
//! or plain `http://` to this computer only (the research note's “Yerel sunucular”: a key never crosses a network in
//! clear). Two clients: one for providers, through the system's proxy, and one for this computer's servers, never
//! through a proxy. A stream has no overall time limit, only a limit between two reads (a model may think for
//! minutes; the providers send keep-alives meanwhile). The key goes in headers marked sensitive, never in an address.

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderValue};
use reqwest::{Client, RequestBuilder, Response, Url};
use serde_json::Value;

use crate::failure::{self, Failure, FailureKind};
use crate::profiles::AgentProfile;
use crate::sse::SseParser;
use crate::wire::{Decoder, Live, Reply, Request};

/// Where an agent's requests go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Endpoint {
    /// The address without its trailing `/`.
    pub base: String,
    pub host: String,
    /// On this computer: plain HTTP allowed, no proxy.
    pub local: bool,
}

/// The agent's address, checked.
pub(crate) fn endpoint(profile: &AgentProfile) -> Result<Endpoint, Failure> {
    let Some(base) = profile.endpoint() else {
        return Err(Failure::bad_request(format!(
            "“{}” ajanının sunucu adresi yok. Ayarlardan sunucunun adresini yazın (örneğin http://localhost:8000/v1).",
            profile.name
        )));
    };
    let wrong = || {
        Failure::bad_request(format!(
            "“{base}” bir sunucu adresi değil; adres https:// ile başlamalı (bu bilgisayardaki bir sunucu için \
             http://localhost:…)."
        ))
    };
    let url = Url::parse(&base).map_err(|_| wrong())?;
    let host = url.host_str().unwrap_or_default().to_owned();
    if host.is_empty() {
        return Err(wrong());
    }
    let local = is_loopback(&host);
    match url.scheme() {
        "https" => {}
        "http" if local => {}
        "http" => {
            return Err(Failure::bad_request(format!(
                "Şifresiz bağlantı (http://) yalnız bu bilgisayardaki sunucular için kullanılır; “{host}” için adresi \
                 https:// ile yazın."
            )));
        }
        _ => return Err(wrong()),
    }
    Ok(Endpoint { base, host, local })
}

/// `localhost`, `127.0.0.0/8`, `::1`.
fn is_loopback(host: &str) -> bool {
    let h = host.trim_start_matches('[').trim_end_matches(']');
    h.eq_ignore_ascii_case("localhost")
        || h.parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn client(local: bool) -> Result<&'static Client, Failure> {
    static REMOTE: OnceLock<Result<Client, String>> = OnceLock::new();
    static LOCAL: OnceLock<Result<Client, String>> = OnceLock::new();
    let cell = if local { &LOCAL } else { &REMOTE };
    cell.get_or_init(|| {
        let builder = Client::builder()
            .connect_timeout(Duration::from_secs(if local { 5 } else { 15 }))
            // Between two reads: a model may think long before its first word, a local one on a CPU longer.
            .read_timeout(Duration::from_secs(if local { 600 } else { 300 }))
            .user_agent(concat!("KentOS-CAD/", env!("CARGO_PKG_VERSION")));
        let builder = if local { builder.no_proxy() } else { builder };
        builder.build().map_err(|e| e.to_string())
    })
    .as_ref()
    .map_err(|e| {
        Failure::new(
            FailureKind::Network,
            format!("HTTPS istemcisi kurulamadı: {e}"),
        )
    })
}

/// A request with its headers; the key's marked sensitive.
fn prepare(
    builder: RequestBuilder,
    headers: &[(&'static str, String)],
    secrets: &[(&'static str, String)],
) -> Result<RequestBuilder, Failure> {
    let mut builder = builder;
    for (name, value) in headers {
        builder = builder.header(*name, value.as_str());
    }
    for (name, value) in secrets {
        let mut v = HeaderValue::from_str(value).map_err(|_| Failure::unusable_key())?;
        v.set_sensitive(true);
        builder = builder.header(*name, v);
    }
    Ok(builder)
}

/// Sends and gives the answer when it is a success; an error answer as a failure.
async fn send(
    builder: RequestBuilder,
    endpoint: &Endpoint,
    who: &str,
    key: Option<&str>,
) -> Result<Response, Failure> {
    let response = builder
        .send()
        .await
        .map_err(|e| Failure::unreachable(who, &endpoint.host, endpoint.local, &e))?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let headers = response.headers().clone();
    let body = read_all(response, 64 * 1024).await;
    Err(failure::from_answer(
        who,
        status.as_u16(),
        &headers,
        &body,
        key,
    ))
}

/// At most `cap` bytes of a body; what came before a cut.
async fn read_all(mut response: Response, cap: usize) -> Vec<u8> {
    let mut out = Vec::new();
    while out.len() < cap {
        match response.chunk().await {
            Ok(Some(c)) => out.extend_from_slice(&c),
            _ => break,
        }
    }
    out.truncate(cap);
    out
}

/// Sends a streamed request and reads its events to the end: text and reasoning to `live` as they come, the
/// assistant's message at the end.
pub(crate) async fn stream(
    endpoint: &Endpoint,
    request: Request,
    mut decoder: Box<dyn Decoder>,
    who: &str,
    key: Option<&str>,
    live: &mut (dyn FnMut(Live) + Send),
) -> Result<Reply, Failure> {
    let builder = client(endpoint.local)?
        .post(&request.url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "text/event-stream")
        .body(request.body);
    let builder = prepare(builder, &request.headers, &request.secrets)?;
    let mut response = send(builder, endpoint, who, key).await?;
    let json_answer = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"));
    if json_answer {
        // A whole JSON answer to a streamed request: an error said with a success status, or a server that does not
        // stream.
        let body = read_all(response, 64 * 1024).await;
        let said = serde_json::from_slice::<Value>(&body).ok();
        let error = said.as_ref().map(|v| &v["error"]).filter(|e| !e.is_null());
        return Err(match error {
            Some(e) => {
                Failure::in_stream(who, e["message"].as_str().unwrap_or(&e.to_string()), key)
            }
            None => Failure::new(
                FailureKind::Stream,
                format!(
                    "{who} akış yerine tek parça yanıt verdi; sunucunun akışlı (SSE) yanıt verdiğini denetleyin."
                ),
            ),
        });
    }
    let mut parser = SseParser::default();
    let mut pieces = Vec::new();
    loop {
        let chunk = response
            .chunk()
            .await
            .map_err(|e| Failure::body_cut(who, &e))?;
        let Some(chunk) = chunk else { break };
        for event in parser.push(&chunk) {
            decoder.event(&event, &mut pieces)?;
            pieces.drain(..).for_each(&mut *live);
        }
        if decoder.done() {
            break;
        }
    }
    for event in parser.finish() {
        decoder.event(&event, &mut pieces)?;
        pieces.drain(..).for_each(&mut *live);
    }
    decoder.finish()
}

/// `GET` a JSON document (a model list), with a time limit.
pub(crate) async fn get_json(
    endpoint: &Endpoint,
    url: Url,
    headers: &[(&'static str, String)],
    secrets: &[(&'static str, String)],
    who: &str,
    key: Option<&str>,
) -> Result<Value, Failure> {
    let builder = client(endpoint.local)?
        .get(url)
        .timeout(Duration::from_secs(30));
    let builder = prepare(builder, headers, secrets)?;
    let response = send(builder, endpoint, who, key).await?;
    let body = read_all(response, 16 * 1024 * 1024).await;
    serde_json::from_slice(&body).map_err(|_| {
        Failure::new(
            FailureKind::Stream,
            format!(
                "{who} model listesini okunamayan bir biçimde verdi; sunucu adresini denetleyin."
            ),
        )
    })
}

/// Sends a request and gives back only whether it was taken (a connection check).
pub(crate) async fn probe(
    endpoint: &Endpoint,
    request: Request,
    who: &str,
    key: Option<&str>,
) -> Result<(), Failure> {
    let builder = client(endpoint.local)?
        .post(&request.url)
        .header(CONTENT_TYPE, "application/json")
        .timeout(Duration::from_secs(60))
        .body(request.body);
    let builder = prepare(builder, &request.headers, &request.secrets)?;
    send(builder, endpoint, who, key).await.map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ProviderKind;

    fn at(kind: ProviderKind, url: Option<&str>) -> Result<Endpoint, Failure> {
        endpoint(&AgentProfile {
            base_url: url.map(str::to_owned),
            ..AgentProfile::new(kind, "Ajan", "m")
        })
    }

    #[test]
    fn plain_http_reaches_this_computer_only() {
        let e = at(ProviderKind::Anthropic, None).unwrap();
        assert_eq!(
            (e.base.as_str(), e.local),
            ("https://api.anthropic.com/v1", false)
        );
        assert!(at(ProviderKind::Ollama, None).unwrap().local);
        for ok in [
            "http://127.0.0.1:8080/v1/",
            "http://[::1]:1234/v1",
            "http://LOCALHOST:1/v1",
        ] {
            let e = at(ProviderKind::OpenAiCompatible, Some(ok)).unwrap();
            assert!(e.local && !e.base.ends_with('/'), "{ok}");
        }
        let f = at(
            ProviderKind::OpenAiCompatible,
            Some("http://10.0.0.5:8000/v1"),
        )
        .unwrap_err();
        assert_eq!(f.kind, FailureKind::BadRequest);
        assert!(f.message.contains("https://"), "{}", f.message);
        assert!(
            at(ProviderKind::OpenAiCompatible, None)
                .unwrap_err()
                .message
                .contains("adresi yok")
        );
        assert!(at(ProviderKind::OpenAiCompatible, Some("ftp://x/v1")).is_err());
        assert!(at(ProviderKind::OpenAiCompatible, Some("sunucu")).is_err());
        assert!(
            at(
                ProviderKind::OpenAiCompatible,
                Some("https://gpu.belediye.local/v1")
            )
            .is_ok()
        );
    }
}
