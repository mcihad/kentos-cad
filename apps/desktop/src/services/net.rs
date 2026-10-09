//! The services' threads (docs/adr/0208 §6, §7): a few workers take the
//! hub's jobs in the order they came (the frames ask for the view's centre
//! first). A tile's address is made with its connection's proof; a fresh
//! answer on the disk is used without asking, a stale one is asked about
//! with its validators; the request itself runs on the cloud's runtime
//! (`kentos_cloud::fetch`, at most the host's own limit at once) and its
//! answer comes back as a job, so reading a picture, a vector tile or a
//! style never runs on the runtime's threads or the window's. A tile
//! dropped by the frames before it leaves never leaves.

use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};

use kentos_cloud::fetch::{self, Failure, Method, Response};
use kentos_contracts::ServiceKind;
use kentos_render_wgpu::styled::ServiceTile;
use kentos_services::{auth, google, query, source};

use super::cache::{Kept, now_ms};
use super::decode;
use super::hub::{Built, Entry, Job, Ready, Shared, TileKey};

/// Worker threads.
const WORKERS: usize = 3;

/// An answer on its way back to a worker.
struct Arrived {
    entry: Arc<Entry>,
    k: TileKey,
    vector: bool,
    cache_url: String,
    kept: Option<Kept>,
    answer: Result<Response, Failure>,
}

pub(super) fn start(shared: &Arc<Shared>) {
    let (tx, rx) = std::sync::mpsc::channel::<Arrived>();
    let rx = Arc::new(std::sync::Mutex::new(rx));
    for i in 0..WORKERS {
        let s = shared.clone();
        let tx = tx.clone();
        let rx = rx.clone();
        let _ = std::thread::Builder::new()
            .name(format!("kentos-servis-{i}"))
            .spawn(move || worker(&s, &tx, &rx));
    }
    // Google's credits for a view that has rested (docs/adr/0208 §8).
    let c = shared.clone();
    let _ = std::thread::Builder::new()
        .name("kentos-servis-atif".into())
        .spawn(move || credits(&c));
    // A trim of the disk now and then, off every other thread.
    let s = shared.clone();
    let _ = std::thread::Builder::new()
        .name("kentos-servis-onbellek".into())
        .spawn(move || {
            s.cache.trim(super::cache::BUDGET);
        });
}

fn worker(
    s: &Arc<Shared>,
    tx: &std::sync::mpsc::Sender<Arrived>,
    rx: &Arc<std::sync::Mutex<std::sync::mpsc::Receiver<Arrived>>>,
) {
    loop {
        // Answers first: they finish what was asked; then the jobs in order.
        let arrived = rx
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .try_recv()
            .ok();
        if let Some(a) = arrived {
            answer(s, a);
            continue;
        }
        let job = {
            let mut jobs = s.jobs.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                if let Some(j) = jobs.pop_front() {
                    break Some(j);
                }
                let (j, _) = s
                    .work
                    .wait_timeout(jobs, std::time::Duration::from_millis(25))
                    .unwrap_or_else(PoisonError::into_inner);
                jobs = j;
                if !jobs.is_empty() {
                    continue;
                }
                break None;
            }
        };
        match job {
            Some(Job::Resolve { entry }) => super::resolve::resolve(s, &entry),
            Some(Job::Fetch { entry, k, vector }) => fetch_tile(s, tx, entry, k, vector),
            Some(Job::Build {
                entry,
                k,
                step,
                look,
            }) => build(s, &entry, k, step, &look),
            None => {}
        }
    }
}

/// The address of a service's answers without their secrets, up to its
/// first placeholder: what “Önbelleği temizle” forgets.
pub(super) fn cache_prefix(entry: &Entry) -> Option<String> {
    let url = if entry.service.kind == ServiceKind::Google {
        format!("{}/v1/2dtiles/", google::TILES)
    } else {
        entry.service.url.clone()
    };
    let cut = url.find(['{', '?']).unwrap_or(url.len());
    (cut > 8).then(|| url[..cut].to_owned())
}

/// A tile's address with its proof, and the same without secrets (its cache key).
fn tile_request(
    entry: &Entry,
    ready: &Ready,
    k: TileKey,
) -> Result<(fetch::Request, String), String> {
    let s = &entry.service;
    let t = k.t;
    let url = if s.kind == ServiceKind::Google {
        let session = ready
            .google
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .ok_or("Google oturumu yok.")?;
        google::tile_url(&session, t.level, t.col, t.row)
    } else {
        source::tile_url(s, &ready.source, t, false).ok_or("Karonun adresi kurulamadı.")?
    };
    let mut req = kentos_services::request::Request::get(url.clone());
    let cache_url = match &entry.connection {
        Some(c) => {
            let names = auth::secret_params(c);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let secret = super::secrets::secrets().get(&c.origin, &c.id);
            if let Some(why) = auth::missing(c, secret.as_ref()) {
                return Err(why);
            }
            if let Some(secret) = &secret {
                let token = ready
                    .token
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                auth::apply(c, secret, token.as_ref(), &mut req);
            }
            query::without_params(&url, &names)
        }
        None => url,
    };
    Ok((to_fetch(req), cache_url))
}

/// A request sent with a connection's proof (its token asked for first,
/// for ArcGIS and OAuth 2) and waited for: its answer, or why it failed in
/// the windows' words (Bağlan, Dene, Servisten veri al, Servis bilgisi).
pub(crate) fn send(
    req: kentos_services::request::Request,
    conn: Option<(
        &kentos_contracts::ServiceConnection,
        Option<&kentos_contracts::ConnectionSecret>,
    )>,
    referer: &str,
    per_host: usize,
) -> Result<Response, String> {
    let mut req = req;
    if let Some((c, secret)) = conn {
        if let Some(why) = auth::missing(c, secret) {
            return Err(why);
        }
        if let Some(secret) = secret {
            let token = match auth::token_request(c, secret, referer) {
                Some(t) => {
                    let res = fetch::wait(to_fetch(t), per_host).map_err(|f| f.message)?;
                    Some(auth::read_token(
                        c,
                        &String::from_utf8_lossy(&res.body),
                        now_ms(),
                    )?)
                }
                None => None,
            };
            auth::apply(c, secret, token.as_ref(), &mut req);
        }
    }
    let res = fetch::wait(to_fetch(req), per_host).map_err(|f| f.message)?;
    match res.status {
        200..=299 => {
            if let Some(e) = exception(&res.body) {
                return Err(format!("Servis bir hata bildirdi: {e}"));
            }
            Ok(res)
        }
        401 | 403 => Err(format!(
            "Sunucu erişimi reddetti ({}): bağlantının değerlerini Bağlantılar'dan denetleyin.",
            res.status
        )),
        404 => Err(format!("Adres bulunamadı (404): {}", res.url)),
        s => Err(format!("Sunucu {s} dedi.")),
    }
}

/// [`send`]'s answer as text.
pub(crate) fn send_text(
    req: kentos_services::request::Request,
    conn: Option<(
        &kentos_contracts::ServiceConnection,
        Option<&kentos_contracts::ConnectionSecret>,
    )>,
    referer: &str,
    per_host: usize,
) -> Result<String, String> {
    let res = send(req, conn, referer, per_host)?;
    String::from_utf8(res.body).map_err(|_| "Servisin yanıtı metin değil.".to_owned())
}

/// [`send`]'s status and size (Dene).
pub(crate) fn ask_with(
    req: kentos_services::request::Request,
    conn: Option<(
        &kentos_contracts::ServiceConnection,
        Option<&kentos_contracts::ConnectionSecret>,
    )>,
    referer: &str,
    per_host: usize,
) -> Result<(u16, usize), String> {
    send(req, conn, referer, per_host).map(|r| (r.status, r.body.len()))
}

/// The services' request as the cloud's fetcher takes it.
pub(super) fn to_fetch(r: kentos_services::request::Request) -> fetch::Request {
    let mut headers = r.headers;
    let (method, body) = match r.body {
        Some((body, media)) => {
            headers.push(("Content-Type".into(), media));
            (Method::Post, Some(body.into_bytes()))
        }
        None => (Method::Get, None),
    };
    fetch::Request {
        method,
        url: r.url,
        headers,
        body,
    }
}

fn fetch_tile(
    s: &Arc<Shared>,
    tx: &std::sync::mpsc::Sender<Arrived>,
    entry: Arc<Entry>,
    k: TileKey,
    vector: bool,
) {
    let cancel = {
        let tiles = s.tiles.lock().unwrap_or_else(PoisonError::into_inner);
        match tiles.pending.get(&k) {
            Some(p) if !p.cancel.load(Ordering::Relaxed) => p.cancel.clone(),
            // Dropped by the frames before it left.
            _ => return,
        }
    };
    let Some(ready) = entry.ready() else {
        finish(s, k, None);
        return;
    };
    let (mut req, cache_url) = match tile_request(&entry, &ready, k) {
        Ok(r) => r,
        Err(why) => {
            s.notice(format!("“{}”: {why}", name_of(&entry)));
            finish(s, k, None);
            return;
        }
    };
    let kept = s.cache.get(&cache_url);
    if let Some(kept) = &kept {
        if kept.meta.fresh(now_ms()) {
            let body = kept.body.clone();
            done(s, &entry, k, vector, &body);
            return;
        }
        req.headers.extend(kept.meta.validators());
    }
    let tx = tx.clone();
    let wake = s.clone();
    let limit = ready.per_host;
    fetch::spawn(req, limit, cancel, move |answer| {
        let _ = tx.send(Arrived {
            entry,
            k,
            vector,
            cache_url,
            kept,
            answer,
        });
        // A worker waiting for jobs looks at the answers too.
        wake.work.notify_one();
    });
}

/// What an answer makes of a tile.
fn answer(s: &Arc<Shared>, a: Arrived) {
    let Arrived {
        entry,
        k,
        vector,
        cache_url,
        kept,
        answer,
    } = a;
    let res = match answer {
        Ok(r) => r,
        Err(f) if f.cancelled => {
            finish(s, k, None);
            return;
        }
        Err(f) => {
            // While the network is gone a kept tile still shows, unless its server said it must not.
            if let Some(kept) = kept.filter(|k| !k.meta.must_revalidate) {
                done(s, &entry, k, vector, &kept.body);
            } else {
                failed(s, k);
                if !f.retryable {
                    s.notice(format!("“{}”: {}", name_of(&entry), f.message));
                }
            }
            return;
        }
    };
    match res.status {
        200 => {
            s.cache.put(&cache_url, 200, &res.headers, &res.body);
            done(s, &entry, k, vector, &res.body);
        }
        304 => match kept {
            Some(kept) => {
                s.cache.renew(&kept.meta, &res.headers);
                done(s, &entry, k, vector, &kept.body);
            }
            None => failed(s, k),
        },
        // No tile there: nothing to draw.
        204 | 404 => finish(s, k, Some(ServiceTile::Empty)),
        401 | 403 => {
            failed(s, k);
            s.notice(format!(
                "“{}”: sunucu erişimi reddetti ({}). Bağlantının bilgilerini Bağlantılar penceresinden denetleyin.",
                name_of(&entry),
                res.status
            ));
        }
        status => {
            failed(s, k);
            if !(status == 429 || status >= 500) {
                s.notice(format!("“{}”: sunucu {status} dedi.", name_of(&entry)));
            }
        }
    }
}

/// A tile's bytes read: a picture into slots, a vector tile's layers kept.
fn done(s: &Arc<Shared>, entry: &Entry, k: TileKey, vector: bool, body: &[u8]) {
    if vector {
        match decode::vector_layers(body) {
            Ok(layers) => {
                let mut tiles = s.tiles.lock().unwrap_or_else(PoisonError::into_inner);
                tiles.put_parsed(k, Arc::new(layers));
                tiles.pending.remove(&k);
                drop(tiles);
                s.tell();
            }
            Err(why) => {
                failed(s, k);
                s.notice(format!(
                    "“{}”: vektör karo okunamadı: {why}",
                    name_of(entry)
                ));
            }
        }
        return;
    }
    let want = entry.ready().and_then(|r| {
        r.view
            .grid
            .matrices
            .get(k.t.level as usize)
            .map(|m| (m.tile_w, m.tile_h))
    });
    match decode::picture(body, want) {
        Ok(tile) => finish(s, k, Some(tile)),
        Err(why) => {
            failed(s, k);
            let said = exception(body).map_or(why, |e| format!("servis şunu dedi: {e}"));
            s.notice(format!("“{}”: {said}", name_of(entry)));
        }
    }
}

/// An OGC exception's words in a body (a WMS answers an error as XML with 200).
fn exception(body: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(body.get(..body.len().min(64 * 1024))?).ok()?;
    if !text.trim_start().starts_with('<') {
        return None;
    }
    let doc = roxmltree::Document::parse(text).ok()?;
    let words = doc
        .descendants()
        .filter(|n| matches!(n.tag_name().name(), "ServiceException" | "ExceptionText"))
        .filter_map(|n| n.text())
        .map(str::trim)
        .find(|t| !t.is_empty())?;
    Some(words.chars().take(300).collect())
}

/// The tile's work ends: what it gives is kept, its request forgotten.
fn finish(s: &Arc<Shared>, k: TileKey, tile: Option<ServiceTile>) {
    let mut tiles = s.tiles.lock().unwrap_or_else(PoisonError::into_inner);
    tiles.pending.remove(&k);
    if let Some(tile) = tile {
        tiles.failed.remove(&k);
        tiles.put_picture(k, tile);
    }
    drop(tiles);
    s.tell();
}

fn failed(s: &Arc<Shared>, k: TileKey) {
    let mut tiles = s.tiles.lock().unwrap_or_else(PoisonError::into_inner);
    tiles.pending.remove(&k);
    let tries = tiles.failed.get(&k).map_or(0, |(_, n)| *n) + 1;
    tiles.failed.insert(k, (now_ms(), tries));
    drop(tiles);
    s.tell();
}

/// Builds a read vector tile at a zoom step for a look.
fn build(s: &Arc<Shared>, entry: &Entry, k: TileKey, step: i32, look: &Arc<decode::VectorLook>) {
    let key = (k, step, look.key());
    let layers = {
        let tiles = s.tiles.lock().unwrap_or_else(PoisonError::into_inner);
        tiles.parsed.get(&k).map(|(l, _)| l.clone())
    };
    let (Some(layers), Some(ready)) = (layers, entry.ready()) else {
        s.tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .building
            .remove(&key);
        return;
    };
    let Some(style) = ready.style.clone() else {
        s.tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .building
            .remove(&key);
        return;
    };
    let (tile, labels) = decode::vector_tile(
        &layers,
        &style,
        &ready.source_id,
        f64::from(step) / 4.0,
        &ready.view,
        k.t,
        look,
    );
    let mut tiles = s.tiles.lock().unwrap_or_else(PoisonError::into_inner);
    tiles.tick += 1;
    let used = tiles.tick;
    tiles.put_built(
        key,
        Built {
            tile: Arc::new(kentos_render_wgpu::styled::VectorTile { part: tile }),
            labels: Arc::new(labels),
            used,
        },
    );
    drop(tiles);
    s.tell();
}

/// How long a view rests before Google's credits are asked for it.
const CREDIT_REST: std::time::Duration = std::time::Duration::from_millis(400);

/// Asks Google for the credits of views that have rested, one at a time.
fn credits(s: &Arc<Shared>) {
    loop {
        let ask = {
            let mut asks = s.credit_asks.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                let now = std::time::Instant::now();
                let rested = asks
                    .iter()
                    .find(|(_, a)| now.duration_since(a.at) >= CREDIT_REST)
                    .map(|(id, _)| *id);
                if let Some(id) = rested {
                    break asks.remove(&id);
                }
                let wait = asks
                    .values()
                    .map(|a| CREDIT_REST.saturating_sub(now.duration_since(a.at)))
                    .min()
                    .unwrap_or(std::time::Duration::from_secs(3600));
                asks = s
                    .credit_wake
                    .wait_timeout(asks, wait)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
        };
        if let Some(ask) = ask {
            google_credit(s, &ask);
        }
    }
}

/// Google's credits for a view (`/tile/v1/viewport`); a view that gets
/// none is not asked again.
fn google_credit(s: &Arc<Shared>, ask: &super::hub::CreditAsk) {
    let Some(ready) = ask.entry.ready() else {
        return;
    };
    let Some(session) = ready
        .google
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
    else {
        return;
    };
    let [south, west, north, east] = ask.view.degrees();
    let mut req = kentos_services::request::Request::get(google::viewport_url(
        &session,
        ask.view.zoom,
        south,
        west,
        north,
        east,
    ));
    if let Some(c) = &ask.entry.connection {
        match super::secrets::secrets().get(&c.origin, &c.id) {
            Some(secret) => auth::apply(c, &secret, None, &mut req),
            None => return,
        }
    }
    let text = match fetch::wait(to_fetch(req), ready.per_host) {
        Ok(res) if res.status == 200 => {
            google::read_copyright(&String::from_utf8_lossy(&res.body)).unwrap_or_default()
        }
        _ => String::new(),
    };
    s.credits
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(ask.entry.id, (ask.view, text));
    s.tell();
}

/// A service as its messages name it.
pub(super) fn name_of(entry: &Entry) -> String {
    entry
        .service
        .preset
        .as_deref()
        .and_then(kentos_services::presets::preset)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| {
            let url = &entry.service.url;
            kentos_contracts::origin_of(url)
                .unwrap_or_else(|| entry.service.kind.label().to_owned())
        })
}
