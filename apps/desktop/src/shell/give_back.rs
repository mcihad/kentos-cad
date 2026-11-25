//! Memory a large job freed, given back to the system (plan/render.md RM-19, docs/adr/9235 §3).
//!
//! glibc keeps the memory a thread freed in that thread's arena: a drawing read on the open's own thread, then made
//! the document, left the process's resident memory at the reading's peak (a city plan's NCZ: the reading peaked at
//! 1 998 MB and the document holds 1 055 MB; the rest stayed the process's, free). Nothing trims an arena but the
//! allocator's own threshold at its top. The open's thread gives back what it freed before the app takes the drawing;
//! a moment after a drawing is shown and after a large styled build, a thread of its own does it (`malloc_trim`), off
//! the interface's thread. Elsewhere there is nothing to do: Windows' heap gives large blocks back as they are freed.

#[cfg(all(target_os = "linux", target_env = "gnu"))]
use std::sync::atomic::{AtomicBool, Ordering};

/// A give-back waits this long: the job's last frees (the first frame's build after an open) come first.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
const WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// One give-back asked and not yet made: more asks meanwhile wait for it.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
static ASKED: AtomicBool = AtomicBool::new(false);

/// Gives the allocator's free pages back to the system a moment from now, once however often asked meanwhile.
pub fn soon() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    if !ASKED.swap(true, Ordering::AcqRel) {
        let started = std::thread::Builder::new()
            .name("kentos-give-back".into())
            .spawn(|| {
                std::thread::sleep(WAIT);
                ASKED.store(false, Ordering::Release);
                now();
            });
        if started.is_err() {
            ASKED.store(false, Ordering::Release);
        }
    }
}

/// Gives the allocator's free pages back to the system now.
pub fn now() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        // SAFETY: glibc's malloc_trim takes a pad and only returns free memory to the system.
        unsafe {
            malloc_trim(0);
        }
    }
}
