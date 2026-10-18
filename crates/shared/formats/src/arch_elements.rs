//! Grids, columns, stairs and ramps between the contract and the shared core (docs/adr/0343 §2): their fields as the
//! core's shapes, which draw them.

use kentos_contracts::{ColumnFields, ColumnShape, Entity, GridBubbles, GridFields, RampFields, StairFields, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arch::{Bubbles, ColumnSpec, GridSpec, RampSpec, StairSpec};

fn core(pts: &[Vec2]) -> Vec<CoreVec2> {
    pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect()
}

/// The grid as the core's shape.
pub fn core_grid(g: &GridFields) -> Shape {
    Shape::Grid {
        pts: core(&g.pts),
        bulges: g.bulges.clone(),
        grid: GridSpec {
            label: g.label.clone(),
            bubbles: match g.bubbles {
                GridBubbles::Both => Bubbles::Both,
                GridBubbles::Start => Bubbles::Start,
                GridBubbles::End => Bubbles::End,
                GridBubbles::None => Bubbles::None,
            },
            bubble_size: g.bubble_size,
        },
    }
}

/// The column as the core's shape.
pub fn core_column(c: &ColumnFields) -> Shape {
    Shape::Column {
        at: CoreVec2::new(c.at.x, c.at.y),
        column: ColumnSpec {
            round: c.shape == ColumnShape::Round,
            width: c.width,
            depth: c.depth,
            rotation: c.rotation,
            fill: c.fill,
            tag: c.tag.clone(),
            place: c.place.clone(),
        },
    }
}

/// The stair as the core's shape.
pub fn core_stair(s: &StairFields) -> Shape {
    Shape::Stair {
        pts: core(&s.pts),
        stair: StairSpec {
            width: s.width,
            risers: f64::from(s.risers),
            riser_height: s.riser_height,
            tread: s.tread,
            landing: s.landing,
            cut: s.cut.map(f64::from),
            numbering: s.numbering,
        },
    }
}

/// The ramp as the core's shape.
pub fn core_ramp(r: &RampFields) -> Shape {
    Shape::Ramp {
        pts: core(&r.pts),
        ramp: RampSpec {
            width: r.width,
            rise: r.rise,
            slope: r.slope,
            base: r.base,
            landing: r.landing,
        },
    }
}

/// One of the four as the core's shape; none for another object.
pub fn core_of(e: &Entity) -> Option<Shape> {
    Some(match e {
        Entity::Grid(g) => core_grid(&g.grid),
        Entity::Column(c) => core_column(&c.column),
        Entity::Stair(s) => core_stair(&s.stair),
        Entity::Ramp(r) => core_ramp(&r.ramp),
        _ => return None,
    })
}

/// Its outline's points (a grid's path, a column's outline, a stair's or ramp's band); none for another object.
pub fn outline_points(e: &Entity) -> Option<Vec<Vec2>> {
    let shape = core_of(e)?;
    let o = kentos_geometry_core::geom::arch::outline(&shape)?;
    Some(
        kentos_geometry_core::entity::entity_vertices(&o)
            .into_iter()
            .map(|p| Vec2 { x: p.x, y: p.y })
            .collect(),
    )
}

/// One of the four as plain objects at 1:`scale`: its pieces, each on its layer with its look (a piece's own line type
/// over it); none for another object. What DXF and Patlat write (docs/adr/0343 §5).
pub fn plain_pieces(e: &Entity, scale: f64) -> Option<Vec<Entity>> {
    let shape = core_of(e)?;
    let mut out = Vec::new();
    for piece in kentos_geometry_core::geom::arch::pieces(&shape, scale) {
        let Some(mut plain) = crate::blocks::piece_entity(&piece.shape) else {
            continue;
        };
        let mut base = e.base().clone();
        base.id = 0;
        base.attachments.clear();
        base.label_pins.clear();
        base.label = None;
        if let Some(t) = piece
            .stroke
            .line_type
            .as_deref()
            .and_then(kentos_contracts::LineType::from_name)
        {
            base.look.line_type = Some(t);
        }
        *plain.base_mut() = base;
        out.push(plain);
    }
    Some(out)
}

/// One of the four as one simple object, for formats without it (GeoJSON, KML, GPX, the server's projection): a
/// grid's path an open polyline, a round column a circle, a rectangular column, a stair or a ramp an area; none for
/// another object.
pub fn simple(e: &Entity) -> Option<Entity> {
    let path = |base: &kentos_contracts::EntityBase, pts: Vec<Vec2>, bulges: Option<Vec<f64>>| kentos_contracts::PathEntity {
        base: base.clone(),
        pts,
        bulges,
        holes: None,
        zs: None,
        ms: None,
        parts: None,
        closed: false,
        widths: None,
    };
    Some(match e {
        Entity::Grid(g) => Entity::Polyline(path(&g.base, g.grid.pts.clone(), g.grid.bulges.clone())),
        Entity::Column(c) if c.column.shape == ColumnShape::Round => {
            Entity::Circle(kentos_contracts::CircleEntity {
                base: c.base.clone(),
                c: c.column.at,
                r: c.column.width / 2.0,
            })
        }
        Entity::Column(_) | Entity::Stair(_) | Entity::Ramp(_) => {
            Entity::Polygon(path(e.base(), outline_points(e)?, None))
        }
        _ => return None,
    })
}
