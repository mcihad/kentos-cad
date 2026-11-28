//! The numeric base (docs/adr/9403): functions on doubles whose answer is the double nearest to the exact value. Such
//! an answer is the same on every platform by its definition, and an independent reference that follows the same
//! definition gives the same bits, so a result is compared to it without a tolerance.
//!
//! Family by family this takes the place of the JavaScript number behaviour the geometry core was ported with. It sits
//! under every other crate. A length ([`hypot`]) is its own code: IEEE 754's arithmetic (`+`, `-`, `*`, `/`, `sqrt`,
//! each correctly rounded on every processor) and integers. The elementary functions are a library's, held bit for
//! bit to an independent reference (`elementary`). Nothing here calls the platform's library (`clippy.toml`).
#![forbid(unsafe_code)]
// A number never crashes the program: outside tests every failure is a value.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod elementary;
mod hypot;
mod order;

pub use elementary::{
    acos, asin, asinh, atan, atan2, atanh, cbrt, cos, cosh, erfc, exp, exp2, lgamma, ln, log2,
    log10, pow, powi, sin, sin_cos, sinh, tan, tanh,
};
pub use hypot::hypot;
pub use order::{maximum, maximum_of, minimum, minimum_of, sign};
