//! `cad.entities.grade` v1 (plan/map-cad.md MAP-06, docs/adr/1290): Eğimle kot ver. The range's elevations from the
//! shared core (`ops::grade`) written to the object's vertices in one undo step; the others keep theirs. The shared
//! cases are `fixtures/commands/v1/cad.entities.grade.json`. Docs/adr/4980: the range lies on any part's outer ring or
//! hole (`part`, `ring`), as `cad.entities.drape`'s.
//!
//! docs/adr/3380: the high or low point (`ahead` and `back`, the core's `peak`). A vertex added inside an edge splits
//! it (an arc's halves keep its circle; a line becomes a polyline, as Köşe ekle makes it); its measure (M) and the
//! polyline's widths come by the shared core's carrying rules, as `cad.entities.edit` carries them (`elevation`,
//! `widths`); the constraints naming the object's indexed parts go by the shared rule after the write (ADR 0337 §1).

use kentos_commands::{
    CommandResult, EntitiesGrade, EntitiesGraded, Entity, GradePointKind, GradedPoint, PathEntity,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::ops::elevation::Carry;
use kentos_geometry_core::ops::grade::{NoPeak, PeakAt, PeakKind, Target, grade, peak};
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

/// The object's path (a line's two ends); a polyline's or an area's ring `at` (part, ring; docs/adr/4980).
fn path_of(e: &Entity, at: (usize, usize)) -> Option<Path> {
    let v = |p: &kentos_commands::Vec2| Vec2::new(p.x, p.y);
    let ring = |pts: &[kentos_commands::Vec2], bulges: &Option<Vec<f64>>, zs: &Option<Vec<Option<f64>>>| -> Path {
        let zs = zs.clone().unwrap_or_else(|| vec![None; pts.len()]);
        (pts.iter().map(v).collect(), bulges.clone(), zs)
    };
    match e {
        Entity::Line(l) if at == (0, 0) => Some((vec![v(&l.a), v(&l.b)], None, vec![l.za, l.zb])),
        Entity::Polyline(p) | Entity::Polygon(p) => Some(match at {
            (0, 0) => ring(&p.pts, &p.bulges, &p.zs),
            (0, h) => p.holes.as_ref()?.get(h - 1).map(|r| ring(&r.pts, &r.bulges, &r.zs))?,
            (k, 0) => p.parts.as_ref()?.get(k - 1).map(|q| ring(&q.pts, &q.bulges, &q.zs))?,
            (k, h) => {
                let q = p.parts.as_ref()?.get(k - 1)?;
                q.holes.as_ref()?.get(h - 1).map(|r| ring(&r.pts, &r.bulges, &r.zs))?
            }
        }),
        _ => None,
    }
}

/// The ring `at` of a polyline or an area to write: its vertices, bulges and elevations (docs/adr/4980).
#[allow(clippy::type_complexity)]
fn ring_mut(
    p: &mut PathEntity,
    (part, ring): (usize, usize),
) -> Option<(&mut Vec<kentos_commands::Vec2>, &mut Option<Vec<f64>>, &mut Option<Vec<Option<f64>>>)> {
    match (part, ring) {
        (0, 0) => Some((&mut p.pts, &mut p.bulges, &mut p.zs)),
        (0, h) => p.holes.as_mut()?.get_mut(h - 1).map(|r| (&mut r.pts, &mut r.bulges, &mut r.zs)),
        (k, 0) => p.parts.as_mut()?.get_mut(k - 1).map(|q| (&mut q.pts, &mut q.bulges, &mut q.zs)),
        (k, h) => {
            let q = p.parts.as_mut()?.get_mut(k - 1)?;
            q.holes.as_mut()?.get_mut(h - 1).map(|r| (&mut r.pts, &mut r.bulges, &mut r.zs))
        }
    }
}

/// Why the object has no ring `at`: the field and the words (docs/adr/4980, as `cad.entities.drape`'s).
fn no_ring(e: &Entity, (part, ring): (usize, usize)) -> Stop {
    let (at, why) = match e {
        Entity::Line(_) => (
            if part != 0 { "part" } else { "ring" },
            "Çizginin tek parçası ve tek halkası var: parça ve halka 0 olmalı.".to_owned(),
        ),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let parts = 1 + p.parts.as_ref().map_or(0, Vec::len);
            if part >= parts {
                ("part", format!("Nesnenin {parts} parçası var (0'dan): {part}. parça yok."))
            } else {
                let holes = match part {
                    0 => p.holes.as_ref().map_or(0, Vec::len),
                    k => p.parts.as_ref().and_then(|q| q.get(k - 1)).and_then(|q| q.holes.as_ref()).map_or(0, Vec::len),
                };
                ("ring", format!("{part}. parçanın {holes} deliği var: {ring}. halka yok (0 dış halka, 1'den delikler)."))
            }
        }
        _ => ("uid", "Nesne çizgi, çoklu çizgi ya da alan değil; kotu eğimle verilemez.".to_owned()),
    };
    fail(codes::WRONG_PLACE, why, at)
}

/// What the range's end is given by: the core's target, or the high or low point's two grades.
enum Mode {
    Target(Target),
    Peak { ahead: f64, back: f64 },
}

fn mode(input: &EntitiesGrade) -> Result<Mode, Stop> {
    match (input.end, input.grade, input.ahead, input.back) {
        (_, Some(_), Some(_), _) | (_, Some(_), _, Some(_)) => Err(fail(
            codes::INVALID_INPUT,
            "Eğim (grade) ile başlangıçtan ve bitişten eğimler (ahead, back) birlikte verilmez: ya sabit eğim ya yüksek/alçak nokta."
                .into(),
            "grade",
        )),
        (_, None, Some(_), None) | (_, None, None, Some(_)) => Err(fail(
            codes::INVALID_INPUT,
            "Yüksek/alçak nokta iki eğimle bulunur: başlangıçtan eğimi (ahead) ve bitişten eğimi (back) birlikte verin.".into(),
            if input.ahead.is_some() { "back" } else { "ahead" },
        )),
        (_, None, Some(ahead), Some(back)) => Ok(Mode::Peak { ahead, back }),
        (Some(z), None, None, None) => Ok(Mode::Target(Target::End(z))),
        (None, Some(g), None, None) => Ok(Mode::Target(Target::Grade(g))),
        _ => Err(fail(codes::INVALID_INPUT, "Bitiş kotunu (end) ya da eğimi (grade) verin, ikisini birden değil.".into(), "end")),
    }
}

fn number_or_length(why: &str) -> Stop {
    match why {
        "length" => fail(codes::INVALID_INPUT, "Köşe aralığının yatay uzunluğu sıfır.".into(), "to"),
        _ => fail(codes::INVALID_INPUT, "Kot ve eğim sonlu sayı olmalı.".into(), "start"),
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
    if !matches!(e, Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)) {
        return Err(fail(codes::NOT_A_PATH, "Nesne çizgi, çoklu çizgi ya da alan değil; kotu eğimle verilemez.".into(), "uid"));
    }
    let at = (input.part as usize, input.ring as usize);
    let Some((pts, bulges, zs)) = path_of(e, at) else {
        return Err(no_ring(e, at));
    };
    let (from, to) = (input.from as usize, input.to as usize);
    if !(from < to && to < pts.len()) {
        let whose = if at == (0, 0) { "nesnenin".to_owned() } else { format!("{}. parçanın {}. halkasının", at.0, at.1) };
        return Err(fail(
            codes::WRONG_PLACE,
            format!(
                "Köşe aralığı {}–{} olamaz: {whose} {} köşesi var (0'dan), başlangıç bitişten önce olmalı.",
                input.from,
                input.to,
                pts.len()
            ),
            "to",
        ));
    }
    let mode = mode(input)?;
    let Some(start) = input.start.or(zs[from]) else {
        return Err(fail(codes::ELEVATION_MISSING, format!("{}. köşenin kotu yok; başlangıç kotunu (start) verin.", input.from), "start"));
    };
    let (next, out) = match mode {
        Mode::Target(target) => {
            let g = grade(&pts, bulges.as_deref(), from, to, start, target).map_err(number_or_length)?;
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
                    if let Some((_, _, ring_zs)) = ring_mut(p, at) {
                        *ring_zs = Some(z);
                    }
                }
                _ => {}
            }
            let out = EntitiesGraded { length: g.length, start, end: g.end, grade: g.grade, zs: g.zs, back: None, point: None, revision: String::new() };
            (next, out)
        }
        Mode::Peak { ahead, back } => {
            let Some(end) = input.end.or(zs[to]) else {
                return Err(fail(codes::ELEVATION_MISSING, format!("{}. köşenin kotu yok; bitiş kotunu (end) verin.", input.to), "end"));
            };
            let g = peak(&pts, bulges.as_deref(), from, to, start, end, ahead, back).map_err(|why| match why {
                NoPeak::Length => number_or_length("length"),
                NoPeak::Range | NoPeak::Number => number_or_length("number"),
                NoPeak::Parallel => fail(
                    codes::NO_INTERSECTION,
                    "Başlangıçtan ve bitişten eğimler paralel (biri öbürünün eksisi): hiçbir yerde kesişmezler. Eğimlerden birini değiştirin."
                        .into(),
                    "back",
                ),
                NoPeak::Outside { along, length } => fail(
                    codes::NO_INTERSECTION,
                    format!(
                        "İki eğim aralığın içinde kesişmiyor: kesişme yol boyunca başlangıçtan {along:.3} m'de, aralık 0 ile {length:.3} m arası (uçlar sayılmaz). Eğimleri ya da kotları değiştirin."
                    ),
                    "back",
                ),
            })?;
            let (next, index, inserted) = peaked(e, at, &zs, from, &g);
            let point = GradedPoint {
                index: index as u32,
                inserted,
                along: g.along,
                x: g.p.x,
                y: g.p.y,
                z: g.z,
                kind: match g.kind {
                    PeakKind::High => GradePointKind::High,
                    PeakKind::Low => GradePointKind::Low,
                    PeakKind::Break => GradePointKind::Break,
                },
            };
            let zs = match g.at {
                PeakAt::Vertex(_) => g.zs,
                PeakAt::Edge { edge, .. } => {
                    let mut zs = g.zs;
                    zs.insert(edge + 1 - from, g.z);
                    zs
                }
            };
            let out = EntitiesGraded { length: g.length, start, end, grade: ahead, zs, back: Some(back), point: Some(point), revision: String::new() };
            (next, out)
        }
    };
    let layer = &e.base().layer_id;
    if doc.layers().is_locked(layer) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{}” katmanı kilitli: nesnenin kotları değişemez. Kilidi açın.", doc.layers().path(layer)),
            "uid",
        ));
    }
    Ok((slot, next, out))
}

/// The object through its high or low point: the range's elevations on the ring `at` (docs/adr/4980), a vertex added
/// inside an edge (its bulges split, a line made a polyline), the measures and widths carried; the point's vertex after
/// it and whether it was added.
fn peaked(
    e: &Entity,
    at: (usize, usize),
    zs: &[Option<f64>],
    from: usize,
    g: &kentos_geometry_core::ops::grade::Peaked,
) -> (Entity, usize, bool) {
    let mut z: Vec<Option<f64>> = zs.to_vec();
    for (k, v) in g.zs.iter().enumerate() {
        z[from + k] = Some(*v);
    }
    let (edge, halves) = match g.at {
        PeakAt::Edge { edge, bulges } => (edge, bulges),
        PeakAt::Vertex(vertex) => {
            let mut next = e.clone();
            if let Entity::Polyline(p) | Entity::Polygon(p) = &mut next
                && let Some((_, _, ring_zs)) = ring_mut(p, at)
            {
                *ring_zs = Some(z);
            }
            return (next, vertex, false);
        }
    };
    let point = kentos_commands::Vec2 { x: g.p.x, y: g.p.y };
    z.insert(edge + 1, Some(g.z));
    let mut next = match e {
        Entity::Line(l) => Entity::Polyline(PathEntity {
            space_separator: None,
            base: l.base.clone(),
            pts: vec![l.a, point, l.b],
            bulges: None,
            holes: None,
            zs: Some(z),
            ms: None,
            parts: None,
            closed: false,
            widths: None,
        }),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let mut q = p.clone();
            if let Some((pts, bulges, ring_zs)) = ring_mut(&mut q, at) {
                let n = pts.len();
                pts.insert(edge + 1, point);
                *bulges = bulges.take().map(|mut b| {
                    b.resize(n, 0.0);
                    b[edge] = halves[0];
                    b.insert(edge + 1, halves[1]);
                    b
                });
                *ring_zs = Some(z);
            }
            // The measures and widths come again by the carrying rules below.
            q.ms = None;
            for part in q.parts.iter_mut().flatten() {
                part.ms = None;
            }
            q.widths = None;
            if matches!(e, Entity::Polygon(_)) { Entity::Polygon(q) } else { Entity::Polyline(q) }
        }
        _ => e.clone(),
    };
    // The added vertex's measure and the edges' widths, as an edit carries them (on a source edge: along it).
    crate::elevation::carry_measures(&mut next, &crate::elevation::measure_paths(e), &[], Carry::Along);
    if let Some(source) = crate::widths::source(e) {
        crate::widths::carry(&mut next, &[source], None, Carry::Along);
    }
    (next, edge + 1, true)
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
