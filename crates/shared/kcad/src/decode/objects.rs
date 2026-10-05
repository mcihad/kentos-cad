//! The objects of document schemas 2 to 9 read from a payload
//! (docs/specs/kcad-v2.md §6.6): each a one-key map, its kind and then its
//! fields, read into the contract's `Entity` with the persistent id the file
//! gives it; the ids are unique in a file. Schema 3 adds an object's own line
//! weight (`lineWeight`, docs/adr/0139), schema 4 the vertex elevations (`za`,
//! `zb`, `zs`, docs/adr/0142), schema 5 an area's parts (`parts`,
//! docs/adr/0143), schema 6 the `insert` kind (docs/adr/0144), schema 7 a
//! text's `align`, `widthFactor` and `mask` (docs/adr/0145), schema 8 the
//! `leader` kind (docs/adr/0146), schema 9 the dimension's new kinds, its
//! `mask` and a slope's `za`, `zb` (docs/adr/0147); in an older schema they
//! are unknown fields, kinds or values. A block definition's objects are
//! read the same way, without persistent ids.

use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_contracts::{
    ArcEntity, AreaPart, BlockDefinition, BlockId, CircleEntity, ConstructionEntity,
    DimensionEntity, DimensionStyle, EllipseEntity, Entity, EntityBase, EntityId, HatchEntity,
    HatchPattern, HatchPatternType, InsertEntity, LeaderArrow, LeaderEntity, LineEntity,
    MAX_LINE_WEIGHT, MAX_WIDTH_FACTOR, PathEntity, PointEntity, PointPart, RingGeometry,
    SplineEntity, TextAlign, TextEntity, Vec2, width_factor_ok,
};

use super::{PREALLOCATE, floats, id16, list, map, named, point, points, required, text, unknown};
use crate::cbor::{Reader, Seg};
use crate::error::{Code, KcadError};
use crate::watch::{EVERY, Step};
use crate::{
    SCHEMA_WITH_BLOCKS, SCHEMA_WITH_CUSTOM_CRS, SCHEMA_WITH_DIMENSIONS, SCHEMA_WITH_DRAWING_UNIT,
    SCHEMA_WITH_ELEVATIONS, SCHEMA_WITH_GROUND, SCHEMA_WITH_LAYER_SNAP, SCHEMA_WITH_LEADERS,
    SCHEMA_WITH_LINE_PARTS, SCHEMA_WITH_LINE_WEIGHTS, SCHEMA_WITH_PARTS, SCHEMA_WITH_SECOND_SRID,
    SCHEMA_WITH_SURVEY, SCHEMA_WITH_TEXT_EXTRAS, SCHEMA_WITH_TRAVERSE_TOLERANCES,
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
    Xline,
    Ray,
    Text,
    Dimension,
    Hatch,
    Insert,
    Leader,
}

/// The dimension's kinds in the contract's order: schema 9 added the last five.
const DIMENSION_STYLES: [DimensionStyle; 10] = DimensionStyle::ALL;

const KINDS: &[(&str, Kind)] = &[
    ("point", Kind::Point),
    ("line", Kind::Line),
    ("polyline", Kind::Polyline),
    ("polygon", Kind::Polygon),
    ("circle", Kind::Circle),
    ("arc", Kind::Arc),
    ("ellipse", Kind::Ellipse),
    ("spline", Kind::Spline),
    ("xline", Kind::Xline),
    ("ray", Kind::Ray),
    ("text", Kind::Text),
    ("dimension", Kind::Dimension),
    ("hatch", Kind::Hatch),
    ("insert", Kind::Insert),
    ("leader", Kind::Leader),
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
    /// Schema 17: a polyline's and a point's parts (`parts`, docs/adr/0174).
    pub(super) line_parts: bool,
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
            line_parts: schema >= SCHEMA_WITH_LINE_PARTS,
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
        || (has.weights && key == "lineWeight")
        || match kind {
            Kind::Point => matches!(key, "p" | "z") || (has.line_parts && key == "parts"),
            Kind::Line => {
                matches!(key, "a" | "b") || (has.elevations && matches!(key, "za" | "zb"))
            }
            Kind::Polyline => {
                matches!(key, "pts" | "bulges")
                    || (has.elevations && key == "zs")
                    || (has.line_parts && key == "parts")
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
            Kind::Xline | Kind::Ray => matches!(key, "p" | "dir"),
            Kind::Text => {
                matches!(key, "p" | "text" | "height" | "rotation")
                    || (has.texts && matches!(key, "align" | "widthFactor" | "mask"))
            }
            Kind::Dimension => {
                matches!(
                    key,
                    "a" | "b" | "c" | "text" | "angle" | "style" | "height" | "offset"
                ) || (has.dimensions && matches!(key, "mask" | "za" | "zb"))
            }
            Kind::Hatch => matches!(key, "ring" | "holes" | "pattern"),
            Kind::Insert => matches!(key, "block" | "p" | "scale" | "rotation" | "mirror"),
            Kind::Leader => matches!(
                key,
                "pts" | "text" | "height" | "rotation" | "arrow" | "mask"
            ),
        }
}

/// Every field an object may have; each kind takes its own.
#[derive(Default)]
struct Fields {
    uid: Option<EntityId>,
    attrs: Option<BTreeMap<String, String>>,
    color: Option<String>,
    label: Option<String>,
    symbol: Option<String>,
    layer_id: Option<String>,
    line_weight: Option<f64>,
    p: Option<Vec2>,
    a: Option<Vec2>,
    b: Option<Vec2>,
    c: Option<Vec2>,
    major: Option<Vec2>,
    dir: Option<Vec2>,
    z: Option<f64>,
    za: Option<f64>,
    zb: Option<f64>,
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
    rings: Option<Vec<RingGeometry>>,
    parts: Option<Vec<AreaPart>>,
    /// A multi-point object's points past its first (schema 17).
    point_parts: Option<Vec<PointPart>>,
    loops: Option<Vec<Vec<Vec2>>>,
    closed: Option<bool>,
    text: Option<String>,
    style: Option<DimensionStyle>,
    pattern: Option<HatchPattern>,
    block: Option<BlockId>,
    /// Where the insert's `block` is in the payload (for an unknown block).
    block_at: usize,
    scale: Option<f64>,
    mirror: Option<bool>,
    align: Option<TextAlign>,
    arrow: Option<LeaderArrow>,
    width_factor: Option<f64>,
    mask: Option<bool>,
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
    let Some(&(_, kind)) = KINDS
        .iter()
        .find(|(k, _)| *k == name)
        .filter(|(_, kind)| {
            (has.blocks || *kind != Kind::Insert) && (has.leaders || *kind != Kind::Leader)
        })
    else {
        return Err(r.fail(
            Code::UnknownKind,
            &format!(
                "“{name}” nesne türü bilinmiyor; dosya daha yeni bir KentOS'la yazılmış olabilir"
            ),
        ));
    };
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
            "r" => f.r = Some(r.float()?),
            "a0" => f.a0 = Some(r.float()?),
            "a1" => f.a1 = Some(r.float()?),
            "ratio" => f.ratio = Some(r.float()?),
            "t0" => f.t0 = Some(r.float()?),
            "t1" => f.t1 = Some(r.float()?),
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
            "height" => f.height = Some(r.float()?),
            "rotation" => f.rotation = Some(r.float()?),
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
            "holes" if kind == Kind::Polygon => f.rings = Some(list(r, |r, _| ring(r, has))?),
            "holes" => f.loops = Some(list(r, |r, _| points(r))?),
            "parts" if kind == Kind::Point => f.point_parts = Some(list(r, |r, _| point_part(r))?),
            "parts" if kind == Kind::Polyline => f.parts = Some(list(r, |r, _| line_part(r, has))?),
            "parts" => f.parts = Some(list(r, |r, _| part(r, has))?),
            "closed" => f.closed = Some(r.bool()?),
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
                let known = if has.dimensions { &styles[..] } else { &styles[..5] };
                f.style = Some(named(r, known)?)
            }
            "pattern" => f.pattern = Some(pattern(r)?),
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
            "arrow" => f.arrow = Some(leader_arrow(r)?),
            "widthFactor" => f.width_factor = Some(width_factor(r)?),
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
    let uid = if has.uids {
        Some(required(r, f.uid, "uid")?)
    } else {
        None
    };
    r.pop();
    r.leave();
    Ok((entity, uid, f.block_at))
}

fn build(
    r: &mut Reader<'_>,
    kind: Kind,
    index: usize,
    f: &mut Fields,
) -> Result<Entity, KcadError> {
    // The slot a reader gives: 1, 2, 3 … in file order (§6.6). A payload holds far fewer than 2³² objects.
    let slot = u32::try_from(index + 1).unwrap_or(u32::MAX);
    let base = EntityBase {
        id: slot,
        layer_id: required(r, f.layer_id.take(), "layerId")?,
        color: f.color.take(),
        attrs: required(r, f.attrs.take(), "attrs")?,
        label: f.label.take(),
        symbol: f.symbol.take(),
        line_weight: f.line_weight.take(),
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
        }),
        Kind::Polyline | Kind::Polygon => {
            let pts = required(r, f.pts.take(), "pts")?;
            let zs = as_long_as(r, f.zs.take(), f.zs_at, pts.len())?;
            let path = PathEntity {
                base,
                pts,
                bulges: f.bulges.take(),
                holes: f.rings.take(),
                zs,
                parts: f.parts.take(),
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
        Kind::Text => Entity::Text(TextEntity {
            base,
            p: required(r, f.p, "p")?,
            text: required(r, f.text.take(), "text")?,
            height: required(r, f.height, "height")?,
            rotation: required(r, f.rotation, "rotation")?,
            align: f.align,
            width_factor: f.width_factor,
            mask: f.mask.unwrap_or(false),
        }),
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
                (Some(required(r, f.za, "za")?), Some(required(r, f.zb, "zb")?))
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
                    "kot (za, zb) yalnız eğim ölçüsünde yazılır",
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
            })
        }
        Kind::Hatch => Entity::Hatch(HatchEntity {
            base,
            ring: required(r, f.ring.take(), "ring")?,
            holes: f.loops.take(),
            pattern: required(r, f.pattern.take(), "pattern")?,
        }),
        Kind::Insert => Entity::Insert(InsertEntity {
            base,
            block: required(r, f.block, "block")?,
            p: required(r, f.p, "p")?,
            scale: required(r, f.scale, "scale")?,
            rotation: required(r, f.rotation, "rotation")?,
            mirror: f.mirror.unwrap_or(false),
        }),
        Kind::Leader => Entity::Leader(LeaderEntity {
            base,
            pts: required(r, f.pts.take(), "pts")?,
            text: f.text.take(),
            height: required(r, f.height, "height")?,
            rotation: required(r, f.rotation, "rotation")?,
            arrow: f.arrow,
            mask: f.mask.unwrap_or(false),
        }),
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
    })
}

/// A part of a multi-part polyline past its first (schema 17, §6.6): two
/// vertices or more, its arcs and elevations; no holes (an unknown field).
fn line_part(r: &mut Reader<'_>, has: Features) -> Result<AreaPart, KcadError> {
    let (mut pts, mut bulges, mut zs, mut zs_at, mut pts_at) = (None, None, None, 0, 0);
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
    Ok(AreaPart {
        pts,
        bulges,
        holes: None,
        zs,
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

fn pattern(r: &mut Reader<'_>) -> Result<HatchPattern, KcadError> {
    let (mut kind, mut angle, mut spacing) = (None, None, None);
    map(r, |r, key| {
        match key {
            "type" => {
                kind = Some(named(
                    r,
                    &[
                        ("solid", HatchPatternType::Solid),
                        ("lines", HatchPatternType::Lines),
                        ("cross", HatchPatternType::Cross),
                    ],
                )?)
            }
            "angle" => angle = Some(r.float()?),
            "spacing" => spacing = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(HatchPattern {
        kind: required(r, kind, "type")?,
        angle: required(r, angle, "angle")?,
        spacing: required(r, spacing, "spacing")?,
    })
}

/// A text's alignment by its name (§6.6); an unknown name is `bad_value`.
/// A leader's arrowhead (§6.6, docs/adr/0146): its name; the filled arrow has none.
fn leader_arrow(r: &mut Reader<'_>) -> Result<LeaderArrow, KcadError> {
    let names: Vec<(&str, LeaderArrow)> = LeaderArrow::ALL.iter().map(|a| (a.name(), *a)).collect();
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
