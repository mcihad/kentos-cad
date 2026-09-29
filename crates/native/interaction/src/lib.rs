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
//! - Çizimden ([`pick`], docs/adr/0070): one point for a window, handed to the host;
//! - Öznitelikler's writes ([`properties`], docs/adr/0066): the layer, colour,
//!   symbol, attributes and label through `cad.entities.set`, a geometry
//!   value through `cad.entities.edit`;
//! - the drawing and editing tools of docs/adr/0140: the reshaping, splitting,
//!   cleaning and property-copying tools of phase 1 ([`reshape`], [`split`],
//!   [`cleanup`], [`match_properties`]); phase 2's slice ([`sector`]), points
//!   between two points ([`between`]), the point found from distances,
//!   bearings or lines ([`meeting`]), the angle measured ([`angle`]), the
//!   coordinate read ([`coordinate`]) and the chained and stacked dimensions
//!   ([`dimension_chain`]); phase 3's fence for trim and extend
//!   ([`trim`], `fence`), the two sides and deleted source of offset
//!   ([`offset`]) and the array along a path ([`array_path`]);
//! - docs/adr/0141: the view history ([`view_history`]), the views left by
//!   navigating, for Önceki and Sonraki görünüm; Mesafe ölç's fixed first point
//!   and Alan hesapla's İçine tıkla and Alan olarak çiz ([`path`]); Dik ayak
//!   ölç ([`station_offset`]); Çitle seç, Daireyle seç and İçeren alanı seç
//!   ([`select_fence`], [`select_circle`], [`select_containing`]), which select
//!   what a fence crosses, a circle holds or touches and the areas around a
//!   point, and go back to Seç;
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
pub mod angle;
pub mod arc;
pub mod area;
pub mod array;
pub mod array_path;
pub mod between;
pub mod boundary;
pub mod breaking;
pub mod calc;
pub mod circle;
pub mod cleanup;
pub mod clipboard;
pub mod construction;
pub mod coordinate;
pub mod corner;
pub mod dimension;
pub mod dimension_chain;
pub mod divide;
pub mod donut;
mod edge;
pub mod ellipse;
pub mod erase;
mod faces;
mod fence;
mod format;
pub mod grip_menu;
pub mod hatch;
pub mod lengthen;
pub mod line;
mod log;
pub mod match_properties;
pub mod meeting;
pub mod mirror;
pub mod modify;
pub mod move_copy;
pub mod navigate;
pub mod object;
pub mod object_tracking;
pub mod offset;
mod outlines;
pub mod parallel;
pub mod paste;
pub mod path;
pub mod perpendicular;
pub mod pick;
pub mod pick_objects;
pub mod point;
pub mod point_calc;
mod points;
pub mod polar;
mod prompt;
pub mod properties;
pub mod rectangle;
pub mod regular;
pub mod reshape;
pub mod revcloud;
pub mod rotate;
pub mod rotated;
pub mod scale;
pub mod sector;
pub mod select;
pub mod select_circle;
pub mod select_containing;
pub mod select_fence;
mod selection;
mod session;
pub mod spatial;
pub mod spline;
pub mod split;
pub mod station_offset;
pub mod stretch;
pub mod text;
mod tool;
pub mod trim;
pub mod vertex;
pub mod view_history;

pub use clipboard::Clipboard;
pub use format::{Format, fixed};
pub use kentos_geometry_core::Vec2;
/// The surveying computations the Hesap windows call (docs/adr/0070).
pub use kentos_geometry_core::survey;
/// Measures of a vertex list, from the shared core (the coordinate list's).
pub use kentos_geometry_core::geometry::{angle_deg, bearing_grad, dist, path_length, signed_area};
pub use kentos_geometry_core::store::snap::{SnapHit, SnapKind};
pub use kentos_geometry_core::tools::point_input::Tracking;
/// JavaScript's `trim()`, as typed input is read (the shared grammar).
pub use kentos_geometry_core::tools::point_text::{is_js_space, js_trim};
pub use log::{Level, Line};
pub use prompt::{Prompt, PromptOption, upper_tr};
pub use select::SelectBox;
pub use selection::Selection;
pub use session::Session;
pub use spatial::{
    GripSet, LabelSpot, Spatial, arc_sweep, dimension_layout, full_ellipse, measures, vertices,
};
pub use tool::{Area, Label};
pub use view_history::{ViewHistory, Viewpoint};
pub use tool::{
    Context, Corners, Cursor, DimensionMode, Draft, Flow, LengthenMode, Marker, MarkerShape,
    Memory, Pointer, Preview, Stroke, Tag, TextField, Tone, Tool, View, ViewChange, snap_kinds,
};
