//! The map services of the desktop (docs/adr/0208 §3–§9): what the scene
//! shows, by key; for each, what is known of it (resolving, ready, failed)
//! and its tiles in memory. The render pass asks for tiles each frame
//! (`tile`, `vector`); one not here is asked for in the order the frame asks
//! (the view's centre first) and given back by a later frame; one not asked
//! for again in two frames is let go (its request dropped). Requests,
//! decoding and building run on the services' own threads (`net.rs`), the
//! window hears a batch of fresh tiles once a frame at most (`ready`).

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

use kentos_contracts::{ServiceConnection, ServiceLayer};
use kentos_geometry_core::geom::tiles::TileRef;
use kentos_render_wgpu::styled::{ServiceTile, ServiceView, VectorTile};
use kentos_services::attribution::Credit;
use kentos_services::auth::Token;
use kentos_services::google::Session;
use kentos_services::labels::Candidate;
use kentos_services::mvt;
use kentos_services::source::TileSource;
use kentos_services::style::Style;

use super::cache::{Cache, now_ms};
use super::decode::VectorLook;
use super::systems::ProjectSystem;

/// Bytes the picture tiles in memory may take (about 1000 slots).
const PICTURE_BUDGET: usize = 288 * 1024 * 1024;
/// Vector tiles kept read, and kept built.
const PARSED_MOST: usize = 384;
const BUILT_MOST: usize = 512;
/// A tile that failed this many times shows nothing until the drawing asks again later.
const TRIES: u32 = 3;

/// A service's key: its layer's service and its connection (without secrets), hashed.
pub fn key_of(service: &ServiceLayer, connection: Option<&ServiceConnection>) -> String {
    let mut h = DefaultHasher::new();
    serde_json::to_string(service)
        .unwrap_or_default()
        .hash(&mut h);
    serde_json::to_string(&connection)
        .unwrap_or_default()
        .hash(&mut h);
    format!("service:{:016x}", h.finish())
}

fn hash_of(key: &str) -> u64 {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    h.finish()
}

/// What is known of a service.
pub(super) enum State {
    /// A thread reads what it needs (a style, a session, a token).
    Resolving,
    Ready(Arc<Ready>),
    /// Why it shows nothing; tried again when the project, the secrets or the time change.
    Failed {
        message: String,
        at_ms: u64,
        secrets: u64,
    },
}

/// A service ready to draw.
pub struct Ready {
    pub view: Arc<ServiceView>,
    pub source: TileSource,
    /// A vector service's style and the source its tiles are of.
    pub style: Option<Arc<Style>>,
    pub source_id: String,
    pub google: Mutex<Option<Session>>,
    pub token: Mutex<Option<Token>>,
    pub per_host: usize,
    /// Its credits as the view shows them, and their links.
    pub attribution: Mutex<Credit>,
}

pub(super) struct Entry {
    pub id: u64,
    pub service: ServiceLayer,
    pub connection: Option<ServiceConnection>,
    pub project: Arc<ProjectSystem>,
    pub state: Mutex<State>,
}

impl Entry {
    pub fn ready(&self) -> Option<Arc<Ready>> {
        match &*self.state.lock().unwrap_or_else(PoisonError::into_inner) {
            State::Ready(r) => Some(r.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct TileKey {
    pub service: u64,
    pub t: TileRef,
}

/// A tile's work on its way.
pub(super) struct Pending {
    pub cancel: Arc<AtomicBool>,
    /// The frame it was last asked in.
    pub wanted: u64,
}

/// A vector tile built at a zoom step for a look.
pub(super) struct Built {
    pub tile: Arc<VectorTile>,
    pub labels: Arc<Vec<Candidate>>,
    pub used: u64,
}

#[derive(Default)]
pub(super) struct Tiles {
    pub pictures: HashMap<TileKey, (ServiceTile, usize, u64)>,
    pub picture_bytes: usize,
    pub parsed: HashMap<TileKey, (Arc<Vec<mvt::Layer>>, u64)>,
    /// By tile, zoom step (a quarter level) and look.
    pub built: HashMap<(TileKey, i32, u64), Built>,
    pub building: HashSet<(TileKey, i32, u64)>,
    pub pending: HashMap<TileKey, Pending>,
    pub failed: HashMap<TileKey, (u64, u32)>,
    pub tick: u64,
}

impl Tiles {
    pub fn put_picture(&mut self, k: TileKey, tile: ServiceTile) {
        let bytes = match &tile {
            ServiceTile::Image(img) => img.slots.iter().map(|s| s.len()).sum(),
            ServiceTile::Empty => 0,
        };
        self.tick += 1;
        if let Some((_, old, _)) = self.pictures.insert(k, (tile, bytes, self.tick)) {
            self.picture_bytes -= old;
        }
        self.picture_bytes += bytes;
        while self.picture_bytes > PICTURE_BUDGET {
            let Some((&oldest, _)) = self.pictures.iter().min_by_key(|(_, (_, _, at))| *at) else {
                break;
            };
            if let Some((_, n, _)) = self.pictures.remove(&oldest) {
                self.picture_bytes -= n;
            }
        }
    }

    pub fn put_parsed(&mut self, k: TileKey, layers: Arc<Vec<mvt::Layer>>) {
        self.tick += 1;
        self.parsed.insert(k, (layers, self.tick));
        if self.parsed.len() > PARSED_MOST
            && let Some((&oldest, _)) = self.parsed.iter().min_by_key(|(_, (_, at))| *at)
        {
            self.parsed.remove(&oldest);
        }
    }

    pub fn put_built(&mut self, k: (TileKey, i32, u64), built: Built) {
        self.building.remove(&k);
        self.built.insert(k, built);
        if self.built.len() > BUILT_MOST
            && let Some((&oldest, _)) = self.built.iter().min_by_key(|(_, b)| b.used)
        {
            self.built.remove(&oldest);
        }
    }
}

/// Work for the services' threads.
pub(super) enum Job {
    /// Get a tile's bytes (a picture, or a vector tile to read).
    Fetch {
        entry: Arc<Entry>,
        k: TileKey,
        vector: bool,
    },
    /// Build a read vector tile at a zoom step for a look.
    Build {
        entry: Arc<Entry>,
        k: TileKey,
        step: i32,
        look: Arc<VectorLook>,
    },
    /// Read what a service needs before its tiles.
    Resolve { entry: Arc<Entry> },
}

pub(super) struct Shared {
    pub entries: Mutex<HashMap<String, Arc<Entry>>>,
    pub tiles: Mutex<Tiles>,
    pub jobs: Mutex<VecDeque<Job>>,
    pub work: Condvar,
    pub generation: AtomicU64,
    pub fresh: Mutex<bool>,
    pub told: Condvar,
    pub cache: Cache,
    pub look: Mutex<Option<Arc<VectorLook>>>,
    /// What the window should be told (a service that failed), each once.
    pub notices: Mutex<Vec<String>>,
    /// Counts the times tiles came or a service changed: what the overlay's labels rest on.
    pub changes: AtomicU64,
    /// Google's credits for the view (docs/adr/0208 §8): asked once the view
    /// rests, by service; the last answered.
    pub credit_asks: Mutex<HashMap<u64, CreditAsk>>,
    pub credit_wake: Condvar,
    pub credits: Mutex<HashMap<u64, (CreditView, String)>>,
}

/// A view Google's credits are asked for: its zoom and its box in degrees
/// (south, west, north, east), rounded so a still view asks once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CreditView {
    pub zoom: u32,
    pub micro: [i64; 4],
}

impl CreditView {
    pub fn new(zoom: u32, south: f64, west: f64, north: f64, east: f64) -> CreditView {
        let m = |v: f64| (v * 1e4).round() as i64;
        CreditView {
            zoom,
            micro: [m(south), m(west), m(north), m(east)],
        }
    }

    pub fn degrees(&self) -> [f64; 4] {
        self.micro.map(|v| v as f64 / 1e4)
    }
}

/// A credits request waiting for the view to rest.
pub(super) struct CreditAsk {
    pub entry: Arc<Entry>,
    pub view: CreditView,
    pub at: std::time::Instant,
}

impl Shared {
    pub fn tell(&self) {
        self.changes.fetch_add(1, Ordering::Relaxed);
        let mut fresh = self.fresh.lock().unwrap_or_else(PoisonError::into_inner);
        *fresh = true;
        self.told.notify_all();
    }

    pub fn push(&self, job: Job) {
        self.jobs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(job);
        self.work.notify_one();
    }

    pub fn notice(&self, text: String) {
        let mut n = self.notices.lock().unwrap_or_else(PoisonError::into_inner);
        if !n.contains(&text) {
            n.push(text);
        }
        drop(n);
        self.tell();
    }
}

/// The desktop's services: one for the app.
pub struct Hub {
    pub(super) shared: Arc<Shared>,
}

static USED: AtomicBool = AtomicBool::new(false);

/// Whether the drawing has shown a service (the window then listens for tiles).
pub fn in_use() -> bool {
    USED.load(Ordering::Relaxed)
}

/// The app's hub, its threads started the first time.
pub fn hub() -> &'static Hub {
    static HUB: OnceLock<Hub> = OnceLock::new();
    HUB.get_or_init(|| {
        let shared = Arc::new(Shared {
            entries: Mutex::new(HashMap::new()),
            tiles: Mutex::new(Tiles::default()),
            jobs: Mutex::new(VecDeque::new()),
            work: Condvar::new(),
            generation: AtomicU64::new(0),
            fresh: Mutex::new(false),
            told: Condvar::new(),
            cache: Cache::device(),
            look: Mutex::new(None),
            notices: Mutex::new(Vec::new()),
            changes: AtomicU64::new(0),
            credit_asks: Mutex::new(HashMap::new()),
            credit_wake: Condvar::new(),
            credits: Mutex::new(HashMap::new()),
        });
        super::net::start(&shared);
        Hub { shared }
    })
}

impl Hub {
    /// The scene shows `service` under `key`: known from now, resolved when
    /// it is new or what it rests on changed (the project's system, the
    /// device's secrets, a failure an hour ago).
    pub fn register(
        &self,
        key: &str,
        service: &ServiceLayer,
        connection: Option<&ServiceConnection>,
        project: &Arc<ProjectSystem>,
    ) {
        USED.store(true, Ordering::Relaxed);
        let secrets = super::secrets::secrets().version();
        let mut entries = self
            .shared
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let again = match entries.get(key) {
            None => true,
            Some(e) if e.project.key != project.key => true,
            Some(e) => match &*e.state.lock().unwrap_or_else(PoisonError::into_inner) {
                State::Failed {
                    at_ms, secrets: s, ..
                } => *s != secrets || now_ms().saturating_sub(*at_ms) > 60 * 60_000,
                _ => false,
            },
        };
        if !again {
            return;
        }
        let entry = Arc::new(Entry {
            id: hash_of(&format!("{key}/{}", project.key)),
            service: service.clone(),
            connection: connection.cloned(),
            project: project.clone(),
            state: Mutex::new(State::Resolving),
        });
        entries.insert(key.to_owned(), entry.clone());
        drop(entries);
        self.shared.push(Job::Resolve { entry });
    }

    /// Only these keys are shown now: the others' tiles go.
    pub fn keep_only(&self, keys: &HashSet<String>) {
        let mut entries = self
            .shared
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let gone: Vec<u64> = entries
            .iter()
            .filter(|(k, _)| !keys.contains(*k))
            .map(|(k, _)| hash_of(k))
            .collect();
        if gone.is_empty() {
            return;
        }
        entries.retain(|k, _| keys.contains(k));
        drop(entries);
        let mut tiles = self
            .shared
            .tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for (k, p) in &tiles.pending {
            if gone.contains(&k.service) {
                p.cancel.store(true, Ordering::Relaxed);
            }
        }
        tiles.pending.retain(|k, _| !gone.contains(&k.service));
    }

    /// The colours and anchor vector tiles are built with now.
    pub fn set_look(&self, look: VectorLook) {
        let mut l = self
            .shared
            .look
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if l.as_ref().is_some_and(|o| o.key() == look.key()) {
            return;
        }
        *l = Some(Arc::new(look));
    }

    fn entry(&self, key: &str) -> Option<Arc<Entry>> {
        self.shared
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .cloned()
    }

    /// A service's view, when it is ready.
    pub fn view(&self, key: &str) -> Option<Arc<ServiceView>> {
        self.entry(key)?.ready().map(|r| r.view.clone())
    }

    /// Why a service shows nothing, when it failed.
    pub fn failure(&self, key: &str) -> Option<String> {
        let e = self.entry(key)?;
        match &*e.state.lock().unwrap_or_else(PoisonError::into_inner) {
            State::Failed { message, .. } => Some(message.clone()),
            _ => None,
        }
    }

    /// A service's credits now, and their links.
    pub fn attribution(&self, key: &str) -> Option<Credit> {
        let r = self.entry(key)?.ready()?;
        let a = r
            .attribution
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        (!a.text.is_empty()).then_some(a)
    }

    /// What tiles and services have changed since: the overlay's labels are
    /// placed again when it moves.
    pub fn changes(&self) -> u64 {
        self.shared.changes.load(Ordering::Relaxed)
    }

    /// A Google service's credits for `view` (docs/adr/0208 §8): the last
    /// answered; asked again for this view once it has rested 400 ms.
    pub fn google_credit(&self, key: &str, view: CreditView) -> Option<String> {
        let entry = self.entry(key)?;
        entry.ready()?;
        let known = self
            .shared
            .credits
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&entry.id)
            .cloned();
        if known.as_ref().is_none_or(|(v, _)| *v != view) {
            let mut asks = self
                .shared
                .credit_asks
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if asks.get(&entry.id).is_none_or(|a| a.view != view) {
                let id = entry.id;
                asks.insert(
                    id,
                    CreditAsk {
                        entry,
                        view,
                        at: std::time::Instant::now(),
                    },
                );
                self.shared.credit_wake.notify_one();
            }
        }
        known.map(|(_, text)| text).filter(|t| !t.is_empty())
    }

    /// A frame begins: the tiles not asked for in the last two let go of their requests.
    pub fn frame(&self) {
        let now = self.shared.generation.fetch_add(1, Ordering::Relaxed) + 1;
        let mut tiles = self
            .shared
            .tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        tiles.pending.retain(|_, p| {
            let keep = p.wanted + 2 >= now;
            if !keep {
                p.cancel.store(true, Ordering::Relaxed);
            }
            keep
        });
    }

    /// Whether a tile failed often or lately: it shows nothing for now.
    fn failing(tiles: &Tiles, k: &TileKey) -> bool {
        tiles.failed.get(k).is_some_and(|(at, tries)| {
            *tries >= TRIES || now_ms().saturating_sub(*at) < 2_000 * u64::from(*tries)
        })
    }

    /// A picture tile, when it is here; asked for when it is not.
    pub fn tile(&self, key: &str, t: TileRef) -> Option<ServiceTile> {
        let entry = self.entry(key)?;
        entry.ready()?;
        let k = TileKey {
            service: entry.id,
            t,
        };
        let generation = self.shared.generation.load(Ordering::Relaxed);
        let mut tiles = self
            .shared
            .tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        tiles.tick += 1;
        let tick = tiles.tick;
        if let Some((tile, _, at)) = tiles.pictures.get_mut(&k) {
            *at = tick;
            return Some(tile.clone());
        }
        if Self::failing(&tiles, &k) {
            return Some(ServiceTile::Empty);
        }
        if let Some(p) = tiles.pending.get_mut(&k) {
            p.wanted = generation;
            return None;
        }
        tiles.pending.insert(
            k,
            Pending {
                cancel: Arc::new(AtomicBool::new(false)),
                wanted: generation,
            },
        );
        drop(tiles);
        self.shared.push(Job::Fetch {
            entry,
            k,
            vector: false,
        });
        None
    }

    /// A vector tile built for `zoom`, when it is here; read and built when it
    /// is not, an earlier step's shown meanwhile.
    pub fn vector(&self, key: &str, t: TileRef, zoom: f64) -> Option<Arc<VectorTile>> {
        let entry = self.entry(key)?;
        entry.ready()?;
        let look = self
            .shared
            .look
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()?;
        let k = TileKey {
            service: entry.id,
            t,
        };
        let step = (zoom * 4.0).round() as i32;
        let lk = look.key();
        let generation = self.shared.generation.load(Ordering::Relaxed);
        let mut tiles = self
            .shared
            .tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        tiles.tick += 1;
        let tick = tiles.tick;
        if let Some(b) = tiles.built.get_mut(&(k, step, lk)) {
            b.used = tick;
            return Some(b.tile.clone());
        }
        // Built at another step: shown while this one is built.
        let near = tiles
            .built
            .iter()
            .filter(|((bk, _, l), _)| *bk == k && *l == lk)
            .min_by_key(|((_, s, _), _)| (s - step).abs())
            .map(|(_, b)| b.tile.clone());
        if tiles.parsed.contains_key(&k) {
            if tiles.building.insert((k, step, lk)) {
                drop(tiles);
                self.shared.push(Job::Build {
                    entry,
                    k,
                    step,
                    look,
                });
            }
            return near;
        }
        if Self::failing(&tiles, &k) {
            return near;
        }
        if let Some(p) = tiles.pending.get_mut(&k) {
            p.wanted = generation;
            return near;
        }
        tiles.pending.insert(
            k,
            Pending {
                cancel: Arc::new(AtomicBool::new(false)),
                wanted: generation,
            },
        );
        drop(tiles);
        self.shared.push(Job::Fetch {
            entry,
            k,
            vector: true,
        });
        near
    }

    /// The label candidates of a service's tiles `tiles` built for `zoom` (those built so far).
    pub fn labels(
        &self,
        key: &str,
        tiles_in_view: &[TileRef],
        zoom: f64,
    ) -> Vec<Arc<Vec<Candidate>>> {
        let Some(entry) = self.entry(key) else {
            return Vec::new();
        };
        let Some(look) = self
            .shared
            .look
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
        else {
            return Vec::new();
        };
        let step = (zoom * 4.0).round() as i32;
        let lk = look.key();
        let tiles = self
            .shared
            .tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        tiles_in_view
            .iter()
            .filter_map(|t| {
                let k = TileKey {
                    service: entry.id,
                    t: *t,
                };
                tiles
                    .built
                    .get(&(k, step, lk))
                    .or_else(|| {
                        tiles
                            .built
                            .iter()
                            .filter(|((bk, _, l), _)| *bk == k && *l == lk)
                            .min_by_key(|((_, s, _), _)| (s - step).abs())
                            .map(|(_, b)| b)
                    })
                    .map(|b| b.labels.clone())
            })
            .collect()
    }

    /// Waits (pictures and tests) until nothing is on its way: no service
    /// resolving, no tile asked for and not come, no build running; at most
    /// `most`. Whether it settled.
    #[cfg(test)]
    pub fn settle(&self, most: std::time::Duration) -> bool {
        let until = std::time::Instant::now() + most;
        loop {
            let resolving = self
                .shared
                .entries
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .values()
                .any(|e| {
                    matches!(
                        *e.state.lock().unwrap_or_else(PoisonError::into_inner),
                        State::Resolving
                    )
                });
            let busy = {
                let t = self
                    .shared
                    .tiles
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                !t.pending.is_empty() || !t.building.is_empty()
            };
            let queued = !self
                .shared
                .jobs
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_empty();
            if !(resolving || busy || queued) {
                return true;
            }
            if std::time::Instant::now() >= until {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
    }

    /// What the window should be told since it last asked.
    pub fn take_notices(&self) -> Vec<String> {
        std::mem::take(
            &mut *self
                .shared
                .notices
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        )
    }

    /// Reads a service again from its address (“Yeniden yükle”): its state and
    /// its tiles in memory go; the next frame that shows it resolves it anew.
    pub fn reload(&self, key: &str) {
        let Some(entry) = self
            .shared
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(key)
        else {
            return;
        };
        let id = entry.id;
        let mut tiles = self
            .shared
            .tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for (k, p) in &tiles.pending {
            if k.service == id {
                p.cancel.store(true, Ordering::Relaxed);
            }
        }
        tiles.pending.retain(|k, _| k.service != id);
        tiles.pictures.retain(|k, _| k.service != id);
        tiles.picture_bytes = tiles.pictures.values().map(|(_, n, _)| n).sum();
        tiles.parsed.retain(|k, _| k.service != id);
        tiles.built.retain(|(k, _, _), _| k.service != id);
        tiles.failed.retain(|k, _| k.service != id);
        drop(tiles);
        self.shared
            .credits
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id);
        self.shared.tell();
    }

    /// Forgets a service's tiles, in memory and on the disk (“Önbelleği temizle”), and asks again.
    pub fn forget(&self, key: &str) -> usize {
        let Some(entry) = self.entry(key) else {
            return 0;
        };
        {
            let mut tiles = self
                .shared
                .tiles
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let id = entry.id;
            tiles.pictures.retain(|k, _| k.service != id);
            tiles.picture_bytes = tiles.pictures.values().map(|(_, n, _)| n).sum();
            tiles.parsed.retain(|k, _| k.service != id);
            tiles.built.retain(|(k, _, _), _| k.service != id);
            tiles.failed.retain(|k, _| k.service != id);
        }
        let prefix = super::net::cache_prefix(&entry);
        let gone = prefix.map_or(0, |p| self.shared.cache.forget(&p));
        self.shared.tell();
        gone
    }
}

/// The window's message each time tiles came (at most one a frame).
pub fn ready() -> impl iced::futures::Stream<Item = crate::app::Message> {
    use iced::futures::SinkExt;
    let (mut out, stream) = iced::futures::channel::mpsc::channel(1);
    let shared = hub().shared.clone();
    std::thread::spawn(move || {
        loop {
            {
                let mut fresh = shared.fresh.lock().unwrap_or_else(PoisonError::into_inner);
                while !*fresh {
                    fresh = shared
                        .told
                        .wait(fresh)
                        .unwrap_or_else(PoisonError::into_inner);
                }
                *fresh = false;
            }
            if iced::futures::executor::block_on(out.send(crate::app::Message::ServicesReady))
                .is_err()
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
    stream
}
