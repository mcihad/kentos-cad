//! A wall's temporary distances and Kaydır (docs/adr/0710): on each side of a straight edge, the clear distance to the
//! nearest parallel wall's facing face and, from its axis, to the nearest parallel grid, and its length
//! (`cad.walls.distances`, docs/adr/0728); `cad.walls.edit`'s `shift`, `distance` and `length` move it, what is joined
//! to it following. The core's `ops::wall_shift` is the rule; this reads the drawing's walls for it. The shared cases
//! are `fixtures/commands/v1/cad.walls.distances.json`, `cad.walls.edit.shift.json` and `cad.walls.edit.length.json`.

use kentos_commands::{
    CommandResult, DimensionSetFace, Entity, Vec2, WallEntity, WallGap, WallGaps, WallLength, WallsDistances,
    WallsDistancesInput,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::wall_join::WallIn;
use kentos_geometry_core::ops::wall_shift::{self, Gap};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub(crate) fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The drawing's walls as the core takes them, `first` first, with their slots.
pub(crate) struct Walls {
    pub slots: Vec<Slot>,
    shapes: Vec<Shape>,
}

impl Walls {
    pub(crate) fn of(doc: &Document, first: Slot, wall: &WallEntity) -> Walls {
        let mut slots = vec![first];
        let mut shapes = vec![crate::walls::core_shape(wall)];
        for e in doc.entities() {
            let slot = Slot(e.base().id);
            if let Entity::Wall(w) = e
                && slot != first
            {
                slots.push(slot);
                shapes.push(crate::walls::core_shape(w));
            }
        }
        Walls { slots, shapes }
    }

    pub(crate) fn ins(&self) -> Vec<WallIn<'_>> {
        self.shapes
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
}

fn v(p: kentos_geometry_core::Vec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

fn described(doc: &Document, walls: &Walls, g: &Gap) -> Option<WallGap> {
    Some(WallGap {
        distance: g.distance,
        uid: doc.uid(walls.slots[g.wall])?.to_string(),
        edge: g.edge as u32,
        face: if g.left_face {
            DimensionSetFace::Left
        } else {
            DimensionSetFace::Right
        },
        from: v(g.from),
        to: v(g.to),
    })
}

/// The gaps of the wall in `slot` on its edge `edge` (absent: its first); why there are none (not a wall, an edge it
/// has not, an arc). With each side's nearest parallel grid from the edge's axis and the edge's length when an end of
/// it can move (docs/adr/0728).
pub fn gaps_of(doc: &Document, slot: Slot, edge: Option<u32>) -> Result<WallGaps, String> {
    let Some(Entity::Wall(w)) = doc.get(slot) else {
        return Err("Nesne duvar değil.".into());
    };
    let walls = Walls::of(doc, slot, w);
    let ins = walls.ins();
    let k = edge.unwrap_or(0) as usize;
    let [left, right] = wall_shift::gaps(&ins, 0, k)?;
    let grids = crate::grid_refs::Grids::of(doc);
    let (at, u) = wall_shift::axis_middle(&ins, 0, k)?;
    let [left_grid, right_grid] =
        kentos_geometry_core::ops::grid_ref::beside(&grids.ins(), wall_shift::level(&ins[0]), at, u);
    let free = wall_shift::free_ends(&ins, 0, k);
    let length = (free[0] || free[1]).then(|| {
        let (a, b) = (ins[0].pts[k], ins[0].pts[(k + 1) % ins[0].pts.len()]);
        WallLength {
            length: wall_shift::edge_length(&ins, 0, k).unwrap_or_default(),
            from: v(a),
            to: v(b),
            start_free: free[0],
            end_free: free[1],
        }
    });
    Ok(WallGaps {
        uid: doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
        edge: k as u32,
        left: left.and_then(|g| described(doc, &walls, &g)),
        right: right.and_then(|g| described(doc, &walls, &g)),
        left_grid: left_grid.and_then(|g| grids.described(doc, &g)),
        right_grid: right_grid.and_then(|g| grids.described(doc, &g)),
        length,
    })
}

fn read(doc: &Document, input: &WallsDistancesInput) -> Result<WallsDistances, Stop> {
    checks::uids(&input.uids, "Ölçülecek duvar verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    let mut out = Vec::with_capacity(found.len());
    for (at, slot, e, _) in found {
        if !matches!(e, Entity::Wall(_)) {
            return Err(fail(
                codes::NOT_A_WALL,
                "Nesne duvar değil; duvarın mesafeleri yalnız duvarlarda ölçülür.".into(),
                &format!("uids[{at}]"),
            ));
        }
        out.push(
            gaps_of(doc, slot, input.edge).map_err(|why| fail(codes::INVALID_WALL, why, "edge"))?,
        );
    }
    Ok(WallsDistances {
        walls: out,
        revision: doc.revision().to_string(),
    })
}

pub mod distances {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &WallsDistancesInput) -> CommandResult<()> {
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
        input: &WallsDistancesInput,
    ) -> CommandResult<WallsDistances> {
        plan_in(cx.doc, input)
    }
    /// The answer without a context, for a tool's overlay.
    pub fn plan_in(doc: &Document, input: &WallsDistancesInput) -> CommandResult<WallsDistances> {
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
        input: WallsDistancesInput,
    ) -> CommandResult<WallsDistances> {
        plan_in(cx.doc, &input)
    }
}
