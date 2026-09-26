//! What a long read or write of a drawing reports while it works, and how it
//! is stopped (docs/adr/0030, TODOS.md FILE-20): the desktop reads and writes
//! on a thread of its own and the browser in its formats worker; both show the
//! stages and let the user cancel. A cancelled read returns no drawing, so
//! half a file never becomes the document; a cancelled write returns no bytes.

use crate::error::{Code, KcadError};

/// A stage of reading or writing a drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step<'s> {
    /// The file's integrity (SHA-256) is being checked: bytes so far, of all.
    Checking { done: u64, total: u64 },
    /// What the project is, known before its objects are read: its name,
    /// the layer tree's top-level nodes and the number of objects.
    Project {
        name: &'s str,
        layers: usize,
        objects: usize,
    },
    /// Objects read so far.
    Reading { done: usize, total: usize },
    /// Objects written so far.
    Writing { done: usize, total: usize },
    /// The written bytes are read back and compared with the drawing
    /// (`Checking` and `Reading` follow).
    Verifying,
}

/// Hears the steps. Answering `false` stops the work at once: it fails with
/// `Code::Cancelled` and gives nothing back.
pub trait Watch {
    fn step(&mut self, step: Step<'_>) -> bool;
}

/// Hears nothing and never stops (the plain `decode`, `encode`).
pub struct Quiet;

impl Watch for Quiet {
    fn step(&mut self, _: Step<'_>) -> bool {
        true
    }
}

impl<F: FnMut(Step<'_>) -> bool> Watch for F {
    fn step(&mut self, step: Step<'_>) -> bool {
        self(step)
    }
}

/// Objects between two reports of a long list: often enough for a progress
/// bar and a prompt cancel, rare enough to cost nothing.
pub(crate) const EVERY: usize = 2048;
/// Bytes hashed between two reports of the integrity check.
pub(crate) const HASH_CHUNK: usize = 4 << 20;

/// The error of work the watcher stopped.
pub(crate) fn cancelled() -> KcadError {
    KcadError::new(
        Code::Cancelled,
        "İşlem durduruldu; açık çizime ve dosyaya dokunulmadı.",
    )
}

/// Reports a step; stops with `cancelled` when the watcher says so.
pub(crate) fn report(watch: &mut dyn Watch, step: Step<'_>) -> Result<(), KcadError> {
    if watch.step(step) {
        Ok(())
    } else {
        Err(cancelled())
    }
}
