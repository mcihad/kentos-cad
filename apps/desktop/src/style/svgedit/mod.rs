//! SVG çizim düzenleyicisi on the desktop (docs/STYLE.md §7, docs/adr/0095;
//! the web's `ui/svgedit/`): KentOS's own editor for the drawings symbols
//! use, pictograms of markers and motifs of pattern fills. Tools and the
//! shape list on the left, the drawing on its canvas in the middle with
//! rulers, guides and snapping, the chosen shape's properties and the
//! Hizala, Dönüştür and Dizi tabs on the right. Its own undo; Kaydet writes
//! the drawing to the library as an SVG asset (a system drawing is saved as
//! the user's copy).
//!
//! The geometry is the SVG core's (`kentos-svg-core`), the one the web runs
//! as WebAssembly: shapes stay its JSON objects field for field, so what an
//! operation makes here is what it makes there, to the drawing's text.
//!
//! - `state.rs`: the editor's state, its history and the panels' settings;
//! - `stage.rs`, `paint.rs`, `camera.rs`: the canvas (Iced's canvas), how
//!   shapes are painted and the view's scale and offset;
//! - `pointer.rs`, `draw_tool.rs`, `node_tool.rs`, `measure.rs`,
//!   `rulers.rs`, `snap.rs`, `hit.rs`: what the pointer does with each tool;
//! - `actions.rs`: path, node and arranging operations, arrays, selection;
//! - `panels/`, `list.rs`, `menus.rs`, `icons.rs`, `view.rs`: the window;
//! - `files/`: open, import, save as, export, document properties, the XML
//!   source, the tracing reference and bitmap tracing;
//! - `keys.rs`, `update.rs`: keys and the application's side.

mod actions;
mod camera;
mod doc;
mod draw_tool;
mod files;
#[cfg(test)]
mod fixture;
mod hit;
mod icons;
mod keys;
mod list;
mod measure;
mod menus;
mod node_tool;
mod paint;
mod panels;
mod pointer;
mod raster;
mod rulers;
#[cfg(test)]
mod screens;
mod snap;
mod stage;
mod state;
#[cfg(test)]
mod tests;
mod update;
mod view;

use std::sync::Arc;

use crate::app::Message;

pub use doc::Drawing;
#[cfg(test)]
pub(crate) use icons::known as known_icon;
pub(crate) use state::{After, Opening, SvgEditor};
pub(crate) use update::dropped;

pub(crate) fn ev(e: Event) -> Message {
    Message::SvgEdit(Box::new(e))
}

/// A change a control makes to the editor: its panels build these, the
/// editor runs them and redraws (one kind of message for the many controls).
#[derive(Clone)]
pub struct Change(pub Arc<dyn Fn(&mut SvgEditor) + Send + Sync>);

impl std::fmt::Debug for Change {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Change")
    }
}

/// The message of a change.
pub(crate) fn change(f: impl Fn(&mut SvgEditor) + Send + Sync + 'static) -> Message {
    ev(Event::Do(Change(Arc::new(f))))
}

/// A tool of the left column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolId {
    Select,
    Node,
    Rect,
    Ellipse,
    Polygon,
    Line,
    Pen,
    Text,
    Measure,
}

/// A tool as the column shows it: its name, key, icon (the web's name) and hint.
pub struct ToolDef {
    pub id: ToolId,
    pub label: &'static str,
    pub key: &'static str,
    pub icon: &'static str,
    pub hint: &'static str,
}

pub const TOOLS: [ToolDef; 9] = [
    ToolDef {
        id: ToolId::Select,
        label: "Seç",
        key: "V",
        icon: "select",
        hint: "Tıkla, sürükle, köşelerden boyutlandır, üstteki düğmeden döndür",
    },
    ToolDef {
        id: ToolId::Node,
        label: "Düğüm",
        key: "A",
        icon: "vertex",
        hint: "Yolun düğümlerini ve kollarını düzenler (yola çift tık da açar)",
    },
    ToolDef {
        id: ToolId::Rect,
        label: "Dikdörtgen",
        key: "R",
        icon: "rectangle",
        hint: "Sürükleyin; Shift kare, Alt merkezden",
    },
    ToolDef {
        id: ToolId::Ellipse,
        label: "Elips",
        key: "E",
        icon: "ellipse",
        hint: "Sürükleyin; Shift daire, Alt merkezden",
    },
    ToolDef {
        id: ToolId::Polygon,
        label: "Çokgen",
        key: "P",
        icon: "regularPolygon",
        hint: "Merkezden sürükleyin; kenar sayısı ve yıldız sağda",
    },
    ToolDef {
        id: ToolId::Line,
        label: "Kırık çizgi",
        key: "L",
        icon: "polyline",
        hint: "Tıklayarak noktalar; çift tık ya da Enter bitirir, ilk noktaya tık kapatır",
    },
    ToolDef {
        id: ToolId::Pen,
        label: "Kalem",
        key: "B",
        icon: "spline",
        hint: "Tık köşe, sürükle eğri düğümü; ilk düğüme tık kapatır, Enter bitirir",
    },
    ToolDef {
        id: ToolId::Text,
        label: "Yazı",
        key: "T",
        icon: "text",
        hint: "Tıklayın, metni sağdan yazın",
    },
    ToolDef {
        id: ToolId::Measure,
        label: "Ölç",
        key: "M",
        icon: "measure",
        hint: "İki noktayı kenetleyerek ölçer (sürükleyin ya da iki tık); yolun üstünde parça boyları",
    },
];

/// What the window asks for.
#[derive(Clone, Debug)]
pub enum Event {
    /// Vazgeç, Esc with nothing left to cancel, ×: closes, or asks first about changes.
    Close,
    /// The unsaved question's answers.
    Discard,
    Stay,
    SaveAndClose,
    Save,
    Name(String),
    Path(String),
    Undo,
    Redo,
    /// The canvas: the pointer, the wheel and its size.
    Stage(stage::Input),
    Tool(ToolId),
    /// − and + (a factor), and Tuvale sığdır.
    ZoomBy(f64),
    Fit,
    /// A panel's control.
    Do(Change),
    /// Enter in a number field: its text gives way to the value's own.
    Settle(String),
    /// A key, with what holds the keyboard.
    Key(crate::keys::KeyPress, crate::style::designer::Focus),
    /// The files: open, import, export, the source, the reference, tracing.
    File(files::Event),
}
