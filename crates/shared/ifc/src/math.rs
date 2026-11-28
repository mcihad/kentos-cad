//! Transcendental functions from libm, so every platform writes a file to the same bits (CLAUDE.md §23.4): the std
//! methods call the platform's C library and differ in the last digit from one system to another. The crate's
//! clippy.toml forbids them outside tests.

pub(crate) fn sin(x: f64) -> f64 {
    libm::sin(x)
}

pub(crate) fn cos(x: f64) -> f64 {
    libm::cos(x)
}

pub(crate) fn atan(x: f64) -> f64 {
    libm::atan(x)
}

pub(crate) fn tan(x: f64) -> f64 {
    libm::tan(x)
}

pub(crate) fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

pub(crate) fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}
