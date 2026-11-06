//! `cad.entities.grade` v1 (plan/map-cad.md MAP-06, docs/adr/1290): Eğimle kot ver. The range's elevations from the
//! shared core (`ops::grade`) written to the object's vertices in one undo step; the others keep theirs. The shared
//! cases are `fixtures/commands/v1/cad.entities.grade.json`.

use kentos_commands::{CommandResult, EntitiesGrade, EntitiesGraded, Entity};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::ops::grade::{Target, grade};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

const STEP: &str = "Eğimle kot ver";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A path: its vertices, bulges and elevations.
type Path = (Vec<Vec2>, Option<Vec<f64>>, Vec<Option<f64>>);

/// The object's path (a line's two ends).
fn path_of(e: &Entity) -> Option<Path> {
    let v = |p: &kentos_commands::Vec2| Vec2::new(p.x, p.y);
    match e {
        Entity::Line(l) => Some((vec![v(&l.a), v(&l.b)], None, vec![l.za, l.zb])),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let zs = p.zs.clone().unwrap_or_else(|| vec![None; p.pts.len()]);
            Some((p.pts.iter().map(v).collect(), p.bulges.clone(), zs))
        }
        _ => None,
    }
}

fn check(doc: &Document, input: &EntitiesGrade) -> Result<(Slot, Entity, EntitiesGraded), Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    if !checks::is_uid_text(&input.uid) {
        return Err(fail(codes::INVALID_UID, format!("“{}” geçerli bir nesne kimliği değil.", input.uid), "uid"));
    }
    let Some((slot, e)) = Uuid::parse_str(&input.uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s).map(|e| (s, e)))
    else {
        return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{}” kimlikli nesne çizimde yok.", input.uid), "uid"));
    };
    let Some((pts, bulges, zs)) = path_of(e) else {
        return Err(fail(codes::NOT_A_PATH, "Nesne çizgi, çoklu çizgi ya da alan değil; kotu eğimle verilemez.".into(), "uid"));
    };
    let (from, to) = (input.from as usize, input.to as usize);
    if !(from < to && to < pts.len()) {
        return Err(fail(
            codes::WRONG_PLACE,
            format!("Köşe aralığı {}–{} olamaz: nesnenin {} köşesi var (0'dan), başlangıç bitişten önce olmalı.", input.from, input.to, pts.len()),
            "to",
        ));
    }
    let target = match (input.end, input.grade) {
        (Some(z), None) => Target::End(z),
        (None, Some(g)) => Target::Grade(g),
        _ => {
            return Err(fail(codes::INVALID_INPUT, "Bitiş kotunu (end) ya da eğimi (grade) verin, ikisini birden değil.".into(), "end"));
        }
    };
    let Some(start) = input.start.or(zs[from]) else {
        return Err(fail(codes::ELEVATION_MISSING, format!("{}. köşenin kotu yok; başlangıç kotunu (start) verin.", input.from), "start"));
    };
    let g = grade(&pts, bulges.as_deref(), from, to, start, target).map_err(|why| match why {
        "length" => fail(codes::INVALID_INPUT, "Köşe aralığının yatay uzunluğu sıfır.".into(), "to"),
        _ => fail(codes::INVALID_INPUT, "Kot ve eğim sonlu sayı olmalı.".into(), "start"),
    })?;
    let layer = &e.base().layer_id;
    if doc.layers().is_locked(layer) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{}” katmanı kilitli: nesnenin kotları değişemez. Kilidi açın.", doc.layers().path(layer)),
            "uid",
        ));
    }
    let mut next = e.clone();
    match &mut next {
        Entity::Line(l) => {
            l.za = Some(g.zs[0]);
            l.zb = Some(g.zs[1]);
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let mut z = zs;
            for (k, v) in g.zs.iter().enumerate() {
                z[from + k] = Some(*v);
            }
            p.zs = Some(z);
        }
        _ => {}
    }
    let out = EntitiesGraded { length: g.length, start, end: g.end, grade: g.grade, zs: g.zs, revision: String::new() };
    Ok((slot, next, out))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesGrade) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The elevations the range would take; nothing written.
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesGrade) -> CommandResult<EntitiesGraded> {
    match check(cx.doc, input) {
        Ok((_, _, mut out)) => {
            out.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output: out, warnings: Vec::new() }
        }
        Err(stop) => stop.into(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: EntitiesGrade) -> CommandResult<EntitiesGraded> {
    let (slot, next, mut out) = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    cx.doc.update_many(vec![(slot, next)], STEP);
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings: Vec::new() }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: EntitiesGrade) -> CommandResult<EntitiesGraded> {
    crate::call::recorded(cx, kentos_commands::CAD_ENTITIES_GRADE, kentos_commands::CAD_ENTITIES_GRADE_VERSION, input, write)
}
