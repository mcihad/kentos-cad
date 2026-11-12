//! NetCDF-4 (HDF5), GRIB and Zarr seen as classic NetCDF for every reader of a NetCDF slice (docs/adr/0645 §5): a
//! file's first bytes say what it is; a Zarr store is a folder (or its metadata file) or an address ending in
//! `.zarr`. The formats core's [`VirtualFile`] reads the store's chunks through the host's file, folder or network;
//! opened files are kept (by path, size and change time, or by address) so a raster drawn tile by tile and a tool
//! stepping a series open each once.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use kentos_formats::multidim::chunked::{Fetch, VirtualFile, open_file, open_zarr};

use crate::remote_host::{self, RangeRead};

/// Files kept opened.
const KEPT: usize = 8;

/// The opened virtual files with their names, the newest last.
type Kept = Mutex<Vec<(String, Arc<VirtualFile>)>>;

/// The opened virtual files by their name.
fn kept() -> &'static Kept {
    static KEPT_FILES: std::sync::OnceLock<Kept> = std::sync::OnceLock::new();
    KEPT_FILES.get_or_init(|| Mutex::new(Vec::new()))
}

fn keep_or_open(
    name: String,
    open: impl FnOnce() -> Result<VirtualFile, String>,
) -> Result<Arc<VirtualFile>, String> {
    if let Some((_, v)) = kept()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|(n, _)| *n == name)
    {
        return Ok(v.clone());
    }
    let v = Arc::new(open()?);
    let mut k = kept().lock().unwrap_or_else(PoisonError::into_inner);
    k.push((name, v.clone()));
    if k.len() > KEPT {
        k.remove(0);
    }
    Ok(v)
}

/// A host's bytes of a file as the chunked readers fetch them.
struct Ranges(Arc<dyn RangeRead>);

impl Fetch for Ranges {
    fn range(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        self.0.read(offset, len)
    }
}

/// A Zarr store in a folder.
struct Folder(PathBuf);

impl Fetch for Folder {
    fn range(&self, _: u64, _: u64) -> Result<Vec<u8>, String> {
        Err("Zarr deposu parça parça okunmaz.".into())
    }

    fn object(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        match std::fs::read(self.0.join(key)) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("“{}/{key}” okunamadı: {e}.", self.0.display())),
        }
    }
}

/// A Zarr store at an address.
struct Address(String);

impl Fetch for Address {
    fn range(&self, _: u64, _: u64) -> Result<Vec<u8>, String> {
        Err("Zarr deposu parça parça okunmaz.".into())
    }

    fn object(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let remote = remote_host::remote().ok_or(remote_host::NO_NETWORK)?;
        remote.object(&format!("{}/{key}", self.0.trim_end_matches('/')))
    }
}

/// A virtual file as a host's bytes.
pub struct Virtual(pub Arc<VirtualFile>);

impl RangeRead for Virtual {
    fn size(&self) -> u64 {
        self.0.size()
    }

    fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        self.0.read(offset, len)
    }
}

/// Whether a path is a Zarr store: a folder holding Zarr's metadata, or one of those files.
pub fn zarr_root(path: &Path) -> Option<PathBuf> {
    const META: [&str; 4] = [".zmetadata", "zarr.json", ".zgroup", ".zarray"];
    if path.is_dir() {
        return META
            .iter()
            .any(|m| path.join(m).is_file())
            .then(|| path.to_path_buf());
    }
    let name = path.file_name()?.to_str()?;
    META.contains(&name)
        .then(|| path.parent().map(Path::to_path_buf))
        .flatten()
}

/// Whether an address names a Zarr store.
pub fn is_zarr_url(url: &str) -> bool {
    let u = url.trim().trim_end_matches('/').to_ascii_lowercase();
    let u = u.split(['?', '#']).next().unwrap_or(&u);
    u.ends_with(".zarr") || u.ends_with("/zarr.json") || u.ends_with("/.zmetadata")
}

/// A folder's Zarr store opened (kept by the folder and its metadata's change time).
pub fn zarr_folder(path: &Path) -> Result<Arc<VirtualFile>, String> {
    let root =
        zarr_root(path).ok_or_else(|| format!("“{}” bir Zarr deposu değil.", path.display()))?;
    let stamp = [".zmetadata", "zarr.json", ".zgroup"]
        .iter()
        .filter_map(|m| std::fs::metadata(root.join(m)).ok())
        .filter_map(|m| m.modified().ok())
        .max()
        .map_or(0, |t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        });
    let name = format!("zarr|{}|{stamp}", root.display());
    keep_or_open(name, || {
        let dir = root.clone();
        let list = move |prefix: &str| -> Option<Vec<String>> {
            let mut out = Vec::new();
            for e in std::fs::read_dir(dir.join(prefix)).ok()? {
                let e = e.ok()?;
                let n = e.file_name().to_string_lossy().into_owned();
                if e.path().is_dir() && !n.starts_with('.') {
                    out.push(n);
                }
            }
            Some(out)
        };
        open_zarr(Arc::new(Folder(root.clone())), &list)
    })
}

/// A Zarr store at an address opened (its metadata consolidated: an address is not listed).
pub fn zarr_address(url: &str) -> Result<Arc<VirtualFile>, String> {
    let base = url.trim().trim_end_matches('/');
    let base = base
        .strip_suffix("/zarr.json")
        .or_else(|| base.strip_suffix("/.zmetadata"))
        .unwrap_or(base)
        .to_owned();
    keep_or_open(format!("zarr-url|{base}"), || {
        open_zarr(Arc::new(Address(base.clone())), &|_| None)
    })
}

/// A file's bytes as classic NetCDF: a NetCDF-4 (HDF5) or GRIB file opened virtual (kept by `name`); none for a classic
/// NetCDF file or another file.
pub fn chunked_view(
    name: &str,
    source: Arc<dyn RangeRead>,
) -> Result<Option<Arc<VirtualFile>>, String> {
    let size = source.size();
    let head = source.read(0, size.min(8))?;
    use kentos_formats::multidim::netcdf::{Sniff, sniff};
    if !matches!(sniff(&head), Sniff::Hdf5 | Sniff::Grib) {
        return Ok(None);
    }
    keep_or_open(format!("file|{name}"), || {
        open_file(Arc::new(Ranges(source.clone())), size, &head)?
            .ok_or_else(|| "Dosya tanınmadı.".to_owned())
    })
    .map(Some)
}

/// A file's name for keeping: its path, size and change time.
pub fn file_name(path: &Path) -> String {
    let m = std::fs::metadata(path).ok();
    let size = m.as_ref().map_or(0, std::fs::Metadata::len);
    let t = m.and_then(|m| m.modified().ok()).map_or(0, |t| {
        t.duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    });
    format!("{}|{size}|{t}", path.display())
}
