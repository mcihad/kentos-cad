//! Transcendental functions from libm, so the browser (WASM) and the server (native) write a file to the same bits
//! (CLAUDE.md §23.4; the std methods call the platform's C library natively and are forbidden by clippy.toml).

pub(crate) fn sin(x: f64) -> f64 {
    libm::sin(x)
}

pub(crate) fn cos(x: f64) -> f64 {
    libm::cos(x)
}

pub(crate) fn atan(x: f64) -> f64 {
    libm::atan(x)
}
