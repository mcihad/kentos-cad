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
    }
}

/// The space as the core's shape.
pub fn core_space(s: &SpaceFields) -> Shape {
    Shape::Space {
        parts: s.parts.iter().map(area).collect(),
        space: spec(s),
    }
}
