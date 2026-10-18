//! An opening between the contract and the shared core (docs/adr/0341 §2): what its symbol is drawn from, and its
//! symbol as the contract's plain objects, which the DXF writer, Patlat and the other formats take where an opening
//! has no kind of its own.

use kentos_contracts::{
    ArcEntity, Entity, EntityBase, LineEntity, OpeningAnchor, OpeningFields, OpeningForm,
    OpeningHand, OpeningOperation, OpeningSide, PathEntity, Vec2,
};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::opening::{self, Anchor, Form, Operation, Spec};

/// The core's anchor.
pub fn anchor(a: OpeningAnchor) -> Anchor {
    match a {
        OpeningAnchor::Start => Anchor::Start,
        OpeningAnchor::Center => Anchor::Center,
        OpeningAnchor::End => Anchor::End,
    }
}

/// What the opening's symbol is drawn from.
pub fn spec(o: &OpeningFields) -> Spec {
    Spec {
        anchor: anchor(o.anchor),
        form: match o.form {
            OpeningForm::Door => Form::Door,
            OpeningForm::Window => Form::Window,
            OpeningForm::Opening => Form::Opening,
        },
        leaves: o.leaves,
        operation: match o.operation {
            OpeningOperation::Swing => Operation::Swing,
            OpeningOperation::Slide => Operation::Slide,
            OpeningOperation::Fixed => Operation::Fixed,
        },
        hand_end: o.hand == OpeningHand::End,
        side_right: o.side == OpeningSide::Right,
        swing: o.swing_angle(),
        show_swing: o.show_swing,
        frame: o.frame.unwrap_or(0.0),
    }
}

/// Its place along its wall, from its start to its end.
pub fn span(o: &OpeningFields) -> (f64, f64) {
    opening::span(o.at, anchor(o.anchor), o.width)
}

fn core(p: &Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

/// The opening as the core's shape.
pub fn core_opening(o: &OpeningFields) -> Shape {
    Shape::Opening {
        at: o.at,
        width: o.width,
        place: o.place.iter().map(core).collect(),
        opening: spec(o),
    }
}

/// Its symbol as the contract's plain objects on its layer, with its look: closed polylines for its frame, lines for
/// its leaves and glass, arcs for its swings. Empty for a bare opening or one without its four jambs.
pub fn plain_pieces(base: &EntityBase, o: &OpeningFields) -> Vec<Entity> {
    let place: Vec<CoreVec2> = o.place.iter().map(core).collect();
    let mut b = base.clone();
    b.attachments.clear();
    b.label_pins.clear();
    b.label = None;
    let back = |p: CoreVec2| Vec2 { x: p.x, y: p.y };
    opening::symbol(&place, &spec(o))
        .into_iter()
        .filter_map(|s| {
            Some(match s {
                Shape::Line { a, b: end } => Entity::Line(LineEntity {
                    base: b.clone(),
                    a: back(a),
                    b: back(end),
                    za: None,
                    zb: None,
                    ma: None,
                    mb: None,
                }),
                Shape::Arc { c, r, a0, a1 } => Entity::Arc(ArcEntity {
                    base: b.clone(),
                    c: back(c),
                    r,
                    a0,
                    a1,
                }),
                Shape::Polyline {
                    pts,
                    bulges,
                    closed,
                    ..
                } => Entity::Polyline(PathEntity {
                    base: b.clone(),
                    pts: pts.into_iter().map(back).collect(),
                    bulges,
                    holes: None,
                    zs: None,
                    ms: None,
                    parts: None,
                    closed,
                    widths: None,
                }),
                _ => return None,
            })
        })
        .collect()
}

/// Every wall's openings by the wall's place in `walls` (indexes into `entities`): the openings of `entities` whose
/// host is one of those walls by its persistent id (`uid_of` an object's id) and that are not broken, as their spans
/// along it.
pub fn spans_by_wall(
    entities: &[Entity],
    walls: &[usize],
    uid_of: impl Fn(u32) -> Option<String>,
) -> Vec<Vec<(f64, f64)>> {
    let mut out = vec![Vec::new(); walls.len()];
    let place: std::collections::HashMap<String, usize> = walls
        .iter()
        .enumerate()
        .filter_map(|(k, &i)| uid_of(entities[i].base().id).map(|u| (u, k)))
        .collect();
    for e in entities {
        if let Entity::Opening(o) = e
            && o.opening.broken.is_none()
            && let Some(&k) = place.get(o.opening.host.as_str())
        {
            out[k].push(span(&o.opening));
        }
    }
    out
}
