//! Pure analytic CAD geometry, shared by the browser (through `kentos-geometry-wasm`)
//! and the server. It is a faithful port of the TypeScript core in
//! `apps/web/src/model/geom` and `apps/web/src/model/geometry.ts`: same formulas, same order of
//! operations, same tolerances, so both sides agree on the golden fixtures in
//! `fixtures/geometry/`. Coordinates are float64 world units (metres);
//! nothing here knows pixels, documents or I/O.
#![forbid(unsafe_code)]
// User data must never crash the core (a panic traps the WASM module): outside
// tests every failure is a value (docs/adr/0008).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
// A faithful port keeps the TypeScript's comparisons and index loops: `t < lo
// || t > hi` lets NaN through where `!(lo..=hi).contains(&t)` would not, and
// `!(r > 0)` is the TypeScript's way of refusing NaN as well as r ≤ 0.
#![allow(
    clippy::manual_range_contains,
    clippy::needless_range_loop,
    clippy::neg_cmp_op_on_partial_ord
)]

pub mod api;
pub mod entity;
pub mod ewkb;
pub mod geom;
pub mod geometry;
pub mod jsmath;
pub mod measure;
pub mod numeric;
pub mod ops;
pub mod predicates;
pub mod processing;
pub mod store;
pub mod survey;
pub mod tessellate;
pub mod text;
pub mod tools;
pub mod triangulate;
pub mod vec2;

pub use vec2::Vec2;
