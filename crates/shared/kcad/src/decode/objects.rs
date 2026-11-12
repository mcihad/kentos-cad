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
    RasterSample, RasterStretch, RasterStyle, RecordEntity, RingGeometry, SUBREF_INDEX_LIMIT,
    SplineEntity, SubPlace, SubRef, TableAlign, TableEntity, TableGrid, TableSource, TextAlign,
    TextEntity, TextFace, TextRun, TextScript, Vec2, is_hex_colour, label_scale_ok,
    leader_arrow_holds, line_weight_holds, oblique_holds, width_factor_ok,
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
    Record,
    Wall,
    Opening,
    Space,
    Grid,
    Column,
    Stair,
    Ramp,
    Tag,
    ViewMark,
    Slab,
    Beam,
    Roof,
    Railing,
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
    ("record", Kind::Record),
    ("wall", Kind::Wall),
    ("opening", Kind::Opening),
    ("space", Kind::Space),
    ("grid", Kind::Grid),
    ("column", Kind::Column),
    ("stair", Kind::Stair),
    ("ramp", Kind::Ramp),
    ("tag", Kind::Tag),
    ("viewmark", Kind::ViewMark),
    ("slab", Kind::Slab),
    ("beam", Kind::Beam),
    ("roof", Kind::Roof),
    ("railing", Kind::Railing),
];

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
    georef: Option<Box<kentos_contracts::RasterGeoref>>,
    tiles: Option<kentos_contracts::RasterTiles>,
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
    /// An annotation's scales (docs/adr/0323).
    annotative: Option<Box<kentos_contracts::Annotative>>,
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
    /// A wall's own fields (docs/adr/0339 §4).
    wall_style: Option<String>,
    wall_justify: Option<kentos_contracts::WallJustify>,
    wall_caps: Option<kentos_contracts::WallCaps>,
    wall_joins: Option<kentos_contracts::WallJoins>,
    wall_cleanup: Option<bool>,
    wall_fill: Option<String>,
    wall_components: Vec<kentos_contracts::WallComponent>,
    wall_arch: Option<kentos_contracts::ArchInfo>,
    opening_host: Option<String>,
    opening_at: Option<f64>,
    opening_place: Option<Vec<Vec2>>,
    opening: Option<crate::opening::OpeningRest>,
    space_parts: Option<Vec<kentos_contracts::SpacePart>>,
    space: Option<crate::space::SpaceRest>,
    grid_line: Option<kentos_contracts::GridFields>,
    clipping: Option<kentos_contracts::InsertClip>,
    column: Option<kentos_contracts::ColumnFields>,
    stair: Option<kentos_contracts::StairFields>,
    ramp: Option<kentos_contracts::RampFields>,
    tag: Option<kentos_contracts::TagFields>,
    viewmark: Option<kentos_contracts::ViewMarkFields>,
    /// A slab's fields (docs/adr/0722).
    slab: Option<kentos_contracts::SlabFields>,
    beam: Option<kentos_contracts::BeamFields>,
    roof: Option<kentos_contracts::RoofFields>,
    railing: Option<kentos_contracts::RailingFields>,
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
    space_separator: Option<kentos_contracts::ArchInfo>,
    text: Option<String>,
    style: Option<DimensionStyle>,
    pattern: Option<HatchPattern>,
    assoc: Option<HatchAssoc>,
    /// A hatch's exact boundary (schema 69, docs/adr/0328 §1).
    hatch_boundary: Option<Vec<kentos_contracts::HatchLoop>>,
    /// A dimension's references (docs/adr/0249 §4).
    refs: Option<DimensionRefs>,
    /// A dimension's moved value and breaks (docs/adr/0321).
    text_shift: Option<kentos_contracts::TextShift>,
    /// A Kot's level (docs/adr/0721 §2), where it was read.
    dimension_level: Option<(kentos_contracts::DimensionLevel, usize)>,
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
    layout_at: usize,
    fields_at: usize,
    /// A leader's schema 67 fields (docs/adr/0326) and where they began.
    leader_arms: Vec<kentos_contracts::LeaderArm>,
    leader_tip: Option<kentos_contracts::SubRef>,
    leader_blocks: Vec<kentos_contracts::EntityId>,
    leader_style: Option<String>,
    leader_at: usize,
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
    /// Schema 68 (docs/adr/0327): a table's cells' styles, formulas and fields, a linked file's data.
    table_styles: Vec<kentos_contracts::CellStyle>,
    table_formulas: Vec<kentos_contracts::CellFormula>,
    table_fields: Vec<kentos_contracts::CellField>,
    table_link: Option<Link>,
    table_breaks: Option<kentos_contracts::TableBreaks>,
    /// Schema 185 (docs/adr/1126 §3): the table style a table follows, by id.
    table_style: Option<String>,
}

/// A linked file's path, digest, copy and rule, as a table's `link` holds them (schema 68, docs/adr/0327 §5).
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Link {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    copy: Option<Vec<Vec<String>>>,
    #[serde(
        default,
        skip_serializing_if = "kentos_contracts::SourceOverrides::is_keep"
    )]
    overrides: kentos_contracts::SourceOverrides,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key_column: Option<u32>,
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
        match r.check_object(i, &entity, &uid) {
            None => entities.push(entity),
            Some(None) => {}
            Some(Some(what)) => return Err(KcadError::new(Code::VerifyFailed, what)),
        }
        r.pop();
    }
    r.leave();
    r.report(Step::Reading { done: n, total: n })?;
    Ok((entities, uids))
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
    let mut origin = None;
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
            // Schema 71 (docs/adr/0331 §2).
            "origin" if has.hatch_extras => origin = Some(point(r)?),
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
        origin,
    };
    if (p.has_definition() || p.origin.is_some())
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

fn hatch_assoc(r: &mut Reader<'_>, faces: bool, gaps: bool) -> Result<HatchAssoc, KcadError> {
    let at = r.position();
    let (mut seed, mut outer, mut islands, mut cutouts) = (None, None, Vec::new(), Vec::new());
    let mut bounds = Vec::new();
    let mut gap = None;
    map(r, |r, key| {
        match key {
            // Schema 70 (docs/adr/0329 §3).
            "gap" if gaps => gap = Some(r.float()?),
            "seed" => seed = Some(point(r)?),
            "outer" => outer = Some(EntityId(id16(r)?)),
            // A face's line work (schema 69, docs/adr/0328 §3).
            "cutouts" | "islands" | "bounds" if key != "bounds" || faces => {
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
                match key {
                    "cutouts" => cutouts = ids,
                    "bounds" => bounds = ids,
                    _ => islands = ids,
                }
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    // Before schema 69 a tie always names its closed object.
    if !faces && outer.is_none() {
        return Err(r.fail_at(Code::BadValue, at, "taramanın ilişkisinde outer eksik"));
    }
    let a = HatchAssoc {
        gap,
        outer,
        bounds,
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
    crosses: bool,
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
                    // Schema 118 (docs/adr/0711).
                    "cross" if crosses => kentos_contracts::DimensionRule::Cross,
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

fn text_run(r: &mut Reader<'_>, stacks: bool) -> Result<TextRun, KcadError> {
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
                // Schema 65 (docs/adr/0324 §1): a stacked fraction.
                let scripts: &[(&str, TextScript)] = if stacks {
                    &[
                        ("super", TextScript::Super),
                        ("sub", TextScript::Sub),
                        ("stack", TextScript::Stack),
                    ]
                } else {
                    &[("super", TextScript::Super), ("sub", TextScript::Sub)]
                };
                run.script = Some(named(r, scripts)?)
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
/// has none. The DXF arrowheads beside the first three come with schema 30
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
    json_value_part(r, what, value, at)
}

fn json_value_part<T: serde::de::DeserializeOwned + serde::Serialize>(
    r: &mut Reader<'_>,
    what: &str,
    value: serde_json::Value,
    at: usize,
) -> Result<(T, usize), KcadError> {
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

mod features;
pub(crate) use features::*;

mod object;
pub(crate) use object::*;

mod build;
use build::*;
