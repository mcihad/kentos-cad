//! Why a call to a provider failed (docs/adr/8056 §4), sorted as the research note's “Hata sınıflandırması” sorts it:
//! the kind says what the desktop can do (go to the settings, wait, try again, show the details), the message says the
//! cause and the fix in Turkish. A message never carries the API key: the provider's own words are scrubbed of it
//! ([`scrub`]) and cut short.

use std::fmt;
use std::time::Duration;

use reqwest::header::HeaderMap;
use serde_json::Value;

/// What kind of failure it is, for what the desktop offers next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FailureKind {
    /// The key is missing, refused or not allowed this request (401, 403): the settings.
    Auth,
    /// A rate limit or a used-up quota (429, 402); `retry_after` when waiting helps.
    RateLimit,
    /// The provider is busy or failed on its side (5xx, 529, an overloaded 429): try again later.
    Overloaded,
    /// The request was refused as it is (400, 404, 413, 422 …), or there is nothing to send: the details.
    BadRequest,
    /// No answer: no connection, a timeout, a wrong address.
    Network,
    /// The answer broke off or reported an error part way (an error inside a 200 stream).
    Stream,
    /// The user stopped it.
    Cancelled,
}

/// A failed call: its kind, the message for the user and how long the provider asks to wait.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub kind: FailureKind,
    pub message: String,
    pub retry_after: Option<Duration>,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Failure {}

impl Failure {
    pub fn new(kind: FailureKind, message: impl Into<String>) -> Failure {
        Failure {
            kind,
            message: message.into(),
            retry_after: None,
        }
    }

    pub(crate) fn cancelled() -> Failure {
        Failure::new(FailureKind::Cancelled, "Durduruldu.")
    }

    pub(crate) fn bad_request(message: impl Into<String>) -> Failure {
        Failure::new(FailureKind::BadRequest, message)
    }

    /// The answer broke off before its end.
    pub(crate) fn cut(who: &str) -> Failure {
        Failure::new(
            FailureKind::Stream,
            format!("{who} yanıtı yarıda kesildi; yanıtın sonu gelmedi. Yeniden deneyin."),
        )
    }

    /// The provider reported an error inside its answer (an `error` event, `response.failed`, an `error` field).
    pub(crate) fn in_stream(who: &str, what: &str, key: Option<&str>) -> Failure {
        Failure::new(
            FailureKind::Stream,
            format!(
                "{who} yanıt verirken hata bildirdi: {}. Yeniden deneyin.",
                clip(&scrub(what, key))
            ),
        )
    }

    /// A tool call whose input is not JSON.
    pub(crate) fn broken_call(who: &str) -> Failure {
        Failure::new(
            FailureKind::Stream,
            format!("{who} bozuk bir araç çağrısı gönderdi (girdisi JSON değil). Yeniden deneyin."),
        )
    }

    /// The conversation no longer fits the model.
    pub(crate) fn context_full() -> Failure {
        Failure::bad_request(
            "Konuşma modelin bağlam penceresini aştı. Yeni bir sohbet başlatın ya da daha büyük bağlamlı bir model seçin.",
        )
    }

    /// No key for a provider that needs one.
    pub(crate) fn no_key(agent: &str, env: Option<&str>) -> Failure {
        let more = env.map_or(String::new(), |e| {
            format!(" ya da {e} ortam değişkenini tanımlayın")
        });
        Failure::new(
            FailureKind::Auth,
            format!(
                "“{agent}” ajanının API anahtarı yok. Ayarlar → Yapay zekâ ajanları'ndan anahtarı girin{more}."
            ),
        )
    }

    /// A key a header cannot carry (a line break, a non-ASCII letter).
    pub(crate) fn unusable_key() -> Failure {
        Failure::new(
            FailureKind::Auth,
            "API anahtarı geçersiz bir karakter içeriyor (satır sonu ya da Türkçe harf gibi). Anahtarı sağlayıcının \
             panelinden yeniden kopyalayıp yapıştırın.",
        )
    }

    /// No answer came: no connection, a timeout, a cut connection.
    pub(crate) fn unreachable(who: &str, host: &str, local: bool, e: &reqwest::Error) -> Failure {
        let message = if e.is_timeout() {
            format!(
                "{who} ({host}) zamanında yanıt vermedi. Bağlantınızı denetleyip yeniden deneyin."
            )
        } else if e.is_connect() && local {
            format!(
                "{who} {host} adresinde yanıt vermiyor. Programın açık ve sunucusunun başlatılmış olduğunu denetleyin."
            )
        } else if e.is_connect() {
            format!(
                "{who} sunucusuna ({host}) bağlanılamadı. İnternet bağlantınızı ve sunucu adresini denetleyin."
            )
        } else {
            format!(
                "{who} ({host}) ile bağlantı kesildi: {}. Yeniden deneyin.",
                chain(e)
            )
        };
        Failure::new(FailureKind::Network, message)
    }

    /// The answer's body stopped coming part way.
    pub(crate) fn body_cut(who: &str, e: &reqwest::Error) -> Failure {
        let message = if e.is_timeout() {
            format!("{who} uzun süre yanıt göndermedi ve bağlantı kapandı. Yeniden deneyin.")
        } else {
            format!(
                "{who} yanıtı yarıda kesildi: {}. Yeniden deneyin.",
                chain(e)
            )
        };
        Failure::new(FailureKind::Stream, message)
    }
}

/// An error answer's status, headers and body as a failure. The body is read for the provider's code and message:
/// `{"error":{"type"|"code"|"status","message"}}` (most), `{"type":"error","error":{…}}` (Anthropic), `{"error":"…"}`
/// (Ollama), a top-level `message`/`detail` (Mistral).
pub(crate) fn from_answer(
    who: &str,
    status: u16,
    headers: &HeaderMap,
    body: &[u8],
    key: Option<&str>,
) -> Failure {
    let said = Said::read(body);
    let detail = said
        .message
        .as_deref()
        .map(|m| clip(&scrub(m, key)))
        .filter(|m| !m.is_empty());
    let words = |prefix: &str| {
        detail
            .as_ref()
            .map_or(String::new(), |d| format!("{prefix}{d}"))
    };
    let codes = said.codes.join(" ");
    let quota = QUOTA.iter().any(|m| codes.contains(m));
    let overloaded = OVERLOADED.iter().any(|m| codes.contains(m));
    let refused_key = REFUSED_KEY.iter().any(|m| codes.contains(m));
    let wait = retry_after(headers);
    let (kind, message) = match status {
        _ if status == 401 || (refused_key && status != 403) => (
            FailureKind::Auth,
            format!(
                "{who} API anahtarını kabul etmedi ({status}). Ayarlar → Yapay zekâ ajanları'ndan bu ajanın anahtarını \
                 denetleyin.{}",
                words(" Sağlayıcının iletisi: ")
            ),
        ),
        403 => (
            FailureKind::Auth,
            format!(
                "{who} bu isteğe izin vermedi (403). Anahtarın yetkisini, hesabın bölgesini ya da IP izinlerini \
                 denetleyin.{}",
                words(" Sağlayıcının iletisi: ")
            ),
        ),
        _ if quota || status == 402 => (
            FailureKind::RateLimit,
            format!(
                "{who} hesabının kotası ya da bakiyesi bitmiş ({status}). Beklemek çözmez; sağlayıcının panelinden \
                 bakiyeyi ve harcama sınırını denetleyin.{}",
                words(" Sağlayıcının iletisi: ")
            ),
        ),
        _ if overloaded || status == 529 || status >= 500 || status == 408 => (
            FailureKind::Overloaded,
            format!(
                "{who} şu an yoğun ya da geçici bir sunucu hatası verdi ({status}); biraz sonra yeniden deneyin."
            ),
        ),
        429 => (
            FailureKind::RateLimit,
            match wait {
                Some(w) => format!(
                    "{who} istek sınırına ulaşıldı (429); {} sn sonra yeniden deneyin.",
                    w.as_secs_f64().ceil() as u64
                ),
                None => {
                    format!("{who} istek sınırına ulaşıldı (429); biraz bekleyip yeniden deneyin.")
                }
            },
        ),
        404 => (
            FailureKind::BadRequest,
            format!(
                "{who} modeli ya da adresi bulamadı (404). Model listesini yenileyip başka bir model seçin ya da sunucu \
                 adresini denetleyin.{}",
                words(" Sağlayıcının iletisi: ")
            ),
        ),
        _ => (
            FailureKind::BadRequest,
            format!("{who} isteği reddetti ({status}){}", words(": ")),
        ),
    };
    let message = if message.ends_with(['.', '!', '?']) {
        message
    } else {
        format!("{message}.")
    };
    Failure {
        kind,
        message,
        retry_after: if quota { None } else { wait },
    }
}

/// Codes and types that say a quota or balance is used up: waiting does not help (the note's trap 7, 13, 27).
const QUOTA: &[&str] = &[
    "insufficient_quota",
    "credit_balance_exhausted",
    "organization_spend_limit_exceeded",
    "project_spend_limit_exceeded",
    "organization_usage_limit_exceeded",
    "enforced_spend_limit_reached",
    "exceeded_current_quota_error",
    "quota_exceeded",
    "billing_error",
    "Throttling.AllocationQuota",
    "1113",
];

/// Codes and types that say the key itself was refused, whatever the status: Gemini answers a wrong key with 400
/// (`API_KEY_INVALID` in its details), Qwen a key of another region with `invalid_api_key`.
const REFUSED_KEY: &[&str] = &[
    "API_KEY_INVALID",
    "invalid_api_key",
    "InvalidApiKey",
    "authentication_error",
    "invalid_authentication_error",
    "incorrect_api_key_error",
];

/// Codes and types that say the provider is overloaded, whatever the status (Kimi and GLM answer 429).
const OVERLOADED: &[&str] = &[
    "overloaded_error",
    "engine_overloaded_error",
    "server_is_overloaded",
    "service_unavailable",
    "UNAVAILABLE",
    "1305",
];

/// What an error body said: every code-like word in it and the message.
#[derive(Debug, Default)]
struct Said {
    codes: Vec<String>,
    message: Option<String>,
}

impl Said {
    fn read(body: &[u8]) -> Said {
        let mut said = Said::default();
        let Ok(v) = serde_json::from_slice::<Value>(body) else {
            let text = String::from_utf8_lossy(body).trim().to_owned();
            if !text.is_empty() && !text.starts_with('<') {
                said.message = Some(text);
            }
            return said;
        };
        let mut word = |v: Option<&Value>| match v {
            Some(Value::String(s)) if !s.is_empty() => said.codes.push(s.clone()),
            Some(Value::Number(n)) => said.codes.push(n.to_string()),
            _ => {}
        };
        let error = &v["error"];
        for field in ["type", "code", "status"] {
            word(error.get(field));
            word(v.get(field));
        }
        word(error.get("details").and_then(|d| d.get("error_code")));
        // Gemini's details are a list, each with its reason (`API_KEY_INVALID`).
        for d in error
            .get("details")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            word(d.get("reason"));
        }
        said.message = match error {
            Value::String(s) => Some(s.clone()),
            Value::Object(_) => error["message"].as_str().map(str::to_owned),
            _ => None,
        }
        .or_else(|| v["message"].as_str().map(str::to_owned))
        .or_else(|| match &v["detail"] {
            Value::String(s) => Some(s.clone()),
            Value::Null => None,
            other => Some(other.to_string()),
        });
        said
    }
}

/// How long the provider asks to wait: `retry-after-ms`, else `retry-after` in seconds (a date is not read).
pub(crate) fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let number = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v >= 0.0)
    };
    number("retry-after-ms")
        .map(|ms| Duration::from_secs_f64(ms / 1000.0))
        .or_else(|| number("retry-after").map(Duration::from_secs_f64))
}

/// `text` without the key: a provider or a proxy may echo the key it was sent.
pub(crate) fn scrub(text: &str, key: Option<&str>) -> String {
    match key.map(str::trim).filter(|k| k.len() >= 4) {
        Some(k) => text.replace(k, "***"),
        None => text.to_owned(),
    }
}

/// At most 400 characters of a provider's words.
fn clip(text: &str) -> String {
    const MAX: usize = 400;
    let text = text.trim();
    match text.char_indices().nth(MAX) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_owned(),
    }
}

/// An error and its causes in one line (a connection error's cause is the useful part).
fn chain(e: &reqwest::Error) -> String {
    let mut out = e.to_string();
    let mut source = std::error::Error::source(e);
    while let Some(s) = source {
        let text = s.to_string();
        if !out.contains(&text) {
            out.push_str(": ");
            out.push_str(&text);
        }
        source = s.source();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_str(v).unwrap());
        }
        h
    }

    #[test]
    fn statuses_and_codes_sort_into_kinds() {
        let none = HeaderMap::new();
        let f = from_answer(
            "Anthropic",
            401,
            &none,
            br#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key sk-ant-SECRET"}}"#,
            Some("sk-ant-SECRET"),
        );
        assert_eq!(f.kind, FailureKind::Auth);
        assert!(!f.message.contains("sk-ant-SECRET"), "{}", f.message);
        assert!(f.message.contains("***"));
        let f = from_answer(
            "OpenAI",
            429,
            &headers(&[("retry-after", "7")]),
            br#"{"error":{"message":"Rate limit reached","type":"requests","code":"rate_limit_exceeded"}}"#,
            None,
        );
        assert_eq!(
            (f.kind, f.retry_after),
            (FailureKind::RateLimit, Some(Duration::from_secs(7)))
        );
        assert!(f.message.contains("7 sn"), "{}", f.message);
        // A used-up quota is a rate limit nobody should wait out.
        let f = from_answer("OpenAI", 429, &headers(&[("retry-after", "3")]), br#"{"error":{"message":"You exceeded your current quota","type":"insufficient_quota","code":"insufficient_quota"}}"#, None);
        assert_eq!((f.kind, f.retry_after), (FailureKind::RateLimit, None));
        let f = from_answer("Anthropic", 429, &none, br#"{"type":"error","error":{"type":"rate_limit_error","message":"spend","details":{"error_code":"enforced_spend_limit_reached"}}}"#, None);
        assert_eq!((f.kind, f.retry_after), (FailureKind::RateLimit, None));
        // Kimi's and GLM's overload comes as 429.
        let f = from_answer(
            "Kimi",
            429,
            &none,
            br#"{"error":{"type":"engine_overloaded_error","message":"busy"}}"#,
            None,
        );
        assert_eq!(f.kind, FailureKind::Overloaded);
        let f = from_answer(
            "GLM",
            429,
            &none,
            br#"{"error":{"code":"1305","message":"busy"}}"#,
            None,
        );
        assert_eq!(f.kind, FailureKind::Overloaded);
        let f = from_answer(
            "GLM",
            429,
            &none,
            br#"{"error":{"code":"1113","message":"balance"}}"#,
            None,
        );
        assert_eq!(f.kind, FailureKind::RateLimit);
        // Gemini refuses a wrong key with 400.
        let f = from_answer("Gemini", 400, &none, br#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"API_KEY_INVALID","domain":"googleapis.com"}]}}"#, None);
        assert_eq!(f.kind, FailureKind::Auth);
        assert!(f.message.contains("(400)"), "{}", f.message);
        assert_eq!(
            from_answer("Anthropic", 529, &none, b"{}", None).kind,
            FailureKind::Overloaded
        );
        assert_eq!(
            from_answer(
                "Gemini",
                503,
                &none,
                br#"{"error":{"code":503,"message":"x","status":"UNAVAILABLE"}}"#,
                None
            )
            .kind,
            FailureKind::Overloaded
        );
        assert_eq!(
            from_answer(
                "DeepSeek",
                402,
                &none,
                br#"{"error":{"message":"Insufficient Balance"}}"#,
                None
            )
            .kind,
            FailureKind::RateLimit
        );
        let f = from_answer(
            "Ollama",
            404,
            &none,
            br#"{"error":"model \"x\" not found"}"#,
            None,
        );
        assert_eq!(f.kind, FailureKind::BadRequest);
        assert!(f.message.contains("model \"x\" not found"), "{}", f.message);
        let f = from_answer(
            "Mistral",
            422,
            &none,
            br#"{"object":"error","message":"Extra inputs are not permitted","type":"invalid_request_error"}"#,
            None,
        );
        assert_eq!(f.kind, FailureKind::BadRequest);
        assert!(f.message.contains("Extra inputs"), "{}", f.message);
        assert!(
            from_answer("X", 400, &none, b"<html>proxy</html>", None)
                .message
                .ends_with("(400).")
        );
    }

    #[test]
    fn retry_after_reads_seconds_and_milliseconds() {
        assert_eq!(
            retry_after(&headers(&[("retry-after", "2.5")])),
            Some(Duration::from_millis(2500))
        );
        assert_eq!(
            retry_after(&headers(&[
                ("retry-after-ms", "1500"),
                ("retry-after", "9")
            ])),
            Some(Duration::from_millis(1500))
        );
        assert_eq!(
            retry_after(&headers(&[(
                "retry-after",
                "Wed, 21 Oct 2026 07:28:00 GMT"
            )])),
            None
        );
        assert_eq!(retry_after(&HeaderMap::new()), None);
    }

    #[test]
    fn long_words_are_cut_on_a_character() {
        let long = "ğ".repeat(500);
        let c = clip(&long);
        assert_eq!(c.chars().count(), 401);
        assert!(c.ends_with('…'));
    }
}
