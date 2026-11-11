//! Planı tanı (plan/architecture-advanced.md M48, docs/adr/8048, `cad.arch.recognize` v1): an exploded plan read
//! back as walls and their openings, one undo step “Planı tanı”. The geometry is
//! `kentos_geometry_core::ops::plan_recognition`'s; here the selection's objects become its lines by their layers'
//! roles, and what it reads becomes walls (`walls::build`, as a drawn wall is checked) and openings of KentOS's types
//! (`openings::fields_of`, `openings::placed`, as a placed one is). The shared cases are
//! `fixtures/commands/v1/cad.arch.recognize.json`.

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::LayerNodeType;
use kentos_commands::{
    ArchInfo, ArchRecognize, ArchRecognizePlan, ArchRecognized, ColumnEntity, ColumnFields,
    ColumnShape, CommandResult, CommandWarning, Entity, EntityBase, NewOpening, NewWall,
    OpeningForm, OpeningHand, OpeningSide, OpeningTypeDef, PlanLayerRole, PlanRole, Vec2,
    WallEntity, WallJustify, WallStyleDef, opening_type,
};
use kentos_domain::{Document, Slot, Uuid, opening_follow};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::block::Blocks;
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::geom::arc::sweep;
use kentos_geometry_core::geom::bulge::bulge_arc;
use kentos_geometry_core::jsmath::{PI, atan2, cos, js_hypot, js_round, sin};
use kentos_geometry_core::ops::curve_cuts::Cut;
use kentos_geometry_core::ops::plan_recognition::{self as read, Role};

use crate::ExecutionContext;
use crate::arch_elements;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::shape;
use crate::openings;
use crate::walls;

pub const STEP: &str = "Planı tanı";

/// A layer's name folded for matching: lower case, Turkish letters as their Latin bases.
fn fold(name: &str) -> String {
    name.to_lowercase()
        .replace("i\u{307}", "i")
        .chars()
        .map(|c| match c {
            'ı' => 'i',
            'ş' => 's',
            'ğ' => 'g',
            'ü' => 'u',
            'ö' => 'o',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// A layer's role from its name, as offices name them in Turkish and English (AIA's A-WALL, ISO 13567's codes): a
/// word starting duvar, perde, bölme or wall is a wall's; kapı or door a door's; pencere, window, glaz, camekan or cam
/// a window's; kolon, column, col or cols a column's. A word of what is no wall, door, window or column first (a
/// hatch, a dimension, a text, an axis, a stair, furniture, a sheet's frame) makes it ignored: a wall's hatch drawn on
/// DUVAR-TARAMA is no wall. `None`: the name says nothing ("0", "Katman1").
pub fn guess_role(name: &str) -> Option<PlanRole> {
    let folded = fold(name);
    let words: Vec<&str> = folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let starts = |keys: &[&str]| words.iter().any(|w| keys.iter().any(|k| w.starts_with(k)));
    let is = |keys: &[&str]| words.iter().any(|w| keys.contains(w));
    if starts(&[
        "tarama",
        "hatch",
        "olcu",
        "dim",
        "yazi",
        "text",
        "anno",
        "mobilya",
        "furn",
        "esya",
        "tefris",
        "vitrifiye",
        "merdiven",
        "stair",
        "basamak",
        "rampa",
        "ramp",
        "korkuluk",
        "rail",
        "peyzaj",
        "agac",
        "kesit",
        "cerceve",
        "pafta",
        "antet",
        "defpoint",
    ]) || is(&["aks", "akslar", "axis", "grid", "kot", "not"])
    {
        Some(PlanRole::Ignore)
    } else if starts(&["duvar", "wall", "perde", "bolme"]) {
        Some(PlanRole::Wall)
    } else if starts(&["kapi", "door"]) {
        Some(PlanRole::Door)
    } else if starts(&["pencere", "window", "glaz", "camekan"]) || is(&["cam"]) {
        Some(PlanRole::Window)
    } else if starts(&["kolon", "column"]) || is(&["col", "cols"]) {
        Some(PlanRole::Column)
    } else {
        None
    }
}

/// Why an object of the plan was not read.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Unread {
    /// Its layer has no role.
    Ignored,
    /// A block that does not open.
    Block,
    /// A column layer's object no column is made of (an L, a hatch outside a column).
    Column,
    /// No wall's, door's or window's.
    Other,
}

/// What the call writes: the walls, openings and columns with their layers, the sources taken away and those left, why
/// each left one was, KentOS's styles taken, the warnings.
struct Work {
    walls: Vec<(Uuid, Entity)>,
    openings: Vec<(Uuid, Entity)>,
    columns: Vec<(Uuid, Entity)>,
    counts: [u32; 3],
    removed: Vec<String>,
    left: Vec<String>,
    added: Vec<WallStyleDef>,
    warnings: Vec<CommandWarning>,
}

/// The role each of `layers` plays (docs/adr/8048 §2): the one `given`, else its name's; when none is a wall's, every
/// layer whose name says nothing is (a plan drawn on "0"). What Planı tanı's tool shows and its command reads.
pub fn layer_roles<'a>(
    doc: &Document,
    layers: impl IntoIterator<Item = &'a str>,
    given: &[PlanLayerRole],
) -> BTreeMap<String, PlanRole> {
    let tree = doc.layers();
    let mut out: BTreeMap<String, PlanRole> = BTreeMap::new();
    let mut guessed: BTreeMap<String, Option<PlanRole>> = BTreeMap::new();
    for id in layers {
        if let Some(r) = given.iter().find(|r| r.layer_id == id) {
            out.insert(id.to_owned(), r.role);
        } else if !guessed.contains_key(id) {
            let name = tree.get(id).map_or(id, |n| n.name.as_str());
            guessed.insert(id.to_owned(), guess_role(name));
        }
    }
    let any_wall = given.iter().any(|r| r.role == PlanRole::Wall)
        || guessed.values().flatten().any(|r| *r == PlanRole::Wall);
    for (id, role) in guessed {
        let role = match role {
            Some(r) => r,
            // A plan drawn on one layer ("0"): its lines are walls', its arcs doors'.
            None if !any_wall => PlanRole::Wall,
            None => PlanRole::Ignore,
        };
        out.insert(id, role);
    }
    out
}

/// [`layer_roles`] of the objects' layers and their blocks' pieces' (`more`); `layer_not_found` for a role given to a
/// layer not in the drawing.
fn roles<'a>(
    doc: &Document,
    input: &ArchRecognize,
    found: &'a [checks::Named<'_>],
    more: impl IntoIterator<Item = &'a str>,
) -> Result<BTreeMap<String, PlanRole>, Stop> {
    for (i, r) in input.roles.iter().enumerate() {
        if doc.layers().get(&r.layer_id).is_none() {
            return Err(Stop::failed(error(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{}” kimlikli katman çizimde yok. Rolünü verdiğiniz katman çizimin bir katmanı olmalı.",
                    r.layer_id
                ),
                Some(format!("roles[{i}].layerId")),
            )));
        }
    }
    let mut out = layer_roles(
        doc,
        found
            .iter()
            .map(|(_, _, e, _)| e.base().layer_id.as_str())
            .chain(more),
        &input.roles,
    );
    // A role given to a layer none of the objects is on still counts (it says nothing of them).
    for r in &input.roles {
        out.entry(r.layer_id.clone()).or_insert(r.role);
    }
    Ok(out)
}

/// A path's edges: its straight ones as lines, its bulged ones as arcs (counter-clockwise).
fn edges(
    pts: &[V],
    bulges: Option<&[f64]>,
    closed: bool,
    role: Role,
    source: usize,
    lines: &mut read::Lines,
) {
    let n = pts.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    for i in 0..count {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let bulge = bulges.and_then(|b| b.get(i).copied()).unwrap_or(0.0);
        match bulge_arc(a, b, bulge) {
            Some(arc) => {
                let (a0, s) = if arc.sweep >= 0.0 {
                    (arc.a0, arc.sweep)
                } else {
                    (arc.a0 + arc.sweep, -arc.sweep)
                };
                lines.arcs.push(read::Arc {
                    c: arc.c,
                    r: arc.r,
                    a0,
                    sweep: s,
                    role,
                    source,
                });
            }
            None => lines.segments.push(read::Segment { a, b, role, source }),
        }
    }
}

/// A shape's lines, arcs and outlines for the reading, under `role`: an object's, or a block's piece; false when it
/// gives none (a text, a hatch, a circle not a column's).
fn read_shape(s: &Shape, role: Role, source: usize, lines: &mut read::Lines) -> bool {
    let ring = |pts: Vec<V>, lines: &mut read::Lines| {
        lines.rings.push(read::Ring { pts, role, source });
    };
    match s {
        Shape::Line { a, b } => lines.segments.push(read::Segment {
            a: *a,
            b: *b,
            role,
            source,
        }),
        Shape::Polyline {
            pts,
            bulges,
            parts: None,
            closed,
            ..
        } if pts.len() >= 2 => {
            if role == Role::Column && *closed && pts.len() >= 3 {
                ring(pts.clone(), lines);
            } else {
                edges(pts, bulges.as_deref(), *closed, role, source, lines);
            }
        }
        Shape::Polygon {
            pts,
            bulges,
            parts: None,
            ..
        } if pts.len() >= 3 => {
            if role == Role::Column {
                ring(pts.clone(), lines);
            } else {
                edges(pts, bulges.as_deref(), true, role, source, lines);
            }
        }
        Shape::Arc { c, r, a0, a1 } => lines.arcs.push(read::Arc {
            c: *c,
            r: *r,
            a0: *a0,
            sweep: sweep(*a0, *a1),
            role,
            source,
        }),
        // A round column: its outline as sixteen sides.
        Shape::Circle { c, r } if role == Role::Column => {
            let pts = (0..16)
                .map(|k| {
                    let t = f64::from(k) * PI / 8.0;
                    V::new(c.x + r * cos(t), c.y + r * sin(t))
                })
                .collect();
            ring(pts, lines);
        }
        _ => return false,
    }
    true
}

/// A block's pieces as the reading takes them (docs/adr/8048 §6): the insert's definition opened, a nested insert's
/// too (at most eight deep); each piece with its layer: its own when the drawing has it, else the insert's (a piece of
/// the block's layer, "0", draws with the insert's). `None` when the block does not open.
fn block_pieces(
    doc: &Document,
    blocks: &Blocks,
    insert: &CoreEntity,
    layer: &str,
    depth: usize,
    out: &mut Vec<(String, Shape)>,
) -> Option<()> {
    if depth > 8 {
        return Some(());
    }
    let Cut::Pieces(pieces) = blocks.explode(insert) else {
        return None;
    };
    for piece in pieces {
        let own = piece.rest.iter().find_map(|(k, v)| match v {
            Json::Str(s) if k == "layerId" && !s.is_empty() => Some(s.clone()),
            _ => None,
        });
        let layer = own
            .filter(|id| {
                doc.layers()
                    .get(id)
                    .is_some_and(|n| n.kind == LayerNodeType::Layer)
            })
            .unwrap_or_else(|| layer.to_owned());
        if matches!(piece.shape, Shape::Insert { .. }) {
            block_pieces(doc, blocks, &piece, &layer, depth + 1, out)?;
        } else {
            out.push((layer, piece.shape));
        }
    }
    Some(())
}

/// A number to the nanometre, as the plan's reading gives its walls'.
fn clean(x: f64) -> f64 {
    let r = js_round(x * 1e9) / 1e9;
    if r == 0.0 { 0.0 } else { r }
}

/// A column layer's outline as a column reads it: a circle, or a closed straight outline's vertices.
fn outline_of(s: &Shape) -> Option<Result<(V, f64), Vec<V>>> {
    let straight = |b: &Option<Vec<f64>>| b.as_ref().is_none_or(|b| b.iter().all(|x| *x == 0.0));
    match s {
        Shape::Circle { c, r } => Some(Ok((*c, *r))),
        Shape::Polyline {
            pts,
            bulges,
            parts: None,
            closed: true,
            ..
        }
        | Shape::Polygon {
            pts,
            bulges,
            parts: None,
            ..
        } if straight(bulges) => Some(Err(pts.clone())),
        _ => None,
    }
}

/// The column a column layer's outline is (docs/adr/8048 §5): a circle a round one as wide as it; a closed outline of
/// four right angles a rectangular one about its middle, its width the side nearer east–west, turned as that side.
/// Another outline is none.
fn column_of(outline: Result<(V, f64), Vec<V>>, arch: &ArchInfo) -> Option<ColumnFields> {
    let made =
        |at: V, shape: ColumnShape, width: f64, depth: Option<f64>, rotation: f64| ColumnFields {
            at: Vec2 {
                x: clean(at.x),
                y: clean(at.y),
            },
            shape,
            width: clean(width),
            depth: depth.map(clean),
            legs: None,
            rotation: clean(rotation),
            anchor: Default::default(),
            kind: None,
            tag: None,
            fill: true,
            material: None,
            grids: None,
            place: None,
            broken: None,
            arch: arch.clone(),
        };
    let mut pts = match outline {
        Ok((c, r)) => return Some(made(c, ColumnShape::Round, 2.0 * r, None, 0.0)),
        Err(pts) => pts,
    };
    // A vertex drawn twice is one; a closing vertex on the first is the first.
    pts.dedup_by(|p, q| js_hypot(p.x - q.x, p.y - q.y) <= 1e-9);
    if pts.len() > 1
        && let (Some(first), Some(last)) = (pts.first(), pts.last())
        && js_hypot(first.x - last.x, first.y - last.y) <= 1e-9
    {
        pts.pop();
    }
    // Its corners: a point where the outline turns, a straight one left out.
    let n = pts.len();
    let corners: Vec<V> = (0..n)
        .filter(|&i| {
            let (a, b, c) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            let (u, w) = ((b.x - a.x, b.y - a.y), (c.x - b.x, c.y - b.y));
            let turn = u.0 * w.1 - u.1 * w.0;
            turn.abs() > 1e-9 * js_hypot(u.0, u.1) * js_hypot(w.0, w.1)
        })
        .map(|i| pts[i])
        .collect();
    let [a, b, c, d] = corners[..] else {
        return None;
    };
    let side = |p: V, q: V| (q.x - p.x, q.y - p.y);
    let square = |u: (f64, f64), w: (f64, f64)| {
        (u.0 * w.0 + u.1 * w.1).abs() <= 1e-6 * js_hypot(u.0, u.1) * js_hypot(w.0, w.1)
    };
    let sides = [side(a, b), side(b, c), side(c, d), side(d, a)];
    if !(0..4).all(|i| square(sides[i], sides[(i + 1) % 4])) {
        return None;
    }
    // A side's turn within a half turn either way of east.
    let turn = |s: (f64, f64)| {
        let mut t = atan2(s.1, s.0);
        if t > PI / 2.0 {
            t -= PI;
        } else if t <= -PI / 2.0 {
            t += PI;
        }
        t
    };
    let (t0, t1) = (turn(sides[0]), turn(sides[1]));
    let (l0, l1) = (
        js_hypot(sides[0].0, sides[0].1),
        js_hypot(sides[1].0, sides[1].1),
    );
    let (width, depth, rotation) = if t1.abs() < t0.abs() {
        (l1, l0, t1)
    } else {
        (l0, l1, t0)
    };
    let middle = V::new((a.x + b.x + c.x + d.x) / 4.0, (a.y + b.y + c.y + d.y) / 4.0);
    let depth = ((depth - width).abs() > 1e-9).then_some(depth);
    Some(made(middle, ColumnShape::Rect, width, depth, rotation))
}

/// Whether `p` lies in `c`'s outline, within `tol`.
fn in_column(c: &ColumnFields, p: V, tol: f64) -> bool {
    let (dx, dy) = (p.x - c.at.x, p.y - c.at.y);
    if c.shape == ColumnShape::Round {
        return js_hypot(dx, dy) <= c.width / 2.0 + tol;
    }
    let (cos_r, sin_r) = (cos(c.rotation), sin(c.rotation));
    let (along, across) = (dx * cos_r + dy * sin_r, -dx * sin_r + dy * cos_r);
    along.abs() <= c.width / 2.0 + tol && across.abs() <= c.depth.unwrap_or(c.width) / 2.0 + tol
}

/// The KentOS type an opening read is made with: a door as wide as Kapı 80 or 90 (the nearer), İki kanatlı kapı 140
/// with two leaves; a window Pencere 120; a bare opening Boşluk 100.
fn type_of(o: &read::Opening) -> OpeningTypeDef {
    let width = o.to - o.from;
    let id = match o.form {
        read::Form::Door if o.leaves == 2 => "kentos-iki-kanatli-kapi-140",
        read::Form::Door if width < 0.85 => "kentos-kapi-80",
        read::Form::Door => "kentos-kapi-90",
        read::Form::Window => "kentos-pencere-120",
        read::Form::Opening => "kentos-bosluk-100",
    };
    opening_type(id).unwrap_or_else(|| openings::first_type(OpeningForm::Door))
}

fn base(layer: &str) -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: layer.to_owned(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
        look: Default::default(),
    }
}

fn check(doc: &Document, input: &ArchRecognize) -> Result<Work, Stop> {
    checks::uids(&input.uids, "Tanınacak planın nesneleri verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    let (found, locked) = if input.keep_sources {
        (found, None)
    } else {
        checks::skip_locked(doc, found)?
    };
    let mut warnings: Vec<CommandWarning> = locked.into_iter().collect();
    // The blocks' pieces, each with its layer; `None` for a block that does not open.
    let mut pieces: BTreeMap<usize, Option<Vec<(String, Shape)>>> = BTreeMap::new();
    if found
        .iter()
        .any(|(_, _, e, _)| matches!(e, Entity::Insert(_)))
    {
        let blocks = crate::blocks::drawing_blocks(doc);
        for (k, (_, _, e, _)) in found.iter().enumerate() {
            if matches!(e, Entity::Insert(_)) {
                let mut out = Vec::new();
                let opened = block_pieces(
                    doc,
                    &blocks,
                    &crate::blocks::core_entity(e),
                    &e.base().layer_id,
                    0,
                    &mut out,
                );
                pieces.insert(k, opened.map(|()| out));
            }
        }
    }
    let roles = roles(
        doc,
        input,
        &found,
        pieces
            .values()
            .flatten()
            .flatten()
            .map(|(layer, _)| layer.as_str()),
    )?;
    for (value, what, name) in [
        (input.min_width, "minWidth", "En ince duvar"),
        (input.max_width, "maxWidth", "En kalın duvar"),
        (input.max_opening, "maxOpening", "En geniş açıklık"),
        (input.angle, "angle", "Açı"),
    ] {
        if let Some(x) = value {
            checks::finite(x, name, "Sonlu bir sayı verin.", what)?;
        }
    }
    let o = read::Options {
        min_width: input
            .min_width
            .unwrap_or(kentos_commands::DEFAULT_RECOGNIZE_MIN_WIDTH),
        max_width: input
            .max_width
            .unwrap_or(kentos_commands::DEFAULT_RECOGNIZE_MAX_WIDTH),
        max_opening: input
            .max_opening
            .unwrap_or(kentos_commands::DEFAULT_RECOGNIZE_MAX_OPENING),
        angle: input
            .angle
            .unwrap_or(kentos_commands::DEFAULT_RECOGNIZE_ANGLE),
        tol: 0.005,
    };
    let option = |what: &str, words: &str| {
        Stop::failed(error(
            codes::RECOGNIZE_OPTION,
            words.to_owned(),
            Some(what.to_owned()),
        ))
    };
    if o.min_width <= 0.0 {
        return Err(option("minWidth", "En ince duvar sıfırdan kalın olmalı."));
    }
    if o.max_width <= o.min_width {
        return Err(option(
            "maxWidth",
            "En kalın duvar en ince duvardan kalın olmalı.",
        ));
    }
    if o.max_opening <= 0.0 {
        return Err(option(
            "maxOpening",
            "En geniş açıklık sıfırdan geniş olmalı.",
        ));
    }
    if !(o.angle > 0.0 && o.angle <= 45.0) {
        return Err(option("angle", "Açı 0 ile 45° arasında olmalı."));
    }
    if let Some(arch) = &input.arch {
        walls::level_known(doc, arch, "arch.level")?;
    }
    // The plan's lines; why each object not given is.
    let mut lines = read::Lines::default();
    let mut unread: BTreeMap<usize, Unread> = BTreeMap::new();
    // The columns made of the column layers' objects, and those layers' hatches by their middle.
    let mut made_columns: Vec<(usize, ColumnFields)> = Vec::new();
    let mut fills: Vec<(usize, V)> = Vec::new();
    let arch = input.arch.clone().unwrap_or_default();
    let role_of = |layer: &str| match roles.get(layer).copied().unwrap_or(PlanRole::Ignore) {
        PlanRole::Wall => Some(Role::Wall),
        PlanRole::Door => Some(Role::Door),
        PlanRole::Window => Some(Role::Window),
        PlanRole::Column => Some(Role::Column),
        PlanRole::Ignore => None,
    };
    for (k, (_, _, e, _)) in found.iter().enumerate() {
        // A block: its pieces, each by its layer's role; it goes when all of them are read.
        if let Some(opened) = pieces.get(&k) {
            let Some(parts) = opened else {
                unread.insert(k, Unread::Block);
                continue;
            };
            let mut roled = false;
            let mut given = false;
            for (layer, shape) in parts {
                let Some(role) = role_of(layer) else { continue };
                roled = true;
                if role == Role::Column
                    && let Some(c) = outline_of(shape).and_then(|o| column_of(o, &arch))
                {
                    made_columns.push((k, c));
                }
                given |= read_shape(shape, role, k, &mut lines);
            }
            if !roled {
                unread.insert(k, Unread::Ignored);
            } else if !given {
                unread.insert(k, Unread::Other);
            }
            continue;
        }
        let Some(role) = role_of(&e.base().layer_id) else {
            unread.insert(k, Unread::Ignored);
            continue;
        };
        let s = shape(e);
        let given = read_shape(&s, role, k, &mut lines);
        if role == Role::Column {
            match (e, outline_of(&s).and_then(|o| column_of(o, &arch))) {
                (_, Some(c)) => made_columns.push((k, c)),
                // A column's hatch goes with the column it fills.
                (Entity::Hatch(h), None) if !h.ring.is_empty() => {
                    let n = h.ring.len() as f64;
                    let x = h.ring.iter().map(|p| p.x).sum::<f64>() / n;
                    let y = h.ring.iter().map(|p| p.y).sum::<f64>() / n;
                    fills.push((k, V::new(x, y)));
                    unread.insert(k, Unread::Column);
                }
                _ => {
                    unread.insert(k, Unread::Column);
                }
            }
        } else if !given {
            unread.insert(k, Unread::Other);
        }
    }
    let read = read::recognize(&lines, &o);
    if read.walls.is_empty() {
        return Err(Stop::failed(error(
            codes::PLAN_UNRECOGNIZED,
            format!(
                "Seçimde duvar tanınmadı: duvarın iki yüzü {}–{} m arayla paralel iki çizgi olmalı. Duvar çizgilerinin \
                 katmanına Duvar rolünü verin; blokları önce patlatın.",
                o.min_width, o.max_width
            ),
            Some("uids".into()),
        )));
    }
    let layers = doc.layers();
    let first_layer = |role: PlanRole| {
        found
            .iter()
            .map(|(_, _, e, _)| &e.base().layer_id)
            .find(|id| roles.get(*id) == Some(&role))
            .cloned()
    };
    let source_layer = |segment: usize| {
        lines
            .segments
            .get(segment)
            .map(|s| found[s.source].2.base().layer_id.clone())
            .unwrap_or_default()
    };
    let mut added = Vec::new();
    let mut work = Work {
        walls: Vec::new(),
        openings: Vec::new(),
        columns: Vec::new(),
        counts: [0; 3],
        removed: Vec::new(),
        left: Vec::new(),
        added: Vec::new(),
        warnings: Vec::new(),
    };
    let mut lines_of_walls = Vec::new();
    for (k, w) in read.walls.iter().enumerate() {
        let layer = input
            .wall_layer_id
            .clone()
            .unwrap_or_else(|| source_layer(w.line));
        let given = NewWall {
            pts: vec![Vec2 { x: w.a.x, y: w.a.y }, Vec2 { x: w.b.x, y: w.b.y }],
            bulges: None,
            closed: false,
            width: Some(w.width),
            justify: Some(WallJustify::Center),
            offset: None,
            style: None,
            caps: None,
            height: None,
            arch: input.arch.clone(),
            attrs: None,
            label: None,
        };
        let fields = walls::build(doc, &given, &format!("walls[{k}]"), &mut added)?;
        let entity = Entity::Wall(WallEntity {
            base: base(&layer),
            wall: fields,
        });
        lines_of_walls.push(opening_follow::wall_line(&entity));
        work.walls.push((Uuid::now_v7(), entity));
    }
    let door_layer = input
        .opening_layer_id
        .clone()
        .or_else(|| first_layer(PlanRole::Door));
    let window_layer = input
        .opening_layer_id
        .clone()
        .or_else(|| first_layer(PlanRole::Window));
    for (k, o) in read.openings.iter().enumerate() {
        let (host, wall) = &work.walls[o.wall];
        let kind = type_of(o);
        // A door's swing says its leaves, hinge, side and angle; a window and a bare opening keep their type's.
        let door = o.form == read::Form::Door;
        let n = NewOpening {
            host: host.to_string(),
            width: Some(o.to - o.from),
            leaves: door.then_some(o.leaves),
            hand: door.then_some(match o.hand {
                read::End::Start => OpeningHand::Start,
                read::End::End => OpeningHand::End,
            }),
            swing: o.swing.filter(|_| door),
            ..Default::default()
        };
        let mut fields = openings::fields_of(&n, &kind);
        fields.at = (o.from + o.to) / 2.0;
        if door && o.side == read::Side::Right {
            fields.side = OpeningSide::Right;
        }
        let path = format!("openings[{k}]");
        let Some(line) = &lines_of_walls[o.wall] else {
            return Err(openings::fail(
                codes::NOT_A_WALL,
                "Duvarın yolu çizilemiyor; açıklık konamaz.".into(),
                &path,
            ));
        };
        let fields = openings::placed(&fields, line, &path)?;
        let wall_layer = wall.base().layer_id.clone();
        let layer = match o.form {
            read::Form::Door => door_layer.clone(),
            read::Form::Window => window_layer.clone(),
            read::Form::Opening => input.opening_layer_id.clone(),
        }
        .unwrap_or(wall_layer);
        work.counts[match o.form {
            read::Form::Door => 0,
            read::Form::Window => 1,
            read::Form::Opening => 2,
        }] += 1;
        work.openings
            .push((Uuid::now_v7(), openings::entity(base(&layer), fields)));
    }
    // The columns, on their objects' layers; a hatch inside one goes with it.
    let mut used: BTreeSet<usize> = read.used.iter().copied().collect();
    for (k, c) in &made_columns {
        arch_elements::column_valid(c, &format!("uids[{}]", found[*k].0))?;
        used.insert(*k);
        work.columns.push((
            Uuid::now_v7(),
            Entity::Column(ColumnEntity {
                base: base(&found[*k].2.base().layer_id),
                column: c.clone(),
            }),
        ));
    }
    for (k, middle) in &fills {
        if made_columns
            .iter()
            .any(|(_, c)| in_column(c, *middle, o.tol))
        {
            used.insert(*k);
        }
    }
    // The sources: those read whole go unless kept; the others stay, each with why.
    for (k, (_, _, _, uid)) in found.iter().enumerate() {
        if used.contains(&k) {
            if !input.keep_sources {
                work.removed.push((*uid).clone());
            }
            continue;
        }
        let why = unread.get(&k).copied().unwrap_or(Unread::Other);
        unread.insert(k, why);
        work.left.push((*uid).clone());
    }
    if !work.left.is_empty() {
        let mut by: BTreeMap<Unread, usize> = BTreeMap::new();
        for k in found
            .iter()
            .enumerate()
            .filter(|(k, _)| !used.contains(k))
            .map(|(k, _)| k)
        {
            *by.entry(unread[&k]).or_default() += 1;
        }
        let mut ignored_names: Vec<String> = roles
            .iter()
            .filter(|(_, r)| **r == PlanRole::Ignore)
            .filter(|(id, _)| found.iter().any(|(_, _, e, _)| &e.base().layer_id == *id))
            .map(|(id, _)| layers.get(id).map_or(id.clone(), |n| n.name.clone()))
            .collect();
        ignored_names.sort_by_key(|n| fold(n));
        let parts: Vec<String> = by
            .iter()
            .map(|(why, n)| match why {
                Unread::Ignored => format!(
                    "{n} nesne yok sayılan katmanlarda ({})",
                    ignored_names.join(", ")
                ),
                Unread::Block => format!("{n} blok açılamadı"),
                Unread::Column => {
                    format!("{n} kolon nesnesi kolona çevrilemedi (dikdörtgen ya da daire değil)")
                }
                Unread::Other => format!("{n} nesne duvara, kapıya ya da pencereye ait değil"),
            })
            .collect();
        work.warnings.push(CommandWarning {
            code: codes::PLAN_UNRECOGNIZED.into(),
            message: format!(
                "{} nesne tanınmadı ve çizimde kaldı: {}.",
                work.left.len(),
                parts.join("; ")
            ),
            path: Some("uids".into()),
        });
    }
    // The layers written to, each once.
    let mut seen = BTreeSet::new();
    for (_, e) in work.walls.iter().chain(&work.openings).chain(&work.columns) {
        let id = &e.base().layer_id;
        if seen.insert(id.clone()) {
            let path = if input.wall_layer_id.as_ref() == Some(id) {
                "wallLayerId"
            } else if input.opening_layer_id.as_ref() == Some(id) {
                "openingLayerId"
            } else {
                "uids"
            };
            warnings.extend(checks::layer_at(doc, id, path)?);
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    work.added = added;
    warnings.append(&mut work.warnings);
    work.warnings = warnings;
    Ok(work)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ArchRecognize) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(work) => CommandResult::Completed {
            output: (),
            warnings: work.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ArchRecognize) -> CommandResult<ArchRecognizePlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what Planı tanı previews.
pub fn plan_in(doc: &Document, input: &ArchRecognize) -> CommandResult<ArchRecognizePlan> {
    match check(doc, input) {
        Ok(work) => CommandResult::Completed {
            output: ArchRecognizePlan {
                walls: work.walls.into_iter().map(|(_, e)| e).collect(),
                openings: work.openings.into_iter().map(|(_, e)| e).collect(),
                columns: work.columns.into_iter().map(|(_, e)| e).collect(),
                removed: work.removed,
                left: work.left,
                revision: doc.revision().to_string(),
            },
            warnings: work.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ArchRecognize,
) -> CommandResult<ArchRecognized> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ARCH_RECOGNIZE,
        kentos_commands::CAD_ARCH_RECOGNIZE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: ArchRecognize,
) -> CommandResult<ArchRecognized> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    walls::take_styles(doc, &work.added);
    let wall_ids: Vec<String> = work.walls.iter().map(|(u, _)| u.to_string()).collect();
    let opening_ids: Vec<String> = work.openings.iter().map(|(u, _)| u.to_string()).collect();
    let column_ids: Vec<String> = work.columns.iter().map(|(u, _)| u.to_string()).collect();
    let all: Vec<(Uuid, Entity)> = work
        .walls
        .into_iter()
        .chain(work.openings)
        .chain(work.columns)
        .collect();
    if let Err(refused) = doc.add_many_with_uids(all, STEP) {
        return Stop::failed(error(codes::SLOTS_EXHAUSTED, refused.0, None)).into();
    }
    let gone: Vec<Slot> = work
        .removed
        .iter()
        .filter_map(|u| Uuid::parse_str(u).ok())
        .filter_map(|u| doc.slot_of(u))
        .collect();
    if !gone.is_empty() {
        doc.remove(&gone);
    }
    doc.name_step(STEP);
    CommandResult::Completed {
        output: ArchRecognized {
            walls: wall_ids,
            openings: opening_ids,
            columns: column_ids,
            doors: work.counts[0],
            windows: work.counts[1],
            bare: work.counts[2],
            removed: work.removed,
            left: work.left,
            revision: doc.revision().to_string(),
        },
        warnings: work.warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offices_layer_names_say_their_roles() {
        for (name, role) in [
            ("DUVAR", Some(PlanRole::Wall)),
            ("A-WALL", Some(PlanRole::Wall)),
            ("MM-0000-DUVAR", Some(PlanRole::Wall)),
            ("Perde", Some(PlanRole::Wall)),
            ("BÖLME DUVARLARI", Some(PlanRole::Wall)),
            ("KAPI", Some(PlanRole::Door)),
            ("Kapılar", Some(PlanRole::Door)),
            ("A-DOOR", Some(PlanRole::Door)),
            ("PENCERE", Some(PlanRole::Window)),
            ("A-GLAZ", Some(PlanRole::Window)),
            ("CAM", Some(PlanRole::Window)),
            ("KOLON", Some(PlanRole::Column)),
            ("S-COLS", Some(PlanRole::Column)),
            ("DUVAR-TARAMA", Some(PlanRole::Ignore)),
            ("AKS", Some(PlanRole::Ignore)),
            ("ÖLÇÜ", Some(PlanRole::Ignore)),
            ("YAZI", Some(PlanRole::Ignore)),
            ("MOBİLYA", Some(PlanRole::Ignore)),
            ("MERDIVEN", Some(PlanRole::Ignore)),
            ("A-ANNO-DIMS", Some(PlanRole::Ignore)),
            ("0", None),
            ("Aksesuar", None),
        ] {
            assert_eq!(guess_role(name), role, "{name}");
        }
    }
}
