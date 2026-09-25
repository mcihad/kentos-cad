//! The geometry of KentOS's own SVG editor (docs/STYLE.md §7), shared by
//! the browser (through `kentos-svg-wasm`, loaded with the editor, CLAUDE.md
//! §20) and the server (CLAUDE.md §14): path data as Bézier nodes, curve
//! helpers and fitting, path booleans on the geometry core's plane overlay,
//! stroke outlines and offsets, and the drawing model's shapes. A faithful
//! port of the TypeScript it replaces (docs/adr/0008 “SVG düzenleyicisi”):
//! same values, same text, same messages.
#![forbid(unsafe_code)]
// User data must never crash the core (a panic traps the WASM module).
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
pub mod arrange;
pub mod bezier;
pub mod boolean;
pub mod export;
pub mod fit;
pub mod import;
pub mod model;
pub mod nodes;
pub mod ops;
pub mod path;
pub mod shape;
pub mod snap;
pub mod stroke;
pub mod trace;
pub mod values;
