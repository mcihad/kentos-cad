//! Sürdür (docs/adr/0173 §4): the web's `ContinueTool` (`continueTool.ts`),
//! the path tool's `Shape::Continue`.
//!
//! - The object is a line or an open polyline on an unlocked layer: the one
//!   selected when the tool starts (it goes on from the end nearer the
//!   pointer until the first new point), else the one whose edge is clicked
//!   (from the end nearer the click). Any other object: “Sürdür çizgi ve
//!   çoklu çizgi içindir.”
//! - Its end is the path's first point; the path is drawn as Çoklu çizgi's,
//!   an arc going on along the object's end tangent.
//! - Enter writes the object continued (`cad.entities.edit`'s `continue`,
//!   the step “Sürdür”): a polyline in its place, a line becoming one with
//!   its attributes and label. Its vertices keep their elevations; the new
//!   ones have none.
//!
//! The splice and the end directions are the shared core's
//! (`ops::continuation`); none is computed here.

use kentos_contracts::{EditOperation, Entity, EntityEdit, EntityGeometry};
use kentos_domain::Slot;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::continuation::{Ends, continue_path, path_ends};

use crate::Vec2;
use crate::edge;
use crate::log::Level;
use crate::spatial::measures;
use crate::tool::Context;

/// Sürdür: `tool.continue`.
pub const ID: &str = "continue";
pub const LABEL: &str = "Sürdür";

/// An object Sürdür does not take.
pub const NOT_PATH: &str = "Sürdür çizgi ve çoklu çizgi içindir.";
/// A click on no object.
pub const NO_OBJECT: &str = "Sürdürülecek çizgiye ya da çoklu çizgiye, ucunun yakınında tıklayın.";
/// Enter with the end alone.
pub const NOTHING_DRAWN: &str = "Sürdürmek için en az bir nokta verin.";

/// The line or polyline being continued, and the end it goes on from.
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub slot: Slot,
    ends: Ends,
    /// From its first end (else its last).
    pub from_first: bool,
    /// The end follows the pointer until the first new point (the object was selected beforehand).
    pub open: bool,
}

impl Target {
    /// The object at `slot`, when Sürdür takes it; why not otherwise (none
    /// for one on a locked layer or gone: it is not to be picked).
    pub fn of(slot: Slot, cx: &Context<'_>) -> Result<Target, Option<&'static str>> {
        let e = cx.doc.get(slot).ok_or(None)?;
        if !edge::unlocked(e, cx.doc) {
            return Err(None);
        }
        if !matches!(e, Entity::Line(_) | Entity::Polyline(_)) {
            return Err(Some(NOT_PATH));
        }
        let ends = path_ends(&edge::core(e).shape).ok_or(Some(NOT_PATH))?;
        Ok(Target {
            slot,
            ends,
            from_first: false,
            open: false,
        })
    }

    /// The end the path starts at.
    pub fn end(&self) -> Vec2 {
        if self.from_first {
            self.ends.first
        } else {
            self.ends.last
        }
    }

    /// The direction out of that end: an arc drawn first goes on along it.
    pub fn out(&self) -> Option<Vec2> {
        if self.from_first {
            self.ends.out_first
        } else {
            self.ends.out_last
        }
    }

    /// Goes on from the end nearer `p` (the last on a tie).
    pub fn nearer(&mut self, p: Vec2) {
        self.from_first = dist(p, self.ends.first) < dist(p, self.ends.last);
    }
}

/// The object whose edge is at `p`, of any kind on an unlocked layer (the
/// others are said to be no line).
pub fn pick_at(p: Vec2, cx: &Context<'_>) -> Option<Slot> {
    let doc = &*cx.doc;
    cx.spatial.pick_edge(p, cx.pick_tolerance(), |slot| {
        doc.get(slot).is_some_and(|e| edge::unlocked(e, doc))
    })
}

/// The object's vertex elevations in the order of its vertices, none when
/// it has none.
fn elevations(e: &Entity) -> Option<Vec<Option<f64>>> {
    let zs = match e {
        Entity::Line(l) => vec![l.za, l.zb],
        Entity::Polyline(p) => p.zs.clone()?,
        _ => return None,
    };
    zs.iter().any(Option::is_some).then_some(zs)
}

/// Writes the object continued by the drawn path `drawn` (its first point
/// the end) with one bulge per drawn segment, one undo step “Sürdür”.
/// Whether it was written; the command's refusal is said otherwise.
pub fn write(target: &Target, drawn: &[Vec2], bulges: &[f64], cx: &mut Context<'_>) -> bool {
    let Some(e) = cx.doc.get(target.slot) else {
        return false;
    };
    let before = edge::core(e).shape;
    let (old_length, old_count) = (
        measures(e).1.unwrap_or(0.0),
        match e {
            Entity::Polyline(p) => p.pts.len(),
            _ => 2,
        },
    );
    let Some(after) = continue_path(&before, target.from_first, drawn, bulges) else {
        cx.say(Level::Warn, NOTHING_DRAWN);
        return false;
    };
    let Some(mut geometry) = edge::geometry(&after) else {
        return false;
    };
    // Its own vertices keep their elevations; the new ones have none.
    if let (
        Some(zs),
        EntityGeometry::Polyline {
            pts, zs: written, ..
        },
    ) = (elevations(e), &mut geometry)
        && pts.len() >= zs.len()
    {
        let added = vec![None; pts.len().saturating_sub(zs.len())];
        *written = Some(if target.from_first {
            added.into_iter().chain(zs).collect()
        } else {
            zs.into_iter().chain(added).collect()
        });
    }
    let uid = edge::uid(cx.doc, target.slot);
    // A line becomes a polyline: in its place, with its attributes and label.
    let change = if matches!(e, Entity::Line(_)) {
        EntityEdit::Replace {
            uid,
            geometry,
            keep_data: Some(true),
        }
    } else {
        EntityEdit::Update { uid, geometry }
    };
    if edge::write(EditOperation::Continue, vec![change], cx).is_none() {
        return false;
    }
    let (length, count) = cx.doc.get(target.slot).map_or((0.0, 0), |e| {
        (
            measures(e).1.unwrap_or(0.0),
            match e {
                Entity::Polyline(p) => p.pts.len(),
                _ => 2,
            },
        )
    });
    let f = cx.format();
    let text = format!(
        "Sürdürüldü: {} köşe eklendi; uzunluk {} oldu (+{}).",
        count.saturating_sub(old_count),
        f.length(length),
        f.length(length - old_length)
    );
    cx.say(Level::Success, text);
    true
}
