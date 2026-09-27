//! The SVG editor's state (the web's `SvgEditor` fields, `CanvasOptions`
//! and `PanelState`): the drawing and what is chosen, the tool, the canvas
//! options, the panels' settings that outlive a redraw, the history of the
//! drawing's texts (one undo step per interactive change, typing into one
//! field within a second merged), the name and category of the foot, and
//! what the window said.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use iced::widget::canvas;
use kentos_native_style::library::Source;
use kentos_svg_core::shape::{Obj, Pt};
use kentos_svg_core::snap::Kind;
use kentos_svg_core::stroke::Join;

use super::camera::Camera;
use super::doc::Drawing;
use super::draw_tool::DrawTool;
use super::files::{FileState, Pending};
use super::measure::Measure;
use super::node_tool::NodeTool;
use super::pointer::{Click, Op, Pick, Press};
use super::rulers::Rulers;
use super::snap::Snapper;
use super::ToolId;
use crate::app::Dialog;

/// Steps the history keeps.
const HISTORY: usize = 100;
/// Typing into one field within this is one undo step.
const MERGE: Duration = Duration::from_secs(1);

/// Everything but the box middles, which crowd small drawings (`DEFAULT_SNAPS`).
pub const DEFAULT_SNAPS: [Kind; 11] = [
    Kind::Cusp,
    Kind::Smooth,
    Kind::Mid,
    Kind::Intersection,
    Kind::BboxCorner,
    Kind::BboxCentre,
    Kind::Centre,
    Kind::Perpendicular,
    Kind::Tangent,
    Kind::Guide,
    Kind::Page,
];

/// Every snap kind in the order the menu and the canvas panel list them, with its name (`SNAP_KINDS`).
pub const SNAP_KINDS: [(Kind, &str); 12] = [
    (Kind::Cusp, "Köşe düğüm"),
    (Kind::Smooth, "Yumuşak düğüm"),
    (Kind::Mid, "Parça ortası"),
    (Kind::Intersection, "Kesişim"),
    (Kind::BboxCorner, "Kutu köşesi"),
    (Kind::BboxMid, "Kutu kenar ortası"),
    (Kind::BboxCentre, "Kutu merkezi"),
    (Kind::Centre, "Nesne merkezi"),
    (Kind::Perpendicular, "Dik"),
    (Kind::Tangent, "Teğet"),
    (Kind::Guide, "Kılavuz"),
    (Kind::Page, "Tuval kenarı ve ortası"),
];

/// The canvas's options (`CanvasOptions`).
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub grid: f64,
    pub snap_grid: bool,
    /// Snapping to shapes, guides and the canvas at all (the kinds choose which).
    pub snap_objects: bool,
    pub snap_kinds: Vec<Kind>,
    pub rulers: bool,
    pub tile: bool,
    pub sides: f64,
    pub star: bool,
    /// Preview colours: the symbol's colour, its second colour, the paper.
    pub ink: String,
    /// The ink follows the theme until a colour is picked.
    pub ink_auto: bool,
    pub second: String,
    pub paper: String,
}

impl Options {
    /// `defaultOptions`: a grid of a twentieth of the width, every snap but the box middles.
    pub fn new(doc: &Drawing, ink: &str, paper: &str) -> Options {
        Options {
            grid: default_grid(doc.width),
            snap_grid: true,
            snap_objects: true,
            snap_kinds: DEFAULT_SNAPS.to_vec(),
            rulers: true,
            tile: false,
            sides: 6.0,
            star: false,
            ink: ink.to_owned(),
            ink_auto: true,
            second: "#2B83BA".to_owned(),
            paper: paper.to_owned(),
        }
    }
}

/// The panels' starting distance (offset, corner, array gaps): a fiftieth of the width, at least 0.1 (`panelUnit`).
pub fn panel_unit(width: f64) -> f64 {
    use kentos_native_style::classify::js_round;
    (js_round(width / 50.0 * 100.0) / 100.0).max(0.1)
}

/// `Math.max(1, Math.round(width / 20))`.
pub fn default_grid(width: f64) -> f64 {
    kentos_native_style::classify::js_round(width / 20.0).max(1.0)
}

/// The right column's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Props,
    Align,
    Transform,
    Array,
}

/// The Dönüştür tab's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformKind {
    Move,
    Scale,
    Rotate,
    Skew,
    Matrix,
}

/// Where a rotation, a polar array or a mirror turns about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Box,
    Canvas,
    Point,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayKind {
    Rect,
    Polar,
    Mirror,
}

/// The Dönüştür tab's values, kept while the editor is open.
#[derive(Clone, Debug, PartialEq)]
pub struct TransformUi {
    pub kind: TransformKind,
    pub relative: bool,
    pub x: f64,
    pub y: f64,
    pub sx: f64,
    pub sy: f64,
    pub lock: bool,
    pub deg: f64,
    pub ccw: bool,
    /// A box point (`tl` … `br`, `c`), or None for the point picked on the canvas.
    pub about: Option<&'static str>,
    pub point: Option<Pt>,
    pub anchor: &'static str,
    pub ax: f64,
    pub ay: f64,
    pub m: [f64; 6],
    pub separately: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RectUi {
    pub rows: f64,
    pub cols: f64,
    pub dx: f64,
    pub dy: f64,
    /// Boşluk (between boxes) or Adım (centre to centre).
    pub gap: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolarUi {
    pub count: f64,
    pub angle: f64,
    pub rotate: bool,
    pub ccw: bool,
    pub at: At,
    pub point: Option<Pt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MirrorUi {
    /// `v`, `h` or `angle`.
    pub axis: &'static str,
    pub deg: f64,
    pub at: At,
    pub point: Option<Pt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayUi {
    pub kind: ArrayKind,
    pub rect: RectUi,
    pub polar: PolarUi,
    pub mirror: MirrorUi,
    pub preview: bool,
}

/// The panels' settings, kept while the editor is open (`PanelState`).
#[derive(Clone, Debug, PartialEq)]
pub struct PanelState {
    pub tab: Tab,
    /// `selection`, `first`, `last`, `biggest`, `smallest` or `canvas`.
    pub align_to: &'static str,
    pub align_as_one: bool,
    pub offset: f64,
    pub offset_join: Join,
    pub simplify: f64,
    pub corner: f64,
    pub transform: TransformUi,
    pub array: ArrayUi,
    /// The box's width and height keep their ratio (the web forgot it on every redraw).
    pub box_lock: bool,
    /// The angle of the box's ↺ and ↻.
    pub turn: f64,
}

impl PanelState {
    pub fn new(doc: &Drawing) -> PanelState {
        let unit = panel_unit(doc.width);
        PanelState {
            tab: Tab::Props,
            align_to: "selection",
            align_as_one: false,
            offset: unit,
            offset_join: Join::Round,
            simplify: 0.2,
            corner: unit,
            transform: TransformUi {
                kind: TransformKind::Move,
                relative: true,
                x: 0.0,
                y: 0.0,
                sx: 100.0,
                sy: 100.0,
                lock: true,
                deg: 90.0,
                ccw: true,
                about: Some("c"),
                point: None,
                anchor: "c",
                ax: 0.0,
                ay: 0.0,
                m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                separately: false,
            },
            array: ArrayUi {
                kind: ArrayKind::Rect,
                rect: RectUi {
                    rows: 2.0,
                    cols: 3.0,
                    dx: unit,
                    dy: unit,
                    gap: true,
                },
                polar: PolarUi {
                    count: 6.0,
                    angle: 360.0,
                    rotate: true,
                    ccw: true,
                    at: At::Canvas,
                    point: None,
                },
                mirror: MirrorUi {
                    axis: "v",
                    deg: 45.0,
                    at: At::Box,
                    point: None,
                },
                preview: true,
            },
            box_lock: false,
            turn: 90.0,
        }
    }
}

/// The library drawing being edited.
#[derive(Clone, Debug, PartialEq)]
pub struct Original {
    pub id: String,
    pub source: Source,
    /// Kaydet writes it in place (a user or project drawing); a system one is saved as a copy.
    pub editable: bool,
}

/// What a save tells the window that opened the editor.
#[derive(Clone, Debug, PartialEq)]
pub enum After {
    /// Stil yöneticisi shows the saved drawing.
    Manager,
    /// Sembol tasarımcısı's image field takes the saved drawing.
    Designer(String),
    Nothing,
}

/// Where the editor opens from (`SvgEditorOptions`).
#[derive(Clone, Debug)]
pub struct Opening {
    /// A drawing of the library; a new drawing when None.
    pub id: Option<String>,
    /// A new drawing's category.
    pub path: Option<Vec<String>>,
    pub after: After,
}

/// The question over the window: closing with changes, or another drawing replacing them.
#[derive(Clone, Debug)]
pub enum Question {
    Close,
    Replace(Box<Pending>),
}

/// The SVG editor.
pub struct SvgEditor {
    pub doc: Drawing,
    /// Chosen shapes, in the order they were chosen (Hizala's first and last).
    pub selection: Vec<String>,
    pub tool: ToolId,
    /// The path whose nodes are edited.
    pub node_edit: Option<String>,
    pub options: Options,
    pub ui: PanelState,
    pub original: Option<Original>,
    /// The foot's name and category as typed.
    pub name: String,
    pub path_text: String,
    pub after: After,
    /// What the window said, and whether it warns.
    pub said: Option<(String, bool)>,
    pub question: Option<Question>,
    /// The window it stands over (Stil yöneticisi, Sembol tasarımcısı).
    pub under: Option<Dialog>,
    past: Vec<String>,
    future: Vec<String>,
    pending: Option<String>,
    last_key: Option<(String, Instant)>,
    saved_json: String,
    saved_meta: String,
    /// Texts typed into fields, kept while they differ from the value (by the field's key).
    pub typed: HashMap<String, String>,
    // ── The canvas ──
    pub camera: Camera,
    pub op: Option<Op>,
    pub press: Option<Press>,
    pub last_click: Option<Click>,
    pub draw: DrawTool,
    pub nodes: NodeTool,
    pub measure: Measure,
    pub rulers: Rulers,
    pub snapper: Snapper,
    pub picking: Option<Pick>,
    /// A point a panel shows (the chosen centre).
    pub marker: Option<Pt>,
    /// A list row being renamed and its text.
    pub renaming: Option<(String, String)>,
    /// The files' state: dialogs, the source panel, the reference.
    pub files: FileState,
    /// Dışa aktar's and Bitmap izle's choices, for the next time in this session.
    pub export_last: super::files::export::ExportDialog,
    pub trace_last: super::files::trace::TraceSettings,
    /// A trace setting changed: the application traces again.
    pub pending_trace: bool,
    /// The style engine's pictures (glyph outlines, SVG and raster pictures).
    pub images: std::sync::Arc<crate::style::images::Images>,
    /// The theme's preview colours as the view last saw them: ink and paper.
    pub theme: RefCell<(String, String)>,
    /// Shift held (a list row's click adds).
    pub shift_held: bool,
    /// A list row being dragged.
    pub list_drag: Option<super::list::Drag>,
    /// The number fields the view made (for ↑ and ↓) and the texts they showed.
    pub(super) fields: RefCell<HashMap<String, super::panels::FieldDef>>,
    pub(super) shown: RefCell<HashMap<String, String>>,
    /// Bumped on every change the canvas draws; the shapes' picture is cached until it moves.
    pub revision: u64,
    /// The revision and colours the shapes' picture was drawn at.
    pub(super) drawn: Cell<(u64, u64)>,
    pub(super) shapes_cache: canvas::Cache,
    dirty_at: RefCell<Option<(u64, bool)>>,
}

impl SvgEditor {
    pub fn new(
        doc: Drawing,
        original: Option<Original>,
        name: String,
        path: Vec<String>,
        after: After,
        under: Option<Dialog>,
        ink: &str,
        paper: &str,
    ) -> SvgEditor {
        let options = Options::new(&doc, ink, paper);
        let ui = PanelState::new(&doc);
        let saved_json = doc.text();
        let path_text = path.join(" / ");
        let mut ed = SvgEditor {
            doc,
            selection: Vec::new(),
            tool: ToolId::Select,
            node_edit: None,
            options,
            ui,
            original,
            name,
            path_text,
            after,
            said: None,
            question: None,
            under,
            past: Vec::new(),
            future: Vec::new(),
            pending: None,
            last_key: None,
            saved_json,
            saved_meta: String::new(),
            typed: HashMap::new(),
            camera: Camera::default(),
            op: None,
            press: None,
            last_click: None,
            draw: DrawTool::default(),
            nodes: NodeTool::default(),
            measure: Measure::default(),
            rulers: Rulers::default(),
            snapper: Snapper::default(),
            picking: None,
            marker: None,
            renaming: None,
            files: FileState::default(),
            export_last: super::files::export::ExportDialog::default(),
            trace_last: super::files::trace::TraceSettings::default(),
            pending_trace: false,
            images: std::sync::Arc::new(crate::style::images::Images::new()),
            theme: RefCell::new((ink.to_owned(), paper.to_owned())),
            shift_held: false,
            list_drag: None,
            fields: RefCell::new(HashMap::new()),
            shown: RefCell::new(HashMap::new()),
            revision: 1,
            drawn: Cell::new((0, 0)),
            shapes_cache: canvas::Cache::new(),
            dirty_at: RefCell::new(None),
        };
        ed.saved_meta = ed.meta();
        ed
    }

    // ── The drawing's history (`begin`, `commit`, `change`) ─────────────

    /// Remembers the drawing before an interactive change (one undo step).
    pub fn begin(&mut self) {
        if self.pending.is_none() {
            self.pending = Some(self.doc.text());
        }
    }

    /// The interactive change is over; an empty label (a click, not a drag) keeps no step.
    pub fn commit(&mut self, label: &str) {
        let before = self.pending.take();
        if let Some(before) = before
            && !label.is_empty()
            && before != self.doc.text()
        {
            self.push_past(before);
            self.future.clear();
            self.last_key = None;
        }
        self.touch();
    }

    /// The canvas is to be drawn again (the drawing changed during a drag).
    pub fn touch(&mut self) {
        self.revision += 1;
    }

    fn push_past(&mut self, text: String) {
        self.past.push(text);
        if self.past.len() > HISTORY {
            self.past.remove(0);
        }
    }

    /// One undo step for `f`'s change; the same key within a second adds to the last step.
    /// Nothing is kept when `f` changes nothing (the web kept an empty step).
    pub fn edit(&mut self, key: &str, f: impl FnOnce(&mut SvgEditor)) {
        let now = Instant::now();
        let merge = matches!(&self.last_key, Some((k, at)) if k == key && now.duration_since(*at) <= MERGE);
        let before = self.doc.text();
        f(self);
        if self.doc.text() == before {
            return;
        }
        if !merge {
            self.push_past(before);
            self.future.clear();
        }
        self.last_key = Some((key.to_owned(), now));
        self.touch();
        self.snapper.reset();
    }

    /// Ends a run of merged typing (Enter in a field).
    pub fn settle(&mut self) {
        self.last_key = None;
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn undo(&mut self) {
        let Some(prev) = self.past.pop() else {
            return;
        };
        self.future.push(self.doc.text());
        self.restore(&prev);
    }

    pub fn redo(&mut self) {
        let Some(next) = self.future.pop() else {
            return;
        };
        let now = self.doc.text();
        self.push_past(now);
        self.restore(&next);
    }

    fn restore(&mut self, text: &str) {
        if let Some(doc) = Drawing::from_text(text) {
            self.doc = doc;
        }
        let doc = &self.doc;
        self.selection.retain(|id| doc.shape(id).is_some());
        if self
            .node_edit
            .as_deref()
            .is_some_and(|id| doc.shape(id).is_none())
        {
            self.node_edit = None;
        }
        self.last_key = None;
        self.typed.clear();
        self.touch();
        self.snapper.reset();
    }

    /// Another drawing in the window (a file opened as new, a library drawing): history starts anew.
    pub fn open(&mut self, doc: Drawing, original: Option<Original>, name: String, path: Option<Vec<String>>) {
        self.options.grid = super::state::default_grid(doc.width);
        self.doc = doc;
        self.past.clear();
        self.future.clear();
        self.pending = None;
        self.last_key = None;
        self.original = original;
        self.name = name;
        if let Some(path) = path {
            self.path_text = path.join(" / ");
        }
        self.saved_json = self.doc.text();
        self.saved_meta = self.meta();
        self.selection.clear();
        self.node_edit = None;
        self.typed.clear();
        self.op = None;
        self.draw = DrawTool::default();
        self.nodes = NodeTool::default();
        self.measure = Measure::default();
        self.picking = None;
        self.marker = None;
        self.snapper.reset();
        self.touch();
        self.camera.fit(&self.doc, &self.options);
    }

    /// The drawing was saved: what `dirty` compares with.
    pub fn saved(&mut self) {
        self.saved_json = self.doc.text();
        self.saved_meta = self.meta();
        self.touch();
    }

    fn meta(&self) -> String {
        format!("{}\n{}", self.name, self.path_text)
    }

    /// What Kaydet would write differs from what was saved or opened (a new drawing: from the blank one).
    pub fn dirty(&self) -> bool {
        if let Some((rev, d)) = *self.dirty_at.borrow()
            && rev == self.revision
        {
            return d || self.meta() != self.saved_meta;
        }
        let d = self.doc.text() != self.saved_json;
        *self.dirty_at.borrow_mut() = Some((self.revision, d));
        d || self.meta() != self.saved_meta
    }

    /// Kaydet writes nothing without a visible shape; closing such a drawing loses nothing.
    pub fn savable(&self) -> bool {
        self.doc.has_visible()
    }

    /// The name Kaydet gives (`Adsız çizim` for none).
    pub fn save_name(&self) -> String {
        let t = kentos_processing::text::js_trim(&self.name);
        if t.is_empty() {
            "Adsız çizim".to_owned()
        } else {
            t.to_owned()
        }
    }

    /// The category typed, `A / B`, as its parts (Çizimlerim when none).
    pub fn save_path(&self) -> Vec<String> {
        let parts: Vec<String> = self
            .path_text
            .split('/')
            .map(|s| kentos_processing::text::js_trim(s).to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            vec!["Çizimlerim".to_owned()]
        } else {
            parts
        }
    }

    // ── Selection, tools, status ─────────────────────────────────────────

    pub fn is_selected(&self, id: &str) -> bool {
        self.selection.iter().any(|s| s == id)
    }

    /// The chosen shapes back to front (as the list stacks them).
    pub fn chosen(&self) -> Vec<Obj> {
        self.doc
            .shapes
            .iter()
            .filter(|s| self.is_selected(super::doc::id_of(s)))
            .cloned()
            .collect()
    }

    pub fn select(&mut self, ids: Vec<String>) {
        let mut seen: Vec<String> = Vec::with_capacity(ids.len());
        for id in ids {
            if !seen.contains(&id) {
                seen.push(id);
            }
        }
        self.selection = seen;
        if self
            .node_edit
            .as_deref()
            .is_some_and(|id| !self.selection.iter().any(|s| s == id))
        {
            self.node_edit = None;
        }
        self.touch();
    }

    pub fn status(&mut self, text: impl Into<String>, warn: bool) {
        let text = text.into();
        self.said = if text.is_empty() {
            None
        } else {
            Some((text, warn))
        };
    }

    pub fn say(&mut self, text: impl Into<String>) {
        self.status(text, false);
    }

    pub fn warn(&mut self, text: impl Into<String>) {
        self.status(text, true);
    }

    /// Another tool (`setTool`): the half-done work of the last one goes.
    pub fn set_tool(&mut self, t: ToolId) {
        self.tool_changed();
        self.tool = t;
        if t != ToolId::Node {
            self.node_edit = None;
        } else if self.node_edit.is_none() {
            let path = self
                .doc
                .shapes
                .iter()
                .find(|s| self.is_selected(super::doc::id_of(s)) && s.kind() == "path")
                .map(|s| super::doc::id_of(s).to_owned());
            let shape = self.doc.shapes.iter().any(|s| {
                self.is_selected(super::doc::id_of(s)) && matches!(s.kind(), "rect" | "ellipse")
            });
            self.node_edit = path.clone();
            if path.is_none() {
                if shape {
                    self.warn("Dikdörtgen ve elips düğümle düzenlenmez: önce Yol → Nesneyi yola çevir (Ctrl+Shift+C).");
                } else {
                    self.say("Düğüm düzenlemek için bir yol seçin ya da yola çift tıklayın.");
                }
            }
        }
        if t == ToolId::Measure {
            self.say("Ölç: iki noktayı tıklayın ya da sürükleyin (kenetlenir); bir yolun üstünde durunca parça boyları görünür. Açı, Döndürme gibi saat yönünde artar.");
        }
        self.touch();
    }

    /// Edits the nodes of a path, or ends node editing (`editNodes`).
    pub fn edit_nodes(&mut self, id: Option<String>) {
        if self.nodes.mode.is_some() && id.is_none() {
            self.nodes.set_mode(None);
        }
        self.tool = if id.is_some() {
            ToolId::Node
        } else {
            ToolId::Select
        };
        if let Some(id) = &id {
            self.selection = vec![id.clone()];
        }
        self.node_edit = id;
        self.touch();
    }

    /// The tool changed: its half-done work goes (`toolChanged`).
    pub fn tool_changed(&mut self) {
        self.draw.cancel();
        self.measure.reset();
        if self.nodes.mode.is_some() {
            self.nodes.set_mode(None);
        }
    }

    /// The symbol's colour as the preview paints it: the theme's until one is picked.
    pub fn ink(&self) -> String {
        if self.options.ink_auto {
            self.theme.borrow().0.clone()
        } else {
            self.options.ink.clone()
        }
    }

    /// The preview's paper: the drawing's own, else the theme's.
    pub fn paper(&self) -> String {
        self.doc
            .background
            .clone()
            .unwrap_or_else(|| self.theme.borrow().1.clone())
    }

    /// The title, with a dot for unsaved changes.
    pub fn title(&self) -> String {
        if self.dirty() {
            "SVG çizim düzenleyicisi •".to_owned()
        } else {
            "SVG çizim düzenleyicisi".to_owned()
        }
    }
}
