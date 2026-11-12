//! An opening's centre grip uses the existing semantic opening command.
//! Generic EntityGeometry deliberately excludes openings; passing one there
//! used to silently produce no change. Host projection stays in native plan.
use super::*;
use kentos_commands::{CommandResult, Entity, OpeningsEdit, OpeningsEdited, OpeningsOperation};
use kentos_native_application::openings_edit;

fn request(
    grips: &[HotGrip],
    dx: f64,
    dy: f64,
    copy: bool,
    cx: &Context<'_>,
) -> Option<Result<OpeningsEdit, CommandResult<OpeningsEdited>>> {
    let [grip] = grips else { return None };
    let Entity::Opening(o) = cx.doc.get(grip.slot)? else {
        return None;
    };
    let uid = cx.doc.uid(grip.slot)?.to_string();
    let input = OpeningsEdit {
        uids: vec![uid],
        expected_revision: None,
        operation: OpeningsOperation::Rehost {
            to: o.opening.host.clone(),
            point: Some(crate::points::wire(Vec2::new(
                grip.origin.x + dx,
                grip.origin.y + dy,
            ))),
        },
    };
    if !copy {
        return Some(Ok(input));
    }
    // Copy's own native plan validates collision with the source too. The
    // Rehost plan supplies the host-path coordinate, including a curved host.
    Some(match openings_edit::plan_in(cx.doc, &input) {
        CommandResult::Completed { output, .. } => {
            let Entity::Opening(target) = &output.openings[0] else {
                unreachable!("opening command")
            };
            Ok(OpeningsEdit {
                operation: OpeningsOperation::Copy {
                    offsets: vec![target.opening.at - o.opening.at],
                },
                ..input
            })
        }
        other => Err(other),
    })
}

/// The opening as the grip would leave it (its copies too), and what follows it: the dimensions it adds, changes and
/// removes (docs/adr/0695: a set's members, docs/adr/0729); the bonds' refusal instead, marked in place
/// (docs/adr/0790 §1).
pub(super) fn preview(
    grips: &[HotGrip],
    dx: f64,
    dy: f64,
    copy: bool,
    cx: &Context<'_>,
) -> Option<(
    Vec<Shape>,
    kentos_commands::DerivedEntitiesPlan,
    Option<crate::bond_refusal::Refused>,
)> {
    let input = request(grips, dx, dy, copy, cx)?;
    let result = match input {
        Ok(input) => openings_edit::plan_in(cx.doc, &input),
        Err(result) => result,
    };
    Some(match result {
        CommandResult::Completed { output, .. } => (
            output
                .openings
                .iter()
                .chain(&output.added)
                .map(shape)
                .collect(),
            output.followers,
            None,
        ),
        other => (
            Vec::new(),
            Default::default(),
            crate::bond_refusal::Refused::of_result(&other, cx),
        ),
    })
}

impl Select {
    pub(super) fn commit_opening_grip(
        &mut self,
        grips: &[HotGrip],
        dx: f64,
        dy: f64,
        copy: bool,
        cx: &mut Context<'_>,
    ) -> bool {
        let Some(input) = request(grips, dx, dy, copy, cx) else {
            // A mixed hot-grip operation must not skip the opening and write
            // only the other objects through generic EntityGeometry.
            if grips
                .iter()
                .any(|g| matches!(cx.doc.get(g.slot), Some(Entity::Opening(_))))
            {
                cx.say(Level::Warn,"Açıklığı kendi merkez tutamacıyla tek başına taşıyın; hiçbir nesne değiştirilmedi.");
                return true;
            }
            return false;
        };
        let result = match input {
            Ok(input) => openings_edit::execute(&mut ExecutionContext::new(cx.doc), input),
            Err(result) => result,
        };
        points::written(result, cx);
        true
    }
}
