//! A wall changed at a fixed reference and its node decisions (docs/adr/0771 §1, §2): `cad.walls.edit`'s `keep` moves
//! the wall's offset so the line asked for (its path, its centre, a finished face, a core face) stays where it was when
//! its width or its style changes, and says what that does round it (the spaces' areas, the openings' depths); its
//! `startPass` and `endPass` keep the user's decision at a tee, refused where the end tees into no straight wall.

use kentos_commands::{
    Entity, OpeningDepthChange, SpaceAreaChange, WallChange, WallChangeEffects, WallEntity,
    WallFields, WallReference,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::wall_join::{JointKind, WallIn, solve};

use crate::checks::{Stop, error};
use crate::codes;
use crate::walls;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// `invalid_wall` for a fixed reference given with an offset or a justification: the reference sets the offset.
pub(crate) fn keep_alone(c: &WallChange) -> Result<(), Stop> {
    if c.keep.is_some() && (c.offset.is_some() || c.justify.is_some()) {
        return Err(fail(
            codes::INVALID_WALL,
            "Sabit referans duvarın ötelemesini kendisi bulur; öteleme ya da hiza ile birlikte verilmez.".into(),
            "operation.keep",
        ));
    }
    Ok(())
}

/// `after` with its offset moved so the line `r` stands where it stood on `before` (its width or components changed);
/// left as it is when either's components do not fit (its check then says so).
pub(crate) fn kept(before: &WallFields, after: &mut WallFields, r: WallReference) {
    if let (Some(was), Some(now)) = (before.reference_at(r), after.reference_at(r)) {
        after.offset += was - now;
    }
}

/// What the walls written (`updates`) do round them when kept at a reference: each space the step's followers change
/// with its area before and after, each opening of a changed wall whose depth (its wall's width) changes.
pub(crate) fn effects(doc: &Document, updates: &[(Slot, WallEntity)]) -> WallChangeEffects {
    let mut out = WallChangeEffects::default();
    let changes: Vec<(Slot, Entity)> = updates
        .iter()
        .map(|(s, w)| (*s, Entity::Wall(w.clone())))
        .collect();
    let delta = doc.preview_all_followers(&changes);
    let area = |s: &kentos_commands::SpaceEntity| {
        let parts: Vec<_> = s
            .space
            .parts
            .iter()
            .map(kentos_formats::spaces::area)
            .collect();
        kentos_geometry_core::geom::space::area(&parts)
    };
    for (_, e) in &delta.updated {
        let Entity::Space(after) = e else { continue };
        let slot = Slot(after.base.id);
        let Some(Entity::Space(before)) = doc.get(slot) else {
            continue;
        };
        let (a, b) = (area(before), area(after));
        if (a - b).abs() > 1e-12 {
            out.spaces.push(SpaceAreaChange {
                uid: doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
                before: a,
                after: b,
            });
        }
    }
    for (slot, w) in updates {
        let Some(Entity::Wall(old)) = doc.get(*slot) else {
            continue;
        };
        if (old.wall.width - w.wall.width).abs() <= 1e-12 {
            continue;
        }
        let Some(host) = doc.uid(*slot).map(|u| u.to_string()) else {
            continue;
        };
        for e in doc.entities() {
            if let Entity::Opening(o) = e
                && o.opening.host == host
            {
                out.openings.push(OpeningDepthChange {
                    uid: doc
                        .uid(Slot(o.base.id))
                        .map(|u| u.to_string())
                        .unwrap_or_default(),
                    before: old.wall.width,
                    after: w.wall.width,
                });
            }
        }
    }
    out
}

/// `wall_node_unsupported` for a node decision on an end of `w` (at `slot`, its fields as they would be) that tees into
/// no straight wall: a corner, an inline join, a free end, a curved joining edge or host.
pub(crate) fn node_supported(
    doc: &Document,
    slot: Slot,
    w: &WallEntity,
    start: bool,
    path: &str,
) -> Result<(), Stop> {
    let refuse = |why: &str| {
        Err(fail(
            codes::WALL_NODE_UNSUPPORTED,
            format!(
                "Düğüm kararı verilemez: {why} Kararı kaldırın (null) ya da ucu düz bir duvarın içine uzatın."
            ),
            path,
        ))
    };
    let end_curved = w.wall.bulges.as_ref().is_some_and(|b| {
        let n = w.wall.pts.len();
        let k = if start { 0 } else { n.saturating_sub(2) };
        b.get(k).is_some_and(|x| *x != 0.0)
    });
    if end_curved {
        return refuse(
            "duvarın bu ucundaki kenar yay; eğrisel birleşimde bileşen kararı desteklenmez.",
        );
    }
    let changed = [(Some(slot), w)];
    let (shapes, others) = walls::neighbourhood(doc, &changed, &[]);
    let slots: Vec<Slot> = std::iter::once(slot).chain(others).collect();
    let ins: Vec<WallIn<'_>> = shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Wall { pts, bulges, wall } => Some(WallIn {
                pts,
                bulges: bulges.as_deref(),
                spec: wall,
            }),
            _ => None,
        })
        .collect();
    let solved = solve(&ins, Some(&[]));
    let end = if start {
        kentos_geometry_core::geom::wall_join::End::Start
    } else {
        kentos_geometry_core::geom::wall_join::End::End
    };
    let joint = solved
        .joints
        .iter()
        .find(|j| j.members.iter().any(|m| m.wall == 0 && m.end == Some(end)));
    let Some(joint) = joint else {
        return refuse("bu uç hiçbir duvara birleşmiyor (serbest uç ya da birleşmeyen köşe).");
    };
    if joint.kind != JointKind::Tee {
        return refuse(match joint.kind {
            JointKind::Corner => {
                "bu uç bir köşede; köşede bileşenler gönyeyle birleşir, Alın ya da Geçen kullanın."
            }
            JointKind::Inline => {
                "bu uç düz devam ediyor; düz devamda bileşenler karşı karşıya biter."
            }
            _ => "bu uç bir T birleşiminde değil.",
        });
    }
    for m in joint.members.iter().filter(|m| m.end.is_none()) {
        let Some(Entity::Wall(host)) = slots.get(m.wall).and_then(|s| doc.get(*s)) else {
            continue;
        };
        if host
            .wall
            .bulges
            .as_ref()
            .is_some_and(|b| b.iter().any(|x| *x != 0.0))
        {
            return refuse("konak duvar eğrisel; eğrisel konakta bileşen kararı desteklenmez.");
        }
    }
    Ok(())
}
