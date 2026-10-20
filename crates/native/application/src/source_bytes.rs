//! The bytes of a file a project object names (a point cloud's source, a mesh raster's NetCDF): its library asset
//! or its file, found as the project's other links are (docs/adr/0350, 0536, 0552). An address is not reached here.

use std::cell::RefCell;
use std::io::{Read, Seek, SeekFrom};

use kentos_domain::Document;

use crate::links::find;

/// Where a source's bytes are read from.
pub(crate) enum Bytes {
    Memory(Vec<u8>),
    File(RefCell<std::fs::File>, u64),
}

impl Bytes {
    pub(crate) fn size(&self) -> u64 {
        match self {
            Bytes::Memory(b) => b.len() as u64,
            Bytes::File(_, n) => *n,
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
        }
    }
}

/// A source's bytes: its library asset, or its file found as the project's links are; why not. `url` names an
/// address (not read here), `what` what an address would be (for the message).
pub(crate) fn bytes_for(
    doc: &Document,
    drawing: Option<&str>,
    asset: Option<&str>,
    file: Option<&str>,
    url: Option<&str>,
    address_hint: &str,
) -> Result<Bytes, String> {
    if let Some(id) = asset {
        return crate::rasters_georef::asset_bytes(doc, id)
            .map(Bytes::Memory)
            .ok_or_else(|| format!("projenin kitaplığında “{id}” yok"));
    }
    if let Some(path) = file {
        let found = find(path, None, drawing, &doc.settings().xref_paths);
        let at = found
            .at
            .ok_or_else(|| format!("bulunamadı; bakılan yerler: {}", found.tried.join(", ")))?;
        let handle = std::fs::File::open(&at).map_err(|e| format!("açılamadı ({e})"))?;
        let size = handle.metadata().map_err(|e| e.to_string())?.len();
        return Ok(Bytes::File(RefCell::new(handle), size));
    }
    if url.is_some() {
        return Err(address_hint.into());
    }
    Err("kaynağı yok".into())
}
