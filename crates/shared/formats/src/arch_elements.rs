//! Grids, columns, stairs and ramps between the contract and the shared core (docs/adr/0343 §2): their fields as the
//! core's shapes, which draw them.

use kentos_contracts::{
    BeamFields, ColumnFields, ColumnShape, Entity, GridBubbles, GridFields, RampFields, RoofFields, StairFields,
    TagFields, TagShape, Vec2, WallJustify,
};
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
            ends: g.ends,
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
            legs: c.legs,
            rotation: c.rotation,
            anchor: c.anchor.unit(),
            fill: c.fill,
            tag: c.tag.clone(),
            place: c.place.clone(),
            material: c.material.clone(),
            level: c.arch.level.clone(),
        },
    }
}

/// The beam as the core's shape (docs/adr/8041 §1): its middle off its axis as its justification says, an axis that is
/// its left face putting its body to the right.
pub fn core_beam(b: &BeamFields) -> Shape {
    let middle = match b.justify {
        WallJustify::Left => -b.width / 2.0,
        WallJustify::Center => 0.0,
        WallJustify::Right => b.width / 2.0,
    };
    Shape::Beam {
        pts: core(&b.pts),
        beam: kentos_geometry_core::geom::beam::BeamSpec {
            width: b.width,
            middle,
            height: b.height,
            tag: b.tag.clone(),
        },
    }
}

/// The roof as the core's shape (docs/adr/8044 §1): its eaves line and slopes; its eaves at its offset (a block or a file
/// without its storeys; the document stands it on its storey).
/// A roof's part as the core takes it (docs/adr/8044 §6): its rise its kind's when none is given.
pub fn core_roof_part(p: &kentos_contracts::RoofPart) -> kentos_geometry_core::geom::roof::Part {
    use kentos_contracts::RoofPartKind;
    use kentos_geometry_core::geom::roof::PartKind;
    kentos_geometry_core::geom::roof::Part {
        kind: match p.kind {
            RoofPartKind::Chimney => PartKind::Chimney,
            RoofPartKind::Vent => PartKind::Vent,
            RoofPartKind::Window => PartKind::Window,
            RoofPartKind::Downpipe => PartKind::Downpipe,
        },
        at: kentos_geometry_core::Vec2::new(p.at.x, p.at.y),
        width: p.width,
        depth: p.depth,
        rotation: p.rotation,
        rise: p.rise_m().unwrap_or(0.0),
    }
}

pub fn core_roof(r: &RoofFields) -> Shape {
    Shape::Roof {
        pts: core(&r.pts),
        roof: kentos_geometry_core::geom::roof::RoofSpec {
            slopes: r.slopes.clone(),
            base: r.offset,
            overhead: false,
            parts: r.parts.iter().map(core_roof_part).collect(),
            gutter: r.gutter,
            snow_guard: r.snow_guard,
        },
    }
}

/// The stair as the core's shape.
pub fn core_stair(s: &StairFields) -> Shape {
    Shape::Stair {
        pts: core(&s.pts),
        stair: StairSpec {
            bulge: s.bulge(),
            width: s.width,
            risers: f64::from(s.risers),
            riser_height: s.riser_height,
            tread: s.tread,
            landing: s.landing,
            cut: s.cut.map(f64::from),
            numbering: s.numbering,
            railings: s.railings.map(railings),
            post: s.post,
            upper: false,
            winders: s.winders.map(f64::from),
        },
    }
}

/// A stair's or ramp's railings as the core's (docs/adr/0705 §2).
fn railings(r: kentos_contracts::Railings) -> kentos_geometry_core::geom::arch::Railings {
    use kentos_geometry_core::geom::arch::Railings as Core;
    match r {
        kentos_contracts::Railings::Left => Core::Left,
        kentos_contracts::Railings::Right => Core::Right,
        kentos_contracts::Railings::Both => Core::Both,
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
            railings: r.railings.map(railings),
        },
    }
}

/// The tag as the core's shape (docs/adr/0344 §1).
pub fn core_tag(t: &TagFields) -> Shape {
    use kentos_geometry_core::geom::tag::{Frame, Spec};
    Shape::Tag {
        at: CoreVec2::new(t.at.x, t.at.y),
        tag: Spec {
            text: t.text.clone(),
            frame: match t.shape {
                TagShape::Circle => Frame::Circle,
                TagShape::Hexagon => Frame::Hexagon,
                TagShape::Rectangle => Frame::Rectangle,
                TagShape::Rounded => Frame::Rounded,
                TagShape::None => Frame::None,
                TagShape::Ellipse => Frame::Ellipse,
            },
            size: t.size,
            leader: t.leader,
            anchor: t.anchor.map(|a| CoreVec2::new(a.x, a.y)),
        },
    }
}

/// The view mark as the core's shape (docs/adr/0345 §1).
pub fn core_viewmark(m: &kentos_contracts::ViewMarkFields) -> Shape {
    use kentos_contracts::ViewMarkForm;
    use kentos_geometry_core::geom::viewmark::{Form, Spec};
    Shape::ViewMark {
        points: m.points.iter().map(|p| CoreVec2::new(p.x, p.y)).collect(),
        mark: Spec {
            form: match m.form {
                ViewMarkForm::Section => Form::Section,
                ViewMarkForm::Elevation => Form::Elevation,
                ViewMarkForm::Detail => Form::Detail,
            },
            radius: m.radius,
            at: m.at.map(|a| CoreVec2::new(a.x, a.y)),
            angle: m.angle,
            flip: m.flip,
            size: m.size,
            top: m.words.top.clone(),
            bottom: m.words.bottom.clone(),
        },
    }
}

/// One of the four, or a tag, as the core's shape; none for another object.
pub fn core_of(e: &Entity) -> Option<Shape> {
    Some(match e {
        Entity::Grid(g) => core_grid(&g.grid),
        Entity::Column(c) => core_column(&c.column),
        Entity::Stair(s) => core_stair(&s.stair),
        Entity::Ramp(r) => core_ramp(&r.ramp),
        Entity::Beam(b) => core_beam(&b.beam),
        Entity::Roof(r) => core_roof(&r.roof),
        Entity::Tag(t) => core_tag(&t.tag),
        Entity::ViewMark(m) => core_viewmark(&m.mark),
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
    plain_of(e, kentos_geometry_core::geom::arch::pieces(&shape, scale))
}

/// [`plain_pieces`] as a drawing at `view` shows them (docs/adr/8011): what DXF writes, at the walls' detail and the
/// file's scale; Patlat keeps the whole.
pub fn plain_pieces_in(
    e: &Entity,
    view: kentos_geometry_core::geom::wall::View,
) -> Option<Vec<Entity>> {
    let shape = core_of(e)?;
    plain_of(e, kentos_geometry_core::geom::arch::pieces_in(&shape, view))
}

/// Pieces of `e` as plain objects, each on its layer with its look (a piece's own line type over it).
pub fn plain_of_pieces(e: &Entity, pieces: Vec<kentos_geometry_core::block::Piece>) -> Option<Vec<Entity>> {
    plain_of(e, pieces)
}

fn plain_of(e: &Entity, pieces: Vec<kentos_geometry_core::block::Piece>) -> Option<Vec<Entity>> {
    let mut out = Vec::new();
    for piece in pieces {
        let Some(mut plain) = crate::blocks::piece_entity(&piece.shape) else {
            continue;
        };
        let mut base = e.base().clone();
        base.id = 0;
        base.attachments.clear();
        base.label_pins.clear();
        base.label = None;
        // A piece's own colour and weight: a column one section with a wall's layer takes its fill's (docs/adr/8041 §2).
        if piece.color.is_some() {
            base.color = piece.color.clone();
        }
        if piece.line_weight.is_some() {
            base.line_weight = piece.line_weight;
        }
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
/// grid's path an open polyline, a round column a circle, a rectangular column, a stair, a ramp or a beam's band
/// (docs/adr/8041 §1) an area; none for another object.
pub fn simple(e: &Entity) -> Option<Entity> {
    let path = |base: &kentos_contracts::EntityBase, pts: Vec<Vec2>, bulges: Option<Vec<f64>>| kentos_contracts::PathEntity {
        space_separator: None,
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
        // Round its box's middle, whatever point its place is (docs/adr/0725 §2).
        Entity::Column(c) if c.column.shape == ColumnShape::Round => {
            let Shape::Column { at, column } = core_column(&c.column) else {
                return None;
            };
            let centre = kentos_geometry_core::geom::arch::column_center(at, &column);
            Entity::Circle(kentos_contracts::CircleEntity {
                base: c.base.clone(),
                c: Vec2 { x: centre.x, y: centre.y },
                r: c.column.width / 2.0,
            })
        }
        Entity::Column(_) | Entity::Stair(_) | Entity::Ramp(_) | Entity::Beam(_) | Entity::Roof(_) => {
            Entity::Polygon(path(e.base(), outline_points(e)?, None))
        }
        // A view mark a point at its first point, its words its label (docs/adr/0345 §1).
        Entity::ViewMark(m) => {
            let mut base = m.base.clone();
            if base.label.is_none() {
                base.label = Some(format!("{}/{}", m.mark.words.top, m.mark.words.bottom));
            }
            Entity::Point(kentos_contracts::PointEntity {
                base,
                p: m.mark.points.first().copied().unwrap_or(kentos_contracts::Vec2 { x: 0.0, y: 0.0 }),
                z: None,
                parts: None,
            })
        }
        // A tag a point at its middle, its words its label (docs/adr/0344 §1).
        Entity::Tag(t) => {
            let mut base = t.base.clone();
            if base.label.is_none() && !t.tag.text.is_empty() {
                base.label = Some(t.tag.text.replace('\n', " "));
            }
            Entity::Point(kentos_contracts::PointEntity {
                base,
                p: t.tag.at,
                z: None,
                parts: None,
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_geometry_core::geom::wall::{Detail, View};

    fn entity(json: serde_json::Value) -> Entity {
        serde_json::from_value(json).expect("an object")
    }

    fn words(pieces: &[Entity]) -> Vec<String> {
        pieces
            .iter()
            .filter_map(|e| match e {
                Entity::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect()
    }

    /// DXF writes a column and a stair as the file's scale draws them (docs/adr/8011): at 1:200 the column without its
    /// name and the stair without railings; at 1:50 the column's name and size and the stair's risers in words. Patlat
    /// keeps the whole.
    #[test]
    fn dxf_writes_a_column_and_a_stair_at_the_files_scale() {
        let simple = View::new(Detail::Schematic, 200.0);
        let standard = View::new(Detail::Standard, 50.0);
        let column = entity(
            serde_json::json!({ "kind": "column", "id": 1, "layerId": "kolon", "attrs": {},
            "at": { "x": 3.0, "y": 3.0 }, "width": 0.3, "depth": 0.5, "tag": "S1" }),
        );
        assert!(words(&plain_pieces_in(&column, simple).unwrap()).is_empty());
        assert_eq!(
            words(&plain_pieces_in(&column, standard).unwrap()),
            ["S1\n30/50"]
        );
        assert_eq!(words(&plain_pieces(&column, 200.0).unwrap()), ["S1"]);
        let stair = entity(
            serde_json::json!({ "kind": "stair", "id": 2, "layerId": "merdiven", "attrs": {},
            "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 2.8, "y": 0.0 }], "width": 1.2, "risers": 11,
            "riserHeight": 0.175, "tread": 0.28, "railings": "both" }),
        );
        let whole = plain_pieces(&stair, 100.0).unwrap().len();
        assert_eq!(
            plain_pieces_in(&stair, simple).unwrap().len(),
            whole - 2,
            "no railings at 1:200"
        );
        let at_50 = plain_pieces_in(&stair, standard).unwrap();
        assert_eq!(at_50.len(), whole + 1);
        assert_eq!(words(&at_50), ["11 × 17.5/28"]);
    }

    /// A beam (docs/adr/8041 §1): DXF writes its band dashed, its words by the file's scale (none at 1:200, its number
    /// and size at 1:50); the formats without it (GeoJSON, KML, the server's projection) take its band as an area.
    #[test]
    fn dxf_writes_a_beam_dashed_and_the_simple_formats_its_band() {
        let simple_view = View::new(Detail::Schematic, 200.0);
        let standard = View::new(Detail::Standard, 50.0);
        let beam = entity(
            serde_json::json!({ "kind": "beam", "id": 3, "layerId": "kiris", "attrs": {},
            "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 5.0, "y": 0.0 }], "width": 0.25, "height": 0.5,
            "justify": "left", "tag": "K101" }),
        );
        let plan = plain_pieces_in(&beam, simple_view).unwrap();
        assert_eq!(plan.len(), 1, "{plan:?}");
        assert!(words(&plan).is_empty());
        assert_eq!(
            plan[0].base().look.line_type,
            kentos_contracts::LineType::from_name("dashed")
        );
        assert_eq!(plan[0].base().layer_id, "kiris");
        assert_eq!(
            words(&plain_pieces_in(&beam, standard).unwrap()),
            ["K101 25/50"]
        );
        // Its band, its left face on its axis: 0 to −0.25.
        let Some(Entity::Polygon(band)) = simple(&beam) else {
            panic!("an area")
        };
        let ys: Vec<f64> = band.pts.iter().map(|p| p.y).collect();
        assert_eq!(band.pts.len(), 4, "{:?}", band.pts);
        assert!(ys.iter().all(|y| *y == 0.0 || (*y + 0.25).abs() < 1e-12), "{ys:?}");
    }
}
