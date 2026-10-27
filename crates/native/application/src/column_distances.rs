//! A column's temporary distances (docs/adr/0712): on each of its four sides, the clear distance to the nearest
//! parallel face, a wall's or another column's (`cad.columns.distances`); `cad.arch.edit`'s `distance` moves the column
//! square to a side. The core's `ops::column_shift` is the rule; this reads the drawing for it. The shared cases are
//! `fixtures/commands/v1/cad.columns.distances.json` and `cad.arch.edit.distance.json`.

use kentos_commands::{
    ColumnGap, ColumnGapTarget, ColumnGaps, ColumnsDistances, ColumnsDistancesInput, CommandResult,
    DimensionSetFace, Entity, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arch::ColumnSpec;
use kentos_geometry_core::geom::wall_join::WallIn;
use kentos_geometry_core::ops::column_shift::{self, ColumnIn, Target};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The drawing's walls in their order, and its columns, `first` first, with their slots and levels.
pub(crate) struct Around {
    pub wall_slots: Vec<Slot>,
    walls: Vec<Shape>,
    pub column_slots: Vec<Slot>,
    columns: Vec<(kentos_geometry_core::Vec2, ColumnSpec, Option<String>)>,
}

impl Around {
    pub(crate) fn of(doc: &Document, first: Slot) -> Around {
        let mut out = Around {
            wall_slots: Vec::new(),
            walls: Vec::new(),
            column_slots: Vec::new(),
            columns: Vec::new(),
        };
        let column = |slot: Slot, e: &Entity, out: &mut Around, front: bool| {
            let (Entity::Column(c), Shape::Column { at, column }) = (e, crate::geometry::shape(e))
            else {
                return;
            };
            let item = (at, column, c.column.arch.level.clone());
            if front {
                out.column_slots.insert(0, slot);
                out.columns.insert(0, item);
            } else {
                out.column_slots.push(slot);
                out.columns.push(item);
            }
        };
        for e in doc.entities() {
            let slot = Slot(e.base().id);
            match e {
                Entity::Wall(w) => {
                    out.wall_slots.push(slot);
                    out.walls.push(crate::walls::core_shape(w));
                }
                Entity::Column(_) => column(slot, e, &mut out, slot == first),
                _ => {}
            }
        }
        out
    }

    pub(crate) fn walls(&self) -> Vec<WallIn<'_>> {
        self.walls
            .iter()
            .filter_map(|s| match s {
                Shape::Wall { pts, bulges, wall } => Some(WallIn {
                    pts,
                    bulges: bulges.as_deref(),
                    spec: wall,
                }),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn columns(&self) -> Vec<ColumnIn<'_>> {
        self.columns
            .iter()
            .map(|(at, spec, level)| ColumnIn {
                at: *at,
                spec,
                level: level.as_deref(),
            })
            .collect()
    }
}

fn v(p: kentos_geometry_core::Vec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

/// The gaps of the column in `slot`; why there are none (not a column).
pub fn gaps_of(doc: &Document, slot: Slot) -> Result<ColumnGaps, String> {
    let Some(Entity::Column(c)) = doc.get(slot) else {
        return Err("Nesne kolon değil.".into());
    };
    let around = Around::of(doc, slot);
    let (walls, columns) = (around.walls(), around.columns());
    let uid = |s: Slot| doc.uid(s).map(|u| u.to_string()).unwrap_or_default();
    let sides = column_shift::gaps(&walls, &columns, 0)
        .into_iter()
        .flatten()
        .map(|g| {
            let (uid, target, edge, face) = match g.target {
                Target::Wall {
                    wall,
                    edge,
                    left_face,
                } => (
                    uid(around.wall_slots[wall]),
                    ColumnGapTarget::Wall,
                    edge as u32,
                    Some(if left_face {
                        DimensionSetFace::Left
                    } else {
                        DimensionSetFace::Right
                    }),
                ),
                Target::Column { column, side } => (
                    uid(around.column_slots[column]),
                    ColumnGapTarget::Column,
                    side as u32,
                    None,
                ),
            };
            ColumnGap {
                side: g.side as u32,
                distance: g.distance,
                uid,
                target,
                edge,
                face,
                from: v(g.from),
                to: v(g.to),
            }
        })
        .collect();
    Ok(ColumnGaps {
        uid: uid(slot),
        tied: c.column.grids.is_some(),
        sides,
    })
}

fn read(doc: &Document, input: &ColumnsDistancesInput) -> Result<ColumnsDistances, Stop> {
    checks::uids(&input.uids, "Ölçülecek kolon verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    let mut out = Vec::with_capacity(found.len());
    for (at, slot, e, _) in found {
        if !matches!(e, Entity::Column(_)) {
            return Err(fail(
                codes::NOT_A_COLUMN,
                "Nesne kolon değil; kolonun mesafeleri yalnız kolonlarda ölçülür.".into(),
                &format!("uids[{at}]"),
            ));
        }
        out.push(gaps_of(doc, slot).map_err(|why| fail(codes::NOT_A_COLUMN, why, "uids"))?);
    }
    Ok(ColumnsDistances {
        columns: out,
        revision: doc.revision().to_string(),
    })
}

pub mod distances {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &ColumnsDistancesInput) -> CommandResult<()> {
        match read(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(e) => e.into(),
        }
    }
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &ColumnsDistancesInput,
    ) -> CommandResult<ColumnsDistances> {
        plan_in(cx.doc, input)
    }
    /// The answer without a context, for a tool's overlay.
    pub fn plan_in(doc: &Document, input: &ColumnsDistancesInput) -> CommandResult<ColumnsDistances> {
        match read(doc, input) {
            Ok(output) => CommandResult::Completed {
                output,
                warnings: Vec::new(),
            },
            Err(e) => e.into(),
        }
    }
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: ColumnsDistancesInput,
    ) -> CommandResult<ColumnsDistances> {
        plan_in(cx.doc, &input)
    }
}
