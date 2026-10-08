//! The host's files (docs/adr/0207 §7): what the point cloud tools read and
//! write. A tool never opens a file itself: the host gives each run that
//! may touch files its [`Files`] through the run's feedback
//! ([`crate::Feedback::files`]). The desktop reads a cloud's files where
//! they are (a linked file, an address by HTTP ranges, an embedded file's
//! bytes) and writes the results where the user chose, or beside the
//! source; a host without files (a test, the headless server) gives none,
//! and such a tool refuses, saying why.

use kentos_contracts::CloudSource;
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
        beside: Option<&CloudSource>,
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
}

/// Why a tool that needs files does not run here.
pub const NO_FILES: &str =
    "Bu araç dosya okuyup yazar; yalnız masaüstünde çalışır (bu ortamda dosya erişimi yok).";
