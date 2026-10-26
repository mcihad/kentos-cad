//! The bytes of a file a project object names (a point cloud's source, a mesh raster's NetCDF): its library asset
//! or its file, found as the project's other links are (docs/adr/0350, 0536, 0552), or an address's file through the
//! host's network; a mesh's NetCDF-4 (HDF5), GRIB or Zarr data set seen as classic NetCDF (docs/adr/0645 §5).

use std::cell::RefCell;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kentos_domain::Document;

use crate::links::find;
use crate::multidim_source::{self as chunked, Virtual};
use crate::remote_host::{self, RangeRead};

/// Where a source's bytes are read from.
pub(crate) enum Bytes {
    Memory(Vec<u8>),
    File(RefCell<std::fs::File>, u64),
    Shared(Arc<dyn RangeRead>),
}

impl Bytes {
    pub(crate) fn size(&self) -> u64 {
        match self {
            Bytes::Memory(b) => b.len() as u64,
            Bytes::File(_, n) => *n,
            Bytes::Shared(r) => r.size(),
        }
    }

    pub(crate) fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        match self {
            Bytes::Memory(b) => {
                let (a, z) = (offset as usize, (offset + len) as usize);
                b.get(a..z)
                    .map(<[u8]>::to_vec)
                    .ok_or_else(|| "dosyanın dışına okunmak istendi".into())
            }
            Bytes::File(f, _) => {
                let mut f = f.borrow_mut();
                f.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
                let mut out = vec![0; len as usize];
                f.read_exact(&mut out).map_err(|e| e.to_string())?;
                Ok(out)
            }
            Bytes::Shared(r) => r.read(offset, len),
        }
    }
}

/// A source's bytes: its library asset, its file found as the project's links are, or an address's file through the
/// host's network; why not. `address_hint` says why an address is not read where the host reaches no network.
pub(crate) fn bytes_for(
    doc: &Document,
    drawing: Option<&str>,
    asset: Option<&str>,
    file: Option<&str>,
    url: Option<&str>,
    address_hint: &str,
) -> Result<Bytes, String> {
    found_bytes(doc, drawing, asset, file, url, address_hint).map(|(b, _)| b)
}

/// A mesh's NetCDF bytes: as [`bytes_for`], a Zarr store (a folder or an address) and a NetCDF-4 (HDF5) or GRIB file
/// seen as classic NetCDF.
pub(crate) fn netcdf_bytes_for(
    doc: &Document,
    drawing: Option<&str>,
    asset: Option<&str>,
    file: Option<&str>,
    url: Option<&str>,
    address_hint: &str,
) -> Result<Bytes, String> {
    if asset.is_none() {
        if let Some(f) = file {
            let given = Path::new(f);
            let given = match drawing.and_then(|d| Path::new(d).parent()) {
                Some(dir) if given.is_relative() => dir.join(given),
                _ => given.to_path_buf(),
            };
            if chunked::zarr_root(&given).is_some() {
                return Ok(Bytes::Shared(Arc::new(Virtual(chunked::zarr_folder(
                    &given,
                )?))));
            }
        } else if let Some(u) = url.filter(|u| chunked::is_zarr_url(u)) {
            if remote_host::remote().is_none() {
                return Err(address_hint.into());
            }
            return Ok(Bytes::Shared(Arc::new(Virtual(chunked::zarr_address(u)?))));
        }
    }
    let (bytes, name) = found_bytes(doc, drawing, asset, file, url, address_hint)?;
    let shared: Arc<dyn RangeRead> = match bytes {
        Bytes::Shared(r) => r,
        other => Arc::new(Owned(std::sync::Mutex::new(other))),
    };
    Ok(match chunked::chunked_view(&name, shared.clone())? {
        Some(v) => Bytes::Shared(Arc::new(Virtual(v))),
        None => Bytes::Shared(shared),
    })
}

/// A source's own bytes shared with a chunked reader.
struct Owned(std::sync::Mutex<Bytes>);

impl RangeRead for Owned {
    fn size(&self) -> u64 {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .size()
    }

    fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .read(offset, len)
    }
}

/// A source's bytes and its name for keeping a chunked view (`file:<path>|<size>|<time>`, `url:<address>`, `asset:<id>`).
fn found_bytes(
    doc: &Document,
    drawing: Option<&str>,
    asset: Option<&str>,
    file: Option<&str>,
    url: Option<&str>,
    address_hint: &str,
) -> Result<(Bytes, String), String> {
    if let Some(id) = asset {
        return crate::rasters_georef::asset_bytes(doc, id)
            .map(|b| (Bytes::Memory(b), format!("asset:{id}")))
            .ok_or_else(|| format!("projenin kitaplığında “{id}” yok"));
    }
    if let Some(path) = file {
        let found = find(path, None, drawing, &doc.settings().xref_paths);
        let at = found
            .at
            .ok_or_else(|| format!("bulunamadı; bakılan yerler: {}", found.tried.join(", ")))?;
        let handle = std::fs::File::open(&at).map_err(|e| format!("açılamadı ({e})"))?;
        let size = handle.metadata().map_err(|e| e.to_string())?.len();
        let name = format!("file:{}", chunked::file_name(&PathBuf::from(&at)));
        return Ok((Bytes::File(RefCell::new(handle), size), name));
    }
    if let Some(u) = url {
        let remote = remote_host::remote().ok_or_else(|| address_hint.to_owned())?;
        let r = remote.open(u)?;
        return Ok((Bytes::Shared(Arc::from(r)), format!("url:{u}")));
    }
    Err("kaynağı yok".into())
}
