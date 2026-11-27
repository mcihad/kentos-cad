//! KentOS's headless command host (TODOS.md §14–15, docs/adr/0130): a
//! drawing without a window, and everything a program may do with it
//! through the product command system.
//!
//! - [`Session`]: a drawing opened from a `.kcad` file (v2, or v1 JSON) or
//!   started as a new project, saved as `.kcad` v2 the way the desktop saves.
//! - [`Session::run`]: a catalog command by name, `validate`, `plan` or
//!   `execute`, its input and answer as JSON (the catalog's schemas). The
//!   commands are the desktop's own handlers (`kentos_native_application`),
//!   so a script, an agent and the window write alike, one undo step each.
//! - [`catalog`]: every command with its schemas, hosts and examples; the
//!   ones the server runs are listed too, and refused here with the reason.
//! - Queries ([`Session::layers`], [`Session::entities`], [`Session::entity`])
//!   and measures ([`Session::measure`]) read without writing.
//! - [`Session::export_dxf`]: the drawing as a DXF file, as the desktop's
//!   DXF dışa aktar writes it ([`export`], docs/adr/0252); [`Session::export_pdf`]:
//!   the project's sheet book as a PDF (`kentos_plot::book`).
//! - [`rpc::call`]: the same by name over a document another host owns (the
//!   desktop's open drawing, asked by its Python console).
//!
//! JSON crosses the boundary on purpose: the Python SDK (`kentos.cad`) and
//! the MCP server speak the catalog's own wire form, which the web and the
//! server speak too. Objects are named by their persistent ids (UUID text),
//! never by their slots, which mean nothing once a drawing is closed.
#![forbid(unsafe_code)]
// A build without every module (docs/adr/8054 §11) leaves helpers only the missing ones call.
#![cfg_attr(
    not(all(
        feature = "mod-arch",
        feature = "mod-map",
        feature = "mod-gis",
        feature = "mod-road"
    )),
    allow(dead_code, unused_imports)
)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

/// The agents' tools, one surface for the MCP server and the desktop's chat (docs/adr/8056 §2).
pub mod agent_tools;
mod dispatch;
mod error;
pub mod export;
pub mod exchange_check;
pub mod exchange_matrix;
mod exchange_parts;
mod cad_shape_files;
pub mod file_commands;
pub mod files;
pub mod gis_export;
pub mod package;
pub mod publish;
#[cfg(all(feature = "mod-arch", feature = "mod-gis"))]
mod publish_bim;
mod publish_status;
#[cfg(feature = "mod-arch")]
mod package_review;
mod publish_update;
pub mod recover;
#[cfg(feature = "mod-gis")]
pub mod regulation;
mod sheet_resources;
pub mod sheet_preflight;
mod delivery_integrity;
/// The delivery package's open files and their losses (docs/adr/2780).
pub mod package_exchange;
mod package_data;
mod package_resources;
mod package_sources;
use kentos_scene_query::query;
pub mod rpc;
mod session;
pub mod selection;
pub mod maplibre_export;
pub mod shp_export;
pub mod sprite;
pub mod veil;

pub use dispatch::{Op, call_envelope, commands, with_own_file};
pub use error::HeadlessError;
pub use query::{
    Basis, Conventions, MEASURE_MAX, Measure, MeasureItem, Measures, Method, Page, Totals, Units,
    conventions, measure_by, units,
};
pub use session::{NewDrawing, Session, new_drawing};
pub use veil::Veil;

/// Reads the fonts folders and gives the core their faces, once, before the first text is laid out (docs/adr/9313):
/// `kentos`, `kentos-mcp` and the Python module call it when they start, as the desktop does.
pub fn load_fonts() {
    kentos_native_application::font_folders::load();
}

/// The command catalog as JSON: every product command, its version, title,
/// summary, hosts, permissions, input and output schemas and examples
/// (`kentos_commands::catalog`, the file the web and the server read).
pub fn catalog() -> serde_json::Value {
    serde_json::to_value(kentos_commands::catalog::catalog()).unwrap_or(serde_json::Value::Null)
}

/// A product command's title as the catalog gives it (`Kapalı alan oluştur` for `cad.polygon.create`): a list naming
/// an agent's job by its command (the desktop's Çalışan işler). The catalog is read once, on a thread of its own, the
/// first time a title is asked; until then none, and the caller says the command's id. The interface thread never
/// waits for it: building the catalog's schemas took two seconds in a debug build, and the desktop's first agent job
/// answered that much later (measured in the real window, 16 November 2026).
pub fn command_title(id: &str) -> Option<&'static str> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static TITLES: std::sync::OnceLock<std::collections::HashMap<String, String>> =
        std::sync::OnceLock::new();
    static ASKED: AtomicBool = AtomicBool::new(false);
    if let Some(titles) = TITLES.get() {
        return titles.get(id).map(String::as_str);
    }
    if !ASKED.swap(true, Ordering::Relaxed) {
        let _ = std::thread::Builder::new()
            .name("kentos-komut-adlari".into())
            .spawn(|| {
                TITLES.get_or_init(|| {
                    kentos_commands::catalog::catalog()
                        .commands
                        .into_iter()
                        .map(|c| (c.id, c.title))
                        .collect()
                });
            });
    }
    None
}
