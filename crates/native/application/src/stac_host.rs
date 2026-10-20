//! The network a STAC search goes through (docs/adr/0528 §2). The product commands stay pure (no runtime, no
//! network): a host that can reach a network (the desktop, the MCP server) registers a fetcher once at start, and
//! `cad.stac.search` asks it. A host that registers none answers `stac_unavailable`.
//!
//! For tests there is a recorded transport: [`Recorded`] answers a request from a folder's `index.json` (method,
//! address, canonical body -> file, or an error) and nothing else, so the shared cases run offline and a request the
//! code builds differently from the independent reference is not found. A host that registered no fetcher answers from
//! one when `KENTOS_STAC_RECORDED` names the folder (the Python tests do; a real run never sets it); the variable is
//! read at each search, so it may be set after the host started.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use kentos_commands::StacMethod;
use kentos_services::stac::{Fetch, FetchError, Request};
use serde::Deserialize;

static FETCH: OnceLock<Box<dyn Fetch>> = OnceLock::new();

/// Registers the host's fetcher; the first registration stays.
pub fn register(fetch: Box<dyn Fetch>) {
    let _ = FETCH.set(fetch);
}

/// The environment variable that names a recorded folder.
pub const RECORDED_ENV: &str = "KENTOS_STAC_RECORDED";

/// Sends `request` through the host's fetcher, or the recorded folder the environment names; none where the host
/// has neither (it reaches no network).
pub fn send(request: &Request) -> Option<Result<Vec<u8>, FetchError>> {
    if let Some(fetch) = FETCH.get() {
        return Some(fetch.send(request));
    }
    let dir = std::env::var_os(RECORDED_ENV).filter(|d| !d.is_empty())?;
    Some(Recorded::open(PathBuf::from(dir)).send(request))
}

#[derive(Deserialize)]
struct Entry {
    method: String,
    url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct Index {
    requests: Vec<Entry>,
}

/// Answers from a folder of recorded answers (`index.json`).
pub struct Recorded {
    dir: PathBuf,
}

impl Recorded {
    pub fn open(dir: impl AsRef<Path>) -> Recorded {
        Recorded {
            dir: dir.as_ref().to_path_buf(),
        }
    }
}

impl Fetch for Recorded {
    fn send(&self, request: &Request) -> Result<Vec<u8>, FetchError> {
        let fail = |message: String| FetchError { message };
        let text = std::fs::read_to_string(self.dir.join("index.json"))
            .map_err(|e| fail(format!("Kayıtlı cevap dizini okunamadı: {e}")))?;
        let index: Index = serde_json::from_str(&text)
            .map_err(|e| fail(format!("Kayıtlı cevap dizini geçersiz: {e}")))?;
        let method = match request.method {
            StacMethod::Get => "GET",
            StacMethod::Post => "POST",
        };
        let entry = index
            .requests
            .iter()
            .find(|e| e.method == method && e.url == request.url && e.body == request.body)
            .ok_or_else(|| {
                fail(format!(
                    "Kayıtlı cevabı olmayan istek: {method} {} {}",
                    request.url,
                    request.body.as_deref().unwrap_or("")
                ))
            })?;
        if let Some(e) = &entry.error {
            return Err(fail(e.clone()));
        }
        let file = entry
            .file
            .as_deref()
            .ok_or_else(|| fail("Kayıtta ne dosya ne hata var.".into()))?;
        std::fs::read(self.dir.join(file)).map_err(|e| fail(format!("{file} okunamadı: {e}")))
    }
}
