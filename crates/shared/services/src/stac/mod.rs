//! STAC discovery and data preparation (docs/adr/0528): what a host asks a STAC API for and how it reads the
//! answer, and how the assets of a chosen item become rasters. A host fetches; nothing here reaches for the
//! network, so the rules run the same on the desktop, in a headless host and (later) the browser, and are tested
//! against recorded answers. The independent reference is `scripts/fixtures/stac_cases.py`
//! (fixtures/stac/v1/cases.json).
//!
//! - [`endpoint`]: an endpoint's address, its origin and the addresses a server's links may lead to.
//! - [`datetime`]: RFC 3339 instants and intervals as a search sends them.
//! - [`href`]: an asset's address without a signature (the lasting place) and whether it had one.
//! - [`request`]: a search's request (method, address, body), a `next` link's request.
//! - [`read`]: an item collection page, an item and its assets read as the contract's types.
//! - [`catalog`]: a catalog's root and its collections read (what the window lists).
//! - [`prepare`]: the assets of an item as rasters, the quality band's mask rule and the preparation record.

pub mod access;
pub mod catalog;
pub mod datetime;
pub mod endpoint;
pub mod href;
pub mod prepare;
pub mod read;
pub mod request;

use kentos_contracts::StacMethod;

/// Why a request or a preparation was refused: the command's code (`kentos_contracts::codes`), its words and the
/// input field it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub code: &'static str,
    pub message: String,
    pub path: Option<String>,
}

impl Problem {
    pub fn new(code: &'static str, message: impl Into<String>, path: &str) -> Problem {
        Problem {
            code,
            message: message.into(),
            path: (!path.is_empty()).then(|| path.to_owned()),
        }
    }
}

/// A request a host sends: the method, the address and a POST's body (compact JSON).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub method: StacMethod,
    pub url: String,
    pub body: Option<String>,
}

/// Why no answer came, in the user's words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchError {
    pub message: String,
}

/// What fetches a STAC request: the desktop's cloud client, a recorded answer in a test. It returns the answer's
/// body when the server answered 2xx, and why not otherwise (a status, a refused connection, a timeout).
pub trait Fetch: Send + Sync {
    fn send(&self, request: &Request) -> Result<Vec<u8>, FetchError>;
}

/// A JSON value as compact text with every object's keys in alphabetical order (the same on every host, and the
/// key under which a recorded answer is found): integral numbers without a point.
pub fn canonical(value: &serde_json::Value) -> String {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = String::from("{");
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                out.push_str(&canonical(&map[*k]));
            }
            out.push('}');
            out
        }
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() && f.is_finite() && f == f.trunc() && f.abs() < 1e15 => {
                format!("{}", f as i64)
            }
            _ => n.to_string(),
        },
        other => other.to_string(),
    }
}
