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
//! - the drawing tools, each writing through its product command
//!   (`kentos-native-application`) over the native document: the closed
//!   area and the polyline ([`path`], `cad.polygon.create` and
//!   `cad.polyline.create`, docs/adr/0022, 0027), one object in one undo
//!   step; the line ([`line`], `cad.line.create`), one object and one undo
//!   step per segment of its chain; the point ([`point`], `cad.point.create`),
//!   the circle ([`circle`], `cad.circle.create`), the arc ([`arc`],
//!   `cad.arc.create`), the rectangle, the rotated rectangle and the regular
//!   polygon ([`rectangle`], [`rotated`], [`regular`], `cad.polygon.create`;
//!   docs/adr/0032);
//! - selecting and deleting (docs/adr/0029): the [`Selection`], the select
//!   tool that has the pointer while no command runs ([`select`]), the erase
//!   tool ([`erase`], `cad.entities.delete`);
//! - the geometry store kept in step with the document ([`Spatial`]): what
//!   a click picks, a box selects and a point snaps to;
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

pub mod arc;
pub mod circle;
pub mod erase;
mod format;
pub mod line;
mod log;
pub mod path;
pub mod point;
mod points;
mod prompt;
pub mod rectangle;
pub mod regular;
pub mod rotated;
pub mod select;
mod selection;
mod session;
pub mod spatial;
mod tool;

pub use format::Format;
pub use kentos_geometry_core::Vec2;
pub use kentos_geometry_core::store::snap::{SnapHit, SnapKind};
pub use kentos_geometry_core::tools::point_input::Tracking;
/// JavaScript's `trim()`, as typed input is read (the shared grammar).
pub use kentos_geometry_core::tools::point_text::js_trim;
pub use log::{Level, Line};
pub use prompt::{Prompt, PromptOption, upper_tr};
pub use select::SelectBox;
pub use selection::Selection;
pub use session::Session;
pub use spatial::Spatial;
pub use tool::{
    Context, Corners, Draft, Flow, Memory, Pointer, Preview, Stroke, Tag, Tool, View, snap_kinds,
};
