//! A wall between the contract and the shared core (docs/adr/0339 §1): field by field, both ways, nothing lost (the
//! core carries its style, height and architectural information untouched); and its drawing as the contract's plain
//! objects, which the DXF writer, Patlat and the other formats take where a wall has no kind of its own.

use kentos_contracts::{
    ArchInfo, ComponentEdge, ComponentHatch, Entity, EntityBase, HatchEntity, HatchPattern,
    HatchPatternType, LineEntity, LineType, PathEntity, Vec2, WallCaps, WallComponent, WallDetail,
    WallFields, WallJoin, WallJoins, WallJustify,
};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::wall::{
    self, Component, ComponentFill, Detail, JoinMode, Justify, Spec, View,
};

/// The contract's join in the core's terms.
pub fn join_mode(j: WallJoin) -> JoinMode {
    match j {
        WallJoin::Auto => JoinMode::Auto,
        WallJoin::None => JoinMode::None,
        WallJoin::Miter => JoinMode::Miter,
        WallJoin::Butt => JoinMode::Butt,
        WallJoin::Through => JoinMode::Through,
    }
}

pub fn join_back(m: JoinMode) -> WallJoin {
    match m {
        JoinMode::Auto => WallJoin::Auto,
        JoinMode::None => WallJoin::None,
        JoinMode::Miter => WallJoin::Miter,
        JoinMode::Butt => WallJoin::Butt,
        JoinMode::Through => WallJoin::Through,
    }
}

fn justify(j: WallJustify) -> Justify {
    match j {
        WallJustify::Left => Justify::Left,
        WallJustify::Center => Justify::Center,
        WallJustify::Right => Justify::Right,
    }
}

fn justify_back(j: Justify) -> WallJustify {
    match j {
        Justify::Left => WallJustify::Left,
        Justify::Center => WallJustify::Center,
        Justify::Right => WallJustify::Right,
    }
}

/// The contract's detail in the core's terms.
pub fn detail(d: WallDetail) -> Detail {
    match d {
        WallDetail::Schematic => Detail::Schematic,
        WallDetail::Standard => Detail::Standard,
        WallDetail::Detailed => Detail::Detailed,
    }
}

fn detail_back(d: Detail) -> WallDetail {
    match d {
        Detail::Schematic => WallDetail::Schematic,
        Detail::Standard => WallDetail::Standard,
        Detail::Detailed => WallDetail::Detailed,
    }
}

fn component(c: &WallComponent) -> Component {
    Component {
        name: Some(c.name.clone()),
        material: c.material.clone(),
        thickness: c.thickness,
        edge: c.edge.as_ref().map(|e| wall::ComponentEdge {
            color: e.color.clone(),
            line_weight: e.line_weight,
            line_type: e.line_type.map(|t| t.name().to_owned()),
        }),
        hatch: c.hatch.as_ref().map(|h| ComponentFill {
            pattern: h.pattern.clone(),
            scale: h.scale,
            angle: h.angle,
            color: h.color.clone(),
        }),
        detail: detail(c.detail),
        priority: c.priority,
    }
}

fn component_back(c: &Component) -> WallComponent {
    WallComponent {
        name: c.name.clone().unwrap_or_default(),
        material: c.material.clone(),
        thickness: c.thickness,
        edge: c.edge.as_ref().map(|e| ComponentEdge {
            color: e.color.clone(),
            line_weight: e.line_weight,
            line_type: e.line_type.as_deref().and_then(LineType::from_name),
        }),
        hatch: c.hatch.as_ref().map(|h| ComponentHatch {
            pattern: h.pattern.clone(),
            scale: h.scale,
            angle: h.angle,
            color: h.color.clone(),
        }),
        detail: detail_back(c.detail),
        priority: c.priority,
    }
}

/// The wall's fields beside its path as the core takes them.
pub fn spec(w: &WallFields) -> Spec {
    Spec {
        closed: w.closed,
        width: w.width,
        justify: justify(w.justify),
        offset: w.offset,
        components: w.components.iter().map(component).collect(),
        caps_open: w.caps == WallCaps::Open,
        fill: w.fill.clone(),
        style: w.style.clone(),
        height: w.height,
        arch: (!w.arch.is_empty())
            .then(|| serde_json::to_string(&w.arch).ok())
            .flatten()
            .and_then(|t| Json::parse(&t).ok()),
        joins: [join_mode(w.joins.start), join_mode(w.joins.end)],
        cleanup: w.cleanup,
    }
}

/// The wall as the core's shape.
pub fn core_wall(w: &WallFields) -> Shape {
    Shape::Wall {
        pts: w.pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect(),
        bulges: w.bulges.clone(),
        wall: spec(w),
    }
}

/// The core's wall back as the contract's fields; none for another shape.
pub fn contract_wall(shape: &Shape) -> Option<WallFields> {
    let Shape::Wall { pts, bulges, wall } = shape else {
        return None;
    };
    let arch: ArchInfo = wall
        .arch
        .as_ref()
        .and_then(|a| serde_json::from_str(&kentos_geometry_core::api::json::to_string(a)).ok())
        .unwrap_or_default();
    let n = pts.len();
    Some(WallFields {
        pts: pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect(),
        // The document keeps one per vertex, an open wall's last 0, as a polyline's.
        bulges: bulges.clone().map(|mut b| {
            b.resize(n, 0.0);
            if !wall.closed
                && let Some(last) = b.last_mut()
            {
                *last = 0.0;
            }
            b
        }),
        closed: wall.closed,
        width: wall.width,
        justify: justify_back(wall.justify),
        offset: wall.offset,
        style: wall.style.clone(),
        components: wall.components.iter().map(component_back).collect(),
        caps: if wall.caps_open {
            WallCaps::Open
        } else {
            WallCaps::Closed
        },
        fill: wall.fill.clone(),
        height: wall.height,
        arch,
        joins: WallJoins {
            start: join_back(wall.joins[0]),
            end: join_back(wall.joins[1]),
        },
        cleanup: wall.cleanup,
    })
}

fn back(pts: &[CoreVec2]) -> Vec<Vec2> {
    pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect()
}

/// The wall's drawing at `view` as the contract's plain objects on its layer, each with the wall's own look where the
/// piece has none: polylines for its faces and component lines, lines for its caps, hatches for its components and
/// fill. None when it cannot be drawn.
pub fn plain_pieces(base: &EntityBase, w: &WallFields, view: View) -> Option<Vec<Entity>> {
    let pieces = wall::pieces(
        &w.pts
            .iter()
            .map(|p| CoreVec2::new(p.x, p.y))
            .collect::<Vec<_>>(),
        w.bulges.as_deref(),
        &spec(w),
        view,
    )
    .ok()?;
    Some(plain_of(base, pieces))
}

/// Every wall of `entities` as `plain_pieces` gives it, but joined with the others (docs/adr/0340 §5) and cut by its
/// openings (docs/adr/0341 §2; the walls' persistent ids `uid_of` their object ids), and every opening as its symbol:
/// one entry per entity, none for one that is neither or cannot be drawn.
pub fn plain_joined(
    entities: &[Entity],
    view: View,
    uid_of: impl Fn(u32) -> Option<String>,
) -> Vec<Option<Vec<Entity>>> {
    use kentos_geometry_core::geom::wall_join::{WallIn, solve_with};
    let walls: Vec<(usize, Shape)> = entities
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match e {
            Entity::Wall(w) => Some((i, core_wall(&w.wall))),
            _ => None,
        })
        .collect();
    let ins: Vec<WallIn<'_>> = walls
        .iter()
        .filter_map(|(_, s)| match s {
            Shape::Wall { pts, bulges, wall } => Some(WallIn {
                pts,
                bulges: bulges.as_deref(),
                spec: wall,
            }),
            _ => None,
        })
        .collect();
    let at: Vec<usize> = walls.iter().map(|(i, _)| *i).collect();
    let spans = crate::openings::spans_by_wall(entities, &at, uid_of);
    let solved = solve_with(&ins, &spans, None);
    let mut out = vec![None; entities.len()];
    for (i, e) in entities.iter().enumerate() {
        if let Entity::Opening(o) = e {
            out[i] = Some(crate::openings::plain_pieces(&o.base, &o.opening));
        }
    }
    for ((i, shape), drawn) in walls.iter().zip(solved.drawn) {
        let (Some(d), Shape::Wall { pts, wall, .. }, Entity::Wall(w)) =
            (drawn, shape, &entities[*i])
        else {
            continue;
        };
        out[*i] = Some(plain_of(&w.base, wall::pieces_of(&d, pts, wall, view)));
    }
    out
}

/// A wall's pieces as the contract's plain objects with its look.
fn plain_of(base: &EntityBase, pieces: Vec<kentos_geometry_core::block::Piece>) -> Vec<Entity> {
    let mut out = Vec::new();
    for piece in pieces {
        let mut b = base.clone();
        b.attachments.clear();
        b.label_pins.clear();
        b.label = None;
        if piece.color.is_some() {
            b.color = piece.color.clone();
        }
        if piece.line_weight.is_some() {
            b.line_weight = piece.line_weight;
        }
        if let Some(t) = piece
            .stroke
            .line_type
            .as_deref()
            .and_then(LineType::from_name)
        {
            b.look.line_type = Some(t);
        }
        out.push(match piece.shape {
            Shape::Polyline {
                pts,
                bulges,
                closed,
                ..
            } => Entity::Polyline(PathEntity {
                space_separator: None,
                base: b,
                pts: back(&pts),
                bulges,
                holes: None,
                zs: None,
                ms: None,
                parts: None,
                closed,
                widths: None,
            }),
            Shape::Line { a, b: end } => Entity::Line(LineEntity {
                base: b,
                a: Vec2 { x: a.x, y: a.y },
                b: Vec2 { x: end.x, y: end.y },
                za: None,
                zb: None,
                ma: None,
                mb: None,
            }),
            Shape::Hatch {
                ring,
                holes,
                pattern,
                ..
            } => Entity::Hatch(HatchEntity {
                base: b,
                ring: back(&ring),
                holes: holes.map(|hs| hs.iter().map(|h| back(h)).collect()),
                pattern: serde_json::from_str(&kentos_geometry_core::api::json::to_string(
                    &pattern,
                ))
                .unwrap_or_else(|_| HatchPattern::user(HatchPatternType::Solid, 0.0, 1.0)),
                assoc: None,
                boundary: None,
                annotative: None,
            }),
            _ => continue,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wall_crosses_to_the_core_and_back_whole() {
        let mut styles = kentos_contracts::builtin_wall_styles();
        let s = styles.remove(0);
        let w = WallFields {
            pts: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 5.0, y: 0.0 },
                Vec2 { x: 5.0, y: 4.0 },
            ],
            bulges: Some(vec![0.0, 0.25, 0.0]),
            closed: false,
            width: 0.3,
            justify: WallJustify::Left,
            offset: 0.02,
            style: Some(s.id.clone()),
            components: s.components.clone(),
            caps: WallCaps::Open,
            fill: s.fill.clone(),
            height: Some(2.8),
            arch: ArchInfo {
                level: Some("zemin".into()),
                phase: Some(kentos_contracts::ArchPhase::New),
                status: None,
                discipline: Some(kentos_contracts::ArchDiscipline::Architecture),
            },
            joins: WallJoins {
                start: WallJoin::Butt,
                end: WallJoin::Auto,
            },
            cleanup: false,
        };
        assert_eq!(contract_wall(&core_wall(&w)), Some(w));
    }
}
