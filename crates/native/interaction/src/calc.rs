//! What the Hesap windows write (docs/adr/0070, 0071, 0622 §9). Poligon hesabı, Kutupsal alım, Önden and Geriden
//! kestirme hand what was typed to `cad.survey.points`, which computes the points and writes them, one undo step
//! named after the window; the network adjustments write their new points through `cad.entities.create`, as the
//! web's `addPoints` writes them since dbdacc1. Each point carries its name as its label and its Ad attribute, the
//! window's kind as Tür, and its height as z and “Z (m)”. The window says a locked or hidden layer in its own words
//! (it has a layer picker).

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, SurveyPoints,
    Vec2 as Wire,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, codes, create, survey_points};

use crate::Vec2;
use crate::format::fixed;

/// A point a Hesap window adds.
#[derive(Clone, Debug, PartialEq)]
pub struct SurveyPoint {
    pub name: String,
    pub p: Vec2,
    pub z: Option<f64>,
}

/// Writes `points` onto `layer` as one step named after `operation`. The
/// new points' slots and the command's warnings but the hidden layer's, or
/// why nothing was written.
pub fn add_points(
    doc: &mut Document,
    layer: &str,
    points: &[SurveyPoint],
    kind: &str,
    operation: CreateOperation,
) -> Result<(Vec<Slot>, Vec<String>), String> {
    let objects = points
        .iter()
        .map(|pt| {
            let mut attrs = BTreeMap::from([
                ("Ad".to_owned(), pt.name.clone()),
                ("Tür".to_owned(), kind.to_owned()),
            ]);
            if let Some(z) = pt.z {
                attrs.insert("Z (m)".to_owned(), fixed(z, 3));
            }
            NewObject {
                line_type: None,
                geometry: EntityGeometry::Point {
                    p: Wire {
                        x: pt.p.x,
                        y: pt.p.y,
                    },
                    z: pt.z,
                    parts: None,
                },
                color: None,
                line_weight: None,
                attrs: Some(attrs),
                label: Some(pt.name.clone()),
                label_of: None,
                label_scale: None,
                symbol: None,
                order: None,
                transparency: None,
            }
        })
        .collect();
    let input = EntitiesCreate {
        layer_id: layer.to_owned(),
        objects,
        operation: Some(operation),
        expected_revision: None,
    };
    written(
        create::execute(&mut ExecutionContext::new(doc), input),
        |output| output.ids,
    )
}

/// Writes what a survey window was given through `cad.survey.points` (docs/adr/0622 §9): the window hands over the
/// known points and the observations as typed, the command computes the points and writes them as one step named
/// after the window. The new points' slots and the command's warnings but the hidden layer's, or why nothing was
/// written.
pub fn write_survey(
    doc: &mut Document,
    input: SurveyPoints,
) -> Result<(Vec<Slot>, Vec<String>), String> {
    written(
        survey_points::execute(&mut ExecutionContext::new(doc), input),
        |output| output.ids,
    )
}

/// A command's new objects and its warnings but the hidden layer's (the window says that in its own words), or its
/// refusal's words.
fn written<T>(
    result: CommandResult<T>,
    ids: impl FnOnce(T) -> Vec<u32>,
) -> Result<(Vec<Slot>, Vec<String>), String> {
    match result {
        CommandResult::Completed { output, warnings } => Ok((
            ids(output).into_iter().map(Slot).collect(),
            warnings
                .into_iter()
                .filter(|w| w.code != codes::LAYER_HIDDEN)
                .map(|w| w.message)
                .collect(),
        )),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        CommandResult::Queued { .. } | CommandResult::Cancelled => Ok((Vec::new(), Vec::new())),
    }
}
