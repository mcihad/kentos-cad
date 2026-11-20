//! A request's stop switch (docs/adr/9100 §4): the window that asked flips it (a view moved on, Durdur pressed) and the
//! driver reading for it stops between two parts, closing what it opened.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A shared stop switch; clones flip the same one.
#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Cancel {
        Cancel::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
