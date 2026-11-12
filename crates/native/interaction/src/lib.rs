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
//! - docs/adr/0142: vertex elevations. Kot ver ([`set_elevation`]) gives the
//!   vertices of lines, polylines, areas and points theirs (Sabit, Artır,
//!   Sıfırla) through `cad.entities.edit`; [`elevation`] reads them for
//!   Öznitelikler, the grip's tag, Koordinat oku and the hover card, and
//!   builds what Kot ver and Öznitelikler write;
//! - docs/adr/0148: Topolojik temizlik ([`topology`]), which puts line work
//!   and area outlines right within a typed tolerance through
//!   `cad.entities.edit`;
//! - docs/adr/0151: Toplu alan ([`polygonize`]), which makes every region
//!   the line work closes an area, its label the attribute, through
//!   `cad.entities.create`;
//! - docs/adr/0162: Bitişik alan ([`adjoin`], a shape of [`path`]), which
//!   writes the region a drawn path closes with the neighbouring areas
//!   through `cad.entities.create`;
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

#[path = "map/address_form.rs"]

pub mod address_form;
#[path = "map/address_reverse.rs"]
pub mod address_reverse;
#[path = "map/adjoin.rs"]
pub mod adjoin;
#[path = "core/align.rs"]
pub mod align;
#[path = "core/align_distribute.rs"]
pub mod align_distribute;
#[path = "viewer_tools/angle.rs"]
pub mod angle;
/// Moved to the commands' crate, where `cad.settings.set` follows a plot scale too (docs/adr/0248 §5).
pub use kentos_native_application::annotation_follow as annotation_scale;
#[path = "core/arc.rs"]
pub mod arc;
#[path = "core/area.rs"]
pub mod area;
#[path = "core/array.rs"]
pub mod array;
#[path = "core/array_edit.rs"]
pub mod array_edit;
#[path = "core/array_path.rs"]
pub mod array_path;
#[path = "core/audit.rs"]
pub mod audit;
#[path = "viewer_tools/between.rs"]
pub mod between;
#[path = "core/block_define.rs"]
pub mod block_define;
#[path = "core/block_insert.rs"]
pub mod block_insert;
#[path = "core/blocks.rs"]
pub mod blocks;
#[path = "map/boundary.rs"]
pub mod boundary;
#[path = "core/breaking.rs"]
pub mod breaking;
#[path = "gis/calc.rs"]
pub mod calc;
#[path = "road/centerline.rs"]
pub mod centerline;
#[path = "viewer_tools/chart_data.rs"]
pub mod chart_data;
#[path = "core/circle.rs"]
pub mod circle;
#[path = "core/cleanup.rs"]
pub mod cleanup;
#[path = "core/clipboard.rs"]
pub mod clipboard;
#[path = "viewer_tools/cloud_query.rs"]
pub mod cloud_query;
#[path = "core/construction.rs"]
pub mod construction;
#[path = "core/continuation.rs"]
pub mod continuation;
#[path = "viewer_tools/coordinate.rs"]
pub mod coordinate;
#[path = "viewer_tools/coordinate_labels.rs"]
pub mod coordinate_labels;
#[path = "core/copy_base.rs"]
pub mod copy_base;
#[path = "core/corner/mod.rs"]
pub mod corner;
#[path = "core/curve_edit/mod.rs"]
pub mod curve_edit;
#[path = "viewer_tools/data_search.rs"]
pub mod data_search;
#[path = "core/dimension.rs"]
pub mod dimension;
#[path = "core/constraint_marks.rs"]
pub mod constraint_marks;
#[path = "core/dimension_bonds.rs"]
pub mod dimension_bonds;
#[path = "core/dimension_chain.rs"]
pub mod dimension_chain;
#[path = "core/dimension_preview.rs"]
mod dimension_preview;
#[path = "core/annotative.rs"]
pub mod annotative;
#[path = "core/dimension_marks.rs"]
pub mod dimension_marks;
#[path = "core/constraint_tool.rs"]
pub mod constraint_tool;
#[path = "core/dimension_reassociate.rs"]
pub mod dimension_reassociate;
#[path = "core/divide.rs"]
pub mod divide;
#[path = "core/donut.rs"]
pub mod donut;
#[path = "core/draw_order.rs"]
pub mod draw_order;
#[path = "core/drawing_extras.rs"]
pub mod drawing_extras;
#[path = "core/edge.rs"]
mod edge;
#[path = "core/edge_shift.rs"]
pub mod edge_shift;
#[path = "arch/elevation.rs"]
pub mod elevation;
#[path = "core/ellipse.rs"]
pub mod ellipse;
#[path = "core/erase.rs"]
pub mod erase;
#[path = "core/faces.rs"]
mod faces;
#[path = "viewer_tools/feature_table.rs"]
pub mod feature_table;
#[path = "viewer_tools/fence.rs"]
mod fence;
#[path = "core/find_replace.rs"]
pub mod find_replace;
/// Numbers as the user reads them: the project crate's (the headless host formats the same).
mod format {
    pub use kentos_project::format::*;
}
#[path = "map/gnss.rs"]
pub mod gnss;
#[path = "core/grip_edit.rs"]
pub mod grip_edit;
#[path = "core/grip_menu.rs"]
pub mod grip_menu;
#[path = "map/ground.rs"]
pub mod ground;
#[path = "viewer_tools/scale_summary.rs"]
pub mod scale_summary;
#[path = "core/hatch.rs"]
pub mod hatch;
#[path = "core/hatch_options.rs"]
pub mod hatch_options;
#[path = "core/hatch_selected.rs"]
pub mod hatch_selected;
#[path = "map/holes.rs"]
pub mod holes;
#[path = "viewer_tools/identify.rs"]
pub mod identify;
#[path = "gis/image_clip.rs"]
pub mod image_clip;
#[path = "core/insert_clip.rs"]
pub mod insert_clip;
#[path = "gis/image_insert.rs"]
pub mod image_insert;
#[path = "core/label_tools.rs"]
pub mod label_tools;
#[path = "core/labels_to_text.rs"]
pub mod labels_to_text;
#[path = "core/layer_move.rs"]
pub mod layer_move;
#[path = "core/layer_tools.rs"]
pub mod layer_tools;
#[path = "viewer_tools/level_view.rs"]
pub mod level_view;
#[path = "core/leader.rs"]
pub mod leader;
#[path = "core/lengthen.rs"]
pub mod lengthen;
#[path = "core/line.rs"]
pub mod line;
#[path = "core/locks.rs"]
pub mod locks;
#[path = "core/log.rs"]
mod log;
#[path = "core/look.rs"]
pub mod look;
#[path = "core/hatch_edit.rs"]
pub mod hatch_edit;
#[path = "core/match_properties.rs"]
pub mod match_properties;
#[path = "viewer_tools/meeting.rs"]
pub mod meeting;
#[path = "core/mirror.rs"]
pub mod mirror;
#[path = "core/modify.rs"]
pub mod modify;
#[path = "core/move_copy.rs"]
pub mod move_copy;
#[path = "viewer_tools/navigate.rs"]
pub mod navigate;
#[path = "viewer_tools/neighbours.rs"]
pub mod neighbours;
#[path = "gis/network/mod.rs"]
pub mod network;
#[path = "core/nurbs_tool.rs"]
pub mod nurbs_tool;
#[path = "core/object.rs"]
pub mod object;
#[path = "viewer_tools/object_tracking.rs"]
pub mod object_tracking;
#[path = "core/offset.rs"]
pub mod offset;
#[path = "core/outlines.rs"]
mod outlines;
#[path = "map/overlap.rs"]
mod overlap;
#[path = "core/paragraph.rs"]
pub mod paragraph;
#[path = "core/parallel.rs"]
pub mod parallel;
#[path = "core/param_grip.rs"]
mod param_grip;
#[path = "core/paste.rs"]
pub mod paste;
#[path = "core/path.rs"]
pub mod path;
#[path = "core/perpendicular.rs"]
pub mod perpendicular;
#[path = "gis/photos.rs"]
pub mod photos;
#[path = "viewer_tools/pick.rs"]
pub mod pick;
#[path = "viewer_tools/pick_objects.rs"]
pub mod pick_objects;
#[path = "road/plan_road.rs"]
pub mod plan_road;
#[path = "core/point.rs"]
pub mod point;
#[path = "map/point_calc/mod.rs"]
pub mod point_calc;
#[path = "map/points.rs"]
mod points;
#[path = "core/polar.rs"]
pub mod polar;
#[path = "map/polygonize.rs"]
pub mod polygonize;
#[path = "core/prompt.rs"]
mod prompt;
#[path = "core/properties.rs"]
pub mod properties;
#[path = "core/quick_dimension.rs"]
pub mod quick_dimension;
#[path = "core/rectangle.rs"]
pub mod rectangle;
#[path = "core/regular.rs"]
pub mod regular;
#[path = "core/reshape.rs"]
pub mod reshape;
#[path = "core/reshape_by.rs"]
pub mod reshape_by;
#[path = "arch/opening_codes_tool.rs"]
pub mod opening_codes_tool;
#[path = "map/parcel_tool.rs"]
pub mod parcel_tool;
#[path = "gis/calc_tables.rs"]
pub mod calc_tables;
#[path = "arch/joinery_list_tool.rs"]
pub mod joinery_list_tool;
#[path = "arch/renovation_legend_tool.rs"]
pub mod renovation_legend_tool;
#[path = "core/escape_tool.rs"]
pub mod escape_tool;
#[path = "arch/plan_dimensions.rs"]
pub mod plan_dimensions;
#[path = "core/revcloud.rs"]
pub mod revcloud;
#[path = "arch/room_grip.rs"]
pub mod room_grip;
#[path = "arch/symbols.rs"]
pub mod symbols;
#[path = "core/rotate.rs"]
pub mod rotate;
#[path = "core/rotated.rs"]
pub mod rotated;
#[path = "core/scale.rs"]
pub mod scale;
#[path = "map/second.rs"]
pub mod second;
#[path = "core/sector.rs"]
pub mod sector;
#[path = "core/segment_edit.rs"]
pub mod segment_edit;
#[path = "viewer_tools/select.rs"]
pub mod select;
#[path = "viewer_tools/snap_names.rs"]
pub mod snap_names;
#[path = "viewer_tools/select_circle.rs"]
pub mod select_circle;
#[path = "viewer_tools/select_containing.rs"]
pub mod select_containing;
#[path = "viewer_tools/select_fence.rs"]
pub mod select_fence;
#[path = "viewer_tools/select_polygon.rs"]
pub mod select_polygon;
#[path = "viewer_tools/select_similar.rs"]
pub mod select_similar;
#[path = "viewer_tools/selectable.rs"]
pub mod selectable;
#[path = "viewer_tools/selection.rs"]
mod selection;
#[path = "core/session.rs"]
mod session;
#[path = "arch/set_elevation.rs"]
pub mod set_elevation;
#[path = "gis/sources.rs"]
pub mod sources;
#[path = "viewer_tools/spatial.rs"]
pub mod spatial;
#[path = "core/spline.rs"]
pub mod spline;
#[path = "core/split.rs"]
pub mod split;
#[path = "core/standard_layer.rs"]
mod standard_layer;
#[path = "arch/arch_layer.rs"]
pub mod arch_layer;
#[path = "road/station_labels.rs"]
pub mod station_labels;
#[path = "viewer_tools/station_offset.rs"]
pub mod station_offset;
#[path = "core/stretch.rs"]
pub mod stretch;
#[path = "core/temporary_refs.rs"]
pub mod temporary_refs;
#[path = "core/ucs_tool.rs"]
pub mod ucs_tool;
/// Moved to the commands' crate, where `cad.styles.edit` writes through it (docs/adr/0248 §4).
pub use kentos_native_application::style_tables;
#[path = "core/styles.rs"]
pub mod styles;
#[path = "core/table.rs"]
pub mod table;
#[path = "core/table_place.rs"]
pub mod table_place;
#[path = "core/extract_place.rs"]
pub mod extract_place;
#[path = "arch/templates.rs"]
pub mod templates;
#[path = "core/text.rs"]
pub mod text;
#[path = "core/text_along.rs"]
pub mod text_along;
#[path = "core/text_layout.rs"]
pub mod text_layout;
#[path = "core/text_file.rs"]
pub mod text_file;
#[path = "gis/time_version.rs"]
pub mod time_version;
#[path = "core/tool.rs"]
mod tool;
#[path = "gis/topology.rs"]
pub mod topology;
#[path = "road/trace_work.rs"]
mod trace_work;
#[path = "core/trim.rs"]
pub mod trim;
#[path = "core/vertex.rs"]
pub mod vertex;
#[path = "map/vertex_points.rs"]
pub mod vertex_points;
#[path = "viewer_tools/view_history.rs"]
pub mod view_history;
#[path = "viewer_tools/view_twist.rs"]
pub mod view_twist;
#[path = "arch/wall.rs"]
pub mod wall;
#[path = "arch/wall_edit_tools.rs"]
pub mod wall_edit_tools;
#[path = "arch/opening_distance.rs"]
pub mod opening_distance;
#[path = "core/bond_refusal.rs"]
pub mod bond_refusal;
#[path = "arch/wall_distance.rs"]
pub mod wall_distance;
#[path = "arch/wall_shift.rs"]
pub mod wall_shift;
#[path = "arch/opening_tools.rs"]
pub mod opening_tools;
#[path = "arch/opening_array.rs"]
pub mod opening_array;
#[path = "arch/space_tools.rs"]
pub mod space_tools;
// Döşeme (docs/adr/0722).
#[path = "arch/slab_tools.rs"]
pub mod slab_tools;
#[path = "arch/space_discover.rs"]
pub mod space_discover;
#[path = "arch/space_number.rs"]
pub mod space_number;
#[path = "arch/architecture_level.rs"]
mod architecture_level;
#[path = "arch/space_level.rs"]
mod space_level;
#[path = "arch/space_sources.rs"]
mod space_sources;
#[path = "arch/arch_tools.rs"]
pub mod arch_tools;
#[path = "arch/beam_tools.rs"]
pub mod beam_tools;
#[path = "arch/railing_tools.rs"]
pub mod railing_tools;
#[path = "arch/roof_part_tools.rs"]
pub mod roof_part_tools;
#[path = "arch/roof_tools.rs"]
pub mod roof_tools;
#[path = "arch/casework.rs"]
pub mod casework;
#[path = "map/grid_system.rs"]
pub mod grid_system;
#[path = "arch/module_place.rs"]
pub mod module_place;
// Oda iç görünüşü: a derived drawing (docs/adr/0799).
#[path = "arch/room_elevation.rs"]
pub mod room_elevation;
#[path = "arch/section_tool.rs"]
pub mod section_tool;
#[path = "viewer_tools/view_crop.rs"]
pub mod view_crop;
#[path = "arch/tag_tools.rs"]
pub mod tag_tools;
#[path = "arch/viewmark_tools.rs"]
pub mod viewmark_tools;
#[path = "arch/wall_from_lines.rs"]
pub mod wall_from_lines;
#[path = "arch/wall_rectangle.rs"]
pub mod wall_rectangle;
#[path = "arch/wall_partition.rs"]
pub mod wall_partition;
#[path = "arch/wall_dimensions.rs"]
pub mod wall_dimensions;
// İç ölçü zinciri (docs/adr/0711).
#[path = "arch/interior_dims.rs"]
pub mod interior_dims;
// Kolonun geçici ölçüleri (docs/adr/0712).
#[path = "arch/column_distance.rs"]
pub mod column_distance;
#[path = "core/tool_settings.rs"]
pub mod tool_settings;

// Planı tanı: an office's exploded plan read as walls and openings (docs/adr/8048 §4).
#[path = "arch/plan_recognize.rs"]
pub mod plan_recognize;

pub use clipboard::Clipboard;
pub use format::{Axes, Format, fixed};
pub use kentos_geometry_core::Vec2;
/// Measures of a vertex list, from the shared core (the coordinate list's).
pub use kentos_geometry_core::geometry::{angle_deg, bearing_grad, dist, path_length, signed_area};
pub use kentos_geometry_core::store::snap::{SnapHit, SnapKind};
/// The surveying computations the Hesap windows call (docs/adr/0070).
pub use kentos_geometry_core::survey;
pub use kentos_geometry_core::tools::point_input::Tracking;
/// JavaScript's `trim()`, as typed input is read (the shared grammar).
pub use kentos_geometry_core::tools::point_text::{is_js_space, js_trim};
pub use locks::{LockAsk, LockPick, LockState, NO_LOCK_EDGE, Toward};
pub use log::{Level, Line};
pub use path::NO_SQUARE_CLOSE;
pub use points::{Aid, ortho_now};
pub use prompt::{Prompt, PromptOption, lower_tr, upper_tr};
pub use select::SelectBox;
pub use selection::{Cycle, Selection, SubObject};
pub use session::{
    LOCKS_GONE, NO_LOCK_REFERENCE, NO_REFERENCE_TOOL, NO_TRACK_TOOL, NO_TRAVEL, Session, TOOLS,
};
pub use spatial::{
    GripSet, LabelSpot, Spatial, arc_sweep, dimension_layout, enclosed, full_ellipse, measures,
    vertices,
};
pub use tool::{Area, CellGhost, ImageFile, InputPreview, Label, TextGhost, frame_of};
pub use tool::{
    Context, Corners, Cursor, DimensionMode, Draft, DraftColor, Flow, LengthenMode, Marker,
    MarkerShape, Memory, Name, Offers, OptionChoice, Overlap, ParagraphField, Pointer, Preview,
    Stroke, Tag, TextField, Tone, Tool, ToolOption, View, ViewChange, default_snap_kinds,
    screen_scale, snap_kinds,
};
pub use view_history::{ViewHistory, Viewpoint};

#[cfg(test)]
#[path = "core/sheet_view_tests.rs"]
mod sheet_view_tests;
