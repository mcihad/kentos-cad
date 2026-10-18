//! `cad.walls.joins` v1 (read-only, docs/adr/0340 §2): how the walls named (none: every wall) join the walls round
//! them (`geom::wall_join`), and where walls meet but do not join, each with the edit that resolves it (an end's join,
//! a `cad.walls.edit` input to run as it is). It writes nothing. The shared cases are
//! `fixtures/commands/v1/cad.walls.joins.json` (`scripts/fixtures/wall_command_cases.py`).

use kentos_contracts::{
    CommandResult, Entity, Vec2, WallEnd, WallEntity, WallJoinIssue, WallJoint, WallJointKind,
    WallJointMember, WallsEdit, WallsJoined, WallsJoins, WallsOperation,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::wall_join::{End, JointKind, WallIn, solve};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::walls;

/// The walls asked about, each its slot; none named: every wall.
fn asked(doc: &Document, input: &WallsJoins) -> Result<Vec<(Slot, WallEntity)>, Stop> {
    if input.uids.is_empty() {
        return Ok(doc
            .entities()
            .filter_map(|e| match e {
                Entity::Wall(w) => Some((Slot(e.base().id), w.clone())),
                _ => None,
            })
            .collect());
    }
    checks::uids(&input.uids, "Birleşimleri sorulacak duvar verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    let mut out = Vec::with_capacity(found.len());
    for (at, slot, e, _) in found {
        let Entity::Wall(w) = e else {
            return Err(Stop::failed(error(
                codes::NOT_A_WALL,
                "Nesne duvar değil; birleşimler yalnız duvarlarda sorulur.".into(),
                Some(format!("uids[{at}]")),
            )));
        };
        out.push((slot, w.clone()));
    }
    Ok(out)
}

fn end_of(e: End) -> WallEnd {
    match e {
        End::Start => WallEnd::Start,
        End::End => WallEnd::End,
    }
}

fn answer(doc: &Document, input: &WallsJoins) -> CommandResult<WallsJoined> {
    let walls_asked = match asked(doc, input) {
        Ok(w) => w,
        Err(stop) => return stop.into(),
    };
    let changed: Vec<(Option<Slot>, &WallEntity)> =
        walls_asked.iter().map(|(s, w)| (Some(*s), w)).collect();
    let (shapes, others) = walls::neighbourhood(doc, &changed, &[]);
    let slots: Vec<Slot> = walls_asked.iter().map(|(s, _)| *s).chain(others).collect();
    let uid = |k: usize| doc.uid(slots[k]).map(|u| u.to_string()).unwrap_or_default();
    let ins: Vec<WallIn<'_>> = shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Wall { pts, bulges, wall } => Some(WallIn {
                pts,
                bulges: bulges.as_deref(),
                spec: wall,
            }),
            _ => None,
        })
        .collect();
    let solved = solve(&ins, Some(&[]));
    let n = walls_asked.len();
    let joints: Vec<WallJoint> = solved
        .joints
        .iter()
        .filter(|j| j.members.iter().any(|m| m.wall < n))
        .map(|j| WallJoint {
            kind: match j.kind {
                JointKind::Corner => WallJointKind::Corner,
                JointKind::Inline => WallJointKind::Inline,
                JointKind::Tee => WallJointKind::Tee,
                JointKind::Cross => WallJointKind::Cross,
            },
            at: Vec2 {
                x: j.at.x,
                y: j.at.y,
            },
            walls: j
                .members
                .iter()
                .map(|m| WallJointMember {
                    uid: uid(m.wall),
                    end: m.end.map(end_of),
                })
                .collect(),
        })
        .collect();
    let issues: Vec<WallJoinIssue> = solved
        .issues
        .iter()
        .filter(|i| i.walls.iter().any(|&w| w < n))
        .map(|i| {
            let (message, repair) = i.code.message();
            WallJoinIssue {
                code: i.code.code().into(),
                message: message.into(),
                repair: repair.into(),
                at: Vec2 {
                    x: i.at.x,
                    y: i.at.y,
                },
                uids: i.walls.iter().map(|&w| uid(w)).collect(),
                fix: i.fix.map(|f| {
                    let mode = kentos_formats::walls::join_back(f.mode);
                    let (start, end) = match f.end {
                        End::Start => (Some(mode), None),
                        End::End => (None, Some(mode)),
                    };
                    WallsEdit {
                        uids: vec![uid(f.wall)],
                        operation: WallsOperation::Joins {
                            start,
                            end,
                            cleanup: None,
                        },
                        expected_revision: None,
                    }
                }),
            }
        })
        .collect();
    // In order along x, then y, then the kind's name.
    let mut joints: Vec<WallJoint> = joints;
    let kind_name = |k: WallJointKind| match k {
        WallJointKind::Corner => "corner",
        WallJointKind::Inline => "inline",
        WallJointKind::Tee => "tee",
        WallJointKind::Cross => "cross",
    };
    joints.sort_by(|a, b| {
        a.at.x
            .total_cmp(&b.at.x)
            .then(a.at.y.total_cmp(&b.at.y))
            .then(kind_name(a.kind).cmp(kind_name(b.kind)))
    });
    let mut issues: Vec<WallJoinIssue> = issues;
    issues.sort_by(|a, b| {
        a.at.x
            .total_cmp(&b.at.x)
            .then(a.at.y.total_cmp(&b.at.y))
            .then(a.code.cmp(&b.code))
    });
    CommandResult::Completed {
        output: WallsJoined {
            joints,
            issues,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &WallsJoins) -> CommandResult<()> {
    match asked(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The joints; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &WallsJoins) -> CommandResult<WallsJoined> {
    answer(cx.doc, input)
}

/// The joints. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: WallsJoins) -> CommandResult<WallsJoined> {
    answer(cx.doc, &input)
}
