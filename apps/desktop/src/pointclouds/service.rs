//! The point clouds of the desktop (docs/adr/0207 §4, §6): the clouds the
//! scene draws, by their key (`cloud:<hash>`), each file opened off the
//! interface's thread (a COPC as it is, any other through its index in the
//! device's cache, which `index` builds once), and the nodes the points pass
//! asks for read, decoded and coloured on worker threads (the core's
//! `nodes::decode`, `look::colours`), kept within a budget, the least recently
//! used let go first. A request of a frame past is dropped when a newer frame
//! has not asked again.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

use kentos_contracts::{CloudFormat, PointCloudStyle};
use kentos_geometry_core::geom::pointcloud::Octree;
use kentos_pointcloud::copc::Key;
use kentos_pointcloud::source::{Cloud, Kind, Opening};
use kentos_pointcloud::{Step, nodes};
use kentos_render_wgpu::styled::{CloudNode, CloudTrees};

use super::bytes::{Bytes, Origin};

/// Bytes the decoded nodes may take.
const DECODED_BUDGET: usize = 512 * 1024 * 1024;
/// Bytes the coloured nodes may take.
const COLOURED_BUDGET: usize = 256 * 1024 * 1024;

/// A file of a cloud as the service opens it: where it is read from, its
/// format, its name, and an embedded one's bytes.
pub type MemberFile = (Origin, CloudFormat, String, Option<Arc<Vec<u8>>>);

/// What a file of a cloud is doing.
#[derive(Clone, Debug)]
pub enum State {
    /// Not opened yet.
    Waiting,
    /// Its index is being made (`index`), this far (0–1).
    Indexing(f64),
    /// Opened: its COPC (the file, or its index) and how its bytes are read.
    Ready(Arc<Opened>),
    /// Its index was stopped: only its plan shows.
    Stopped,
    Failed(String),
}

/// A COPC opened.
#[derive(Debug)]
pub struct Opened {
    pub cloud: Cloud,
    pub bytes: Bytes,
    pub tree: Octree,
}

/// A file of a cloud.
#[derive(Debug)]
pub struct Member {
    pub id: u64,
    pub origin: Origin,
    pub format: CloudFormat,
    pub name: String,
    /// Its bytes when embedded (the library's).
    pub embedded: Option<Arc<Vec<u8>>>,
    pub state: Mutex<State>,
}

impl Member {
    pub fn state(&self) -> State {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn set(&self, s: State) {
        *self.state.lock().unwrap_or_else(PoisonError::into_inner) = s;
    }

    /// Its bytes opened.
    pub fn bytes(&self) -> Result<Bytes, String> {
        match (&self.origin, &self.embedded) {
            (_, Some(b)) => Ok(Bytes::Memory(b.clone())),
            (Origin::File(p), _) => Bytes::file(p),
            (Origin::Url(u), _) => super::bytes::Remote::open(u).map(Bytes::Remote),
            (Origin::Asset(id), None) => Err(format!(
                "“{id}” kimlikli nokta bulutu projenin kitaplığında yok."
            )),
        }
    }
}

/// A cloud the scene draws: its files, its heights' range.
#[derive(Debug)]
pub struct Entry {
    pub members: Vec<Arc<Member>>,
    pub z: [f64; 2],
    trees: Mutex<Option<(u64, Arc<CloudTrees>)>>,
}

impl Entry {
    /// A count of its files' openings: the trees are made again when it changes.
    fn revision(&self) -> u64 {
        self.members
            .iter()
            .map(|m| match m.state() {
                State::Ready(_) => 1u64,
                _ => 0,
            })
            .fold(0, |a, b| a * 2 + b)
            ^ self.members.len() as u64
    }
}

/// A node decoded for what a look reads (`nodes::Needs::bits`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NodeKey {
    member: u64,
    node: u32,
    needs: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LookKey {
    member: u64,
    node: u32,
    look: String,
}

#[derive(Clone, Debug)]
enum Job {
    Open(Arc<Member>),
    Node {
        member: Arc<Member>,
        node: u32,
        look: String,
        generation: u64,
    },
}

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    asked: HashSet<LookKey>,
}

struct Cache<K, V> {
    map: HashMap<K, (Arc<V>, usize, u64)>,
    bytes: usize,
    tick: u64,
}

impl<K, V> Default for Cache<K, V> {
    fn default() -> Self {
        Cache {
            map: HashMap::new(),
            bytes: 0,
            tick: 0,
        }
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> Cache<K, V> {
    fn get(&mut self, k: &K) -> Option<Arc<V>> {
        self.tick += 1;
        let t = self.tick;
        self.map.get_mut(k).map(|e| {
            e.2 = t;
            e.0.clone()
        })
    }

    fn put(&mut self, k: K, v: Arc<V>, size: usize, budget: usize) {
        self.tick += 1;
        if let Some(old) = self.map.insert(k, (v, size, self.tick)) {
            self.bytes -= old.1;
        }
        self.bytes += size;
        if self.bytes > budget {
            let mut ages: Vec<(u64, K)> = self.map.iter().map(|(k, e)| (e.2, k.clone())).collect();
            ages.sort_unstable_by_key(|(t, _)| *t);
            for (_, k) in ages {
                if self.bytes <= budget {
                    break;
                }
                if let Some(e) = self.map.remove(&k) {
                    self.bytes -= e.1;
                }
            }
        }
    }
}

struct Shared {
    clouds: Mutex<HashMap<String, Arc<Entry>>>,
    /// The clouds each drawing thread's scene shows: the interface's one
    /// thread in the app; in the tests each test's own, so that one test's
    /// scene never lets go of another's clouds.
    shown: Mutex<HashMap<std::thread::ThreadId, HashSet<String>>>,
    queue: Mutex<Queue>,
    work: Condvar,
    decoded: Mutex<Cache<NodeKey, nodes::NodePoints>>,
    coloured: Mutex<Cache<LookKey, CloudNode>>,
    generation: AtomicU64,
    next_id: AtomicU64,
    fresh: Mutex<bool>,
    told: Condvar,
}

/// The clouds of the desktop: one for the app.
pub struct Service {
    shared: Arc<Shared>,
}

static USED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether the drawing has drawn a cloud (the window then listens for nodes).
pub fn in_use() -> bool {
    USED.load(Ordering::Relaxed)
}

/// The app's service, its workers started the first time.
pub fn service() -> &'static Service {
    static SERVICE: OnceLock<Service> = OnceLock::new();
    SERVICE.get_or_init(|| {
        let shared = Arc::new(Shared {
            clouds: Mutex::new(HashMap::new()),
            shown: Mutex::new(HashMap::new()),
            queue: Mutex::new(Queue::default()),
            work: Condvar::new(),
            decoded: Mutex::new(Cache::default()),
            coloured: Mutex::new(Cache::default()),
            generation: AtomicU64::new(0),
            next_id: AtomicU64::new(1),
            fresh: Mutex::new(false),
            told: Condvar::new(),
        });
        let workers = std::thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(1))
            .clamp(1, 4);
        for i in 0..workers {
            let s = shared.clone();
            let _ = std::thread::Builder::new()
                .name(format!("kentos-bulut-{i}"))
                .spawn(move || worker(&s));
        }
        Service { shared }
    })
}

impl Service {
    /// The scene draws the cloud `key` of these files (their origins, formats,
    /// names and, embedded, bytes) and heights: registered once; its files are
    /// opened off this thread. This thread shows it until its scene says
    /// otherwise (`keep_only`): a cloud XYZ sor registered stays.
    pub fn register(&self, key: &str, files: impl FnOnce() -> Vec<MemberFile>, z: [f64; 2]) {
        USED.store(true, Ordering::Relaxed);
        {
            let mut shown = self
                .shared
                .shown
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let mine = shown.entry(std::thread::current().id()).or_default();
            if !mine.contains(key) {
                mine.insert(key.to_owned());
            }
        }
        let mut clouds = self
            .shared
            .clouds
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if clouds.contains_key(key) {
            return;
        }
        let members: Vec<Arc<Member>> = files()
            .into_iter()
            .map(|(origin, format, name, embedded)| {
                Arc::new(Member {
                    id: self.shared.next_id.fetch_add(1, Ordering::Relaxed),
                    origin,
                    format,
                    name,
                    embedded,
                    state: Mutex::new(State::Waiting),
                })
            })
            .collect();
        let mut q = self
            .shared
            .queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for m in &members {
            q.jobs.push_back(Job::Open(m.clone()));
        }
        drop(q);
        self.shared.work.notify_all();
        clouds.insert(
            key.to_owned(),
            Arc::new(Entry {
                members,
                z,
                trees: Mutex::new(None),
            }),
        );
    }

    /// Lets go of every cloud no drawing thread's scene shows: `keep` is this thread's.
    pub fn keep_only(&self, keep: &HashSet<String>) {
        let mut shown = self
            .shared
            .shown
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let thread = std::thread::current().id();
        if shown.get(&thread) != Some(keep) {
            shown.insert(thread, keep.clone());
        }
        let mut clouds = self
            .shared
            .clouds
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let before = clouds.len();
        clouds.retain(|k, _| shown.values().any(|s| s.contains(k)));
        drop(shown);
        if clouds.len() != before {
            let alive: HashSet<u64> = clouds
                .values()
                .flat_map(|e| e.members.iter().map(|m| m.id))
                .collect();
            drop(clouds);
            let mut q = self
                .shared
                .queue
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            q.jobs.retain(|j| match j {
                Job::Open(m) | Job::Node { member: m, .. } => alive.contains(&m.id),
            });
            q.asked.retain(|k| alive.contains(&k.member));
        }
    }

    /// The cloud `key`, if the scene registered it.
    pub fn entry(&self, key: &str) -> Option<Arc<Entry>> {
        self.shared
            .clouds
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .cloned()
    }

    /// Every file of every cloud (the tests wait on their openings).
    #[cfg(test)]
    pub fn members(&self) -> Vec<Arc<Member>> {
        self.shared
            .clouds
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .flat_map(|e| e.members.iter().cloned())
            .collect()
    }

    /// The cloud's octrees: one per opened file.
    pub fn trees(&self, key: &str) -> Option<Arc<CloudTrees>> {
        let entry = self.entry(key)?;
        let rev = entry.revision();
        let mut t = entry.trees.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((r, trees)) = t.as_ref()
            && *r == rev
        {
            return Some(trees.clone());
        }
        let trees = Arc::new(CloudTrees {
            members: entry
                .members
                .iter()
                .map(|m| match m.state() {
                    State::Ready(o) => Some(o.tree.clone()),
                    _ => None,
                })
                .collect(),
            z: entry.z,
        });
        *t = Some((rev, trees.clone()));
        Some(trees)
    }

    /// A node coloured for `look` when made; otherwise it is asked for.
    pub fn node(&self, key: &str, member: u32, node: u32, look: &str) -> Option<Arc<CloudNode>> {
        let entry = self.entry(key)?;
        let m = entry.members.get(member as usize)?.clone();
        let lk = LookKey {
            member: m.id,
            node,
            look: look.to_owned(),
        };
        if let Some(n) = self
            .shared
            .coloured
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&lk)
        {
            return Some(n);
        }
        let generation = self.shared.generation.load(Ordering::Relaxed);
        let mut q = self
            .shared
            .queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if q.asked.insert(lk) {
            q.jobs.push_back(Job::Node {
                member: m,
                node,
                look: look.to_owned(),
                generation,
            });
            drop(q);
            self.shared.work.notify_one();
        }
        None
    }

    /// A frame begins.
    pub fn frame(&self) {
        self.shared.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Asks the window for a frame (a node made, a file opened, an index's progress).
    pub fn wake(&self) {
        *self
            .shared
            .fresh
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = true;
        self.shared.told.notify_one();
    }

    /// Waits until nothing asked for waits, at most `limit` (tests and pictures).
    #[cfg(test)]
    pub fn settle(&self, limit: std::time::Duration) {
        let start = std::time::Instant::now();
        loop {
            let idle = {
                let q = self
                    .shared
                    .queue
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                q.jobs.is_empty()
            };
            let busy = self
                .members()
                .iter()
                .any(|m| matches!(m.state(), State::Waiting | State::Indexing(_)));
            if (idle && !busy) || start.elapsed() > limit {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// Whether nothing is asked for or being made (the measurements wait on it).
    #[cfg(test)]
    pub fn idle(&self) -> bool {
        let q = self
            .shared
            .queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        q.jobs.is_empty() && q.asked.is_empty()
    }

    /// The window's wait for a fresh frame (its subscription).
    pub(crate) fn wait_fresh(&self) {
        let mut fresh = self
            .shared
            .fresh
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        while !*fresh {
            fresh = self
                .shared
                .told
                .wait(fresh)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *fresh = false;
    }
}

/// A COPC opened from its bytes.
pub fn open_copc(bytes: Bytes) -> Result<Opened, String> {
    let mut o = Opening::new(bytes.size());
    let cloud = loop {
        match o.step().map_err(|e| e.0)? {
            Step::Done(c) => break c,
            Step::Need(n) => {
                if n.len > 256 * 1024 * 1024 {
                    return Err("Nokta bulutunun başlığı çok büyük.".into());
                }
                o.put(n.offset, bytes.read(n.offset, n.len)?);
            }
        }
    };
    if cloud.kind != Kind::Copc {
        return Err("Dizin dosyası COPC değil.".into());
    }
    let info = cloud.info.ok_or("COPC bilgisi yok.")?;
    let tree = Octree::new(
        info.center,
        info.halfsize,
        info.spacing,
        cloud.hierarchy.as_ref().map_or_else(Vec::new, |h| {
            h.sorted()
                .into_iter()
                .map(|n| ([n.key.d, n.key.x, n.key.y, n.key.z], n.count))
                .collect()
        }),
    );
    Ok(Opened { cloud, bytes, tree })
}

/// A file opened: a COPC as it is, any other through its index (made once).
fn open_member(m: &Arc<Member>) -> State {
    if m.format == CloudFormat::Copc {
        return match m.bytes().and_then(open_copc) {
            Ok(o) => State::Ready(Arc::new(o)),
            Err(e) => State::Failed(e),
        };
    }
    match super::index::cached(m) {
        Ok(Some(path)) => match Bytes::file(&path).and_then(open_copc) {
            Ok(o) => State::Ready(Arc::new(o)),
            Err(e) => State::Failed(e),
        },
        Ok(None) => {
            super::index::start(m.clone());
            State::Indexing(0.0)
        }
        Err(e) => State::Failed(e),
    }
}

/// Opens a file again once its index is made (`index` calls it).
pub(super) fn indexed(m: &Arc<Member>, path: Result<std::path::PathBuf, String>) {
    let state = match path {
        Ok(p) => match Bytes::file(&p).and_then(open_copc) {
            Ok(o) => State::Ready(Arc::new(o)),
            Err(e) => State::Failed(e),
        },
        Err(e) if e.is_empty() => State::Stopped,
        Err(e) => State::Failed(e),
    };
    m.set(state);
    service().wake();
}

/// A node's points read and decoded with what a look `needs`.
fn decoded(
    shared: &Shared,
    m: &Member,
    o: &Opened,
    node: u32,
    needs: nodes::Needs,
) -> Result<Arc<nodes::NodePoints>, String> {
    let nk = NodeKey {
        member: m.id,
        node,
        needs: needs.bits(),
    };
    if let Some(p) = shared
        .decoded
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&nk)
    {
        return Ok(p);
    }
    let k = o
        .tree
        .keys
        .get(node as usize)
        .ok_or("Böyle bir düğüm yok.")?;
    let key = Key {
        d: k[0],
        x: k[1],
        y: k[2],
        z: k[3],
    };
    let run = o.cloud.node_run(key).ok_or("Düğümün noktası yok.")?;
    let bytes = o.bytes.read(run.need.offset, run.need.len)?;
    // Only what the look reads: a COPC's other layers stay compressed.
    let mut records = Vec::new();
    o.cloud
        .view_records(&run, &bytes, &mut records, needs)
        .map_err(|e| e.0)?;
    let center = o.tree.node_center(*k);
    let points = Arc::new(nodes::decode(
        &o.cloud.layout,
        &records,
        o.cloud.head.scale,
        o.cloud.head.offset,
        center,
        needs,
    ));
    shared
        .decoded
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .put(nk, points.clone(), points.bytes(), DECODED_BUDGET);
    Ok(points)
}

/// The colour a CSS hex colour names (`#rrggbb`); grey otherwise.
fn css_rgb(s: &str) -> [u8; 3] {
    let h = s.trim().trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("80"), 16).unwrap_or(128);
    if h.len() >= 6 {
        [byte(0), byte(2), byte(4)]
    } else {
        [128, 128, 128]
    }
}

fn worker(shared: &Arc<Shared>) {
    loop {
        let job = {
            let mut q = shared.queue.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                match q.jobs.pop_front() {
                    Some(j) => break j,
                    None => q = shared.work.wait(q).unwrap_or_else(PoisonError::into_inner),
                }
            }
        };
        match job {
            Job::Open(m) => {
                let s = open_member(&m);
                m.set(s);
                service().wake();
            }
            Job::Node {
                member,
                node,
                look,
                generation,
            } => {
                let lk = LookKey {
                    member: member.id,
                    node,
                    look: look.clone(),
                };
                // A frame or two past: asked again if still wanted.
                let now = shared.generation.load(Ordering::Relaxed);
                let stale = now > generation + 2;
                let opened = match member.state() {
                    State::Ready(o) => Some(o),
                    _ => None,
                };
                let (style_text, colour) = look.rsplit_once('|').unwrap_or((look.as_str(), ""));
                let style = PointCloudStyle::from_json_text(style_text);
                if let (false, Some(o), Some(style)) = (stale, opened, style)
                    && let Ok(points) = decoded(shared, &member, &o, node, nodes::Needs::of(&style))
                {
                    let mut rgba = Vec::new();
                    kentos_pointcloud::look::colours(&style, &points, css_rgb(colour), &mut rgba);
                    let node = Arc::new(CloudNode {
                        center: points.center,
                        xyz: points.xyz.clone(),
                        rgba,
                    });
                    let size = node.xyz.len() * 4 + node.rgba.len();
                    shared
                        .coloured
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .put(lk.clone(), node, size, COLOURED_BUDGET);
                    service().wake();
                }
                shared
                    .queue
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .asked
                    .remove(&lk);
            }
        }
    }
}
