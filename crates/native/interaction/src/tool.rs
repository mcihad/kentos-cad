//! What a tool is and what it gets: the web's `Tool` interface
//! (`apps/web/src/tools/Tool.ts`) without its DOM side. The host feeds
//! pointer events and typed text; the tool changes the drawing only through
//! the document and says what to draw as [`Preview`] data.

use kentos_domain::Document;
use kentos_geometry_core::entity::Shape as GeomShape;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::js_round;
use kentos_geometry_core::store::snap::{SnapHit, SnapKind};
use kentos_geometry_core::tools::point_input::Tracking;

use crate::Vec2;
use crate::format::Format;
use crate::log::{Level, Line};
use crate::object_tracking::{Aids, ObjectTracking};
use crate::prompt::Prompt;
use crate::select::SelectBox;
use crate::selection::Selection;
use crate::spatial::Spatial;

/// Where the pointer is, as a tool sees it (the web's `ToolPointer`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointer {
    /// The world point it stands for (x east, y north): the object snap's
    /// point when one applies, else `raw`.
    pub world: Vec2,
    /// The world point under it, through the view's float64 camera, before snapping.
    pub raw: Vec2,
    /// Logical pixels from the drawing area's top-left corner.
    pub screen: [f64; 2],
    /// Shift held: turns ortho over for this point, and adds to the selection (web).
    pub shift: bool,
    /// The object snap `world` came from. A snapped point is exact: ortho and
    /// polar tracking never move it (the web's `constrainPoint`).
    pub snap: Option<SnapHit>,
    /// `world` is where object tracking locked the cursor (no snap under it):
    /// exact as a snap is (docs/adr/0085).
    pub tracked: bool,
}

impl Pointer {
    /// The pointer at `raw`, on `snap`'s point when there is one.
    pub fn new(raw: Vec2, screen: [f64; 2], shift: bool, snap: Option<SnapHit>) -> Self {
        Self {
            world: snap.map_or(raw, |s| s.point),
            raw,
            screen,
            shift,
            snap,
            tracked: false,
        }
    }

    /// On the point object tracking locked the cursor to, when there is no
    /// snap (the web's `pointer`: the snap, else the lock, else the cursor).
    pub fn tracked(mut self, lock: Option<Vec2>) -> Self {
        if let (None, Some(p)) = (self.snap, lock) {
            self.world = p;
            self.tracked = true;
        }
        self
    }
}

/// The drawing area as a tool needs it: the camera's mapping, nothing else.
pub trait View {
    /// Where a world point is on the area, in logical pixels.
    fn to_screen(&self, p: Vec2) -> [f64; 2];
    /// How much world `px` logical pixels span at the current zoom.
    fn world_length(&self, px: f64) -> f64;
    /// The world box the area shows: the edges a trim or an extend meets by
    /// default (the web's `camera.visibleBounds()`).
    fn visible(&self) -> Bounds;
}

/// A change of the view a tool asks for (docs/adr/0056): Kaydır drags it,
/// Pencere yakınlaştır fits a box. The web's tools move `ctx.view.camera`
/// themselves; here the host's camera takes the changes, in order, right
/// after the call that made them.
#[derive(Clone, Debug, PartialEq)]
pub enum ViewChange {
    /// The view follows the pointer by logical pixels, right and down (the web's `camera.panBy`).
    Pan { dx: f64, dy: f64 },
    /// This world box as large as it fits, `padding` logical pixels in from the edges (`camera.fit`).
    Fit { bounds: Bounds, padding: f64 },
    /// A text field over the drawing where the text will start (Yazı, the
    /// web's `view.requestTextInput`); the host gives back what was typed
    /// with [`Tool::text_typed`].
    Text(TextField),
    /// The paragraph editor at a multi-line text's box (Çok satırlı yazı,
    /// docs/adr/0182 §4); the host gives back what was written with
    /// [`Tool::paragraph_typed`].
    Paragraph(ParagraphField),
    /// The point a window asked for (Çizimden, [`crate::pick::PickPoint`],
    /// docs/adr/0070), or none when the user left without one.
    Picked(Option<Vec2>),
    /// Objects picked for a window's field ([`crate::pick_objects::PickObjects`],
    /// docs/adr/0088): kept (the selection holds them), or left (Esc).
    PickedObjects(bool),
    /// Blok oluştur's base point ([`crate::block_define`], docs/adr/0144): the
    /// host opens the window that names a block of the selected objects.
    DefineBlock(Vec2),
    /// Blok ekle's point for a block with attribute definitions
    /// ([`crate::block_insert`], docs/adr/0144 §7): the host asks their
    /// values and gives them back with [`Tool::values_given`].
    AttributeValues(kentos_contracts::BlockId),
    /// Metin dosyası yerleştir's file ([`crate::text_file`], docs/adr/0145 §6):
    /// the host picks one and gives back its name and bytes with [`Tool::file_given`].
    OpenTextFile,
    /// A table to place from the cursor, its top left corner at the origin
    /// (Köşelere koordinat yaz's Çizelge, docs/adr/0185 §1): the host runs
    /// Tablo ekle's placement ([`crate::table_place::TablePlace`]) with it
    /// under this name.
    PlaceTable(kentos_contracts::EntityGeometry, &'static str),
}

/// Where a text field opens and how its text will look: its start, height in
/// metres and angle in degrees counter-clockwise from east, which point of
/// the text `at` is and its letters' width factor (docs/adr/0145), and what
/// it opens with, selected (Yazı's Artır).
#[derive(Clone, Debug, PartialEq)]
pub struct TextField {
    pub at: Vec2,
    pub height: f64,
    pub rotation: f64,
    pub align: Option<kentos_contracts::TextAlign>,
    pub width_factor: f64,
    pub initial: Option<String>,
    /// What the empty field shows, and the hint under it; none: Yazı's.
    pub placeholder: Option<&'static str>,
    pub hint: Option<&'static str>,
    /// Enter in the empty field answers with an empty text (Kılavuz: the
    /// arrow without a note, docs/adr/0146 §7); otherwise it is as Esc.
    pub empty: bool,
    /// The text style's face it will have (docs/adr/0183 §2); none for Standart.
    pub face: kentos_contracts::TextFace,
}

/// Where the paragraph editor opens and how its text will look (docs/adr/0182
/// §4): its point (the box's top left; the text's alignment the top's left),
/// height in metres, turn in degrees, box width (none: the lines end at their
/// breaks), line spacing (none: 1) and mask.
#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphField {
    pub at: Vec2,
    pub height: f64,
    pub rotation: f64,
    pub box_width: Option<f64>,
    pub line_spacing: Option<f64>,
    pub mask: bool,
    /// The text style's face and width factor it will have (docs/adr/0183
    /// §2); none and none for Standart.
    pub face: kentos_contracts::TextFace,
    pub width_factor: Option<f64>,
}

/// One value an option offers in its menu (Yazı's Hiza, docs/adr/0145 §6;
/// a style, docs/adr/0183 §4; the web's `OptionChoice`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionChoice {
    pub label: String,
    /// What typing it after the option's key gives (“sağ üst”): the command
    /// history says it.
    pub typed: String,
    /// The web's icon (`ui/icons.ts`); none for a list of names (the styles).
    pub icon: Option<&'static str>,
    /// A wide sample drawn in its menu instead of the icon (a hatch
    /// pattern's, docs/adr/0186 §11; the web's `preview`).
    pub preview: Option<&'static str>,
    pub checked: bool,
    /// A command the host runs instead of choosing (“Yazı stilleri…” opens
    /// its window); none for a value.
    pub command: Option<&'static str>,
}

/// The pointer's look over the drawing while a tool runs (the web's `Tool.cursor`):
/// the drawing's crosshair for a point, a shorter one with a pick box for
/// an object, or an open hand for Kaydır. The host draws the crosshair
/// (the web's `drawCrosshair`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cursor {
    /// A point is wanted (the web's `cross`).
    #[default]
    Cross,
    /// An object is wanted: the select tool, Sil, Tarama, the edge tools and
    /// Sahneden seç (the web's `pick`).
    Pick,
    Grab,
}

/// Drafting aids that change where a point goes, and what a click picks
/// (the typed settings' `drafting.*` and `snap.*`, docs/adr/0023, 0029).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draft {
    pub ortho: bool,
    /// Dik açı (`drafting.rightAngle`, docs/adr/0166 §4): from the tool's
    /// second edge on, each edge square to the one before.
    pub right_angle: bool,
    /// Polar tracking's step in degrees; `None` when it is off.
    pub polar: Option<f64>,
    /// How near a click must be to a point to be that point (kenet yarıçapı), logical pixels.
    pub snap_aperture: f64,
    /// Object snap is on (F3, `drafting.snap`).
    pub snap: bool,
    /// The snap kinds that apply, as `SnapKind::bit`s (`snap.*`, see [`snap_kinds`]).
    pub snap_kinds: u32,
    /// Karelaj's spacings, east (Y) and north (X), metres (`snap.gridEast`,
    /// `snap.gridNorth`, docs/adr/0163 §1).
    pub snap_grid: [f64; 2],
    /// The object being drawn is snapped to too (`snap.self`, §3).
    pub snap_self: bool,
    /// The screen scales snapping works between, as 1:N denominators
    /// (`snap.scaleMin`, `snap.scaleMax`; 0 no limit, §5).
    pub snap_scale: [f64; 2],
    /// How near a click must be to an object to pick it (`drafting.pickAperture`), logical pixels.
    pub pick_aperture: f64,
    /// Object tracking is on (Shift+F3, `drafting.tracking`, docs/adr/0085).
    pub tracking: bool,
    /// Topological editing is on (`drafting.topology`, docs/adr/0160): a
    /// grip puts the shared corners and edges of the objects around it right
    /// with the object it edits.
    pub topology: bool,
    /// Points count as shared corners too (Noktalar da, `drafting.topologyPoints`).
    pub topology_points: bool,
    /// The overlap control (`drafting.overlap`, docs/adr/0162 §1): a new
    /// area drawn by its outline loses what overlaps its neighbours.
    pub overlap: Overlap,
    /// The colour new objects take (the ribbon's Renk, the web's
    /// `ctx.settings.color`): one of the drawing colours (`ink`, `#E5484D`
    /// …) or an object template's (docs/adr/0176 §3), explicit in the
    /// product command's input (CMD-07); `None`: the layer's (“Katmana göre”).
    pub color: Option<DraftColor>,
    /// The line weight new objects drawn with lines take, mm (the ribbon's
    /// Kalınlık, the web's `ctx.settings.lineWeight`; docs/adr/0139); `None`:
    /// the layer's (“Katmana göre”).
    pub line_weight: Option<f64>,
    /// How a geographic second system's values are written
    /// (`display.geographic`, docs/adr/0167 §1).
    pub geographic: crate::second::Notation,
    /// Seçim süzgeci (`drafting.selectFilter`, docs/adr/0187 §5): the kinds
    /// that may be selected, as [`crate::selectable::bit`]s; `None` while the
    /// filter is off.
    pub select_kinds: Option<u32>,
}

impl Default for Draft {
    /// The web's defaults: ortho and polar off, an 11 px snap aperture,
    /// snapping on with its default kinds (all but nearest and the
    /// additions of docs/adr/0163), a 1 m Karelaj, the object being drawn
    /// snapped to, every scale, a 5 px pick
    /// aperture, topological editing off, the layer's colour and weight
    /// (app/state.ts, the settings schema).
    fn default() -> Self {
        Self {
            ortho: false,
            right_angle: false,
            polar: None,
            snap_aperture: 11.0,
            snap: true,
            snap_kinds: default_snap_kinds(),
            snap_grid: [1.0, 1.0],
            snap_self: true,
            snap_scale: [0.0, 0.0],
            pick_aperture: 5.0,
            tracking: true,
            topology: false,
            topology_points: false,
            overlap: Overlap::Allow,
            color: None,
            line_weight: None,
            geographic: crate::second::Notation::Dms,
            select_kinds: None,
        }
    }
}

/// The overlap control's modes (`drafting.overlap`, docs/adr/0162 §1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overlap {
    /// Serbest: a new area is written as drawn.
    #[default]
    Allow,
    /// Kendi katmanında önle: the visible areas of its own layer.
    Layer,
    /// Seçili katmanlarda önle: those of the chosen layers ([`Context::overlap_layers`]).
    Layers,
}

impl Overlap {
    /// The setting's value: `allow`, `layer` or `layers` (anything else: Serbest).
    pub fn parse(value: &str) -> Overlap {
        match value {
            "layer" => Overlap::Layer,
            "layers" => Overlap::Layers,
            _ => Overlap::Allow,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Overlap::Allow => "allow",
            Overlap::Layer => "layer",
            Overlap::Layers => "layers",
        }
    }
}

/// The snap kinds the settings turn on, as `SnapKind::bit`s: the web's
/// `snapKinds` (ViewportController), where the endpoint setting also brings
/// quadrants (its text says so: “dairelerin çeyrek noktaları”). `on` reads
/// a `snap.*` setting.
pub fn snap_kinds(on: impl Fn(&str) -> bool) -> u32 {
    let mut kinds = 0;
    for (key, kind) in [
        ("snap.endpoint", SnapKind::Endpoint),
        ("snap.midpoint", SnapKind::Midpoint),
        ("snap.center", SnapKind::Center),
        ("snap.node", SnapKind::Node),
        ("snap.endpoint", SnapKind::Quadrant),
        ("snap.intersection", SnapKind::Intersection),
        ("snap.perpendicular", SnapKind::Perpendicular),
        ("snap.nearest", SnapKind::Nearest),
        ("snap.tangent", SnapKind::Tangent),
        ("snap.centroid", SnapKind::Centroid),
        ("snap.extension", SnapKind::Extension),
        ("snap.parallel", SnapKind::Parallel),
        ("snap.grid", SnapKind::Grid),
    ] {
        if on(key) {
            kinds |= kind.bit();
        }
    }
    kinds
}

/// The snap kinds on at first, as the settings schema has them: all but
/// nearest and the additions of docs/adr/0163.
pub fn default_snap_kinds() -> u32 {
    snap_kinds(|key| {
        !matches!(
            key,
            "snap.nearest" | "snap.centroid" | "snap.extension" | "snap.parallel" | "snap.grid"
        )
    })
}

/// Metres of paper per logical pixel at 96 dpi (0.26458 mm), the web's
/// `METRES_PER_PX`: the status bar's “Ekran 1:N”.
const METRES_PER_PX: f64 = 0.00026458;

/// The view's screen scale as the status bar shows it, the N of 1:N, from
/// the metres one logical pixel spans (the web's `screenScale`).
pub fn screen_scale(metres_per_px: f64) -> f64 {
    js_round(metres_per_px / METRES_PER_PX)
}

/// A colour new objects take, as a product command writes it: a theme token
/// (`ink`) or a hex (`#E5484D`, with alpha `#E5484D80`). At most nine ASCII
/// bytes, so [`Draft`] stays `Copy`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DraftColor {
    len: u8,
    bytes: [u8; DraftColor::MAX_BYTES],
}

impl DraftColor {
    const MAX_BYTES: usize = 9;

    /// The colour, or `None` for a text longer than nine bytes or not ASCII
    /// (no colour a command takes is).
    pub fn new(text: &str) -> Option<Self> {
        if text.len() > Self::MAX_BYTES || !text.is_ascii() {
            return None;
        }
        let mut bytes = [0; Self::MAX_BYTES];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Some(Self {
            len: text.len() as u8,
            bytes,
        })
    }

    pub fn as_str(&self) -> &str {
        // Written from ASCII in `new`: always UTF-8.
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

impl std::fmt::Debug for DraftColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl Draft {
    /// The current colour as a command's input takes it (CMD-07); `None`:
    /// the layer's.
    pub fn color_text(&self) -> Option<String> {
        self.color.map(|c| c.as_str().to_owned())
    }

    /// Whether snapping works at screen scale 1:`n` (docs/adr/0163 §5):
    /// not nearer than `snap.scaleMin`, not farther than `snap.scaleMax`, 0
    /// no limit. Out of it no snap shows, a one-shot snap neither.
    pub fn snap_in_range(&self, n: f64) -> bool {
        let [min, max] = self.snap_scale;
        (min <= 0.0 || n >= min) && (max <= 0.0 || n <= max)
    }

    /// What rests acquire at screen scale 1:`n` (docs/adr/0085, 0163 §2):
    /// tracking points while tracking is on; ends' extensions and edges'
    /// directions while snapping takes Uzantı and Paralel there.
    pub fn aids(&self, n: f64) -> Aids {
        let snapping = self.snap && self.snap_in_range(n);
        Aids {
            tracking: self.tracking,
            extension: snapping && self.snap_kinds & SnapKind::Extension.bit() != 0,
            parallel: snapping && self.snap_kinds & SnapKind::Parallel.bit() != 0,
        }
    }
}

/// The rectangle tool's corners (the web's `CornerStyle`): sharp, rounded
/// by a radius (Köşe yuvarla) or cut by a distance (Pah).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Corners {
    Sharp,
    Fillet(f64),
    Chamfer(f64),
}

/// Uzat-kısalt's mode (the web's `LengthenTool.mode`): the moving end
/// follows the mouse, or every clicked end changes by a difference, a
/// percentage or to a total length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LengthenMode {
    Dynamic,
    Delta,
    Percent,
    Total,
}

/// Ölçülendirme's style (the web's `DimensionTool.mode`, a `DimensionStyle`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DimensionMode {
    #[default]
    Aligned,
    Linear,
    Angular,
    Radius,
    Diameter,
    /// Koordinat, Yay uzunluğu, Kırıklı yarıçap, Semt and Eğim (docs/adr/0147 §7).
    Ordinate,
    ArcLength,
    Jogged,
    Azimuth,
    Slope,
}

/// What the web's drawing tools keep from one run to the next for as long as
/// the page lives (their static fields; docs/adr/0032, 0047): the host keeps
/// it for as long as the app lives. A new app starts with the web's values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Memory {
    /// The last circle's radius, offered by Teğet-teğet-yarıçap (`CircleTool.lastRadius`); 0: none yet.
    pub circle_radius: f64,
    /// The rectangle tool's rotation, radians (`RectangleTool.rotation`).
    pub rect_rotation: f64,
    /// The rectangle tool's corners (`RectangleTool.corners`).
    pub rect_corners: Corners,
    /// The regular polygon's side count (`RegularPolygonTool.sides`).
    pub polygon_sides: u32,
    /// Whether the regular polygon's circle passes through its corners
    /// (`RegularPolygonTool.inscribed`), else it touches its edges.
    pub polygon_inscribed: bool,
    /// Ötele's distance (`OffsetTool.distance`), metres.
    pub offset_distance: f64,
    /// Ötele's “Noktadan geç” (`OffsetTool.through`): the copy passes through the clicked point.
    pub offset_through: bool,
    /// Whether fillet and chamfer cut the corner's sides back (`CornerTool.trimSides`, Kırp).
    pub corner_trim: bool,
    /// The last fillet radius (`FilletTool.last`); none yet.
    pub fillet_radius: Option<f64>,
    /// The last chamfer distances (`ChamferTool.last`); none yet.
    pub chamfer: Option<(f64, f64)>,
    /// Birleştir's end gap tolerance (`JoinTool.tolerance`), metres.
    pub join_tolerance: f64,
    /// Birleştir's Zincir: a click joins the chain of the object clicked
    /// (`JoinTool.chain`, docs/adr/0161 §2).
    pub join_chain: bool,
    /// Uzat-kısalt's mode and values (`LengthenTool.mode`, `LengthenTool.values`).
    pub lengthen_mode: LengthenMode,
    pub lengthen_delta: f64,
    pub lengthen_percent: f64,
    pub lengthen_total: f64,
    /// Dizi's last rows, columns and spacing, east and north (`ArrayTool.last`).
    pub array_rows: u32,
    pub array_cols: u32,
    pub array_dx: f64,
    pub array_dy: f64,
    /// Kutupsal dizi's count, fill angle in degrees and whether the copies
    /// turn (`PolarArrayTool.last`).
    pub polar_count: u32,
    pub polar_fill: f64,
    pub polar_rotate: bool,
    /// Whether Hizala scales to fit its second pair (`AlignTool.scale`).
    pub align_scale: bool,
    /// Yardımcı çizgi's typed angle, degrees (`XlineTool.angle`; docs/adr/0057).
    pub xline_angle: f64,
    /// Paralel çizgi's left and right distances, metres, whether the axis
    /// is drawn and whether the corridor is one area (`ParallelLineTool`).
    pub parallel_left: f64,
    pub parallel_right: f64,
    pub parallel_axis: bool,
    pub parallel_area: bool,
    /// Halka's inner and outer diameters, metres (`DonutTool`).
    pub donut_inner: f64,
    pub donut_outer: f64,
    /// Revizyon bulutu: a rectangle (else a polygon), and its arc length in
    /// paper millimetres (`RevCloudTool`).
    pub cloud_rect: bool,
    pub cloud_arc_mm: f64,
    /// Böl: the parts, the step in metres, and whether the step applies (`DivideTool`).
    pub divide_parts: u32,
    pub divide_step: f64,
    pub divide_by_step: bool,
    /// Yazı's height in paper millimetres and angle in degrees (`TextTool.heightMm`, `.angle`).
    pub text_height_mm: f64,
    pub text_angle: f64,
    /// Yazı's Hiza (none: the left of the baseline), Genişlik, Zemin and
    /// Artır (`TextTool.align`, `.widthFactor`, `.mask`, `.increment`; docs/adr/0145 §6).
    pub text_align: Option<kentos_contracts::TextAlign>,
    pub text_width_factor: f64,
    pub text_mask: bool,
    pub text_increment: bool,
    /// Çok satırlı yazı's line spacing (`ParagraphTextTool.lineSpacing`, docs/adr/0182 §4): 1 none.
    pub paragraph_spacing: f64,
    /// Kılavuz's arrowhead (none: the filled arrow) and Zemin
    /// (`LeaderTool.arrow`, `.mask`; docs/adr/0146 §7); its height is Yazı's.
    pub leader_arrow: Option<kentos_contracts::LeaderArrow>,
    pub leader_mask: bool,
    /// Tarama's and Çoklu tara's pattern (an index into the core's
    /// `tools::hatch::choices`), its Ölçek and Açı (degrees), İkinci renk
    /// (0xRRGGBB) and Ters, whether the region is found by the line work
    /// rather than a closed object, whether closed objects inside are left
    /// out, whether the hatch follows its objects and whether texts and
    /// inserts are left open (`HatchTool.choice`, `.scale`, `.angle`,
    /// `.color2`, `.inverted`, `.byLines`, `.islands`, `.assoc`, `.texts`;
    /// docs/adr/0186 §4).
    pub hatch_choice: usize,
    pub hatch_scale: f64,
    pub hatch_angle: f64,
    pub hatch_color2: u32,
    pub hatch_inverted: bool,
    pub hatch_by_lines: bool,
    pub hatch_islands: bool,
    pub hatch_assoc: bool,
    pub hatch_texts: bool,
    /// Alan kesiştir's Kaynakları sil, Alan çıkar's Çıkarılanları sil and
    /// İçine tıklayarak alan's Adalar (`AreaIntersectTool.erase`,
    /// `AreaSubtractTool.eraseCutters`, `BoundaryTool.islands`).
    pub area_intersect_erase: bool,
    pub area_subtract_erase: bool,
    pub boundary_islands: bool,
    /// Tek nesne of Alan birleştir, kesiştir and çıkar: the result one
    /// multi-part area (`AreaTools.oneObject`, docs/adr/0143).
    pub area_one_object: bool,
    /// Ölçülendirme's style, its linear direction lock in degrees (0 ΔY, 90 ΔX;
    /// none: from where the line is placed) and whether an angle is measured
    /// from its vertex (`DimensionTool.mode`, `.lock`, `.byVertex`).
    pub dimension_mode: DimensionMode,
    pub dimension_lock: Option<f64>,
    pub dimension_by_vertex: bool,
    /// Koordinat's axis lock in degrees (0 its Y, 90 its X; none: from the
    /// cursor) and Yay uzunluğu's Kısmi (`DimensionTool.ordinateLock`,
    /// `.arcPartial`; docs/adr/0147 §7).
    pub ordinate_lock: Option<f64>,
    pub arc_partial: bool,
    /// Semt's and Eğim's Kenardan: an edge clicked gives the two points
    /// (`DimensionTool.byEdge`; docs/adr/0147 §7).
    pub dimension_by_edge: bool,
    /// Ölçülendirme's Zemin (Z): the value written over the drawing's
    /// background (`DimensionTool.mask`; docs/adr/0147 §7).
    pub dimension_mask: bool,
    /// Sadeleştir's tolerance, metres (docs/adr/0140). Tüm köşeleri yuvarla and
    /// Tüm köşelere pah keep their values in `fillet_radius` and `chamfer`,
    /// as the one-corner tools do.
    pub simplify_tolerance: f64,
    /// Parçala's part count (Eşit parçalara) and piece length, metres (Uzunluktan).
    pub split_parts: u32,
    pub split_length: f64,
    /// Ara nokta's part count, and the distances (metres) and ratios last typed.
    pub between_parts: u32,
    pub between_distances: Values,
    pub between_ratios: Values,
    /// Kesişim noktası's last distance, metres, offered by Enter (docs/adr/0140).
    pub meeting_distance: f64,
    /// The newest aligned or linear dimension written, by its persistent id:
    /// where Zincir ölçü and Baz ölçü start when none is clicked.
    pub last_dimension: Option<kentos_domain::Uuid>,
    /// The text style Yazı, Çok satırlı yazı and Metin dosyası yerleştir
    /// write in, and the dimension style of every dimension tool, by their
    /// ids (docs/adr/0183 §4); none: Standart. A style the project no
    /// longer has is Standart.
    pub text_style: Option<kentos_domain::Uuid>,
    pub dimension_style: Option<kentos_domain::Uuid>,
    /// Ötele's “İki yana” (both sides) and “Kaynağı sil” (delete the source).
    pub offset_both: bool,
    pub offset_erase: bool,
    /// Yol boyunca dizi's count, spacing (metres), whether the spacing rules
    /// (the count then fills the path) and whether the copies turn along it.
    pub path_count: u32,
    pub path_spacing: f64,
    pub path_by_spacing: bool,
    pub path_align: bool,
    /// Mesafe ölç's “Sabit ilk nokta” (every point is measured from the first),
    /// Alan hesapla's “İçine tıkla” (a click inside a closed region measures
    /// it) and Daireyle seç's “Kesişen” (touching objects are selected too;
    /// docs/adr/0141).
    pub measure_fixed: bool,
    pub area_inside: bool,
    pub circle_crossing: bool,
    /// Çokgenle seç's mode: İçindekiler, Kesişenler or Dışındakiler
    /// (docs/adr/0187 §2).
    pub polygon_select: kentos_geometry_core::store::polygon::PolygonMode,
    /// Benzerini seç's criteria (docs/adr/0187 §4): all on at first.
    pub similar: crate::select_similar::Criteria,
    /// The path tools' İzle: the next segments follow the visible line work
    /// (docs/adr/0161 §1).
    pub trace: bool,
    /// The path tools' Akış and its step, metres: the pointer leaves a vertex
    /// every step it goes (docs/adr/0161 §3).
    pub stream: bool,
    pub stream_step: f64,
    /// Blok ekle's block, scale, turn in degrees and mirror
    /// (`BlockInsertTool.block`, `.scale`, `.rotation`, `.mirror`; docs/adr/0144).
    pub block_insert: Option<kentos_contracts::BlockId>,
    pub block_scale: f64,
    pub block_rotation: f64,
    pub block_mirror: bool,
    /// Topolojik temizlik's tolerance, metres, and its four works
    /// (`TopologyTool.tolerance`, `.works`; docs/adr/0148 §3).
    pub topology_tolerance: f64,
    pub topology_works: kentos_geometry_core::ops::topology::TopoWorks,
    /// Toplu alan's Adalar and the attribute its labels go to
    /// (`PolygonizeTool.islands`, `.attribute`; docs/adr/0151 §3, §4).
    pub polygonize_islands: bool,
    pub polygonize_attribute: Name,
    /// Nokta's Ad, Kod and Kot: the next point's name (it moves on by
    /// Artır), the points' code and elevation; Köşelere nokta starts from
    /// the name and gives its code (`SurveyPointTool.next`, `.code`, `.z`;
    /// docs/adr/0152 §2, §5).
    pub point_name: Name,
    pub point_code: Name,
    pub point_z: Option<f64>,
    /// Etiketleri yazıya çevir's Örtüşenler de, Zemin, Katman (the active
    /// layer instead of the text layer) and Nesneye bağlı
    /// (`LabelsToTextTool.every`, `.mask`, `.active`, `.linked`; docs/adr/0175 §3).
    pub labels_every: bool,
    pub labels_mask: bool,
    pub labels_active: bool,
    pub labels_linked: bool,
    /// Koordinat yaz's Kollu, Yön, Şablon (empty: the project type's),
    /// Basamak (none: the project's) and Yükseklik on paper, mm, and
    /// Köşelere koordinat yaz's Çizelge (`coordinateOptions`; docs/adr/0185 §5).
    pub coordinate_leader: bool,
    pub coordinate_direction: kentos_geometry_core::ops::coordinate_labels::Direction,
    pub coordinate_template: Name,
    pub coordinate_decimals: Option<u8>,
    pub coordinate_height_mm: f64,
    pub coordinate_schedule: bool,
}

/// A short text kept in [`Memory`], which is `Copy`: at most
/// [`Name::MAX_CHARS`] characters (an attribute's name).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Name {
    len: u16,
    bytes: [u8; Name::MAX_BYTES],
}

impl Name {
    pub const MAX_CHARS: usize = 60;
    const MAX_BYTES: usize = Self::MAX_CHARS * 4;
    /// The empty text.
    pub const EMPTY: Self = Self {
        len: 0,
        bytes: [0; Self::MAX_BYTES],
    };

    /// The text, or `None` when it is longer than [`Name::MAX_CHARS`] characters.
    pub fn new(text: &str) -> Option<Self> {
        if text.chars().count() > Self::MAX_CHARS {
            return None;
        }
        let mut bytes = [0; Self::MAX_BYTES];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Some(Self {
            len: text.len() as u16,
            bytes,
        })
    }

    pub fn as_str(&self) -> &str {
        // Written from a `&str` in `new`: always UTF-8.
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

/// A short list of numbers typed as one answer (Ara nokta's distances and
/// ratios), kept in [`Memory`], which is `Copy`: at most [`Values::MAX`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Values {
    len: u8,
    v: [f64; Values::MAX],
}

impl Values {
    pub const MAX: usize = 32;

    /// The list, or `None` when it is longer than [`Values::MAX`].
    pub fn from_slice(list: &[f64]) -> Option<Self> {
        if list.len() > Self::MAX {
            return None;
        }
        let mut v = [0.0; Self::MAX];
        v[..list.len()].copy_from_slice(list);
        Some(Self {
            len: list.len() as u8,
            v,
        })
    }

    pub fn as_slice(&self) -> &[f64] {
        &self.v[..usize::from(self.len)]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for Values {
    fn default() -> Self {
        Self {
            len: 0,
            v: [0.0; Self::MAX],
        }
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            circle_radius: 0.0,
            rect_rotation: 0.0,
            rect_corners: Corners::Sharp,
            polygon_sides: 6,
            polygon_inscribed: true,
            offset_distance: 1.0,
            offset_through: false,
            corner_trim: true,
            fillet_radius: None,
            chamfer: None,
            join_tolerance: 0.001,
            join_chain: false,
            lengthen_mode: LengthenMode::Dynamic,
            lengthen_delta: 1.0,
            lengthen_percent: 100.0,
            lengthen_total: 10.0,
            array_rows: 2,
            array_cols: 3,
            array_dx: 10.0,
            array_dy: 10.0,
            polar_count: 6,
            polar_fill: 360.0,
            polar_rotate: true,
            align_scale: false,
            xline_angle: 0.0,
            parallel_left: 5.0,
            parallel_right: 5.0,
            parallel_axis: true,
            parallel_area: false,
            donut_inner: 0.5,
            donut_outer: 1.0,
            cloud_rect: true,
            cloud_arc_mm: 8.0,
            divide_parts: 4,
            divide_step: 10.0,
            divide_by_step: false,
            text_height_mm: 2.5,
            text_angle: 0.0,
            text_align: None,
            text_width_factor: 1.0,
            text_mask: false,
            text_increment: false,
            paragraph_spacing: 1.0,
            leader_arrow: None,
            leader_mask: false,
            hatch_choice: kentos_geometry_core::tools::hatch::DEFAULT_CHOICE,
            hatch_scale: 1.0,
            hatch_angle: 0.0,
            hatch_color2: 0xFF_FF_FF,
            hatch_inverted: false,
            hatch_by_lines: false,
            hatch_islands: true,
            hatch_assoc: true,
            hatch_texts: false,
            area_intersect_erase: false,
            area_subtract_erase: false,
            boundary_islands: true,
            area_one_object: false,
            dimension_mode: DimensionMode::Aligned,
            dimension_lock: None,
            dimension_by_vertex: false,
            ordinate_lock: None,
            arc_partial: false,
            dimension_by_edge: false,
            dimension_mask: false,
            simplify_tolerance: 0.01,
            split_parts: 4,
            split_length: 10.0,
            between_parts: 4,
            between_distances: Values::default(),
            between_ratios: Values::default(),
            meeting_distance: 10.0,
            last_dimension: None,
            text_style: None,
            dimension_style: None,
            offset_both: false,
            offset_erase: false,
            path_count: 5,
            path_spacing: 10.0,
            path_by_spacing: false,
            path_align: true,
            measure_fixed: false,
            area_inside: false,
            circle_crossing: false,
            polygon_select: kentos_geometry_core::store::polygon::PolygonMode::Inside,
            similar: crate::select_similar::Criteria::ALL,
            trace: false,
            stream: false,
            stream_step: 1.0,
            block_insert: None,
            block_scale: 1.0,
            block_rotation: 0.0,
            block_mirror: false,
            topology_tolerance: crate::topology::FIRST_TOLERANCE,
            topology_works: crate::topology::FIRST_WORKS,
            polygonize_islands: true,
            polygonize_attribute: Name::new(crate::polygonize::FIRST_ATTRIBUTE)
                .unwrap_or(Name::EMPTY),
            point_name: Name::EMPTY,
            point_code: Name::EMPTY,
            point_z: None,
            labels_every: false,
            labels_mask: false,
            labels_active: false,
            labels_linked: false,
            coordinate_leader: true,
            coordinate_direction: kentos_geometry_core::ops::coordinate_labels::Direction::Auto,
            coordinate_template: Name::EMPTY,
            coordinate_decimals: None,
            coordinate_height_mm: crate::coordinate_labels::FIRST_HEIGHT_MM,
            coordinate_schedule: false,
        }
    }
}

/// What a tool works with during one call.
pub struct Context<'a> {
    /// The open drawing: the only way a tool changes anything.
    pub doc: &'a mut Document,
    pub view: &'a dyn View,
    pub draft: Draft,
    /// Messages for the user, newest last; the host shows them.
    pub log: &'a mut Vec<Line>,
    /// The geometry store, in step with the document when the call began
    /// (docs/adr/0029): what a click picks.
    pub spatial: &'a Spatial,
    /// The selection: the select tool changes it, the erase tool deletes it.
    pub selection: &'a mut Selection,
    /// What the drawing tools remember between runs.
    pub memory: &'a mut Memory,
    /// Changes of the view the tool asks for (Kaydır, Pencere yakınlaştır):
    /// the host applies them after the call.
    pub view_changes: &'a mut Vec<ViewChange>,
    /// Object tracking's points and lock (docs/adr/0085): a typed distance
    /// goes along the line the cursor is locked to.
    pub tracking: &'a ObjectTracking,
    /// Shift is held now: what a selecting tool finds is added to the
    /// selection instead of replacing it (docs/adr/0141). A pointer event
    /// carries it too ([`Pointer::shift`]); this is for Enter and the like.
    pub shift: bool,
    /// Seçili katmanlarda önle's layers, by id (docs/adr/0162 §1): the
    /// session's, not a setting (settings hold no lists).
    pub overlap_layers: &'a [String],
    /// The digitizing locks (docs/adr/0166): what holds the next point. The
    /// session lets one-shot locks go when the tool's reference moves; the
    /// tools read them through [`crate::points`]'s cursor rule.
    pub locks: &'a mut crate::locks::LockState,
    /// The object template being drawn with (docs/adr/0176 §3): what every
    /// object the tool writes takes besides its geometry; none while the
    /// tool runs by itself.
    pub template: Option<&'a crate::templates::Stamp>,
    /// The layers and groups Katmanı yalıt hid since the last Yalıtımı
    /// kaldır, by id, in order (docs/adr/0177 §1): the drawing's, the host
    /// forgets them with it.
    pub isolated_layers: &'a mut Vec<String>,
}

impl Context<'_> {
    pub fn format(&self) -> Format {
        Format::of(self.doc.settings())
    }

    /// The symbol a new object takes: its object template's (docs/adr/0176
    /// §3); none without one.
    pub fn template_symbol(&self) -> Option<String> {
        self.template.and_then(|t| t.symbol.clone())
    }

    /// The label a new object takes: its object template's; none without one.
    pub fn template_label(&self) -> Option<String> {
        self.template.and_then(|t| t.label.clone())
    }

    /// The attributes a new object takes: its object template's, the tool's
    /// own (`own`: Nokta's Kod) over them; none when both are empty.
    pub fn template_attrs(
        &self,
        own: Option<std::collections::BTreeMap<String, String>>,
    ) -> Option<std::collections::BTreeMap<String, String>> {
        let mut attrs = self.template.map(|t| t.attrs.clone()).unwrap_or_default();
        attrs.extend(own.unwrap_or_default());
        (!attrs.is_empty()).then_some(attrs)
    }

    /// The point `distance` along the tracking line the cursor is locked to
    /// (a typed distance while tracking, the web's `trackAlong`); none when
    /// the cursor is on no single line.
    pub fn track_along(&self, distance: f64) -> Option<Vec2> {
        self.tracking.along(distance)
    }

    /// Typed point text (the web's `pointFromText`): an absolute, relative or
    /// polar point, or a distance along the tracking line or toward
    /// `cursor`; its lengths and coordinates typed in the project's unit
    /// (docs/adr/0165 §2), a polar angle in its type's way and angle unit (§4).
    pub fn typed_point(
        &self,
        text: &str,
        last: Option<Vec2>,
        cursor: Option<Vec2>,
    ) -> Option<Vec2> {
        let format = self.format();
        // Relative input is measured from a reference point, when one is set (docs/adr/0166 §5).
        let last = self.locks.reference.or(last);
        kentos_geometry_core::tools::point_text::point_from_text_in(
            text,
            last,
            cursor,
            |d| self.track_along(d),
            |v| format.to_metres(v),
            format.angles(),
        )
    }

    /// A typed length (a distance, a radius, a tolerance, an elevation) in
    /// the project's unit, in metres (the web's `parseLength`); none for no
    /// number. Counts, angles, factors and paper millimetres are read with
    /// `parse_number`.
    pub fn typed_length(&self, text: &str) -> Option<f64> {
        kentos_geometry_core::tools::point_text::parse_number(text)
            .map(|n| self.format().to_metres(n))
    }

    pub(crate) fn say(&mut self, level: Level, text: impl Into<String>) {
        self.log.push(Line::new(level, text));
    }

    /// The world length the pick aperture spans at this zoom (the web's
    /// `pickAperture / camera.scale`).
    pub(crate) fn pick_tolerance(&self) -> f64 {
        self.view.world_length(self.draft.pick_aperture)
    }
}

/// Whether the tool stays after a confirm, or the session goes back to idle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Stay,
    Exit,
}

/// A measurement shown beside the cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct Tag {
    /// The cursor's world point; the host places the tag below-right of it.
    pub at: Vec2,
    pub lines: Vec<String>,
}

/// The colour a preview part is drawn in, from the theme (the web's palette):
/// the accent, the danger colour (what a trim takes away, a vertex to go) or
/// the snap colour (a corner's reach).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Accent,
    Danger,
    Snap,
}

/// A line of a draft drawn as the web's `strokePath` draws it: the circle,
/// the arc or the rectangle that would be written, a dashed guide circle.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    /// World points, curves already tessellated by the shared core.
    pub pts: Vec<Vec2>,
    pub closed: bool,
    /// Dash and gap, logical pixels; solid when none.
    pub dash: Option<[f32; 2]>,
    /// Logical pixels.
    pub width: f32,
    pub tone: Tone,
}

impl Stroke {
    /// A solid 1 px line.
    pub fn solid(pts: Vec<Vec2>, closed: bool) -> Self {
        Self {
            pts,
            closed,
            dash: None,
            width: 1.0,
            tone: Tone::Accent,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn dashed(pts: Vec<Vec2>, closed: bool, dash: [f32; 2]) -> Self {
        Self {
            dash: Some(dash),
            ..Self::solid(pts, closed)
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

/// What the running tool wants drawn over the drawing; the drawing itself
/// does not change until a confirm (ADR 0018, “Önizleme”). World
/// coordinates; arcs already tessellated by the shared core.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preview {
    /// The shape so far with the cursor's segment, open.
    pub path: Vec<Vec2>,
    /// The closed shape (dashed, lightly filled) once it has three corners.
    pub ring: Option<Vec<Vec2>>,
    /// Helper lines, dashed: to a point given before an arc's end, and the like.
    pub guides: Vec<[Vec2; 2]>,
    /// Lines drawn as the web's `strokePath` draws them (docs/adr/0032).
    pub strokes: Vec<Stroke>,
    /// Points marked with a 9 px square: the objects picked for a tangent circle.
    pub squares: Vec<Vec2>,
    /// Points marked with a 7 px square, solid: points and texts among a
    /// modify tool's ghosts (the web's `strokePaths` markers, docs/adr/0037).
    pub marks: Vec<Vec2>,
    pub tag: Option<Tag>,
    /// The tag's colour: the danger colour where it names what goes (web `drawTag`).
    pub tag_tone: Tone,
    /// Marks at points, drawn 2 px wide: a corner found under the cursor, a
    /// vertex to add or remove (docs/adr/0047).
    pub markers: Vec<Marker>,
    /// A polar tracking ray the cursor is locked to.
    pub tracking: Option<Tracking>,
    /// Filled areas as the web's `drawArea` draws them: a corridor, a donut (docs/adr/0057).
    pub areas: Vec<Area>,
    /// Short texts beside points: the reference line's start “A” (docs/adr/0057).
    pub labels: Vec<Label>,
    /// The lines of a hatch to come, drawn faint (the web's 60 %; docs/adr/0062).
    pub hatch: Vec<[Vec2; 2]>,
    /// Texts to come, drawn faint as text objects are drawn (the web's
    /// `drawTextGhost`; Etiketleri yazıya çevir, docs/adr/0175 §3).
    pub texts: Vec<TextGhost>,
    /// A table's words to come, drawn faint in its face (Tablo ekle's
    /// placement, docs/adr/0184 §3; the web's `TablePlaceTool.draw`).
    pub cells: Vec<CellGhost>,
}

/// A table's cell to come: where its words' baseline starts, its words,
/// its height on the ground, bold (a heading row's), the table's face.
#[derive(Clone, Debug, PartialEq)]
pub struct CellGhost {
    pub at: Vec2,
    pub text: String,
    pub height: f64,
    pub bold: bool,
    pub face: kentos_contracts::TextFace,
}

/// A text to come: where its alignment puts it, its text, its height on
/// the ground, its turn in degrees and its alignment (none: the left of its
/// baseline); over its mask; in its face (a style's, docs/adr/0183 §2) and
/// width factor.
#[derive(Clone, Debug, PartialEq)]
pub struct TextGhost {
    pub p: Vec2,
    pub text: String,
    pub height: f64,
    pub rotation: f64,
    pub align: Option<kentos_geometry_core::text::TextAlign>,
    pub mask: bool,
    pub face: kentos_contracts::TextFace,
    pub width_factor: f64,
}

/// An area of a draft, filled in its tone and outlined: the outer ring, then
/// its holes (even-odd).
#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    pub rings: Vec<Vec<Vec2>>,
    /// The fill's opacity over the tone's colour (the web's `tint(accent, 0.16)`).
    pub fill: f32,
    /// The outline, logical pixels.
    pub width: f32,
    /// The outline's dash and gap, logical pixels; solid when none (a hatch's region, docs/adr/0062).
    pub dash: Option<[f32; 2]>,
    /// The fill's colour: the accent, or the snap colour for every other
    /// piece Alan böl would leave (docs/adr/0065).
    pub fill_tone: Tone,
}

/// A text at a world point, moved by a logical pixel offset (right and down).
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub at: Vec2,
    pub text: String,
    pub offset: [f32; 2],
    pub tone: Tone,
}

/// A mark at a world point, sized in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marker {
    pub at: Vec2,
    pub shape: MarkerShape,
    pub tone: Tone,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MarkerShape {
    /// A circle of this radius (the corner tools' 7 px ring).
    Ring(f32),
    /// An × this far from its centre (a vertex to remove).
    Cross(f32),
    /// A + this far from its centre (a vertex to add).
    Plus(f32),
    /// A 1 px circle of this radius (Böl's points to come; docs/adr/0057).
    Circle(f32),
    /// The right angle at a perpendicular's foot: an 8 px square corner, 1
    /// px, its sides towards these world points (docs/adr/0057).
    RightAngle { along: Vec2, up: Vec2 },
}

/// An interactive tool.
pub trait Tool {
    /// The tool's id (`polygon`); its command is `tool.<id>`.
    fn id(&self) -> &'static str;
    /// The tool's name in the prompt (`Kapalı alan`).
    fn label(&self) -> &'static str;
    fn prompt(&self) -> Prompt;
    /// Points the running command has taken (the web's `Tool.pointCount`).
    fn point_count(&self) -> usize;
    /// Right after it starts, with what the session knows (the web's
    /// `activate`): the erase tool deletes the selection and leaves.
    fn activate(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }
    /// Whether object snaps apply while it runs (the web's `Tool.snaps`).
    fn snaps(&self) -> bool {
        true
    }
    /// The point perpendicular and tangent snaps are taken from: the last
    /// point given (the web's `snapFrom`).
    fn snap_from(&self) -> Option<Vec2> {
        None
    }
    /// The object being drawn, its points so far (not the segment to the
    /// cursor) as an open path: snapped to as one more object while
    /// `snap.self` is on (the web's `draftPath`, docs/adr/0163 §3).
    fn draft_path(&self) -> Option<GeomShape> {
        None
    }
    /// The unit direction the object being drawn travels at its last point
    /// (its last edge's; a path's tangent there): Sapma turns from it and
    /// Dik açı keeps square to it (docs/adr/0166 §1, §4; the web's
    /// `travelDirection`). None before the first edge.
    fn travel(&self) -> Option<Vec2> {
        None
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>);
    /// The left button went down.
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>);
    /// The left button came up.
    fn pointer_up(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}
    /// Typed text: a coordinate, a number or an option. False when not understood.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool;
    /// The values an option chooses between, for its chip and the right
    /// button's menu to offer as a menu (Yazı's Hiza, docs/adr/0145 §6; the
    /// web's `optionChoices`); none: the chip sends the key.
    fn option_choices(&self, _key: &str) -> Vec<OptionChoice> {
        Vec::new()
    }
    /// One of `option_choices(key)` chosen, as typing the key and then
    /// `typed`; false when this step takes none (the web's `chooseOption`).
    fn choose_option(&mut self, _key: &str, _typed: &str, _cx: &mut Context<'_>) -> bool {
        false
    }
    /// Enter, Space or a quick right click.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow;
    /// Whether the tool takes a confirm at all (the web's `Tool.confirm`
    /// being there). Kaydır and Pencere yakınlaştır take none: Enter and
    /// Space repeat the last command, a quick right click does what it does
    /// with no command running (docs/adr/0056).
    fn confirms(&self) -> bool {
        true
    }
    /// Whether this step asks for words (a style's name, docs/adr/0183 §4):
    /// Space types a space in the command line then, Enter alone confirms
    /// (the web's `Tool.takesWords`).
    fn takes_words(&self) -> bool {
        false
    }
    /// The pointer's look over the drawing while it runs.
    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }
    /// Ctrl+Z while the tool runs: takes back its newest step and returns
    /// true, or false when nothing is pending and the drawing is undone instead.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool;
    /// The answer of a text field it asked for ([`ViewChange::Text`]): the
    /// typed text (Enter, a click elsewhere), or none (Esc).
    fn text_typed(&mut self, _text: Option<&str>, _cx: &mut Context<'_>) {}
    /// The answer of the paragraph editor it asked for
    /// ([`ViewChange::Paragraph`]): the text and its letter formats (Tamam),
    /// or none (Vazgeç).
    fn paragraph_typed(
        &mut self,
        _typed: Option<(&str, &[kentos_contracts::TextRun])>,
        _cx: &mut Context<'_>,
    ) {
    }
    /// The file it asked for ([`ViewChange::OpenTextFile`]): its name and
    /// bytes, or none (the picker cancelled).
    fn file_given(&mut self, _file: Option<(&str, &[u8])>, _cx: &mut Context<'_>) {}
    /// The answer of the values it asked for ([`ViewChange::AttributeValues`]):
    /// the attributes to write (Yerleştir), or none (Vazgeç, Esc).
    fn values_given(
        &mut self,
        _values: Option<&std::collections::BTreeMap<String, String>>,
        _cx: &mut Context<'_>,
    ) {
    }
    /// Esc while the tool runs: it steps back (drops the object it picked,
    /// leaves a sub-step) and returns true, or false when it has nothing to
    /// drop and the session leaves it (the web's `Tool.cancel`).
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }
    fn preview(&self, format: &Format) -> Preview;
    /// The tool is done and leaves after the call that finished it (the web's
    /// `ctx.tools.exit()` from a click or typed text: a move written).
    fn finished(&self) -> bool {
        false
    }
    /// The selection box being drawn by a tool that picks objects itself
    /// (the modify tools before their points), for the host to show.
    fn select_box(&self) -> Option<SelectBox> {
        None
    }
    /// Whether the tool takes points computed elsewhere: the point
    /// calculator runs over it (the web's `acceptPoint` is there,
    /// docs/adr/0083).
    fn accepts_points(&self) -> bool {
        false
    }
    /// A computed point, as if clicked at this step: the same as a typed
    /// point, no ortho or polar applied (the web's `acceptPoint`). False
    /// when this step takes no point.
    fn accept_point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> bool {
        false
    }
    /// A tool run over a suspended command (the point calculator): the
    /// point it hands to that command once it has finished.
    fn computed(&self) -> Option<Vec2> {
        None
    }
}
