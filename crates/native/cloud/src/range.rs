//! Public addresses read by ranges (docs/adr/0207 §1): a COG or a COPC on a
//! web server, read a run of bytes at a time with HTTP `Range`. The probe
//! (`Range: bytes=0-0`) gives the size (`Content-Range`) and the version
//! (`ETag`, else `Last-Modified`); a server that answers anything but
//! `206 Partial Content` is refused with that reason. Passing failures
//! (no connection, 5xx, 30 s without an answer) are tried three times,
//! waiting longer each time. No cookies or proxies: those are the services'
//! (`GIS-10`); an S3 bucket is signed with the user's AWS credentials and a
//! Planetary Computer blob takes its SAS token (`access`, docs/adr/0644 §1).
//! Blocking calls, for the desktop's reading threads; the requests run on the
//! cloud's runtime.

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

/// A GET of `url` with the access it wants (docs/adr/0644 §1): an S3 bucket's signature, a blob's SAS token (asked
/// for when the plain address is refused).
async fn send(
    c: &Client,
    url: &str,
    range: Option<&str>,
    long: bool,
) -> Result<reqwest::Response, String> {
    let ask = |target: String| {
        let mut rq = c.get(&target);
        if long {
            rq = rq.timeout(Duration::from_secs(3600));
        }
        if let Some(r) = range {
            rq = rq.header(RANGE, r);
        }
        for (k, v) in crate::access::headers(&target, range) {
            rq = rq.header(k, v);
        }
        rq.send()
    };
    let res = ask(crate::access::with_known_sas(url))
        .await
        .map_err(|e| format!("“{url}” okunamadı: {e}."))?;
    if crate::access::wants_sas(url, res.status())
        && let Ok(signed) = crate::access::fresh_sas(c, url).await
    {
        return ask(signed)
            .await
            .map_err(|e| format!("“{url}” okunamadı: {e}."));
    }
    Ok(res)
}

/// Bytes `from` to `to` (inclusive) of `url`; a failure says whether to try again.
async fn get(url: String, from: u64, to: u64) -> Result<Answer, ApiFailure> {
    let c = client().map_err(ApiFailure::local)?;
    let res = send(c, &url, Some(&format!("bytes={from}-{to}")), false)
        .await
        .map_err(|e| ApiFailure::local(e).with_retryable(true))?;
    let status = res.status();
    if status != StatusCode::PARTIAL_CONTENT {
        if let Some(words) = crate::access::s3_refusal(&url, status) {
            return Err(ApiFailure::local(words));
        }
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

/// The words of a read refused because the address changed since it was opened (another version): a reader that
/// finds them opens the address again (docs/adr/0642 §2).
pub const CHANGED: &str = "açıldıktan sonra değişmiş";

/// Whether a read's failure is the address having changed since it was opened.
pub fn is_changed(why: &str) -> bool {
    why.contains(CHANGED)
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
            "“{url}” {CHANGED}; yeni sürümü yeniden açılarak okunur."
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

/// The most bytes `download` takes (docs/adr/0642 §3).
pub const DOWNLOAD_MOST: u64 = 8 << 30;

/// `url` whole into the file `to` (written beside it as `<name>.part`, then renamed): for an address that is not read
/// by ranges (docs/adr/0642 §3). At most `most` bytes; `stop` set leaves no file. The bytes written.
pub fn download(
    url: &str,
    to: &std::path::Path,
    most: u64,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<u64, String> {
    let part = to.with_extension(match to.extension() {
        Some(e) => format!("{}.part", e.to_string_lossy()),
        None => "part".to_owned(),
    });
    let work = fetch_to(url.to_owned(), part.clone(), most, stop);
    let result = crate::runtime::wait(async move { work.await.map_err(ApiFailure::local) })
        .map_err(|f| f.message.clone());
    match result {
        Ok(n) => {
            std::fs::rename(&part, to)
                .map_err(|e| format!("“{}” yazılamadı: {e}.", to.display()))?;
            Ok(n)
        }
        Err(why) => {
            let _ = std::fs::remove_file(&part);
            Err(why)
        }
    }
}

async fn fetch_to(
    url: String,
    part: std::path::PathBuf,
    most: u64,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<u64, String> {
    use std::io::Write;
    use std::sync::atomic::Ordering;
    let c = client()?;
    let mut res = send(c, &url, None, true)
        .await
        .map_err(|e| e.replace("okunamadı", "indirilemedi"))?;
    if !res.status().is_success() {
        if let Some(words) = crate::access::s3_refusal(&url, res.status()) {
            return Err(words);
        }
        return Err(format!(
            "“{url}” indirilemedi: sunucu {} dedi.",
            res.status()
        ));
    }
    let total = res.content_length();
    if total.is_some_and(|t| t > most) {
        return Err(format!(
            "“{url}” {} MB; en çok {} MB indirilir.",
            total.unwrap_or(0) >> 20,
            most >> 20
        ));
    }
    let mut file = std::fs::File::create(&part)
        .map_err(|e| format!("“{}” açılamadı: {e}.", part.display()))?;
    let mut written = 0u64;
    loop {
        if stop.load(Ordering::Relaxed) {
            return Err("İndirme durduruldu.".into());
        }
        match res.chunk().await {
            Ok(Some(bytes)) => {
                written += bytes.len() as u64;
                if written > most {
                    return Err(format!("“{url}” {} MB'tan büyük; indirilmedi.", most >> 20));
                }
                file.write_all(&bytes)
                    .map_err(|e| format!("“{}” yazılamadı: {e}.", part.display()))?;
            }
            Ok(None) => break,
            Err(e) => return Err(format!("“{url}” indirilirken kesildi: {e}.")),
        }
    }
    if total.is_some_and(|t| t != written) {
        return Err(format!(
            "“{url}”: {} bayt bildirildi, {written} bayt geldi.",
            total.unwrap_or(0)
        ));
    }
    file.sync_all()
        .map_err(|e| format!("“{}” yazılamadı: {e}.", part.display()))?;
    Ok(written)
}

/// The most bytes an object of a store (a Zarr chunk, its metadata) may take.
const OBJECT_MOST: usize = 1 << 30;

async fn get_object(url: String) -> Result<Option<Vec<u8>>, ApiFailure> {
    let c = client().map_err(ApiFailure::local)?;
    let res = send(c, &url, None, false)
        .await
        .map_err(|e| ApiFailure::local(e).with_retryable(true))?;
    let status = res.status();
    // An object never written: 404, or an S3 bucket's 403 for a key it does not hold (a bucket that does not let its
    // keys be listed answers so; a refused store's metadata is then missing and says so).
    if status == StatusCode::NOT_FOUND
        || (status == StatusCode::FORBIDDEN && crate::access::s3_refusal(&url, status).is_some())
    {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(
            ApiFailure::local(format!("“{url}” okunamadı: sunucu {status} dedi."))
                .with_retryable(status.is_server_error()),
        );
    }
    if res.content_length().is_some_and(|n| n > OBJECT_MOST as u64) {
        return Err(ApiFailure::local(format!(
            "“{url}” 1 GB'tan büyük; okunmuyor."
        )));
    }
    let bytes = res.bytes().await.map_err(|e| {
        ApiFailure::local(format!("“{url}” okunurken kesildi: {e}.")).with_retryable(true)
    })?;
    Ok(Some(bytes.to_vec()))
}

/// An object at `url` whole; none when the server has none there.
pub fn object(url: &str) -> Result<Option<Vec<u8>>, String> {
    let mut wait = Duration::from_millis(250);
    let mut last = String::new();
    for _ in 0..3 {
        match crate::runtime::wait(get_object(url.to_owned())) {
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

/// The cloud client as a host's network (docs/adr/0645 §5): the desktop and the MCP server register it.
pub struct HttpRemote;

/// An address opened: its size and the version it was opened at.
struct Opened {
    url: String,
    probe: Probe,
}

impl kentos_services::remote::RangeRead for Opened {
    fn size(&self) -> u64 {
        self.probe.size
    }

    fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        let len = len.min(self.probe.size.saturating_sub(offset));
        read(&self.url, offset, len, self.probe.version.as_deref())
    }
}

impl kentos_services::remote::Remote for HttpRemote {
    fn open(&self, url: &str) -> Result<Box<dyn kentos_services::remote::RangeRead>, String> {
        let probe = probe(url)?;
        Ok(Box::new(Opened {
            url: url.to_owned(),
            probe,
        }))
    }

    fn object(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        object(url)
    }
}
