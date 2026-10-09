//! The shared product command cases (fixtures/commands/v1, docs/adr/0022,
//! 0027, 0029, 0032, 0037, 0047, 0057, 0066) run against the desktop's handlers over the native document. The web
//! runs the same files against its own handlers
//! (apps/web/src/product/fixtures.test.ts); the format is in
//! fixtures/commands/README.md.
//!
//! JSON carries no NaN or ±∞: a step's `nonFinite` puts them into the typed
//! input after it is read, at the paths the errors name.

use std::collections::HashMap;
use std::path::PathBuf;

use kentos_domain::contracts::{
    ArcCreate, ArrayLayout, CAD_ARC_CREATE, CAD_CIRCLE_CREATE, CAD_ENTITIES_ARRAY,
    CAD_ENTITIES_DELETE, CAD_ENTITIES_EDIT, CAD_ENTITIES_TRANSFORM, CAD_LINE_CREATE,
    CAD_POINT_CREATE, CAD_POLYGON_CREATE, CAD_POLYLINE_CREATE, CircleCreate, DocumentSnapshotV1,
    EntitiesArray, EntitiesDelete, EntitiesEdit, EntitiesTransform, EntityEdit, EntityGeometry,
    LineCreate, PointCreate, PolygonCreate, PolylineCreate, Transform,
};
use kentos_domain::contracts::{
    BlockId, BlocksDefine, BlocksEdit, CAD_BLOCKS_DEFINE, CAD_BLOCKS_EDIT, CAD_ENTITIES_SET,
    EntitiesCreate, EntitiesSetProperties, LayersFilter, LayersService, LayersTime, NetworkDefine,
    ScenariosEdit,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::create;
use kentos_native_application::{
    DESKTOP_COMMANDS, ExecutionContext, arc, array, blocks_define, blocks_edit, circle, delete,
    edit, layers_filter, layers_service, layers_time, line, network_define, point, polygon,
    polyline, scenarios_edit, set, transform,
};
use serde_json::{Value, json};

type Outcome<T> = Result<T, String>;

#[derive(Default)]
struct State {
    /// Revisions taken by `captureRevision`, as the text commands use.
    revisions: HashMap<String, String>,
    /// Persistent ids taken by `captureUid`.
    uids: HashMap<String, Uuid>,
    /// Block ids taken by `captureBlock` (docs/adr/0144).
    blocks: HashMap<String, BlockId>,
    /// The ids of the setup's definitions: a new one is none of them.
    setup_blocks: Vec<BlockId>,
    /// The layer the last command added or would add (`$layer`, docs/adr/0208 §15).
    layer: Option<String>,
    /// The layers the last command that added any added or would add, by
    /// their number (`$layer:1`, `$layer:2` …, docs/adr/0210 §11).
    fresh: Vec<String>,
}

const STEP_KEYS: &[&str] = &[
    "op",
    "input",
    "nonFinite",
    "result",
    "returns",
    "as",
    "id",
    "name",
    "expect",
    "note",
];

/// JSON equality with numbers compared as numbers (the file's `1` is the document's `1.0`).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

fn expect_same(got: &Value, want: &Value, what: &str, at: &str) -> Outcome<()> {
    if same(got, want) {
        Ok(())
    } else {
        Err(format!("{at}: {what}: beklenen {want}, bulunan {got}"))
    }
}

/// `value` with its `$…` placeholders filled in: `$current` is the
/// document's revision now, `$name` one `captureRevision` took; `$uid:name`,
/// anywhere in a text (an id list, a message), the persistent id
/// `captureUid` took, lowercase with hyphens; `$uidOf:12`, the persistent id
/// the object in slot 12 has now (a copy a command just wrote);
/// `$blockOf:Rögar`, the id of the drawing's block of that name now, and
/// `$block:name`, one `captureBlock` took (docs/adr/0144); `$layer:2`, the
/// second layer the last command that added any added or would add, by
/// their numbers (docs/adr/0210 §11).
fn fill(value: &Value, doc: &Document, state: &State, at: &str) -> Outcome<Value> {
    Ok(match value {
        Value::String(text) if text.starts_with("$blockOf:") => {
            let name = &text["$blockOf:".len()..];
            let block = doc
                .block_named(name)
                .ok_or_else(|| format!("{at}: “{name}” bloğu çizimde yok"))?;
            Value::String(block.id.to_text())
        }
        Value::String(text) if text.starts_with("$block:") => {
            let name = &text["$block:".len()..];
            let id = state
                .blocks
                .get(name)
                .ok_or_else(|| format!("{at}: “{name}” blok kimliği alınmadı"))?;
            Value::String(id.to_text())
        }
        Value::String(text) if text.starts_with("$uidOf:") => {
            let slot = text["$uidOf:".len()..]
                .parse::<u32>()
                .map_err(|_| format!("{at}: {text}: yuva bir sayı olmalı"))?;
            let uid = doc
                .uid(Slot(slot))
                .ok_or_else(|| format!("{at}: {slot} yuvasında nesne yok"))?;
            Value::String(uid.to_string())
        }
        Value::String(text) if text.contains("$uid:") => Value::String(with_uids(text, state, at)?),
        Value::String(text) if text.starts_with("$layer:") => {
            let k = text["$layer:".len()..]
                .parse::<usize>()
                .ok()
                .filter(|k| *k >= 1)
                .ok_or_else(|| format!("{at}: {text}: sıra 1'den başlayan bir sayı olmalı"))?;
            Value::String(state.fresh.get(k - 1).cloned().ok_or_else(|| {
                format!(
                    "{at}: {text}: son komut {} katman ekledi",
                    state.fresh.len()
                )
            })?)
        }
        Value::String(text) if text == "$layer" => Value::String(
            state
                .layer
                .clone()
                .ok_or_else(|| format!("{at}: $layer: bir komut katman eklemedi"))?,
        ),
        Value::String(text) if text.starts_with('$') => {
            let name = &text[1..];
            let revision = if name == "current" {
                doc.revision().to_string()
            } else {
                state
                    .revisions
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("{at}: “{name}” sürümü alınmadı"))?
            };
            Value::String(revision)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| fill(v, doc, state, at))
                .collect::<Outcome<_>>()?,
        ),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), fill(v, doc, state, at)?)))
                .collect::<Outcome<_>>()?,
        ),
        other => other.clone(),
    })
}

/// An expected object with its block ids (`$blockOf:Ad`, `$block:name`,
/// docs/adr/0144) and the persistent ids of the objects it names (`$uidOf:12`:
/// a linked text's object, docs/adr/0175 §4) filled in; nothing else of it
/// is read as a placeholder.
fn block_ids(value: &Value, doc: &Document, state: &State, at: &str) -> Outcome<Value> {
    Ok(match value {
        Value::String(text)
            if text.starts_with("$blockOf:")
                || text.starts_with("$block:")
                || text.starts_with("$uidOf:")
                || text.starts_with("$layer:") =>
        {
            fill(value, doc, state, at)?
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| block_ids(v, doc, state, at))
                .collect::<Outcome<_>>()?,
        ),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), block_ids(v, doc, state, at)?)))
                .collect::<Outcome<_>>()?,
        ),
        other => other.clone(),
    })
}

/// `text` with every `$uid:name` replaced by the persistent id `captureUid` took as `name`.
fn with_uids(text: &str, state: &State, at: &str) -> Outcome<String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("$uid:") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "$uid:".len()..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(after.len());
        let name = &after[..end];
        let uid = state
            .uids
            .get(name)
            .ok_or_else(|| format!("{at}: “{name}” kimliği alınmadı"))?;
        out.push_str(&uid.to_string());
        rest = &after[end..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The number of a point list at `rest` (`pts[1].y`, after any ring prefix).
fn point_number<'a>(
    pts: &'a mut [kentos_domain::contracts::Vec2],
    rest: &str,
) -> Option<&'a mut f64> {
    let (i, axis) = rest.strip_prefix("pts[")?.split_once("].")?;
    let p = pts.get_mut(i.parse::<usize>().ok()?)?;
    match axis {
        "x" => Some(&mut p.x),
        "y" => Some(&mut p.y),
        _ => None,
    }
}

/// The bulge at `rest` (`bulges[3]`, after any ring prefix).
fn bulge_number<'a>(bulges: &'a mut Option<Vec<f64>>, rest: &str) -> Option<&'a mut f64> {
    let i = rest.strip_prefix("bulges[")?.strip_suffix(']')?;
    bulges.as_mut()?.get_mut(i.parse::<usize>().ok()?)
}

/// A command's typed input, with the numbers `nonFinite` may name.
trait Input {
    /// The number at an error path; `None` when the input has no such place.
    fn number(&mut self, path: &str) -> Option<&mut f64>;
}

impl Input for PolygonCreate {
    /// `pts[1].y`, `bulges[3]`, `holes[0].pts[2].x`, `holes[1].bulges[0]`,
    /// `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (pts, bulges, rest) = match path.strip_prefix("holes[") {
            Some(rest) => {
                let (h, rest) = rest.split_once("].")?;
                let ring = self.holes.as_mut()?.get_mut(h.parse::<usize>().ok()?)?;
                (&mut ring.pts, &mut ring.bulges, rest)
            }
            None => (&mut self.pts, &mut self.bulges, path),
        };
        if rest.starts_with("pts[") {
            point_number(pts, rest)
        } else {
            bulge_number(bulges, rest)
        }
    }
}

impl Input for LineCreate {
    /// `a.x`, `b.y`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (end, axis) = path.split_once('.')?;
        let p = match end {
            "a" => &mut self.a,
            "b" => &mut self.b,
            _ => return None,
        };
        match axis {
            "x" => Some(&mut p.x),
            "y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for PolylineCreate {
    /// `pts[1].y`, `bulges[0]`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            self.line_weight.as_mut()
        } else if path.starts_with("pts[") {
            point_number(&mut self.pts, path)
        } else {
            bulge_number(&mut self.bulges, path)
        }
    }
}

impl Input for EntitiesDelete {
    /// No number: the input is ids.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for BlocksDefine {
    /// `base.x`, `base.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        coordinate(&mut self.base, "base", path)
    }
}

impl Input for LayersService {
    /// `service.opacity` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "service.opacity" => self.service.as_mut()?.opacity.as_mut(),
            _ => None,
        }
    }
}

/// No number of these is a NaN or ±∞ case (docs/adr/0210 §11).
impl Input for LayersTime {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ScenariosEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersFilter {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for NetworkDefine {
    /// `network.tolerance` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "network.tolerance" => self.network.as_mut().map(|n| &mut n.tolerance),
            _ => None,
        }
    }
}

impl Input for BlocksEdit {
    /// `base.x`, `base.y`; `attributes[0].p.x`, `attributes[1].rotation`,
    /// `attributes[2].height` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let Some(rest) = path.strip_prefix("attributes[") else {
            return coordinate(self.base.as_mut()?, "base", path);
        };
        let (i, field) = rest.split_once("].")?;
        let a = self
            .attributes
            .as_mut()?
            .get_mut(i.parse::<usize>().ok()?)?;
        match field {
            "rotation" => Some(&mut a.rotation),
            "height" => Some(&mut a.height),
            _ => coordinate(&mut a.p, "p", field),
        }
    }
}

impl Input for EntitiesSetProperties {
    /// `lineWeight` (which the step's input must give); the rest is ids and texts.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "lineWeight" => self.line_weight.as_mut()?.as_mut(),
            _ => None,
        }
    }
}

/// A coordinate of a named point: `c.x`, `p.y`.
fn coordinate<'a>(
    p: &'a mut kentos_domain::contracts::Vec2,
    name: &str,
    path: &str,
) -> Option<&'a mut f64> {
    match path.strip_prefix(name)?.strip_prefix('.')? {
        "x" => Some(&mut p.x),
        "y" => Some(&mut p.y),
        _ => None,
    }
}

impl Input for PointCreate {
    /// `p.x`, `p.y`, `z` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "z" => self.z.as_mut(),
            _ => coordinate(&mut self.p, "p", path),
        }
    }
}

impl Input for CircleCreate {
    /// `c.x`, `c.y`, `r`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "r" => Some(&mut self.r),
            "lineWeight" => self.line_weight.as_mut(),
            _ => coordinate(&mut self.c, "c", path),
        }
    }
}

impl Input for ArcCreate {
    /// `c.x`, `c.y`, `r`, `a0`, `a1`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "r" => Some(&mut self.r),
            "lineWeight" => self.line_weight.as_mut(),
            "a0" => Some(&mut self.a0),
            "a1" => Some(&mut self.a1),
            _ => coordinate(&mut self.c, "c", path),
        }
    }
}

impl Input for EntitiesTransform {
    /// `transform.dx`, `transform.center.x`, `transform.angle`, `transform.factor`, `transform.a.y` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let rest = path.strip_prefix("transform.")?;
        match &mut self.transform {
            Transform::Move { dx, dy } => match rest {
                "dx" => Some(dx),
                "dy" => Some(dy),
                _ => None,
            },
            Transform::Rotate { center, angle } => match rest {
                "angle" => Some(angle),
                _ => coordinate(center, "center", rest),
            },
            Transform::Scale { center, factor } => match rest {
                "factor" => Some(factor),
                _ => coordinate(center, "center", rest),
            },
            Transform::Mirror { a, b } => {
                coordinate(a, "a", rest).or_else(|| coordinate(b, "b", rest))
            }
            Transform::Align {
                source,
                target,
                source2,
                target2,
                ..
            } => coordinate(source, "source", rest)
                .or_else(|| coordinate(target, "target", rest))
                .or_else(|| coordinate(source2.as_mut()?, "source2", rest))
                .or_else(|| coordinate(target2.as_mut()?, "target2", rest)),
            // Oturt (docs/adr/0156): `transform.from.x`, `transform.a`, `transform.m[2]`, `transform.h[6]` …
            Transform::Similarity { from, to, a, b } => match rest {
                "a" => Some(a),
                "b" => Some(b),
                _ => coordinate(from, "from", rest).or_else(|| coordinate(to, "to", rest)),
            },
            Transform::Affine { from, to, m } => indexed(m, "m", rest)
                .or_else(|| coordinate(from, "from", rest))
                .or_else(|| coordinate(to, "to", rest)),
            Transform::Projective { from, to, h } => indexed(h, "h", rest)
                .or_else(|| coordinate(from, "from", rest))
                .or_else(|| coordinate(to, "to", rest)),
            // Kauçuk levha (docs/adr/0158): `transform.links[1].to.x` …
            Transform::Rubbersheet { links } => {
                let (at, field) = rest.strip_prefix("links[")?.split_once("].")?;
                let link = links.get_mut(at.parse::<usize>().ok()?)?;
                coordinate(&mut link.from, "from", field)
                    .or_else(|| coordinate(&mut link.to, "to", field))
            }
            // Hizala ve dağıt (docs/adr/0194): `transform.at`.
            Transform::Arrange { at, .. } => match rest {
                "at" => at.as_mut(),
                _ => None,
            },
        }
    }
}

/// `name[i]` of a fixed list of numbers.
fn indexed<'a>(values: &'a mut [f64], name: &str, path: &str) -> Option<&'a mut f64> {
    let i: usize = path
        .strip_prefix(name)?
        .strip_prefix('[')?
        .strip_suffix(']')?
        .parse()
        .ok()?;
    values.get_mut(i)
}

impl Input for EntitiesArray {
    /// `layout.dx`, `layout.dy`, `layout.center.x`, `layout.fill`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let rest = path.strip_prefix("layout.")?;
        match &mut self.layout {
            ArrayLayout::Grid { dx, dy, .. } => match rest {
                "dx" => Some(dx),
                "dy" => Some(dy),
                _ => None,
            },
            ArrayLayout::Polar { center, fill, .. } => match rest {
                "fill" => Some(fill),
                _ => coordinate(center, "center", rest),
            },
            ArrayLayout::Path { spacing, .. } => match rest {
                "spacing" => spacing.as_mut(),
                _ => None,
            },
        }
    }
}

impl Input for EntitiesEdit {
    /// `changes[0].geometry.a.x`, `changes[1].geometry.r`, `changes[0].geometry.pts[1].y`,
    /// `changes[2].geometry.bulges[0]` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        // An add's own line weight (docs/adr/0144): `changes[0].lineWeight`.
        if let Some(i) = path
            .strip_prefix("changes[")
            .and_then(|rest| rest.strip_suffix("].lineWeight"))
        {
            return match self.changes.get_mut(i.parse::<usize>().ok()?)? {
                EntityEdit::Add { line_weight, .. } => line_weight.as_mut(),
                _ => None,
            };
        }
        let (i, rest) = path.strip_prefix("changes[")?.split_once("].geometry.")?;
        let geometry = match self.changes.get_mut(i.parse::<usize>().ok()?)? {
            EntityEdit::Update { geometry, .. }
            | EntityEdit::Replace { geometry, .. }
            | EntityEdit::Add { geometry, .. } => geometry,
            EntityEdit::Remove { .. } => return None,
        };
        geometry_number(geometry, rest)
    }
}

impl Input for EntitiesCreate {
    /// `objects[0].geometry.major.x`, `objects[1].geometry.pts[2].y`, `objects[0].geometry.ratio`,
    /// `objects[1].lineWeight`, `objects[0].labelScale` (which the step's input must give) …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let (i, rest) = path.strip_prefix("objects[")?.split_once("].")?;
        let object = self.objects.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "lineWeight" => object.line_weight.as_mut(),
            "labelScale" => object.label_scale.as_mut(),
            _ => geometry_number(&mut object.geometry, rest.strip_prefix("geometry.")?),
        }
    }
}

/// An elevation `zs[i]` written with a geometry (docs/adr/0142): only one
/// given as a number can be made non-finite.
fn elevation_number<'a>(zs: &'a mut Option<Vec<Option<f64>>>, rest: &str) -> Option<&'a mut f64> {
    let i: usize = rest.strip_prefix("zs[")?.strip_suffix(']')?.parse().ok()?;
    zs.as_mut()?.get_mut(i)?.as_mut()
}

/// The number of a geometry at `rest`: `a.x`, `r`, `pts[1].y`, `bulges[0]`,
/// `major.y`, `ratio`, `dir.x`, a text's `height`, `ring[2].x`, `pattern.angle` …
fn geometry_number<'a>(geometry: &'a mut EntityGeometry, rest: &str) -> Option<&'a mut f64> {
    match geometry {
        EntityGeometry::Point { p, z, .. } => match rest {
            "z" => z.as_mut(),
            _ => coordinate(p, "p", rest),
        },
        EntityGeometry::Line { a, b, zs } => {
            if rest.starts_with("zs[") {
                return elevation_number(zs, rest);
            }
            coordinate(a, "a", rest).or_else(|| coordinate(b, "b", rest))
        }
        EntityGeometry::Polyline {
            pts, bulges, zs, ..
        } => {
            if rest.starts_with("pts[") {
                point_number(pts, rest)
            } else if rest.starts_with("zs[") {
                elevation_number(zs, rest)
            } else {
                bulge_number(bulges, rest)
            }
        }
        EntityGeometry::Polygon {
            pts,
            bulges,
            zs,
            parts,
            ..
        } => {
            // A part's own numbers: `parts[0].pts[1].x`, `parts[0].bulges[2]`, `parts[0].zs[1]` (docs/adr/0143).
            if let Some(after) = rest.strip_prefix("parts[") {
                let (k, inner) = after.split_once("].")?;
                let part = parts.as_mut()?.get_mut(k.parse::<usize>().ok()?)?;
                return if inner.starts_with("pts[") {
                    point_number(&mut part.pts, inner)
                } else if inner.starts_with("zs[") {
                    elevation_number(&mut part.zs, inner)
                } else {
                    bulge_number(&mut part.bulges, inner)
                };
            }
            if rest.starts_with("pts[") {
                point_number(pts, rest)
            } else if rest.starts_with("zs[") {
                elevation_number(zs, rest)
            } else {
                bulge_number(bulges, rest)
            }
        }
        EntityGeometry::Circle { c, r } => match rest {
            "r" => Some(r),
            _ => coordinate(c, "c", rest),
        },
        EntityGeometry::Arc { c, r, a0, a1 } => match rest {
            "r" => Some(r),
            "a0" => Some(a0),
            "a1" => Some(a1),
            _ => coordinate(c, "c", rest),
        },
        EntityGeometry::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => match rest {
            "ratio" => Some(ratio),
            "t0" => Some(t0),
            "t1" => Some(t1),
            _ => coordinate(c, "c", rest).or_else(|| coordinate(major, "major", rest)),
        },
        EntityGeometry::Spline { pts, .. } => point_number(pts, rest),
        EntityGeometry::Xline { p, dir } | EntityGeometry::Ray { p, dir } => {
            coordinate(p, "p", rest).or_else(|| coordinate(dir, "dir", rest))
        }
        EntityGeometry::Text {
            p,
            height,
            rotation,
            width_factor,
            box_width,
            line_spacing,
            face,
            ..
        } => match rest {
            "height" => Some(height),
            "rotation" => Some(rotation),
            // The width factor the case gives (docs/adr/0145).
            "widthFactor" => width_factor.as_mut(),
            // A multi-line text's box and spacing the case gives (docs/adr/0182).
            "boxWidth" => box_width.as_mut(),
            "lineSpacing" => line_spacing.as_mut(),
            // A text's slant the case gives (docs/adr/0183).
            "oblique" => face.oblique.as_mut(),
            _ => coordinate(p, "p", rest),
        },
        // A slope's elevations and an arc length's or jogged radius's centre too (docs/adr/0147).
        EntityGeometry::Dimension {
            a,
            b,
            offset,
            height,
            c,
            za,
            zb,
            look,
            ..
        } => match rest {
            "offset" => Some(offset),
            "height" => Some(height),
            "za" => za.as_mut(),
            "zb" => zb.as_mut(),
            // A dimension's sizes the case gives (docs/adr/0183).
            "arrowSize" => look.arrow_size.as_mut(),
            "extOffset" => look.ext_offset.as_mut(),
            "extBeyond" => look.ext_beyond.as_mut(),
            "textGap" => look.text_gap.as_mut(),
            _ => coordinate(a, "a", rest)
                .or_else(|| coordinate(b, "b", rest))
                .or_else(|| c.as_mut().and_then(|c| coordinate(c, "c", rest))),
        },
        EntityGeometry::Insert {
            p, scale, rotation, ..
        } => match rest {
            "scale" => Some(scale),
            "rotation" => Some(rotation),
            _ => coordinate(p, "p", rest),
        },
        // A leader's height, turn and vertices (docs/adr/0146).
        EntityGeometry::Leader {
            pts,
            height,
            rotation,
            ..
        } => match rest {
            "height" => Some(height),
            "rotation" => Some(rotation),
            _ => point_number(pts, rest),
        },
        // A table's sizes and corner (docs/adr/0184): `rows[1]`, `columns[0]`, `height`, `p.x` ….
        // A picture's corner, size, turn and opacity (docs/adr/0192).
        EntityGeometry::Image(i) => match rest {
            "width" => Some(&mut i.width),
            "height" => Some(&mut i.height),
            "rotation" => Some(&mut i.rotation),
            "opacity" => i.opacity.as_mut(),
            _ => coordinate(&mut i.p, "p", rest),
        },
        // A raster's affine, its look's numbers and its opacity (docs/adr/0204): `affine[3]`, `style.min` ….
        EntityGeometry::Raster(r) => match rest {
            "opacity" => r.opacity.as_mut(),
            "style.min" => r.style.min.as_mut(),
            "style.max" => r.style.max.as_mut(),
            "style.azimuth" => r.style.azimuth.as_mut(),
            "style.altitude" => r.style.altitude.as_mut(),
            "style.zFactor" => r.style.z_factor.as_mut(),
            "style.nodata" => r.style.nodata.as_mut(),
            _ => {
                let i: usize = rest
                    .strip_prefix("affine[")?
                    .strip_suffix(']')?
                    .parse()
                    .ok()?;
                r.affine.get_mut(i)
            }
        },
        // A point cloud's bounds, its files', its look's numbers and its opacity
        // (docs/adr/0207 §3): `bounds[2]`, `sources[0].bounds[5]`, `style.size` ….
        EntityGeometry::PointCloud(c) => match rest {
            "opacity" => c.opacity.as_mut(),
            "style.min" => c.style.min.as_mut(),
            "style.max" => c.style.max.as_mut(),
            "style.size" => Some(&mut c.style.size),
            _ => {
                let index = |text: &str| -> Option<usize> {
                    text.strip_prefix("bounds[")?
                        .strip_suffix(']')?
                        .parse()
                        .ok()
                };
                if let Some(i) = index(rest) {
                    return c.bounds.get_mut(i);
                }
                let (k, inner) = rest.strip_prefix("sources[")?.split_once("].")?;
                let k: usize = k.parse().ok()?;
                c.sources.get_mut(k)?.bounds.get_mut(index(inner)?)
            }
        },
        EntityGeometry::Table {
            p,
            rotation,
            height,
            rows,
            columns,
            ..
        } => match rest {
            "height" => Some(height),
            "rotation" => Some(rotation),
            _ => {
                let item = |list: &'a mut Vec<f64>, name: &str| {
                    let i: usize = rest
                        .strip_prefix(name)?
                        .strip_prefix('[')?
                        .strip_suffix(']')?
                        .parse()
                        .ok()?;
                    list.get_mut(i)
                };
                if rest.starts_with("rows[") {
                    item(rows, "rows")
                } else if rest.starts_with("columns[") {
                    item(columns, "columns")
                } else {
                    coordinate(p, "p", rest)
                }
            }
        },
        EntityGeometry::Hatch { ring, pattern, .. } => match rest {
            "pattern.angle" => Some(&mut pattern.angle),
            "pattern.spacing" => Some(&mut pattern.spacing),
            _ => {
                let (i, axis) = rest.strip_prefix("ring[")?.split_once("].")?;
                let p = ring.get_mut(i.parse::<usize>().ok()?)?;
                match axis {
                    "x" => Some(&mut p.x),
                    "y" => Some(&mut p.y),
                    _ => None,
                }
            }
        },
    }
}

/// Puts NaN or ±∞ into the typed input at the paths the step's `nonFinite` names.
fn put_non_finite(input: &mut impl Input, step: &Value, at: &str) -> Outcome<()> {
    let Some(table) = step.get("nonFinite") else {
        return Ok(());
    };
    let table = table
        .as_object()
        .ok_or_else(|| format!("{at}: nonFinite bir nesne olmalı"))?;
    for (path, value) in table {
        let number = match value.as_str() {
            Some("NaN") => f64::NAN,
            Some("Infinity") => f64::INFINITY,
            Some("-Infinity") => f64::NEG_INFINITY,
            _ => {
                return Err(format!(
                    "{at}: {path}: NaN, Infinity ya da -Infinity olmalı"
                ));
            }
        };
        *input
            .number(path)
            .ok_or_else(|| format!("{at}: girdide böyle bir yer yok: {path}"))? = number;
    }
    Ok(())
}

/// Runs `op` of the fixture's command on the document; its whole result as the wire carries it.
fn run_op(
    command: &str,
    op: &str,
    doc: &mut Document,
    input: Value,
    step: &Value,
    at: &str,
) -> Outcome<Value> {
    // The step's input, read as the command's type, with its non-finite numbers put in.
    macro_rules! run {
        ($module:ident, $input:ty) => {{
            let mut input: $input =
                serde_json::from_value(input).map_err(|e| format!("{at}: girdi okunamadı: {e}"))?;
            put_non_finite(&mut input, step, at)?;
            match op {
                "validate" => {
                    serde_json::to_value($module::validate(&ExecutionContext::new(doc), &input))
                }
                "plan" => serde_json::to_value($module::plan(&ExecutionContext::new(doc), &input)),
                _ => serde_json::to_value($module::execute(&mut ExecutionContext::new(doc), input)),
            }
        }};
    }
    match command {
        CAD_POLYGON_CREATE => run!(polygon, PolygonCreate),
        CAD_LINE_CREATE => run!(line, LineCreate),
        CAD_POLYLINE_CREATE => run!(polyline, PolylineCreate),
        CAD_ENTITIES_DELETE => run!(delete, EntitiesDelete),
        CAD_POINT_CREATE => run!(point, PointCreate),
        CAD_CIRCLE_CREATE => run!(circle, CircleCreate),
        CAD_ARC_CREATE => run!(arc, ArcCreate),
        CAD_ENTITIES_TRANSFORM => run!(transform, EntitiesTransform),
        CAD_ENTITIES_EDIT => run!(edit, EntitiesEdit),
        CAD_ENTITIES_ARRAY => run!(array, EntitiesArray),
        kentos_domain::contracts::CAD_ENTITIES_CREATE => run!(create, EntitiesCreate),
        CAD_ENTITIES_SET => run!(set, EntitiesSetProperties),
        CAD_BLOCKS_DEFINE => run!(blocks_define, BlocksDefine),
        CAD_BLOCKS_EDIT => run!(blocks_edit, BlocksEdit),
        kentos_domain::contracts::CAD_LAYERS_SERVICE => run!(layers_service, LayersService),
        kentos_domain::contracts::CAD_NETWORK_DEFINE => run!(network_define, NetworkDefine),
        kentos_domain::contracts::CAD_LAYERS_TIME => run!(layers_time, LayersTime),
        kentos_domain::contracts::CAD_SCENARIOS_EDIT => run!(scenarios_edit, ScenariosEdit),
        kentos_domain::contracts::CAD_LAYERS_FILTER => run!(layers_filter, LayersFilter),
        other => return Err(format!("{at}: {other} için koşucu yok")),
    }
    .map_err(|e| format!("{at}: sonuç yazılamadı: {e}"))
}

/// Runs one command step and compares its whole result.
fn run_command(
    command: &str,
    doc: &mut Document,
    state: &mut State,
    step: &Value,
    at: &str,
) -> Outcome<()> {
    let op = step["op"].as_str().unwrap_or("?");
    let input = fill(&step["input"], doc, state, at)?;
    let before = layer_ids(doc.layers().nodes());
    let got = run_op(command, op, doc, input, step, at)?;
    let Some(want) = step.get("result") else {
        return Err(format!("{at}: komut adımında “result” yok"));
    };
    let mut want = want.clone();
    // “$uid”: the persistent id of the object the command wrote, lowercase with hyphens.
    if want.pointer("/output/uid") == Some(&json!("$uid")) {
        let text = got
            .pointer("/output/uid")
            .and_then(Value::as_str)
            .unwrap_or("");
        let slot = got.pointer("/output/id").and_then(Value::as_u64);
        let uid =
            Uuid::parse_str(text).map_err(|_| format!("{at}: kimlik UUID değil: “{text}”"))?;
        let own = slot
            .and_then(|s| u32::try_from(s).ok())
            .and_then(|s| doc.uid(Slot(s)));
        if uid.to_string() != text || own != Some(uid) {
            return Err(format!(
                "{at}: output.uid {text}, yuvadaki nesnenin kimliği {own:?}"
            ));
        }
        if let Some(field) = want.pointer_mut("/output/uid") {
            *field = Value::String(text.to_owned());
        }
    }
    // “$layer:K”: the layers the command added (execute) or would add (plan), ids the tree did not have, by their number
    // (docs/adr/0210 §11); a command that adds none leaves the last ones.
    let fresh = fresh_layers(&got, &before);
    if !fresh.is_empty() {
        let after = layer_ids(doc.layers().nodes());
        if let Some(id) = fresh
            .iter()
            .find(|id| after.contains(*id) != (op == "execute"))
        {
            return Err(format!(
                "{at}: “{id}” yeni katman ağaçta {}",
                if op == "execute" { "yok" } else { "var" }
            ));
        }
        state.fresh = fresh;
    }
    // “$layer”: the layer the command added (execute) or would add (plan): an id the tree did not have (docs/adr/0208 §15).
    for pointer in ["/output/layer", "/output/node/id"] {
        if want.pointer(pointer) != Some(&json!("$layer")) {
            continue;
        }
        let id = got.pointer(pointer).and_then(Value::as_str).unwrap_or("");
        let after = layer_ids(doc.layers().nodes());
        let fresh = id
            .strip_prefix("layer-")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            && !before.contains(id)
            && after.contains(id) == (op == "execute");
        if !fresh {
            return Err(format!(
                "{at}: {pointer}: “{id}” yeni bir katmanın kimliği değil"
            ));
        }
        state.layer = Some(id.to_owned());
    }
    expect_same(&got, &fill(&want, doc, state, at)?, "sonuç", at)
}

/// The `layer-N` ids in a result the tree did not have before, by N.
fn fresh_layers(got: &Value, before: &std::collections::HashSet<String>) -> Vec<String> {
    fn walk(v: &Value, before: &std::collections::HashSet<String>, out: &mut Vec<(u64, String)>) {
        match v {
            Value::String(s) => {
                if let Some(n) = s.strip_prefix("layer-").and_then(|n| n.parse::<u64>().ok())
                    && !before.contains(s)
                    && !out.iter().any(|(_, o)| o == s)
                {
                    out.push((n, s.clone()));
                }
            }
            Value::Array(items) => items.iter().for_each(|i| walk(i, before, out)),
            Value::Object(fields) => fields.values().for_each(|f| walk(f, before, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(got, before, &mut out);
    out.sort_by_key(|(n, _)| *n);
    out.into_iter().map(|(_, s)| s).collect()
}

/// Every node's id of a layer tree.
fn layer_ids(nodes: &[kentos_domain::contracts::LayerNode]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let mut stack: Vec<&kentos_domain::contracts::LayerNode> = nodes.iter().collect();
    while let Some(n) = stack.pop() {
        out.insert(n.id.clone());
        stack.extend(n.children.iter());
    }
    out
}

fn run_step(
    command: &str,
    doc: &mut Document,
    state: &mut State,
    step: &Value,
    at: &str,
) -> Outcome<()> {
    if let Some(fields) = step.as_object() {
        // A misspelt field would otherwise pass unchecked.
        if let Some(key) = fields.keys().find(|k| !STEP_KEYS.contains(&k.as_str())) {
            return Err(format!("{at}: bilinmeyen alan “{key}”"));
        }
    }
    let before = doc.revision();
    match step["op"].as_str().unwrap_or("?") {
        "validate" | "plan" | "execute" => run_command(command, doc, state, step, at)?,
        op @ ("undo" | "redo") => {
            let label = if op == "undo" { doc.undo() } else { doc.redo() };
            if let Some(want) = step.get("returns") {
                expect_same(&json!(label), want, "dönen değer", at)?;
            }
        }
        "captureRevision" => {
            let name = step["as"].as_str().ok_or(format!("{at}: “as” yok"))?;
            state
                .revisions
                .insert(name.to_owned(), doc.revision().to_string());
        }
        "captureBlock" => {
            let name = step["as"].as_str().ok_or(format!("{at}: “as” yok"))?;
            let block = step["name"].as_str().ok_or(format!("{at}: “name” yok"))?;
            let id = doc
                .block_named(block)
                .map(|b| b.id)
                .ok_or(format!("{at}: “{block}” bloğu yok"))?;
            state.blocks.insert(name.to_owned(), id);
        }
        "captureUid" => {
            let name = step["as"].as_str().ok_or(format!("{at}: “as” yok"))?;
            let id = step["id"].as_u64().and_then(|n| u32::try_from(n).ok());
            let uid = id
                .and_then(|id| doc.uid(Slot(id)))
                .ok_or(format!("{at}: {id:?} nesnesi yok"))?;
            state.uids.insert(name.to_owned(), uid);
        }
        other => return Err(format!("{at}: bilinmeyen işlem “{other}”")),
    }
    check(doc, state, step.get("expect"), before, at)
}

fn check(
    doc: &Document,
    state: &State,
    expect: Option<&Value>,
    before: u64,
    at: &str,
) -> Outcome<()> {
    let Some(expect) = expect else {
        return Ok(());
    };
    let expect = expect
        .as_object()
        .ok_or(format!("{at}: expect bir nesne olmalı"))?;
    let empty = serde_json::Map::new();
    for (key, want) in expect {
        match key.as_str() {
            "ids" => {
                let ids: Vec<u32> = doc.entities().map(|e| e.base().id).collect();
                expect_same(&json!(ids), want, "nesneler", at)?;
            }
            "entities" => {
                for (id, entity) in want.as_object().unwrap_or(&empty) {
                    let slot = id
                        .parse::<u32>()
                        .map(Slot)
                        .map_err(|_| format!("{at}: {id}"))?;
                    let got = doc
                        .get(slot)
                        .map(serde_json::to_value)
                        .transpose()
                        .map_err(|e| format!("{at}: {e}"))?
                        .unwrap_or(Value::Null);
                    let entity = block_ids(entity, doc, state, at)?;
                    expect_same(&got, &entity, &format!("nesne {id}"), at)?;
                }
            }
            "canUndo" => expect_same(&json!(doc.can_undo()), want, "canUndo", at)?,
            "canRedo" => expect_same(&json!(doc.can_redo()), want, "canRedo", at)?,
            "dirty" => expect_same(&json!(doc.is_dirty()), want, "dirty", at)?,
            "revision" => {
                let got = if doc.revision() == before {
                    "same"
                } else {
                    "changed"
                };
                expect_same(&json!(got), want, "sürüm", at)?;
            }
            "uids" => {
                for (id, name) in want.as_object().unwrap_or(&empty) {
                    let slot = id
                        .parse::<u32>()
                        .map(Slot)
                        .map_err(|_| format!("{at}: {id}"))?;
                    let uid = doc
                        .uid(slot)
                        .ok_or(format!("{at}: {id} nesnesinin kalıcı kimliği yok"))?;
                    match name.as_str() {
                        Some("new") if state.uids.values().any(|u| *u == uid) => {
                            return Err(format!("{at}: {id} nesnesinin kimliği yeni değil"));
                        }
                        Some("new") => {}
                        Some(name) if state.uids.get(name) == Some(&uid) => {}
                        Some(name) => {
                            return Err(format!(
                                "{at}: {id} nesnesinin kimliği {uid}, “{name}” {:?}",
                                state.uids.get(name)
                            ));
                        }
                        None => return Err(format!("{at}: kimlik adı metin olmalı")),
                    }
                }
            }
            // The drawing's definitions in order, each without its id (docs/adr/0144).
            "blocks" => {
                let got: Vec<Value> = doc
                    .blocks()
                    .iter()
                    .map(|b| {
                        let mut v = serde_json::to_value(&**b).map_err(|e| format!("{at}: {e}"))?;
                        if let Some(fields) = v.as_object_mut() {
                            fields.remove("id");
                        }
                        Ok(v)
                    })
                    .collect::<Outcome<_>>()?;
                expect_same(&json!(got), want, "bloklar", at)?;
            }
            // A definition's id by its name: "new" (none the setup had nor
            // `captureBlock` took), "$block:name" or the id itself.
            "blockIds" => {
                for (name, id) in want.as_object().unwrap_or(&empty) {
                    let got = doc
                        .block_named(name)
                        .map(|b| b.id)
                        .ok_or(format!("{at}: “{name}” bloğu yok"))?;
                    match id.as_str() {
                        Some("new")
                            if state.setup_blocks.contains(&got)
                                || state.blocks.values().any(|b| *b == got) =>
                        {
                            return Err(format!("{at}: “{name}” bloğunun kimliği yeni değil"));
                        }
                        Some("new") => {}
                        Some(text) => {
                            let wanted = fill(&json!(text), doc, state, at)?;
                            expect_same(
                                &json!(got.to_text()),
                                &wanted,
                                &format!("“{name}” bloğunun kimliği"),
                                at,
                            )?;
                        }
                        None => return Err(format!("{at}: blok kimliği metin olmalı")),
                    }
                }
            }
            // The layer tree, every node whole; `$layer` the one a command added (docs/adr/0208 §15).
            "layers" => {
                let got =
                    serde_json::to_value(doc.layers().nodes()).map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, &fill(want, doc, state, at)?, "katmanlar", at)?;
            }
            // The project's connections (docs/adr/0208 §2).
            "connections" => {
                let got = serde_json::to_value(&doc.settings().connections)
                    .map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, want, "bağlantılar", at)?;
            }
            // The project's networks (docs/adr/0209 §2).
            "networks" => {
                let got = serde_json::to_value(&doc.settings().networks)
                    .map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, want, "ağlar", at)?;
            }
            // A misspelt expectation would otherwise pass unchecked.
            other => return Err(format!("{at}: bilinmeyen beklenti “{other}”")),
        }
    }
    Ok(())
}

fn run_case(command: &str, setup: &Value, case: &Value, at: &str) -> Outcome<()> {
    let snapshot = DocumentSnapshotV1::from_json(&setup.to_string())
        .map_err(|e| format!("{at}: kurulum: {e}"))?;
    let mut doc = Document::from_snapshot(snapshot).map_err(|e| format!("{at}: kurulum: {e}"))?;
    if doc.is_dirty() || doc.can_undo() {
        return Err(format!("{at}: kurulumdan sonra belge temiz değil"));
    }
    let steps = case["steps"].as_array().ok_or(format!("{at}: adım yok"))?;
    if steps.is_empty() {
        return Err(format!("{at}: adım yok"));
    }
    let mut state = State {
        setup_blocks: doc.blocks().iter().map(|b| b.id).collect(),
        ..State::default()
    };
    for (i, step) in steps.iter().enumerate() {
        let op = step["op"].as_str().unwrap_or("?");
        run_step(
            command,
            &mut doc,
            &mut state,
            step,
            &format!("{at} › {} {op}", i + 1),
        )?;
    }
    Ok(())
}

/// Runs every file of `dir`: each a desktop command's, with at least `least`
/// cases. Answers the commands they were for, the cases run and what failed.
fn run_folder(dir: &std::path::Path, least: usize) -> (Vec<String>, usize, Vec<String>) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "{}: no files", dir.display());
    let mut problems = Vec::new();
    let mut cases = 0;
    let mut commands = Vec::new();
    for path in &files {
        let file = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let fixture: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
                .expect("JSON");
        assert_eq!(fixture["format"], "kentos.command-cases", "{file}");
        assert_eq!(fixture["version"], 1, "{file}");
        let command = (
            fixture["command"].as_str().unwrap_or(""),
            fixture["commandVersion"].as_u64().unwrap_or(0),
        );
        assert!(
            DESKTOP_COMMANDS
                .iter()
                .any(|(id, v)| (*id, u64::from(*v)) == command),
            "{file}: {command:?} is not a desktop command"
        );
        commands.push(command.0.to_owned());
        let listed = fixture["cases"].as_array().expect("cases");
        assert!(listed.len() >= least, "{file}: {} cases", listed.len());
        for case in listed {
            cases += 1;
            let name = case["name"].as_str().unwrap_or("?");
            let setup = case.get("setup").unwrap_or(&fixture["setup"]);
            if let Err(problem) = run_case(command.0, setup, case, &format!("{file} › {name}")) {
                problems.push(problem);
            }
        }
    }
    (commands, cases, problems)
}

#[test]
fn every_command_case_matches_the_desktop_handlers() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/commands/v1");
    let (commands, cases, problems) = run_folder(&dir, 20);
    // Every command the desktop runs has its cases.
    for (id, _) in DESKTOP_COMMANDS {
        assert!(
            commands.iter().any(|c| c == id),
            "{id}: no file in fixtures/commands/v1"
        );
    }
    assert!(
        problems.is_empty(),
        "{} of {cases} cases failed:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// The desktop's own cases (fixtures/commands/v1/desktop, docs/adr/0207 §10):
/// point clouds, which only the desktop adds yet. The web's runner reads the
/// folder above only.
#[test]
fn the_desktops_own_cases_match_its_handlers() {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/commands/v1/desktop");
    let (_, cases, problems) = run_folder(&dir, 1);
    assert!(
        problems.is_empty(),
        "{} of {cases} cases failed:\n{}",
        problems.len(),
        problems.join("\n")
    );
}
