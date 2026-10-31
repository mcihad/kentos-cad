//! Bonds in the way while an edit is made (docs/adr/0790 §1): a grip's drag, Duvarı kaydır, Boy, an opening's move
//! or its distance plan their command as the pointer moves or the value is typed; when the architectural bonds refuse
//! it (`constraint_conflict` with `bonds`), the preview marks the bonds where they stand (each wall line thick, each
//! jamb ringed, a dashed line between a bond's parts and its name) and a card says the refusal and each bond, as
//! Duvarın mesafesi does (docs/adr/0780 §3). The card's way through is the bond that frees it alone, or the set that
//! frees it together (docs/adr/0790 §2): [`way_through`], written by [`remove`] as one `cad.constraints.edit` `remove`.

use kentos_commands::{BondInTheWay, CommandError, CommandResult, ConstraintEdit, ConstraintsEdit};
use kentos_native_application::{ExecutionContext, constraints_edit};

use crate::log::Level;
use crate::tool::{Context, Preview, Tag, Tone};
use crate::{Vec2, wall_distance};

/// A planned write the bonds refuse: its card's lines and its bonds marked in place.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Refused {
    pub lines: Vec<String>,
    pub marks: Preview,
    /// The bonds (their ids) whose taking away lets it through, and the card's word for it.
    pub through: Option<(Vec<String>, String)>,
}

impl Refused {
    /// A refusal with bonds in the way; none for every other answer.
    pub fn of(error: &CommandError, cx: &Context<'_>) -> Option<Refused> {
        if error.bonds.is_empty() {
            return None;
        }
        let f = cx.format();
        Some(Refused {
            lines: wall_distance::refusal_lines(error, &f),
            marks: wall_distance::bonds_in_way(&error.bonds, cx.doc),
            through: way_through(&error.bonds),
        })
    }

    /// A plan's answer: its refusal with bonds in the way, else none.
    pub fn of_result<T>(result: &CommandResult<T>, cx: &Context<'_>) -> Option<Refused> {
        match result {
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Self::of(error, cx),
            _ => None,
        }
    }

    /// The preview with the bonds marked and the card at `at` in the danger colour.
    pub fn over(&self, mut preview: Preview, at: Vec2) -> Preview {
        let marks = self.marks.clone();
        preview.strokes.extend(marks.strokes);
        preview.markers.extend(marks.markers);
        preview.labels.extend(marks.labels);
        preview.tag = Some(Tag {
            at,
            lines: self.lines.clone(),
        });
        preview.tag_tone = Tone::Danger;
        preview
    }
}

/// The way through a refusal (docs/adr/0790 §2): the first bond that frees it alone (“Bağı kaldır: d1”), else the
/// set that frees it together (“Bu iki bağı kaldır”, “Bu üç bağı kaldır”); none when neither is known.
pub fn way_through(bonds: &[BondInTheWay]) -> Option<(Vec<String>, String)> {
    let name = |b: &BondInTheWay| b.parameter.clone().unwrap_or_else(|| b.label.clone());
    if let Some(b) = bonds.iter().find(|b| b.frees) {
        return Some((vec![b.id.to_string()], format!("Bağı kaldır: {}", name(b))));
    }
    let set: Vec<&BondInTheWay> = bonds.iter().filter(|b| !b.frees_with.is_empty()).collect();
    match set.len() {
        2 => Some((set.iter().map(|b| b.id.to_string()).collect(), "Bu iki bağı kaldır".into())),
        3 => Some((set.iter().map(|b| b.id.to_string()).collect(), "Bu üç bağı kaldır".into())),
        _ => None,
    }
}

/// Takes the bonds `ids` away in one undo step (`cad.constraints.edit` `remove`, their parameters with them); says
/// how many went. Whether they went.
pub fn remove(ids: Vec<String>, cx: &mut Context<'_>) -> bool {
    let n = ids.len();
    let input = ConstraintsEdit {
        edit: ConstraintEdit::Remove { ids },
        hold: Vec::new(),
        allow_flip: false,
        expected_revision: None,
    };
    let result = constraints_edit::execute(&mut ExecutionContext::new(cx.doc), input);
    if crate::points::written(result, cx).is_none() {
        return false;
    }
    cx.say(
        Level::Success,
        if n == 1 {
            "Bağ kaldırıldı; düzenlemeyi yeniden deneyin.".to_owned()
        } else {
            format!("{n} bağ birlikte kaldırıldı; düzenlemeyi yeniden deneyin.")
        },
    );
    true
}
