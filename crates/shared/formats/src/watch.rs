//! Hears a long read (the formats worker's progress messages in the browser,
//! the import window's bar on the desktop) and stops it: a reader asks its
//! watch every so often how far it is, and a `false` answer ends the read
//! before it finishes. The reader then returns `Err(STOPPED)`; nothing it
//! read is kept. The kcad codec has its own (`kentos_kcad::Watch`), with the
//! stages of opening a drawing.

/// What a stopped read returns, so a caller that did not ask for the stop
/// still has a sentence for it.
pub const STOPPED: &str = "Okuma durduruldu; dosyadan hiçbir şey alınmadı.";

/// Hears how far a read is, and whether it goes on.
pub trait Watch {
    /// `done` of `total` units of the read (bytes of the file, as the reader
    /// walks it); `false` stops the read.
    fn step(&mut self, done: u64, total: u64) -> bool;
}

/// A watch that hears nothing and never stops.
pub struct Quiet;

impl Watch for Quiet {
    fn step(&mut self, _done: u64, _total: u64) -> bool {
        true
    }
}

/// A watch from a closure.
pub struct Steps<F: FnMut(u64, u64) -> bool>(pub F);

impl<F: FnMut(u64, u64) -> bool> Watch for Steps<F> {
    fn step(&mut self, done: u64, total: u64) -> bool {
        (self.0)(done, total)
    }
}
