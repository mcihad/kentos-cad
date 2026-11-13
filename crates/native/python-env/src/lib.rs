//! The user's optional Python (plan/eklenti.md ADIM 18A; EKL-05, PY-OPT-01…14, PY-UV-01…31).
//!
//! KentOS runs without Python: CAD, GIS and every Rust tool work for a user who never turns it on, and while it is off
//! nothing looks for uv or Python and nothing is downloaded (P01). Turned on, uv is found and checked against one
//! compatibility manifest; the environments it prepares are the user's own.
//!
//! - [`compat`]: the manifest (`compat.json`): the least uv, the uv the installation carries (its version and each
//!   platform's file with its SHA-256, for the packaging step), the Python versions, the platforms, uv's official ways in.
//! - [`uv`]: uv found (the chosen path; else the installation's own, `PATH`, the user folders a desktop session's `PATH`
//!   may miss), its version
//!   checked, and uv run with its arguments, never through a shell, with a time limit, a bounded output and none of
//!   the environment's uv or Python settings deciding where it goes.
//! - [`python`]: KentOS's own Python, installed by uv into the user's KentOS folder only when the user asks, found
//!   there alone, and why installing failed said in a sentence.
//! - [`sdk`]: KentOS's Python library (`kentos`) as the installation carries it: a wheel with its version and SHA-256,
//!   installed into environments from that file alone.
//! - [`personal`]: the user's personal environment (P04): where the console runs and NumPy goes; a small uv project
//!   pinned to KentOS's Python and this system, each change previewed aside before the environment follows it.
//! - [`catalog`]: the recommended packages, each with a Turkish line and group, searched offline.
//! - [`extension`]: an extension version's own environment (P05, P07): keyed, previewed, made from its lock, checked,
//!   marked ready last; ready is files on disk, used offline.
//! - [`settings`]: the user's choice (off by default) and their uv path.
//! - [`places`]: where the settings, KentOS's Pythons, uv's cache and the environments are.

pub mod catalog;
pub mod compat;
pub mod extension;
pub mod personal;
pub mod places;
pub mod python;
pub mod sdk;
pub mod settings;
pub mod uv;

#[cfg(test)]
mod tests;
