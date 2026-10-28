//! Façade and space dimension recipes' inputs (docs/adr/0729 §2): which objects a recipe reads (its walls, the walls
//! and grids around them on their level, the openings of its walls or on its space) and the core's plan of them.
//! One rule for the desktop's document and the server's records: each gives the objects it has in the recipe's box.

use kentos_contracts::{
    DimensionSet, DimensionSetBreak, DimensionSetReference, DimensionSetRow, DimensionSetSegment,
    Entity, EntityId, OpeningForm, OpeningSide, SubPlace, SubRef, Vec2,
};
use kentos_geometry_core::ops::dimension_set as core;
use kentos_geometry_core::ops::dimension_span as span;
use kentos_geometry_core::ops::subref::Place;
use kentos_geometry_core::vec2::Vec2 as CoreVec2;

/// How far around its sources a recipe reads walls, openings and grids (metres): a wall meeting a source is in it.
pub const REACH: f64 = 1.0;

/// A box: min x, min y, max x, max y.
pub type Box4 = [f64; 4];

/// Why a recipe cannot be planned: the break it gets, and the source and why when one is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Refusal {
    pub broken: DimensionSetBreak,
    pub source: Option<EntityId>,
    pub why: Option<span::Why>,
}

impl Refusal {
    fn of(broken: DimensionSetBreak) -> Self {
        Self {
            broken,
            source: None,
            why: None,
        }
    }
}

/// A row's account (docs/adr/0729 §5): its points, the objects they come from, the spans it draws.
#[derive(Clone, Debug, PartialEq)]
pub struct RowNote {
    pub row: DimensionSetRow,
    pub points: usize,
    pub sources: usize,
    pub spans: usize,
}

/// A recipe's spans with the identities of the source numbers their points name, and its account.
#[derive(Clone, Debug, PartialEq)]
pub struct Planned {
    pub spans: Vec<core::Span>,
    pub sources: Vec<EntityId>,
    pub rows: Vec<RowNote>,
    /// The walls near a façade left unmeasured (an oblique or curved edge where they meet it).
    pub skipped: Vec<EntityId>,
    /// The side the rows go to: +1 to the direction's left, −1 to its right.
    pub side: f64,
    /// The rows' frame: its origin, its direction and the bodies' reach across it (docs/adr/0729 §6).
    pub origin: CoreVec2,
    pub direction: CoreVec2,
    pub body: (f64, f64),
}

/// An object near a recipe, as its host has it: its identity, itself, whether its layer is shown.
#[derive(Clone, Copy, Debug)]
pub struct Near<'a> {
    pub id: EntityId,
    pub entity: &'a Entity,
    pub shown: bool,
}

fn p(v: Vec2) -> CoreVec2 {
    CoreVec2::new(v.x, v.y)
}

fn grow(b: Box4, r: f64) -> Box4 {
    [b[0] - r, b[1] - r, b[2] + r, b[3] + r]
}

/// Whether two boxes touch.
pub fn meets(a: &Box4, b: &Box4) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

fn around(pts: impl IntoIterator<Item = Vec2>) -> Option<Box4> {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    let mut any = false;
    for q in pts {
        any = true;
        b = [b[0].min(q.x), b[1].min(q.y), b[2].max(q.x), b[3].max(q.y)];
    }
    any.then_some(b)
}

/// The box of an object a recipe reads: a wall's body's, an opening's jambs', a grid's ends', a space's outline's, a
/// column's; none for anything else.
pub fn reach_box(e: &Entity) -> Option<Box4> {
    match e {
        Entity::Wall(w) => around(w.wall.pts.iter().copied())
            .map(|b| grow(b, w.wall.width + w.wall.offset.abs())),
        Entity::Opening(o) => around(o.opening.place.iter().copied()),
        Entity::Grid(g) => around(g.grid.pts.iter().copied()),
        Entity::Space(s) => around(
            s.space
                .parts
                .iter()
                .flat_map(|part| part.outer.pts.iter().copied()),
        ),
        Entity::Column(c) => {
            let r = c.column.width.max(c.column.depth.unwrap_or(c.column.width)) * 1.5;
            around([c.column.at]).map(|b| grow(b, r))
        }
        _ => None,
    }
}

/// The box a recipe reads in: its sources' bodies (as `find` has them), grown by [`REACH`]; none when no source is
/// left.
pub fn reach<'a>(set: &DimensionSet, find: &dyn Fn(EntityId) -> Option<&'a Entity>) -> Option<Box4> {
    let mut b: Option<Box4> = None;
    for id in set.sources() {
        let Some(r) = find(id).and_then(reach_box) else {
            continue;
        };
        b = Some(match b {
            Some(b) => [b[0].min(r[0]), b[1].min(r[1]), b[2].max(r[2]), b[3].max(r[3])],
            None => r,
        });
    }
    b.map(|b| grow(b, REACH))
}

/// The core's row of the contract's.
pub fn core_row(r: DimensionSetRow) -> core::Row {
    match r {
        DimensionSetRow::Openings => core::Row::Openings,
        DimensionSetRow::Overall => core::Row::Overall,
        DimensionSetRow::Walls => core::Row::Walls,
        DimensionSetRow::Grids => core::Row::Grids,
    }
}

/// The contract's row of the core's.
pub fn contract_row(r: core::Row) -> DimensionSetRow {
    match r {
        core::Row::Openings => DimensionSetRow::Openings,
        core::Row::Overall => DimensionSetRow::Overall,
        core::Row::Walls => DimensionSetRow::Walls,
        core::Row::Grids => DimensionSetRow::Grids,
    }
}

/// The contract's place of the core's.
pub fn contract_place(p: Place) -> SubPlace {
    let n = |x: usize| u32::try_from(x).unwrap_or(u32::MAX);
    match p {
        Place::Node { part } => SubPlace::Node { part: n(part) },
        Place::Start => SubPlace::Start,
        Place::End => SubPlace::End,
        Place::Center => SubPlace::Center,
        Place::Vertex { part, ring, index } => SubPlace::Vertex {
            part: n(part),
            ring: n(ring),
            index: n(index),
        },
        Place::Mid { part, ring, index } => SubPlace::Mid {
            part: n(part),
            ring: n(ring),
            index: n(index),
        },
        Place::Along {
            part,
            ring,
            index,
            t,
        } => SubPlace::Along {
            part: n(part),
            ring: n(ring),
            index: n(index),
            t,
        },
    }
}

fn level_of(e: &Entity) -> Option<&String> {
    match e {
        Entity::Wall(w) => w.wall.arch.level.as_ref(),
        Entity::Grid(g) => g.grid.arch.level.as_ref(),
        Entity::Space(s) => s.space.arch.level.as_ref(),
        _ => None,
    }
}

/// A wall as the core reads it.
struct CoreWall {
    pts: Vec<CoreVec2>,
    bulges: Option<Vec<f64>>,
    spec: kentos_geometry_core::geom::wall::Spec,
}

fn core_wall(e: &Entity) -> Option<CoreWall> {
    let Entity::Wall(w) = e else { return None };
    Some(CoreWall {
        pts: w.wall.pts.iter().copied().map(p).collect(),
        bulges: w.wall.bulges.clone(),
        spec: crate::walls::spec(&w.wall),
    })
}

/// A façade or space recipe's plan (docs/adr/0729 §2): its sources as `find` has them, the walls, openings and grids
/// `near` gives (the objects in its box: [`reach`]) on its level, the core's plan; the user's removed spans are not
/// drawn. `plot_scale` turns the rows' paper spacing into the world's.
pub fn plan<'a>(
    set: &DimensionSet,
    find: &dyn Fn(EntityId) -> Option<&'a Entity>,
    near: &[Near<'_>],
    plot_scale: f64,
) -> Result<Planned, Refusal> {
    let Some(rule) = &set.span else {
        return Err(Refusal::of(DimensionSetBreak::UnsupportedWall));
    };
    let mut sources: Vec<EntityId> = Vec::new();
    let mut walls: Vec<(CoreWall, bool)> = Vec::new();
    let mut space_in: Option<span::SpaceIn> = None;
    let mut level: Option<Option<&String>> = None;
    if rule.space {
        let Some(e) = find(set.wall) else {
            return Err(Refusal::of(DimensionSetBreak::HostGone));
        };
        let Entity::Space(s) = e else {
            return Err(Refusal {
                broken: DimensionSetBreak::UnsupportedWall,
                source: Some(set.wall),
                why: None,
            });
        };
        level = Some(level_of(e));
        let parts = s
            .space
            .parts
            .iter()
            .map(|part| {
                std::iter::once(&part.outer)
                    .chain(part.holes.iter())
                    .map(|ring| {
                        let n = ring.pts.len();
                        (
                            ring.pts.iter().copied().map(p).collect(),
                            ring.bulges.clone().unwrap_or_else(|| vec![0.0; n]),
                        )
                    })
                    .collect()
            })
            .collect();
        space_in = Some(span::SpaceIn { source: 0, parts });
    } else {
        for id in set.sources() {
            let Some(e) = find(id) else { continue };
            let Some(w) = core_wall(e) else {
                return Err(Refusal {
                    broken: DimensionSetBreak::UnsupportedWall,
                    source: Some(id),
                    why: None,
                });
            };
            let here = level_of(e);
            match level {
                None => level = Some(here),
                Some(l) if l != here => {
                    return Err(Refusal {
                        broken: DimensionSetBreak::MixedSources,
                        source: Some(id),
                        why: None,
                    });
                }
                Some(_) => {}
            }
            sources.push(id);
            walls.push((w, true));
        }
        if walls.is_empty() {
            return Err(Refusal::of(DimensionSetBreak::HostGone));
        }
    }
    let level = level.flatten();
    let reach = reach(set, find).ok_or(Refusal::of(DimensionSetBreak::HostGone))?;
    // The walls around (a façade's), the openings (a façade's on its walls; a space's in its box) and the grids, in
    // the host's order of identities, so the plan is the same whatever order it has them in.
    let mut near: Vec<&Near<'_>> = near.iter().collect();
    near.sort_by_key(|n| n.id);
    let mut hosts: Vec<EntityId> = sources.clone();
    let mut openings: Vec<(EntityId, &kentos_contracts::OpeningFields)> = Vec::new();
    let mut grids: Vec<(EntityId, Vec<CoreVec2>, Option<Vec<f64>>)> = Vec::new();
    for n in near {
        let e = n.entity;
        match e {
            Entity::Wall(_) if !rule.space && !sources.contains(&n.id) => {
                if !n.shown
                    || level_of(e) != level
                    || !reach_box(e).is_some_and(|b| meets(&b, &reach))
                {
                    continue;
                }
                if let Some(w) = core_wall(e) {
                    hosts.push(n.id);
                    walls.push((w, false));
                }
            }
            Entity::Opening(o) => {
                let on_source = EntityId::parse(&o.opening.host).is_some_and(|h| sources.contains(&h));
                if rule.space {
                    if reach_box(e).is_some_and(|b| meets(&b, &reach)) {
                        openings.push((n.id, &o.opening));
                    }
                } else if on_source {
                    // A misplaced opening on a source breaks the recipe rather than leave it out (ADR 0695).
                    if o.opening.broken.is_some() || o.opening.place.len() != 4 {
                        return Err(Refusal::of(DimensionSetBreak::OpeningUnplaced));
                    }
                    openings.push((n.id, &o.opening));
                }
            }
            Entity::Grid(g) => {
                let on = g.grid.arch.level.is_none() || g.grid.arch.level.as_ref() == level;
                if n.shown && on && g.grid.pts.len() == 2 {
                    grids.push((n.id, g.grid.pts.iter().copied().map(p).collect(), g.grid.bulges.clone()));
                }
            }
            _ => {}
        }
    }
    // Source numbers: the walls, then the openings, the grids, the space last.
    let mut ids: Vec<EntityId> = hosts.clone();
    let opening_ins: Vec<span::OpeningIn> = openings
        .iter()
        .filter(|(_, o)| o.place.len() == 4)
        .map(|(id, o)| {
            let source = ids.len();
            ids.push(*id);
            let host = EntityId::parse(&o.host).and_then(|h| hosts.iter().position(|x| *x == h));
            span::OpeningIn {
                source,
                host,
                jambs: [p(o.place[0]), p(o.place[1]), p(o.place[2]), p(o.place[3])],
                flue: o.form == OpeningForm::Flue,
                niche_right: (o.form == OpeningForm::Niche).then_some(o.side == OpeningSide::Right),
            }
        })
        .collect();
    let grid_sources: Vec<usize> = grids
        .iter()
        .map(|(id, ..)| {
            ids.push(*id);
            ids.len() - 1
        })
        .collect();
    let grid_ins: Vec<span::GridIn<'_>> = grids
        .iter()
        .zip(&grid_sources)
        .map(|((_, pts, bulges), &source)| span::GridIn {
            source,
            pts,
            bulges: bulges.as_deref(),
        })
        .collect();
    if let Some(s) = &mut space_in {
        s.source = ids.len();
        ids.push(set.wall);
    }
    let wall_ins: Vec<span::WallIn<'_>> = walls
        .iter()
        .enumerate()
        .map(|(source, (w, chosen))| span::WallIn {
            source,
            pts: &w.pts,
            bulges: w.bulges.as_deref(),
            spec: &w.spec,
            chosen: *chosen,
        })
        .collect();
    let rows: Vec<core::Row> = set.rows.iter().copied().map(core_row).collect();
    let places: Vec<(core::Row, CoreVec2)> = set
        .row_places
        .iter()
        .map(|r| (core_row(r.row), p(r.at)))
        .collect();
    let suppressed: Vec<String> = set
        .suppressed
        .iter()
        .filter_map(|s| serde_json::to_string(s).ok())
        .collect();
    let reference = |q: &core::Point| SubRef {
        uid: ids[q.source],
        place: contract_place(q.place),
        path: Vec::new(),
        broken: None,
    };
    let drop = |row: core::Row, a: &core::Point, b: &core::Point| {
        let segment = DimensionSetSegment {
            row: contract_row(row),
            a: reference(a),
            b: reference(b),
        };
        serde_json::to_string(&segment).is_ok_and(|key| suppressed.contains(&key))
    };
    let recipe = span::Recipe {
        direction: p(rule.direction),
        reference: match rule.reference {
            DimensionSetReference::Faces => span::Reference::Faces,
            DimensionSetReference::Core => span::Reference::Core,
        },
        openings: match set.openings {
            kentos_contracts::DimensionSetOpenings::Ends => core::Openings::Ends,
            kentos_contracts::DimensionSetOpenings::Centers => core::Openings::Centers,
        },
        rows: &rows,
        at: p(set.at),
        gap: set.gap_mm * plot_scale / 1000.0,
        row_places: &places,
    };
    let planned = span::plan(&recipe, &wall_ins, &opening_ins, &grid_ins, space_in.as_ref(), &drop).map_err(
        |fault| match fault {
            span::Fault::Direction => Refusal::of(DimensionSetBreak::UnsupportedWall),
            span::Fault::Nothing => Refusal::of(DimensionSetBreak::HostGone),
            span::Fault::Unsupported(source, why) => Refusal {
                broken: DimensionSetBreak::UnsupportedWall,
                source: ids.get(source).copied(),
                why: Some(why),
            },
        },
    )?;
    Ok(Planned {
        rows: planned
            .rows
            .iter()
            .map(|r| RowNote {
                row: contract_row(r.row),
                points: r.points,
                sources: r.sources,
                spans: r.spans,
            })
            .collect(),
        skipped: planned.skipped.iter().filter_map(|s| ids.get(*s).copied()).collect(),
        side: planned.side,
        origin: planned.origin,
        direction: planned.direction,
        body: planned.body,
        spans: planned.spans,
        sources: ids,
    })
}
