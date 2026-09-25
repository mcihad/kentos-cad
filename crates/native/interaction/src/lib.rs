//! KentOS interaction: the desktop's tool session (docs/adr/0018, 0021;
//! TODOS.md UX-01, §2.1 `native/interaction`).
//!
//! It is the native counterpart of the web's tools (`apps/web/src/tools`:
//! `ToolManager`, `PointInputTool`, `PathTool`), not a port of their DOM
//! side: the desktop feeds it clicks and typed text and draws what it
//! reports. What makes the two behave alike is shared data:
//!
//! - the interaction traces (`fixtures/interaction/v1`), which the web plays
//!   in a browser and the desktop plays natively, unchanged;
//! - the grammar cases of typed input (`fixtures/point-input/v1`), read by
//!   the web's reader and by the shared Rust one this crate uses.
//!
//! What it holds:
//! - [`Session`]: the ADR 0018 states (idle, running, preview, confirm,
//!   cancel), the running tool and the last one for “repeat”;
//! - [`Prompt`]: the step and its options as data, rendered to the web's
//!   exact Turkish text (`Kapalı alan: sonraki noktayı belirtin [Yay (Y) / …]`);
//! - the closed-area tool ([`polygon`]), which writes one object in one undo
//!   step through the native document (`kentos-domain`);
//! - [`Format`]: numbers as the web shows them in messages and the tag.
//!
//! Pure Rust: no Iced, window system, GPU or runtime (scripts/arch/deps.mjs).
//! Geometry comes from the shared core (`kentos-geometry-core`); none is
//! written here.
#![forbid(unsafe_code)]
// User input must never crash the session.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod format;
mod log;
pub mod polygon;
mod prompt;
mod session;
mod tool;

pub use format::Format;
pub use kentos_geometry_core::Vec2;
pub use kentos_geometry_core::tools::point_input::Tracking;
pub use log::{Level, Line};
pub use prompt::{Prompt, PromptOption, upper_tr};
pub use session::Session;
pub use tool::{Context, Draft, Flow, Pointer, Preview, Tag, Tool, View};
