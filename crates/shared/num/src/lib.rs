//! The numeric base (docs/adr/9403): functions on doubles whose answer is the double nearest to the exact value. Such
//! an answer is the same on every platform by its definition, and an independent reference that follows the same
//! definition gives the same bits, so a result is compared to it without a tolerance.
//!
//! Family by family this takes the place of the JavaScript number behaviour the geometry core was ported with. It sits
//! under every other crate and depends on none: the arithmetic is IEEE 754's (`+`, `-`, `*`, `/`, `sqrt`, each
//! correctly rounded on every processor) and integers; no fused multiply-add, no platform library (`clippy.toml`).
#![forbid(unsafe_code)]
// A number never crashes the program: outside tests every failure is a value.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod hypot;

pub use hypot::hypot;
