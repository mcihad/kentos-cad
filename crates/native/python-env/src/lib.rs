//! The user's optional Python (plan/eklenti.md ADIM 18A; EKL-05, PY-OPT-01…14, PY-UV-01…31).
//!
//! KentOS runs without Python: CAD, GIS and every Rust tool work for a user who never turns it on, and while it is off
//! nothing looks for uv or Python and nothing is downloaded (P01). Turned on, uv is found and checked against one
//! compatibility manifest; the environments it prepares are the user's own.
//!
//! - [`compat`]: the manifest (`compat.json`): the least uv, the Python versions, the platforms, uv's official ways in.
//! - [`uv`]: uv found (the chosen path, `PATH`, the user folders a desktop session's `PATH` may miss), its version
//!   checked, and uv run with its arguments, never through a shell, with a time limit, a bounded output and none of
//!   the environment's uv or Python settings deciding where it goes.
//! - [`settings`]: the user's choice (off by default) and their uv path.
//! - [`places`]: where the settings and the environments are.

pub mod compat;
pub mod places;
pub mod settings;
pub mod uv;

#[cfg(test)]
mod tests;
