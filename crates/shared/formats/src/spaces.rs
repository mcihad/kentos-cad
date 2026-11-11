//! A space between the contract and the shared core (docs/adr/0342 §2): its parts as the core's areas and what its
//! label and its following need.

use kentos_contracts::{SpaceFields, SpaceMethod, SpacePart, SpaceRing, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::space::{Method, Spec};

/// The core's method.
pub fn method(m: SpaceMethod) -> Method {
    match m {
        SpaceMethod::Net => Method::Net,
        SpaceMethod::Centerline => Method::Centerline,
        SpaceMethod::Gross => Method::Gross,
    }
}

fn ring(r: &SpaceRing) -> Ring {
    Ring {
        pts: r.pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect(),
        bulges: r.bulges.clone(),
    }
}

/// A part as the core's area.
pub fn area(p: &SpacePart) -> Area {
    Area {
        outer: ring(&p.outer),
        holes: p.holes.iter().map(ring).collect(),
    }
}

/// The core's area back as the contract's part.
pub fn part(a: &Area) -> SpacePart {
    let ring = |r: &Ring| SpaceRing {
        pts: r.pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect(),
        bulges: r.bulges.clone(),
    };
    SpacePart {
        outer: ring(&a.outer),
        holes: a.holes.iter().map(ring).collect(),
    }
}

/// What the core's space carries beside its rings.
pub fn spec(s: &SpaceFields) -> Spec {
    Spec {
        seed: s.seed.map(|p| CoreVec2::new(p.x, p.y)),
        method: method(s.method),
        number: s.number.clone(),
        name: s.name.clone(),
        void: s.void,
        slab: None,
    }
}

/// The space as the core's shape.
pub fn core_space(s: &SpaceFields) -> Shape {
    Shape::Space {
        parts: s.parts.iter().map(area).collect(),
        space: spec(s),
    }
}

/// A slab's look in the core (docs/adr/0722): its edge's kind, width and runs.
pub fn slab_look(s: &kentos_contracts::SlabFields) -> kentos_geometry_core::geom::slab::Look {
    use kentos_geometry_core::geom::slab::{Edge, Look, Run};
    Look {
        edge: s.edge.as_ref().map(|e| Edge {
            parapet: e.kind == kentos_contracts::SlabEdgeKind::Parapet,
            width: e.width,
            runs: e
                .runs
                .iter()
                .map(|r| Run {
                    pts: r.pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect(),
                    bulges: r.bulges.clone(),
                })
                .collect(),
        }),
        overhead: false,
    }
}

/// The slab as the core's shape (docs/adr/0722): its rings as a space's, drawn by its look; a slab bound to walls
/// carries its seed as a space does, so a transform moves it with the rings (docs/adr/8050 §2).
pub fn core_slab(s: &kentos_contracts::SlabFields) -> Shape {
    Shape::Space {
        parts: s.parts.iter().map(area).collect(),
        space: Spec {
            seed: s
                .bound
                .as_ref()
                .and_then(|b| b.seed)
                .map(|p| CoreVec2::new(p.x, p.y)),
            slab: Some(slab_look(s)),
            ..Spec::default()
        },
    }
}

/// The core's runs back as the contract's.
pub fn runs(rs: &[kentos_geometry_core::geom::slab::Run]) -> Vec<kentos_contracts::SlabRun> {
    rs.iter()
        .map(|r| kentos_contracts::SlabRun {
            pts: r.pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect(),
            bulges: r.bulges.clone(),
        })
        .collect()
}

/// A slab as plain objects (docs/adr/0722 §6): its rings and its edge's lines, each on its layer with its look. What
/// DXF without KentOS's data and Patlat write.
pub fn slab_pieces(s: &kentos_contracts::SlabEntity) -> Vec<kentos_contracts::Entity> {
    slab_pieces_of(s, None)
}

/// [`slab_pieces`] as a drawing at `view` shows them (docs/adr/8011): beyond 1:150 schematic its edge's outer lines
/// alone, as the walls beside it are drawn.
pub fn slab_pieces_in(
    s: &kentos_contracts::SlabEntity,
    view: kentos_geometry_core::geom::wall::View,
) -> Vec<kentos_contracts::Entity> {
    slab_pieces_of(s, Some(view))
}

fn slab_pieces_of(
    s: &kentos_contracts::SlabEntity,
    view: Option<kentos_geometry_core::geom::wall::View>,
) -> Vec<kentos_contracts::Entity> {
    let parts: Vec<Area> = s.slab.parts.iter().map(area).collect();
    let look = slab_look(&s.slab);
    let shapes = match view {
        Some(view) => kentos_geometry_core::geom::slab::pieces_in(&parts, &look, view),
        None => kentos_geometry_core::geom::slab::pieces(&parts, &look),
    };
    plain_of(&s.base, shapes)
}

/// A space as plain objects at `view` (docs/adr/8046 §1): what the drawing shows of it, its rings, a void's cross and,
/// when `label`, its label's text (number, name, area; 2.5 mm on paper, centred on the pole), each on the space's
/// layer with its look. A tagged space's tag is its label: it writes none of its own. What DXF writes.
pub fn space_pieces_in(
    s: &kentos_contracts::SpaceEntity,
    view: kentos_geometry_core::geom::wall::View,
    label: bool,
) -> Vec<kentos_contracts::Entity> {
    let parts: Vec<Area> = s.space.parts.iter().map(area).collect();
    plain_of(
        &s.base,
        kentos_geometry_core::geom::space::pieces_with_view(&parts, &spec(&s.space), view, label),
    )
}

/// Pieces as the object's plain objects: on its layer with its look, none of its own ties.
fn plain_of(
    of: &kentos_contracts::EntityBase,
    shapes: Vec<kentos_geometry_core::entity::Shape>,
) -> Vec<kentos_contracts::Entity> {
    let mut out = Vec::new();
    for piece in shapes {
        let Some(mut plain) = crate::blocks::piece_entity(&piece) else {
            continue;
        };
        let mut base = of.clone();
        base.id = 0;
        base.attachments.clear();
        base.label_pins.clear();
        base.label = None;
        *plain.base_mut() = base;
        out.push(plain);
    }
    out
}
