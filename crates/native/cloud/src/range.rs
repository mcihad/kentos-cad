//! Public addresses read by ranges (docs/adr/0207 §1): a COG or a COPC on a
//! web server, read a run of bytes at a time with HTTP `Range`. The probe
//! (`Range: bytes=0-0`) gives the size (`Content-Range`) and the version
//! (`ETag`, else `Last-Modified`); a server that answers anything but
//! `206 Partial Content` is refused with that reason. Passing failures
//! (no connection, 5xx, 30 s without an answer) are tried three times,
//! waiting longer each time. No credentials, cookies or proxies: those are
//! the services' (`GIS-10`). Blocking calls, for the desktop's reading
//! threads; the requests run on the cloud's runtime.

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::header::{CONTENT_RANGE, ETAG, LAST_MODIFIED, RANGE};
use reqwest::{Client, StatusCode};

use crate::failure::ApiFailure;

/// What the probe tells of an address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probe {
    pub size: u64,
    /// `ETag` or `Last-Modified`, when the server gives one.
    pub version: Option<String>,
}

fn client() -> Result<&'static Client, String> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .user_agent(concat!("KentOS-CAD/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|e| format!("Adres okuyucusu kurulamadı: {e}."))
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// What a range request brought.
struct Answer {
    bytes: Vec<u8>,
    /// Its `Content-Range`.
    range: Option<String>,
    /// The address's version: its `ETag`, else its `Last-Modified`.
    version: Option<String>,
}

/// Bytes `from` to `to` (inclusive) of `url`; a failure says whether to try again.
async fn get(url: String, from: u64, to: u64) -> Result<Answer, ApiFailure> {
    let c = client().map_err(ApiFailure::local)?;
    let res = c
        .get(&url)
        .header(RANGE, format!("bytes={from}-{to}"))
        .send()
        .await
        .map_err(|e| ApiFailure::local(format!("“{url}” okunamadı: {e}.")).with_retryable(true))?;
    let status = res.status();
    if status != StatusCode::PARTIAL_CONTENT {
        let words = if status == StatusCode::OK || status == StatusCode::RANGE_NOT_SATISFIABLE {
            format!(
                "“{url}”: sunucu parça parça okumaya (HTTP Range) izin vermiyor; dosyayı indirip bağlı dosya olarak ekleyin."
            )
        } else {
            format!("“{url}” okunamadı: sunucu {status} dedi.")
        };
        return Err(ApiFailure::local(words).with_retryable(status.is_server_error()));
    }
    let header = |name| {
        res.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let range = header(CONTENT_RANGE);
    let version = header(ETAG).or_else(|| header(LAST_MODIFIED));
    let bytes = res.bytes().await.map_err(|e| {
        ApiFailure::local(format!("“{url}” okunurken kesildi: {e}.")).with_retryable(true)
    })?;
    Ok(Answer {
        bytes: bytes.to_vec(),
        range,
        version,
    })
}

/// Runs a request on the cloud's runtime and waits, trying a passing failure three times.
fn blocking(url: &str, from: u64, to: u64) -> Result<Answer, String> {
    let mut wait = Duration::from_millis(250);
    let mut last = String::new();
    for _ in 0..3 {
        match crate::runtime::wait(get(url.to_owned(), from, to)) {
            Ok(v) => return Ok(v),
            Err(f) if f.retryable => {
                last = f.message.clone();
                std::thread::sleep(wait);
                wait *= 4;
            }
            Err(f) => return Err(f.message.clone()),
        }
    }
    Err(last)
}

/// The size and version of `url`.
pub fn probe(url: &str) -> Result<Probe, String> {
    let Answer { range, version, .. } = blocking(url, 0, 0)?;
    // `bytes 0-0/12345`: the size after the slash.
    let size = range
        .as_deref()
        .and_then(|r| r.rsplit('/').next())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .ok_or_else(|| format!("“{url}”: sunucu dosyanın boyunu (Content-Range) söylemiyor."))?;
    Ok(Probe { size, version })
}

/// `len` bytes of `url` from `offset`; refused when the address changed (another version).
pub fn read(url: &str, offset: u64, len: u64, version: Option<&str>) -> Result<Vec<u8>, String> {
    if len == 0 {
        return Ok(Vec::new());
    }
    let Answer {
        bytes,
        version: now,
        ..
    } = blocking(url, offset, offset + len - 1)?;
    if let (Some(was), Some(now)) = (version, now.as_deref())
        && was != now
    {
        return Err(format!(
            "“{url}” açıldıktan sonra değişmiş; nesneyi yeniden ekleyin ya da çizimi yeniden açın."
        ));
    }
    if bytes.len() as u64 != len {
        return Err(format!(
            "“{url}”: sunucu {len} bayt yerine {} bayt verdi.",
            bytes.len()
        ));
    }
    Ok(bytes)
}
