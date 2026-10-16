//! The objects of document schemas 2 to 9 read from a payload
//! (docs/specs/kcad-v2.md §6.6): each a one-key map, its kind and then its
//! fields, read into the contract's `Entity` with the persistent id the file
//! gives it; the ids are unique in a file. Schema 3 adds an object's own line
//! weight (`lineWeight`, docs/adr/0139), schema 4 the vertex elevations (`za`,
//! `zb`, `zs`, docs/adr/0142), schema 5 an area's parts (`parts`,
//! docs/adr/0143), schema 6 the `insert` kind (docs/adr/0144), schema 7 a
//! text's `align`, `widthFactor` and `mask` (docs/adr/0145), schema 8 the
//! `leader` kind (docs/adr/0146), schema 9 the dimension's new kinds, its
//! `mask` and a slope's `za`, `zb` (docs/adr/0147), schema 22 the `table`
//! kind (docs/adr/0184), schema 24 the `image` kind (docs/adr/0192) and 29
//! the `raster` kind (docs/adr/0204), the drawing's only; in an older schema they are
//! unknown fields, kinds or values. A block definition's objects are read
//! the same way, without persistent ids.

use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_contracts::{
    ArcEntity, AreaPart, BlockDefinition, BlockId, CellRange, CircleEntity, CloudFormat,
    CloudRender, CloudSource, ConstructionEntity, DimensionArrow, DimensionEntity, DimensionLook,
    DimensionRefs, DimensionStyle, DimensionTextPlace, DrawingFont, DrawingUnit, EllipseEntity,
    Entity, EntityBase, EntityId, GradientShape, HatchAssoc, HatchEntity, HatchGradient,
    HatchPattern, HatchPatternType, ImageEntity, ImageFields, InsertEntity, LeaderArrow,
    LeaderEntity, LineEntity, MAX_AFFIX, MAX_DIMENSION_DECIMALS, MAX_DIMENSION_RATIO,
    MAX_LEADER_ARROW, MAX_LINE_SPACING, MAX_LINE_WEIGHT, MAX_OBLIQUE, MAX_RASTER_BANDS,
    MAX_RASTER_SIDE, MAX_WIDTH_FACTOR, MIN_LEADER_ARROW, MIN_LINE_SPACING, Paragraph, PathEntity,
    PatternLine, PointCloudEntity, PointCloudFields, PointCloudStyle, PointEntity, PointPart,
    PointShape, PointSizeUnit, RasterEntity, RasterFields, RasterRender, RasterResampling,
    RasterSample, RasterStretch, RasterStyle, RingGeometry, SUBREF_INDEX_LIMIT, SplineEntity,
    SubPlace, SubRef, TableAlign, TableEntity, TableGrid, TableSource, TextAlign, TextEntity,
    TextFace, TextRun, TextScript, Vec2, is_hex_colour, label_scale_ok, leader_arrow_holds,
    line_weight_holds, oblique_holds, width_factor_ok,
};

use super::{PREALLOCATE, floats, id16, list, map, named, point, points, required, text, unknown};
use crate::cbor::{Reader, Seg};
use crate::error::{Code, KcadError};
use crate::watch::{EVERY, Step};
use crate::{
    SCHEMA_WITH_ADDRESSES, SCHEMA_WITH_ANNOTATION, SCHEMA_WITH_ATTACHMENTS, SCHEMA_WITH_BLOCKS,
    SCHEMA_WITH_COMPOSE, SCHEMA_WITH_CUSTOM_CRS, SCHEMA_WITH_DIMENSION_REFS,
    SCHEMA_WITH_DIMENSIONS, SCHEMA_WITH_DRAWING_UNIT, SCHEMA_WITH_ELEVATIONS, SCHEMA_WITH_FILTERS,
    SCHEMA_WITH_GROUND, SCHEMA_WITH_HATCH_PATTERNS, SCHEMA_WITH_IMAGES, SCHEMA_WITH_LABELS,
    SCHEMA_WITH_LAYER_FIELDS, SCHEMA_WITH_LAYER_SNAP, SCHEMA_WITH_LAYER_STATES,
    SCHEMA_WITH_LEADERS, SCHEMA_WITH_LINE_PARTS, SCHEMA_WITH_LINE_WEIGHTS,
    SCHEMA_WITH_LINKED_TEXTS, SCHEMA_WITH_MEASURES, SCHEMA_WITH_MULTIDIM, SCHEMA_WITH_NETWORKS,
    SCHEMA_WITH_NURBS, SCHEMA_WITH_PARAGRAPHS, SCHEMA_WITH_PARTS, SCHEMA_WITH_POINT_CLOUDS,
    SCHEMA_WITH_POLYLINE_EXTRAS, SCHEMA_WITH_RASTERS, SCHEMA_WITH_REQUESTS,
    SCHEMA_WITH_SECOND_SRID, SCHEMA_WITH_SERVICES, SCHEMA_WITH_SHEETS, SCHEMA_WITH_STYLES,
    SCHEMA_WITH_SURVEY, SCHEMA_WITH_SURVEY_SIGMAS, SCHEMA_WITH_TABLES, SCHEMA_WITH_TEMPORAL,
    SCHEMA_WITH_TEXT_EXTRAS, SCHEMA_WITH_TEXT_PATHS, SCHEMA_WITH_TOPOLOGY,
    SCHEMA_WITH_TRAVERSE_TOLERANCES, SCHEMA_WITH_VARIABLES,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Point,
    Line,
    Polyline,
    Polygon,
    Circle,
    Arc,
    Ellipse,
    Spline,
    Nurbs,
    Xline,
    Ray,
    Text,
    Dimension,
    Hatch,
    Insert,
    Leader,
    Table,
    Image,
    Raster,
    PointCloud,
}

/// The dimension's kinds in the contract's order: schema 9 added the last five.
const DIMENSION_STYLES: [DimensionStyle; 13] = DimensionStyle::ALL;

const KINDS: &[(&str, Kind)] = &[
    ("point", Kind::Point),
    ("line", Kind::Line),
    ("polyline", Kind::Polyline),
    ("polygon", Kind::Polygon),
    ("circle", Kind::Circle),
    ("arc", Kind::Arc),
    ("ellipse", Kind::Ellipse),
    ("spline", Kind::Spline),
    ("nurbs", Kind::Nurbs),
    ("xline", Kind::Xline),
    ("ray", Kind::Ray),
    ("text", Kind::Text),
    ("dimension", Kind::Dimension),
    ("hatch", Kind::Hatch),
    ("insert", Kind::Insert),
    ("leader", Kind::Leader),
    ("table", Kind::Table),
    ("image", Kind::Image),
    ("raster", Kind::Raster),
    ("pointcloud", Kind::PointCloud),
];

/// What a payload's schema lets an object hold beyond schema 2's fields.
#[derive(Clone, Copy)]
pub(super) struct Features {
    /// Schema 3 and up: an object's own line weight (`lineWeight`).
    weights: bool,
    /// Schema 4 and up: the vertex elevations (`za`, `zb`, `zs`).
    elevations: bool,
    /// Schema 5 and up: an area's parts (`parts`).
    parts: bool,
    /// Schema 6 and up: block definitions and the `insert` kind.
    pub(super) blocks: bool,
    /// Schema 7 and up: a text's alignment, width factor and mask, an
    /// attribute definition's alignment and width factor.
    pub(super) texts: bool,
    /// Schema 8 and up: the `leader` kind.
    leaders: bool,
    /// Schema 9: the dimension's new kinds, `mask`, `za` and `zb`.
    dimensions: bool,
    /// Schema 10: a layer's own snapping (`snap`).
    pub(super) layer_snap: bool,
    /// Schema 11: a local project's drawing unit (the settings' `drawingUnit`).
    pub(super) drawing_unit: bool,
    /// Schema 12: the project's second coordinate system (the settings' `secondSrid`).
    pub(super) second_srid: bool,
    /// Schema 13: the project's own systems and datum choices (the settings'
    /// `customCrs`, `secondCustomCrs`, `datumTransforms`).
    pub(super) custom_crs: bool,
    /// Schema 14: the project's survey settings (the settings' `survey`).
    pub(super) survey: bool,
    /// Schema 15: the survey settings' traverse tolerances.
    pub(super) traverse_tolerances: bool,
    /// Schema 16: the survey settings' ground height and reduction to the grid.
    pub(super) ground: bool,
    /// Schema 19: the settings' layer states (docs/adr/0177 §4).
    pub(super) layer_states: bool,
    /// Schema 17: a polyline's and a point's parts (`parts`, docs/adr/0174).
    pub(super) line_parts: bool,
    /// Schema 18: a text's link to the object whose label it writes
    /// (`labelOf`, `labelScale`, docs/adr/0175 §4).
    pub(super) linked_texts: bool,
    /// Schema 20: a multi-line text's `boxWidth`, `lineSpacing` and `runs` (docs/adr/0182).
    pub(super) paragraphs: bool,
    /// Schema 21: the settings' text and dimension styles, a text's face and a
    /// dimension's look (docs/adr/0183).
    pub(super) styles: bool,
    /// Schema 22: the `table` kind (docs/adr/0184).
    tables: bool,
    /// Schema 23: a hatch's pattern and gradient fields and its tie (docs/adr/0186).
    hatches: bool,
    /// Schema 24: the `image` kind (docs/adr/0192).
    images: bool,
    /// Schema 25: a text's curve, `path` (docs/adr/0196).
    text_paths: bool,
    /// Schema 26: a layer's fields, `fields` (docs/adr/0199 §1).
    pub(super) layer_fields: bool,
    /// Schema 27: the settings' topology rules (docs/adr/0202 §7).
    pub(super) topology: bool,
    /// Schema 28: the survey settings' a priori standard deviations (docs/adr/0203 §1).
    pub(super) survey_sigmas: bool,
    /// Schema 29: the `raster` kind (docs/adr/0204 §2).
    rasters: bool,
    /// Schema 31: the `pointcloud` kind and a raster's `url` (docs/adr/0207).
    point_clouds: bool,
    /// Schema 32: a layer node's `service` and `feed`, the settings'
    /// `connections` (docs/adr/0208 §2).
    pub(super) services: bool,
    /// Schema 33: the settings' `networks` (docs/adr/0209 §2).
    pub(super) networks: bool,
    /// Schema 34: a layer node's `time`, `scenario` and `replaces` (docs/adr/0210 §2).
    pub(super) temporal: bool,
    /// Schema 35: a layer node's `filter` (docs/adr/0211 §2).
    pub(super) filters: bool,
    /// Schema 36: the label engine's fields, a layer style's `labels`, an object's `labelPins` (docs/adr/0212 §2).
    pub(super) labels: bool,
    /// Schema 37: the settings' `variables` (docs/adr/0214 §2.3).
    pub(super) variables: bool,
    /// Schema 38: a raster's `dataset` and its look's `edges` (docs/adr/0243 §6).
    multidim: bool,
    /// Schema 39: the vertex measures (`ma`, `mb`, a polyline's and its parts' `ms`; docs/adr/0224).
    measures: bool,
    /// Schema 40: an object's `attachments`, a layer's `actions` and `card` (docs/adr/0225).
    pub(super) attachments: bool,
    /// Schema 41: a layer's `compose` (docs/adr/0226 §3, §4).
    pub(super) compose: bool,
    /// Schema 42: the settings' `addressLocators` (docs/adr/0229 §2).
    pub(super) addresses: bool,
    /// Schema 43: a dimension's `refs` (docs/adr/0249 §4).
    dimension_refs: bool,
    /// Schema 44: the document's `sheets` (docs/adr/0251).
    pub(super) sheets: bool,
    /// Schema 45: the document's `requests` (docs/adr/0253 §6).
    pub(super) requests: bool,
    /// Schema 46: a polyline's `closed` and `widths` (docs/adr/0258).
    polyline_extras: bool,
    /// Schema 47: the NURBS curve (docs/adr/0259).
    nurbs: bool,
    /// Schema 51: a reference's `along` place and block `path` (docs/adr/0263).
    subref_extras: bool,
    /// Schema 54: an object's `wipeout` (docs/adr/0310).
    wipeouts: bool,
    /// Schema 55: a layer's `frozen`, a layer state node's `frozen` and `plot` (docs/adr/0311 §1, §3).
    pub(super) frozen: bool,
    /// Schema 56: the settings' `lineTypes` and `lineTypeScale`, a layer style's and an object's `pattern`
    /// (docs/adr/0313 §1).
    pub(super) patterns: bool,
    /// Schema 57: an insert's `yScale` and a block definition's `units` (docs/adr/0315 §3, §4).
    pub(super) block_extras: bool,
    /// Schema 58: an attribute definition's options, an insert's `attributePlaces` and a block's `card`
    /// (docs/adr/0317 §1, §4).
    pub(super) block_cards: bool,
    /// Schema 59: a block definition's `dynamic` and an insert's `parameters` (docs/adr/0318).
    pub(super) dynamic_blocks: bool,
    /// Schema 60: a dimension's references' `rule` (docs/adr/0320 §1).
    dimension_rules: bool,
    /// Schema 61: a dimension's `options`, `textShift` and `breaks` (docs/adr/0321).
    pub(super) dimension_options: bool,
    /// Schema 62: a dimension's kinds `centerMark`, `centerLine`, `elevation` and the rule `pair` (docs/adr/0322).
    dimension_marks: bool,
    /// Schema 63: a text's and a dimension's `overrides` (docs/adr/0322 §5).
    overrides: bool,
    /// Schema 48: an object's `lineType`, `lineTypeScale`, `transparency`,
    /// `order` and a block's object's `byLayer` (docs/adr/0260).
    pub(super) look: bool,
    /// Schema 49: the settings' `ucs` and `namedUcs` (docs/adr/0261).
    pub(super) ucs: bool,
    /// Schema 52: the settings' `namedViews` (docs/adr/0285).
    pub(super) views: bool,
    /// Schema 53: the document's `arrays` (docs/adr/0300).
    pub(super) arrays: bool,
    /// Schema 50: the drawing units `in` and `ft` and the settings' `lengthStyle` (docs/adr/0262).
    pub(super) imperial: bool,
    /// Schema 30: the settings' annotation heights, a dimension's and a
    /// dimension style's line fields, a leader's `arrowSize` and AutoCAD's
    /// arrowheads (docs/adr/0205).
    pub(super) annotation: bool,
    /// Whether an object has its persistent id (`uid`): the drawing's do, a
    /// block definition's do not.
    uids: bool,
}

impl Features {
    pub(super) fn of(schema: u32) -> Self {
        Self {
            weights: schema >= SCHEMA_WITH_LINE_WEIGHTS,
            elevations: schema >= SCHEMA_WITH_ELEVATIONS,
            parts: schema >= SCHEMA_WITH_PARTS,
            blocks: schema >= SCHEMA_WITH_BLOCKS,
            texts: schema >= SCHEMA_WITH_TEXT_EXTRAS,
            leaders: schema >= SCHEMA_WITH_LEADERS,
            dimensions: schema >= SCHEMA_WITH_DIMENSIONS,
            layer_snap: schema >= SCHEMA_WITH_LAYER_SNAP,
            drawing_unit: schema >= SCHEMA_WITH_DRAWING_UNIT,
            second_srid: schema >= SCHEMA_WITH_SECOND_SRID,
            custom_crs: schema >= SCHEMA_WITH_CUSTOM_CRS,
            survey: schema >= SCHEMA_WITH_SURVEY,
            traverse_tolerances: schema >= SCHEMA_WITH_TRAVERSE_TOLERANCES,
            ground: schema >= SCHEMA_WITH_GROUND,
            layer_states: schema >= SCHEMA_WITH_LAYER_STATES,
            line_parts: schema >= SCHEMA_WITH_LINE_PARTS,
            linked_texts: schema >= SCHEMA_WITH_LINKED_TEXTS,
            paragraphs: schema >= SCHEMA_WITH_PARAGRAPHS,
            styles: schema >= SCHEMA_WITH_STYLES,
            tables: schema >= SCHEMA_WITH_TABLES,
            hatches: schema >= SCHEMA_WITH_HATCH_PATTERNS,
            images: schema >= SCHEMA_WITH_IMAGES,
            text_paths: schema >= SCHEMA_WITH_TEXT_PATHS,
            layer_fields: schema >= SCHEMA_WITH_LAYER_FIELDS,
            topology: schema >= SCHEMA_WITH_TOPOLOGY,
            survey_sigmas: schema >= SCHEMA_WITH_SURVEY_SIGMAS,
            rasters: schema >= SCHEMA_WITH_RASTERS,
            point_clouds: schema >= SCHEMA_WITH_POINT_CLOUDS,
            services: schema >= SCHEMA_WITH_SERVICES,
            networks: schema >= SCHEMA_WITH_NETWORKS,
            temporal: schema >= SCHEMA_WITH_TEMPORAL,
            filters: schema >= SCHEMA_WITH_FILTERS,
            multidim: schema >= SCHEMA_WITH_MULTIDIM,
            measures: schema >= SCHEMA_WITH_MEASURES,
            attachments: schema >= SCHEMA_WITH_ATTACHMENTS,
            compose: schema >= SCHEMA_WITH_COMPOSE,
            addresses: schema >= SCHEMA_WITH_ADDRESSES,
            dimension_refs: schema >= SCHEMA_WITH_DIMENSION_REFS,
            sheets: schema >= SCHEMA_WITH_SHEETS,
            requests: schema >= SCHEMA_WITH_REQUESTS,
            polyline_extras: schema >= SCHEMA_WITH_POLYLINE_EXTRAS,
            look: schema >= crate::SCHEMA_WITH_LOOK,
            ucs: schema >= crate::SCHEMA_WITH_UCS,
            views: schema >= crate::SCHEMA_WITH_VIEWS,
            arrays: schema >= crate::SCHEMA_WITH_ARRAYS,
            wipeouts: schema >= crate::SCHEMA_WITH_WIPEOUTS,
            frozen: schema >= crate::SCHEMA_WITH_FROZEN,
            patterns: schema >= crate::SCHEMA_WITH_PATTERNS,
            block_extras: schema >= crate::SCHEMA_WITH_BLOCK_EXTRAS,
            block_cards: schema >= crate::SCHEMA_WITH_BLOCK_CARDS,
            dynamic_blocks: schema >= crate::SCHEMA_WITH_DYNAMIC_BLOCKS,
            dimension_rules: schema >= crate::SCHEMA_WITH_DIMENSION_RULES,
            dimension_options: schema >= crate::SCHEMA_WITH_DIMENSION_OPTIONS,
            dimension_marks: schema >= crate::SCHEMA_WITH_DIMENSION_MARKS,
            overrides: schema >= crate::SCHEMA_WITH_OVERRIDES,
            imperial: schema >= crate::SCHEMA_WITH_IMPERIAL,
            subref_extras: schema >= crate::SCHEMA_WITH_SUBREF_EXTRAS,
            nurbs: schema >= SCHEMA_WITH_NURBS,
            labels: schema >= SCHEMA_WITH_LABELS,
            variables: schema >= SCHEMA_WITH_VARIABLES,
            annotation: schema >= SCHEMA_WITH_ANNOTATION,
            uids: true,
        }
    }

    /// The same for a block definition's objects.
    pub(super) fn without_uids(self) -> Self {
        Self {
            uids: false,
            ..self
        }
    }
}

/// Whether a kind's map may hold `key` (the fields every kind has, then its
/// own, then what the payload's schema adds).
fn allowed(kind: Kind, key: &str, has: Features) -> bool {
    matches!(key, "attrs" | "color" | "label" | "symbol" | "layerId")
        || (has.uids && key == "uid")
        // A label pinned by hand: the drawing's objects only (a definition's have no persistent id).
        || (has.labels && has.uids && key == "labelPins")
        // Photos and documents: the drawing's objects only (a definition's are not picked one by one).
        || (has.attachments && has.uids && key == "attachments")
        || (has.weights && key == "lineWeight")
        // An object's look (docs/adr/0260): its order the drawing's objects', `byLayer` a definition's.
        || (has.look && matches!(key, "lineType" | "lineTypeScale" | "transparency"))
        || (has.look && has.uids && key == "order")
        || (has.look && !has.uids && key == "byLayer")
        || (has.wipeouts && key == "wipeout")
        || (has.patterns && key == "linePattern")
        || match kind {
            Kind::Point => matches!(key, "p" | "z") || (has.line_parts && key == "parts"),
            Kind::Line => {
                matches!(key, "a" | "b")
                    || (has.elevations && matches!(key, "za" | "zb"))
                    || (has.measures && matches!(key, "ma" | "mb"))
            }
            Kind::Polyline => {
                matches!(key, "pts" | "bulges")
                    || (has.elevations && key == "zs")
                    || (has.line_parts && key == "parts")
                    || (has.measures && key == "ms")
                    || (has.polyline_extras && matches!(key, "closed" | "widths"))
            }
            Kind::Polygon => {
                matches!(key, "pts" | "bulges" | "holes")
                    || (has.elevations && key == "zs")
                    || (has.parts && key == "parts")
            }
            Kind::Circle => matches!(key, "c" | "r"),
            Kind::Arc => matches!(key, "c" | "r" | "a0" | "a1"),
            Kind::Ellipse => matches!(key, "c" | "major" | "ratio" | "t0" | "t1"),
            Kind::Spline => matches!(key, "pts" | "closed"),
            Kind::Nurbs => matches!(
                key,
                "degree" | "ctrl" | "knots" | "weights" | "closed" | "periodic" | "fit" | "tangents" | "fitTolerance"
            ),
            Kind::Xline | Kind::Ray => matches!(key, "p" | "dir"),
            Kind::Text => {
                matches!(key, "p" | "text" | "height" | "rotation")
                    || (has.texts && matches!(key, "align" | "widthFactor" | "mask"))
                    || (has.linked_texts && has.uids && matches!(key, "labelOf" | "labelScale"))
                    || (has.paragraphs && matches!(key, "boxWidth" | "lineSpacing" | "runs"))
                    || (has.styles
                        && matches!(key, "textStyle" | "font" | "bold" | "italic" | "oblique"))
                    || (has.text_paths && key == "path")
                    || (has.overrides && key == "overrides")
            }
            Kind::Dimension => {
                matches!(
                    key,
                    "a" | "b" | "c" | "text" | "angle" | "style" | "height" | "offset"
                ) || (has.dimensions && matches!(key, "mask" | "za" | "zb"))
                    || (has.styles
                        && matches!(
                            key,
                            "dimStyle"
                                | "arrow"
                                | "arrowSize"
                                | "extOffset"
                                | "extBeyond"
                                | "textGap"
                                | "textPlace"
                                | "decimals"
                                | "unit"
                                | "prefix"
                                | "suffix"
                                | "font"
                        ))
                    || (has.annotation
                        && matches!(
                            key,
                            "dimLineColor"
                                | "dimLineWeight"
                                | "dimLineType"
                                | "extColor"
                                | "extWeight"
                                | "extLineType"
                                | "textColor"
                        ))
                    // What its points follow names objects: only the drawing's dimensions (docs/adr/0249 §4).
                    || (has.dimension_refs && has.uids && key == "refs")
                    || (has.dimension_options && matches!(key, "options" | "textShift" | "breaks"))
                    || (has.overrides && key == "overrides")
            }
            // A hatch's tie names objects: only the drawing's hatches (docs/adr/0186 §6).
            Kind::Hatch => {
                matches!(key, "ring" | "holes" | "pattern")
                    || (has.hatches && has.uids && key == "assoc")
            }
            Kind::Insert => {
                matches!(key, "block" | "p" | "scale" | "rotation" | "mirror")
                    || (has.block_extras && key == "yScale")
                    || (has.block_cards && key == "attributePlaces")
                    || (has.dynamic_blocks && key == "parameters")
            }
            Kind::Leader => {
                matches!(
                    key,
                    "pts" | "text" | "height" | "rotation" | "arrow" | "mask"
                ) || (has.annotation && key == "arrowSize")
            }
            Kind::Table => matches!(
                key,
                "p" | "rotation"
                    | "height"
                    | "rows"
                    | "columns"
                    | "cells"
                    | "merges"
                    | "aligns"
                    | "header"
                    | "grid"
                    | "frame"
                    | "source"
                    | "textStyle"
                    | "font"
                    | "bold"
                    | "italic"
                    | "oblique"
            ),
            Kind::Image => matches!(
                key,
                "p" | "width"
                    | "height"
                    | "rotation"
                    | "mirror"
                    | "asset"
                    | "file"
                    | "clip"
                    | "opacity"
            ),
            Kind::Raster => {
                matches!(
                    key,
                    "affine"
                        | "width"
                        | "height"
                        | "bands"
                        | "sample"
                        | "asset"
                        | "file"
                        | "srid"
                        | "style"
                        | "opacity"
                ) || (has.point_clouds && key == "url")
                    || (has.multidim && key == "dataset")
            }
            Kind::PointCloud => matches!(
                key,
                "sources" | "bounds" | "count" | "srid" | "style" | "opacity"
            ),
        }
}

/// Every field an object may have; each kind takes its own.
/// A raster's own fields (docs/adr/0204 §2).
#[derive(Default)]
struct RasterRead {
    affine: Option<[f64; 6]>,
    width: Option<u32>,
    height: Option<u32>,
    bands: Option<u32>,
    sample: Option<RasterSample>,
    srid: Option<u32>,
    style: Option<RasterStyle>,
    dataset: Option<kentos_contracts::RasterDataset>,
}

/// A point cloud's own fields (docs/adr/0207 §3).
#[derive(Default)]
struct CloudRead {
    sources: Option<Vec<CloudSource>>,
    bounds: Option<[f64; 6]>,
    count: Option<u64>,
    style: Option<PointCloudStyle>,
}

#[derive(Default)]
struct Fields {
    raster: RasterRead,
    cloud: CloudRead,
    url: Option<String>,
    uid: Option<EntityId>,
    attrs: Option<BTreeMap<String, String>>,
    color: Option<String>,
    label: Option<String>,
    symbol: Option<String>,
    layer_id: Option<String>,
    line_weight: Option<f64>,
    /// An object's look (docs/adr/0260).
    object_look: kentos_contracts::ObjectLook,
    label_pins: Option<Vec<kentos_contracts::LabelPin>>,
    attachments: Option<Vec<kentos_contracts::Attachment>>,
    /// A NURBS curve's numbers (docs/adr/0259).
    degree: Option<u32>,
    ctrl: Option<Vec<Vec2>>,
    knots: Option<Vec<f64>>,
    weights: Option<Vec<f64>>,
    periodic: bool,
    fit: Option<Vec<Vec2>>,
    tangents: Option<[Option<Vec2>; 2]>,
    fit_tolerance: Option<f64>,
    p: Option<Vec2>,
    a: Option<Vec2>,
    b: Option<Vec2>,
    c: Option<Vec2>,
    major: Option<Vec2>,
    dir: Option<Vec2>,
    z: Option<f64>,
    za: Option<f64>,
    zb: Option<f64>,
    ma: Option<f64>,
    mb: Option<f64>,
    /// Where a dimension's `angle`, `za` and `zb` are (for a refusal).
    angle_at: usize,
    za_at: usize,
    zb_at: usize,
    r: Option<f64>,
    a0: Option<f64>,
    a1: Option<f64>,
    ratio: Option<f64>,
    t0: Option<f64>,
    t1: Option<f64>,
    height: Option<f64>,
    rotation: Option<f64>,
    offset: Option<f64>,
    angle: Option<f64>,
    pts: Option<Vec<Vec2>>,
    ring: Option<Vec<Vec2>>,
    bulges: Option<Vec<f64>>,
    /// A path's elevations, and where in the payload they start (for a length error).
    zs: Option<Vec<Option<f64>>>,
    zs_at: usize,
    /// A polyline's measures, alike (docs/adr/0224).
    ms: Option<Vec<Option<f64>>>,
    ms_at: usize,
    rings: Option<Vec<RingGeometry>>,
    parts: Option<Vec<AreaPart>>,
    /// A multi-point object's points past its first (schema 17).
    point_parts: Option<Vec<PointPart>>,
    loops: Option<Vec<Vec<Vec2>>>,
    closed: Option<bool>,
    /// A polyline's widths (docs/adr/0258 §2).
    widths: Option<Vec<[f64; 2]>>,
    text: Option<String>,
    style: Option<DimensionStyle>,
    pattern: Option<HatchPattern>,
    assoc: Option<HatchAssoc>,
    /// A dimension's references (docs/adr/0249 §4).
    refs: Option<DimensionRefs>,
    /// A dimension's moved value and breaks (docs/adr/0321).
    text_shift: Option<kentos_contracts::TextShift>,
    breaks: Vec<kentos_contracts::DimensionBreak>,
    block: Option<BlockId>,
    /// Where the insert's `block` is in the payload (for an unknown block).
    block_at: usize,
    scale: Option<f64>,
    /// An insert's own y factor and where it is (docs/adr/0315 §3).
    y_scale: Option<(f64, usize)>,
    /// Where an insert shows its movable attributes (docs/adr/0317 §1).
    attribute_places: Option<BTreeMap<String, Vec2>>,
    /// An insert's parameter values (docs/adr/0318 §3).
    parameters: Option<BTreeMap<String, kentos_contracts::ParamValue>>,
    mirror: Option<bool>,
    align: Option<TextAlign>,
    arrow: Option<LeaderArrow>,
    /// A leader's arrowhead size (schema 30).
    arrow_size: Option<f64>,
    width_factor: Option<f64>,
    mask: Option<bool>,
    /// A picture's own fields (docs/adr/0192 §1), and where the object's map starts (for a refusal).
    width: Option<f64>,
    asset: Option<String>,
    file: Option<String>,
    clip: Option<Vec<Vec2>>,
    /// A text's curve and where it is (for a refusal; docs/adr/0196).
    text_path: Option<kentos_contracts::TextPath>,
    path_at: usize,
    opacity: Option<f64>,
    /// A linked text's object and scale (docs/adr/0175 §4), and where the first of them is.
    label_of: Option<EntityId>,
    label_scale: Option<f64>,
    link_at: usize,
    /// A multi-line text's fields (docs/adr/0182), and where its runs start (for a refusal).
    paragraph: Paragraph,
    /// A text's face and a dimension's look (docs/adr/0183), and where the face's first field is.
    face: TextFace,
    face_at: usize,
    look: DimensionLook,
    runs_at: usize,
    /// A table's own fields (docs/adr/0184), and where each is (for a refusal).
    rows: Option<Vec<f64>>,
    columns: Option<Vec<f64>>,
    cells: Option<Vec<Vec<String>>>,
    merges: Option<Vec<CellRange>>,
    aligns: Option<Vec<TableAlign>>,
    header: Option<bool>,
    grid: Option<TableGrid>,
    frame: Option<f64>,
    source: Option<TableSource>,
    table_at: TablePlaces,
}

/// Where a table's fields are in the payload, for a refusal of its shape.
#[derive(Default)]
struct TablePlaces {
    height: usize,
    rows: usize,
    columns: usize,
    cells: usize,
    merges: usize,
    aligns: usize,
    frame: usize,
    source: usize,
}

impl TablePlaces {
    fn of(&self, field: &str) -> usize {
        match field {
            "height" => self.height,
            "rows" => self.rows,
            "columns" => self.columns,
            "cells" => self.cells,
            "merges" => self.merges,
            "aligns" => self.aligns,
            "frame" => self.frame,
            _ => self.source,
        }
    }
}

/// The objects and their persistent ids, each id once (§6.8); each insert
/// naming one of `blocks` (`index` their ids' places). The project (`name`,
/// `layers`, the number of objects) is reported first, then the objects
/// every few thousand.
pub(super) fn objects(
    r: &mut Reader<'_>,
    name: Option<&str>,
    layers: usize,
    has: Features,
    blocks: &[BlockDefinition],
    index: &HashMap<BlockId, usize>,
) -> Result<(Vec<Entity>, Vec<EntityId>), KcadError> {
    let n = r.array()?;
    r.report(Step::Project {
        name: name.unwrap_or_default(),
        layers,
        objects: n,
    })?;
    let mut seen = HashSet::with_capacity(n.min(PREALLOCATE));
    let mut uids = Vec::with_capacity(n.min(PREALLOCATE));
    // As `list`: `array` checked that n items fit the bytes left; the rest grows as they are read.
    let mut entities = Vec::with_capacity(n.min(PREALLOCATE));
    for i in 0..n {
        if i % EVERY == 0 {
            r.report(Step::Reading { done: i, total: n })?;
        }
        r.push(Seg::Index(i));
        let (entity, uid, block_at) = object(r, i, has)?;
        if let Entity::Insert(insert) = &entity
            && !index.contains_key(&insert.block)
        {
            r.pop();
            return Err(super::blocks::unknown_block(r, i, block_at, blocks));
        }
        // A moved attribute only of a movable tag (docs/adr/0317 §1).
        if let Entity::Insert(insert) = &entity
            && let Err(fault) =
                kentos_contracts::blocks::places(blocks, std::slice::from_ref(&entity), None, index)
        {
            let tag = match &fault {
                kentos_contracts::blocks::BlockFault::BadPlace { tag, .. } => tag.clone(),
                _ => String::new(),
            };
            let _ = insert;
            r.push(Seg::Name("insert"));
            r.push(Seg::Name("attributePlaces"));
            return Err(r.fail(
                Code::BadValue,
                &format!("“{tag}” özniteliği blokta taşınabilir değil; yeri yazılmaz"),
            ));
        }
        // `has.uids`: `object` gave the id or refused the object.
        let Some(uid) = uid else {
            return Err(r.fail(Code::MissingField, "zorunlu alan yok (uid)"));
        };
        if !seen.insert(uid) {
            r.push(Seg::Name("uid"));
            let e = r.fail(
                Code::DuplicateUid,
                &format!("kalıcı kimlik {uid} iki nesnede var"),
            );
            r.pop();
            return Err(e);
        }
        uids.push(uid);
        entities.push(entity);
        r.pop();
    }
    r.leave();
    r.report(Step::Reading { done: n, total: n })?;
    Ok((entities, uids))
}

// Out of line on purpose: a browser's WebAssembly engine runs a function in its
// baseline code until the function is called again (no on-stack replacement), so the
// work of each object must be a function called once per object, not inlined into the
// one loop that runs once for the whole drawing (docs/adr/0030).
//
// The object, its persistent id (`None` when `has` says objects have none)
// and, for an insert, where its `block` is.
#[inline(never)]
pub(super) fn object(
    r: &mut Reader<'_>,
    index: usize,
    has: Features,
) -> Result<(Entity, Option<EntityId>, usize), KcadError> {
    let (n, at) = r.map()?;
    if n != 1 {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("nesne haritasında tek anahtar (tür) olmalı, {n} var"),
        ));
    }
    let mut previous = None;
    let name = r.key(&mut previous)?;
    r.push(Seg::Key(name));
    let Some(&(_, kind)) = KINDS.iter().find(|(k, _)| *k == name).filter(|(_, kind)| {
        (has.blocks || *kind != Kind::Insert)
            && (has.leaders || *kind != Kind::Leader)
            && (has.tables || *kind != Kind::Table)
            && (has.images || *kind != Kind::Image)
            && (has.rasters || *kind != Kind::Raster)
            && (has.point_clouds || *kind != Kind::PointCloud)
            && (has.nurbs || *kind != Kind::Nurbs)
    }) else {
        return Err(r.fail(
            Code::UnknownKind,
            &format!(
                "“{name}” nesne türü bilinmiyor; dosya daha yeni bir KentOS'la yazılmış olabilir"
            ),
        ));
    };
    // Only the drawing's: a block definition holds no table (docs/adr/0184 §1).
    if kind == Kind::Table && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında tablo olamaz"));
    }
    // Only the drawing's: a block definition holds no picture (docs/adr/0192 §1).
    if kind == Kind::Image && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında resim olamaz"));
    }
    // Nor a raster (docs/adr/0204 §2).
    if kind == Kind::Raster && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında raster olamaz"));
    }
    // Nor a point cloud (docs/adr/0207 §3).
    if kind == Kind::PointCloud && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında nokta bulutu olamaz"));
    }
    let mut f = Fields::default();
    map(r, |r, key| {
        if !allowed(kind, key, has) {
            return Err(unknown(r));
        }
        match key {
            "uid" => f.uid = Some(EntityId(id16(r)?)),
            "attrs" => {
                let mut attrs = BTreeMap::new();
                map(r, |r, k| {
                    attrs.insert(k.to_owned(), text(r)?);
                    Ok(())
                })?;
                f.attrs = Some(attrs);
            }
            "color" => f.color = Some(text(r)?),
            "label" => f.label = Some(text(r)?),
            "labelPins" => f.label_pins = Some(super::labels::label_pins(r)?),
            "attachments" => f.attachments = Some(super::info::attachments(r)?),
            "symbol" => f.symbol = Some(text(r)?),
            "layerId" => f.layer_id = Some(text(r)?),
            "lineWeight" => {
                let w = r.float()?;
                if !(0.0..=MAX_LINE_WEIGHT).contains(&w) {
                    return Err(r.fail(
                        Code::BadValue,
                        &format!("çizgi kalınlığı {w} mm; 0 ile {MAX_LINE_WEIGHT} arasında olmalı"),
                    ));
                }
                f.line_weight = Some(w);
            }
            "lineType" => {
                f.object_look.line_type = Some(super::named(
                    r,
                    &kentos_contracts::LineType::ALL.map(|t| (t.name(), t)),
                )?)
            }
            "lineTypeScale" => f.object_look.line_type_scale = Some(r.float()?),
            "linePattern" => f.object_look.line_pattern = Some(text(r)?),
            "transparency" => f.object_look.transparency = Some(r.float()?),
            "order" => f.object_look.order = Some(r.int(crate::MAX_ORDER)? as i32),
            "wipeout" => f.object_look.wipeout = Some(wipeout(r)?),
            "byLayer" => {
                f.object_look.by_layer = list(r, |r, _| {
                    super::named(r, &kentos_contracts::LookPart::ALL.map(|p| (p.name(), p)))
                })?
            }
            "p" => f.p = Some(point(r)?),
            "a" => f.a = Some(point(r)?),
            "b" => f.b = Some(point(r)?),
            "c" => f.c = Some(point(r)?),
            "major" => f.major = Some(point(r)?),
            "dir" => f.dir = Some(point(r)?),
            "z" => f.z = Some(r.float()?),
            "za" => {
                f.za_at = r.position();
                f.za = Some(r.float()?);
            }
            "zb" => {
                f.zb_at = r.position();
                f.zb = Some(r.float()?);
            }
            "ma" => f.ma = Some(r.float()?),
            "mb" => f.mb = Some(r.float()?),
            "r" => f.r = Some(r.float()?),
            "a0" => f.a0 = Some(r.float()?),
            "a1" => f.a1 = Some(r.float()?),
            "ratio" => f.ratio = Some(r.float()?),
            "t0" => f.t0 = Some(r.float()?),
            "t1" => f.t1 = Some(r.float()?),
            "width" if kind == Kind::Raster => {
                f.raster.width = Some(r.uint(u64::from(MAX_RASTER_SIDE))? as u32)
            }
            "height" if kind == Kind::Raster => {
                f.raster.height = Some(r.uint(u64::from(MAX_RASTER_SIDE))? as u32)
            }
            "height" if kind == Kind::Leader => {
                let at = r.position();
                let h = r.float()?;
                if h <= 0.0 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("kılavuzun yüksekliği {h}; 0'dan büyük olmalı"),
                    ));
                }
                f.height = Some(h);
            }
            "height" => {
                f.table_at.height = r.position();
                f.height = Some(r.float()?);
            }
            "rotation" => f.rotation = Some(r.float()?),
            "bands" => f.raster.bands = Some(r.uint(u64::from(MAX_RASTER_BANDS))? as u32),
            "sample" => {
                let names: Vec<(&str, RasterSample)> =
                    RasterSample::ALL.iter().map(|s| (s.name(), *s)).collect();
                f.raster.sample = Some(named(r, &names)?);
            }
            "srid" => f.raster.srid = Some(r.uint(u64::from(u32::MAX))? as u32),
            "affine" => {
                let at = r.position();
                let v = floats(r)?;
                let affine: [f64; 6] = v.as_slice().try_into().map_err(|_| {
                    r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("rasterin dönüşümü 6 sayı olmalı, {} var", v.len()),
                    )
                })?;
                f.raster.affine = Some(affine);
            }
            "style" if kind == Kind::Raster => {
                f.raster.style = Some(raster_style(r, has.multidim)?)
            }
            "dataset" => f.raster.dataset = Some(raster_dataset(r)?),
            "style" if kind == Kind::PointCloud => f.cloud.style = Some(cloud_style(r)?),
            "sources" => f.cloud.sources = Some(cloud_sources(r)?),
            "bounds" => f.cloud.bounds = Some(bounds6(r)?),
            "count" => f.cloud.count = Some(r.uint(u64::MAX)?),
            "url" => f.url = Some(text(r)?),
            "width" => f.width = Some(r.float()?),
            "asset" => f.asset = Some(text(r)?),
            "file" => f.file = Some(text(r)?),
            "clip" => f.clip = Some(points(r)?),
            "path" => {
                f.path_at = r.position();
                f.text_path = Some(text_path(r)?);
            }
            "opacity" => f.opacity = Some(r.float()?),
            "rows" => {
                f.table_at.rows = r.position();
                f.rows = Some(floats(r)?);
            }
            "columns" => {
                f.table_at.columns = r.position();
                f.columns = Some(floats(r)?);
            }
            "cells" => {
                f.table_at.cells = r.position();
                f.cells = Some(list(r, |r, _| list(r, |r, _| text(r)))?);
            }
            "merges" => {
                f.table_at.merges = r.position();
                let ranges = list(r, |r, _| cell_range(r))?;
                // The writer leaves an empty list out: one spelling (§6.6).
                if ranges.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        f.table_at.merges,
                        "birleşik alan listesi boş; birleşik alanı olmayan tabloda alan yazılmaz",
                    ));
                }
                f.merges = Some(ranges);
            }
            "aligns" => {
                f.table_at.aligns = r.position();
                let names: Vec<(&str, TableAlign)> =
                    TableAlign::ALL.iter().map(|a| (a.name(), *a)).collect();
                f.aligns = Some(list(r, |r, _| named(r, &names))?);
            }
            "header" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "başlık false yazılmaz; başlıksız tabloda alan yoktur",
                    ));
                }
                f.header = Some(true);
            }
            "grid" => {
                let names: Vec<(&str, TableGrid)> =
                    TableGrid::ALL.iter().map(|g| (g.name(), *g)).collect();
                f.grid = Some(named(r, &names)?);
            }
            "frame" => {
                f.table_at.frame = r.position();
                f.frame = Some(r.float()?);
            }
            "source" => {
                f.table_at.source = r.position();
                f.source = Some(table_source(r)?);
            }
            "offset" => f.offset = Some(r.float()?),
            "angle" => {
                f.angle_at = r.position();
                f.angle = Some(r.float()?);
            }
            "pts" if kind == Kind::Leader => {
                let at = r.position();
                let pts = points(r)?;
                if pts.len() < 2 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("kılavuzun {} köşesi var; en az iki olmalı", pts.len()),
                    ));
                }
                f.pts = Some(pts);
            }
            "pts" => f.pts = Some(points(r)?),
            "ring" => f.ring = Some(points(r)?),
            "bulges" => f.bulges = Some(floats(r)?),
            "zs" => {
                f.zs_at = r.position();
                f.zs = Some(elevations(r)?);
            }
            "ms" => {
                f.ms_at = r.position();
                f.ms = Some(elevations(r)?);
            }
            "holes" if kind == Kind::Polygon => f.rings = Some(list(r, |r, _| ring(r, has))?),
            "holes" => f.loops = Some(list(r, |r, _| points(r))?),
            "parts" if kind == Kind::Point => f.point_parts = Some(list(r, |r, _| point_part(r))?),
            "parts" if kind == Kind::Polyline => f.parts = Some(list(r, |r, _| line_part(r, has))?),
            "parts" => f.parts = Some(list(r, |r, _| part(r, has))?),
            // A closed polyline (docs/adr/0258 §1): an open one has no field.
            "closed" if kind == Kind::Polyline => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "kapalılık false yazılmaz; açık çoklu çizgide alan yoktur",
                    ));
                }
                f.closed = Some(true);
            }
            // A NURBS curve's words are written only when true, as a closed polyline's (docs/adr/0259).
            "closed" | "periodic" if kind == Kind::Nurbs => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("{key} false yazılmaz; alan yalnız true iken yazılır"),
                    ));
                }
                if key == "closed" {
                    f.closed = Some(true);
                } else {
                    f.periodic = true;
                }
            }
            "degree" => {
                f.degree = Some(r.uint(u64::from(kentos_contracts::MAX_NURBS_DEGREE))? as u32)
            }
            "ctrl" => f.ctrl = Some(points(r)?),
            "knots" => f.knots = Some(floats(r)?),
            "weights" => f.weights = Some(floats(r)?),
            "fit" => f.fit = Some(points(r)?),
            "fitTolerance" => f.fit_tolerance = Some(r.float()?),
            "tangents" => {
                let at = r.position();
                let n = r.array()?;
                if n != 2 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("teğetler 2 öğe olmalı (başlangıç, bitiş), {n} var"),
                    ));
                }
                let mut pair = [None, None];
                for t in &mut pair {
                    *t = if r.next_is_null() {
                        r.float_or_null()?;
                        None
                    } else {
                        Some(point(r)?)
                    };
                }
                r.leave();
                f.tangents = Some(pair);
            }
            "closed" => f.closed = Some(r.bool()?),
            "widths" => f.widths = Some(list(r, |r, _| width(r))?),
            "text" if kind == Kind::Leader => {
                let at = r.position();
                let note = text(r)?;
                if note.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "kılavuzun notu boş; notsuz kılavuzun not alanı yazılmaz",
                    ));
                }
                f.text = Some(note);
            }
            "text" => f.text = Some(text(r)?),
            "style" => {
                // Schema 9's kinds after the first five (docs/adr/0147).
                let styles = DIMENSION_STYLES.map(|s| (s.name(), s));
                // Schema 62's three after them (docs/adr/0322).
                let known = if has.dimension_marks {
                    &styles[..]
                } else if has.dimensions {
                    &styles[..10]
                } else {
                    &styles[..5]
                };
                f.style = Some(named(r, known)?)
            }
            "pattern" => f.pattern = Some(pattern(r, has)?),
            "assoc" => f.assoc = Some(hatch_assoc(r)?),
            "refs" => {
                f.refs = Some(dimension_refs(
                    r,
                    has.subref_extras,
                    has.dimension_rules,
                    has.dimension_marks,
                )?)
            }
            // Schema 63 (docs/adr/0322 §5): the fields set apart from the style, by name, as their JSON.
            "overrides" => {
                let (o, at): (Vec<String>, usize) = json_part(r, "geçersiz kılınanlar")?;
                if o.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş geçersiz kılma listesi yazılmaz",
                    ));
                }
                let problem = if kind == Kind::Text {
                    TextFace {
                        overrides: o.clone(),
                        font: Some(kentos_contracts::DrawingFont::Barlow),
                        ..TextFace::default()
                    }
                    .problem()
                } else {
                    DimensionLook {
                        overrides: o.clone(),
                        ..DimensionLook::default()
                    }
                    .problem()
                };
                if let Some((_, words)) = problem {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.face.overrides.clone_from(&o);
                f.look.overrides = o;
            }
            // Schema 61 (docs/adr/0321): as their JSON, as the writer writes them.
            "options" => {
                let (o, at): (kentos_contracts::DimensionOptions, usize) =
                    json_part(r, "seçenekler")?;
                if let Some(words) = o.problem() {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.look.options = Some(Box::new(o));
            }
            "textShift" => {
                let (s, at): (kentos_contracts::TextShift, usize) =
                    json_part(r, "değerin kayması")?;
                if let Some(words) = s.problem() {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.text_shift = Some(s);
            }
            "breaks" => {
                let (b, at): (Vec<kentos_contracts::DimensionBreak>, usize) =
                    json_part(r, "kırmalar")?;
                if b.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş kırma listesi yazılmaz"));
                }
                if let Some(words) = kentos_contracts::breaks_problem(&b) {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.breaks = b;
            }
            "block" => {
                f.block_at = r.position();
                f.block = Some(BlockId(id16(r)?));
            }
            "scale" => {
                let at = r.position();
                let scale = r.float()?;
                if !kentos_contracts::blocks::scale_ok(scale) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("blok ölçeği {scale}; pozitif olmalı"),
                    ));
                }
                f.scale = Some(scale);
            }
            "yScale" => {
                let at = r.position();
                let y = r.float()?;
                if !kentos_contracts::blocks::scale_ok(y) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("blok y ölçeği {y}; pozitif olmalı"),
                    ));
                }
                f.y_scale = Some((y, at));
            }
            "attributePlaces" => {
                let at = r.position();
                let mut places = BTreeMap::new();
                map(r, |r, tag| {
                    places.insert(tag.to_owned(), point(r)?);
                    Ok(())
                })?;
                // One way to write an insert: no places is no map.
                if places.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş öznitelik yerleri yazılmaz"));
                }
                f.attribute_places = Some(places);
            }
            "parameters" => {
                let at = r.position();
                let mut values = BTreeMap::new();
                map(r, |r, name| {
                    values.insert(name.to_owned(), param_value(r)?);
                    Ok(())
                })?;
                if values.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş parametre değerleri yazılmaz"));
                }
                f.parameters = Some(values);
            }
            "mirror" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "aynalama false yazılmaz; aynalı olmayan yerleştirmede alan yoktur",
                    ));
                }
                f.mirror = Some(true);
            }
            "align" => f.align = Some(text_align(r)?),
            "boxWidth" => {
                let at = r.position();
                let w = r.float()?;
                if w <= 0.0 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("çok satırlı yazının kutu genişliği {w}; sıfırdan büyük olmalı"),
                    ));
                }
                f.paragraph.box_width = Some(w);
            }
            "lineSpacing" => {
                let at = r.position();
                let s = r.float()?;
                if !(MIN_LINE_SPACING..=MAX_LINE_SPACING).contains(&s) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "çok satırlı yazının satır aralığı {s}; {MIN_LINE_SPACING} ile {MAX_LINE_SPACING} arasında olmalı"
                        ),
                    ));
                }
                f.paragraph.line_spacing = Some(s);
            }
            "runs" => {
                f.runs_at = r.position();
                f.paragraph.runs = list(r, |r, _| text_run(r))?;
                // The writer leaves an empty list out: one spelling (§6.6).
                if f.paragraph.runs.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        f.runs_at,
                        "biçim dilimi listesi boş; dilimsiz yazıda alan yazılmaz",
                    ));
                }
            }
            "arrow" if kind == Kind::Dimension => {
                let names: Vec<(&str, DimensionArrow)> =
                    DimensionArrow::ALL.iter().map(|a| (a.name(), *a)).collect();
                f.look.arrow = Some(super::named(r, &names)?)
            }
            "arrow" => f.arrow = Some(leader_arrow(r, has)?),
            "arrowSize" if kind == Kind::Leader => {
                let at = r.position();
                let x = r.float()?;
                if !leader_arrow_holds(x) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "kılavuzun ok boyu {x}; notun yüksekliğinin {MIN_LEADER_ARROW} ile {MAX_LEADER_ARROW} katı olmalı"
                        ),
                    ));
                }
                f.arrow_size = Some(x);
            }
            "dimLineColor" | "extColor" | "textColor" => {
                let at = r.position();
                let c = text(r)?;
                if !is_hex_colour(&c) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("ölçünün {key} değeri “{c}”; #RRGGBB olmalı"),
                    ));
                }
                let slot = match key {
                    "dimLineColor" => &mut f.look.dim_line_color,
                    "extColor" => &mut f.look.ext_color,
                    _ => &mut f.look.text_color,
                };
                *slot = Some(c);
            }
            "dimLineWeight" | "extWeight" => {
                let at = r.position();
                let w = r.float()?;
                if !line_weight_holds(w) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "ölçünün {key} değeri {w}; 0 ile {MAX_LINE_WEIGHT} mm arasında olmalı"
                        ),
                    ));
                }
                if key == "dimLineWeight" {
                    f.look.dim_line_weight = Some(w);
                } else {
                    f.look.ext_weight = Some(w);
                }
            }
            "dimLineType" | "extLineType" => {
                let t = super::line_type_named(r)?;
                if key == "dimLineType" {
                    f.look.dim_line_type = Some(t);
                } else {
                    f.look.ext_line_type = Some(t);
                }
            }
            "textStyle" | "dimStyle" => {
                let at = r.position();
                let id = text(r)?;
                if id.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "stil kimliği boş"));
                }
                if matches!(kind, Kind::Text | Kind::Table) {
                    f.face_at = at;
                    f.face.text_style = Some(id);
                } else {
                    f.look.dim_style = Some(id);
                }
            }
            "font" => {
                let faced = matches!(kind, Kind::Text | Kind::Table);
                if faced && f.face_at == 0 {
                    f.face_at = r.position();
                }
                let names: Vec<(&str, DrawingFont)> =
                    DrawingFont::ALL.iter().map(|d| (d.id(), *d)).collect();
                let font = Some(super::named(r, &names)?);
                if faced {
                    f.face.font = font;
                } else {
                    f.look.font = font;
                }
            }
            "bold" | "italic" => {
                let at = r.position();
                if f.face_at == 0 {
                    f.face_at = at;
                }
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("{key} false yazılmaz; alan yoksa yazı öyle değildir"),
                    ));
                }
                if key == "bold" {
                    f.face.bold = true;
                } else {
                    f.face.italic = true;
                }
            }
            "oblique" => {
                let at = r.position();
                if f.face_at == 0 {
                    f.face_at = at;
                }
                let o = r.float()?;
                if !oblique_holds(o) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "yazının eğikliği {o}; −{MAX_OBLIQUE} ile {MAX_OBLIQUE} arasında ve sıfırdan farklı olmalı"
                        ),
                    ));
                }
                f.face.oblique = Some(o);
            }
            "arrowSize" | "extOffset" | "extBeyond" | "textGap" => {
                let at = r.position();
                let x = r.float()?;
                let positive = key == "arrowSize";
                if !(x <= MAX_DIMENSION_RATIO && if positive { x > 0.0 } else { x >= 0.0 }) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "ölçünün {key} değeri {x}; {} ve en çok {MAX_DIMENSION_RATIO} olmalı",
                            if positive {
                                "sıfırdan büyük"
                            } else {
                                "0 ya da büyük"
                            }
                        ),
                    ));
                }
                let slot = match key {
                    "arrowSize" => &mut f.look.arrow_size,
                    "extOffset" => &mut f.look.ext_offset,
                    "extBeyond" => &mut f.look.ext_beyond,
                    _ => &mut f.look.text_gap,
                };
                *slot = Some(x);
            }
            "textPlace" => {
                f.look.text_place =
                    Some(super::named(r, &[("centre", DimensionTextPlace::Centre)])?)
            }
            "decimals" => f.look.decimals = Some(r.uint(u64::from(MAX_DIMENSION_DECIMALS))? as u32),
            "unit" => {
                f.look.unit = Some(super::named(
                    r,
                    &[
                        ("mm", DrawingUnit::Mm),
                        ("cm", DrawingUnit::Cm),
                        ("m", DrawingUnit::M),
                    ],
                )?)
            }
            "prefix" | "suffix" => {
                let at = r.position();
                let s = text(r)?;
                let n = s.chars().count();
                if n == 0 || n > MAX_AFFIX || s.chars().any(char::is_control) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "ölçünün {key} değeri yazılamaz: boş olmamalı, en çok {MAX_AFFIX} harf, satır sonu ya da denetim karakteri yok"
                        ),
                    ));
                }
                if key == "prefix" {
                    f.look.prefix = Some(s);
                } else {
                    f.look.suffix = Some(s);
                }
            }
            "widthFactor" => f.width_factor = Some(width_factor(r)?),
            "labelOf" => {
                if f.label_scale.is_none() {
                    f.link_at = r.position();
                }
                f.label_of = Some(EntityId(id16(r)?));
            }
            "labelScale" => {
                if f.label_of.is_none() {
                    f.link_at = r.position();
                }
                let at = r.position();
                let n = r.float()?;
                if !label_scale_ok(n) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("bağlı yazının ölçeği 1:{n}; sıfırdan büyük olmalı"),
                    ));
                }
                f.label_scale = Some(n);
            }
            "mask" => {
                let at = r.position();
                if !r.bool()? {
                    let words = if kind == Kind::Dimension {
                        "zemin false yazılmaz; zeminsiz ölçüde alan yoktur"
                    } else {
                        "zemin false yazılmaz; zeminsiz yazıda alan yoktur"
                    };
                    return Err(r.fail_at(Code::BadValue, at, words));
                }
                f.mask = Some(true);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let entity = build(r, kind, index, &mut f)?;
    // A closed and a wide polyline's rules (docs/adr/0258).
    if let Entity::Polyline(p) = &entity
        && let Some(why) = p.extras_problem(false)
    {
        return Err(r.fail(Code::BadValue, &why));
    }
    let uid = if has.uids {
        Some(required(r, f.uid, "uid")?)
    } else {
        None
    };
    r.pop();
    r.leave();
    Ok((entity, uid, f.block_at))
}

/// A polyline edge's start and end width (docs/adr/0258 §2): two numbers.
fn width(r: &mut Reader<'_>) -> Result<[f64; 2], KcadError> {
    let at = r.position();
    let n = r.array()?;
    if n != 2 {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("genişlik 2 sayı olmalı (başlangıç, bitiş), {n} var"),
        ));
    }
    let start = r.float()?;
    let end = r.float()?;
    r.leave();
    Ok([start, end])
}

fn build(
    r: &mut Reader<'_>,
    kind: Kind,
    index: usize,
    f: &mut Fields,
) -> Result<Entity, KcadError> {
    // The slot a reader gives: 1, 2, 3 … in file order (§6.6). A payload holds far fewer than 2³² objects.
    let slot = u32::try_from(index + 1).unwrap_or(u32::MAX);
    let look = std::mem::take(&mut f.object_look);
    if let Some(problem) = kentos_contracts::look_problem(
        &look,
        f.color.is_some(),
        f.line_weight.is_some(),
        f.uid.is_none(),
    ) {
        return Err(r.fail(Code::BadValue, &problem));
    }
    if look.wipeout.is_some() && !matches!(kind, Kind::Polygon) {
        let name = KINDS
            .iter()
            .find(|(_, k)| *k == kind)
            .map_or("?", |(n, _)| n);
        if let Some(problem) = kentos_contracts::wipeout_problem(&look, name) {
            return Err(r.fail(Code::BadValue, &problem));
        }
    }
    let base = EntityBase {
        look,
        id: slot,
        layer_id: required(r, f.layer_id.take(), "layerId")?,
        color: f.color.take(),
        attrs: required(r, f.attrs.take(), "attrs")?,
        label: f.label.take(),
        symbol: f.symbol.take(),
        line_weight: f.line_weight.take(),
        label_pins: f.label_pins.take().unwrap_or_default(),
        attachments: f.attachments.take().unwrap_or_default(),
    };
    Ok(match kind {
        Kind::Point => Entity::Point(PointEntity {
            base,
            p: required(r, f.p, "p")?,
            z: f.z,
            parts: f.point_parts.take(),
        }),
        Kind::Line => Entity::Line(LineEntity {
            base,
            a: required(r, f.a, "a")?,
            b: required(r, f.b, "b")?,
            za: f.za,
            zb: f.zb,
            ma: f.ma,
            mb: f.mb,
        }),
        Kind::Polyline | Kind::Polygon => {
            let pts = required(r, f.pts.take(), "pts")?;
            let zs = as_long_as(r, f.zs.take(), f.zs_at, pts.len())?;
            let ms = measures_as_long_as(r, f.ms.take(), f.ms_at, pts.len())?;
            let path = PathEntity {
                base,
                pts,
                bulges: f.bulges.take(),
                holes: f.rings.take(),
                zs,
                ms,
                parts: f.parts.take(),
                closed: f.closed.take().unwrap_or(false),
                widths: f.widths.take(),
            };
            if kind == Kind::Polygon {
                Entity::Polygon(path)
            } else {
                Entity::Polyline(path)
            }
        }
        Kind::Circle => Entity::Circle(CircleEntity {
            base,
            c: required(r, f.c, "c")?,
            r: required(r, f.r, "r")?,
        }),
        Kind::Arc => Entity::Arc(ArcEntity {
            base,
            c: required(r, f.c, "c")?,
            r: required(r, f.r, "r")?,
            a0: required(r, f.a0, "a0")?,
            a1: required(r, f.a1, "a1")?,
        }),
        Kind::Ellipse => Entity::Ellipse(EllipseEntity {
            base,
            c: required(r, f.c, "c")?,
            major: required(r, f.major, "major")?,
            ratio: required(r, f.ratio, "ratio")?,
            t0: required(r, f.t0, "t0")?,
            t1: required(r, f.t1, "t1")?,
        }),
        Kind::Spline => Entity::Spline(SplineEntity {
            base,
            pts: required(r, f.pts.take(), "pts")?,
            closed: required(r, f.closed, "closed")?,
        }),
        Kind::Nurbs => {
            let e = kentos_contracts::NurbsEntity {
                base,
                degree: required(r, f.degree, "degree")?,
                ctrl: required(r, f.ctrl.take(), "ctrl")?,
                knots: required(r, f.knots.take(), "knots")?,
                weights: f.weights.take(),
                closed: f.closed.unwrap_or(false),
                periodic: f.periodic,
                fit: f.fit.take(),
                tangents: f.tangents.take(),
                fit_tolerance: f.fit_tolerance,
            };
            if let Some(problem) = e.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Nurbs(e)
        }
        Kind::Xline | Kind::Ray => {
            let line = ConstructionEntity {
                base,
                p: required(r, f.p, "p")?,
                dir: required(r, f.dir, "dir")?,
            };
            if kind == Kind::Xline {
                Entity::Xline(line)
            } else {
                Entity::Ray(line)
            }
        }
        Kind::Text => {
            // A linked text names its object and its scale together (docs/adr/0175 §4).
            if f.label_of.is_some() != f.label_scale.is_some() {
                return Err(r.fail_at(
                    Code::BadValue,
                    f.link_at,
                    "bağlı yazının nesnesi ve ölçeği birlikte verilir",
                ));
            }
            let text = required(r, f.text.take(), "text")?;
            // The runs inside the text, in order, apart and one per format (docs/adr/0182 §1).
            if let Some(words) = f.paragraph.runs_problem(&text) {
                return Err(r.fail_at(Code::BadValue, f.runs_at, &words));
            }
            // Bold, italic and a slant need a typeface (docs/adr/0183 §2).
            if let Some((_, words)) = f.face.problem() {
                return Err(r.fail_at(Code::BadValue, f.face_at, &words));
            }
            // Its curve: a text of one line, not linked (docs/adr/0196 §1).
            if let Some(path) = &f.text_path
                && let Some(words) = kentos_contracts::text_path_problem(
                    path,
                    &text,
                    &f.paragraph,
                    f.label_of.is_some(),
                )
            {
                return Err(r.fail_at(Code::BadValue, f.path_at, &words));
            }
            Entity::Text(TextEntity {
                base,
                p: required(r, f.p, "p")?,
                text,
                height: required(r, f.height, "height")?,
                rotation: required(r, f.rotation, "rotation")?,
                align: f.align,
                width_factor: f.width_factor,
                mask: f.mask.unwrap_or(false),
                label_of: f.label_of,
                label_scale: f.label_scale,
                paragraph: std::mem::take(&mut f.paragraph),
                face: std::mem::take(&mut f.face),
                path: f.text_path.take(),
            })
        }
        Kind::Dimension => {
            // What a style needs, and what only a slope has (docs/adr/0147).
            let style = f.style;
            let c = if matches!(
                style,
                Some(DimensionStyle::ArcLength | DimensionStyle::Jogged)
            ) {
                Some(required(r, f.c, "c")?)
            } else {
                f.c
            };
            let (za, zb) = if style == Some(DimensionStyle::Slope) {
                (
                    Some(required(r, f.za, "za")?),
                    Some(required(r, f.zb, "zb")?),
                )
            } else if style == Some(DimensionStyle::Elevation) {
                // A level's one elevation (docs/adr/0322).
                if f.zb.is_some() {
                    return Err(field_fail(
                        r,
                        "zb",
                        f.zb_at,
                        "kot ölçüsünün yalnız bir kotu (za) olur",
                    ));
                }
                (Some(required(r, f.za, "za")?), None)
            } else if f.za.is_some() || f.zb.is_some() {
                let (key, at) = if f.za.is_some() {
                    ("za", f.za_at)
                } else {
                    ("zb", f.zb_at)
                };
                return Err(field_fail(
                    r,
                    key,
                    at,
                    "kot (za, zb) yalnız eğim ve kot ölçüsünde yazılır",
                ));
            } else {
                (None, None)
            };
            if style == Some(DimensionStyle::Ordinate)
                && let Some(a) = f.angle
                && a != 0.0
                && a != 90.0
            {
                return Err(field_fail(
                    r,
                    "angle",
                    f.angle_at,
                    &format!("koordinat ölçüsünün ekseni {a}; 0 (Y) ya da 90 (X) olmalı"),
                ));
            }
            Entity::Dimension(DimensionEntity {
                text_shift: f.text_shift.take(),
                breaks: std::mem::take(&mut f.breaks),
                base,
                a: required(r, f.a, "a")?,
                b: required(r, f.b, "b")?,
                offset: required(r, f.offset, "offset")?,
                height: required(r, f.height, "height")?,
                text: f.text.take(),
                style,
                angle: f.angle,
                c,
                mask: f.mask.unwrap_or(false),
                za,
                zb,
                refs: f.refs.take().map(Box::new),
                look: std::mem::take(&mut f.look),
            })
        }
        Kind::Hatch => Entity::Hatch(HatchEntity {
            base,
            ring: required(r, f.ring.take(), "ring")?,
            holes: f.loops.take(),
            pattern: required(r, f.pattern.take(), "pattern")?,
            assoc: f.assoc.take(),
        }),
        Kind::Insert => {
            let scale = required(r, f.scale, "scale")?;
            // One way to write a block: the y factor only when it is not the x one.
            if let Some((y, at)) = f.y_scale
                && y == scale
            {
                return Err(r.fail_at(Code::BadValue, at, "y ölçeği x ölçeğine eşitse yazılmaz"));
            }
            Entity::Insert(InsertEntity {
                parameters: f.parameters.take().unwrap_or_default(),
                attribute_places: f.attribute_places.take().unwrap_or_default(),
                base,
                block: required(r, f.block, "block")?,
                p: required(r, f.p, "p")?,
                scale,
                y_scale: f.y_scale.map(|(y, _)| y),
                rotation: required(r, f.rotation, "rotation")?,
                mirror: f.mirror.unwrap_or(false),
            })
        }
        Kind::Leader => Entity::Leader(LeaderEntity {
            base,
            pts: required(r, f.pts.take(), "pts")?,
            text: f.text.take(),
            height: required(r, f.height, "height")?,
            rotation: required(r, f.rotation, "rotation")?,
            arrow: f.arrow,
            arrow_size: f.arrow_size,
            mask: f.mask.unwrap_or(false),
        }),
        Kind::Table => {
            // Bold, italic and a slant need a typeface (docs/adr/0183 §2).
            if let Some((_, words)) = f.face.problem() {
                return Err(r.fail_at(Code::BadValue, f.face_at, &words));
            }
            let table = TableEntity {
                base,
                p: required(r, f.p, "p")?,
                rotation: required(r, f.rotation, "rotation")?,
                height: required(r, f.height, "height")?,
                rows: required(r, f.rows.take(), "rows")?,
                columns: required(r, f.columns.take(), "columns")?,
                cells: required(r, f.cells.take(), "cells")?,
                merges: f.merges.take().unwrap_or_default(),
                aligns: f.aligns.take(),
                header: f.header.unwrap_or(false),
                grid: f.grid,
                frame: f.frame,
                face: std::mem::take(&mut f.face),
                source: f.source.take(),
            };
            // Its rows, columns, cells and ranges hold together (docs/adr/0184 §1).
            if let Some((field, words)) = table.shape().problem() {
                return Err(field_fail(r, field, f.table_at.of(field), &words));
            }
            Entity::Table(table)
        }
        Kind::Image => {
            let image = ImageFields {
                p: required(r, f.p, "p")?,
                width: required(r, f.width, "width")?,
                height: required(r, f.height, "height")?,
                rotation: required(r, f.rotation, "rotation")?,
                mirror: f.mirror.unwrap_or(false),
                asset: f.asset.take(),
                file: f.file.take(),
                clip: f.clip.take(),
                opacity: f.opacity,
            };
            // Its size, its one source, its clip and opacity hold together (docs/adr/0192 §1).
            if let Some(words) = image.problem() {
                return Err(r.fail(Code::BadValue, &words));
            }
            Entity::Image(ImageEntity { base, image })
        }
        Kind::Raster => {
            let raster = RasterFields {
                affine: required(r, f.raster.affine, "affine")?,
                width: required(r, f.raster.width, "width")?,
                height: required(r, f.raster.height, "height")?,
                bands: required(r, f.raster.bands, "bands")?,
                sample: required(r, f.raster.sample, "sample")?,
                asset: f.asset.take(),
                file: f.file.take(),
                url: f.url.take(),
                srid: required(r, f.raster.srid, "srid")?,
                style: required(r, f.raster.style.take(), "style")?,
                opacity: f.opacity,
                dataset: f.raster.dataset.take(),
            };
            // Its place, size, bands, one source, look and opacity hold together (docs/adr/0204 §2).
            if let Some(words) = raster.problem() {
                return Err(r.fail(Code::BadValue, &words));
            }
            Entity::Raster(RasterEntity { base, raster })
        }
        Kind::PointCloud => {
            let cloud = PointCloudFields {
                sources: required(r, f.cloud.sources.take(), "sources")?,
                bounds: required(r, f.cloud.bounds, "bounds")?,
                count: required(r, f.cloud.count, "count")?,
                srid: required(r, f.raster.srid, "srid")?,
                style: required(r, f.cloud.style.take(), "style")?,
                opacity: f.opacity,
            };
            // Its files, bounds, points, look and opacity hold together (docs/adr/0207 §3).
            if let Some(words) = cloud.problem() {
                return Err(r.fail(Code::BadValue, &words));
            }
            Entity::PointCloud(PointCloudEntity { base, cloud })
        }
    })
}

/// Six numbers: a bounds' `[x₁, y₁, z₁, x₂, y₂, z₂]`.
fn bounds6(r: &mut Reader<'_>) -> Result<[f64; 6], KcadError> {
    let at = r.position();
    let v = floats(r)?;
    v.as_slice().try_into().map_err(|_| {
        r.fail_at(
            Code::BadValue,
            at,
            &format!("kapsam 6 sayı olmalı, {} var", v.len()),
        )
    })
}

/// A point cloud's files (§6.6, docs/adr/0207 §3): each its one source, format, points and bounds.
fn cloud_sources(r: &mut Reader<'_>) -> Result<Vec<CloudSource>, KcadError> {
    list(r, |r, _| {
        let (mut asset, mut file, mut url) = (None, None, None);
        let (mut format, mut count, mut bounds) = (None, None, None);
        map(r, |r, key| {
            match key {
                "asset" => asset = Some(text(r)?),
                "file" => file = Some(text(r)?),
                "url" => url = Some(text(r)?),
                "format" => {
                    let names: Vec<(&str, CloudFormat)> =
                        CloudFormat::ALL.iter().map(|v| (v.name(), *v)).collect();
                    format = Some(named(r, &names)?);
                }
                "count" => count = Some(r.uint(u64::MAX)?),
                "bounds" => bounds = Some(bounds6(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(CloudSource {
            asset,
            file,
            url,
            format: required(r, format, "format")?,
            count: required(r, count, "count")?,
            bounds: required(r, bounds, "bounds")?,
        })
    })
}

/// A point cloud's look (§6.6, docs/adr/0207 §5): `render` and `size` always,
/// the rest only when not their default (one spelling: no `invert` or `rgb8`
/// false, no `sizeUnit` "px", no `shape` "round", no empty `hidden`).
fn cloud_style(r: &mut Reader<'_>) -> Result<PointCloudStyle, KcadError> {
    let (mut render, mut size) = (None, None);
    let mut st = PointCloudStyle {
        render: CloudRender::Rgb,
        ramp: None,
        invert: false,
        min: None,
        max: None,
        hidden: Vec::new(),
        rgb8: false,
        size: 0.0,
        size_unit: PointSizeUnit::Px,
        shape: PointShape::Round,
    };
    let only_true = |r: &mut Reader<'_>, words: &str| -> Result<bool, KcadError> {
        let at = r.position();
        if !r.bool()? {
            return Err(r.fail_at(Code::BadValue, at, words));
        }
        Ok(true)
    };
    map(r, |r, key| {
        match key {
            "render" => {
                let names: Vec<(&str, CloudRender)> =
                    CloudRender::ALL.iter().map(|v| (v.name(), *v)).collect();
                render = Some(named(r, &names)?);
            }
            "size" => size = Some(r.float()?),
            "ramp" => st.ramp = Some(text(r)?),
            "min" => st.min = Some(r.float()?),
            "max" => st.max = Some(r.float()?),
            "invert" => {
                st.invert = only_true(
                    r,
                    "ters çevirme false yazılmaz; çevrilmemiş rampada alan yoktur",
                )?;
            }
            "rgb8" => {
                st.rgb8 = only_true(r, "rgb8 false yazılmaz; 16 bitlik renklerde alan yoktur")?;
            }
            "hidden" => {
                let at = r.position();
                let v = list(r, |r, _| Ok(r.uint(255)? as u8))?;
                if v.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "gizlenen sınıf listesi boş; gizlenen sınıf yoksa alan yazılmaz",
                    ));
                }
                st.hidden = v;
            }
            "sizeUnit" => {
                let at = r.position();
                let names: Vec<(&str, PointSizeUnit)> =
                    PointSizeUnit::ALL.iter().map(|v| (v.name(), *v)).collect();
                st.size_unit = named(r, &names)?;
                if st.size_unit == PointSizeUnit::Px {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boyun birimi “px” yazılmaz; piksel boyda alan yoktur",
                    ));
                }
            }
            "shape" => {
                let at = r.position();
                let names: Vec<(&str, PointShape)> =
                    PointShape::ALL.iter().map(|v| (v.name(), *v)).collect();
                st.shape = named(r, &names)?;
                if st.shape == PointShape::Round {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "biçim “round” yazılmaz; yuvarlak noktada alan yoktur",
                    ));
                }
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    st.render = required(r, render, "render")?;
    st.size = required(r, size, "size")?;
    Ok(st)
}

/// A raster's look (§6.6, docs/adr/0204 §4): `render` and `bands` always, the
/// rest only when not their default (one spelling: no `stretch` "none", no
/// `resampling` "bilinear", no `invert` false).
fn raster_style(r: &mut Reader<'_>, multidim: bool) -> Result<RasterStyle, KcadError> {
    let (mut render, mut bands) = (None, None);
    let mut st = RasterStyle {
        render: RasterRender::Rgb,
        bands: Vec::new(),
        stretch: RasterStretch::None,
        min: None,
        max: None,
        ramp: None,
        invert: false,
        azimuth: None,
        altitude: None,
        z_factor: None,
        nodata: None,
        resampling: RasterResampling::Bilinear,
        edges: None,
    };
    map(r, |r, key| {
        match key {
            "render" => {
                let names: Vec<(&str, RasterRender)> =
                    RasterRender::ALL.iter().map(|v| (v.name(), *v)).collect();
                render = Some(named(r, &names)?);
            }
            "bands" => {
                bands = Some(list(r, |r, _| {
                    Ok(r.uint(u64::from(MAX_RASTER_BANDS))? as u32)
                })?)
            }
            "stretch" => {
                let at = r.position();
                let names: Vec<(&str, RasterStretch)> =
                    RasterStretch::ALL.iter().map(|v| (v.name(), *v)).collect();
                st.stretch = named(r, &names)?;
                if st.stretch == RasterStretch::None {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "gerdirme “none” yazılmaz; gerdirmesiz görünüşte alan yoktur",
                    ));
                }
            }
            "resampling" => {
                let at = r.position();
                let names: Vec<(&str, RasterResampling)> = RasterResampling::ALL
                    .iter()
                    .map(|v| (v.name(), *v))
                    .collect();
                st.resampling = named(r, &names)?;
                if st.resampling == RasterResampling::Bilinear {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "örnekleme “bilinear” yazılmaz; çift doğrusal görünüşte alan yoktur",
                    ));
                }
            }
            "invert" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "ters çevirme false yazılmaz; çevrilmemiş rampada alan yoktur",
                    ));
                }
                st.invert = true;
            }
            "ramp" => st.ramp = Some(text(r)?),
            "min" => st.min = Some(r.float()?),
            "max" => st.max = Some(r.float()?),
            "azimuth" => st.azimuth = Some(r.float()?),
            "altitude" => st.altitude = Some(r.float()?),
            "zFactor" => st.z_factor = Some(r.float()?),
            "nodata" => st.nodata = Some(r.float()?),
            // Schema 38: a mesh's lines (docs/adr/0243 §5).
            "edges" if multidim => st.edges = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    st.render = required(r, render, "render")?;
    st.bands = required(r, bands, "bands")?;
    Ok(st)
}

/// A raster's NetCDF variable and slice (§6.6, docs/adr/0243 §6): `variable`
/// always; `vector`, `mesh`, `dims` and `followTime` only when given (no empty
/// list, no false).
fn raster_dataset(r: &mut Reader<'_>) -> Result<kentos_contracts::RasterDataset, KcadError> {
    let mut d = kentos_contracts::RasterDataset {
        variable: String::new(),
        vector: None,
        mesh: None,
        dims: Vec::new(),
        follow_time: false,
    };
    let mut variable = None;
    map(r, |r, key| {
        match key {
            "variable" => variable = Some(text(r)?),
            "vector" => d.vector = Some(text(r)?),
            "mesh" => d.mesh = Some(text(r)?),
            "followTime" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(Code::BadValue, at, "followTime false yazılmaz"));
                }
                d.follow_time = true;
            }
            "dims" => {
                let at = r.position();
                d.dims = list(r, |r, _| dataset_dim(r))?;
                if d.dims.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş dilim boyutu listesi yazılmaz"));
                }
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    d.variable = required(r, variable, "variable")?;
    Ok(d)
}

/// A dataset's slice dimension: `name`, `index`, `values`; `time` and `units` when given.
fn dataset_dim(r: &mut Reader<'_>) -> Result<kentos_contracts::DatasetDim, KcadError> {
    let (mut name, mut index, mut values) = (None, None, None);
    let mut dim = kentos_contracts::DatasetDim {
        name: String::new(),
        index: 0,
        values: Vec::new(),
        time: false,
        units: None,
    };
    map(r, |r, key| {
        match key {
            "name" => name = Some(text(r)?),
            "index" => index = Some(r.uint(u64::from(u32::MAX))? as u32),
            "values" => values = Some(floats(r)?),
            "time" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(Code::BadValue, at, "time false yazılmaz"));
                }
                dim.time = true;
            }
            "units" => dim.units = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    dim.name = required(r, name, "name")?;
    dim.index = required(r, index, "index")?;
    dim.values = required(r, values, "values")?;
    Ok(dim)
}

/// A table's merged range (§6.6): its `row`, `col`, `rows` and `cols`.
fn cell_range(r: &mut Reader<'_>) -> Result<CellRange, KcadError> {
    let (mut row, mut col, mut rows, mut cols) = (None, None, None, None);
    let limit = u64::from(u32::MAX);
    map(r, |r, key| {
        let n = Some(r.uint(limit)? as u32);
        match key {
            "row" => row = n,
            "col" => col = n,
            "rows" => rows = n,
            "cols" => cols = n,
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(CellRange {
        row: required(r, row, "row")?,
        col: required(r, col, "col")?,
        rows: required(r, rows, "rows")?,
        cols: required(r, cols, "cols")?,
    })
}

/// Where a table's rows came from (§6.6): its `kind` and the objects' ids,
/// or a file's `name` and `sheet`.
fn table_source(r: &mut Reader<'_>) -> Result<TableSource, KcadError> {
    let at = r.position();
    let (mut kind, mut objects, mut name, mut sheet) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "kind" => {
                kind = Some(named(
                    r,
                    &[
                        ("coordinates", 0u8),
                        ("areas", 1),
                        ("attributes", 2),
                        ("file", 3),
                    ],
                )?)
            }
            "objects" => objects = Some(list(r, |r, _| Ok(EntityId(id16(r)?)))?),
            "name" => name = Some(text(r)?),
            "sheet" => sheet = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let kind = required(r, kind, "kind")?;
    if kind == 3 {
        if objects.is_some() {
            return Err(r.fail_at(
                Code::BadValue,
                at,
                "dosyadan gelen tablonun kaynağında nesne yazılmaz",
            ));
        }
        return Ok(TableSource::File {
            name: required(r, name, "name")?,
            sheet,
        });
    }
    if name.is_some() || sheet.is_some() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "nesnelerden gelen tablonun kaynağında dosya adı yazılmaz",
        ));
    }
    let objects = required(r, objects, "objects")?;
    Ok(match kind {
        0 => TableSource::Coordinates { objects },
        1 => TableSource::Areas { objects },
        _ => TableSource::Attributes { objects },
    })
}

/// A refusal of the field `key` (at byte `at`) once its object's map is read: its place names it.
fn field_fail(r: &mut Reader<'_>, key: &'static str, at: usize, words: &str) -> KcadError {
    r.push(Seg::Name(key));
    let e = r.fail_at(Code::BadValue, at, words);
    r.pop();
    e
}

/// A hole of a polygon; `has`: whether the payload's schema gives it elevations.
fn ring(r: &mut Reader<'_>, has: Features) -> Result<RingGeometry, KcadError> {
    let (mut pts, mut bulges, mut zs, mut zs_at) = (None, None, None, 0);
    map(r, |r, key| {
        match key {
            "pts" => pts = Some(points(r)?),
            "bulges" => bulges = Some(floats(r)?),
            "zs" if has.elevations => {
                zs_at = r.position();
                zs = Some(elevations(r)?);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let pts = required(r, pts, "pts")?;
    let zs = as_long_as(r, zs, zs_at, pts.len())?;
    Ok(RingGeometry { pts, bulges, zs })
}

/// A part of a multi-part area past its first (§6.6, docs/adr/0143): its
/// ring, arcs, holes and elevations, as the area's own fields are.
fn part(r: &mut Reader<'_>, has: Features) -> Result<AreaPart, KcadError> {
    let (mut pts, mut bulges, mut holes, mut zs, mut zs_at) = (None, None, None, None, 0);
    map(r, |r, key| {
        match key {
            "pts" => pts = Some(points(r)?),
            "bulges" => bulges = Some(floats(r)?),
            "holes" => holes = Some(list(r, |r, _| ring(r, has))?),
            "zs" if has.elevations => {
                zs_at = r.position();
                zs = Some(elevations(r)?);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let pts = required(r, pts, "pts")?;
    let zs = as_long_as(r, zs, zs_at, pts.len())?;
    Ok(AreaPart {
        pts,
        bulges,
        holes,
        zs,
        ms: None,
    })
}

/// A part of a multi-part polyline past its first (schema 17, §6.6): two
/// vertices or more, its arcs and elevations; no holes (an unknown field).
fn line_part(r: &mut Reader<'_>, has: Features) -> Result<AreaPart, KcadError> {
    let (mut pts, mut bulges, mut zs, mut zs_at, mut pts_at) = (None, None, None, 0, 0);
    let (mut ms, mut ms_at) = (None, 0);
    map(r, |r, key| {
        match key {
            "pts" => {
                pts_at = r.position();
                pts = Some(points(r)?);
            }
            "bulges" => bulges = Some(floats(r)?),
            "zs" if has.elevations => {
                zs_at = r.position();
                zs = Some(elevations(r)?);
            }
            // Its measures from schema 39 on (docs/adr/0224).
            "ms" if has.measures => {
                ms_at = r.position();
                ms = Some(elevations(r)?);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let pts = required(r, pts, "pts")?;
    if pts.len() < 2 {
        return Err(r.fail_at(
            Code::BadValue,
            pts_at,
            &format!(
                "çoklu çizginin parçasının {} köşesi var; en az iki olmalı",
                pts.len()
            ),
        ));
    }
    let zs = as_long_as(r, zs, zs_at, pts.len())?;
    let ms = measures_as_long_as(r, ms, ms_at, pts.len())?;
    Ok(AreaPart {
        pts,
        bulges,
        holes: None,
        zs,
        ms,
    })
}

/// A point of a multi-point object past its first (schema 17, §6.6): its
/// place and elevation.
fn point_part(r: &mut Reader<'_>) -> Result<PointPart, KcadError> {
    let (mut p, mut z) = (None, None);
    map(r, |r, key| {
        match key {
            "p" => p = Some(point(r)?),
            "z" => z = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(PointPart {
        p: required(r, p, "p")?,
        z,
    })
}

/// A list of elevations: metres, `null` for a vertex without one (§6.6).
fn elevations(r: &mut Reader<'_>) -> Result<Vec<Option<f64>>, KcadError> {
    list(r, |r, _| r.float_or_null())
}

/// `zs` as it is when it has one elevation per vertex, else a bad value
/// (§6.6). Keys are read in encoded order, `zs` before `pts`, so the count is
/// checked once the whole map is read; `at` is where the `zs` list starts.
fn as_long_as(
    r: &mut Reader<'_>,
    zs: Option<Vec<Option<f64>>>,
    at: usize,
    vertices: usize,
) -> Result<Option<Vec<Option<f64>>>, KcadError> {
    if let Some(list) = &zs
        && list.len() != vertices
    {
        r.push(Seg::Name("zs"));
        let e = r.fail_at(
            Code::BadValue,
            at,
            &format!(
                "{} kot var ama {vertices} köşe var; her köşenin bir kotu olmalı (kotsuz köşe için null)",
                list.len()
            ),
        );
        r.pop();
        return Err(e);
    }
    Ok(zs)
}

/// `ms` as it is when it has one measure per vertex, else a bad value
/// (§6.6, docs/adr/0224), as [`as_long_as`]: `ms` comes before `pts`.
fn measures_as_long_as(
    r: &mut Reader<'_>,
    ms: Option<Vec<Option<f64>>>,
    at: usize,
    vertices: usize,
) -> Result<Option<Vec<Option<f64>>>, KcadError> {
    if let Some(list) = &ms
        && list.len() != vertices
    {
        r.push(Seg::Name("ms"));
        let e = r.fail_at(
            Code::BadValue,
            at,
            &format!(
                "{} ölçü var ama {vertices} köşe var; her köşenin bir ölçüsü olmalı (ölçüsüz köşe için null)",
                list.len()
            ),
        );
        r.pop();
        return Err(e);
    }
    Ok(ms)
}

/// A hatch's pattern (§6.6): schema 23 adds the `pattern` and `gradient`
/// kinds and their fields (docs/adr/0186 §1), checked as the commands check
/// them; the first three kinds read as they always did.
fn pattern(r: &mut Reader<'_>, has: Features) -> Result<HatchPattern, KcadError> {
    const TYPES: [(&str, HatchPatternType); 5] = [
        ("solid", HatchPatternType::Solid),
        ("lines", HatchPatternType::Lines),
        ("cross", HatchPatternType::Cross),
        ("pattern", HatchPatternType::Pattern),
        ("gradient", HatchPatternType::Gradient),
    ];
    let at = r.position();
    let (mut kind, mut angle, mut spacing) = (None, None, None);
    let (mut name, mut scale, mut lines, mut gradient) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "type" => {
                kind = Some(named(
                    r,
                    if has.hatches { &TYPES[..] } else { &TYPES[..3] },
                )?)
            }
            "angle" => angle = Some(r.float()?),
            "spacing" => spacing = Some(r.float()?),
            "name" if has.hatches => name = Some(text(r)?),
            "scale" if has.hatches => scale = Some(r.float()?),
            "lines" if has.hatches => lines = Some(list(r, |r, _| pattern_line(r))?),
            "gradient" if has.hatches => gradient = Some(hatch_gradient(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let p = HatchPattern {
        kind: required(r, kind, "type")?,
        angle: required(r, angle, "angle")?,
        spacing: required(r, spacing, "spacing")?,
        name,
        scale,
        lines,
        gradient,
    };
    if p.has_definition()
        && let Some((_, words)) = p.problem()
    {
        return Err(r.fail_at(Code::BadValue, at, &words));
    }
    Ok(p)
}

/// A pattern's family (docs/adr/0186 §1): its angle, origin, offset and dashes.
fn pattern_line(r: &mut Reader<'_>) -> Result<PatternLine, KcadError> {
    let (mut angle, mut origin, mut offset, mut dashes) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "angle" => angle = Some(r.float()?),
            "dashes" => {
                let at = r.position();
                let list = floats(r)?;
                // The writer leaves an empty list out: one spelling (§6.6).
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "kesik listesi boş; bütün çizgili ailede alan yazılmaz",
                    ));
                }
                dashes = Some(list);
            }
            "offset" => offset = Some(point(r)?),
            "origin" => origin = Some(point(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let pair = |p: Vec2| [p.x, p.y];
    Ok(PatternLine {
        angle: required(r, angle, "angle")?,
        origin: pair(required(r, origin, "origin")?),
        offset: pair(required(r, offset, "offset")?),
        dashes: dashes.unwrap_or_default(),
    })
}

/// A gradient (docs/adr/0186 §1): its shape, its second colour and, written
/// only when it is so, that it runs the other way.
fn hatch_gradient(r: &mut Reader<'_>) -> Result<HatchGradient, KcadError> {
    let (mut shape, mut color2, mut inverted) = (None, None, false);
    map(r, |r, key| {
        match key {
            "shape" => {
                shape = Some(named(
                    r,
                    &[
                        ("linear", GradientShape::Linear),
                        ("cylinder", GradientShape::Cylinder),
                        ("spherical", GradientShape::Spherical),
                    ],
                )?)
            }
            "color2" => color2 = Some(text(r)?),
            "inverted" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "degradenin “inverted”ı yanlışken yazılmaz",
                    ));
                }
                inverted = true;
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(HatchGradient {
        shape: required(r, shape, "shape")?,
        inverted,
        color2: required(r, color2, "color2")?,
    })
}

/// The objects a hatch's region follows (docs/adr/0186 §6): its closed
/// object, islands and cutouts by their persistent ids, and its seed.
/// An area's mask (docs/adr/0310): `{ frame }`.
fn wipeout(r: &mut Reader<'_>) -> Result<kentos_contracts::Wipeout, KcadError> {
    let mut frame = None;
    map(r, |r, key| {
        match key {
            "frame" => frame = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(kentos_contracts::Wipeout {
        frame: required(r, frame, "frame")?,
    })
}

fn hatch_assoc(r: &mut Reader<'_>) -> Result<HatchAssoc, KcadError> {
    let at = r.position();
    let (mut seed, mut outer, mut islands, mut cutouts) = (None, None, Vec::new(), Vec::new());
    map(r, |r, key| {
        match key {
            "seed" => seed = Some(point(r)?),
            "outer" => outer = Some(EntityId(id16(r)?)),
            "cutouts" | "islands" => {
                let at = r.position();
                let ids = list(r, |r, _| Ok(EntityId(id16(r)?)))?;
                // The writer leaves an empty list out: one spelling (§6.6).
                if ids.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "nesne listesi boş; nesnesi olmayan listenin alanı yazılmaz",
                    ));
                }
                if key == "cutouts" {
                    cutouts = ids;
                } else {
                    islands = ids;
                }
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let a = HatchAssoc {
        outer: required(r, outer, "outer")?,
        islands,
        cutouts,
        seed: required(r, seed, "seed")?,
    };
    if let Some((_, words)) = a.problem() {
        return Err(r.fail_at(Code::BadValue, at, &words));
    }
    Ok(a)
}

/// What a dimension's points follow (docs/adr/0249 §4): `a`, `b` and `c`, each a reference; a point
/// without one is not written, nor are references none of its points has.
fn dimension_refs(
    r: &mut Reader<'_>,
    extras: bool,
    rules: bool,
    pairs: bool,
) -> Result<DimensionRefs, KcadError> {
    let at = r.position();
    let mut refs = DimensionRefs::default();
    map(r, |r, key| {
        let slot = match key {
            "a" => &mut refs.a,
            "b" => &mut refs.b,
            "c" => &mut refs.c,
            // Schema 60 (docs/adr/0320 §1).
            "rule" if rules => {
                let here = r.position();
                refs.rule = Some(match r.text()? {
                    "curve" => kentos_contracts::DimensionRule::Curve,
                    "edges" => kentos_contracts::DimensionRule::Edges,
                    // Schema 62 (docs/adr/0322).
                    "pair" if pairs => kentos_contracts::DimensionRule::Pair,
                    other => {
                        let words = format!("ölçünün başvuru kuralı “{other}” bilinmiyor");
                        return Err(r.fail_at(Code::BadValue, here, &words));
                    }
                });
                return Ok(());
            }
            _ => return Err(unknown(r)),
        };
        *slot = Some(subref(r, extras)?);
        Ok(())
    })?;
    if let Some(words) = refs.problem() {
        return Err(r.fail_at(Code::BadValue, at, &words));
    }
    Ok(refs)
}

/// The places a reference may name, by their written kind; `along` only from schema 51.
const PLACE_KINDS: [(&str, u8); 7] = [
    ("node", 1),
    ("start", 0),
    ("end", 0),
    ("center", 0),
    ("vertex", 3),
    ("mid", 3),
    ("along", 3),
];

/// A reference (docs/adr/0249 §4): the object's persistent id, the place (its `kind` and the
/// numbers that kind has), its block path (schema 51, docs/adr/0263 §3; never empty) and, when it
/// broke, why.
fn subref(r: &mut Reader<'_>, extras: bool) -> Result<SubRef, KcadError> {
    let at = r.position();
    let (mut uid, mut place, mut broken, mut path) = (None, None, None, Vec::new());
    map(r, |r, key| {
        match key {
            "uid" => uid = Some(EntityId(id16(r)?)),
            "place" => place = Some(sub_place(r, extras)?),
            "path" if extras => {
                let at = r.position();
                let n = r.array()?;
                if n == 0 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "Başvurunun blok yolu boş: boşsa alan yazılmaz.",
                    ));
                }
                for _ in 0..n {
                    path.push(r.uint(u64::from(SUBREF_INDEX_LIMIT))? as u32);
                }
            }
            "broken" => broken = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let s = SubRef {
        uid: required(r, uid, "uid")?,
        place: required(r, place, "place")?,
        path,
        broken,
    };
    if let Some(words) = s.problem() {
        return Err(r.fail_at(Code::BadValue, at, &words));
    }
    Ok(s)
}

/// A place on an object: `kind` first (the keys' order), then its numbers, each at most
/// [`SUBREF_INDEX_LIMIT`]; a number its kind does not have is an unknown field.
fn sub_place(r: &mut Reader<'_>, extras: bool) -> Result<SubPlace, KcadError> {
    let (mut kind, mut numbers, mut t) = (None::<&str>, [None::<u32>; 3], None::<f64>);
    map(r, |r, key| {
        let slot = match key {
            "kind" => {
                let names = PLACE_KINDS.map(|(n, _)| (n, n));
                let known = if extras { &names[..] } else { &names[..6] };
                kind = Some(named(r, known)?);
                return Ok(());
            }
            // Shorter keys first: an along place's t comes before its kind.
            "t" if extras => {
                t = Some(r.float()?);
                return Ok(());
            }
            "part" => 0,
            "ring" => 1,
            "index" => 2,
            _ => return Err(unknown(r)),
        };
        // A node has its part only; a vertex and a middle all three; the others none.
        let has = kind
            .and_then(|k| PLACE_KINDS.iter().find(|(n, _)| *n == k))
            .map_or(0, |(_, n)| *n as usize);
        if slot >= has {
            return Err(unknown(r));
        }
        numbers[slot] = Some(r.uint(u64::from(SUBREF_INDEX_LIMIT))? as u32);
        Ok(())
    })?;
    let kind = required(r, kind, "kind")?;
    if t.is_some() && kind != "along" {
        return Err(unknown(r));
    }
    let mut number = |i: usize, key: &'static str| required(r, numbers[i], key);
    Ok(match kind {
        "node" => SubPlace::Node {
            part: number(0, "part")?,
        },
        "start" => SubPlace::Start,
        "end" => SubPlace::End,
        "center" => SubPlace::Center,
        "vertex" => SubPlace::Vertex {
            part: number(0, "part")?,
            ring: number(1, "ring")?,
            index: number(2, "index")?,
        },
        "mid" => SubPlace::Mid {
            part: number(0, "part")?,
            ring: number(1, "ring")?,
            index: number(2, "index")?,
        },
        _ => SubPlace::Along {
            part: number(0, "part")?,
            ring: number(1, "ring")?,
            index: number(2, "index")?,
            t: required(r, t, "t")?,
        },
    })
}

/// A multi-line text's run (§6.6, docs/adr/0182): its range and format;
/// a false flag is not written (its absence is false).
/// A text's curve (§6.6, docs/adr/0196): `pts` and, when an edge bends, `bulges`.
fn text_path(r: &mut Reader<'_>) -> Result<kentos_contracts::TextPath, KcadError> {
    let (mut pts, mut bulges) = (None, None);
    map(r, |r, key| {
        match key {
            "pts" => pts = Some(points(r)?),
            "bulges" => bulges = Some(floats(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(kentos_contracts::TextPath {
        pts: required(r, pts, "pts")?,
        bulges,
    })
}

fn text_run(r: &mut Reader<'_>) -> Result<TextRun, KcadError> {
    let (mut start, mut end) = (None, None);
    let mut run = TextRun::default();
    map(r, |r, key| {
        let flag = |r: &mut Reader<'_>, name: &str| -> Result<bool, KcadError> {
            let at = r.position();
            if !r.bool()? {
                return Err(r.fail_at(
                    Code::BadValue,
                    at,
                    &format!("{name} false yazılmaz; biçimsiz dilimde alan yoktur"),
                ));
            }
            Ok(true)
        };
        match key {
            "start" => start = Some(r.uint(u64::from(u32::MAX))? as u32),
            "end" => end = Some(r.uint(u64::from(u32::MAX))? as u32),
            "bold" => run.bold = flag(r, "bold")?,
            "italic" => run.italic = flag(r, "italic")?,
            "underline" => run.underline = flag(r, "underline")?,
            "script" => {
                run.script = Some(named(
                    r,
                    &[("super", TextScript::Super), ("sub", TextScript::Sub)],
                )?)
            }
            "color" => run.color = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    run.start = required(r, start, "start")?;
    run.end = required(r, end, "end")?;
    Ok(run)
}

/// A text's alignment by its name (§6.6); an unknown name is `bad_value`.
/// A leader's arrowhead (§6.6, docs/adr/0146): its name; the filled arrow
/// has none. AutoCAD's arrowheads beside the first three come with schema 30
/// (docs/adr/0205 §7): an older payload naming one is `bad_value`.
fn leader_arrow(r: &mut Reader<'_>, has: Features) -> Result<LeaderArrow, KcadError> {
    let names: Vec<(&str, LeaderArrow)> = LeaderArrow::ALL
        .iter()
        .filter(|a| has.annotation || !a.is_added())
        .map(|a| (a.name(), *a))
        .collect();
    super::named(r, &names)
}

pub(super) fn text_align(r: &mut Reader<'_>) -> Result<TextAlign, KcadError> {
    let names: Vec<(&str, TextAlign)> = TextAlign::ALL.iter().map(|a| (a.name(), *a)).collect();
    super::named(r, &names)
}

/// A width factor (§6.6): over 0, at most `MAX_WIDTH_FACTOR`; not finite is the float's `non_finite`.
pub(super) fn width_factor(r: &mut Reader<'_>) -> Result<f64, KcadError> {
    let at = r.position();
    let w = r.float()?;
    if !width_factor_ok(w) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("genişlik çarpanı {w}; 0'dan büyük, en çok {MAX_WIDTH_FACTOR} olmalı"),
        ));
    }
    Ok(w)
}

/// A parameter's value (docs/adr/0318 §3): `true` or `false`, a float, a text or a point `[x, y]`.
fn param_value(r: &mut Reader<'_>) -> Result<kentos_contracts::ParamValue, KcadError> {
    use kentos_contracts::ParamValue as V;
    Ok(match r.peek() {
        Some(0xf4 | 0xf5) => V::Flag(r.bool()?),
        Some(b) if b >> 5 == 3 => V::Text(text(r)?),
        Some(b) if b >> 5 == 4 => V::Point(point(r)?),
        _ => V::Number(r.float()?),
    })
}

/// A contract's part written as its JSON (docs/adr/0321): read, checked against the writer's form (its defaults not
/// written, floats as floats), and where it starts.
pub(super) fn json_part<T: serde::de::DeserializeOwned + serde::Serialize>(
    r: &mut Reader<'_>,
    what: &str,
) -> Result<(T, usize), KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let read: T = serde_json::from_value(value.clone()).map_err(|e| {
        let words = e.to_string();
        let code = if words.contains("unknown field") {
            Code::UnknownField
        } else {
            Code::BadValue
        };
        r.fail_at(code, at, &format!("{what} okunamadı: {words}"))
    })?;
    if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("{what} yazıcının biçiminde değil (varsayılan değer yazılmış ya da ondalık yerine tam sayı)"),
        ));
    }
    Ok((read, at))
}
