//! `cad.entities.fillet` v1 and `cad.entities.chamfer` v1 (docs/adr/0250):
//! a corner rounded (Köşe yuvarla) or cut (Pah), from what was asked (the
//! corner and its size, Kırp), as one undo step “Köşe yuvarla” (“Pah”).
//! The desktop's handler over the native document, run by the corner tools
//! and Tüm köşeleri yuvarla and Tüm köşelere pah (their preview is
//! [`plan_in`], their click [`execute`]), the headless server, Python and
//! MCP. The shared cases are `fixtures/commands/v1/cad.entities.fillet.json`
//! and `.chamfer.json`.
//!
//! Nothing is computed twice: the corner (`path_corner_at`,
//! `lines_corner_at`), the sides and the arc or the cut (`corner_in_path`,
//! `fillet_lines`, `chamfer_lines`, `fillet_arc`, `chamfer_line`) and every
//! corner of a path (`all_corners`) are the shared core's, as the tools
//! always took them; they are written through `cad.entities.edit`'s
//! `fillet` and `chamfer` operations ([`crate::edit`]).
//!
//! The checks, in order: `all` names objects; the ids; the size and the
//! picks finite; the size (not negative; above zero where something is
//! added); the expected revision; the objects; their layers unlocked; the
//! corner is one; the size fits it.

use kentos_contracts::{
    CommandResult, CommandWarning, CornerPick, EditOperation, EntitiesChamfer, EntitiesCornerPlan,
    EntitiesCornered, EntitiesEdit, EntitiesFillet, Entity, EntityEdit,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::ops::curve_cuts::Geometry;
use kentos_geometry_core::ops::fillet::{
    Chamfer, Corner as Joined, CornerOp, Fillet, Seg, chamfer_lines, corner_in_path, fillet_lines,
};
use kentos_geometry_core::ops::reshape::all_corners;
use kentos_geometry_core::tools::editing::{
    CornerGeom, chamfer_line, fillet_arc, lines_corner_at, path_corner_at,
};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::edit;
use crate::geometry::{edit_geometry, shape};

/// A size: a fillet's radius, or a chamfer's two distances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    Radius(f64),
    Cut(f64, f64),
}

impl Size {
    pub fn core(self) -> CornerOp {
        match self {
            Size::Radius(r) => CornerOp::Radius(r),
            Size::Cut(d1, d2) => CornerOp::Chamfer(d1, d2),
        }
    }

    fn operation(self) -> EditOperation {
        match self {
            Size::Radius(_) => EditOperation::Fillet,
            Size::Cut(..) => EditOperation::Chamfer,
        }
    }

    fn failed(self) -> &'static str {
        match self {
            Size::Radius(_) => codes::FILLET_FAILED,
            Size::Cut(..) => codes::CHAMFER_FAILED,
        }
    }

    /// Whether it adds nothing: a zero radius, two zero distances.
    fn nothing(self) -> bool {
        match self {
            Size::Radius(r) => r == 0.0,
            Size::Cut(d1, d2) => d1 == 0.0 && d2 == 0.0,
        }
    }
}

/// A command's request, whichever of the two.
pub struct Request<'a> {
    pub corner: &'a CornerPick,
    pub size: Size,
    pub trim: bool,
    pub expected_revision: Option<&'a str>,
}

impl<'a> From<&'a EntitiesFillet> for Request<'a> {
    fn from(f: &'a EntitiesFillet) -> Self {
        Request {
            corner: &f.corner,
            size: Size::Radius(f.radius),
            trim: f.trim.unwrap_or(true),
            expected_revision: f.expected_revision.as_deref(),
        }
    }
}

impl<'a> From<&'a EntitiesChamfer> for Request<'a> {
    fn from(c: &'a EntitiesChamfer) -> Self {
        Request {
            corner: &c.corner,
            size: Size::Cut(c.distance, c.second.unwrap_or(c.distance)),
            trim: c.trim.unwrap_or(true),
            expected_revision: c.expected_revision.as_deref(),
        }
    }
}

/// Where the corner is, in the drawing (the tools' `Site`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Site {
    /// Vertex `vertex` of the path at `slot`.
    Vertex { slot: Slot, vertex: usize },
    /// Two lines; each keeps the side of its pick point.
    Lines {
        first: Slot,
        pick1: Vec2,
        second: Slot,
        pick2: Vec2,
    },
}

/// A corner: where it is and its geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    pub site: Site,
    pub geom: CornerGeom,
}

impl Corner {
    pub fn slots(&self) -> Vec<Slot> {
        match self.site {
            Site::Vertex { slot, .. } => vec![slot],
            Site::Lines { first, second, .. } => vec![first, second],
        }
    }
}

/// A line object's segment.
pub fn line_of(e: &Entity) -> Option<Seg> {
    match e {
        Entity::Line(l) => Some(Seg {
            a: Vec2::new(l.a.x, l.a.y),
            b: Vec2::new(l.b.x, l.b.y),
        }),
        _ => None,
    }
}

/// A path's corner at vertex `i`, an area's vertices counted part after
/// part (docs/adr/0143): none at an open path's end, beside an arc or on a
/// straight run (the core's `path_corner_at`).
pub fn path_corner(slot: Slot, e: &Entity, i: usize) -> Option<Corner> {
    if !matches!(e, Entity::Polyline(_) | Entity::Polygon(_)) {
        return None;
    }
    let geom = path_corner_at(&shape(e), i)?;
    Some(Corner {
        site: Site::Vertex { slot, vertex: i },
        geom,
    })
}

/// The corner of two lines, each keeping the side of its pick; none when they
/// are one or parallel, or another kind.
pub fn lines_corner(doc: &Document, a: (Slot, Vec2), b: (Slot, Vec2)) -> Option<Corner> {
    if a.0 == b.0 {
        return None;
    }
    let l1 = doc.get(a.0).and_then(line_of)?;
    let l2 = doc.get(b.0).and_then(line_of)?;
    let geom = lines_corner_at(l1.a, l1.b, a.1, l2.a, l2.b, b.1)?;
    Some(Corner {
        site: Site::Lines {
            first: a.0,
            pick1: a.1,
            second: b.0,
            pick2: b.1,
        },
        geom,
    })
}

/// What a size does to a corner: the sides' new geometry and the piece
/// added (the arc or the cut), made from the first side.
pub struct Done {
    pub updates: Vec<(Slot, Shape)>,
    pub add: Option<(Slot, Shape)>,
}

/// What `size` does to the corner with Kırp, or why it cannot.
pub fn done(c: &Corner, size: Size, doc: &Document) -> Result<Done, String> {
    match c.site {
        Site::Vertex { slot, vertex } => {
            let e = doc.get(slot).ok_or_else(String::new)?;
            // The whole geometry is written (docs/adr/0047): the corner's part keeps its
            // holes, the area its other parts (docs/adr/0143).
            match corner_in_path(&shape(e), vertex, &size.core())? {
                Geometry::Error(error) => Err(error),
                Geometry::Ok(done) => Ok(Done {
                    updates: vec![(slot, done.shape)],
                    add: None,
                }),
            }
        }
        Site::Lines {
            first,
            pick1,
            second,
            pick2,
        } => {
            let l1 = doc.get(first).and_then(line_of).ok_or_else(String::new)?;
            let l2 = doc.get(second).and_then(line_of).ok_or_else(String::new)?;
            let line = |s: Seg| Shape::Line { a: s.a, b: s.b };
            let (joined, piece) = match size {
                Size::Radius(r) => {
                    let Fillet(j) = fillet_lines(l1, pick1, l2, pick2, r);
                    match j {
                        Joined::Error(e) => return Err(e),
                        Joined::Ok { line1, line2, join } => (
                            (line1, line2),
                            join.map(|a| Shape::Arc {
                                c: a.c,
                                r: a.r,
                                a0: a.a0,
                                a1: a.a1,
                            }),
                        ),
                    }
                }
                Size::Cut(d1, d2) => {
                    let Chamfer(j) = chamfer_lines(l1, pick1, l2, pick2, d1, d2);
                    match j {
                        Joined::Error(e) => return Err(e),
                        Joined::Ok { line1, line2, join } => ((line1, line2), join.map(line)),
                    }
                }
            };
            // A side keeps its direction: the core gives the kept end first,
            // which is the line's end when the corner was at its start.
            let along = |was: Seg, s: Seg| {
                if s.a == was.b && s.a != was.a {
                    Seg { a: s.b, b: s.a }
                } else {
                    s
                }
            };
            Ok(Done {
                updates: vec![
                    (first, line(along(l1, joined.0))),
                    (second, line(along(l2, joined.1))),
                ],
                add: piece.map(|s| (first, s)),
            })
        }
    }
}

/// The arc or the cut alone, without Kırp.
pub fn piece(c: &Corner, size: Size) -> Option<Shape> {
    match size {
        Size::Radius(r) => fillet_arc(&c.geom, r).map(|a| Shape::Arc {
            c: a.c,
            r: a.r,
            a0: a.a0,
            a1: a.a1,
        }),
        Size::Cut(d1, d2) => chamfer_line(&c.geom, d1, d2).map(|p| Shape::Line { a: p.a, b: p.b }),
    }
}

/// Checks the request against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, request: Request<'_>) -> CommandResult<()> {
    match check(cx.doc, &request) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, request: Request<'_>) -> CommandResult<EntitiesCornerPlan> {
    plan_in(cx.doc, request)
}

/// [`plan`] over the document alone (the tools' preview).
pub fn plan_in(doc: &Document, request: Request<'_>) -> CommandResult<EntitiesCornerPlan> {
    let checked = match check(doc, &request) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match edit::plan_in(doc, &checked.edit) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesCornerPlan {
                changed: output.changed,
                created: output.created,
                done: checked.done,
                skipped: checked.skipped,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => relay(other),
    }
}

/// Checks the request again and writes it as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    request: Request<'_>,
) -> CommandResult<EntitiesCornered> {
    let checked = match check(cx.doc, &request) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match edit::execute(cx, checked.edit) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesCornered {
                changed: output.changed,
                created: output.created,
                done: checked.done,
                skipped: checked.skipped,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => relay(other),
    }
}

fn relay<T, U>(result: CommandResult<T>) -> CommandResult<U> {
    match result {
        CommandResult::Completed { .. } | CommandResult::Cancelled => CommandResult::Cancelled,
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::Failed { error } => CommandResult::Failed { error },
    }
}

struct Checked {
    edit: EntitiesEdit,
    done: u32,
    skipped: u32,
    warnings: Vec<CommandWarning>,
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.into())))
}

fn uid_of(doc: &Document, slot: Slot) -> String {
    doc.uid(slot).map(|u| u.to_string()).unwrap_or_default()
}

fn check(doc: &Document, r: &Request<'_>) -> Result<Checked, Stop> {
    let size = r.size;
    // The ids, with where each is named.
    let named: Vec<(&str, String)> = match r.corner {
        CornerPick::Vertex { uid, .. } => vec![(uid.as_str(), "corner.uid".into())],
        CornerPick::Lines { first, second } => vec![
            (first.uid.as_str(), "corner.first.uid".into()),
            (second.uid.as_str(), "corner.second.uid".into()),
        ],
        CornerPick::All { uids } => uids
            .iter()
            .enumerate()
            .map(|(i, u)| (u.as_str(), format!("corner.uids[{i}]")))
            .collect(),
    };
    if named.is_empty() {
        return Err(refuse(
            codes::NO_ENTITIES,
            "Köşeleri işlenecek nesne yok. En az bir çoklu çizgi ya da alan verin.".into(),
            "corner.uids",
        ));
    }
    for (uid, path) in &named {
        if !is_uid_text(uid) {
            return Err(refuse(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                ),
                path,
            ));
        }
    }
    let (what, field) = match size {
        Size::Radius(_) => ("Yarıçap", "radius"),
        Size::Cut(..) => ("Pah mesafesi", "distance"),
    };
    let values: Vec<(f64, &str)> = match size {
        Size::Radius(radius) => vec![(radius, field)],
        Size::Cut(d1, d2) => vec![(d1, field), (d2, "second")],
    };
    for (v, path) in &values {
        checks::finite(*v, what, "Sonlu bir sayı yazın.", path)?;
    }
    if let CornerPick::Lines { first, second } = r.corner {
        checks::point(
            first.at,
            "Birinci çizginin tıklanan noktasının",
            "corner.first.at",
        )?;
        checks::point(
            second.at,
            "İkinci çizginin tıklanan noktasının",
            "corner.second.at",
        )?;
    }
    let all = matches!(r.corner, CornerPick::All { .. });
    for (v, path) in &values {
        if *v < 0.0 {
            return Err(refuse(
                codes::INVALID_DISTANCE,
                format!("{what} eksi olamaz; sıfır ya da sıfırdan büyük bir değer yazın."),
                path,
            ));
        }
    }
    // Every corner needs every value above zero (the core's `all_corners`).
    let positive = match size {
        Size::Radius(radius) => radius > 0.0,
        Size::Cut(d1, d2) => d1 > 0.0 && d2 > 0.0,
    };
    if (all && !positive) || (size.nothing() && !r.trim) {
        let why = if all {
            "Tüm köşelerde her değer sıfırdan büyük olmalı; sıfır hiçbir köşeyi değiştirmez."
        } else {
            "Kırpmadan çalışırken sıfırdan büyük bir boyut verin; yoksa eklenecek bir şey yok."
        };
        return Err(refuse(codes::INVALID_DISTANCE, why.into(), field));
    }
    checks::revision(doc, r.expected_revision)?;
    let mut slots = Vec::with_capacity(named.len());
    for (uid, path) in &named {
        let found = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u));
        let Some(slot) = found else {
            return Err(refuse(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                path,
            ));
        };
        slots.push(slot);
    }
    let layers = doc.layers();
    for (slot, (_, path)) in slots.iter().zip(&named) {
        let Some(e) = doc.get(*slot) else { continue };
        let layer = &e.base().layer_id;
        if layers.is_locked(layer) {
            let name = layers
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(refuse(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."
                ),
                path,
            ));
        }
    }
    let operation = size.operation();
    let failed = size.failed();
    let mut warnings = Vec::new();
    let (mut changes, mut done_n, mut skipped) = (Vec::new(), 0u32, 0u32);
    match r.corner {
        CornerPick::All { .. } => {
            let mut first_why: Option<String> = None;
            for (slot, (_, path)) in slots.iter().zip(&named) {
                let e = doc
                    .get(*slot)
                    .ok_or_else(|| refuse(codes::NOT_A_CORNER, String::new(), path))?;
                let reshaped = matches!(e, Entity::Polyline(_) | Entity::Polygon(_))
                    .then(|| all_corners(&CoreEntity::new(shape(e)), &size.core()))
                    .flatten();
                let Some(rs) = reshaped else {
                    return Err(refuse(
                        codes::NOT_A_CORNER,
                        "Tüm köşeler yalnız çoklu çizgide ve alanda işlenir.".into(),
                        path,
                    ));
                };
                skipped += rs.skipped as u32;
                if rs.done == 0 {
                    let why =
                        "Hiçbir köşeye sığmadı; daha küçük bir değer verin ya da kenarları uzatın."
                            .to_owned();
                    first_why.get_or_insert_with(|| why.clone());
                    warnings.push(CommandWarning {
                        code: failed.into(),
                        message: why,
                        path: Some(path.clone()),
                    });
                    continue;
                }
                done_n += rs.done as u32;
                if let Some(geometry) = edit_geometry(rs.entity.shape) {
                    changes.push(EntityEdit::Update {
                        uid: uid_of(doc, *slot),
                        geometry,
                    });
                }
            }
            if changes.is_empty() {
                let why = first_why.unwrap_or_default();
                return Err(refuse(failed, why, "corner.uids"));
            }
        }
        CornerPick::Vertex { index, .. } => {
            let slot = slots[0];
            let corner = doc
                .get(slot)
                .and_then(|e| path_corner(slot, e, *index as usize))
                .ok_or_else(|| {
                    refuse(
                        codes::NOT_A_CORNER,
                        "Bu köşe işlenemez: köşe yolun dışında, açık yolun ucunda ya da bir yayın yanında; yalnızca düz kenarlar arasındaki köşe işlenir.".into(),
                        "corner.index",
                    )
                })?;
            add_corner(doc, &corner, size, r.trim, &mut changes)
                .map_err(|why| refuse(failed, why, "corner"))?;
            done_n = 1;
        }
        CornerPick::Lines { first, second } => {
            let a = (slots[0], Vec2::new(first.at.x, first.at.y));
            let b = (slots[1], Vec2::new(second.at.x, second.at.y));
            let corner = lines_corner(doc, a, b).ok_or_else(|| {
                let why = if a.0 == b.0 {
                    "İkinci çizgi ilkinden farklı olmalı."
                } else if doc.get(a.0).and_then(line_of).is_none() || doc.get(b.0).and_then(line_of).is_none() {
                    "İki çizgi verin; çoklu çizginin ve alanın köşesi kendi köşesiyle (vertex) verilir."
                } else {
                    "Çizgiler paralel ya da seçilen tarafta çizgi yok."
                };
                refuse(codes::NOT_A_CORNER, why.into(), "corner")
            })?;
            add_corner(doc, &corner, size, r.trim, &mut changes)
                .map_err(|why| refuse(failed, why, "corner"))?;
            done_n = 1;
        }
    }
    Ok(Checked {
        edit: EntitiesEdit {
            operation,
            changes,
            expected_revision: None,
        },
        done: done_n,
        skipped,
        warnings,
    })
}

/// The changes that round or cut one corner (the corner tools' `commit`).
fn add_corner(
    doc: &Document,
    c: &Corner,
    size: Size,
    trim: bool,
    changes: &mut Vec<EntityEdit>,
) -> Result<(), String> {
    let done = done(c, size, doc)?;
    if !trim {
        let Some(piece) = piece(c, size) else {
            return Err("Ölçü köşeye sığmıyor; daha küçük bir değer verin.".into());
        };
        let from = uid_of(doc, c.slots()[0]);
        if let Some(geometry) = edit_geometry(piece) {
            changes.push(EntityEdit::add(from, geometry, None));
        }
        return Ok(());
    }
    for (slot, s) in &done.updates {
        if let Some(geometry) = edit_geometry(s.clone()) {
            changes.push(EntityEdit::Update {
                uid: uid_of(doc, *slot),
                geometry,
            });
        }
    }
    if let Some((like, s)) = &done.add
        && let Some(geometry) = edit_geometry(s.clone())
    {
        changes.push(EntityEdit::add(uid_of(doc, *like), geometry, None));
    }
    Ok(())
}
