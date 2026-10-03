//! What the sheet mode is told: the host maps these into its own messages
//! (`Message::Sheet(..)`) and gives them back to [`Designer::update`](crate::Designer::update).

use iced::{Point, Size};
use kentos_sheet::model::{
    Axis, ConstraintBox, HConstraint, ItemId, Orientation, Paper, SheetId, VConstraint,
};
use kentos_sheet::ops::{AlignEdge, AlignTo, DistributeMode, ReorderTo, SizeDimension};
use kentos_ui::widget::inspector;
use kentos_ui::widget::rulers;
use kentos_ui::widget::tree_view::Place;

use crate::gallery::GalleryMessage;
use crate::save_template::SaveMessage;

/// The tool in hand on the paper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    /// The hand: drags the paper (Space held is the hand for a moment).
    Hand,
    /// Adds an item: the profile's tool id and, if it has several, the ready look.
    Add {
        tool: String,
        preset: Option<String>,
    },
}

/// A pointer event on the paper, in micrometres on it (`at`) and pixels on the stage (`px`).
#[derive(Clone, Debug, PartialEq)]
pub enum StageEvent {
    /// Shift keeps a resized item's proportions and steps a turn by 15°; Ctrl drags without snapping; Alt resizes about the middle.
    Move {
        at: [f64; 2],
        size: Size,
        shift: bool,
        ctrl: bool,
        alt: bool,
    },
    Press {
        at: [f64; 2],
        px: Point,
        size: Size,
        shift: bool,
        ctrl: bool,
        alt: bool,
    },
    Release {
        at: [f64; 2],
        shift: bool,
        ctrl: bool,
        alt: bool,
    },
    DoubleClick {
        at: [f64; 2],
    },
    /// The wheel: `factor` > 1 zooms in, about `px`.
    Zoom {
        factor: f64,
        px: Point,
        size: Size,
    },
    /// A middle-button or hand drag, pixels.
    Pan {
        dx: f32,
        dy: f32,
        size: Size,
    },
    Leave,
    /// The stage's size changed (a window resized, a panel shown): what fits something
    /// to the stage (Sığdır, Gerçek, Seçim) uses it.
    Resize {
        size: Size,
    },
}

/// The inspector's tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectorTab {
    #[default]
    Item,
    Page,
    Preflight,
}

impl std::fmt::Display for InspectorTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            InspectorTab::Item => "Öğe",
            InspectorTab::Page => "Sayfa",
            InspectorTab::Preflight => "Ön denetim",
        })
    }
}

/// A frame value of the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameField {
    Left,
    Top,
    Width,
    Height,
    Rotation,
}

/// A margin of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// What to write a sheet out as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    /// The chosen sheets as one PDF, a page each (the core's writer, design §9a).
    Pdf,
    Svg,
    Png,
    /// The book with its pictures (`.kpafta`).
    Kpafta,
}

impl ExportKind {
    pub fn extension(self) -> &'static str {
        match self {
            ExportKind::Pdf => "pdf",
            ExportKind::Svg => "svg",
            ExportKind::Png => "png",
            ExportKind::Kpafta => "kpafta",
        }
    }
}

/// Which sheets a PDF writes (the web's `PdfScope`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PdfScope {
    /// The sheet in front.
    #[default]
    This,
    /// Those chosen in the window.
    Chosen,
    /// Every sheet, in the book's order.
    All,
}

/// The export window's PDF part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PdfMessage {
    Scope(PdfScope),
    Sheet(kentos_sheet::model::SheetId, bool),
    Geo(bool),
    Layers(bool),
    /// Kaydet: where the file goes (the host's file window).
    Save,
    /// Yazdır: the host's viewer prints it.
    Print,
}

#[derive(Clone, Debug)]
pub enum Message {
    // ── Tabs under the drawing: 0 is the model ──
    Tab(usize),
    CloseTab(usize),
    MoveTab(usize, usize),
    /// “+”: a new sheet from the mode's default template.
    NewSheet,
    /// Back to the drawing.
    Model,

    // ── Sheets ──
    DuplicateSheet(SheetId),
    RemoveSheet(SheetId),
    RenameSheet(SheetId, String),

    // ── Tools and the paper ──
    Tool(Tool),
    Stage(StageEvent),
    /// Space held over the paper: the hand for a moment (false: let go).
    Space(bool),
    Guide(rulers::Event),
    ZoomPage,
    ZoomReal,
    /// Seçime yakınlaş: the chosen items fill the stage.
    ZoomSelection,

    // ── The chosen items ──
    Select(Vec<ItemId>),
    /// A click in the item tree; with Ctrl or Shift it adds or takes away.
    TreeClick(ItemId, bool),
    TreeExpand(ItemId),
    TreeMove(usize, usize, Place),
    Hidden(ItemId, bool),
    Locked(ItemId, bool),
    RenameItem(ItemId, String),
    Align(AlignEdge),
    AlignTo(AlignTo),
    Distribute(Axis, DistributeMode),
    MatchSize(SizeDimension),
    Order(ReorderTo),
    Group,
    Ungroup,
    Duplicate,
    Delete,
    Nudge(i32, i32),
    Undo,
    Redo,

    // ── The inspector ──
    InspectorTab(InspectorTab),
    Section(&'static str),
    Frame(FrameField, f64),
    FrameText(FrameField, String),
    ConstraintH(HConstraint),
    ConstraintV(VConstraint),
    ConstraintBox(ConstraintBox),
    Inspector(inspector::Event),
    /// A number's label is dragged: its changes are one undo step until it is let go.
    GestureStart,
    GestureEnd,
    /// ƒ: a property's binding (design §7).
    Binding(crate::binding::BindingMessage),
    Fill(Option<String>),
    Opacity(f64),
    Padding(f64),
    Printable(bool),

    // ── The page ──
    Paper(Paper),
    Orientation(Orientation),
    Margin(Side, f64),
    SnapToGrid(bool),
    GridSpacing(f64),

    // ── Preflight ──
    /// A finding's fix: the finding's and the fix's place in their lists.
    Fix(usize, usize),
    Reveal(ItemId),

    // ── Windows ──
    Gallery(GalleryMessage),
    /// Şablonu paylaş.
    Share(crate::share_template::ShareMessage),
    /// Kuruma yayımla.
    Publish(crate::publish_template::PublishMessage),
    /// The chosen north arrows' declination (the inspector's “Manyetik sapma” part).
    North(NorthMessage),
    /// The host's word on the cloud library.
    Library(crate::library::LibraryEvent),
    SaveTemplate(SaveMessage),
    Variables(crate::variables::VariablesMessage),
    /// The template's questions, asked before its sheet is made.
    Questions(crate::questions::QuestionsMessage),
    Export(ExportKind),
    /// Yazdır…: the export window for a PDF, Yazdır first.
    Print,
    Pdf(PdfMessage),
    /// “Resim seç…”: the host asks which picture file the chosen picture frames show.
    ChoosePicture,
    /// That file's name and bytes, read by the host.
    PictureFile(String, Vec<u8>),
    /// “.kpafta dosyasından…”: the host asks which file.
    ImportKpafta,
    /// A `.kpafta` file's name and text, read by the host.
    ImportText(String, String),
    /// Its sheets in place of the project's (true) or beside them.
    ImportChoose(bool),
    /// The PNG's resolution.
    ExportDpi(u16),
    /// Write it anyway when the preflight found errors.
    ExportAnyway,
    CloseDialog,
}

/// The inspector's declination part of a north arrow (design §8a; the web's `northSection`).
#[derive(Clone, Debug, PartialEq)]
pub enum NorthMessage {
    /// “Sapmayı elle gir”: on, it starts from the model's value to the minute when nothing was
    /// typed yet.
    Hand(bool),
    /// “Elle sapma”, degrees (east positive).
    Declination(f64),
    /// “Sapmanın yılı”.
    Year(f64),
}
