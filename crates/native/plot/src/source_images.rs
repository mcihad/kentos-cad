//! Pin linked CAD image versions before rendering; each physical file supplies one immutable byte set per job.
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use kentos_contracts::{
    Entity,
    encoding::{sha256_hex, sha256_reader_hex},
};
use kentos_domain::Document;
use serde::Serialize;

const CACHE_BYTES: u64 = 512 * 1024 * 1024;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageVersion {
    pub name: String,
    pub sha256: Option<String>,
    pub bytes: u64,
    pub available: bool,
}
#[derive(Debug)]
struct Entry {
    version: ImageVersion,
    problem: Option<String>,
    loaded: Option<Arc<Vec<u8>>>,
}
#[derive(Debug, Default)]
struct State {
    aliases: BTreeMap<String, Option<PathBuf>>,
    entries: BTreeMap<PathBuf, Entry>,
    bytes: u64,
}
#[derive(Debug, Default)]
pub struct SourceImages {
    state: Mutex<State>,
}
fn lock_error() -> String {
    "Çıktının resim kaynağı kilidi okunamadı; yeniden deneyin.".into()
}
impl SourceImages {
    /// Resolution and hashes are frozen once; missing sources stay missing for this snapshot.
    pub fn capture(doc: &Document, drawing: Option<&str>) -> Result<Arc<Self>, String> {
        let mut paths = std::collections::BTreeSet::new();
        for e in doc
            .entities()
            .chain(doc.blocks().iter().flat_map(|b| b.entities.iter()))
        {
            if let Entity::Image(i) = e
                && let Some(file) = &i.image.file
            {
                paths.insert(file.clone());
            }
        }
        let mut state = State::default();
        for file in paths {
            let found = kentos_native_application::links::find(
                &file,
                None,
                drawing,
                &doc.settings().xref_paths,
            )
            .at;
            let path = found.and_then(|p| p.canonicalize().ok());
            state.aliases.insert(file, path.clone());
            let Some(path) = path else { continue };
            if state.entries.contains_key(&path) {
                continue;
            }
            let name = path
                .file_name()
                .map_or_else(|| "resim".into(), |n| n.to_string_lossy().into_owned());
            let read = (|| -> Result<(u64, String), String> {
                if fs::metadata(&path).map_err(|e| e.to_string())?.len()
                    > kentos_contracts::MAX_IMAGE_BYTES
                {
                    return Err("Resim dosyası izin verilen boyutu aşıyor.".into());
                }
                let a = fs::File::open(&path)
                    .and_then(|f| sha256_reader_hex(f.take(kentos_contracts::MAX_IMAGE_BYTES + 1)))
                    .map_err(|e| e.to_string())?;
                let b = fs::File::open(&path)
                    .and_then(|f| sha256_reader_hex(f.take(kentos_contracts::MAX_IMAGE_BYTES + 1)))
                    .map_err(|e| e.to_string())?;
                if a.0 > kentos_contracts::MAX_IMAGE_BYTES || a != b {
                    return Err(
                        "Resim kaynak sürümü yakalanırken değişti; çıktıyı yeniden başlatın."
                            .into(),
                    );
                }
                Ok(a)
            })();
            let (version, problem) = match read {
                Ok((bytes, sha)) => (
                    ImageVersion {
                        name,
                        bytes,
                        sha256: Some(sha),
                        available: true,
                    },
                    None,
                ),
                Err(why) => (
                    ImageVersion {
                        name,
                        bytes: 0,
                        sha256: None,
                        available: false,
                    },
                    Some(why),
                ),
            };
            state.entries.insert(
                path,
                Entry {
                    version,
                    problem,
                    loaded: None,
                },
            );
        }
        Ok(Arc::new(Self {
            state: Mutex::new(state),
        }))
    }
    pub fn versions(&self) -> Result<Vec<ImageVersion>, String> {
        let state = self.state.lock().map_err(|_| lock_error())?;
        let mut out = state
            .entries
            .values()
            .filter(|e| e.loaded.is_some())
            .map(|e| e.version.clone())
            .collect::<Vec<_>>();
        out.sort_by(|a, b| a.name.cmp(&b.name).then(a.sha256.cmp(&b.sha256)));
        Ok(out)
    }
    pub fn fingerprint(&self) -> Result<String, String> {
        let state = self.state.lock().map_err(|_| lock_error())?;
        // Full internal paths bind resolution, but only basenames appear in public provenance.
        let value = serde_json::json!({"aliases":state.aliases,"files":state.entries.iter().map(|(p,e)|(p,&e.version)).collect::<Vec<_>>()});
        serde_json::to_vec(&value)
            .map(|v| sha256_hex(&v))
            .map_err(|e| e.to_string())
    }
    pub(crate) fn read(&self, file: &str) -> Result<Arc<Vec<u8>>, String> {
        let mut state = self.state.lock().map_err(|_| lock_error())?;
        let path=state.aliases.get(file).and_then(Clone::clone).ok_or_else(||format!("“{file}” resmi çıktı başlangıcında bulunamadı; bağlantıyı düzeltip yeniden başlayın."))?;
        let entry = state.entries.get(&path).ok_or_else(lock_error)?;
        if let Some(bytes) = &entry.loaded {
            return Ok(Arc::clone(bytes));
        }
        if let Some(why) = &entry.problem {
            return Err(why.clone());
        }
        let size = entry.version.bytes;
        let expected = entry.version.sha256.clone();
        if state.bytes.saturating_add(size) > CACHE_BYTES {
            return Err("Çıktının bağlı resimleri 512 MB bellek sınırını aşıyor; kaynakları küçültün veya paftaları ayrı işler hâlinde çıkarın.".into());
        }
        let meta = fs::metadata(&path)
            .map_err(|e| format!("Yakalanan resim kaynağı artık okunamıyor: {e}"))?;
        if meta.len() != size {
            return Err("Resim kaynağı çıktı başladıktan sonra değişti; yeniden başlatın.".into());
        }
        let mut bytes = Vec::with_capacity(size as usize);
        fs::File::open(&path)
            .and_then(|f| f.take(size.saturating_add(1)).read_to_end(&mut bytes))
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 != size || Some(sha256_hex(&bytes)) != expected {
            return Err("Resim kaynağı çıktı başladıktan sonra değişti; yeniden başlatın.".into());
        }
        let bytes = Arc::new(bytes);
        state.bytes = state.bytes.saturating_add(size);
        if let Some(entry) = state.entries.get_mut(&path) {
            entry.loaded = Some(Arc::clone(&bytes));
        }
        Ok(bytes)
    }
}

#[cfg(test)]
#[path = "source_images_tests.rs"]
mod tests;
