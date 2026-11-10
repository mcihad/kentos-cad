//! A road drawing drawn again writing only what changed (`road.project.rebuild`'s `regional`, docs/adr/4320 and 4740;
//! plan/map-cad.md MAP-14: a source changed, only the part it reaches is made again).
//!
//! The surface is not kept but made from its sources (docs/adr/0900 §10); what the drawing keeps are the objects drawn
//! from it. So the drawing is made again whole, on a copy of the project, and compared with its objects:
//!
//! - **Alike:** the same objects in the same order, none missing: nothing is written; the record takes its sources'
//!   stamps now and keeps the version and revision of the write that made its objects.
//! - **Changed:** in the command's order each new object takes the first old one exactly alike not taken yet and stays
//!   where that is, with its id, unless the picture could change (docs/adr/4740): an object before it in the order is
//!   written (written objects go to the drawing's end, so they come after it) or stays from a later place, and the two
//!   could draw another picture in the other order. Only strokes draw the same picture in any order: lines and
//!   polylines without a label, of one colour and one transparency, cover the same pixels whichever comes first. A text
//!   and its mask, a point's symbol, a fill keep their order. Such an object is written, and the old one alike stays
//!   free for a later new one. The old objects left are removed, the new ones left written at the drawing's end in its
//!   order. `region` is the box of what was removed and written (a text by its insertion point: its box is its font's).
//!
//! A line drawing (docs/adr/4320) is the case where every object is a stroke of one look: every line alike stays. A
//! sheet's contour lines (docs/adr/4740): the lines alike stay; once a line is written, the heights written after it
//! (masked texts) are written again too, so that their masks stay over the lines.
//!
//! The comparison reads an object's JSON without its number in the document; an object of the drawing that names
//! another of the drawing (a label's object) names it by its index in the drawing, so two drawings made alike compare
//! equal. The objects made on the copy are moved in with the ids they got there, so what they name stays named. A
//! command that changed more than objects on the copy (a setting, a layer, a style, a block, a sheet) is run again in
//! the project, as without `regional`.

use std::collections::{HashMap, VecDeque};

use kentos_commands::{CommandWarning, Entity, RoadDrawingDef, RoadDrawingRebuilt};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::entity_bounds;
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::road_drawings;
use crate::road_project_rebuild::{Refused, redraw, run_drawing};

/// An object's content as the comparison reads it: its JSON without `id`, the drawing's own objects named by `#k`.
fn content(e: &Entity, own: &HashMap<&str, usize>) -> String {
    fn named(v: &mut Value, own: &HashMap<&str, usize>) {
        match v {
            Value::String(s) => {
                if let Some(k) = own.get(s.as_str()) {
                    *s = format!("#{k}");
                }
            }
            Value::Array(a) => a.iter_mut().for_each(|x| named(x, own)),
            Value::Object(o) => o.values_mut().for_each(|x| named(x, own)),
            _ => {}
        }
    }
    let mut v = serde_json::to_value(e).unwrap_or(Value::Null);
    if let Value::Object(o) = &mut v {
        o.remove("id");
    }
    named(&mut v, own);
    v.to_string()
}

/// How an object may change place in the drawing order (docs/adr/4740): a line or a polyline without a label is a
/// stroke of its colour and transparency; anything else keeps its place against what it may overlap.
#[derive(Clone, Debug, PartialEq)]
enum Look {
    Stroke(Option<String>, Option<u64>),
    Ordered,
}

fn look(e: &Entity) -> Look {
    if matches!(e, Entity::Line(_) | Entity::Polyline(_)) && e.base().label.is_none() {
        Look::Stroke(e.base().color.clone(), e.base().look.transparency.map(f64::to_bits))
    } else {
        Look::Ordered
    }
}

/// Whether two objects draw the same picture in either order: strokes of one look.
fn free(a: &Look, b: &Look) -> bool {
    matches!(a, Look::Stroke(..)) && a == b
}

/// Which new objects stay where an old one alike is (the module's rule): in the command's order a new object takes the
/// first old one alike not taken yet, unless an object before it is written, or stays from a later place, and the two
/// are not free in their order; then it is written and the old one stays free for a later one alike.
fn staying(old: &[String], new: &[String], looks: &[Look]) -> Vec<Option<usize>> {
    let mut alike: HashMap<&str, VecDeque<usize>> = HashMap::new();
    for (i, c) in old.iter().enumerate() {
        alike.entry(c.as_str()).or_default().push_back(i);
    }
    // The looks written so far, and each look kept so far with the latest old place one stays at.
    let mut written: Vec<&Look> = Vec::new();
    let mut kept: Vec<(&Look, usize)> = Vec::new();
    let mut stays = vec![None; new.len()];
    for (j, c) in new.iter().enumerate() {
        let l = &looks[j];
        let candidate = alike.get_mut(c.as_str()).and_then(VecDeque::pop_front);
        let blocked = |i: usize| written.iter().any(|w| !free(w, l)) || kept.iter().any(|(k, m)| *m > i && !free(k, l));
        match candidate {
            Some(i) if !blocked(i) => {
                stays[j] = Some(i);
                match kept.iter_mut().find(|(k, _)| *k == l) {
                    Some((_, m)) => *m = (*m).max(i),
                    None => kept.push((l, i)),
                }
            }
            other => {
                if let (Some(i), Some(queue)) = (other, alike.get_mut(c.as_str())) {
                    queue.push_front(i);
                }
                if !written.contains(&l) {
                    written.push(l);
                }
            }
        }
    }
    stays
}

/// The box (min x, min y, max x, max y) of `objects`, a text by its insertion point; none when there are none.
fn region<'a>(objects: impl Iterator<Item = &'a Entity>) -> Option<[f64; 4]> {
    objects.fold(None, |acc, e| {
        let [x0, y0, x1, y1] = match e {
            Entity::Text(t) => [t.p.x, t.p.y, t.p.x, t.p.y],
            _ => {
                let b = entity_bounds(&crate::geometry::shape(e));
                [b.min_x, b.min_y, b.max_x, b.max_y]
            }
        };
        if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
            return acc;
        }
        Some(match acc {
            None => [x0, y0, x1, y1],
            Some([a0, b0, a1, b1]) => [a0.min(x0), b0.min(y0), a1.max(x1), b1.max(y1)],
        })
    })
}

/// Whether running a drawing on `copy` changed more of it than its objects (see the module's note).
fn more_than_objects(doc: &Document, copy: &Document) -> bool {
    copy.settings() != doc.settings()
        || copy.layers().nodes() != doc.layers().nodes()
        || copy.styles() != doc.styles()
        || copy.blocks() != doc.blocks()
        || copy.sheets() != doc.sheets()
}

fn refused_with(stop: Stop) -> Refused {
    Box::new(stop.into())
}

/// Draws `d` again in the step it runs in, writing only what changed (the module's rules). Answers what it did and the
/// command's warnings, or the command's refusal (nothing written).
pub(crate) fn redraw_regional(
    cx: &mut ExecutionContext<'_>,
    d: &RoadDrawingDef,
) -> Result<(RoadDrawingRebuilt, Vec<CommandWarning>), Refused> {
    let Some(main) = road_drawings::main_of(&d.command) else {
        return Err(refused_with(Stop::failed(error(
            codes::UNKNOWN_DRAWING,
            format!("“{}” çiziminin komutu yeniden üretilemez.", d.id),
            None,
        ))));
    };
    // The drawing's objects there now, in its order.
    let present: Vec<(&str, Slot)> = d
        .uids
        .iter()
        .filter_map(|u| Uuid::parse_str(u).ok().and_then(|id| cx.doc.slot_of(id)).map(|s| (u.as_str(), s)))
        .collect();
    let slots: Vec<Slot> = present.iter().map(|(_, s)| *s).collect();
    // Made again on a copy, as drawing it again makes it: its objects removed, its command's check and write.
    let mut copy = cx.doc.reading_copy();
    let (made, warnings) = {
        let mut on_copy = ExecutionContext { doc: &mut copy, store: None, call: None, job: cx.job };
        run_drawing(&mut on_copy, &d.id, &d.command, &d.input, &slots).map_err(|(refused, _)| refused)?
    };
    if more_than_objects(cx.doc, &copy) {
        let (mut r, w) = redraw(cx, d)?;
        r.kept = Some(0);
        return Ok((r, w));
    }
    let old: Vec<&Entity> = slots.iter().filter_map(|s| cx.doc.get(*s)).collect();
    let new_slots: Vec<(Uuid, Slot)> = made
        .iter()
        .filter_map(|u| Uuid::parse_str(u).ok().and_then(|id| copy.slot_of(id).map(|s| (id, s))))
        .collect();
    let new: Vec<&Entity> = new_slots.iter().filter_map(|(_, s)| copy.get(*s)).collect();
    let own_old: HashMap<&str, usize> = present.iter().enumerate().map(|(k, (u, _))| (*u, k)).collect();
    let own_new: HashMap<&str, usize> = made.iter().enumerate().map(|(k, u)| (u.as_str(), k)).collect();
    let old_c: Vec<String> = old.iter().map(|e| content(e, &own_old)).collect();
    let new_c: Vec<String> = new.iter().map(|e| content(e, &own_new)).collect();
    let rebuilt = |removed: usize, kept: usize, region: Option<[f64; 4]>, uids: Vec<String>| RoadDrawingRebuilt {
        drawing: d.id.clone(),
        command: d.command.clone(),
        removed: u32::try_from(removed).unwrap_or(u32::MAX),
        kept: Some(u32::try_from(kept).unwrap_or(u32::MAX)),
        region,
        uids,
    };

    // Alike: nothing written, the record renewed.
    if present.len() == d.uids.len() && old_c == new_c {
        let kept = old.len();
        road_drawings::renew(cx.doc, d, main).map_err(refused_with)?;
        return Ok((rebuilt(0, kept, None, d.uids.clone()), warnings));
    }

    // Which objects stay (the module's rule).
    let looks: Vec<Look> = new.iter().map(|e| look(e)).collect();
    let stays = staying(&old_c, &new_c, &looks);
    let mut taken = vec![false; old.len()];
    stays.iter().flatten().for_each(|&i| taken[i] = true);
    let gone: Vec<Slot> = (0..old.len()).filter(|&i| !taken[i]).map(|i| slots[i]).collect();
    let written: Vec<(Uuid, Entity)> =
        (0..new.len()).filter(|&j| stays[j].is_none()).map(|j| (new_slots[j].0, new[j].clone())).collect();
    let changed = region(gone.iter().filter_map(|s| cx.doc.get(*s)).chain(written.iter().map(|(_, e)| e)));
    let removed = gone.len();
    let kept = new.len() - written.len();
    let uids: Vec<String> = (0..new.len())
        .map(|j| stays[j].map_or_else(|| new_slots[j].0.to_string(), |i| present[i].0.to_owned()))
        .collect();
    if !gone.is_empty() {
        cx.doc.remove(&gone);
    }
    if !written.is_empty() {
        cx.doc
            .add_many_with_uids(written, "Yeniden üret")
            .map_err(|e| refused_with(Stop::failed(error(codes::ROAD_DRAWINGS_FULL, format!("Çizim yazılamadı: {}.", e.0), None))))?;
    }
    road_drawings::keep(cx.doc, &d.command, &d.input, main, &uids, Some(&d.id)).map_err(refused_with)?;
    Ok((rebuilt(removed, kept, changed, uids), warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{CallOp, DocumentSnapshotV2};
    use serde_json::json;

    /// The shared cases' map project with the ground's contour lines drawn (cz-1) and one point of a hill raised: the
    /// drawing stale (`road.project.rebuild.regional.json`'s first case).
    fn stale() -> Document {
        let cases: Value =
            serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.rebuild.regional.json")).expect("cases");
        let snapshot: DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("a snapshot");
        let mut doc = Document::from_snapshot_v2(snapshot).expect("opens");
        let steps = cases["cases"][0]["steps"].as_array().expect("steps");
        for step in &steps[..2] {
            let mut input = step["input"].clone();
            // `$uidOf:n`, as the runner reads it.
            if let Some(change) = input.pointer_mut("/changes/0/uid") {
                let slot: u32 = change.as_str().and_then(|t| t.strip_prefix("$uidOf:")).and_then(|n| n.parse().ok()).expect("a slot");
                *change = json!(doc.uid(Slot(slot)).expect("an object").to_string());
            }
            let command = step["command"].as_str().expect("a command");
            let r = crate::registry::run(&mut ExecutionContext::new(&mut doc), command, CallOp::Execute, input).expect("runs");
            assert_eq!(r["status"], "completed", "{command}: {r}");
        }
        doc
    }

    fn rebuild(doc: &mut Document, regional: bool) -> Value {
        let input = json!({ "regional": regional });
        crate::registry::run(&mut ExecutionContext::new(doc), kentos_commands::ROAD_PROJECT_REBUILD, CallOp::Execute, input).expect("runs")
    }

    /// The drawing's objects in its order, as the comparison reads them.
    fn drawn(doc: &Document) -> Vec<String> {
        let d = &doc.road_drawings()[0];
        let own: HashMap<&str, usize> = d.uids.iter().enumerate().map(|(k, u)| (u.as_str(), k)).collect();
        d.uids
            .iter()
            .map(|u| content(doc.get(doc.slot_of(Uuid::parse_str(u).expect("a uid")).expect("there")).expect("an object"), &own))
            .collect()
    }

    /// Writing only what changed leaves the drawing as drawing it whole does: the same objects in the same order, the
    /// same record digest and stamps; the lines that stayed keep their ids.
    #[test]
    fn regional_is_the_whole_drawing_with_the_unchanged_lines_kept() {
        let mut whole = stale();
        let mut part = stale();
        let before: Vec<String> = part.road_drawings()[0].uids.clone();
        let r = rebuild(&mut whole, false);
        assert_eq!(r["status"], "completed", "{r}");
        let r = rebuild(&mut part, true);
        assert_eq!(r["status"], "completed", "{r}");
        assert_eq!(drawn(&whole), drawn(&part));
        let (w, p) = (&whole.road_drawings()[0], &part.road_drawings()[0]);
        assert_eq!(w.digest, p.digest);
        assert_eq!(w.sources, p.sources);
        let kept = r["output"]["rebuilt"][0]["kept"].as_u64().expect("kept");
        let stayed = p.uids.iter().filter(|u| before.contains(u)).count();
        assert_eq!(stayed as u64, kept);
        assert!(kept > 0 && (kept as usize) < p.uids.len());
        assert!(w.uids.iter().all(|u| !before.contains(u)), "drawn whole, every line is new");
    }

    /// One colour and one transparency, lines and polylines without a label: free in their order; a second colour, a
    /// label or a point is not.
    #[test]
    fn only_strokes_of_one_look_are_free_in_their_order() {
        let doc = stale();
        let lines: Vec<Entity> = doc.road_drawings()[0]
            .uids
            .iter()
            .filter_map(|u| doc.slot_of(Uuid::parse_str(u).ok()?).and_then(|s| doc.get(s)).cloned())
            .collect();
        let (a, b) = (look(&lines[0]), look(&lines[1]));
        assert!(free(&a, &b));
        let mut red = lines[0].clone();
        if let Entity::Polyline(p) = &mut red {
            p.base.color = Some("#E5484D".into());
        }
        assert!(!free(&look(&red), &b));
        let mut labelled = lines[1].clone();
        if let Entity::Polyline(p) = &mut labelled {
            p.base.label = Some("120".into());
        }
        assert!(!free(&a, &look(&labelled)));
        let point = doc.entities().next().cloned().expect("a ground point");
        assert!(!free(&look(&point), &look(&point)), "a point keeps its order even against its like");
    }

    /// The rule's walk (docs/adr/4740): strokes alike stay whatever is written before them; an ordered object stays
    /// only while nothing before it is written and no ordered one before it stays from a later place.
    #[test]
    fn an_ordered_object_after_a_written_one_is_written_too() {
        let s = Look::Stroke(None, None);
        let o = Look::Ordered;
        let c = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        // Lines a, b, c and a text t: b changed to b2.
        let old = c(&["a", "b", "c", "t"]);
        let new = c(&["a", "b2", "c", "t"]);
        assert_eq!(staying(&old, &new, &[s.clone(), s.clone(), s.clone(), o.clone()]), vec![Some(0), None, Some(2), None]);
        // Nothing written before the text: it stays.
        let new = c(&["a", "b", "c", "t"]);
        assert_eq!(staying(&old, &new, &[s.clone(), s.clone(), s.clone(), o.clone()]), vec![Some(0), Some(1), Some(2), Some(3)]);
        // Two texts in the other order: the second stays from an earlier place, the first is written.
        let old = c(&["t1", "t2"]);
        let new = c(&["t2", "t1"]);
        assert_eq!(staying(&old, &new, &[o.clone(), o.clone()]), vec![Some(1), None]);
        // Two strokes of one look in the other order: both stay.
        assert_eq!(staying(&old, &new, &[s.clone(), s.clone()]), vec![Some(1), Some(0)]);
        // A written object leaves its old one alike to a later one alike.
        let old = c(&["t", "x"]);
        let new = c(&["y", "t", "t"]);
        assert_eq!(staying(&old, &new, &[o.clone(), o.clone(), o.clone()]), vec![None, None, None]);
        assert_eq!(staying(&old, &new, &[s.clone(), s.clone(), s.clone()]), vec![None, Some(0), None]);
    }

    /// A drawing's object naming another of the drawing reads its index, so two drawings made alike compare equal.
    #[test]
    fn an_object_named_by_the_drawing_reads_its_index() {
        let doc = stale();
        let e = doc.entities().next().cloned().expect("an object");
        let mut a = e.clone();
        a.base_mut().attrs.insert("Bağlı".into(), "u-old".into());
        let mut b = e;
        b.base_mut().attrs.insert("Bağlı".into(), "u-new".into());
        let old = HashMap::from([("u-old", 3)]);
        let new = HashMap::from([("u-new", 3)]);
        assert_eq!(content(&a, &old), content(&b, &new));
        assert_ne!(content(&a, &old), content(&b, &HashMap::from([("u-new", 4)])));
    }

    /// A run that changed a setting on the copy is no objects-only run.
    #[test]
    fn a_setting_changed_on_the_copy_is_more_than_objects() {
        let doc = stale();
        let mut copy = doc.reading_copy();
        assert!(!more_than_objects(&doc, &copy));
        let mut settings = copy.settings().clone();
        settings.plot_scale = 500.0;
        copy.set_settings(settings);
        assert!(more_than_objects(&doc, &copy));
    }
}
