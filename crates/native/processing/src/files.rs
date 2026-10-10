//! The host's files (docs/adr/0207 §7): what the point cloud tools read and
//! write. A tool never opens a file itself: the host gives each run that
//! may touch files its [`Files`] through the run's feedback
//! ([`crate::Feedback::files`]). The desktop reads a cloud's files where
//! they are (a linked file, an address by HTTP ranges, an embedded file's
//! bytes) and writes the results where the user chose, or beside the
//! source; a host without files (a test, the headless server) gives none,
//! and such a tool refuses, saying why.

use kentos_contracts::{CloudSource, RasterFields};
use kentos_formats::raster::source::{BlockNeed, Reader};
use kentos_pointcloud::ops::convert::Input;

use crate::types::Feedback;

/// A cloud file read whole, in its order.
pub trait CloudRead {
    /// Its header, layout and VLRs (a text cloud's as its parse gives them).
    fn input(&self) -> &Input;
    /// Its next records (of the input's layout), appended to `out`; false at the end.
    fn next(&mut self, out: &mut Vec<u8>) -> Result<bool, String>;
    /// The share of the file read so far, 0..1.
    fn done(&self) -> f64;
}

/// A file being written: its bytes appended in order, then some written again at their places.
pub trait Sink {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn patch(&mut self, at: u64, bytes: &[u8]) -> Result<(), String>;
    /// The file whole (written beside its place, then renamed: a broken run leaves nothing).
    fn finish(self: Box<Self>) -> Result<(), String>;
}

/// Where a source's file is (a cloud's or a raster's): the results go beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Beside<'a> {
    pub file: Option<&'a str>,
    pub url: Option<&'a str>,
    pub asset: Option<&'a str>,
    /// A name for a source that is no file (an interpolation's points: their
    /// layer's, docs/adr/0232 §13); the drawing's folder holds the result.
    pub name: Option<&'a str>,
    /// The result's name when the source's leaves none (“bulut”, “raster”).
    pub fallback: &'static str,
}

impl<'a> Beside<'a> {
    /// A result named after `name` (a layer's), in the drawing's folder.
    pub fn named(name: &'a str, fallback: &'static str) -> Beside<'a> {
        Beside {
            file: None,
            url: None,
            asset: None,
            name: Some(name),
            fallback,
        }
    }
}

impl<'a> From<&'a CloudSource> for Beside<'a> {
    fn from(s: &'a CloudSource) -> Beside<'a> {
        Beside {
            file: s.file.as_deref(),
            url: s.url.as_deref(),
            asset: s.asset.as_deref(),
            name: None,
            fallback: "bulut",
        }
    }
}

impl<'a> From<&'a RasterFields> for Beside<'a> {
    fn from(r: &'a RasterFields) -> Beside<'a> {
        Beside {
            file: r.file.as_deref(),
            url: r.url.as_deref(),
            asset: r.asset.as_deref(),
            name: None,
            fallback: "raster",
        }
    }
}

impl Beside<'_> {
    /// The file's stem as the result names take it: its name without its
    /// folder, query and known extension.
    pub fn stem(&self) -> String {
        let full = self
            .file
            .or(self.url)
            .or(self.asset)
            .or(self.name)
            .unwrap_or("");
        let base = full.rsplit(['/', '\\']).next().unwrap_or(full);
        let base = base.split(['?', '#']).next().unwrap_or(base);
        let lower = base.to_ascii_lowercase();
        let cut = [
            ".copc.laz",
            ".laz",
            ".las",
            ".xyz",
            ".pts",
            ".txt",
            ".csv",
            ".tif",
            ".tiff",
            ".png",
            ".jpg",
            ".jpeg",
            ".nc",
        ]
        .iter()
        .find(|e| lower.ends_with(*e))
        .map_or(base.len(), |e| base.len() - e.len());
        match &base[..cut] {
            "" => self.fallback.to_owned(),
            s => s.to_owned(),
        }
    }
}

/// The blocks a raster's reader keeps for an analysis.
pub use kentos_raster::job::READER_BUDGET as RASTER_READER_BUDGET;

/// Reads a raster block's bytes where they are.
pub type ReadBlock<'f> = Box<dyn Fn(&BlockNeed) -> Result<Vec<u8>, String> + 'f>;

/// Decodes a JPEG stream: its pixels and their components.
pub type DecodeJpeg<'f> = Box<dyn Fn(&[u8]) -> Result<(Vec<u8>, u32), String> + 'f>;

/// A raster's file opened for an analysis (docs/adr/0231 §2): its reader
/// (its header read, its blocks to come), how a block's bytes are read
/// where they are, and the host's JPEG decoder.
pub struct RasterOpen<'f> {
    pub reader: Reader,
    pub block: ReadBlock<'f>,
    pub jpeg: DecodeJpeg<'f>,
}

/// Reads `len` bytes of a file from `offset`.
pub type ReadRun<'f> = Box<dyn Fn(u64, u64) -> Result<Vec<u8>, String> + 'f>;

/// A NetCDF raster's file opened as a cube for a tool that reads it value by
/// value (Zaman serisi, Mesh hesaplayıcı; docs/adr/0243 §9, §10): its
/// header and what the raster's dataset needs (coordinates, its mesh) read,
/// the dataset's part, and how more of the file is read.
pub struct CubeOpen<'f> {
    pub cube: kentos_formats::multidim::cube::Cube,
    pub part: kentos_formats::multidim::cube::Part,
    pub read: ReadRun<'f>,
}

/// The host's files for a run.
pub trait Files: Send + Sync {
    /// A cloud's file opened for a full read.
    fn open_cloud(&self, source: &CloudSource) -> Result<Box<dyn CloudRead + '_>, String>;
    /// Where a result goes: `asked` (a path the user chose; its extension made
    /// `ext`), or, when empty, beside the source `beside` as its name with
    /// `suffix` (the drawing's folder for an address or an embedded file).
    fn output_path(
        &self,
        asked: &str,
        beside: Option<Beside<'_>>,
        suffix: &str,
        ext: &str,
    ) -> Result<String, String>;
    /// A new file at `path` to write.
    fn create(&self, path: &str) -> Result<Box<dyn Sink + '_>, String>;
    /// A COPC of the LAZ or LAS at `from` written at `to` (the index's
    /// builder), its share told through `feedback` from `start` to `end`.
    fn copc(
        &self,
        from: &str,
        to: &str,
        feedback: &mut dyn Feedback,
        start: f64,
        end: f64,
    ) -> Result<(), String>;
    /// Removes a file the run wrote and no longer needs (a COPC's LAZ).
    fn remove(&self, path: &str);
    /// A raster's file opened where it is (a linked file, an embedded
    /// file's bytes, an address by HTTP ranges; docs/adr/0231 §2), its
    /// reader keeping at most [`RASTER_READER_BUDGET`] bytes of blocks.
    fn open_raster(&self, raster: &RasterFields) -> Result<RasterOpen<'_>, String> {
        let _ = raster;
        Err(NO_RASTER_FILES.to_owned())
    }
    /// A NetCDF raster's file opened as a cube (docs/adr/0243 §9, §10).
    fn open_cube(&self, raster: &RasterFields) -> Result<CubeOpen<'_>, String> {
        let _ = raster;
        Err(NO_RASTER_FILES.to_owned())
    }
}

/// Why a tool that needs files does not run here.
pub const NO_FILES: &str =
    "Bu araç dosya okuyup yazar; yalnız masaüstünde çalışır (bu ortamda dosya erişimi yok).";

/// Why a raster tool does not run where the host has no files (a test, the headless server).
pub const NO_RASTER_FILES: &str =
    "Bu araç rasterin dosyasını okuyup sonucu dosyaya yazar; bu ortamda dosya erişimi yok.";

/// `path` with its cloud or raster extension made `ext` (kept as it is when `ext` is empty).
pub fn with_extension(path: &str, ext: &str) -> String {
    if ext.is_empty() {
        return path.to_owned();
    }
    let lower = path.to_ascii_lowercase();
    let cut = [".copc.laz", ".laz", ".las", ".tif", ".tiff", ".vpc", ".nc"]
        .iter()
        .find(|e| lower.ends_with(*e))
        .map_or(path.len(), |e| path.len() - e.len());
    format!("{}{ext}", &path[..cut])
}
