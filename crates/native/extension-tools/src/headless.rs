//! The extensions' İşlemler tools in the headless hosts (plan/eklenti.md ADIM 19; docs/adr/8057 §70): MCP and the
//! Python library run the tools of the user's installed and enabled extensions as the desktop runs them, each in its
//! version's process in the environment the desktop prepared, with the same id, schema and result (PY-TOOL-21).
//!
//! - **Offered from a host's start, never over the desktop's:** [`offer`] is called by a host's entry point (MCP's
//!   program, the Python library's drawing), never by the shared session code the desktop also runs; and it registers
//!   only where nothing runs the extensions' code yet ([`external::register_provider_if_none`]). The lists are kept
//!   only while this provider is the one registered.
//! - **An honest list:** only the tools whose descriptor declares `headless-python` are listed; a run of another
//!   installed tool is refused before any process starts, saying what its descriptor must declare
//!   ([`external::unavailable`]).
//! - **Read, never changed:** the registry is read as `kentos kek dump` reads it (its ready extensions' contributions,
//!   nothing activated) and read again when its files change ([`refresh`]); an environment is prepared on the desktop
//!   only. Before a version's process starts its files are checked against their digest, as activation checks them.
//!   A run is written to the extension's log as the desktop writes it (§73): its start and how it ended, a failure
//!   with its traceback, so that the desktop's Ayrıntı and `kentos://extensions/installed` show a headless run too. The
//!   watchdog's count of crashes stays the desktop's: it belongs to an activation's lease, and this host activates
//!   nothing.
//! - **Python found at the first run:** the user's choice to use it, uv and KentOS's interpreter, as the desktop finds
//!   them; a listing starts nothing.
//! - **No chain of processes:** an extension's own process (`kentos.ext_host` sets [`EXTENSION_PROCESS`]) offers none:
//!   a drawing its code opens does not run another extension's tool round its declared dependencies (§47, PY-RUN-15).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use kentos_extension_host::{Journal, places};
use kentos_processing::Feedback;
use kentos_processing::external::{
    self, DESKTOP_RUNTIME, ExternalTool, Failure, HEADLESS_RUNTIME, Provider,
};
use kentos_python_env::process::Pool;
use kentos_python_env::settings::{self, PythonSettings};
use kentos_python_env::uv::UvStatus;
use serde_json::Value;

use crate::note::Note;
use crate::plan::{Library, Plan, Python, environment_of, read_tools, start_of};
use crate::run::run_tool;

/// Set in an extension's own process (`kentos.ext_host`): a headless host there offers no extension's tools.
pub const EXTENSION_PROCESS: &str = "KENTOS_EXTENSION_PROCESS";

/// What a host does with what a run notes, besides the extension's log: MCP writes it to its standard error, the
/// Python library says it through Python's logging (`kentos.ext`), a failure as a warning with its traceback.
pub type Hook = Arc<dyn Fn(Note) + Send + Sync>;

/// A headless host's extension tools: what it lists and runs, read from the user's registry.
pub struct Headless {
    /// The extension folder (the user's, `KENTOS_EXTENSIONS_DIR`); none on a computer without a data folder.
    root: Option<PathBuf>,
    /// The Python folder (the user's, `KENTOS_PYTHON_DIR`).
    python_root: Option<PathBuf>,
    /// KentOS's Python as given (a host that knows it, the tests); else found at the first run.
    given: Option<Python>,
    found: Mutex<Option<Python>>,
    pool: Arc<Pool>,
    state: RwLock<State>,
    note: Hook,
}

#[derive(Default)]
struct State {
    /// The registry's, the policy's and the Python settings' files as last read; none before the first read.
    stamp: Option<u64>,
    tools: Vec<ExternalTool>,
    plans: HashMap<String, Plan>,
    unavailable: BTreeMap<String, String>,
    /// The descriptors' refusals already said.
    said: BTreeSet<String>,
}

/// Why `tool` is not run in a headless host, and what its descriptor (`descriptor`, its path in the package) must say.
fn desktop_only(tool: &ExternalTool, descriptor: &str) -> String {
    format!(
        "“{}” ({}) yalnız KentOS masaüstünde çalışır: araç tanımı başsız çalışmayı bildirmiyor. MCP'de ve Python'da \
         çalışması için eklentinin {descriptor} dosyasında capabilities.runtimes “{DESKTOP_RUNTIME}” ile birlikte \
         “{HEADLESS_RUNTIME}” içermeli.",
        tool.tool.label, tool.tool.id
    )
}

/// KentOS's own Python as the desktop finds it: the user's choice to use it (`<python>/ayarlar.json`), uv, and the
/// newest interpreter installed in the Python folder `root`; why not, in a sentence saying where it is set right.
pub fn find_python(root: Option<&Path>) -> Result<Python, String> {
    let root = root.ok_or(
        "Bu bilgisayarda kullanıcı veri klasörü yok (XDG_DATA_HOME ya da HOME); KentOS'un Python'u bulunamaz.",
    )?;
    let chosen = PythonSettings::load(root);
    if !chosen.enabled {
        return Err(
            "Python kapalı: KentOS masaüstünde Uygulama ayarları › Python ve eklentiler'den açılır.".into(),
        );
    }
    let uv = match kentos_python_env::uv::detect(chosen.uv_path.as_deref().map(Path::new)) {
        UvStatus::Found { path, .. } => path,
        other => return Err(other.words()),
    };
    let range = &kentos_python_env::compat::compat().python.range;
    let interpreter = kentos_python_env::python::installed(&uv, root, range)?.ok_or(
        "KentOS'un Python'u kurulu değil: KentOS masaüstünde Uygulama ayarları › Python ve eklentiler'den kurulur.",
    )?;
    Ok(Python {
        interpreter,
        root: root.to_path_buf(),
        uv,
    })
}

/// The files whose change changes the tools: the registry, the versions withdrawn, a round of "Sorunlu eklentiyi
/// bul", IT's policy and the Python settings, hashed. A few kilobytes, read when a host asks.
fn stamp(root: Option<&Path>, python_root: Option<&Path>) -> u64 {
    let mut files = vec![places::policy_path()];
    if let Some(root) = root {
        let layout = kentos_extension_host::store::Layout {
            root: root.to_path_buf(),
        };
        files.extend([layout.registry(), layout.revocations(), layout.bisect()]);
    }
    if let Some(python) = python_root {
        files.push(python.join(settings::FILE));
    }
    let mut hasher = DefaultHasher::new();
    for file in files {
        file.hash(&mut hasher);
        std::fs::read(&file).ok().hash(&mut hasher);
    }
    hasher.finish()
}

impl Headless {
    /// The tools of the extensions installed in `root` whose environments are in the Python folder `python_root`, what
    /// a run notes going to `note`. Nothing is read until [`Headless::read`].
    pub fn new(root: Option<PathBuf>, python_root: Option<PathBuf>, note: Hook) -> Headless {
        Headless {
            root,
            python_root,
            given: None,
            found: Mutex::new(None),
            pool: Arc::new(Pool::default()),
            state: RwLock::new(State::default()),
            note,
        }
    }

    /// The same with KentOS's Python given, not looked for.
    pub fn with_python(mut self, python: Python) -> Headless {
        self.given = Some(python);
        self
    }

    fn python_now(&self) -> Option<Python> {
        self.given
            .clone()
            .or_else(|| self.found.lock().ok().and_then(|p| p.clone()))
    }

    /// Reads the registry when its files changed since the last read (an extension installed, enabled, disabled,
    /// updated or removed, IT's policy, the Python settings): the listed tools (those that declare `headless-python`),
    /// their plans and the installed tools not run here; the processes of versions no longer listed are closed.
    /// Whether it read.
    pub fn read(&self) -> bool {
        let now = stamp(self.root.as_deref(), self.python_root.as_deref());
        if self.state.read().is_ok_and(|s| s.stamp == Some(now)) {
            return false;
        }
        // The Python settings may have changed: looked for again at the next run.
        if let Ok(mut found) = self.found.lock() {
            *found = None;
        }
        let environment = self
            .python_now()
            .map(|p| environment_of(&p.interpreter))
            .unwrap_or_default();
        let registry = self
            .root
            .as_deref()
            .and_then(|root| places::open_to_read(root, env!("CARGO_PKG_VERSION")).ok());
        let mut tools = Vec::new();
        let mut plans = HashMap::new();
        let mut unavailable = BTreeMap::new();
        let mut refused = Vec::new();
        if let Some(registry) = &registry {
            let (contributions, _) = registry.planned_contributions();
            let mut listed = read_tools(registry, &contributions, &environment);
            refused = std::mem::take(&mut listed.refused);
            for tool in listed.tools {
                let id = tool.tool.id.clone();
                if tool.headless {
                    if let Some(plan) = listed.plans.remove(&id) {
                        plans.insert(id, plan);
                        tools.push(tool);
                    }
                } else {
                    let descriptor = listed.descriptors.get(&id).cloned().unwrap_or_default();
                    unavailable.insert(id, desktop_only(&tool, &descriptor));
                }
            }
        }
        let keys: BTreeSet<String> = plans.values().map(Plan::key).collect();
        for running in self.pool.running() {
            if !keys.contains(&running.key) {
                self.pool.close(&running.key);
            }
        }
        let mut said = Vec::new();
        if let Ok(mut state) = self.state.write() {
            for (ext, local, why) in refused {
                if state.said.insert(format!("{ext}/{local}/{why}")) {
                    said.push(Note::Failed {
                        ext,
                        part: format!("processing:{local}"),
                        title: local.clone(),
                        code: "tool_descriptor",
                        message: format!("İşlemler aracı “{local}” listelenmedi: {why}"),
                        detail: None,
                        crash: false,
                    });
                }
            }
            state.stamp = Some(now);
            state.tools = tools;
            state.plans = plans;
            state.unavailable = unavailable;
        }
        for note in said {
            (self.note)(note);
        }
        true
    }

    /// The tools listed now.
    pub fn tools(&self) -> Vec<ExternalTool> {
        self.state
            .read()
            .map(|s| s.tools.clone())
            .unwrap_or_default()
    }

    /// The installed tools not run here, each with why.
    pub fn unavailable(&self) -> BTreeMap<String, String> {
        self.state
            .read()
            .map(|s| s.unavailable.clone())
            .unwrap_or_default()
    }

    /// Puts the lists in this process's İşlemler: what `cad.processing.tools` lists and `.run` refuses.
    fn publish(&self) {
        external::set_tools(self.tools());
        external::set_unavailable(self.unavailable());
    }

    /// KentOS's Python: as given, as found before, or looked for now (kept once found); once known, the tools say
    /// where their code runs (a layer one makes records it, docs/adr/8057 §15).
    fn python(&self) -> Result<Python, String> {
        if let Some(python) = self.python_now() {
            return Ok(python);
        }
        let python = find_python(self.python_root.as_deref())?;
        if let Ok(mut found) = self.found.lock() {
            *found = Some(python.clone());
        }
        let environment = environment_of(&python.interpreter);
        if let Ok(mut state) = self.state.write() {
            for tool in &mut state.tools {
                tool.environment = environment.clone();
            }
        }
        if offered().is_some_and(|h| std::ptr::eq(h.as_ref(), self)) && ours() {
            external::set_tools(self.tools());
        }
        Ok(python)
    }

    /// Closes the processes (the host ends).
    pub fn close(&self) {
        self.pool.close_all();
    }

    /// How many of its versions' processes run now.
    pub fn running(&self) -> usize {
        self.pool.running().len()
    }
}

impl Provider for Headless {
    fn run(
        &self,
        tool: &ExternalTool,
        input: Value,
        feedback: &mut dyn Feedback,
    ) -> Result<Value, Failure> {
        let label = &tool.tool.label;
        let plan = self
            .state
            .read()
            .ok()
            .and_then(|s| s.plans.get(&tool.tool.id).cloned())
            .ok_or_else(|| {
                Failure::Failed(format!(
                    "“{label}” artık kurulu ve açık bir eklentinin aracı değil."
                ))
            })?;
        let python = self
            .python()
            .map_err(|why| Failure::Failed(format!("“{label}” çalıştırılamadı: {why}")))?;
        let start = || {
            // Its files as installed, as activation checks them: this host activates nothing.
            kentos_extension_host::verify_installed(&plan.source, &plan.digest)?;
            start_of(&plan, &python, Library::IfFound)
        };
        let note = |n: Note| {
            // The extension's log, as the desktop writes it: Ayrıntı reads a headless run there too (§73).
            if let Some(root) = &self.root {
                Journal::new(root).write(n.record(Some(&plan.version)));
            }
            (self.note)(n)
        };
        run_tool(&self.pool, &plan, &start, tool, input, feedback, &note)
    }
}

static OFFERED: OnceLock<Arc<Headless>> = OnceLock::new();

pub(crate) fn offered() -> Option<&'static Arc<Headless>> {
    OFFERED.get()
}

/// Whether the offered tools' provider is the one this process runs the extensions' code with.
fn ours() -> bool {
    offered().is_some_and(|h| {
        let provider: Arc<dyn Provider> = Arc::<Headless>::clone(h);
        external::is_provider(&provider)
    })
}

/// Offers the tools of the extensions installed in `root` (the user's, `KENTOS_EXTENSIONS_DIR`), their environments in
/// the user's Python folder (`KENTOS_PYTHON_DIR`), to this process's İşlemler: a headless host's start (MCP's program,
/// the Python library's first drawing). The first call decides; a later one reads the registry again when it changed.
/// Whether this host runs them: not in an extension's own process, nor where another provider (the desktop's) was
/// registered first.
pub fn offer(root: Option<PathBuf>, note: Hook) -> bool {
    if std::env::var_os(EXTENSION_PROCESS).is_some() {
        return false;
    }
    offer_with(|| Headless::new(root, kentos_python_env::places::root(), note))
}

/// [`offer`] from the user's own extension folder (`KENTOS_EXTENSIONS_DIR` points elsewhere): the Python library's.
pub fn offer_installed(note: Hook) -> bool {
    offer(places::user_root(), note)
}

/// [`offer`] with the host's tools made by `make` when none were offered yet (a test's folders).
pub fn offer_with(make: impl FnOnce() -> Headless) -> bool {
    let mut first = false;
    let headless = OFFERED.get_or_init(|| {
        first = true;
        Arc::new(make())
    });
    if first {
        let provider: Arc<dyn Provider> = Arc::<Headless>::clone(headless);
        if !external::register_provider_if_none(provider) {
            return false;
        }
    }
    if !ours() {
        return false;
    }
    if headless.read() {
        headless.publish();
    }
    true
}

/// The offered tools again when the user's extensions changed: a host asks before a command runs. Nothing when none
/// were offered, or another provider runs the extensions' code.
pub fn refresh() {
    if let Some(headless) = offered()
        && ours()
        && headless.read()
    {
        headless.publish();
    }
}

/// Closes the offered tools' processes: the host ends.
pub fn close() {
    if let Some(headless) = offered() {
        headless.close();
    }
}
