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
    }
}

/// The slab as the core's shape (docs/adr/0722): its rings as a space's, drawn by its look.
pub fn core_slab(s: &kentos_contracts::SlabFields) -> Shape {
    Shape::Space {
        parts: s.parts.iter().map(area).collect(),
        space: Spec {
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
    let parts: Vec<Area> = s.slab.parts.iter().map(area).collect();
    let mut out = Vec::new();
    for piece in kentos_geometry_core::geom::slab::pieces(&parts, &slab_look(&s.slab)) {
        let Some(mut plain) = crate::blocks::piece_entity(&piece) else {
            continue;
        };
        let mut base = s.base.clone();
        base.id = 0;
        base.attachments.clear();
        base.label_pins.clear();
        base.label = None;
        *plain.base_mut() = base;
        out.push(plain);
    }
    out
}
