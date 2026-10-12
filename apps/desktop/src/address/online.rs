//! The device's online address providers (docs/adr/0229 §10, §16): their
//! settings in `adres-saglayicilari.json` beside the app's settings
//! (`kentos.address-providers` 1, the services core's rules; a file that
//! does not read is set aside as `adres-saglayicilari-okunamadi-<time>.json`),
//! their keys among the connections' secrets (`baglantilar.json`, under
//! `adres:<provider>`; never in a drawing, the log or the cache's names),
//! the answers kept in `$XDG_CACHE_HOME/kentos-cad/adres/` for as many days
//! as the provider allows (at most 50 MB, the least recently read out
//! first; a provider whose results may not be stored keeps nothing), and
//! each provider's pace (the public Nominatim and Photon one request a
//! second). A request runs on the cloud client's runtime
//! (`kentos_cloud::fetch`) and is waited for on the asking thread: the Adres
//! tab's or İşlemler's, never the interface's.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use kentos_cloud::fetch::{self, Method, Request};
use kentos_contracts::ConnectionSecret;
use kentos_processing::address::AddressHost;
use kentos_services::geocode::providers::{KeyRule, Provider, Providers};
use kentos_services::geocode::request::GeoRequest;
use kentos_services::query::encode;
use serde::{Deserialize, Serialize};

use crate::services::secrets::secrets;

const FILE: &str = "adres-saglayicilari.json";
/// The secrets' origin of the providers' keys: the device's, not a project's.
pub const KEY_ORIGIN: &str = "kentos:adres";
/// Bytes the cache may take before the least recently read answers go.
pub const CACHE_BUDGET: u64 = 50 * 1024 * 1024;

/// A provider's key's id among the secrets.
pub fn key_id(provider: &str) -> String {
    format!("adres:{provider}")
}

/// The device's providers: their settings, keys, paces and kept answers.
pub struct Online {
    folder: Option<PathBuf>,
    cache: Option<PathBuf>,
    settings: Mutex<Providers>,
    /// When each provider may be sent its next request.
    next: Mutex<HashMap<String, Instant>>,
    /// Whether a Google basemap is shown (Google's terms 6.2: its results only beside its map).
    google: AtomicBool,
    /// Answers kept since the last trim.
    puts: AtomicU64,
}

fn config_folder() -> Option<PathBuf> {
    // The tests' app keeps its providers in memory, as its secrets.
    if cfg!(test) {
        return None;
    }
    crate::settings::Settings::config_dir()
}

/// `$XDG_CACHE_HOME/kentos-cad/adres`, else `~/.cache/kentos-cad/adres`.
fn cache_folder() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    crate::services::cache::folder().and_then(|f| f.parent().map(|p| p.join("adres")))
}

/// The app's providers.
pub fn online() -> &'static Arc<Online> {
    static ONLINE: OnceLock<Arc<Online>> = OnceLock::new();
    ONLINE.get_or_init(|| Arc::new(Online::open(config_folder(), cache_folder())))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// FNV-1a, 64 bits: a key stable across runs and builds.
fn hash(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// What a kept answer was: its address (without the key) and when it came.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Kept {
    url: String,
    stored_ms: u64,
}

fn read_settings(folder: &Path) -> Providers {
    let path = folder.join(FILE);
    let Ok(bytes) = std::fs::read(&path) else {
        return Providers::default();
    };
    match serde_json::from_slice::<Providers>(&bytes) {
        Ok(p) if p.format == kentos_services::geocode::providers::FORMAT => p.sanitized(),
        _ => {
            // Never written over: set aside with its time.
            let aside = folder.join(format!("adres-saglayicilari-okunamadi-{}.json", now_ms()));
            let _ = std::fs::rename(&path, aside);
            Providers::default()
        }
    }
}

impl Online {
    /// The providers kept in `folder`, their answers in `cache` (none: in memory only, nothing kept).
    pub fn open(folder: Option<PathBuf>, cache: Option<PathBuf>) -> Online {
        let settings = folder.as_deref().map(read_settings).unwrap_or_default();
        Online {
            folder,
            cache,
            settings: Mutex::new(settings),
            next: Mutex::new(HashMap::new()),
            google: AtomicBool::new(false),
            puts: AtomicU64::new(0),
        }
    }

    /// The device's settings.
    pub fn settings(&self) -> Providers {
        self.settings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Keeps the settings (those that hold) and writes the file.
    pub fn save(&self, providers: Providers) -> Result<(), String> {
        let kept = providers.sanitized();
        let text = serde_json::to_string_pretty(&kept).map_err(|e| e.to_string())?;
        *self.settings.lock().unwrap_or_else(PoisonError::into_inner) = kept;
        let Some(folder) = &self.folder else {
            return Ok(());
        };
        std::fs::create_dir_all(folder)
            .and_then(|()| {
                let tmp = folder.join(format!("{FILE}.yaziliyor"));
                std::fs::write(&tmp, text.as_bytes())?;
                std::fs::rename(&tmp, folder.join(FILE))
            })
            .map_err(|e| {
                format!(
                    "Adres sağlayıcıları kaydedilemedi ({}): {e}",
                    folder.display()
                )
            })
    }

    /// A provider's key on this device.
    pub fn key(&self, provider: &str) -> Option<String> {
        secrets()
            .get(KEY_ORIGIN, &key_id(provider))
            .and_then(|s| s.token)
            .filter(|k| !k.is_empty())
    }

    /// Keeps a provider's key, or forgets it.
    pub fn set_key(&self, provider: &str, key: Option<&str>) -> Result<(), String> {
        let id = key_id(provider);
        match key.map(str::trim).filter(|k| !k.is_empty()) {
            Some(k) => secrets().put(
                KEY_ORIGIN,
                ConnectionSecret {
                    id,
                    values: Vec::new(),
                    user: None,
                    password: None,
                    token: Some(k.to_owned()),
                    client_id: None,
                    client_secret: None,
                },
            ),
            None => secrets().remove(KEY_ORIGIN, &id),
        }
    }

    /// Whether a Google basemap is shown now (the app says it as the drawing changes).
    pub fn set_google_shown(&self, shown: bool) {
        self.google.store(shown, Ordering::Relaxed);
    }

    pub fn google_shown(&self) -> bool {
        self.google.load(Ordering::Relaxed)
    }

    /// Why a usable provider cannot be asked now beside its own settings: Google without its map, a key it needs.
    pub fn why_not(&self, p: &Provider) -> Option<String> {
        if p.google_map && !self.google_shown() {
            return Some("Google altlığı görünmüyor".into());
        }
        if p.key == KeyRule::Required && self.key(&p.id).is_none() {
            return Some("anahtar yok".into());
        }
        None
    }

    fn kept_path(&self, url: &str) -> Option<(PathBuf, PathBuf)> {
        let root = self.cache.as_ref()?;
        let key = format!("{:016x}", hash(url));
        let dir = root.join(&key[..2]);
        Some((
            dir.join(format!("{key}.bin")),
            dir.join(format!("{key}.json")),
        ))
    }

    /// The answer kept for `url` within `days`.
    fn kept(&self, url: &str, days: u32) -> Option<Vec<u8>> {
        if days == 0 {
            return None;
        }
        let (bin, json) = self.kept_path(url)?;
        let meta: Kept = serde_json::from_slice(&std::fs::read(&json).ok()?).ok()?;
        if meta.url != url || now_ms().saturating_sub(meta.stored_ms) > u64::from(days) * 86_400_000
        {
            return None;
        }
        let body = std::fs::read(&bin).ok()?;
        if let Ok(f) = std::fs::File::options().write(true).open(&bin) {
            let _ = f.set_modified(SystemTime::now());
        }
        Some(body)
    }

    fn keep(&self, url: &str, body: &[u8]) {
        let Some((bin, json)) = self.kept_path(url) else {
            return;
        };
        let Some(dir) = bin.parent() else {
            return;
        };
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        let meta = Kept {
            url: url.to_owned(),
            stored_ms: now_ms(),
        };
        let _ = std::fs::write(&bin, body);
        if let Ok(text) = serde_json::to_vec(&meta) {
            let _ = std::fs::write(&json, text);
        }
        if self.puts.fetch_add(1, Ordering::Relaxed) % 64 == 63 {
            self.trim(CACHE_BUDGET);
        }
    }

    fn answers(&self) -> Vec<(PathBuf, u64, SystemTime)> {
        let Some(root) = &self.cache else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for dir in std::fs::read_dir(root).into_iter().flatten().flatten() {
            for f in std::fs::read_dir(dir.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                let path = f.path();
                if path.extension().is_some_and(|e| e == "bin")
                    && let Ok(m) = f.metadata()
                {
                    out.push((path, m.len(), m.modified().unwrap_or(UNIX_EPOCH)));
                }
            }
        }
        out
    }

    /// The answers kept: how many and their bytes.
    pub fn cache_size(&self) -> (usize, u64) {
        let all = self.answers();
        (all.len(), all.iter().map(|(_, n, _)| n).sum())
    }

    /// Önbelleği temizle: every answer forgotten; how many and their bytes.
    pub fn clear_cache(&self) -> (usize, u64) {
        let all = self.answers();
        for (bin, _, _) in &all {
            let _ = std::fs::remove_file(bin);
            let _ = std::fs::remove_file(bin.with_extension("json"));
        }
        (all.len(), all.iter().map(|(_, n, _)| n).sum())
    }

    /// The least recently read answers out until the folder takes at most `budget` bytes.
    pub fn trim(&self, budget: u64) {
        let mut all = self.answers();
        let mut total: u64 = all.iter().map(|(_, n, _)| n).sum();
        if total <= budget {
            return;
        }
        all.sort_by_key(|(_, _, used)| *used);
        for (bin, n, _) in all {
            if total <= budget {
                break;
            }
            let _ = std::fs::remove_file(&bin);
            let _ = std::fs::remove_file(bin.with_extension("json"));
            total = total.saturating_sub(n);
        }
    }

    /// Waits for the provider's turn, asking `stop` meanwhile; false when stopped.
    pub(crate) fn wait_turn(&self, p: &Provider, stop: &dyn Fn() -> bool) -> bool {
        let interval = Duration::from_millis(p.interval_ms());
        loop {
            let wait = {
                let mut next = self.next.lock().unwrap_or_else(PoisonError::into_inner);
                let now = Instant::now();
                let at = next.get(&p.id).copied().unwrap_or(now);
                if at <= now {
                    next.insert(p.id.clone(), now + interval);
                    None
                } else {
                    Some(at - now)
                }
            };
            let Some(wait) = wait else {
                return true;
            };
            if stop() {
                return false;
            }
            std::thread::sleep(wait.min(Duration::from_millis(50)));
        }
    }
}

/// An answer's status in the user's words.
fn refused(p: &Provider, status: u16) -> String {
    let host = p.host();
    match status {
        401 | 403 => format!(
            "{host} isteği reddetti ({status}): anahtar geçersiz ya da bu kullanıma izin yok."
        ),
        429 => format!("{host} çok sık istek aldığını söyledi (429); biraz sonra yeniden deneyin."),
        s if s >= 500 => format!("{host} sunucusunda bir sorun var ({s}); sonra yeniden deneyin."),
        s => format!("{host} isteği yanıtlamadı ({s})."),
    }
}

impl AddressHost for Online {
    fn providers(&self) -> Vec<Provider> {
        self.settings().resolved()
    }

    fn ask(
        &self,
        p: &Provider,
        request: &GeoRequest,
        stop: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, String> {
        if let Some(why) = self.why_not(p) {
            return Err(format!("{}: {why}.", p.name));
        }
        if let Some(body) = self.kept(&request.url, p.cache_days()) {
            return Ok(body);
        }
        let mut url = request.url.clone();
        if let Some(param) = &request.key_param {
            match self.key(&p.id) {
                Some(key) => {
                    url.push(if url.contains('?') { '&' } else { '?' });
                    url.push_str(param);
                    url.push('=');
                    url.push_str(&encode(&key));
                }
                None if p.key == KeyRule::Required => {
                    return Err(format!(
                        "{} anahtar ister; Adres sağlayıcıları'nda yazın.",
                        p.name
                    ));
                }
                None => {}
            }
        }
        if !self.wait_turn(p, stop) {
            return Err("Durduruldu.".into());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        fetch::spawn(
            Request {
                method: Method::Get,
                url,
                headers: vec![("Accept".into(), "application/json".into())],
                body: None,
            },
            p.parallel.max(1) as usize,
            cancel.clone(),
            move |answer| {
                let _ = tx.send(answer);
            },
        );
        let answer = loop {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(a) => break a,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if stop() {
                        cancel.store(true, Ordering::Relaxed);
                        return Err("Durduruldu.".into());
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("İstek yarıda kaldı.".into());
                }
            }
        };
        let response = answer.map_err(|f| f.message)?;
        // An error's body says why in the provider's own words; the reader reads it (read::forward).
        if response.status != 200 && !(400..500).contains(&response.status) {
            return Err(refused(p, response.status));
        }
        if matches!(response.status, 401 | 403 | 429) {
            return Err(refused(p, response.status));
        }
        if response.status == 200 && p.cache_days() > 0 {
            self.keep(&request.url, &response.body);
        }
        Ok(response.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_provider_without_its_key_or_map_is_not_asked() {
        let o = Online::open(None, None);
        let all = o.providers();
        let google = all.iter().find(|p| p.google_map).expect("Google");
        assert_eq!(
            o.why_not(google).as_deref(),
            Some("Google altlığı görünmüyor")
        );
        o.set_google_shown(true);
        assert_eq!(o.why_not(google).as_deref(), Some("anahtar yok"));
        let nominatim = all
            .iter()
            .find(|p| p.id == "nominatim-osm")
            .expect("Nominatim");
        assert_eq!(o.why_not(nominatim), None);
    }

    #[test]
    fn answers_are_kept_and_trimmed_least_recent_first() {
        let root = std::env::temp_dir().join(format!("kentos-adres-{}", now_ms()));
        let o = Online::open(None, Some(root.clone()));
        o.keep("https://a/search?q=1", b"one");
        o.keep("https://a/search?q=2", b"twotwo");
        // The second read long ago: it goes first.
        let (old, _) = o.kept_path("https://a/search?q=2").expect("a cache");
        std::fs::File::options()
            .write(true)
            .open(&old)
            .and_then(|f| f.set_modified(UNIX_EPOCH + Duration::from_secs(1)))
            .expect("touched");
        assert_eq!(
            o.kept("https://a/search?q=1", 90).as_deref(),
            Some(&b"one"[..])
        );
        assert_eq!(o.kept("https://a/search?q=1", 0), None);
        assert_eq!(o.cache_size(), (2, 9));
        o.trim(5);
        assert_eq!(o.cache_size(), (1, 3));
        assert_eq!(o.clear_cache().0, 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn turns_keep_the_providers_pace() {
        let o = Online::open(None, None);
        let p = o.providers().remove(0);
        let t0 = Instant::now();
        assert!(o.wait_turn(&p, &|| false));
        assert!(o.wait_turn(&p, &|| false));
        assert!(t0.elapsed() >= Duration::from_millis(p.interval_ms()));
        assert!(!o.wait_turn(&p, &|| true));
    }
}
