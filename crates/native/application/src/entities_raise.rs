//! `cad.entities.raise` v1 (plan/map-cad.md MAP-06, docs/adr/3780): Referanstan kot ver, as Civil 3D's Raise/Lower by
//! Reference. The vertex `at` takes the reference's elevation plus the grade times its horizontal distance from the
//! reference (or plus the difference); every other elevation of the object (a polygon's holes, every part's) moves by
//! as much, so the object's own grades stay. A vertex without an elevation stays without, as Kot ver's Artır leaves it
//! (docs/adr/0142). One undo step. The shared cases are `fixtures/commands/v1/cad.entities.raise.json`.

use kentos_commands::{CommandResult, EntitiesRaise, EntitiesRaised, Entity};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

const STEP: &str = "Referanstan kot ver";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The vertices the command numbers (`at`): a line's two ends, a polyline's first part's, an area's outer ring's;
/// each with its elevation.
fn numbered(e: &Entity) -> Option<Vec<(kentos_commands::Vec2, Option<f64>)>> {
    match e {
        Entity::Line(l) => Some(vec![(l.a, l.za), (l.b, l.zb)]),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            Some(p.pts.iter().enumerate().map(|(i, q)| (*q, p.zs.as_ref().and_then(|z| z.get(i).copied().flatten()))).collect())
        }
        _ => None,
    }
}

/// Every elevation of the object moved by `by`, the vertex `at` set to `after` itself (no rounding left over from the
/// sum); those without one stay without. How many moved and how many have none.
fn raised(e: &Entity, at: usize, by: f64, after: f64) -> (Entity, u32, u32) {
    let (mut moved, mut unset) = (0u32, 0u32);
    let mut lift = |zs: &mut Option<Vec<Option<f64>>>, n: usize| match zs {
        Some(zs) => {
            for z in zs.iter_mut() {
                match z {
                    Some(v) => {
                        *v += by;
                        moved += 1;
                    }
                    None => unset += 1,
                }
            }
        }
        None => unset += n as u32,
    };
    let mut next = e.clone();
    match &mut next {
        Entity::Line(l) => {
            let mut zs = Some(vec![l.za, l.zb]);
            lift(&mut zs, 2);
            let mut zs = zs.unwrap_or_default();
            zs[at] = Some(after);
            l.za = zs[0];
            l.zb = zs[1];
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            lift(&mut p.zs, p.pts.len());
            for h in p.holes.iter_mut().flatten() {
                lift(&mut h.zs, h.pts.len());
            }
            for part in p.parts.iter_mut().flatten() {
                lift(&mut part.zs, part.pts.len());
                for h in part.holes.iter_mut().flatten() {
                    lift(&mut h.zs, h.pts.len());
                }
            }
            if let Some(z) = p.zs.as_mut().and_then(|zs| zs.get_mut(at)) {
                *z = Some(after);
            }
        }
        _ => {}
    }
    (next, moved, unset)
}

fn check(doc: &Document, input: &EntitiesRaise) -> Result<(Slot, Entity, EntitiesRaised), Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    if !checks::is_uid_text(&input.uid) {
        return Err(fail(codes::INVALID_UID, format!("“{}” geçerli bir nesne kimliği değil.", input.uid), "uid"));
    }
    let Some((slot, e)) = Uuid::parse_str(&input.uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s).map(|e| (s, e)))
    else {
        return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{}” kimlikli nesne çizimde yok.", input.uid), "uid"));
    };
    let Some(vertices) = numbered(e) else {
        return Err(fail(
            codes::NOT_A_PATH,
            "Nesne çizgi, çoklu çizgi ya da alan değil; referanstan kot verilemez. Noktaya kotu Kot ver yazar.".into(),
            "uid",
        ));
    };
    let at = input.at as usize;
    let Some(&(p, z)) = vertices.get(at) else {
        return Err(fail(
            codes::WRONG_PLACE,
            format!("{}. köşe yok: nesnenin {} köşesi var, sıra 0'dan başlar.", input.at, vertices.len()),
            "at",
        ));
    };
    let r = input.reference;
    let distance = (p.x - r.x).hypot(p.y - r.y);
    let (after, path) = match (input.grade, input.difference) {
        (Some(g), None) => (r.z + g * distance, "grade"),
        (None, Some(d)) => (r.z + d, "difference"),
        _ => {
            return Err(fail(
                codes::INVALID_INPUT,
                "Referanstan eğimi (grade) ya da kot farkını (difference) verin, ikisini birden değil.".into(),
                "grade",
            ));
        }
    };
    let not_finite = || fail(codes::INVALID_INPUT, "Hesaplanan kot sonlu bir sayı değil; referansı ve eğimi denetleyin.".into(), path);
    if !(distance.is_finite() && after.is_finite()) {
        return Err(not_finite());
    }
    let Some(before) = z else {
        return Err(fail(
            codes::ELEVATION_MISSING,
            format!("{}. köşenin kotu yok; fark bulunamıyor. Kotlu bir köşe seçin ya da önce Kot ver ile kot verin.", input.at),
            "at",
        ));
    };
    let by = after - before;
    if !by.is_finite() {
        return Err(not_finite());
    }
    let layer = &e.base().layer_id;
    if doc.layers().is_locked(layer) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{}” katmanı kilitli: nesnenin kotları değişemez. Kilidi açın.", doc.layers().path(layer)),
            "uid",
        ));
    }
    let (next, moved, unset) = raised(e, at, by, after);
    // The grade given is the grade; a difference's is what it makes over the distance. None on the reference itself,
    // where no grade reaches the vertex.
    let grade = (distance > 0.0).then(|| input.grade.unwrap_or((after - r.z) / distance));
    let out = EntitiesRaised { distance, before, after, by, grade, moved, unset, revision: String::new() };
    Ok((slot, next, out))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesRaise) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What the object would move by; nothing written.
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesRaise) -> CommandResult<EntitiesRaised> {
    match check(cx.doc, input) {
        Ok((_, _, mut out)) => {
            out.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output: out, warnings: Vec::new() }
        }
        Err(stop) => stop.into(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: EntitiesRaise) -> CommandResult<EntitiesRaised> {
    let (slot, next, mut out) = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    cx.doc.update_many(vec![(slot, next)], STEP);
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings: Vec::new() }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: EntitiesRaise) -> CommandResult<EntitiesRaised> {
    crate::call::recorded(cx, kentos_commands::CAD_ENTITIES_RAISE, kentos_commands::CAD_ENTITIES_RAISE_VERSION, input, write)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entity(v: serde_json::Value) -> Entity {
        serde_json::from_value(v).expect("an object")
    }

    #[test]
    fn every_elevation_moves_by_as_much_and_the_vertex_takes_the_result_itself() {
        let area = entity(json!({
            "kind": "polygon", "id": 1, "layerId": "0", "attrs": {},
            "pts": [{"x": 0.0, "y": 0.0}, {"x": 10.0, "y": 0.0}, {"x": 10.0, "y": 10.0}],
            "zs": [100.0, null, 101.0],
            "holes": [{"pts": [{"x": 1.0, "y": 1.0}, {"x": 2.0, "y": 1.0}, {"x": 2.0, "y": 2.0}]}]
        }));
        let (next, moved, unset) = raised(&area, 2, 0.1 + 0.2, 101.3);
        let Entity::Polygon(p) = next else { panic!("an area") };
        assert_eq!(p.zs, Some(vec![Some(100.0 + (0.1 + 0.2)), None, Some(101.3)]));
        assert_eq!((moved, unset), (2, 4), "two moved; one vertex and the hole's three have none");
        let line = entity(json!({
            "kind": "line", "id": 2, "layerId": "0", "attrs": {},
            "a": {"x": 0.0, "y": 0.0}, "b": {"x": 3.0, "y": 4.0}, "za": 5.0
        }));
        let (next, moved, unset) = raised(&line, 0, -1.0, 4.0);
        let Entity::Line(l) = next else { panic!("a line") };
        assert_eq!((l.za, l.zb, moved, unset), (Some(4.0), None, 1, 1));
    }
}
