//! Where an extension's tools run (docs/adr/8057 §70): every tool on the desktop ([`DESKTOP_RUNTIME`]); one whose
//! descriptor also declares [`HEADLESS_RUNTIME`] in the headless hosts (MCP, the Python library) too, with the same id,
//! schema and result. A headless host registers what runs them only where nothing does yet, so that it never takes the
//! desktop's place in a process that hosts both, and says why an installed tool it does not run is not run there.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use serde_json::{Map, Value};

use super::{PROVIDER, Provider, provider};

/// The runtime every extension's tool has: KentOS's desktop, in the extension's own Python process.
pub const DESKTOP_RUNTIME: &str = "desktop-python";

/// The runtime a descriptor declares for the headless hosts (MCP, the Python library) to run it too, in the same
/// process kind from the same environment.
pub const HEADLESS_RUNTIME: &str = "headless-python";

/// The runtimes a descriptor may name (`capabilities.runtimes`).
pub(super) const RUNTIMES: [&str; 2] = [DESKTOP_RUNTIME, HEADLESS_RUNTIME];

/// Registers what runs the extensions' code unless something already does: a headless host's, which never takes the
/// desktop's place in a process that hosts both. Whether it was registered.
pub fn register_provider_if_none(provider: Arc<dyn Provider>) -> bool {
    match PROVIDER.write() {
        Ok(mut slot) if slot.is_none() => {
            *slot = Some(provider);
            true
        }
        _ => false,
    }
}

/// Whether `provider` is what runs the extensions' code now (a host keeps the lists only while it is).
pub fn is_provider(provider: &Arc<dyn Provider>) -> bool {
    self::provider().is_some_and(|p| Arc::ptr_eq(&p, provider))
}

/// The installed and enabled extensions' tools this host does not run, each with why in a sentence (a headless host's
/// desktop-only tools): a run of one is refused with it before any process starts.
static UNAVAILABLE: RwLock<BTreeMap<String, String>> = RwLock::new(BTreeMap::new());

/// The tools this host does not run now, replaced whole with the tools (`set_tools`).
pub fn set_unavailable(tools: BTreeMap<String, String>) {
    if let Ok(mut slot) = UNAVAILABLE.write() {
        *slot = tools;
    }
}

/// Why `id` is an installed extension's tool this host does not run; none when it is not one.
pub fn unavailable(id: &str) -> Option<String> {
    UNAVAILABLE.read().ok()?.get(id).cloned()
}

/// The runtimes a descriptor names (`capabilities.runtimes`), none when it names none: each one of [`RUNTIMES`] and
/// the desktop's among them (every tool runs there). A name written wrong is said, not taken as the desktop's alone:
/// "headles-python" would have kept the tool out of the headless hosts without a word.
pub(super) fn runtimes(o: &Map<String, Value>) -> Result<Option<Vec<String>>, String> {
    let Some(given) = o.get("capabilities").and_then(|c| c.get("runtimes")) else {
        return Ok(None);
    };
    let known = RUNTIMES.join(", ");
    let list: Vec<String> = given
        .as_array()
        .and_then(|l| l.iter().map(|r| r.as_str().map(str::to_owned)).collect())
        .ok_or_else(|| format!("capabilities.runtimes bir ad listesi olmalı ({known})."))?;
    if let Some(other) = list.iter().find(|r| !RUNTIMES.contains(&r.as_str())) {
        return Err(format!(
            "capabilities.runtimes: “{other}” bilinmiyor ({known})."
        ));
    }
    if !list.iter().any(|r| r == DESKTOP_RUNTIME) {
        return Err(format!(
            "capabilities.runtimes: araç KentOS masaüstünde her zaman çalışır, “{DESKTOP_RUNTIME}” içermeli; MCP'de ve \
             Python'da da çalışacaksa yanına “{HEADLESS_RUNTIME}” yazılır."
        ));
    }
    Ok(Some(list))
}
