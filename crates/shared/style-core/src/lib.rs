//! The style engine's core, shared by the browser (through `kentos-geometry-wasm`)
//! and the server (CLAUDE.md §14 `style-core`): the expression language that
//! processing tools and symbols evaluate, and the style engine that draws
//! symbols on objects. A faithful port of the TypeScript it
//! replaces, slice by slice (docs/adr/0008): same values, same text, same errors.
#![forbid(unsafe_code)]
// User data must never crash the core (a panic traps the WASM module).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod api;
pub mod expr;
pub mod js;
pub mod style;
