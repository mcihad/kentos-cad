//! Paralel kaydır with a reference (docs/adr/2580): the edge put at a distance from another object's straight edge
//! (“şimdi bu değere getir”), and with Bu uzaklığı koru the distance kept as a map bond (“bu ilişkiyi koru”) that the
//! bond adapter follows when the reference moves (`map_bonds`, `arch_bonds`). `shape_edits` asks it before it writes.

use kentos_commands::{
    CadElement, ConstraintRule, DistanceAxis, ElementRef, Entity, EntityId, ShiftReference,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::ops::arch_bonds::{PARALLEL, TIE};
use kentos_geometry_core::ops::edge_shift::Picked;
use kentos_geometry_core::ops::subref::{EdgeKind, Place, edge_line, edge_place_near};
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::shape;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn unit(a: Vec2, b: Vec2) -> Vec2 {
    let l = (b.x - a.x).hypot(b.y - a.y);
    Vec2::new((b.x - a.x) / l, (b.y - a.y) / l)
}

/// The bond Bu uzaklığı koru writes after the shift, and the value its parameter takes (metres; none: what the drawing
/// measures, the distance there is now).
#[derive(Clone, Debug)]
pub(crate) struct Kept {
    pub rule: ConstraintRule,
    pub value: Option<f64>,
}

/// The edge `picked` of object `slot` (`e`) and its reference `r`: the shift along the edge's normal (Paralel kaydır's
/// `d`) that puts it `distance` from the reference's line on the side it is on now (none: 0, only the bond), and the
/// bond `keep` asks for. Refused, nothing written: a target area, no distance and no keep, a negative distance, an arc
/// edge, a reference that is not a straight segment or is the edge itself, edges not parallel within 0.001°, an edge on
/// the reference's line asked to leave it, a kept edge on a locked layer or governed by another relation.
#[allow(clippy::too_many_arguments)]
pub(crate) fn by_reference(
    doc: &Document,
    slot: Slot,
    e: &Entity,
    picked: &Picked,
    distance: Option<f64>,
    area: Option<f64>,
    r: &ShiftReference,
) -> Result<(f64, Option<Kept>), Stop> {
    let refused = |text: String, path: &str| refuse(codes::EDGE_SHIFT_REFUSED, text, path);
    if area.is_some() {
        return Err(refuse(
            codes::INVALID_INPUT,
            "Referansla hedef alan verilmez: referansa uzaklığı (distance) verin.",
            "work.area",
        ));
    }
    match distance {
        Some(d) => {
            checks::finite(d, "Uzaklık", "Sonlu bir sayı verin.", "work.distance")?;
            if d < 0.0 {
                return Err(refuse(
                    codes::INVALID_DISTANCE,
                    "Referansa uzaklık eksi olamaz: kenar bulunduğu yanda kalır; sıfır ya da daha büyük bir uzaklık yazın.",
                    "work.distance",
                ));
            }
        }
        None if !r.keep => {
            return Err(refuse(
                codes::INVALID_INPUT,
                "Referansla bir uzaklık (distance) verin ya da şimdiki uzaklığı Bu uzaklığı koru (keep) ile bağlayın.",
                "work.distance",
            ));
        }
        None => {}
    }
    checks::point(r.at, "Referansın noktasının", "work.reference.at")?;
    let (rslot, re) = crate::shape_edits::object(doc, &r.id, "work.reference.id")?;
    if picked.bulge != 0.0 {
        return Err(refused(
            "Yaylı kenarın bir doğruya uzaklığı tek bir sayı değil; referansla yalnız düz kenar kaydırılır.".into(),
            "work.at",
        ));
    }
    // The reference's straight segment nearest its point, as a click there picks it (docs/adr/0320 §1).
    let core = CoreEntity::new(shape(re));
    let found = edge_place_near(&core, EdgeKind::Straight, Vec2::new(r.at.x, r.at.y))
        .and_then(|p| Some((p, edge_line(&core, p)?)));
    let Some((place, (ra, rb))) = found.filter(|(_, (a, b))| a != b) else {
        return Err(refused(
            "Referans düz bir kenar olmalı: bir çizgi ya da bir yolun veya alanın düz kenarı.".into(),
            "work.reference.id",
        ));
    };
    let (rpart, rring, rindex) = match place {
        Place::Along {
            part, ring, index, ..
        }
        | Place::Mid { part, ring, index } => (part, ring, index),
        _ => (0, 0, 0),
    };
    if rslot == slot && (rpart, rring, rindex) == (0, picked.ring, picked.edge) {
        return Err(refused(
            "Kenar kendisinin referansı olamaz; başka bir kenar seçin.".into(),
            "work.reference.at",
        ));
    }
    let (u, w) = (unit(ra, rb), unit(picked.a, picked.b));
    let sine = (u.x * w.y - u.y * w.x).abs();
    if sine > PARALLEL {
        return Err(refused(
            format!(
                "Kenar referansa paralel değil (aralarında {:.4}° var); uzaklık yalnız 0,001° içinde paralel kenarlar arasında tutulur.",
                sine.min(1.0).asin().to_degrees()
            ),
            "work.reference.at",
        ));
    }
    // The edge's signed distance from the reference's line (its left positive), and how a shift along the edge's
    // normal changes it (±1: the two are parallel).
    let n = Vec2::new(-u.y, u.x);
    let g0 = n.x * (picked.a.x - ra.x) + n.y * (picked.a.y - ra.y);
    let k = n.x * picked.normal.x + n.y * picked.normal.y;
    let d = match distance {
        None => 0.0,
        Some(target) => {
            if g0.abs() <= TIE && target > 0.0 {
                return Err(refused(
                    "Kenar referansın doğrusu üstünde; hangi yana kayacağı belli değil. Önce kenarı Paralel kaydır ile bir yana alın.".into(),
                    "work.distance",
                ));
            }
            let side = if g0 >= 0.0 { 1.0 } else { -1.0 };
            (side * target - g0) / k
        }
    };
    if !r.keep {
        return Ok((d, None));
    }
    let layer = &e.base().layer_id;
    if distance.is_none() && doc.layers().is_locked(layer) {
        let name = doc.layers().get(layer).map_or(layer.as_str(), |l| l.name.as_str());
        return Err(refuse(
            codes::LAYER_LOCKED,
            format!("“{name}” katmanı kilitli; kenara bağ kurulmaz. Kilidi Katmanlar panelinden açın."),
            "work.id",
        ));
    }
    if let Some(why) = doc.governed(e) {
        return Err(refuse(
            codes::CONSTRAINT_GOVERNED,
            format!("Bağ bu nesneye konmaz: {why}."),
            "work.id",
        ));
    }
    let uid = |s: Slot| doc.uid(s).map(|u| EntityId(u.into_bytes()));
    let (Some(of_reference), Some(of_edge)) = (uid(rslot), uid(slot)) else {
        return Err(refused("Nesnenin kimliği yok.".into(), "work.id"));
    };
    let index = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let rule = ConstraintRule::Distance {
        a: ElementRef {
            uid: of_reference,
            element: CadElement::Segment {
                part: index(rpart),
                ring: index(rring),
                index: index(rindex),
            },
        },
        b: ElementRef {
            uid: of_edge,
            element: CadElement::Boundary {
                part: 0,
                ring: index(picked.ring),
                index: index(picked.edge),
            },
        },
        axis: DistanceAxis::Aligned,
        left: false,
        parameter: String::new(),
    };
    Ok((d, Some(Kept { rule, value: distance })))
}
