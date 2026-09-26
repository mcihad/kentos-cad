//! The objects of document schema 2 read from a payload (docs/specs/kcad-v2.md
//! §6.6): each a one-key map, its kind and then its fields, read into the
//! contract's `Entity` with the persistent id the file gives it; the ids are
//! unique in a file.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{
    ArcEntity, CircleEntity, ConstructionEntity, DimensionEntity, DimensionStyle, EllipseEntity,
    Entity, EntityBase, EntityId, HatchEntity, HatchPattern, HatchPatternType, LineEntity,
    PathEntity, PointEntity, RingGeometry, SplineEntity, TextEntity, Vec2,
};

use super::{floats, id16, list, map, named, point, points, required, text, unknown};
use crate::cbor::{Reader, Seg};
use crate::error::{Code, KcadError};

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
}

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
];

/// Whether a kind's map may hold `key` (the fields every kind has, then its own).
fn allowed(kind: Kind, key: &str) -> bool {
    matches!(
        key,
        "uid" | "attrs" | "color" | "label" | "symbol" | "layerId"
    ) || match kind {
        Kind::Point => matches!(key, "p" | "z"),
        Kind::Line => matches!(key, "a" | "b"),
        Kind::Polyline => matches!(key, "pts" | "bulges"),
        Kind::Polygon => matches!(key, "pts" | "bulges" | "holes"),
        Kind::Circle => matches!(key, "c" | "r"),
        Kind::Arc => matches!(key, "c" | "r" | "a0" | "a1"),
        Kind::Ellipse => matches!(key, "c" | "major" | "ratio" | "t0" | "t1"),
        Kind::Spline => matches!(key, "pts" | "closed"),
        Kind::Xline | Kind::Ray => matches!(key, "p" | "dir"),
        Kind::Text => matches!(key, "p" | "text" | "height" | "rotation"),
        Kind::Dimension => matches!(
            key,
            "a" | "b" | "c" | "text" | "angle" | "style" | "height" | "offset"
        ),
        Kind::Hatch => matches!(key, "ring" | "holes" | "pattern"),
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
    p: Option<Vec2>,
    a: Option<Vec2>,
    b: Option<Vec2>,
    c: Option<Vec2>,
    major: Option<Vec2>,
    dir: Option<Vec2>,
    z: Option<f64>,
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
    rings: Option<Vec<RingGeometry>>,
    loops: Option<Vec<Vec<Vec2>>>,
    closed: Option<bool>,
    text: Option<String>,
    style: Option<DimensionStyle>,
    pattern: Option<HatchPattern>,
}

/// The objects and their persistent ids, each id once (§6.8).
pub(super) fn objects(r: &mut Reader<'_>) -> Result<(Vec<Entity>, Vec<EntityId>), KcadError> {
    let mut seen = HashSet::new();
    let mut uids = Vec::new();
    let entities = list(r, |r, i| {
        let (entity, uid) = object(r, i)?;
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
        Ok(entity)
    })?;
    Ok((entities, uids))
}

fn object(r: &mut Reader<'_>, index: usize) -> Result<(Entity, EntityId), KcadError> {
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
    let Some(&(_, kind)) = KINDS.iter().find(|(k, _)| *k == name) else {
        return Err(r.fail(
            Code::UnknownKind,
            &format!(
                "“{name}” nesne türü bilinmiyor; dosya daha yeni bir KentOS'la yazılmış olabilir"
            ),
        ));
    };
    let mut f = Fields::default();
    map(r, |r, key| {
        if !allowed(kind, key) {
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
            "p" => f.p = Some(point(r)?),
            "a" => f.a = Some(point(r)?),
            "b" => f.b = Some(point(r)?),
            "c" => f.c = Some(point(r)?),
            "major" => f.major = Some(point(r)?),
            "dir" => f.dir = Some(point(r)?),
            "z" => f.z = Some(r.float()?),
            "r" => f.r = Some(r.float()?),
            "a0" => f.a0 = Some(r.float()?),
            "a1" => f.a1 = Some(r.float()?),
            "ratio" => f.ratio = Some(r.float()?),
            "t0" => f.t0 = Some(r.float()?),
            "t1" => f.t1 = Some(r.float()?),
            "height" => f.height = Some(r.float()?),
            "rotation" => f.rotation = Some(r.float()?),
            "offset" => f.offset = Some(r.float()?),
            "angle" => f.angle = Some(r.float()?),
            "pts" => f.pts = Some(points(r)?),
            "ring" => f.ring = Some(points(r)?),
            "bulges" => f.bulges = Some(floats(r)?),
            "holes" if kind == Kind::Polygon => f.rings = Some(list(r, |r, _| ring(r))?),
            "holes" => f.loops = Some(list(r, |r, _| points(r))?),
            "closed" => f.closed = Some(r.bool()?),
            "text" => f.text = Some(text(r)?),
            "style" => {
                f.style = Some(named(
                    r,
                    &[
                        ("aligned", DimensionStyle::Aligned),
                        ("linear", DimensionStyle::Linear),
                        ("angular", DimensionStyle::Angular),
                        ("radius", DimensionStyle::Radius),
                        ("diameter", DimensionStyle::Diameter),
                    ],
                )?)
            }
            "pattern" => f.pattern = Some(pattern(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let entity = build(r, kind, index, &mut f)?;
    let uid = required(r, f.uid, "uid")?;
    r.pop();
    r.leave();
    Ok((entity, uid))
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
    };
    Ok(match kind {
        Kind::Point => Entity::Point(PointEntity {
            base,
            p: required(r, f.p, "p")?,
            z: f.z,
        }),
        Kind::Line => Entity::Line(LineEntity {
            base,
            a: required(r, f.a, "a")?,
            b: required(r, f.b, "b")?,
        }),
        Kind::Polyline | Kind::Polygon => {
            let path = PathEntity {
                base,
                pts: required(r, f.pts.take(), "pts")?,
                bulges: f.bulges.take(),
                holes: f.rings.take(),
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
        }),
        Kind::Dimension => Entity::Dimension(DimensionEntity {
            base,
            a: required(r, f.a, "a")?,
            b: required(r, f.b, "b")?,
            offset: required(r, f.offset, "offset")?,
            height: required(r, f.height, "height")?,
            text: f.text.take(),
            style: f.style,
            angle: f.angle,
            c: f.c,
        }),
        Kind::Hatch => Entity::Hatch(HatchEntity {
            base,
            ring: required(r, f.ring.take(), "ring")?,
            holes: f.loops.take(),
            pattern: required(r, f.pattern.take(), "pattern")?,
        }),
    })
}

fn ring(r: &mut Reader<'_>) -> Result<RingGeometry, KcadError> {
    let (mut pts, mut bulges) = (None, None);
    map(r, |r, key| {
        match key {
            "pts" => pts = Some(points(r)?),
            "bulges" => bulges = Some(floats(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(RingGeometry {
        pts: required(r, pts, "pts")?,
        bulges,
    })
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
