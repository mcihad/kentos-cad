//! The sheet mode's state (design §11): the book, the sheet in front, the
//! chosen items, the tool, the view, the undo stack and the windows. Every
//! change of the book is an operation of the core with its inverse (design
//! §10: the sheet mode's own undo, apart from the drawing's); what is shown
//! is worked out again from the book after each one ([`Designer::refresh`]).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use iced::Size;
use iced::widget::canvas;
use kentos_contracts::{ProjectType, Workspace};
use kentos_sheet::display::{
    self, DisplayList, MapPrim, Prim, ProjectInfo, RenderInputs, RenderMode,
};
use kentos_sheet::kinds::GroundPoint;
use kentos_sheet::model::{Item, ItemId, Owner, Sheet, SheetBook, SheetId};
use kentos_sheet::ops::{self, Op};
use kentos_sheet::preflight::{self, Finding};
use kentos_sheet::profile::{self, Capabilities, Profile};
use kentos_sheet::snap::{GapBadge, SnapLine, SpacingMark};
use kentos_sheet::units::RectUm;
use kentos_ui::widget::Guides;
use kentos_ui::widget::inspector;
use kentos_ui::widget::rulers::Guide;

use crate::gallery::Gallery;
use crate::message::{ExportKind, InspectorTab, Tool};
use crate::paint::{self, Plan};
use crate::pictures::{self, Raster};
use crate::save_template::SaveDialog;
use crate::store::Store;
use crate::view_math::View;

/// What the host is told to do after a message.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// The drawing is in front again.
    ShowModel,
    /// A sheet is in front.
    ShowSheet,
    /// Ask where to write the export (the host's file dialog), then call [`Designer::export_to`].
    AskExportPath { kind: ExportKind, suggested: String },
    /// Ask which `.kpafta` file to read (the host's file dialog), then give its text back as [`crate::Message::ImportText`].
    AskImportPath,
    /// Write the PDF (`Designer::pdf`) to a file of its own and open it in the system's viewer to print.
    Print,
    /// Ask which picture file (PNG, JPEG) the chosen picture frames show, then give its
    /// name and bytes back as [`crate::Message::PictureFile`].
    AskPicturePath,
    /// An action of the host a preflight fix names (`project.crs`; the sheet mode does
    /// `map.placeFromView` and `sheet.variables` itself).
    HostAction(String),
    /// Something to tell the user, Turkish.
    Notice(String),
    /// The same, with how it is said (the message log's level).
    Say(Say, String),
    /// What the gallery and the share window ask of the host's cloud library.
    Library(crate::library::LibraryRequest),
}

/// Which sheets a PDF writes and how (the export window's PDF part).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PdfChoice {
    pub scope: crate::message::PdfScope,
    pub chosen: BTreeSet<kentos_sheet::model::SheetId>,
    pub geo: bool,
    pub layers: bool,
}

impl Default for PdfChoice {
    fn default() -> Self {
        PdfChoice {
            scope: crate::message::PdfScope::This,
            chosen: BTreeSet::new(),
            geo: true,
            layers: true,
        }
    }
}

/// How a sentence is said (the host's message log).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Say {
    Info,
    Success,
    Warn,
    Error,
}

/// What the host knows of the project: its mode, type and capabilities (design §11a), its data for the sheets' values.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    pub workspace: Option<Workspace>,
    pub project_type: Option<ProjectType>,
    pub capabilities: Capabilities,
    pub project: ProjectInfo,
    pub crs: Option<display::CrsInfo>,
    /// Where a new map looks (the drawing area's centre).
    pub center: Option<GroundPoint>,
}

impl Default for Context {
    fn default() -> Self {
        Context {
            workspace: None,
            project_type: None,
            capabilities: Capabilities::default(),
            project: ProjectInfo {
                name: String::new(),
                user: String::new(),
                date: String::new(),
                crs_name: String::new(),
            },
            crs: None,
            center: None,
        }
    }
}

/// One step of the undo or redo stack: its name and the operations that
/// take it back (on the undo stack) or do it again (on the redo stack).
#[derive(Clone, Debug)]
pub(crate) struct Step {
    pub label: String,
    pub ops: Vec<Op>,
}

/// How many steps are kept.
const HISTORY: usize = 200;

/// A drag on the paper, from its press to its release.
pub(crate) enum Drag {
    /// The chosen items moved together; snapping worked out once at the start.
    Move {
        ids: Vec<ItemId>,
        start: [f64; 2],
        session: kentos_sheet::snap::SnapSession,
        moved: bool,
    },
    Resize {
        id: ItemId,
        handle: kentos_sheet::ops::Handle,
        session: kentos_sheet::snap::SnapSession,
    },
    Rotate {
        ids: Vec<ItemId>,
        center: [f64; 2],
        start_angle: f64,
        base: i32,
    },
    /// A box drawn to choose what it holds (to the right) or touches (to the left).
    Band {
        start: [f64; 2],
        end: [f64; 2],
        add: bool,
    },
    /// A new item's box.
    Create {
        tool: String,
        preset: Option<String>,
        start: [f64; 2],
        end: [f64; 2],
    },
    Pan,
}

/// What a drag shows while it is under way: the book as it would be, and the lines it snapped to.
#[derive(Debug, Default)]
pub(crate) struct Preview {
    pub ops: Vec<Op>,
    /// The book as the drag would leave it.
    pub book: Option<SheetBook>,
    pub list: Option<DisplayList>,
    pub plan: Plan,
    pub lines: Vec<SnapLine>,
    pub gaps: Vec<GapBadge>,
    pub spacing: Vec<SpacingMark>,
}

/// A map's content on its own canvas: kept while what it shows stays the same.
#[derive(Debug, Default)]
pub(crate) struct MapCache {
    pub cache: canvas::Cache,
    /// What the cache holds: a hash of the view, the size on the screen and the painter's revision.
    pub key: Cell<u64>,
}

/// The windows over the mode.
#[derive(Debug)]
pub(crate) enum Dialog {
    Export {
        kind: ExportKind,
        dpi: u16,
        errors: usize,
        /// Opened to print (Yazdır first).
        print: bool,
    },
    SaveTemplate(SaveDialog),
    Variables(crate::variables::VariablesDialog),
    /// A template's questions before its sheet is made (Kullan, Yeni pafta); boxed: a template is large.
    Questions(Box<crate::questions::QuestionsDialog>),
    /// ƒ: a property's value from an expression (design §7).
    Binding(crate::binding::BindingDialog),
    /// A `.kpafta` read while the project has sheets: beside them or in their place?
    Import {
        label: String,
        text: String,
        sheets: usize,
        have: usize,
    },
}

/// The sheet mode.
pub struct Designer {
    pub(crate) book: SheetBook,
    /// The sheet in front; none: the drawing (the model tab).
    pub(crate) open: Option<SheetId>,
    pub(crate) selection: Vec<ItemId>,
    pub(crate) tool: Tool,
    pub(crate) ctx: Context,
    /// The host's data for legends, tables and coordinate lists.
    pub(crate) data: RenderInputs,
    pub(crate) view: View,
    /// The stage's size as last seen.
    pub(crate) stage: Size,
    pub(crate) undo: Vec<Step>,
    pub(crate) redo: Vec<Step>,
    /// The last operation that failed, said once.
    pub(crate) error: Option<String>,

    // What the book shows, worked out by `refresh`.
    pub(crate) list: Option<DisplayList>,
    pub(crate) plan: Plan,
    pub(crate) findings: Vec<Finding>,
    pub(crate) profile: Profile,
    pub(crate) guides: Guides,
    pub(crate) guide_ids: Vec<String>,

    // The paper under the pointer.
    pub(crate) drag: Option<Drag>,
    pub(crate) preview: Option<Preview>,
    pub(crate) hover: Option<[f64; 2]>,
    pub(crate) hovered_item: Option<ItemId>,
    /// Space held: the hand for a moment.
    pub(crate) space: bool,

    // Panels.
    pub(crate) inspector_tab: InspectorTab,
    pub(crate) inspector: inspector::State,
    /// The kind's own properties of the chosen items: their fields and values (`inspect.rs`).
    pub(crate) kind_fields: Vec<crate::inspect::KindField>,
    /// The north arrow chosen alone and what it shows (the core's `north_info`): its id, and
    /// none when it could not be read.
    pub(crate) north_info: Option<(String, Option<kentos_sheet::display::NorthInfo>)>,
    /// The chosen items' constraints, in their order (the constraint editor's).
    pub(crate) chosen_constraints: Vec<kentos_sheet::model::Constraints>,
    pub(crate) closed_sections: BTreeSet<&'static str>,
    pub(crate) expanded: BTreeSet<ItemId>,

    pub(crate) gallery: Option<Gallery>,
    pub(crate) dialog: Option<Dialog>,
    pub(crate) export_dpi: u16,
    /// The export window's PDF choices (kept while the mode lives, as the web keeps them).
    pub(crate) pdf: PdfChoice,

    // Caches the view draws from.
    pub(crate) layer_caches: Vec<canvas::Cache>,
    /// How many times the paper's caches were emptied (a new book, a new view).
    pub(crate) paper_resets: u64,
    pub(crate) map_caches: BTreeMap<ItemId, MapCache>,
    pub(crate) pictures: RefCell<BTreeMap<String, Option<Rc<Raster>>>>,
    /// Pictures' bytes by their SHA-256 (the book keeps only their metadata).
    pub(crate) asset_bytes: BTreeMap<String, Vec<u8>>,

    // Where the book is kept.
    pub(crate) store: Option<Store>,
    pub(crate) key: Option<String>,
    /// The templates kept on this device: its own and the copies of an account's cloud library.
    pub(crate) user_templates: Vec<crate::store::StoredTemplate>,
    /// Template files the core could not read (listed with their reasons, kept as they are).
    pub(crate) refused: Vec<crate::store::Unreadable>,
    /// What the host says of the account's cloud library.
    pub(crate) library: crate::library::Library,
    /// A value dragged in the inspector: its changes are one undo step until it is let go.
    pub(crate) gesture: Option<Gesture>,
    pub(crate) next_id: u64,
}

/// A value being dragged in the inspector (a number's label, a field's name).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Gesture {
    /// The drag's first change made its undo step: the next ones join it.
    pub stepped: bool,
}

impl Default for Designer {
    fn default() -> Self {
        Self::new(Context::default())
    }
}

impl Designer {
    pub fn new(ctx: Context) -> Self {
        let profile = profile::profile_for(ctx.workspace, &ctx.capabilities);
        let mut d = Designer {
            book: SheetBook::default(),
            open: None,
            selection: Vec::new(),
            tool: Tool::Select,
            ctx,
            data: empty_inputs(),
            view: View::default(),
            stage: Size::new(1000.0, 700.0),
            undo: Vec::new(),
            redo: Vec::new(),
            error: None,
            list: None,
            plan: Plan::default(),
            findings: Vec::new(),
            profile,
            guides: Guides::new(),
            guide_ids: Vec::new(),
            drag: None,
            preview: None,
            hover: None,
            hovered_item: None,
            space: false,
            inspector_tab: InspectorTab::Item,
            inspector: inspector::State::new(),
            kind_fields: Vec::new(),
            north_info: None,
            chosen_constraints: Vec::new(),
            closed_sections: BTreeSet::new(),
            expanded: BTreeSet::new(),
            gallery: None,
            dialog: None,
            export_dpi: 150,
            pdf: PdfChoice::default(),
            layer_caches: Vec::new(),
            paper_resets: 0,
            map_caches: BTreeMap::new(),
            pictures: RefCell::new(BTreeMap::new()),
            asset_bytes: BTreeMap::new(),
            store: None,
            key: None,
            user_templates: Vec::new(),
            refused: Vec::new(),
            library: crate::library::Library::default(),
            gesture: None,
            next_id: 1,
        };
        // The inspector's numbers are written by the display rule (ADR 0149: a point, never a comma).
        kentos_ui::attribute::number::set_point_rule(kentos_geometry_core::display::fixed);
        d.refresh();
        d
    }

    // ── What the host reads ──

    pub fn book(&self) -> &SheetBook {
        &self.book
    }

    /// The sheet in front; none while the drawing is.
    pub fn open_sheet(&self) -> Option<&Sheet> {
        self.open.as_deref().and_then(|id| self.book.sheet(id))
    }

    /// Whether a sheet is in front (the host shows the sheet mode instead of the drawing).
    pub fn is_active(&self) -> bool {
        self.open_sheet().is_some()
    }

    pub fn selection(&self) -> &[ItemId] {
        &self.selection
    }

    pub fn tool(&self) -> &Tool {
        &self.tool
    }

    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// What the north arrow chosen alone shows (the inspector's declination part): none for no
    /// such arrow or one the core could not read.
    pub fn north_info(&self) -> Option<&display::NorthInfo> {
        self.north_info.as_ref().and_then(|(_, i)| i.as_ref())
    }

    /// The names of the kind's fields the inspector shows for the chosen items, in order.
    pub fn kind_field_names(&self) -> Vec<&str> {
        self.kind_fields.iter().map(|k| k.prop.label).collect()
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// The display list of the sheet in front (as designed).
    pub fn display_list(&self) -> Option<&DisplayList> {
        self.list.as_ref()
    }

    /// The undo step's name, if there is one (“Geri al: Taşı: Harita”).
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|s| s.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|s| s.label.as_str())
    }

    /// The last operation's failure, Turkish (it stays until the next change).
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    // ── What the host gives ──

    /// The project's mode, type, capabilities and values: the profile and the values follow.
    pub fn set_context(&mut self, ctx: Context) {
        if ctx != self.ctx {
            self.ctx = ctx;
            self.profile = profile::profile_for(self.ctx.workspace, &self.ctx.capabilities);
            self.refresh();
            // An open gallery's cards: what a template needs that the project lacks may have changed.
            self.ensure_thumbs();
        }
    }

    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// The zoom of the sheet in front, per cent of its real size (the status bar's).
    pub fn zoom_percent(&self) -> Option<u32> {
        self.open_sheet()
            .map(|s| self.view.percent(self.stage, s.page.size))
    }

    /// The host's data for the sheets' legends, tables and coordinate lists (its maps, layers and objects).
    pub fn set_data(&mut self, data: RenderInputs) {
        self.data = data;
        self.refresh();
    }

    /// A book read from elsewhere (a `.kpafta`, the store): the undo stack starts again.
    pub fn load(&mut self, book: SheetBook) {
        self.book = book;
        self.undo.clear();
        self.redo.clear();
        self.selection.clear();
        if self
            .open
            .as_ref()
            .is_some_and(|id| self.book.sheet(id).is_none())
        {
            self.open = None;
        }
        self.refresh();
    }

    /// Operations of the core from the host (a script, a test) as one undo
    /// step named `label`; false (and the error kept) when the core refuses them.
    pub fn apply(&mut self, ops: Vec<Op>, label: &str) -> bool {
        self.commit_as(ops, Some(label))
    }

    /// Pictures' bytes for the book (a template's, a `.kpafta`'s).
    pub fn add_asset_bytes(&mut self, sha256: &str, bytes: Vec<u8>) {
        self.pictures.borrow_mut().remove(sha256);
        if let Some(store) = &self.store {
            store.put_asset(sha256, &bytes);
        }
        self.asset_bytes.insert(sha256.to_owned(), bytes);
    }

    /// Keeps the book in `store` under the project's `key` from now on, reading
    /// what was kept there before (the host's data folder, design §10).
    pub fn attach(&mut self, store: Store, key: &str) {
        let book = store.load_book(key);
        let (records, refused) = store.records();
        self.user_templates = records;
        self.refused = refused;
        for a in book.iter().flat_map(|b| b.assets.iter()) {
            if let Some(bytes) = store.asset(&a.sha256) {
                self.asset_bytes.insert(a.sha256.clone(), bytes);
            }
        }
        self.store = Some(store);
        self.key = Some(key.to_owned());
        self.open = None;
        self.load(book.unwrap_or_default());
    }

    /// The project's key changed (a new drawing saved under a name): the book goes with it.
    pub fn rekey(&mut self, key: &str) {
        self.key = Some(key.to_owned());
        self.persist();
    }

    pub(crate) fn persist(&mut self) {
        if let (Some(store), Some(key)) = (&self.store, &self.key)
            && let Err(e) = store.save_book(key, &self.book)
        {
            self.error = Some(format!("Paftalar kaydedilemedi: {e}"));
        }
    }

    // ── Ids ──

    /// A new id no item, sheet or master of the book has.
    pub(crate) fn new_id(&mut self, prefix: &str) -> String {
        let taken = |b: &SheetBook, id: &str| {
            b.sheets.iter().any(|s| s.id == id)
                || b.masters.iter().any(|m| m.id == id)
                || b.find_item(id).is_some()
                || b.sheets.iter().any(|s| s.guides.iter().any(|g| g.id == id))
        };
        loop {
            let id = format!("{prefix}{}", self.next_id);
            self.next_id += 1;
            if !taken(&self.book, &id) {
                return id;
            }
        }
    }

    // ── Changes ──

    /// Applies operations as one undo step; false (and the error kept) when the core refuses them.
    pub(crate) fn commit(&mut self, ops: Vec<Op>) -> bool {
        self.commit_as(ops, None)
    }

    /// The same, the step named `label` (else the core's name for it). While a
    /// value is dragged in the inspector its changes join the drag's first
    /// step: the step takes back each of them, the last first.
    pub(crate) fn commit_as(&mut self, ops: Vec<Op>, label: Option<&str>) -> bool {
        if ops.is_empty() {
            return false;
        }
        match ops::apply_all(&self.book, &ops) {
            Ok(applied) => {
                let joins = self.gesture.as_ref().is_some_and(|g| g.stepped);
                if let (true, Some(top)) = (joins, self.undo.last_mut()) {
                    let mut back = applied.inverse;
                    back.append(&mut top.ops);
                    top.ops = back;
                } else {
                    self.undo.push(Step {
                        label: label.map_or(applied.label, str::to_owned),
                        ops: applied.inverse,
                    });
                }
                if let Some(g) = &mut self.gesture {
                    g.stepped = true;
                }
                if self.undo.len() > HISTORY {
                    self.undo.remove(0);
                }
                self.redo.clear();
                self.book = applied.book;
                self.error = None;
                self.after_change();
                true
            }
            Err(e) => {
                self.error = Some(e.message.clone());
                false
            }
        }
    }

    /// Takes a step from one stack, applies it and puts its inverse on the other.
    fn step(&mut self, back: bool) -> bool {
        let popped = if back {
            self.undo.pop()
        } else {
            self.redo.pop()
        };
        let Some(step) = popped else {
            return false;
        };
        match ops::apply_all(&self.book, &step.ops) {
            Ok(applied) => {
                self.book = applied.book;
                let other = Step {
                    label: step.label,
                    ops: applied.inverse,
                };
                if back {
                    self.redo.push(other);
                } else {
                    self.undo.push(other);
                }
                self.after_change();
                true
            }
            Err(e) => {
                self.error = Some(e.message.clone());
                false
            }
        }
    }

    pub(crate) fn undo_step(&mut self) -> bool {
        self.step(true)
    }

    pub(crate) fn redo_step(&mut self) -> bool {
        self.step(false)
    }

    fn after_change(&mut self) {
        if self
            .open
            .as_ref()
            .is_some_and(|id| self.book.sheet(id).is_none())
        {
            self.open = self.book.sheets.first().map(|s| s.id.clone());
        }
        let book = &self.book;
        self.selection.retain(|id| book.find_item(id).is_some());
        self.refresh();
        self.persist();
    }

    // ── What is shown ──

    /// The inputs the core lays out with: the host's data and the project's values.
    pub(crate) fn inputs(&self, mode: RenderMode) -> RenderInputs {
        let mut i = self.data.clone();
        i.mode = mode;
        i.project = self.ctx.project.clone();
        i.capabilities = self.ctx.capabilities.clone();
        if self.ctx.crs.is_some() {
            i.crs = self.ctx.crs.clone();
        }
        i
    }

    /// Works out again what the book shows: the display list, its plan, the
    /// findings, the guides and the inspector's values; the caches whose
    /// content changed are emptied.
    pub fn refresh(&mut self) {
        if self
            .open
            .as_ref()
            .is_some_and(|id| self.book.sheet(id).is_none())
        {
            self.open = None;
        }
        let built = self
            .open
            .as_deref()
            .map(|id| display::build(&self.book, id, &self.inputs(RenderMode::Design)));
        match built {
            Some(Ok((list, _notes))) => {
                self.plan = paint::plan(&list);
                self.list = Some(list);
            }
            Some(Err(e)) => {
                self.error = Some(e.message.clone());
                self.list = None;
                self.plan = Plan::default();
            }
            None => {
                self.list = None;
                self.plan = Plan::default();
            }
        }
        self.findings = self
            .open
            .as_deref()
            .and_then(|id| {
                preflight::preflight(&self.book, id, &self.inputs(RenderMode::Design)).ok()
            })
            .unwrap_or_default();
        self.sync_guides();
        self.reset_caches();
        crate::inspect::refresh_fields(self);
    }

    /// How many times the paper's caches were emptied: a host's test sees a new view drawn.
    pub fn paper_resets(&self) -> u64 {
        self.paper_resets
    }

    /// The paper's caches emptied (the book or the view changed); a map's only when what it shows did.
    pub(crate) fn reset_caches(&mut self) {
        let layers = self
            .preview
            .as_ref()
            .filter(|p| p.list.is_some())
            .map_or(self.plan.layers.len(), |p| p.plan.layers.len());
        self.layer_caches.resize_with(layers, canvas::Cache::new);
        for c in &self.layer_caches {
            c.clear();
        }
        self.paper_resets += 1;
        let live: BTreeSet<ItemId> = self.maps().iter().map(|m| m.item.clone()).collect();
        self.map_caches.retain(|id, _| live.contains(id));
        for id in live {
            self.map_caches.entry(id).or_default();
        }
    }

    /// The map primitives shown now (the drag's preview, else the sheet's).
    pub(crate) fn maps(&self) -> Vec<&MapPrim> {
        let (list, plan) = self.shown();
        plan.maps
            .iter()
            .filter_map(|e| match list.and_then(|l| l.prims.get(e.prim)) {
                Some(Prim::Map(m)) => Some(m),
                _ => None,
            })
            .collect()
    }

    /// The list and plan to paint: a drag's preview, else the sheet's.
    pub(crate) fn shown(&self) -> (Option<&DisplayList>, &Plan) {
        match &self.preview {
            Some(p) if p.list.is_some() => (p.list.as_ref(), &p.plan),
            _ => (self.list.as_ref(), &self.plan),
        }
    }

    fn sync_guides(&mut self) {
        self.guides.clear();
        self.guide_ids.clear();
        if let Some(s) = self.open_sheet() {
            let mut ids = Vec::new();
            let mut guides = Guides::new();
            for g in &s.guides {
                let mm = g.at as f32 / 1000.0;
                guides.push(match g.axis {
                    kentos_sheet::model::Axis::X => Guide::vertical(mm),
                    kentos_sheet::model::Axis::Y => Guide::horizontal(mm),
                });
                ids.push(g.id.clone());
            }
            self.guides = guides;
            self.guide_ids = ids;
        }
    }

    /// A picture's pixels, decoded the first time they are asked for.
    pub(crate) fn picture(&self, sha256: &str) -> Option<Rc<Raster>> {
        if let Some(p) = self.pictures.borrow().get(sha256) {
            return p.clone();
        }
        let decoded = self
            .asset_bytes
            .get(sha256)
            .and_then(|b| pictures::decode(b))
            .map(Rc::new);
        self.pictures
            .borrow_mut()
            .insert(sha256.to_owned(), decoded.clone());
        decoded
    }

    // ── Helpers of the chosen items ──

    pub(crate) fn owner(&self) -> Option<Owner> {
        self.open.as_deref().map(Owner::sheet)
    }

    /// The items of the sheet in front, in drawing order.
    pub(crate) fn items(&self) -> &[Item] {
        self.open_sheet().map_or(&[], |s| s.items.as_slice())
    }

    pub(crate) fn chosen(&self) -> Vec<&Item> {
        self.selection
            .iter()
            .filter_map(|id| self.book.item(id))
            .collect()
    }

    /// The box around the chosen items' frames.
    pub(crate) fn chosen_bounds(&self) -> Option<RectUm> {
        self.chosen()
            .iter()
            .map(|i| kentos_sheet::units::rotated_bounds(&i.frame, i.rotation))
            .reduce(|a, b| a.union(&b))
    }

    /// Whether any operation is ready to be taken back.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

/// Inputs with nothing of the host's yet.
pub fn empty_inputs() -> RenderInputs {
    RenderInputs {
        mode: RenderMode::Design,
        project: ProjectInfo {
            name: String::new(),
            user: String::new(),
            date: String::new(),
            crs_name: String::new(),
        },
        capabilities: Capabilities::default(),
        crs: None,
        maps: Vec::new(),
        legends: Vec::new(),
        tables: Vec::new(),
        coordinates: Vec::new(),
        atlas: None,
        page: None,
        fonts: None,
        assets: None,
        dpi: None,
    }
}
