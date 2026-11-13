//! The installed extensions as an agent sees them (plan/eklenti.md ADIM 18, docs/adr/8057 §3): the resource
//! `kentos://extensions/installed` (the view `kentos kek dump` prints), the overview's short list, and the watch that
//! tells a subscriber the list changed. No extension runs here: the user's registry is read.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use kentos_extension_host::places;
use serde_json::{Value, json};

/// The resource's address.
pub(crate) const INSTALLED: &str = "kentos://extensions/installed";

/// What `kentos://extensions/installed` reads: every installed extension with its state, what the running or ready
/// ones add and what was left out, from the registry in `root` (the user's, `KENTOS_EXTENSIONS_DIR` or a test's).
pub(crate) fn installed(root: Option<&Path>) -> Value {
    let Some(root) = root else {
        return unread(
            Value::Null,
            json!({
                "code": "no_data_folder",
                "message": "Bu bilgisayarda kullanıcı veri klasörü yok (XDG_DATA_HOME ya da HOME); eklenti kurulamaz.",
            }),
        );
    };
    match places::open_to_read(root, env!("CARGO_PKG_VERSION")) {
        Ok(registry) => registry.dump(),
        Err(problem) => unread(json!(root.display().to_string()), json!(problem)),
    }
}

/// The view when the registry cannot be read: empty lists and why.
fn unread(root: Value, problem: Value) -> Value {
    json!({
        "root": root,
        "extensions": [],
        "contributions": [],
        "skipped": [],
        "problem": problem,
    })
}

/// The overview's short list: each installed extension's id, name, version and state; the resource has the rest.
pub(crate) fn short(installed: &Value) -> Value {
    let list: Vec<Value> = installed["extensions"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|e| {
                    json!({
                        "id": e["id"],
                        "name": e["name"],
                        "version": e["version"],
                        "state": e["state"]["state"],
                        "stateLabel": e["stateLabel"],
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    json!(list)
}

/// The registry's files as last read and the view they gave. A change of the files is read again and told only when
/// the view changed too: a file written back the same says nothing.
pub(crate) struct Watch {
    root: Option<PathBuf>,
    stamp: u64,
    view: Value,
}

impl Watch {
    pub(crate) fn new(root: Option<PathBuf>) -> Watch {
        Watch {
            stamp: stamp(root.as_deref()),
            view: installed(root.as_deref()),
            root,
        }
    }

    /// Whether `kentos://extensions/installed` reads differently than when last asked.
    pub(crate) fn changed(&mut self) -> bool {
        let now = stamp(self.root.as_deref());
        if now == self.stamp {
            return false;
        }
        self.stamp = now;
        let view = installed(self.root.as_deref());
        if view == self.view {
            return false;
        }
        self.view = view;
        true
    }
}

/// The registry's, the withdrawn versions' and the organisation's policy files, hashed: what an install, an update, a
/// switch, a removal or IT's change writes; and the logs' sizes and times, so that an extension's new error is told
/// too (docs/adr/8057 §4). A few kilobytes read every two seconds while a client listens.
fn stamp(root: Option<&Path>) -> u64 {
    let mut hasher = DefaultHasher::new();
    let mut files = vec![places::policy_path()];
    if let Some(root) = root {
        let layout = kentos_extension_host::store::Layout {
            root: root.to_path_buf(),
        };
        files.push(layout.registry());
        files.push(layout.revocations());
        // A round of Sorunlu eklentiyi bul holds extensions off (State::Held).
        files.push(layout.bisect());
    }
    for file in files {
        file.hash(&mut hasher);
        std::fs::read(&file).ok().hash(&mut hasher);
    }
    if let Some(root) = root
        && let Ok(logs) = std::fs::read_dir(root.join("logs"))
    {
        let mut seen: Vec<(std::ffi::OsString, u64, Option<std::time::SystemTime>)> = logs
            .flatten()
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                Some((e.file_name(), meta.len(), meta.modified().ok()))
            })
            .collect();
        seen.sort();
        seen.hash(&mut hasher);
    }
    hasher.finish()
}
