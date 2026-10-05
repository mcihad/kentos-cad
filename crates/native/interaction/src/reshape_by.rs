//! Biçim değiştir (docs/adr/0173 §2–§3): the web's `ReshapeTool`
//! (`reshapeTool.ts`), the path tool's `Shape::Reshape`.
//!
//! - The object is an area, a line or a polyline on an unlocked layer: the
//!   one selected when the tool starts, else the one whose edge is clicked,
//!   else the smallest area around the click. It stays the object after each
//!   reshape; Geri (G) before the first point lets it go.
//! - The line is drawn point by point, straight edges only (no Yay, no
//!   İzle); the preview fills the area it would leave (or draws the path)
//!   and says its size.
//! - Enter writes it (`cad.entities.edit`'s `reshape`, the step “Biçim
//!   değiştir”): an area or a polyline in its place, a line becoming a
//!   polyline with its attributes and label; elevations by the command's
//!   rule. A refusal is said and the line stays to be put right.
//!
//! The reshape is the shared core's (`ops::reshape_by`); none is computed here.

use kentos_contracts::{EditOperation, Entity, EntityEdit};
use kentos_domain::Slot;
use kentos_geometry_core::entity::{Shape, area_parts, entity_area, entity_length, polygon_ring};
use kentos_geometry_core::geom::arc::DEFAULT_STEP;
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::ops::reshape_by::{ReshapeRefusal, reshape};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::spatial::measures;
use crate::tool::{Area, Context, Stroke, Tone};

/// Biçim değiştir: `tool.reshape`.
pub const ID: &str = "reshape";
pub const LABEL: &str = "Biçim değiştir";

/// An object Biçim değiştir does not take.
pub const NOT_SHAPE: &str = "Biçim değiştir alan, çizgi ve çoklu çizgi içindir.";
/// More than one object selected when the tool starts.
pub const MANY: &str = "Biçimi değişecek tek alan ya da çizgi seçin.";
/// A click on no object.
pub const NO_OBJECT: &str = "Biçimi değişecek alana ya da çizgiye tıklayın.";
/// Topoloji on: the neighbours stay as they are (docs/adr/0173 §6).
pub const TOPOLOGY: &str = "Topoloji açık: komşular Biçim değiştir'le değişmez.";

/// The object being reshaped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub slot: Slot,
    /// An area (else a line or a polyline).
    pub area: bool,
}

impl Target {
    /// The object at `slot`, when Biçim değiştir takes it; why not otherwise
    /// (none for one on a locked layer or gone: it is not to be picked).
    pub fn of(slot: Slot, cx: &Context<'_>) -> Result<Target, Option<&'static str>> {
        let e = cx.doc.get(slot).ok_or(None)?;
        if !edge::unlocked(e, cx.doc) {
            return Err(None);
        }
        match e {
            Entity::Polygon(_) => Ok(Target { slot, area: true }),
            Entity::Line(_) | Entity::Polyline(_) => Ok(Target { slot, area: false }),
            _ => Err(Some(NOT_SHAPE)),
        }
    }
}

/// The object at `p`: the one whose edge is there (any kind on an unlocked
/// layer: the others are said to be no shape), else the smallest unlocked
/// area around it.
pub fn pick_at(p: Vec2, cx: &Context<'_>) -> Option<Slot> {
    let doc = &*cx.doc;
    let open = |slot: Slot| doc.get(slot).is_some_and(|e| edge::unlocked(e, doc));
    cx.spatial
        .pick_edge(p, cx.pick_tolerance(), open)
        .or_else(|| {
            cx.spatial
                .containing(p)
                .into_iter()
                .map(|(slot, _)| slot)
                .find(|&slot| open(slot) && matches!(doc.get(slot), Some(Entity::Polygon(_))))
        })
}

/// The refusal in the tool's words.
pub fn refusal_text(why: &ReshapeRefusal, area: bool) -> &'static str {
    match why {
        ReshapeRefusal::NotShape => NOT_SHAPE,
        ReshapeRefusal::TooShort => "Hat en az iki noktalı olmalı.",
        ReshapeRefusal::NoCrossing if area => {
            "Hat alanın sınırını iki kez kesmeli ya da ona değmeli."
        }
        ReshapeRefusal::NoCrossing => "Hat çizgiyi kesmeli ya da ona değmeli.",
        ReshapeRefusal::ManyParts => {
            "Hat alanın birden çok parçasına değiyor; bir parçayı düzenleyin."
        }
        ReshapeRefusal::TouchesHole => {
            "Hat bir deliğe değiyor; deliği Delik araçlarıyla düzenleyin."
        }
        ReshapeRefusal::BothWays => {
            "Hat alanı hem kesiyor hem büyütüyor; ikisini ayrı hatlarla yapın."
        }
        ReshapeRefusal::Apart => "Hattın kapattığı cepler alanı parçalara ayırırdı.",
        // Which part the sketch reshapes is not known (docs/adr/0174).
        ReshapeRefusal::MultiPart => kentos_geometry_core::entity::MULTI_PART_REFUSED,
    }
}

/// A size and its change as the log and the tag say them: `830.00 m²`, `−70.00 m²`.
fn sizes(f: &Format, area: bool, after: f64, before: f64) -> (String, String) {
    let show = |v: f64| if area { f.area(v) } else { f.length(v) };
    let d = after - before;
    let sign = if d < 0.0 { "−" } else { "+" };
    (show(after), format!("{sign}{}", show(d.abs())))
}

/// What is measured: Alan or Uzunluk.
fn noun(area: bool) -> &'static str {
    if area { "Alan" } else { "Uzunluk" }
}

/// The object's size: an area's net area, a path's length.
fn size(e: &Entity, area: bool) -> f64 {
    let (a, l) = measures(e);
    if area { a } else { l }.unwrap_or(0.0)
}

/// What the preview shows of a reshape: the area it leaves or the path it
/// draws, and its size with the change.
#[derive(Clone, Debug, PartialEq)]
pub struct Reshaped {
    pub area: Option<Area>,
    pub stroke: Option<Stroke>,
    pub line: String,
}

/// The object reshaped by the line `sketch` as the preview shows it; none
/// when the core refuses it (the line is still being drawn).
pub fn preview(t: &Target, sketch: &[Vec2], cx: &Context<'_>) -> Option<Reshaped> {
    let e = cx.doc.get(t.slot)?;
    let after = reshape(&edge::core(e).shape, sketch).ok()?;
    let now = if t.area {
        entity_area(&after)
    } else {
        entity_length(&after)
    }
    .unwrap_or(0.0);
    let (now, change) = sizes(&cx.format(), t.area, now, size(e, t.area));
    let line = format!("{} {now} ({change})", noun(t.area));
    let (area, stroke) = match &after {
        Shape::Polygon { .. } => {
            let mut rings = Vec::new();
            for part in area_parts(&after).iter() {
                if let Shape::Polygon {
                    pts, bulges, holes, ..
                } = part
                {
                    rings.push(polygon_ring(pts, bulges.as_deref()));
                    for h in holes.iter().flatten() {
                        rings.push(polygon_ring(&h.pts, h.bulges.as_deref()));
                    }
                }
            }
            let area = Area {
                rings,
                fill: 0.16,
                width: 2.0,
                dash: None,
                fill_tone: Tone::Accent,
            };
            (Some(area), None)
        }
        Shape::Polyline { pts, bulges, .. } => {
            let stroke = Stroke {
                pts: bulge_path_outline(pts, bulges.as_deref(), false, DEFAULT_STEP),
                closed: false,
                dash: None,
                width: 2.5,
                tone: Tone::Accent,
            };
            (None, Some(stroke))
        }
        _ => (None, None),
    };
    Some(Reshaped { area, stroke, line })
}

/// Writes the object reshaped by the line `sketch`, one undo step “Biçim
/// değiştir”. Whether it was written; the refusal is said otherwise.
pub fn write(t: &Target, sketch: &[Vec2], cx: &mut Context<'_>) -> bool {
    let Some(e) = cx.doc.get(t.slot) else {
        return false;
    };
    let before = size(e, t.area);
    let after = match reshape(&edge::core(e).shape, sketch) {
        Ok(shape) => shape,
        Err(why) => {
            cx.say(Level::Warn, refusal_text(&why, t.area));
            return false;
        }
    };
    let Some(geometry) = edge::geometry(&after) else {
        return false;
    };
    let uid = edge::uid(cx.doc, t.slot);
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
    if edge::write(EditOperation::Reshape, vec![change], cx).is_none() {
        return false;
    }
    let now = cx.doc.get(t.slot).map_or(0.0, |e| size(e, t.area));
    let (now, change) = sizes(&cx.format(), t.area, now, before);
    cx.say(
        Level::Success,
        format!("{} {now} oldu ({change}).", noun(t.area)),
    );
    true
}
