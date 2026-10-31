//! An opening between the contract and the shared core (docs/adr/0341 §2): what its symbol is drawn from, and its
//! symbol as the contract's plain objects, which the DXF writer, Patlat and the other formats take where an opening
//! has no kind of its own.

use kentos_contracts::{
    ArcEntity, CircleEntity, Entity, EntityBase, LineEntity, OpeningAnchor, OpeningFields, OpeningForm,
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
            OpeningForm::Niche => Form::Niche,
            OpeningForm::Flue => Form::Flue,
        },
        leaves: o.leaves,
        operation: match o.operation {
            OpeningOperation::Swing => Operation::Swing,
            OpeningOperation::Slide => Operation::Slide,
            OpeningOperation::Fixed => Operation::Fixed,
            OpeningOperation::Revolving => Operation::Revolving,
            OpeningOperation::Pocket => Operation::Pocket,
            OpeningOperation::Folding => Operation::Folding,
        },
        hand_end: o.hand == OpeningHand::End,
        side_right: o.side == OpeningSide::Right,
        swing: o.drawn_swing(),
        show_swing: o.show_swing,
        frame: o.frame.unwrap_or(0.0),
        sill: o.sill,
        head: o.head,
        depth: o.depth,
        inset: o.inset,
        build: build(o.build.as_ref()),
    }
}

/// What plan draws of its construction (docs/adr/0772 §3).
pub fn build(b: Option<&kentos_contracts::OpeningBuild>) -> opening::Build {
    let Some(b) = b else {
        return opening::Build::default();
    };
    opening::Build {
        fit: b.fit.unwrap_or(0.0),
        leaf_thickness: b.leaf_thickness,
        split: b.split,
        frame_depth: b.frame_depth,
        frame_offset: b.frame_offset,
        threshold: b.threshold,
        casing: b.casing,
        sill_board: b.sill_board,
        mullions: b.mullions.unwrap_or(0),
        use_zone: b.use_zone,
        drum: b.drum,
        overlap: b.overlap,
        track: b.track.unwrap_or(false),
        both_sides: b.both_sides.unwrap_or(false),
        lip: b.lip,
    }
}

/// What it takes out of its wall in plan (docs/adr/0719 §3); none when plan does not cut it.
pub fn hole(o: &OpeningFields) -> Option<opening::Hole> {
    opening::hole(o.at, o.width, &spec(o))
}

/// What it takes out of its wall in plan with a pocket door's cavities (docs/adr/0777 §2); none when plan does not cut
/// it.
pub fn holes(o: &OpeningFields) -> Vec<opening::Hole> {
    opening::holes(o.at, o.width, &spec(o))
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
/// its leaves and glass, arcs for its swings, a flue's cross, the dashed outline of one plan does not cut (docs/adr/0719
/// §2). Empty for a cut bare opening or niche or one without its four jambs.
pub fn plain_pieces(base: &EntityBase, o: &OpeningFields) -> Vec<Entity> {
    let place: Vec<CoreVec2> = o.place.iter().map(core).collect();
    let mut plain = base.clone();
    plain.attachments.clear();
    plain.label_pins.clear();
    plain.label = None;
    let back = |p: CoreVec2| Vec2 { x: p.x, y: p.y };
    opening::pieces(&place, &spec(o))
        .into_iter()
        .filter_map(|m| {
            let mut b = plain.clone();
            if m.dashed {
                b.look.line_type = kentos_contracts::LineType::from_name("dashed");
            }
            Some(match m.shape {
                Shape::Line { a, b: end } => Entity::Line(LineEntity {
                    base: b,
                    a: back(a),
                    b: back(end),
                    za: None,
                    zb: None,
                    ma: None,
                    mb: None,
                }),
                Shape::Arc { c, r, a0, a1 } => Entity::Arc(ArcEntity {
                    base: b,
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
                    space_separator: None,
                    base: b,
                    pts: pts.into_iter().map(back).collect(),
                    bulges,
                    holes: None,
                    zs: None,
                    ms: None,
                    parts: None,
                    closed,
                    widths: None,
                }),
                // A revolving door's drum (docs/adr/0777 §1).
                Shape::Circle { c, r } => Entity::Circle(CircleEntity {
                    base: b,
                    c: back(c),
                    r,
                }),
                _ => return None,
            })
        })
        .collect()
}

/// Every wall's openings by the wall's place in `walls` (indexes into `entities`): the openings of `entities` whose
/// host is one of those walls by its persistent id (`uid_of` an object's id), that are not broken and that plan cuts,
/// as their holes along it.
pub fn spans_by_wall(
    entities: &[Entity],
    walls: &[usize],
    uid_of: impl Fn(u32) -> Option<String>,
) -> Vec<Vec<opening::Hole>> {
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
            out[k].extend(holes(&o.opening));
        }
    }
    out
}
