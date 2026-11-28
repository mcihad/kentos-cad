//! Transcendental functions from the numeric base (kentos-num, docs/adr/9403), so every platform writes a file to the same bits (CLAUDE.md §23.4): the std
//! methods call the platform's C library and differ in the last digit from one system to another. The crate's
//! clippy.toml forbids them outside tests.

pub(crate) fn sin(x: f64) -> f64 {
    kentos_num::sin(x)
}

pub(crate) fn cos(x: f64) -> f64 {
    kentos_num::cos(x)
}

pub(crate) fn atan(x: f64) -> f64 {
    kentos_num::atan(x)
}

pub(crate) fn tan(x: f64) -> f64 {
    kentos_num::tan(x)
}

/// The numeric base's: the nearest double (docs/adr/9403).
pub(crate) fn hypot(x: f64, y: f64) -> f64 {
    kentos_num::hypot(x, y)
}

pub(crate) fn atan2(y: f64, x: f64) -> f64 {
    kentos_num::atan2(y, x)
}
