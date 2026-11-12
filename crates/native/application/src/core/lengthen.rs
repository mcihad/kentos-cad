//! `cad.entities.lengthen` v1, Uzat-kısalt (docs/adr/0298; AutoCAD's
//! LENGTHEN): an object's length changed at one end by a difference, a
//! percentage, a total, an arc's included angle or toward a point, as one undo
//! step “Uzat-kısalt”. The desktop's handler over the native document, run by
//! the Uzat-kısalt tool (its preview is [`entities_lengthen::plan_in`]), the
//! headless server, Python and MCP. The shared cases are
//! `fixtures/commands/v1/cad.entities.lengthen.json`.
//!
//! The new shape and both lengths are the shared core's
//! (`ops::lengthen::lengthen`), written through `cad.entities.edit`'s
//! `lengthen` operation ([`crate::edit`]). What the object carries:
//!
//! - lengthening moves the end vertex alone: a polyline's edges keep their
//!   widths (the core keeps them), the moving end's elevation and measure (M)
//!   run on along the end edge by length, from its two ends' (written here);
//! - shortening cuts the path: the edit carries the kept vertices' values and
//!   gives the new end the cut edge's by length (`Carry::Along`).
//!
//! The checks, in order (the first that fails answers):
//! 1. the id is lowercase UUID text with hyphens;
//! 2. the change's numbers finite, Fark and Toplam one of a length and an angle;
//! 3. the expected revision (every command's, `checks.rs`);
//! 4. the id names an object, not on a locked layer;
//! 5. the core's refusals: the kind, a multi-part polyline, an angle for other
//!    than an arc, a new length not above zero, an arc closing on itself.

use kentos_commands::{
    CommandResult, EditOperation, EntitiesEdit, EntitiesEditPlan, EntitiesEdited, EntitiesLengthen,
    Entity, EntityEdit, EntityGeometry, LengthenChange, LengthenEnd,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::ops::lengthen::{Change, Lengthened, Why, lengthen};
use kentos_geometry_core::ops::path::path_of;

use crate::ExecutionContext;
use crate::break_join::{Checked, Ties, execute_with, plan_with, validate_with};
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::{edit_geometry, shape};

/// The undo step of `cad.entities.lengthen`: the tool's name.
pub const LABEL: &str = "Uzat-kısalt";

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

/// The change as the core takes it; the input's own refusals first.
fn change_of(change: &LengthenChange) -> Result<Change, Stop> {
    let finite =
        |v: f64, what: &str, path: &str| checks::finite(v, what, "Sonlu bir sayı verin.", path);
    let one =
        |length: Option<f64>, angle: Option<f64>| -> Result<(Option<f64>, Option<f64>), Stop> {
            match (length, angle) {
                (Some(_), None) | (None, Some(_)) => Ok((length, angle)),
                _ => Err(refuse(
                    codes::INVALID_CHANGE,
                    "Fark ve Toplam'da uzunluk (length) ya da açıdan (angle) yalnız birini verin.",
                    "change",
                )),
            }
        };
    Ok(match *change {
        LengthenChange::Delta { length, angle } => {
            let (length, angle) = one(length, angle)?;
            match (length, angle) {
                (Some(l), _) => {
                    finite(l, "Uzunluk farkı", "change.length")?;
                    Change::Delta(l)
                }
                (_, Some(a)) => {
                    finite(a, "Açı farkı", "change.angle")?;
                    Change::DeltaAngle(a)
                }
                _ => unreachable!("one of them"),
            }
        }
        LengthenChange::Percent { percent } => {
            finite(percent, "Yüzde", "change.percent")?;
            Change::Percent(percent)
        }
        LengthenChange::Total { length, angle } => {
            let (length, angle) = one(length, angle)?;
            match (length, angle) {
                (Some(l), _) => {
                    finite(l, "Toplam uzunluk", "change.length")?;
                    Change::Total(l)
                }
                (_, Some(a)) => {
                    finite(a, "Toplam açı", "change.angle")?;
                    Change::TotalAngle(a)
                }
                _ => unreachable!("one of them"),
            }
        }
        LengthenChange::Dynamic { to } => {
            checks::point(to, "Hedef noktanın", "change.to")?;
            Change::Toward(Vec2::new(to.x, to.y))
        }
    })
}

/// The core's refusal as the command's.
fn refusal(why: Why) -> Stop {
    let (code, path) = match why {
        Why::NotLengthenable => (codes::NOT_LENGTHENABLE, "uid"),
        Why::MultiPart => (codes::MULTI_PART, "uid"),
        Why::NoLength => (codes::NOT_LENGTHENABLE, "uid"),
        Why::AngleNeedsArc => (codes::ANGLE_NEEDS_ARC, "change.angle"),
        Why::InvalidLength => (codes::INVALID_LENGTH, "change"),
        Why::Closes => (codes::ARC_CLOSES, "change"),
    };
    let fix = match why {
        Why::NotLengthenable => {
            " Kapalı çoklu çizgiyi önce açın; daire ve bütün elips için Kır kullanın."
        }
        Why::MultiPart => "",
        Why::NoLength => " Uzat-kısalt uzunluğu olan nesnenin ucunu taşır.",
        Why::AngleNeedsArc => " Çizgiye, elips yayına ve çoklu çizgiye uzunluk verin.",
        Why::InvalidLength => " Daha küçük bir kısaltma ya da sıfırdan büyük bir uzunluk verin.",
        Why::Closes => " Daha kısa bir uzunluk ya da açı verin.",
    };
    refuse(code, format!("{}{fix}", why.message()), path)
}

/// The value at the moving end of an end edge run on by length: `fixed` at the
/// edge's other end, `moving` at its end, the edge `was` long and now `now`.
fn run_on(fixed: Option<f64>, moving: Option<f64>, was: f64, now: f64) -> Option<f64> {
    match (fixed, moving) {
        (Some(f), Some(m)) if was > 0.0 => Some(f + (m - f) * (now / was)),
        _ => moving,
    }
}

/// The end edge's length of a path (its last edge at the end, its first at the start).
fn end_edge(s: &Shape, at_end: bool) -> Option<f64> {
    let path = path_of(s)?;
    let n = path.edges.len();
    Some(if at_end {
        path.length - path.cum[n - 1]
    } else if n > 1 {
        path.cum[1]
    } else {
        path.length
    })
}

/// The geometry to write: lengthening a line or a polyline writes the moving
/// end's elevation and measure run on; anything else leaves them to the edit.
fn geometry_of(e: &Entity, was: &Shape, done: &Lengthened, at_end: bool) -> Option<EntityGeometry> {
    let mut geometry = edit_geometry(done.entity.shape.clone())?;
    if done.after <= done.before {
        return Some(geometry);
    }
    let (Some(old), Some(new)) = (end_edge(was, at_end), end_edge(&done.entity.shape, at_end))
    else {
        return Some(geometry);
    };
    match (e, &mut geometry) {
        (Entity::Line(l), EntityGeometry::Line { zs, ms, .. }) => {
            let pair = |a: Option<f64>, b: Option<f64>| -> Option<Vec<Option<f64>>> {
                (a.is_some() || b.is_some()).then(|| {
                    if at_end {
                        vec![a, run_on(a, b, old, new)]
                    } else {
                        vec![run_on(b, a, old, new), b]
                    }
                })
            };
            *zs = pair(l.za, l.zb);
            *ms = pair(l.ma, l.mb);
        }
        (Entity::Polyline(p), EntityGeometry::Polyline { zs, ms, .. }) => {
            let n = p.pts.len();
            let carried = |values: &Option<Vec<Option<f64>>>| -> Option<Vec<Option<f64>>> {
                let v = values
                    .as_ref()
                    .filter(|v| v.len() == n && v.iter().any(Option::is_some))?;
                let mut out = v.clone();
                let (fixed, moving) = if at_end { (n - 2, n - 1) } else { (1, 0) };
                out[moving] = run_on(v[fixed], v[moving], old, new);
                Some(out)
            };
            *zs = carried(&p.zs);
            *ms = carried(&p.ms);
        }
        _ => {}
    }
    Some(geometry)
}

/// The checks of `cad.entities.lengthen`, in order; the edit it writes.
fn check(doc: &Document, input: &EntitiesLengthen) -> Result<Checked, Stop> {
    if !checks::is_uid_text(&input.uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!(
                "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).",
                input.uid
            ),
            "uid",
        ));
    }
    let change = change_of(&input.change)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let found = Uuid::parse_str(&input.uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|slot| Some((slot, doc.get(slot)?)));
    let Some((_, e)) = found else {
        return Err(refuse(
            codes::ENTITY_NOT_FOUND,
            format!(
                "“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                input.uid
            ),
            "uid",
        ));
    };
    let layers = doc.layers();
    if layers.is_locked(&e.base().layer_id) {
        let layer = &e.base().layer_id;
        let name = layers
            .get(layer)
            .map_or(layer.as_str(), |n| n.name.as_str());
        return Err(refuse(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
            ),
            "uid",
        ));
    }
    let at_end = input.end == LengthenEnd::End;
    let was = shape(e);
    let done = lengthen(&CoreEntity::new(was.clone()), at_end, change).map_err(refusal)?;
    let Some(geometry) = geometry_of(e, &was, &done, at_end) else {
        return Err(refusal(Why::NotLengthenable));
    };
    Ok((
        EntitiesEdit {
            operation: EditOperation::Lengthen,
            changes: vec![EntityEdit::Update {
                uid: input.uid.clone(),
                geometry,
            }],
            expected_revision: None,
        },
        Vec::new(),
        // A dimension's point follows its place (an end runs with it): nothing is looked for by coordinates.
        Ties::default(),
    ))
}

/// The lengths before and after, for a tool's words (the core's, the checks passed).
pub fn lengths(doc: &Document, input: &EntitiesLengthen) -> Option<(f64, f64)> {
    let change = change_of(&input.change).ok()?;
    let e = Uuid::parse_str(&input.uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))?;
    let done = lengthen(
        &CoreEntity::new(shape(e)),
        input.end == LengthenEnd::End,
        change,
    )
    .ok()?;
    Some((done.before, done.after))
}

/// `cad.entities.lengthen` v1: Uzat-kısalt.
pub mod entities_lengthen {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesLengthen) -> CommandResult<()> {
        validate_with(check(cx.doc, input))
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &EntitiesLengthen,
    ) -> CommandResult<EntitiesEditPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over the document alone (the tool's preview).
    pub fn plan_in(doc: &Document, input: &EntitiesLengthen) -> CommandResult<EntitiesEditPlan> {
        plan_with(doc, check(doc, input))
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: EntitiesLengthen,
    ) -> CommandResult<EntitiesEdited> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_ENTITIES_LENGTHEN,
            kentos_commands::CAD_ENTITIES_LENGTHEN_VERSION,
            input,
            |cx, input| {
                let checked = check(cx.doc, &input);
                execute_with(cx, checked)
            },
        )
    }
}
