//! What each kind of item holds (design §3.1): the shared frame is `Item`'s,
//! the content here. Kinds are a tagged union on `type`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::model::{ItemId, SortKey, yes};
use crate::style::{HAlign, Stroke, TextStyle, VAlign, black, white};
use crate::units::{Mdeg, SizeUm, Um};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum ItemKind {
    /// A map frame: in CAD the “Görünüm penceresi” (profiles name it).
    Map(MapItem),
    Text(TextItem),
    ScaleBar(ScaleBarItem),
    NorthArrow(NorthArrowItem),
    Legend(LegendItem),
    Picture(PictureItem),
    Shape(ShapeItem),
    Line(LineItem),
    Table(TableItem),
    CoordinateList(CoordinateListItem),
    TitleBlock(TitleBlockItem),
    /// The sheet's frame (pafta çerçevesi).
    Border(BorderItem),
    /// Its children name it in their `group`; its frame is theirs together.
    Group(GroupItem),
}

/// Nothing of its own: the content of a tagged variant that has none, so a stray field beside its `type` is refused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Empty {}

/// A group holds nothing of its own: its children name it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct GroupItem {}

impl ItemKind {
    /// The kind's `type` (stable).
    pub fn type_name(&self) -> &'static str {
        match self {
            ItemKind::Map(_) => "map",
            ItemKind::Text(_) => "text",
            ItemKind::ScaleBar(_) => "scaleBar",
            ItemKind::NorthArrow(_) => "northArrow",
            ItemKind::Legend(_) => "legend",
            ItemKind::Picture(_) => "picture",
            ItemKind::Shape(_) => "shape",
            ItemKind::Line(_) => "line",
            ItemKind::Table(_) => "table",
            ItemKind::CoordinateList(_) => "coordinateList",
            ItemKind::TitleBlock(_) => "titleBlock",
            ItemKind::Border(_) => "border",
            ItemKind::Group(_) => "group",
        }
    }

    /// The kind's Turkish name, for messages.
    pub fn label(&self) -> &'static str {
        match self {
            ItemKind::Map(_) => "Harita",
            ItemKind::Text(_) => "Metin",
            ItemKind::ScaleBar(_) => "Ölçek çubuğu",
            ItemKind::NorthArrow(_) => "Kuzey oku",
            ItemKind::Legend(_) => "Lejant",
            ItemKind::Picture(_) => "Resim",
            ItemKind::Shape(_) => "Şekil",
            ItemKind::Line(_) => "Çizgi",
            ItemKind::Table(_) => "Tablo",
            ItemKind::CoordinateList(_) => "Koordinat listesi",
            ItemKind::TitleBlock(_) => "Antet",
            ItemKind::Border(_) => "Pafta çerçevesi",
            ItemKind::Group(_) => "Grup",
        }
    }

    /// The map an item is tied to (scale bar, north arrow, legend, overview, text's `@olcek`).
    pub fn map_link(&self) -> Option<&ItemId> {
        match self {
            ItemKind::ScaleBar(s) => s.map.as_ref(),
            ItemKind::NorthArrow(n) => n.map.as_ref(),
            ItemKind::Legend(l) => l.map.as_ref(),
            ItemKind::Text(t) => t.map.as_ref(),
            _ => None,
        }
    }
}

// ── Map ──────────────────────────────────────────────────────────────────

/// A point on the ground in the project's coordinate system, metres: `x` east (Y in the surveyor's words), `y` north (X).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct GroundPoint {
    pub x: f64,
    pub y: f64,
}

/// A map at a fixed place and scale. A template's map has no centre: it keeps its scale, not a place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct FixedView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub center: Option<GroundPoint>,
    /// Denominator: 1000 is 1/1000.
    pub scale: u32,
    /// The map's content turned clockwise on the paper.
    #[serde(default)]
    pub rotation: Mdeg,
}

/// A map that follows the atlas: each page's object, at the scale the policy chooses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AtlasView {
    pub policy: AtlasScale,
    #[serde(default)]
    pub rotation: Mdeg,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum MapView {
    Fixed(FixedView),
    Atlas(AtlasView),
}

/// The object with a margin around it, at the largest standard scale it fits (or the exact scale).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct FitScale {
    /// Margin around the object, percent of its size on each side.
    #[serde(default)]
    pub margin_pct: u16,
    #[serde(default = "yes")]
    pub standard_scales: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct FixedScale {
    pub scale: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PredefinedScales {
    pub scales: Vec<u32>,
}

/// How an atlas map chooses its scale (design §7a).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum AtlasScale {
    Fit(FitScale),
    Fixed(FixedScale),
    Predefined(PredefinedScales),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ThemeRef {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LayerList {
    pub layers: Vec<String>,
}

/// Which layers a map shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum MapLayers {
    /// The layers visible in the drawing.
    All(Empty),
    Theme(ThemeRef),
    List(LayerList),
}

impl Default for MapLayers {
    fn default() -> Self {
        MapLayers::All(Empty {})
    }
}

fn overview_frame() -> Stroke {
    Stroke::solid("#d0021b", 500)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MapItem {
    pub view: MapView,
    #[serde(default)]
    pub layers: MapLayers,
    /// Another coordinate system for the map (EPSG code text); none: the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crs: Option<String>,
    /// Coordinate grids (karelaj).
    #[serde(default)]
    pub grids: Vec<MapGrid>,
    /// An overview: this map shows where that map's frame is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub overview_of: Option<ItemId>,
    /// How the overview draws the other map's frame.
    #[serde(default = "overview_frame")]
    pub overview_frame: Stroke,
    /// The host clips the content to the atlas object.
    #[serde(default)]
    pub clip_to_atlas_feature: bool,
    /// A band inside the frame and around the content for the grid's outside labels: the frame holds them (design §3.1).
    #[serde(default)]
    pub label_band: Um,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum GridKind {
    /// Crosses at the intersections (karelaj artıları).
    #[default]
    Cross,
    Lines,
    /// Ticks on the frame only.
    Ticks,
    /// Only the frame and the labels.
    FrameOnly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum GridFrame {
    None,
    Zebra,
    Ticks,
    #[default]
    Line,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum LabelPosition {
    /// In the label band, outside the content.
    #[default]
    Outside,
    Inside,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum LabelDirection {
    Horizontal,
    /// Along the edge: upright on the left and right edges.
    #[default]
    AlongEdge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct GridSides {
    #[serde(default = "yes")]
    pub top: bool,
    #[serde(default = "yes")]
    pub right: bool,
    #[serde(default = "yes")]
    pub bottom: bool,
    #[serde(default = "yes")]
    pub left: bool,
}

impl Default for GridSides {
    fn default() -> Self {
        GridSides {
            top: true,
            right: true,
            bottom: true,
            left: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MetresFormat {
    #[serde(default)]
    pub decimals: u8,
    /// A space between thousands (485 200).
    #[serde(default)]
    pub group_thousands: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct DmsFormat {
    #[serde(default)]
    pub seconds_decimals: u8,
}

/// An expression over `@deger` (the line's value) and `@eksen` (“Y” or “X”).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ExpressionFormat {
    pub expression: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum GridLabelFormat {
    Metres(MetresFormat),
    Dms(DmsFormat),
    Expression(ExpressionFormat),
}

impl Default for GridLabelFormat {
    fn default() -> Self {
        GridLabelFormat::Metres(MetresFormat::default())
    }
}

fn grid_label_style() -> TextStyle {
    TextStyle::new(1_800, 400)
}

fn one_mm() -> Um {
    1_000
}

/// A grid's labels: by default whole metres, Y on the top and bottom edges,
/// X on the left and right ones along the edge, in the label band.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct GridLabels {
    #[serde(default = "yes")]
    pub show: bool,
    #[serde(default)]
    pub sides: GridSides,
    #[serde(default)]
    pub position: LabelPosition,
    #[serde(default)]
    pub direction: LabelDirection,
    #[serde(default)]
    pub format: GridLabelFormat,
    /// “Y=” and “X=” before the value.
    #[serde(default = "yes")]
    pub axis_prefix: bool,
    #[serde(default = "grid_label_style")]
    pub style: TextStyle,
    /// From the content's edge (outside the zebra or ticks) to the label.
    #[serde(default = "one_mm")]
    pub gap: Um,
}

impl Default for GridLabels {
    fn default() -> Self {
        GridLabels {
            show: true,
            sides: GridSides::default(),
            position: LabelPosition::Outside,
            direction: LabelDirection::AlongEdge,
            format: GridLabelFormat::default(),
            axis_prefix: true,
            style: grid_label_style(),
            gap: one_mm(),
        }
    }
}

fn grid_stroke() -> Stroke {
    Stroke::solid("#000000", 180)
}

fn cross_arm() -> Um {
    2_500
}

fn zebra_width() -> Um {
    1_500
}

/// A coordinate grid on a map (karelaj, design §3.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MapGrid {
    #[serde(default)]
    pub name: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub kind: GridKind,
    /// Metres between lines, `[east (Y), north (X)]`; 0: chosen from the scale (10 cm on paper).
    #[serde(default)]
    pub interval: [f64; 2],
    #[serde(default)]
    pub offset: [f64; 2],
    #[serde(default = "grid_stroke")]
    pub stroke: Stroke,
    /// A cross's arm from its centre.
    #[serde(default = "cross_arm")]
    pub cross_size: Um,
    #[serde(default)]
    pub frame: GridFrame,
    #[serde(default = "zebra_width")]
    pub frame_width: Um,
    #[serde(default)]
    pub labels: GridLabels,
    /// A grid in another coordinate system (EPSG code text); none: the map's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crs: Option<String>,
}

impl Default for MapGrid {
    fn default() -> Self {
        MapGrid {
            name: String::new(),
            enabled: true,
            kind: GridKind::Cross,
            interval: [0.0, 0.0],
            offset: [0.0, 0.0],
            stroke: grid_stroke(),
            cross_size: cross_arm(),
            frame: GridFrame::Line,
            frame_width: zebra_width(),
            labels: GridLabels::default(),
            crs: None,
        }
    }
}

// ── Text ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum TextFit {
    #[default]
    None,
    /// The letters get smaller until the text fits its frame (never below half their size).
    ShrinkToFit,
}

pub(crate) fn line_height() -> u16 {
    120
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TextItem {
    /// Text with `[% expression %]` parts.
    pub content: String,
    #[serde(default)]
    pub style: TextStyle,
    #[serde(default)]
    pub align: HAlign,
    #[serde(default)]
    pub valign: VAlign,
    /// Percent of the letter size.
    #[serde(default = "line_height")]
    pub line_height: u16,
    #[serde(default = "yes")]
    pub wrap: bool,
    #[serde(default)]
    pub fit: TextFit,
    /// The map `@olcek` reads; none: the sheet's first map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub map: Option<ItemId>,
}

// ── Scale bar ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum ScaleBarStyle {
    /// Alternating filled and empty boxes in a frame: the drawing area's bar (ADR 0110).
    #[default]
    SingleBox,
    DoubleBox,
    Ticks,
    Stepped,
    Hollow,
    /// Only “1/1000”.
    Numeric,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct FixedLength {
    /// One segment on the ground, metres.
    pub metres: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum ScaleBarLength {
    /// 1, 2 or 5 times a power of ten: the longest that fits the frame (ADR 0110's rule).
    Auto(Empty),
    Fixed(FixedLength),
}

impl Default for ScaleBarLength {
    fn default() -> Self {
        ScaleBarLength::Auto(Empty {})
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum ScaleUnit {
    /// Kilometres from 1 000 m.
    #[default]
    Auto,
    M,
    Km,
}

fn four() -> u8 {
    4
}

fn bar_height() -> Um {
    1_500
}

fn scale_text() -> TextStyle {
    TextStyle::new(2_000, 500)
}

fn thin() -> Um {
    250
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ScaleBarItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub map: Option<ItemId>,
    #[serde(default)]
    pub style: ScaleBarStyle,
    /// Segments right of zero.
    #[serde(default = "four")]
    pub segments: u8,
    /// Segments left of zero (subdivided).
    #[serde(default)]
    pub left_segments: u8,
    /// Parts of a left segment.
    #[serde(default)]
    pub subdivisions: u8,
    #[serde(default)]
    pub length: ScaleBarLength,
    #[serde(default)]
    pub unit: ScaleUnit,
    /// The bar's thickness.
    #[serde(default = "bar_height")]
    pub height: Um,
    #[serde(default = "scale_text")]
    pub text: TextStyle,
    /// “Ölçek 1/1000” under the bar.
    #[serde(default)]
    pub show_scale: bool,
    #[serde(default = "black")]
    pub color: String,
    #[serde(default = "white")]
    pub secondary: String,
    #[serde(default = "thin")]
    pub line_width: Um,
}

// ── North arrow ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum NorthKind {
    /// The grid's north (the map's up).
    #[default]
    Grid,
    /// Geographic north: the grid's turned by the meridian convergence.
    True,
    /// Magnetic north: the geographic turned by the declination.
    Magnetic,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum NorthArrowStyle {
    /// Half filled, half outlined, “K” above (ADR 0110).
    #[default]
    KentosK,
    Simple,
    /// A four-pointed star with K, D, G, B.
    Compass,
    /// The Turkish topographic sheets' north diagram (design §8a): grid, true and magnetic
    /// north from one point, the angles between them written under it.
    Diagram,
}

fn north_text() -> TextStyle {
    TextStyle::new(3_500, 600)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct NorthArrowItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub map: Option<ItemId>,
    #[serde(default)]
    pub north: NorthKind,
    #[serde(default)]
    pub style: NorthArrowStyle,
    /// Magnetic declination typed by hand, east positive: used instead of the model's when
    /// [`declination_hand`](Self::declination_hand) says so.
    #[serde(default)]
    pub declination: Mdeg,
    /// The declination is the one typed by hand (written “elle”); otherwise the magnetic model's
    /// (WMM2025) at the map's centre on the sheet's date (design §8a). Absent (a book of before
    /// the model): by hand when a declination was typed, else the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub declination_hand: Option<bool>,
    /// The year a declination typed by hand is for, written in the note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub declination_year: Option<u16>,
    /// A note under the arrow: the meridian convergence (and the declination).
    #[serde(default)]
    pub note: bool,
    #[serde(default = "black")]
    pub color: String,
    #[serde(default = "north_text")]
    pub text: TextStyle,
}

// ── Legend ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum LegendFilter {
    #[default]
    All,
    VisibleInMap,
    AtlasFeature,
}

fn legend_title_gap() -> Um {
    2_000
}

fn legend_row_gap() -> Um {
    1_200
}

fn legend_column_gap() -> Um {
    5_000
}

fn legend_symbol_gap() -> Um {
    2_000
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LegendSpacing {
    #[serde(default = "legend_title_gap")]
    pub title_gap: Um,
    /// Before a group's heading.
    #[serde(default = "legend_title_gap")]
    pub group_gap: Um,
    #[serde(default = "legend_row_gap")]
    pub row_gap: Um,
    #[serde(default = "legend_column_gap")]
    pub column_gap: Um,
    /// Between a symbol and its label.
    #[serde(default = "legend_symbol_gap")]
    pub symbol_gap: Um,
}

impl Default for LegendSpacing {
    fn default() -> Self {
        LegendSpacing {
            title_gap: legend_title_gap(),
            group_gap: legend_title_gap(),
            row_gap: legend_row_gap(),
            column_gap: legend_column_gap(),
            symbol_gap: legend_symbol_gap(),
        }
    }
}

/// A layer's row as the user set it: its own label, or left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LegendEntryOverride {
    pub layer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct CustomEntries {
    /// The rows in this order; a layer not listed is left out.
    pub entries: Vec<LegendEntryOverride>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum LegendEntries {
    /// Every row the host's legend gives, in its order.
    Auto(Empty),
    Custom(CustomEntries),
}

impl Default for LegendEntries {
    fn default() -> Self {
        LegendEntries::Auto(Empty {})
    }
}

fn one() -> u8 {
    1
}

fn legend_symbol() -> SizeUm {
    SizeUm {
        width: 8_000,
        height: 4_000,
    }
}

fn legend_title() -> TextStyle {
    TextStyle::new(3_000, 600)
}

fn legend_group() -> TextStyle {
    TextStyle::new(2_200, 600)
}

fn legend_label() -> TextStyle {
    TextStyle::new(2_200, 400)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LegendItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub map: Option<ItemId>,
    #[serde(default)]
    pub filter: LegendFilter,
    #[serde(default)]
    pub title: String,
    #[serde(default = "one")]
    pub columns: u8,
    /// Labels wrap at this width; 0: at the column's.
    #[serde(default)]
    pub wrap_width: Um,
    #[serde(default = "legend_symbol")]
    pub symbol: SizeUm,
    #[serde(default)]
    pub spacing: LegendSpacing,
    #[serde(default)]
    pub entries: LegendEntries,
    #[serde(default = "legend_title")]
    pub title_style: TextStyle,
    #[serde(default = "legend_group")]
    pub group_style: TextStyle,
    #[serde(default = "legend_label")]
    pub label_style: TextStyle,
}

// ── Picture, shape, line ─────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum PictureFit {
    /// Whole, as large as fits.
    #[default]
    Contain,
    /// Fills the frame, the rest cut off.
    Cover,
    Stretch,
    /// At its own size (its dpi, else 96).
    Original,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PictureItem {
    /// The asset's SHA-256; none: an empty picture frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    #[serde(default)]
    pub fit: PictureFit,
    /// What falls outside the frame is cut off.
    #[serde(default = "yes")]
    pub clip: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RectShape {
    /// Corner radius.
    #[serde(default)]
    pub radius: Um,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PolygonShape {
    /// Corners in millionths of the frame (0: left or top edge, 1 000 000: right or bottom), so they follow the frame.
    pub points: Vec<[i32; 2]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum ShapeKind {
    Rect(RectShape),
    Ellipse(Empty),
    /// Its apex at the top centre.
    Triangle(Empty),
    Polygon(PolygonShape),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ShapeItem {
    pub shape: ShapeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stroke: Option<Stroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum LineEnd {
    #[default]
    None,
    Arrow,
    Dot,
    Bar,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LineItem {
    /// At least two, in millionths of the frame (0: left or top edge, 1 000 000: right or bottom), so the line follows its frame.
    pub points: Vec<[i32; 2]>,
    #[serde(default)]
    pub stroke: Stroke,
    #[serde(default)]
    pub start: LineEnd,
    #[serde(default)]
    pub end: LineEnd,
}

// ── Tables ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct FixedRows {
    /// Cells by column; `[% … %]` text.
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LayerRows {
    pub layer: String,
    /// An expression; empty: every object.
    #[serde(default)]
    pub filter: String,
    #[serde(default)]
    pub sort: Vec<SortKey>,
    /// Only objects inside this map's frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub only_in_map: Option<ItemId>,
    /// Only objects of the atlas page's object.
    #[serde(default)]
    pub atlas_filter: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum TableSource {
    Fixed(FixedRows),
    /// An attribute table: the host gives the layer's objects, the core filters, sorts and writes them.
    Layer(LayerRows),
    /// A frame another table continues in (its `overflow`).
    Continued(Empty),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TableColumn {
    pub heading: String,
    /// An expression over the object (layer tables).
    #[serde(default)]
    pub value: String,
    /// 0: from the content.
    #[serde(default)]
    pub width: Um,
    #[serde(default)]
    pub align: HAlign,
    /// Numbers with this many decimals (the display rule).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub decimals: Option<u8>,
}

fn cell_padding() -> Um {
    800
}

/// A cell's look.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct CellStyle {
    #[serde(default)]
    pub text: TextStyle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    #[serde(default = "cell_padding")]
    pub padding: Um,
}

impl Default for CellStyle {
    fn default() -> Self {
        CellStyle {
            text: TextStyle::default(),
            fill: None,
            padding: cell_padding(),
        }
    }
}

fn outer_rule() -> Option<Stroke> {
    Some(Stroke::solid("#000000", 350))
}

fn inner_rule() -> Option<Stroke> {
    Some(Stroke::solid("#000000", 180))
}

/// Which rules a table draws.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TableLines {
    #[serde(default = "outer_rule", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub outer: Option<Stroke>,
    /// Under the heading row.
    #[serde(default = "outer_rule", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub header: Option<Stroke>,
    #[serde(default = "inner_rule", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rows: Option<Stroke>,
    #[serde(default = "inner_rule", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub columns: Option<Stroke>,
}

impl Default for TableLines {
    fn default() -> Self {
        TableLines {
            outer: outer_rule(),
            header: outer_rule(),
            rows: inner_rule(),
            columns: inner_rule(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct EmptyMessage {
    pub text: String,
}

/// What a table with no rows shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum EmptyTable {
    ShowHeader(Empty),
    Hide(Empty),
    Message(EmptyMessage),
}

impl Default for EmptyTable {
    fn default() -> Self {
        EmptyTable::ShowHeader(Empty {})
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ContinueIn {
    /// Table items with a `continued` source, in order.
    pub items: Vec<ItemId>,
}

/// Rows that do not fit the frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum Overflow {
    /// Cut off (the preflight reports it).
    Clip(Empty),
    ContinueIn(ContinueIn),
}

impl Default for Overflow {
    fn default() -> Self {
        Overflow::Clip(Empty {})
    }
}

fn table_title() -> TextStyle {
    TextStyle::new(2_500, 600)
}

fn header_cells() -> CellStyle {
    CellStyle {
        text: TextStyle::new(2_000, 600),
        fill: Some("#eeeeee".to_owned()),
        padding: cell_padding(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TableItem {
    pub source: TableSource,
    #[serde(default)]
    pub columns: Vec<TableColumn>,
    /// A heading above the table; empty: none.
    #[serde(default)]
    pub title: String,
    #[serde(default = "table_title")]
    pub title_style: TextStyle,
    /// The heading row.
    #[serde(default = "yes")]
    pub header: bool,
    #[serde(default = "header_cells")]
    pub header_style: CellStyle,
    #[serde(default)]
    pub cell_style: CellStyle,
    #[serde(default)]
    pub lines: TableLines,
    /// Every second row's background.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zebra: Option<String>,
    #[serde(default)]
    pub empty: EmptyTable,
    #[serde(default)]
    pub overflow: Overflow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct CoordLayer {
    pub layer: String,
}

/// Where a coordinate list's points come from; the host gives them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum CoordSource {
    /// The objects selected when the list was made.
    Selection(Empty),
    Layer(CoordLayer),
}

impl Default for CoordSource {
    fn default() -> Self {
        CoordSource::Selection(Empty {})
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SequenceNaming {
    #[serde(default)]
    pub prefix: String,
    #[serde(default = "start_one")]
    pub start: u32,
}

fn start_one() -> u32 {
    1
}

/// How points are named: their own names (the host's), or numbered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum PointNaming {
    Given(Empty),
    Sequence(SequenceNaming),
}

impl Default for PointNaming {
    fn default() -> Self {
        PointNaming::Given(Empty {})
    }
}

fn two() -> u8 {
    2
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct CoordinateListItem {
    #[serde(default)]
    pub source: CoordSource,
    #[serde(default)]
    pub naming: PointNaming,
    /// A Z column.
    #[serde(default)]
    pub z: bool,
    #[serde(default = "two")]
    pub decimals: u8,
    /// The first point again under the last, for a closed figure.
    #[serde(default)]
    pub closing_row: bool,
    /// The area of the closed figure, under the list.
    #[serde(default)]
    pub area_row: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default = "table_title")]
    pub title_style: TextStyle,
    #[serde(default = "header_cells")]
    pub header_style: CellStyle,
    #[serde(default)]
    pub cell_style: CellStyle,
    #[serde(default)]
    pub lines: TableLines,
    #[serde(default)]
    pub overflow: Overflow,
}

// ── Title block ──────────────────────────────────────────────────────────

fn weight_one() -> u16 {
    1
}

/// A cell of the title block. With `rows` it is split into rows itself (a cell spanning several rows of its neighbours).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TitleCell {
    /// Width weight among the row's cells.
    #[serde(default = "weight_one")]
    pub width: u16,
    /// The small caption in the top-left corner.
    #[serde(default)]
    pub label: String,
    /// `[% … %]` text.
    #[serde(default)]
    pub value: String,
    /// The value's style; none: the block's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<TextStyle>,
    #[serde(default)]
    pub align: HAlign,
    #[serde(default = "middle")]
    pub valign: VAlign,
    /// A place to sign: the value (a name) under a signature line.
    #[serde(default)]
    pub signature: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    /// A picture (a logo) in the cell: an asset's SHA-256.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub picture: Option<String>,
    #[serde(default)]
    pub rows: Vec<TitleRow>,
}

fn middle() -> VAlign {
    VAlign::Middle
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TitleRow {
    /// Height weight among the rows.
    #[serde(default = "weight_one")]
    pub height: u16,
    pub cells: Vec<TitleCell>,
}

fn tb_outer() -> Stroke {
    Stroke::solid("#000000", 500)
}

fn tb_inner() -> Stroke {
    Stroke::solid("#000000", 180)
}

fn tb_label() -> TextStyle {
    let mut t = TextStyle::new(1_500, 500);
    t.color = "#3c3c3c".to_owned();
    t
}

fn tb_value() -> TextStyle {
    TextStyle::new(2_500, 400)
}

fn tb_padding() -> Um {
    1_000
}

/// The title block (antet): rows of cells, moved and resized as one item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TitleBlockItem {
    pub rows: Vec<TitleRow>,
    #[serde(default = "tb_outer")]
    pub outer: Stroke,
    #[serde(default = "tb_inner")]
    pub inner: Stroke,
    #[serde(default = "tb_label")]
    pub label_style: TextStyle,
    #[serde(default = "tb_value")]
    pub value_style: TextStyle,
    #[serde(default = "tb_padding")]
    pub cell_padding: Um,
}

// ── Border ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum BorderStyle {
    #[default]
    Single,
    Double,
}

fn zone_size() -> Um {
    50_000
}

fn zone_text() -> TextStyle {
    TextStyle::new(2_500, 500)
}

/// Zone marks in the band between the two lines (ISO 5457): columns 1, 2, 3 … from the left, rows A, B, C … from the top, I and O left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ZoneMarks {
    /// About this long each; an even number of them per side.
    #[serde(default = "zone_size")]
    pub size: Um,
    #[serde(default = "zone_text")]
    pub text: TextStyle,
}

fn border_outer() -> Stroke {
    Stroke::solid("#000000", 700)
}

fn border_inner() -> Stroke {
    Stroke::solid("#000000", 350)
}

fn border_gap() -> Um {
    5_000
}

/// The sheet's frame (pafta çerçevesi): one or two lines inside the item's frame; in CAD with zone and centring marks, in GIS a plain neat line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct BorderItem {
    #[serde(default)]
    pub style: BorderStyle,
    /// From the item's frame to the outer line.
    #[serde(default)]
    pub inset: Um,
    #[serde(default = "border_outer")]
    pub stroke: Stroke,
    /// The inner line of a double frame.
    #[serde(default = "border_inner")]
    pub inner_stroke: Stroke,
    /// Between the two lines (the zone band).
    #[serde(default = "border_gap")]
    pub gap: Um,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zones: Option<ZoneMarks>,
    /// A mark at the middle of each side.
    #[serde(default)]
    pub centring_marks: bool,
}

/// The whole frame in a line's or polygon's points.
pub const FRAME_ONE: i32 = 1_000_000;
pub const FRAME_HALF: i32 = 500_000;

/// A frame-relative point on the paper.
pub fn frame_point(frame: &crate::units::RectUm, p: [i32; 2]) -> [f64; 2] {
    [
        f64::from(frame.left) + f64::from(frame.width) * f64::from(p[0]) / f64::from(FRAME_ONE),
        f64::from(frame.top) + f64::from(frame.height) * f64::from(p[1]) / f64::from(FRAME_ONE),
    ]
}

/// What a kind's own text would be before it is written: the kinds a new item gets by default.
pub fn default_kind(type_name: &str) -> Option<ItemKind> {
    Some(match type_name {
        "map" => ItemKind::Map(MapItem {
            view: MapView::Fixed(FixedView {
                center: None,
                scale: 1000,
                rotation: 0,
            }),
            layers: MapLayers::All(Empty {}),
            crs: None,
            grids: Vec::new(),
            overview_of: None,
            overview_frame: overview_frame(),
            clip_to_atlas_feature: false,
            label_band: 0,
        }),
        "text" => ItemKind::Text(TextItem {
            content: String::new(),
            style: TextStyle::new(3_500, 400),
            align: HAlign::Left,
            valign: VAlign::Top,
            line_height: line_height(),
            wrap: true,
            fit: TextFit::None,
            map: None,
        }),
        "scaleBar" => ItemKind::ScaleBar(ScaleBarItem {
            map: None,
            style: ScaleBarStyle::SingleBox,
            segments: 4,
            left_segments: 0,
            subdivisions: 0,
            length: ScaleBarLength::Auto(Empty {}),
            unit: ScaleUnit::Auto,
            height: bar_height(),
            text: scale_text(),
            show_scale: false,
            color: black(),
            secondary: white(),
            line_width: thin(),
        }),
        "northArrow" => ItemKind::NorthArrow(NorthArrowItem {
            map: None,
            north: NorthKind::Grid,
            style: NorthArrowStyle::KentosK,
            declination: 0,
            declination_hand: None,
            declination_year: None,
            note: false,
            color: black(),
            text: north_text(),
        }),
        "legend" => ItemKind::Legend(LegendItem {
            map: None,
            filter: LegendFilter::All,
            title: "Lejant".to_owned(),
            columns: 1,
            wrap_width: 0,
            symbol: legend_symbol(),
            spacing: LegendSpacing::default(),
            entries: LegendEntries::Auto(Empty {}),
            title_style: legend_title(),
            group_style: legend_group(),
            label_style: legend_label(),
        }),
        "picture" => ItemKind::Picture(PictureItem {
            asset: None,
            fit: PictureFit::Contain,
            clip: true,
        }),
        "shape" => ItemKind::Shape(ShapeItem {
            shape: ShapeKind::Rect(RectShape { radius: 0 }),
            stroke: Some(Stroke::default()),
            fill: None,
        }),
        "line" => ItemKind::Line(LineItem {
            points: vec![[0, FRAME_HALF], [FRAME_ONE, FRAME_HALF]],
            stroke: Stroke::default(),
            start: LineEnd::None,
            end: LineEnd::None,
        }),
        "table" => ItemKind::Table(TableItem {
            source: TableSource::Fixed(FixedRows { rows: Vec::new() }),
            columns: Vec::new(),
            title: String::new(),
            title_style: table_title(),
            header: true,
            header_style: header_cells(),
            cell_style: CellStyle::default(),
            lines: TableLines::default(),
            zebra: None,
            empty: EmptyTable::ShowHeader(Empty {}),
            overflow: Overflow::Clip(Empty {}),
        }),
        "coordinateList" => ItemKind::CoordinateList(CoordinateListItem {
            source: CoordSource::Selection(Empty {}),
            naming: PointNaming::Given(Empty {}),
            z: false,
            decimals: 2,
            closing_row: false,
            area_row: false,
            title: String::new(),
            title_style: table_title(),
            header_style: header_cells(),
            cell_style: CellStyle::default(),
            lines: TableLines::default(),
            overflow: Overflow::Clip(Empty {}),
        }),
        "titleBlock" => ItemKind::TitleBlock(TitleBlockItem {
            rows: Vec::new(),
            outer: tb_outer(),
            inner: tb_inner(),
            label_style: tb_label(),
            value_style: tb_value(),
            cell_padding: tb_padding(),
        }),
        "border" => ItemKind::Border(BorderItem {
            style: BorderStyle::Single,
            inset: 0,
            stroke: border_outer(),
            inner_stroke: border_inner(),
            gap: border_gap(),
            zones: None,
            centring_marks: false,
        }),
        "group" => ItemKind::Group(GroupItem {}),
        _ => return None,
    })
}
