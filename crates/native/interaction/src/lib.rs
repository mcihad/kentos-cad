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
//! - the modify tools (docs/adr/0037): move and copy ([`move_copy`]),
//!   rotate ([`rotate`]), scale ([`scale`]) and mirror ([`mirror`]) on one
//!   selection-first base ([`modify`]), writing through
//!   `cad.entities.transform`;
//! - the edge, corner and object modify tools (docs/adr/0047): offset
//!   ([`offset`]), trim and extend ([`trim`]), fillet and chamfer
//!   ([`corner`]), break ([`breaking`]), vertex ([`vertex`]), lengthen
//!   ([`lengthen`]) on an edge-picking base, join and explode ([`object`])
//!   on the selection-first one, writing through `cad.entities.edit`;
//!   stretch ([`stretch`], `cad.entities.edit`), the rectangular and polar
//!   arrays ([`array`], [`polar`], `cad.entities.array`) and align
//!   ([`align`], `cad.entities.transform`);
//! - the clipboard (docs/adr/0056): Kes, Panoya kopyala and Özgün
//!   koordinatlara yapıştır ([`clipboard`]) and the paste tool ([`paste`]),
//!   writing through the document as the web's do; Kaydır and Pencere
//!   yakınlaştır ([`navigate`]), which ask the host for view changes;
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

pub mod align;
pub mod arc;
pub mod array;
pub mod breaking;
pub mod circle;
pub mod clipboard;
pub mod construction;
pub mod corner;
pub mod dimension;
pub mod divide;
pub mod donut;
mod edge;
pub mod ellipse;
pub mod erase;
mod format;
pub mod lengthen;
pub mod line;
mod log;
pub mod mirror;
pub mod modify;
pub mod move_copy;
pub mod navigate;
pub mod object;
pub mod offset;
mod outlines;
pub mod parallel;
pub mod paste;
pub mod path;
pub mod perpendicular;
pub mod point;
mod points;
pub mod polar;
mod prompt;
pub mod rectangle;
pub mod regular;
pub mod revcloud;
pub mod rotate;
pub mod rotated;
pub mod scale;
pub mod select;
mod selection;
mod session;
pub mod spatial;
pub mod spline;
pub mod stretch;
pub mod text;
mod tool;
pub mod trim;
pub mod vertex;

pub use clipboard::Clipboard;
pub use format::Format;
pub use kentos_geometry_core::Vec2;
/// Measures of a vertex list, from the shared core (the coordinate list's).
pub use kentos_geometry_core::geometry::{bearing_grad, dist, path_length, signed_area};
pub use kentos_geometry_core::store::snap::{SnapHit, SnapKind};
pub use kentos_geometry_core::tools::point_input::Tracking;
/// JavaScript's `trim()`, as typed input is read (the shared grammar).
pub use kentos_geometry_core::tools::point_text::js_trim;
pub use log::{Level, Line};
pub use prompt::{Prompt, PromptOption, upper_tr};
pub use select::SelectBox;
pub use selection::Selection;
pub use session::Session;
pub use spatial::{LabelSpot, Spatial, dimension_layout, measures, vertices};
pub use tool::{Area, Label};
pub use tool::{
    Context, Corners, Cursor, DimensionMode, Draft, Flow, LengthenMode, Marker, MarkerShape, Memory, Pointer,
    Preview, Stroke, Tag, TextField, Tone, Tool, View, ViewChange, snap_kinds,
};
