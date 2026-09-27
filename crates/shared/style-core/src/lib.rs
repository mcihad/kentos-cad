//! The style engine's core, shared by the browser (through `kentos-geometry-wasm`)
//! and the server (CLAUDE.md §14 `style-core`): the style engine that draws
//! symbols on objects. A faithful port of the TypeScript it
//! replaces, slice by slice (docs/adr/0008): same values, same text, same errors.
//! The expression language that processing tools and symbols evaluate is its
//! own crate, `kentos-expression` (docs/adr/0100); `expr` and `js` stay here as
//! re-exports, so the paths callers use keep working.
#![forbid(unsafe_code)]
// User data must never crash the core (a panic traps the WASM module).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod api;
pub mod style;

/// The expression language (`kentos_expression`), under its old path.
pub use kentos_expression as expr;
/// JavaScript's number and text semantics (`kentos_expression::js`), under their old path.
pub use kentos_expression::js;
