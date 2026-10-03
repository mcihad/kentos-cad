//! Sheet layouts on the desktop, pafta düzeni (docs/sheet/design.md §11):
//! the same mode as the web's, built with the KentOS UI components over the
//! shared core (`kentos-sheet`), which computes everything; this crate only
//! paints and asks.
//!
//! - [`Designer`]: the mode's state and its [`Message`]s; every change of the
//!   book is one operation of the core with its inverse (its own undo).
//! - The host places [`Designer::body`] (sheets and items, the paper,
//!   the inspector) where its drawing is while [`Designer::is_active`],
//!   [`Designer::tabs`] under it, [`Designer::status_cells`] in its status
//!   bar, [`Designer::ribbon_groups`] as the contextual Pafta tab and
//!   [`Designer::window`] over everything; it routes the keys no field took
//!   to [`Designer::key`].
//! - [`MapPainter`]: the host paints the maps' content; the stage keeps what
//!   it painted while a map's view stays the same.
//! - [`Effect`]s are what the host does after a message: show the drawing,
//!   ask where to write an export, run one of its own actions, ask its cloud
//!   ([`LibraryRequest`]); its answers come back as [`LibraryEvent`]s.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod binding;
pub mod constraint;
pub mod designer;
pub mod export;
pub mod export_pdf;
pub mod fonts;
pub mod gallery;
pub mod icons;
pub mod inspect;
pub mod inspector_view;
mod interact;
pub mod library;
pub mod message;
pub mod paint;
pub mod painter;
pub mod panels;
pub mod pictures;
pub mod publish_template;
pub mod questions;
pub mod ribbon;
pub mod save_template;
pub mod share_template;
pub mod stage;
pub mod status;
pub mod store;
pub mod text;
mod update;
pub mod variables;
pub mod view;
pub mod view_math;

pub use designer::{Context, Designer, Effect, Say, empty_inputs};
pub use library::{Library, LibraryAccount, LibraryEvent, LibraryRequest, SyncStatus};
pub use message::{ExportKind, InspectorTab, Message, StageEvent, Tool};
pub use painter::{DemoMaps, MapPainter, MapRequest, NoMaps, Painter, lend};
pub use store::{ProjectKey, Store, StoredTemplate, project_key};
